//! `pbo setup`: puts the two stress engines where the program looks for them, and checks what it puts there.
//!
//! The program runs as administrator and starts what it downloads, so nothing is taken on trust (invariant I8): the
//! address must be https on the vendor's own domain (`pbo_core::download`), the file must have the SHA-256 this version
//! was made with, and only then is it unpacked. A vendor that publishes a different file under the same name is not
//! installed silently: the setup says what it got and stops, and `--accept-unverified` is the person's explicit choice.
//!
//! The hashes are those of the archives this program was developed against (Prime95 30.8 build 17 and y-cruncher
//! 0.8.7.9547b), the same builds the 1.x program fetches. Fetching uses `curl.exe` and unpacking `tar.exe`, which ship
//! with Windows 10 and later; no network code lives in this program.

use std::path::{Path, PathBuf};

use pbo_core::download;

use crate::hash::sha256_file;

struct Engine {
    name: &'static str,
    /// Where it comes from, in order, and which domains count as the vendor's.
    urls: &'static [&'static str],
    vendor_hosts: &'static [&'static str],
    sha256: &'static str,
    folder: &'static str,
    /// A file that has to exist after unpacking, relative to the engine folder.
    marker: &'static str,
}

const ENGINES: [Engine; 2] = [
    Engine {
        name: "prime95",
        urls: &[
            "https://www.mersenne.org/download/software/v30/30.8/p95v308b17.win64.zip",
            "https://download.mersenne.ca/gimps/v30/30.8/p95v308b17.win64.zip",
        ],
        vendor_hosts: &["mersenne.org", "mersenne.ca"],
        sha256: "d9475f2ff3f4a6a701abc49a86a66126cb48abd10bda6fa87039d98fa8756bca",
        folder: "prime95",
        marker: "prime95.exe",
    },
    Engine {
        name: "ycruncher",
        urls: &[
            "https://cdn.numberworld.org/y-cruncher-downloads/y-cruncher%20v0.8.7.9547b.zip",
            "https://www.numberworld.org/y-cruncher/y-cruncher%20v0.8.7.9547b.zip",
        ],
        vendor_hosts: &["numberworld.org"],
        sha256: "3be696c1cc44907f2ea73aac35de4a5d45048255308f34ed4410a743163a4f87",
        folder: "ycruncher",
        marker: "",
    },
];

const PAWNIO_URL: &str =
    "https://github.com/namazso/PawnIO.Setup/releases/download/2.2.0/PawnIO_setup.exe";
const PAWNIO_HOSTS: &[&str] = &["github.com"];

/// Puts the PawnIO installer where it can be started, after checking it, and starts it with administrator rights (the
/// system asks the person and the installer shows its own window). The checks are the 1.x program's: a bundled
/// installer next to the program (`drivers\PawnIO_setup.exe`) is checked like a downloaded one, because the folder it sits
/// in is as writable as the program itself; a download goes into a fresh folder with a name nobody can know in advance; and
/// nothing is started whose SHA-256 is not the one this version was made with.
fn install_pawnio(from: Option<&Path>) -> Result<(), String> {
    let bundled = crate::store::program_dir()
        .join("drivers")
        .join("PawnIO_setup.exe");
    let (installer, private): (PathBuf, Option<PathBuf>) = match from {
        Some(path) => (path.to_path_buf(), None),
        None if bundled.is_file()
            && sha256_file(&bundled)
                .is_ok_and(|h| h.eq_ignore_ascii_case(download::PAWNIO_INSTALLER_SHA256)) =>
        {
            println!("Using the bundled installer (the checksum matches).");
            (bundled, None)
        }
        None => {
            if !download::is_allowed(PAWNIO_URL, PAWNIO_HOSTS) {
                return Err("the address is not on the vendor's domain: not fetched".into());
            }
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos());
            let dir =
                std::env::temp_dir().join(format!("pbo-pawnio-{}-{nanos}", std::process::id()));
            // `create_dir` fails if the folder exists: nothing planted in advance is used.
            std::fs::create_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            let target = dir.join("PawnIO_setup.exe");
            let got = quiet(
                "curl.exe",
                &[
                    "--location",
                    "--fail",
                    "--silent",
                    "--show-error",
                    "--proto",
                    "=https",
                    "--proto-redir",
                    "=https",
                    "--tlsv1.2",
                    "--max-time",
                    "600",
                    "--output",
                    &target.display().to_string(),
                    PAWNIO_URL,
                ],
            );
            match got {
                Some(o) if o.success => (target, Some(dir)),
                Some(o) => {
                    let _ = std::fs::remove_dir_all(&dir);
                    return Err(format!("download failed: {}", o.stderr_text().trim()));
                }
                None => {
                    let _ = std::fs::remove_dir_all(&dir);
                    return Err("curl.exe could not be started".into());
                }
            }
        }
    };

    let hash = sha256_file(&installer).map_err(|e| format!("{}: {e}", installer.display()))?;
    if !hash.eq_ignore_ascii_case(download::PAWNIO_INSTALLER_SHA256) {
        if let Some(dir) = private {
            let _ = std::fs::remove_dir_all(dir);
        }
        return Err(format!(
            "the installer has SHA-256 {hash}, this version of pbo was made with {}. It is not started.",
            download::PAWNIO_INSTALLER_SHA256
        ));
    }
    println!("Starting the installer; Windows will ask for permission.");
    if pbo_os::run_as_administrator(&installer, "", true) {
        Ok(())
    } else {
        Err("the installer could not be started (the permission was refused, or it could not be run)".into())
    }
}

