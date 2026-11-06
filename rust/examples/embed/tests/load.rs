//! The loader test for the example guest (gate G8): builds the module, loads it the way a host would, and drives it
//! through the C table. It checks that messages, events and the status line work, that a panic in the adapter does not
//! take the loader down, and that destroying the guest leaves no thread running in the unloaded code.

use std::ffi::{c_char, CString};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use libloading::{Library, Symbol};

#[repr(C)]
struct ModuleV1 {
    abi_version: u32,
    create: extern "C" fn() -> *mut core::ffi::c_void,
    destroy: extern "C" fn(*mut core::ffi::c_void),
    send: extern "C" fn(*mut core::ffi::c_void, *const c_char, *mut c_char, usize) -> i32,
}

/// Builds the module with the same profile this test runs under and returns the path of the library.
fn build_module() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let profile_dir = exe.parent().unwrap().parent().unwrap().to_path_buf();
    let release = profile_dir.file_name().is_some_and(|n| n == "release");
    let mut cmd = std::process::Command::new(env!("CARGO"));
    cmd.args(["build", "-p", "pbo-embed-example", "--offline"]);
    if release {
        cmd.arg("--release");
    }
    let status = cmd.status().expect("cargo build");
    assert!(status.success(), "building the module failed");
    profile_dir.join("pbo_embed_example.dll")
}

struct Loaded {
    lib: Option<Library>,
    table: *const ModuleV1,
    guest: *mut core::ffi::c_void,
}

impl Loaded {
    fn open() -> Loaded {
        let path = build_module();
        let lib =
            unsafe { Library::new(&path) }.unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let table = {
            let symbol: Symbol<extern "C" fn() -> *const ModuleV1> =
                unsafe { lib.get(b"pbo_module_v1\0") }.expect("the exported symbol");
            symbol()
        };
        assert!(!table.is_null());
        let guest = (unsafe { &*table }.create)();
        assert!(!guest.is_null());
        Loaded {
            lib: Some(lib),
            table,
            guest,
        }
    }

    fn send_with(&self, message: &str, capacity: usize) -> (i32, String) {
        let msg = CString::new(message).unwrap();
        let mut buffer = vec![0u8; capacity];
        let n = (unsafe { &*self.table }.send)(
            self.guest,
            msg.as_ptr(),
            buffer.as_mut_ptr().cast(),
            capacity,
        );
        let text = if n >= 0 {
            String::from_utf8_lossy(&buffer[..n as usize]).into_owned()
        } else {
            String::new()
        };
        (n, text)
    }

    fn send(&self, message: &str) -> String {
        let (n, text) = self.send_with(message, 4096);
        assert!(n >= 0, "send returned {n} for {message}");
        text
    }

