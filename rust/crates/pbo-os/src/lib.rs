//! The Windows layer: threads, CPUID, and child processes inside a job object.
//!
//! Everything that has to be put back is held by a guard that puts it back when it is dropped, however the
//! caller leaves (ADR-0006): a pinned thread goes back to where it was, a child process is ended and its
//! handles closed. That is why the release profile unwinds on panic instead of aborting.

#![cfg(windows)]

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
    CreateProcessW, GetActiveProcessorCount, GetCurrentThread, GetProcessTimes, ResumeThread,
    SetThreadGroupAffinity, TerminateProcess, CREATE_NO_WINDOW, CREATE_SUSPENDED,
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
        let mut command: Vec<u16> = wide(OsStr::new(&format!("\"{}\" {}", exe.display(), args)));

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
                CREATE_SUSPENDED | CREATE_NO_WINDOW,
                std::ptr::null(),
                std::ptr::null(),
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
