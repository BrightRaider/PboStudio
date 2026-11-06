//! The Windows layer: threads, CPUID, and child processes inside a job object.
//!
//! Everything that has to be put back is held by a guard that puts it back when it is dropped, however the
//! caller leaves (ADR-0006): a pinned thread goes back to where it was, a child process is ended and its
//! handles closed. That is why the release profile unwinds on panic instead of aborting.

#![cfg(windows)]

pub mod pipe;
pub mod system;

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, HANDLE};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_AFFINITY,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows_sys::Win32::System::SystemInformation::GROUP_AFFINITY;
use windows_sys::Win32::System::Threading::{
    CreateProcessW, GetActiveProcessorCount, GetCurrentThread, GetExitCodeProcess, GetProcessTimes,
    ResumeThread, SetThreadGroupAffinity, TerminateProcess, ABOVE_NORMAL_PRIORITY_CLASS,
    BELOW_NORMAL_PRIORITY_CLASS, CREATE_NO_WINDOW, CREATE_SUSPENDED, NORMAL_PRIORITY_CLASS,
    PROCESS_INFORMATION, STARTUPINFOW,
};

/// A thread pinned to one logical processor. Dropping it restores the previous affinity.
pub struct Pin {
    previous: GROUP_AFFINITY,
}

impl Drop for Pin {
    fn drop(&mut self) {
        // Best effort: there is nothing useful to do if putting the thread back fails.
        unsafe {
            SetThreadGroupAffinity(GetCurrentThread(), &self.previous, std::ptr::null_mut());
        }
    }
}

/// Number of logical processors across all processor groups.
pub fn logical_processors() -> usize {
    // ALL_PROCESSOR_GROUPS is 0xFFFF.
    unsafe { GetActiveProcessorCount(0xFFFF) as usize }
}

/// Pins the calling thread to one logical processor. `None` if Windows refuses.
pub fn pin_current_thread(logical: usize) -> Option<Pin> {
    let target = GROUP_AFFINITY {
        Mask: 1usize << (logical % 64),
        Group: (logical / 64) as u16,
        Reserved: [0; 3],
    };
    let mut previous = GROUP_AFFINITY {
        Mask: 0,
        Group: 0,
        Reserved: [0; 3],
    };
    let ok = unsafe { SetThreadGroupAffinity(GetCurrentThread(), &target, &mut previous) };
    if ok == 0 {
        return None;
    }
    // The move to the other processor happens when the thread is next scheduled.
    std::thread::yield_now();
    Some(Pin { previous })
}

/// CPUID with a sub-leaf. Returns EAX, EBX, ECX, EDX.
pub fn cpuid(leaf: u32, sub_leaf: u32) -> [u32; 4] {
    #[allow(unused_unsafe)]
    let r = unsafe { core::arch::x86_64::__cpuid_count(leaf, sub_leaf) };
    [r.eax, r.ebx, r.ecx, r.edx]
}

/// The processor's brand string from CPUID leaves 8000_0002 to 8000_0004.
pub fn brand_string() -> String {
    let mut bytes = Vec::with_capacity(48);
    for leaf in 0x8000_0002u32..=0x8000_0004 {
        for reg in cpuid(leaf, 0) {
            bytes.extend_from_slice(&reg.to_le_bytes());
        }
    }
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_string()
}

/// The x2APIC ID of every logical processor, read with CPUID leaf 0xB on a thread pinned to it.
/// An entry is `None` when the thread could not be pinned, so a gap is visible and nothing is guessed.
pub fn apic_ids() -> Vec<Option<u32>> {
    (0..logical_processors())
        .map(|logical| {
            let _pin = pin_current_thread(logical)?;
            Some(cpuid(0xB, 0)[3])
        })
        .collect()
}

/// Threads per core from CPUID leaf 8000_001E (EBX bits 15:8, plus one).
pub fn threads_per_core() -> u32 {
    ((cpuid(0x8000_001E, 0)[1] >> 8) & 0xFF) + 1
}

fn wide(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(std::iter::once(0)).collect()
}

#[link(name = "ntdll")]
extern "system" {
    fn NtSuspendProcess(process: HANDLE) -> i32;
    fn NtResumeProcess(process: HANDLE) -> i32;
}

const STILL_ACTIVE: u32 = 259;

/// How hard a child competes with the rest of the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    BelowNormal,
    Normal,
    AboveNormal,
}

impl Priority {
    fn flag(self) -> u32 {
        match self {
            Priority::BelowNormal => BELOW_NORMAL_PRIORITY_CLASS,
            Priority::Normal => NORMAL_PRIORITY_CLASS,
            Priority::AboveNormal => ABOVE_NORMAL_PRIORITY_CLASS,
        }
    }
}

