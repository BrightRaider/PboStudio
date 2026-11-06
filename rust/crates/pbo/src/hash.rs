//! Checksums: `pbo hash`, `pbo verify`, `pbo version`.
//!
//! There is no code signature (the owner decided so), which makes the checksum the way to tell that the file one holds
//! is the one that was released. The release lists the SHA-256 of `pbo.exe`; `pbo version` prints the checksum of the
//! running program so it can be compared without any other tool.

use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// `pbo hash FILE...`: one line per file, `<sha256>  <name>`, the format `sha256sum` writes and checks.
pub fn hash(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("usage: pbo hash FILE...");
        return 2;
    }
    let mut code = 0;
    for f in args {
        match sha256_file(Path::new(f)) {
            Ok(h) => println!("{h}  {f}"),
            Err(e) => {
                eprintln!("{f}: {e}");
                code = 1;
            }
        }
    }
    code
}

/// `pbo verify FILE SHA256`: exit 0 when the file is the one the checksum names.
pub fn verify(args: &[String]) -> i32 {
    let (Some(file), Some(expected)) = (args.first(), args.get(1)) else {
        eprintln!("usage: pbo verify FILE SHA256");
        return 2;
    };
    match sha256_file(Path::new(file)) {
        Ok(h) if h.eq_ignore_ascii_case(expected.trim()) => {
            println!("{file}: matches");
            0
        }
        Ok(h) => {
            println!("{file}: does NOT match (it is {h})");
            1
        }
        Err(e) => {
            eprintln!("{file}: {e}");
            1
        }
    }
}

/// `pbo version`: the version and the checksum of this very file.
pub fn version() -> i32 {
    println!("pbo {}", env!("CARGO_PKG_VERSION"));
    match std::env::current_exe().and_then(|p| sha256_file(&p)) {
        Ok(h) => println!("sha256 {h}"),
        Err(e) => eprintln!("could not read this program: {e}"),
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_known_digest_of_abc() {
        let path = std::env::temp_dir().join(format!("pbo-hash-{}.txt", std::process::id()));
        std::fs::write(&path, b"abc").unwrap();
        assert_eq!(
            sha256_file(&path).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            verify(&[
                path.display().to_string(),
                "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD".into()
            ]),
            0
        );
        assert_eq!(verify(&[path.display().to_string(), "00".into()]), 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_missing_file_is_an_error_not_a_match() {
        assert_eq!(verify(&["definitely-not-here.bin".into(), "00".into()]), 1);
    }
}
