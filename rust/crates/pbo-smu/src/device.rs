//! The PawnIO device: load a module, call its functions.

use std::fmt;

use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::IO::DeviceIoControl;

const DEVICE_TYPE: u32 = 41394 << 16;
const IOCTL_PIO_EXECUTE_FN: u32 = 0x841 << 2;
const IOCTL_PIO_LOAD_BINARY: u32 = 0x821 << 2;
/// A function name is sent in a fixed 32-byte field, NUL padded.
const FN_NAME_LENGTH: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The device could not be opened: PawnIO is not installed or the caller is not elevated. Carries the Win32 code.
    DriverUnavailable(u32),
    /// The driver refused the module (not signed, wrong build). Carries the Win32 code.
    ModuleRejected(u32),
    /// A function call failed. Carries the HRESULT.
    CallFailed(i32),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::DriverUnavailable(code) => write!(
                f,
                "PawnIO device could not be opened (Win32 error {code}): not installed, or not running as administrator"
            ),
            Error::ModuleRejected(code) => write!(f, "PawnIO refused the module (Win32 error {code})"),
            Error::CallFailed(hr) => write!(f, "module call failed (HRESULT 0x{:08X})", *hr as u32),
        }
    }
}

impl std::error::Error for Error {}

/// `HRESULT_FROM_WIN32`.
fn hresult(code: u32) -> i32 {
    if code == 0 {
        0
    } else {
        ((code & 0xFFFF) | 0x8007_0000) as i32
    }
}

/// Whether the driver is there and this program may open it, without loading anything.
pub fn driver_status() -> Result<(), Error> {
    let path: Vec<u16> = r"\\?\GLOBALROOT\Device\PawnIO"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        )
    };
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        return Err(Error::DriverUnavailable(unsafe { GetLastError() }));
    }
    unsafe { CloseHandle(handle) };
    Ok(())
}

/// One loaded module, on its own handle. Dropping it closes the handle and unloads the module.
pub struct Module {
    handle: HANDLE,
}

// The handle is used for synchronous IOCTLs only, one at a time per call.
unsafe impl Send for Module {}

impl Drop for Module {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.handle);
        }
    }
}

impl Module {
    /// Opens the device and loads `binary` into it.
    pub fn load(binary: &[u8]) -> Result<Module, Error> {
        let path: Vec<u16> = r"\\?\GLOBALROOT\Device\PawnIO"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let handle = unsafe {
            CreateFileW(
                path.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            )
        };
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            return Err(Error::DriverUnavailable(unsafe { GetLastError() }));
        }

        let mut returned = 0u32;
        let ok = unsafe {
            DeviceIoControl(
                handle,
                DEVICE_TYPE | IOCTL_PIO_LOAD_BINARY,
                binary.as_ptr().cast(),
                binary.len() as u32,
                std::ptr::null_mut(),
                0,
                &mut returned,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            let code = unsafe { GetLastError() };
            unsafe { CloseHandle(handle) };
            return Err(Error::ModuleRejected(code));
        }
        Ok(Module { handle })
    }

    /// Calls a named function of the module. `input` and the result are 64-bit words, as the driver defines.
    /// The result holds as many words as the module returned, which can be fewer than `out_words`.
    pub fn execute(&self, name: &str, input: &[i64], out_words: usize) -> Result<Vec<i64>, Error> {
        let mut request = vec![0u8; FN_NAME_LENGTH + input.len() * 8];
        let name_bytes = name.as_bytes();
        let n = name_bytes.len().min(FN_NAME_LENGTH - 1);
        request[..n].copy_from_slice(&name_bytes[..n]);
        for (i, word) in input.iter().enumerate() {
            request[FN_NAME_LENGTH + i * 8..FN_NAME_LENGTH + (i + 1) * 8]
                .copy_from_slice(&word.to_le_bytes());
        }

        let mut output = vec![0u8; out_words * 8];
        let mut returned = 0u32;
        let ok = unsafe {
            DeviceIoControl(
                self.handle,
                DEVICE_TYPE | IOCTL_PIO_EXECUTE_FN,
                request.as_ptr().cast(),
                request.len() as u32,
                output.as_mut_ptr().cast(),
                output.len() as u32,
                &mut returned,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(Error::CallFailed(hresult(unsafe { GetLastError() })));
        }

        let words = (returned as usize / 8).min(out_words);
        Ok((0..words)
            .map(|i| i64::from_le_bytes(output[i * 8..(i + 1) * 8].try_into().expect("8 bytes")))
            .collect())
    }
}
