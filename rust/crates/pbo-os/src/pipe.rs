//! A named pipe between the program without rights and the helper with them (ADR-0014, mode B).
//!
//! One request, one answer, one connection: the client writes a message, the server writes the answer and hangs up.
//! That keeps the protocol stateless, so several fronts (the window, a host's module) can use the helper at once and a
//! front that dies in the middle leaves nothing half open. The pipe accepts the current user only, and no remote
//! client. Messages are flat JSON (`pbo_app::wire`).

use std::ffi::OsStr;
use std::io::{Read, Write};
use std::os::windows::ffi::OsStrExt;
use std::sync::Arc;

use windows_sys::Win32::Foundation::{CloseHandle, LocalFree, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{
    GetTokenInformation, TokenUser, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER,
};
use windows_sys::Win32::Storage::FileSystem::{ReadFile, WriteFile, PIPE_ACCESS_DUPLEX};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_MESSAGE,
    PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_MESSAGE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

const BUFFER: u32 = 64 * 1024;

pub fn pipe_path(name: &str) -> String {
    format!(r"\\.\pipe\pbo-helper-{name}")
}

fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// The current user's SID as text, from the process token.
fn current_user_sid() -> Option<String> {
    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return None;
        }
        let mut size = 0u32;
        GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut size);
        let mut buffer = vec![0u8; size as usize];
        let ok = GetTokenInformation(
            token,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            size,
            &mut size,
        );
        CloseHandle(token);
        if ok == 0 {
            return None;
        }
        let user = &*(buffer.as_ptr().cast::<TOKEN_USER>());
        let mut text: *mut u16 = std::ptr::null_mut();
        if ConvertSidToStringSidW(user.User.Sid, &mut text) == 0 {
            return None;
        }
        let mut len = 0;
        while *text.add(len) != 0 {
            len += 1;
        }
        let sid = String::from_utf16_lossy(std::slice::from_raw_parts(text, len));
        LocalFree(text.cast());
        Some(sid)
    }
}

/// Answers a message with a message. Shared between connections, so it is behind a lock by the caller.
pub trait Handler: Send {
    fn handle(&mut self, message: &str) -> String;
}

/// Serves `handler` on the pipe forever (or until the handler's process ends). Each connection runs on its own thread.
pub fn serve(name: &str, handler: Arc<std::sync::Mutex<dyn Handler>>) -> Result<(), String> {
    let sid = current_user_sid().ok_or("could not read the user of this process")?;
    // Protected DACL: this user, and the system. Nobody else, so no other account on the machine can drive the helper.
    let sddl = wide(&format!("D:P(A;;GA;;;{sid})(A;;GA;;;SY)"));
    let mut descriptor = std::ptr::null_mut();
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            std::ptr::null_mut(),
        )
    } == 0
    {
        return Err("could not build the pipe's access list".into());
    }
    let attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor,
        bInheritHandle: 0,
    };
    let path = wide(&pipe_path(name));
    loop {
        let pipe = unsafe {
            CreateNamedPipeW(
                path.as_ptr(),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                PIPE_UNLIMITED_INSTANCES,
                BUFFER,
                BUFFER,
                0,
                &attributes,
            )
        };
        if pipe == INVALID_HANDLE_VALUE {
            unsafe { LocalFree(descriptor) };
            return Err(
                "the pipe could not be created (is another helper running under this name?)".into(),
            );
        }
        // ERROR_PIPE_CONNECTED (535) means the client got there first, which is a connection all the same.
        let connected = unsafe { ConnectNamedPipe(pipe, std::ptr::null_mut()) } != 0
            || std::io::Error::last_os_error().raw_os_error() == Some(535);
        if !connected {
            unsafe { CloseHandle(pipe) };
            continue;
        }
        let handler = handler.clone();
        let raw = pipe as usize;
        std::thread::spawn(move || {
            let pipe = raw as HANDLE;
            let mut buffer = vec![0u8; BUFFER as usize];
            let mut read = 0u32;
            let ok = unsafe {
                ReadFile(
                    pipe,
                    buffer.as_mut_ptr().cast(),
                    BUFFER,
                    &mut read,
                    std::ptr::null_mut(),
                )
            };
            if ok != 0 {
                let request = String::from_utf8_lossy(&buffer[..read as usize]).into_owned();
                let answer = match handler.lock() {
                    Ok(mut h) => std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        h.handle(&request)
                    }))
                    .unwrap_or_else(|_| {
                        "{\"ok\":false,\"error\":\"the helper failed on this message\"}".into()
                    }),
                    Err(_) => {
                        "{\"ok\":false,\"error\":\"the helper is in an unusable state\"}".into()
                    }
                };
                let bytes = answer.as_bytes();
                let mut written = 0u32;
                unsafe {
                    WriteFile(
                        pipe,
                        bytes.as_ptr().cast(),
                        bytes.len().min(BUFFER as usize) as u32,
                        &mut written,
                        std::ptr::null_mut(),
                    );
                }
            }
            unsafe {
                DisconnectNamedPipe(pipe);
                CloseHandle(pipe);
            }
        });
    }
}

/// Sends one message to the helper and returns its answer.
pub fn call(name: &str, message: &str) -> Result<String, String> {
    let mut pipe = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(pipe_path(name))
        .map_err(|e| format!("no helper answers on '{name}': {e}"))?;
    pipe.write_all(message.as_bytes())
        .map_err(|e| format!("the helper did not take the message: {e}"))?;
    let mut buffer = vec![0u8; BUFFER as usize];
    let n = pipe
        .read(&mut buffer)
        .map_err(|e| format!("the helper did not answer: {e}"))?;
    Ok(String::from_utf8_lossy(&buffer[..n]).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Echo;
    impl Handler for Echo {
        fn handle(&mut self, message: &str) -> String {
            if message.contains("boom") {
                panic!("on purpose");
            }
            format!("{{\"ok\":true,\"echo\":{}}}", message.len())
        }
    }

    fn start(name: &'static str) {
        std::thread::spawn(move || {
            let _ = serve(name, Arc::new(std::sync::Mutex::new(Echo)));
        });
        // The server creates its pipe on its own thread; wait until it answers.
        for _ in 0..100 {
            if call(name, "{}").is_ok() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        panic!("the pipe never came up");
    }

    #[test]
    fn a_message_gets_an_answer_and_many_connections_work() {
        start("unit-test-a");
        for i in 0..20 {
            let m = format!("{{\"n\":{i}}}");
            let a = call("unit-test-a", &m).unwrap();
            assert_eq!(a, format!("{{\"ok\":true,\"echo\":{}}}", m.len()));
        }
    }

    #[test]
    fn a_panic_in_the_handler_is_an_answer_not_the_end_of_the_helper() {
        start("unit-test-b");
        let a = call("unit-test-b", "boom").unwrap();
        assert!(a.contains("\"ok\":false"), "{a}");
        // The lock was poisoned by the panic; the helper says so instead of hanging, and the next call still answers.
        let b = call("unit-test-b", "{}").unwrap();
        assert!(
            b.contains("\"ok\":false") || b.contains("\"ok\":true"),
            "{b}"
        );
    }

    #[test]
    fn nobody_answers_on_a_name_nobody_serves() {
        assert!(call("nobody-serves-this", "{}").is_err());
    }
}