/// The handle of a child, for suspending and resuming it from a thread of its own. See [`Child::suspender`].
#[derive(Debug, Clone, Copy)]
pub struct Suspender(usize);

impl Suspender {
    pub fn suspend(&self) {
        unsafe { NtSuspendProcess(self.0 as HANDLE) };
    }

    pub fn resume(&self) {
        unsafe { NtResumeProcess(self.0 as HANDLE) };
    }
}

/// A child process inside a job object. Dropping it ends the process and closes everything; the job
/// also carries `KILL_ON_JOB_CLOSE`, so the process cannot outlive its parent either.
pub struct Child {
    job: HANDLE,
    process: HANDLE,
    thread: HANDLE,
    pub pid: u32,
}

impl Drop for Child {
    fn drop(&mut self) {
        unsafe {
            TerminateProcess(self.process, 0);
            CloseHandle(self.thread);
            CloseHandle(self.process);
            CloseHandle(self.job);
        }
    }
}

fn ticks(ft: &FILETIME) -> u64 {
    (u64::from(ft.dwHighDateTime) << 32) | u64::from(ft.dwLowDateTime)
}

impl Child {
    /// Starts `exe` with `args`, suspended, puts it in a job object limited to `affinity_mask`, then lets it run.
    /// A process can widen its own affinity; it cannot get out of the job's.
    pub fn start_in_job(exe: &Path, args: &str, affinity_mask: usize) -> Result<Child, String> {
        Child::start(exe, args, affinity_mask, None, Priority::Normal)
    }

    /// [`Child::start_in_job`] with a working directory and a priority class.
    pub fn start(
        exe: &Path,
        args: &str,
        affinity_mask: usize,
        work_dir: Option<&Path>,
        priority: Priority,
    ) -> Result<Child, String> {
        let mut command: Vec<u16> = wide(OsStr::new(&format!("\"{}\" {}", exe.display(), args)));
        let dir: Option<Vec<u16>> = work_dir.map(|d| wide(d.as_os_str()));

        let mut startup: STARTUPINFOW = unsafe { std::mem::zeroed() };
        startup.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        let mut info: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

        let created = unsafe {
            CreateProcessW(
                std::ptr::null(),
                command.as_mut_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                CREATE_SUSPENDED | CREATE_NO_WINDOW | priority.flag(),
                std::ptr::null(),
                dir.as_ref().map_or(std::ptr::null(), |d| d.as_ptr()),
                &startup,
                &mut info,
            )
        };
        if created == 0 {
            return Err("CreateProcess failed".into());
        }

        // From here on the guard owns the handles, so every early return cleans up.
        let job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        let child = Child {
            job,
            process: info.hProcess,
            thread: info.hThread,
            pid: info.dwProcessId,
        };
        if job.is_null() {
            return Err("CreateJobObject failed".into());
        }

        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        limits.BasicLimitInformation.LimitFlags =
            JOB_OBJECT_LIMIT_AFFINITY | JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        limits.BasicLimitInformation.Affinity = affinity_mask;
        let set = unsafe {
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const core::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if set == 0 {
            return Err("SetInformationJobObject failed".into());
        }
        if unsafe { AssignProcessToJobObject(job, info.hProcess) } == 0 {
            return Err("AssignProcessToJobObject failed".into());
        }
        if unsafe { ResumeThread(info.hThread) } == u32::MAX {
            return Err("ResumeThread failed".into());
        }
        Ok(child)
    }

    /// Whether the process is still running.
    pub fn is_alive(&self) -> bool {
        let mut code = 0u32;
        let ok = unsafe { GetExitCodeProcess(self.process, &mut code) };
        ok != 0 && code == STILL_ACTIVE
    }

    /// The exit code, or `None` while the process runs.
    pub fn exit_code(&self) -> Option<u32> {
        let mut code = 0u32;
        let ok = unsafe { GetExitCodeProcess(self.process, &mut code) };
        (ok != 0 && code != STILL_ACTIVE).then_some(code)
    }

    /// Freezes every thread of the process. The engines are paused this way while a core's value is changed.
    pub fn suspend(&self) {
        unsafe { NtSuspendProcess(self.process) };
    }

    pub fn resume(&self) {
        unsafe { NtResumeProcess(self.process) };
    }

    /// A copy of the handle for freezing and releasing the process from another thread (the pulses of a micro-burst
    /// test run on their own clock). It owns nothing: it must not be used after the child is dropped.
    pub fn suspender(&self) -> Suspender {
        Suspender(self.process as usize)
    }

    /// Ends the process now. Dropping the child does the same.
    pub fn kill(&self) {
        unsafe { TerminateProcess(self.process, 1) };
    }

    /// CPU time the process has used so far (kernel plus user).
    pub fn cpu_time(&self) -> Duration {
        let (mut created, mut exited, mut kernel, mut user) = (
            FILETIME {
                dwLowDateTime: 0,
                dwHighDateTime: 0,
            },
            FILETIME {
                dwLowDateTime: 0,
                dwHighDateTime: 0,
            },
            FILETIME {
                dwLowDateTime: 0,
                dwHighDateTime: 0,
            },
            FILETIME {
                dwLowDateTime: 0,
                dwHighDateTime: 0,
            },
        );
        unsafe {
            GetProcessTimes(
                self.process,
                &mut created,
                &mut exited,
                &mut kernel,
                &mut user,
            )
        };
        // FILETIME counts 100 ns ticks.
        Duration::from_nanos((ticks(&kernel) + ticks(&user)) * 100)
    }

    /// Average number of cores the process kept busy over `window`.
    pub fn measure_load(&self, window: Duration) -> f64 {
        let before = self.cpu_time();
        let started = Instant::now();
        std::thread::sleep(window);
        let used = self.cpu_time() - before;
        used.as_secs_f64() / started.elapsed().as_secs_f64()
    }
}

/// The local wall-clock time, for the log's file name and its lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTime {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

pub fn local_time() -> LocalTime {
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;
    let mut t = unsafe { std::mem::zeroed() };
    unsafe { GetLocalTime(&mut t) };
    LocalTime {
        year: i32::from(t.wYear),
        month: u32::from(t.wMonth),
        day: u32::from(t.wDay),
        hour: u32::from(t.wHour),
        minute: u32::from(t.wMinute),
        second: u32::from(t.wSecond),
    }
}

/// The current UTC time as Windows writes event times (`2026-11-05T10:00:00.123Z`), for event log queries.
pub fn utc_iso() -> String {
    use windows_sys::Win32::System::SystemInformation::GetSystemTime;
    let mut t = unsafe { std::mem::zeroed() };
    unsafe { GetSystemTime(&mut t) };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond, t.wMilliseconds
    )
}

