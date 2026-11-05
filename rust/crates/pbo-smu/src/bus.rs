//! The system-wide PCI bus mutex that monitoring tools share.
//!
//! HWiNFO, ZenTimings and others take `Global\Access_PCI` around SMN and SMU access, so that two programs
//! do not talk to the SMU at once. A program that skips it works until it collides with one that does not.

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0};
use windows_sys::Win32::System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject};

/// Held while the guard lives. Released when it is dropped.
pub struct PciBusLock {
    mutex: HANDLE,
}

impl PciBusLock {
    /// Waits up to `timeout_ms` for the mutex. `None` on timeout or when it cannot be opened.
    pub fn acquire(timeout_ms: u32) -> Option<PciBusLock> {
        let name: Vec<u16> = r"Global\Access_PCI"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let mutex = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if mutex.is_null() {
            return None;
        }
        let wait = unsafe { WaitForSingleObject(mutex, timeout_ms) };
        // An abandoned mutex (its owner died) is ours now, which is the right outcome.
        if wait == WAIT_OBJECT_0 || wait == WAIT_ABANDONED {
            Some(PciBusLock { mutex })
        } else {
            unsafe { CloseHandle(mutex) };
            None
        }
    }
}

impl Drop for PciBusLock {
    fn drop(&mut self) {
        unsafe {
            ReleaseMutex(self.mutex);
            CloseHandle(self.mutex);
        }
    }
}
