//! `pbo`: stage E0 of the Rust rebuild.
//!
//! One command, `pbo diag`: it reads what this machine reports and writes a text report that can be
//! attached to an issue. It is read-only. It writes nothing to the processor, changes no setting and
//! needs no other file than this one.

#![cfg(windows)]

mod diag;

use std::time::{Duration, Instant};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("diag") => diag::run(&args[1..]),
        // Hidden: the busy child the job-object check starts.
        Some("burn") => burn(&args[1..]),
        _ => {
            eprintln!("pbo {}  (stage E0: read-only)", env!("CARGO_PKG_VERSION"));
            eprintln!();
            eprintln!(
                "  pbo diag [--raw]   read this machine and write pbo-diag.txt next to the program"
            );
            eprintln!("                     Run it as administrator. --raw adds the whole PM table to the report.");
            2
        }
    };
    std::process::exit(code);
}

/// Keeps `threads` threads busy for `seconds`. Used only to check that a job object's affinity holds.
fn burn(args: &[String]) -> i32 {
    let threads: usize = args.first().and_then(|a| a.parse().ok()).unwrap_or(2);
    let seconds: u64 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(8);
    let end = Instant::now() + Duration::from_secs(seconds);
    let workers: Vec<_> = (0..threads)
        .map(|_| {
            std::thread::spawn(move || {
                let mut x = 1u64;
                while Instant::now() < end {
                    for _ in 0..1_000_000 {
                        x = std::hint::black_box(
                            x.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1),
                        );
                    }
                }
                x
            })
        })
        .collect();
    for w in workers {
        let _ = w.join();
    }
    0
}