/// Milliseconds since the machine started: two different values for the same boot never go backwards, and a
/// smaller value than a stored one means the machine has restarted since (invariant I9).
pub fn uptime_ms() -> u64 {
    use windows_sys::Win32::System::SystemInformation::GetTickCount64;
    unsafe { GetTickCount64() }
}

/// The frequency the time stamp counter runs at, in MHz, measured over about a tenth of a second. On AMD it is the
/// frequency of the base P-state, which is the rate MPERF counts at; the effective clock of a core is that times
/// APERF/MPERF. Measured once, then remembered.
pub fn tsc_mhz() -> f64 {
    use std::sync::OnceLock;
    static MHZ: OnceLock<f64> = OnceLock::new();
    *MHZ.get_or_init(|| {
        #[cfg(target_arch = "x86_64")]
        {
            let t0 = Instant::now();
            let c0 = unsafe { std::arch::x86_64::_rdtsc() };
            std::thread::sleep(Duration::from_millis(100));
            let c1 = unsafe { std::arch::x86_64::_rdtsc() };
            let secs = t0.elapsed().as_secs_f64();
            (c1.wrapping_sub(c0)) as f64 / secs / 1.0e6
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            0.0
        }
    })
}

/// The BIOS version string, from the registry. Firmware moves stability, so what was learned is stamped with it.
pub fn bios_version() -> Option<String> {
    local_machine_string(r"HARDWARE\DESCRIPTION\System\BIOS", "BIOSVersion")
}

/// A text value under `HKEY_LOCAL_MACHINE`, or `None` when it is not there or not readable.
pub fn local_machine_string(key: &str, name: &str) -> Option<String> {
    use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};
    let key = wide(OsStr::new(key));
    let name = wide(OsStr::new(name));
    let mut size = 0u32;
    let first = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut size,
        )
    };
    if first != 0 || size == 0 {
        return None;
    }
    let mut buffer = vec![0u16; (size as usize).div_ceil(2)];
    let second = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buffer.as_mut_ptr().cast(),
            &mut size,
        )
    };
    if second != 0 {
        return None;
    }
    let end = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
    Some(String::from_utf16_lossy(&buffer[..end]))
}

static CANCELLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

unsafe extern "system" fn on_ctrl(_kind: u32) -> i32 {
    CANCELLED.store(true, std::sync::atomic::Ordering::SeqCst);
    1
}

/// Makes Ctrl+C and closing the console set a flag instead of ending the program at once, so a running test stops
/// its engine and puts everything back (ADR-0006).
pub fn handle_ctrl_c() {
    use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;
    unsafe { SetConsoleCtrlHandler(Some(on_ctrl), 1) };
}

pub fn cancelled() -> bool {
    CANCELLED.load(std::sync::atomic::Ordering::SeqCst)
}

/// Asks a running session to stop, the same as Ctrl+C does (the window's stop button).
pub fn request_cancel() {
    CANCELLED.store(true, std::sync::atomic::Ordering::SeqCst);
}