fn engines_dir(args: &[String]) -> PathBuf {
    args.windows(2)
        .find(|w| w[0] == "--engines-dir")
        .map(|w| PathBuf::from(&w[1]))
        .unwrap_or_else(|| crate::store::program_dir().join("engines"))
}

fn installed(engine: &Engine, root: &Path) -> bool {
    let dir = root.join(engine.folder);
    if engine.marker.is_empty() {
        // y-cruncher unpacks into a folder named for its version; look for its Binaries.
        std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .any(|e| e.path().join("Binaries").is_dir())
    } else {
        dir.join(engine.marker).is_file()
    }
}

/// curl and tar are Windows' own, by full path in the system folder, with a limit on the wait.
fn quiet(program: &str, args: &[&str]) -> Option<pbo_os::system::Output> {
    pbo_os::system::run(program, args, std::time::Duration::from_secs(1200))
}

/// Fetches one of the engine's addresses into `target`. Only https, only the vendor's domain, redirects included.
fn fetch(engine: &Engine, target: &Path) -> Result<(), String> {
    let mut last = String::from("no address worked");
    for url in engine.urls {
        if !download::is_allowed(url, engine.vendor_hosts) {
            return Err(format!("{url} is not on the vendor's domain: not fetched"));
        }
        let out = quiet(
            "curl.exe",
            &[
                "--location",
                "--fail",
                "--silent",
                "--show-error",
                "--proto",
                "=https",
                "--proto-redir",
                "=https",
                "--tlsv1.2",
                "--max-time",
                "900",
                "--output",
                &target.display().to_string(),
                url,
            ],
        );
        match out {
            Some(o) if o.success => return Ok(()),
            Some(o) => last = format!("{url}: {}", o.stderr_text().trim()),
            None => last = "curl.exe could not be started".into(),
        }
    }
    Err(last)
}

fn install(
    engine: &Engine,
    root: &Path,
    local: Option<&Path>,
    accept_unverified: bool,
) -> Result<(), String> {
    let work =
        std::env::temp_dir().join(format!("pbo-setup-{}-{}", engine.name, std::process::id()));
    std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let archive = work.join(format!("{}.zip", engine.name));
    let cleanup = |w: &Path| {
        let _ = std::fs::remove_dir_all(w);
    };

    let got = match local {
        Some(path) => std::fs::copy(path, &archive)
            .map(|_| ())
            .map_err(|e| format!("{}: {e}", path.display())),
        None => fetch(engine, &archive),
    };
    if let Err(e) = got {
        cleanup(&work);
        return Err(e);
    }

    let hash = sha256_file(&archive).map_err(|e| e.to_string())?;
    if !hash.eq_ignore_ascii_case(engine.sha256) {
        let message = format!(
            "{}: the file has SHA-256 {hash}, this version of pbo was made with {}",
            engine.name, engine.sha256
        );
        if !accept_unverified {
            cleanup(&work);
            return Err(format!(
                "{message}. Not installed. The vendor may have published a newer build; if you want it anyway, run again with --accept-unverified."
            ));
        }
        println!("{message}. Installing it because --accept-unverified was given.");
    }

    let target = root.join(engine.folder);
    std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
    // bsdtar refuses paths that climb out of the folder, which is what a hostile archive would need.
    let unpacked = quiet(
        "tar.exe",
        &[
            "-xf",
            &archive.display().to_string(),
            "-C",
            &target.display().to_string(),
        ],
    );
    cleanup(&work);
    match unpacked {
        Some(o) if o.success => {}
        Some(o) => {
            // bsdtar cannot write file names this Windows has no code page for (y-cruncher ships digit files named
            // with fractions). Those are skipped and reported; anything else it complains about is a real failure.
            let text = o.stderr_text();
            let other: Vec<&str> = text
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .filter(|l| !l.contains("unreadable filename") && !l.contains("Error exit delayed"))
                .collect();
            if !other.is_empty() {
                return Err(format!("unpacking failed: {}", other.join("; ")));
            }
            let skipped = text.matches("unreadable filename").count();
            println!(
                "{}: {skipped} file(s) with names this Windows cannot write were skipped (digit files, not needed)",
                engine.name
            );
        }
        None => return Err("tar.exe could not be started".into()),
    }
    if installed(engine, root) {
        Ok(())
    } else {
        Err(format!(
            "{}: unpacked, but the program was not found where it should be",
            engine.name
        ))
    }
}