    fn wait_for_state(&self, wanted: &str) -> String {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let status = self.send("{\"type\":\"status\"}");
            if status.contains(&format!("\"state\":\"{wanted}\"")) {
                return status;
            }
            assert!(
                Instant::now() < deadline,
                "never reached {wanted}; last: {status}"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn destroy(&mut self) {
        (unsafe { &*self.table }.destroy)(self.guest);
        self.guest = std::ptr::null_mut();
    }
}

impl Drop for Loaded {
    fn drop(&mut self) {
        if !self.guest.is_null() {
            self.destroy();
        }
        self.lib.take();
    }
}

#[test]
fn the_module_loads_and_speaks_its_version() {
    let m = Loaded::open();
    assert_eq!(unsafe { &*m.table }.abi_version, 1);
}

#[test]
fn the_plan_is_read_and_changed_by_message() {
    let m = Loaded::open();
    let plan = m.send("{\"type\":\"plan.get\"}");
    assert!(
        plan.contains("\"schema_version\":1") && plan.contains("\"seconds\":\"60\""),
        "{plan}"
    );
    assert_eq!(
        m.send("{\"type\":\"plan.set\",\"key\":\"seconds\",\"value\":\"120\"}"),
        "{\"ok\":true}"
    );
    assert!(m
        .send("{\"type\":\"plan.get\"}")
        .contains("\"seconds\":\"120\""));
    // A wrong value is an answer, not a crash, and changes nothing.
    let bad = m.send("{\"type\":\"plan.set\",\"key\":\"seconds\",\"value\":\"5\"}");
    assert!(bad.contains("\"ok\":false"), "{bad}");
    assert!(m
        .send("{\"type\":\"plan.get\"}")
        .contains("\"seconds\":\"120\""));
    assert!(m.send("not json").contains("\"ok\":false"));
    assert!(m
        .send("{\"type\":\"nope\"}")
        .contains("unknown message type"));
}

#[test]
fn a_run_goes_through_the_states_with_a_status_line_in_either_language() {
    let m = Loaded::open();
    assert!(m
        .send("{\"type\":\"status\"}")
        .contains("\"line\":\"Ready\""));
    assert_eq!(
        m.send("{\"type\":\"lang\",\"value\":\"de\"}"),
        "{\"ok\":true}"
    );
    assert!(m
        .send("{\"type\":\"status\"}")
        .contains("\"line\":\"Bereit\""));
    assert_eq!(
        m.send("{\"type\":\"lang\",\"value\":\"en\"}"),
        "{\"ok\":true}"
    );

    assert_eq!(m.send("{\"type\":\"start\"}"), "{\"ok\":true}");
    // A second start is an error, never a second run.
    assert!(m.send("{\"type\":\"start\"}").contains("\"ok\":false"));
    // The plan is locked while a run goes on.
    assert!(m
        .send("{\"type\":\"plan.set\",\"key\":\"smt\",\"value\":\"true\"}")
        .contains("\"ok\":false"));

    m.wait_for_state("Finished");
    let events = m.send("{\"type\":\"events\"}");
    // The state loop above drained the events into the state; what is left is what came after.
    assert!(events.contains("\"ok\":true"));
    assert_eq!(m.send("{\"type\":\"dismiss\"}"), "{\"ok\":true}");
    assert!(m
        .send("{\"type\":\"status\"}")
        .contains("\"state\":\"Ready\""));
}

#[test]
fn a_run_can_be_stopped_and_ends_halted() {
    let m = Loaded::open();
    m.send("{\"type\":\"start\"}");
    m.wait_for_state("Running");
    assert_eq!(m.send("{\"type\":\"stop\"}"), "{\"ok\":true}");
    let status = m.send("{\"type\":\"status\"}");
    assert!(status.contains("\"state\":\"Halted\""), "{status}");
    // Nothing is left to stop.
    assert!(m.send("{\"type\":\"stop\"}").contains("\"ok\":false"));
}

#[test]
fn a_buffer_that_is_too_small_is_refused_and_a_null_is_refused() {
    let m = Loaded::open();
    let (n, _) = m.send_with("{\"type\":\"plan.get\"}", 8);
    assert_eq!(n, -2);
    let msg = CString::new("{\"type\":\"status\"}").unwrap();
    let rc = (unsafe { &*m.table }.send)(m.guest, msg.as_ptr(), std::ptr::null_mut(), 0);
    assert_eq!(rc, -1);
    // Still usable.
    assert!(m.send("{\"type\":\"status\"}").contains("\"ok\":true"));
}

#[test]
fn a_panic_in_the_adapter_does_not_take_the_loader_down() {
    let m = Loaded::open();
    let (n, _) = m.send_with("{\"type\":\"test.panic\"}", 256);
    assert_eq!(n, -3);
    assert!(m.send("{\"type\":\"status\"}").contains("\"ok\":true"));
}

#[test]
fn destroying_the_guest_ends_its_threads_before_the_library_goes() {
    let mut m = Loaded::open();
    m.send("{\"type\":\"start\"}");
    m.wait_for_state("Running");
    m.destroy();
    // Unload the code. A thread still running in it would crash the process now or within a few steps.
    m.lib.take();
    std::thread::sleep(Duration::from_millis(500));
}