/// Clears the stop request before a new run starts.
pub fn reset_cancel() {
    CANCELLED.store(false, std::sync::atomic::Ordering::SeqCst);
}

/// Detaches from the console when this program owns it alone (it was started by a double click), so the window
/// does not come with a black box. From a terminal the console is shared and stays.
/// What a sound says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sound {
    /// A core failed.
    Error,
    /// The run is over.
    Done,
}

#[link(name = "user32")]
extern "system" {
    fn MessageBeep(kind: u32) -> i32;
}

/// Plays the system's own sound for an error or for a notice (the ones the person set in the sound settings, or none).
pub fn sound(which: Sound) {
    const ICON_HAND: u32 = 0x10;
    const ICON_ASTERISK: u32 = 0x40;
    unsafe {
        MessageBeep(match which {
            Sound::Error => ICON_HAND,
            Sound::Done => ICON_ASTERISK,
        })
    };
}

#[link(name = "kernel32")]
extern "system" {
    fn GetShortPathNameW(long_path: *const u16, short_path: *mut u16, size: u32) -> u32;
}

/// The 8.3 name of a path (no spaces in it), for a program that cannot be given a path with a space in it. `None` when
/// the file system has no short names or the path is not there.
pub fn short_path(path: &Path) -> Option<String> {
    let long = wide(path.as_os_str());
    let mut buffer = vec![0u16; 1024];
    let n = unsafe { GetShortPathNameW(long.as_ptr(), buffer.as_mut_ptr(), buffer.len() as u32) }
        as usize;
    if n == 0 || n >= buffer.len() {
        return None;
    }
    Some(String::from_utf16_lossy(&buffer[..n]))
}

pub fn hide_own_console() {
    use windows_sys::Win32::System::Console::{FreeConsole, GetConsoleProcessList};
    let mut list = [0u32; 2];
    let attached = unsafe { GetConsoleProcessList(list.as_mut_ptr(), 2) };
    if attached == 1 {
        unsafe { FreeConsole() };
    }
}

/// The user's interface language is German.
pub fn ui_language_is_german() -> bool {
    use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;
    (unsafe { GetUserDefaultUILanguage() } & 0x3FF) == 0x07
}

/// Restarts this program with administrator rights (the system asks the person). Returns whether it was launched.
pub fn relaunch_elevated() -> bool {
    match std::env::current_exe() {
        Ok(exe) => run_as_administrator(&exe, "", true),
        Err(_) => false,
    }
}

/// Starts this program with administrator rights and these arguments, without a window (the system asks the person).
/// For the helper, which is a console program nobody needs to see.
pub fn relaunch_elevated_with(arguments: &str) -> bool {
    match std::env::current_exe() {
        Ok(exe) => run_as_administrator(&exe, arguments, false),
        Err(_) => false,
    }
}

/// Starts a file with administrator rights; the system asks the person. `show` is whether its window is shown.
pub fn run_as_administrator(file: &Path, arguments: &str, show: bool) -> bool {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    let verb = wide(OsStr::new("runas"));
    let file = wide(file.as_os_str());
    let params = wide(OsStr::new(arguments));
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            if arguments.is_empty() {
                std::ptr::null()
            } else {
                params.as_ptr()
            },
            std::ptr::null(),
            i32::from(show),
        )
    };
    (result as isize) > 32
}

/// Keeps the machine from going to sleep while it is alive (the display may still turn off). A stress test is no user
/// activity, so without this an overnight run is cut off by the sleep timer. Set on the thread that creates it and
/// undone when it is dropped, so it is held exactly as long as the run is.
pub struct KeepAwake;

impl KeepAwake {
    pub fn new() -> KeepAwake {
        use windows_sys::Win32::System::Power::{
            SetThreadExecutionState, ES_CONTINUOUS, ES_SYSTEM_REQUIRED,
        };
        unsafe { SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED) };
        KeepAwake
    }
}

impl Default for KeepAwake {
    fn default() -> Self {
        KeepAwake::new()
    }
}

impl Drop for KeepAwake {
    fn drop(&mut self) {
        use windows_sys::Win32::System::Power::{SetThreadExecutionState, ES_CONTINUOUS};
        unsafe { SetThreadExecutionState(ES_CONTINUOUS) };
    }
}

/// Free space, in megabytes, on the drive that holds `path`.
pub fn free_disk_mb(path: &Path) -> Option<u64> {
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let wide_path = wide(path.as_os_str());
    let mut free_to_caller = 0u64;
    let mut total = 0u64;
    let mut free = 0u64;
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            wide_path.as_ptr(),
            &mut free_to_caller,
            &mut total,
            &mut free,
        )
    };
    (ok != 0).then_some(free_to_caller / (1024 * 1024))
}
