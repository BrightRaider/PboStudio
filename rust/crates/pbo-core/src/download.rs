//! What the program will fetch and run (invariant I8).
//!
//! It runs as administrator and starts what it downloads: stress engines that execute with those rights for
//! hours, and the installer of a kernel driver. So: https only, and only on a domain the vendor already uses.
//! A link found on a page is not trusted because the page named it. The root of a host is its last two labels:
//! good enough for the few vendor domains involved, not a public-suffix lookup, and it does not pretend to be.
//! Hashing a file needs a hash function and lives in the operating system layer. Vectors:
//! `spec/vectors/download-policy.json`.

/// PawnIO_setup.exe 2.2.0, as GitHub lists it for the release asset.
pub const PAWNIO_INSTALLER_SHA256: &str =
    "1f519a22e47187f70a1379a48ca604981c4fcf694f4e65b734aaa74a9fba3032";

/// The host of an https URL, lower-cased, without user information and port. `None` for anything else:
/// another scheme, no scheme, a scheme-relative address, an empty host.
fn https_host(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    if !scheme.eq_ignore_ascii_case("https") {
        return None;
    }
    let authority = rest.split(['/', '?', '#']).next()?;
    // User information in front of an '@' is not the host: "cdn.vendor.org@evil.com" is evil.com.
    let host_port = authority.rsplit('@').next()?;
    let host = match host_port.rsplit_once(':') {
        Some((h, port)) if !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) => h,
        _ => host_port,
    };
    (!host.is_empty()).then(|| host.to_lowercase())
}

/// The last two labels of a host: `cdn.numberworld.org` and `www.numberworld.org` are one owner,
/// `numberworld.org.evil.com` and `evilnumberworld.org` are not.
pub fn registrable_root(host: &str) -> String {
    let lower = host.trim_matches('.').to_lowercase();
    let labels: Vec<&str> = lower.split('.').filter(|l| !l.is_empty()).collect();
    if labels.len() <= 2 {
        labels.join(".")
    } else {
        labels[labels.len() - 2..].join(".")
    }
}

/// Whether to fetch `url` for the vendor whose hosts are `allowed_hosts`.
pub fn is_allowed(url: &str, allowed_hosts: &[&str]) -> bool {
    let Some(host) = https_host(url) else {
        return false;
    };
    let root = registrable_root(&host);
    allowed_hosts
        .iter()
        .any(|h| registrable_root(h).eq_ignore_ascii_case(&root))
}
