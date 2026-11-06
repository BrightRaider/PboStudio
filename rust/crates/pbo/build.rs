//! Gives the executable what a finished Windows program has: its icon in the Explorer and on the taskbar, and the
//! version information in the file's properties. The resources are written here as a `.res` file by hand and handed to the
//! linker, so no resource compiler and no crate is needed. The picture is the one the window draws (`pbo-ui/src/icon.rs`),
//! the PNG writer is the one the previews use.

#![allow(dead_code)]

#[path = "../pbo-ui/src/icon.rs"]
mod icon;
#[path = "../pbo-ui/src/png.rs"]
mod png;

use std::path::PathBuf;

const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;
const RT_VERSION: u16 = 16;
const RT_MANIFEST: u16 = 24;

fn pad4(buffer: &mut Vec<u8>) {
    while !buffer.len().is_multiple_of(4) {
        buffer.push(0);
    }
}

/// One resource of a `.res` file: a header of 32 bytes, then the data, padded to four bytes.
fn entry(kind: u16, name: u16, language: u16, data: &[u8]) -> Vec<u8> {
    let mut e = Vec::new();
    e.extend_from_slice(&(data.len() as u32).to_le_bytes());
    e.extend_from_slice(&32u32.to_le_bytes());
    e.extend_from_slice(&0xFFFFu16.to_le_bytes());
    e.extend_from_slice(&kind.to_le_bytes());
    e.extend_from_slice(&0xFFFFu16.to_le_bytes());
    e.extend_from_slice(&name.to_le_bytes());
    e.extend_from_slice(&0u32.to_le_bytes()); // data version
    e.extend_from_slice(&0x0030u16.to_le_bytes()); // movable, pure
    e.extend_from_slice(&language.to_le_bytes());
    e.extend_from_slice(&0u32.to_le_bytes()); // version
    e.extend_from_slice(&0u32.to_le_bytes()); // characteristics
    e.extend_from_slice(data);
    pad4(&mut e);
    e
}

fn wide(text: &str) -> Vec<u8> {
    text.encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(u16::to_le_bytes)
        .collect()
}

/// A block of a version resource: its length, the length of its value, its type, its key, its value, its children.
fn block(key: &str, value: &[u8], value_len: u16, text: bool, children: &[Vec<u8>]) -> Vec<u8> {
    let mut b = vec![0u8; 2];
    b.extend_from_slice(&value_len.to_le_bytes());
    b.extend_from_slice(&u16::from(text).to_le_bytes());
    b.extend_from_slice(&wide(key));
    pad4(&mut b);
    b.extend_from_slice(value);
    for child in children {
        pad4(&mut b);
        b.extend_from_slice(child);
    }
    let length = b.len() as u16;
    b[0..2].copy_from_slice(&length.to_le_bytes());
    b
}

fn string(key: &str, value: &str) -> Vec<u8> {
    let v = wide(value);
    block(key, &v, (v.len() / 2) as u16, true, &[])
}

fn version_resource(version: &str) -> Vec<u8> {
    let numbers: Vec<u32> = version
        .split(['.', '-'])
        .map_while(|p| p.parse::<u32>().ok())
        .collect();
    let part = |i: usize| numbers.get(i).copied().unwrap_or(0);
    let prerelease = version.contains('-');
    let mut fixed = Vec::new();
    for word in [
        0xFEEF_04BDu32,
        0x0001_0000,
        (part(0) << 16) | part(1),
        part(2) << 16,
        (part(0) << 16) | part(1),
        part(2) << 16,
        0x3F,
        if prerelease { 0x2 } else { 0 },
        0x0004_0004,
        0x1,
        0,
        0,
        0,
    ] {
        fixed.extend_from_slice(&word.to_le_bytes());
    }
    let strings = [
        ("CompanyName", "PboStudio"),
        (
            "FileDescription",
            "PboStudio: Curve Optimizer tuning and stress testing for AMD Ryzen",
        ),
        ("FileVersion", version),
        ("InternalName", "pbo"),
        ("LegalCopyright", "GPL-3.0"),
        ("OriginalFilename", "pbo.exe"),
        ("ProductName", "PboStudio"),
        ("ProductVersion", version),
    ];
    let table = block(
        "040904B0",
        &[],
        0,
        true,
        &strings
            .iter()
            .map(|(k, v)| string(k, v))
            .collect::<Vec<_>>(),
    );
    let string_info = block("StringFileInfo", &[], 0, true, &[table]);
    let translation = block("Translation", &0x04B0_0409u32.to_le_bytes(), 4, false, &[]);
    let var_info = block("VarFileInfo", &[], 0, true, &[translation]);
    block(
        "VS_VERSION_INFO",
        &fixed,
        fixed.len() as u16,
        false,
        &[string_info, var_info],
    )
}

fn manifest(version: &str) -> String {
    let four: Vec<String> = version
        .split(['.', '-'])
        .map_while(|p| p.parse::<u32>().ok().map(|n| n.to_string()))
        .collect();
    let numeric = format!(
        "{}.{}.{}.0",
        four.first().map_or("0", String::as_str),
        four.get(1).map_or("0", String::as_str),
        four.get(2).map_or("0", String::as_str)
    );
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity type="win32" name="PboStudio.pbo" version="{numeric}"/>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}}"/>
    </application>
  </compatibility>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <longPathAware xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">true</longPathAware>
    </windowsSettings>
  </application>
</assembly>
"#
    )
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../pbo-ui/src/icon.rs");
    println!("cargo:rerun-if-changed=../pbo-ui/src/png.rs");
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        return;
    }
    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".into());

    // The marker every .res file starts with: an empty resource.
    let mut res: Vec<u8> = Vec::new();
    for word in [0u32, 32, 0x0000_FFFF, 0x0000_FFFF, 0, 0, 0, 0] {
        res.extend_from_slice(&word.to_le_bytes());
    }

    // The icon: the same picture at four sizes, as PNG (Windows takes PNG for icons since Vista).
    let sizes = [16usize, 32, 48, 256];
    let mut group = Vec::new();
    group.extend_from_slice(&0u16.to_le_bytes());
    group.extend_from_slice(&1u16.to_le_bytes());
    group.extend_from_slice(&(sizes.len() as u16).to_le_bytes());
    for (i, size) in sizes.iter().enumerate() {
        let image = png::encode(*size as u32, *size as u32, &icon::rgba(*size));
        let id = (i + 1) as u16;
        res.extend(entry(RT_ICON, id, 0, &image));
        group.push(if *size >= 256 { 0 } else { *size as u8 });
        group.push(if *size >= 256 { 0 } else { *size as u8 });
        group.extend_from_slice(&[0, 0]);
        group.extend_from_slice(&1u16.to_le_bytes());
        group.extend_from_slice(&32u16.to_le_bytes());
        group.extend_from_slice(&(image.len() as u32).to_le_bytes());
        group.extend_from_slice(&id.to_le_bytes());
    }
    res.extend(entry(RT_GROUP_ICON, 1, 0, &group));
    res.extend(entry(RT_VERSION, 1, 0x0409, &version_resource(&version)));
    res.extend(entry(RT_MANIFEST, 1, 0x0409, manifest(&version).as_bytes()));

    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR")).join("pbo.res");
    std::fs::write(&out, res).expect("write pbo.res");
    println!("cargo:rustc-link-arg-bins={}", out.display());
}