/// Whether each test program is where the program looks for it.
pub fn prime95_present() -> bool {
    crate::stress::find_prime95().is_some()
}

pub fn ycruncher_present() -> bool {
    crate::stress::find_ycruncher(3).is_some()
}

pub fn run(args: &[String]) -> i32 {
    let root = engines_dir(args);
    let accept = args.iter().any(|a| a == "--accept-unverified");
    let local = args
        .windows(2)
        .find(|w| w[0] == "--from")
        .map(|w| PathBuf::from(&w[1]));
    let which: Vec<&str> = args
        .iter()
        .map(String::as_str)
        .filter(|a| {
            ENGINES.iter().any(|e| e.name == *a) || matches!(*a, "all" | "status" | "pawnio")
        })
        .collect();

    match which.first().copied() {
        None => {
            eprintln!("usage: pbo setup status | prime95 | ycruncher | all | pawnio   [--from FILE] [--engines-dir DIR] [--accept-unverified]");
            2
        }
        Some("status") => {
            for e in &ENGINES {
                println!(
                    "{:10} {}",
                    e.name,
                    if installed(e, &root) {
                        "installed"
                    } else {
                        "not installed"
                    }
                );
            }
            println!("folder: {}", root.display());
            match pbo_smu::driver_status() {
                Ok(()) => println!("PawnIO     driver present and open"),
                Err(e) => println!("PawnIO     {e}"),
            }
            0
        }
        Some("pawnio") => match install_pawnio(local.as_deref()) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("{e}");
                1
            }
        },
        Some(choice) => {
            if local.is_some() && choice == "all" {
                eprintln!("--from names one archive; choose one engine with it.");
                return 2;
            }
            let mut code = 0;
            for e in ENGINES
                .iter()
                .filter(|e| choice == "all" || e.name == choice)
            {
                if installed(e, &root) && local.is_none() {
                    println!("{}: already installed", e.name);
                    continue;
                }
                println!(
                    "{}: {}",
                    e.name,
                    if local.is_some() {
                        "installing from the archive"
                    } else {
                        "fetching"
                    }
                );
                match install(e, &root, local.as_deref(), accept) {
                    Ok(()) => {
                        println!("{}: installed in {}", e.name, root.join(e.folder).display())
                    }
                    Err(err) => {
                        eprintln!("{err}");
                        code = 1;
                    }
                }
            }
            code
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_address_is_https_on_the_vendors_own_domain() {
        for e in &ENGINES {
            assert!(!e.urls.is_empty());
            for url in e.urls {
                assert!(download::is_allowed(url, e.vendor_hosts), "{url}");
            }
        }
    }

    #[test]
    fn an_address_elsewhere_is_not_fetched() {
        let e = &ENGINES[0];
        assert!(!download::is_allowed(
            "https://mersenne.org.evil.com/p95.zip",
            e.vendor_hosts
        ));
        assert!(!download::is_allowed(
            "http://www.mersenne.org/p95.zip",
            e.vendor_hosts
        ));
    }

    #[test]
    fn the_hashes_are_sha256_in_hex() {
        for e in &ENGINES {
            assert_eq!(e.sha256.len(), 64);
            assert!(e.sha256.bytes().all(|b| b.is_ascii_hexdigit()));
        }
    }
}
