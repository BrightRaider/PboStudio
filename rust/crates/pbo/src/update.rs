//! `pbo update [--beta]`: is there a newer release? It only looks and says; it never replaces the program.
//!
//! The list of releases comes from GitHub's public API over https (`curl.exe`, full path, a limit on the wait), and the
//! choice of what to offer is `pbo_core::version::pick_update`. Installing is the person's step: download the file from the
//! release page and compare its SHA-256 with `pbo verify`, because there is no code signature.

use pbo_core::download;
use pbo_core::version::{self, Channel, Release};
use serde_json::Value;

const RELEASES_URL: &str = "https://api.github.com/repos/BrightRaider/PboStudio/releases";

/// The tag prefix of this program's releases. The C# program has releases of its own on the same page, tagged `v1.1.3` and so on;
/// they are not this program's to offer, and a plain `v` tag here would also start the C# release workflow.
const TAG_PREFIX: &str = "rust-";

/// The releases of this program in the answer of the API: tag (without the prefix), whether it is a pre-release or a draft,
/// and the page to read it on.
pub fn parse_releases(json: &str) -> Result<Vec<(Release, String)>, String> {
    let doc: Value =
        serde_json::from_str(json).map_err(|e| format!("the answer is not JSON: {e}"))?;
    let list = doc
        .as_array()
        .ok_or("the answer is not a list of releases")?;
    Ok(list
        .iter()
        .filter_map(|r| {
            Some((
                Release {
                    tag: r["tag_name"]
                        .as_str()?
                        .strip_prefix(TAG_PREFIX)?
                        .to_string(),
                    prerelease: r["prerelease"].as_bool().unwrap_or(false),
                    draft: r["draft"].as_bool().unwrap_or(false),
                },
                r["html_url"].as_str().unwrap_or("").to_string(),
            ))
        })
        .collect())
}

fn fetch() -> Result<String, String> {
    if !download::is_allowed(RELEASES_URL, &["github.com"]) {
        return Err("the address is not on the vendor's domain: not fetched".into());
    }
    let out = pbo_os::system::run(
        "curl.exe",
        &[
            "--silent",
            "--show-error",
            "--fail",
            "--proto",
            "=https",
            "--tlsv1.2",
            "--max-time",
            "30",
            "--user-agent",
            "pbo-update-check",
            "--header",
            "Accept: application/vnd.github+json",
            RELEASES_URL,
        ],
        std::time::Duration::from_secs(60),
    )
    .ok_or("curl.exe could not be run, or took too long")?;
    if out.success {
        Ok(out.stdout_text())
    } else {
        Err(out.stderr_text().trim().to_string())
    }
}

pub fn run(args: &[String]) -> i32 {
    let channel = if args.iter().any(|a| a == "--beta") {
        Channel::Beta
    } else {
        Channel::Stable
    };
    let json = match args.windows(2).find(|w| w[0] == "--file") {
        Some(w) => std::fs::read_to_string(&w[1]).map_err(|e| format!("{}: {e}", w[1])),
        None => fetch(),
    };
    let json = match json {
        Ok(j) => j,
        Err(e) => {
            eprintln!("Could not look for a newer release: {e}");
            return 1;
        }
    };
    let releases = match parse_releases(&json) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    let current = env!("CARGO_PKG_VERSION");
    let list: Vec<Release> = releases.iter().map(|(r, _)| r.clone()).collect();
    match version::pick_update(current, &list, channel) {
        Some(tag) => {
            let page = releases
                .iter()
                .find(|(r, _)| r.tag == tag)
                .map_or("", |(_, p)| p.as_str());
            println!("pbo {current} is installed; {TAG_PREFIX}{tag} is available: {page}");
            println!("Download it there and compare its checksum: pbo verify FILE SHA256");
            0
        }
        None => {
            println!(
                "pbo {current} is the newest {} release.",
                if channel == Channel::Beta {
                    "beta or stable"
                } else {
                    "stable"
                }
            );
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"[
      {"tag_name":"rust-v1.2.0-beta.2","prerelease":true,"draft":false,"html_url":"https://github.com/x/y/releases/tag/rust-v1.2.0-beta.2"},
      {"tag_name":"rust-v1.1.9","prerelease":false,"draft":false,"html_url":"https://github.com/x/y/releases/tag/rust-v1.1.9"},
      {"tag_name":"v1.1.3","prerelease":false,"draft":false,"html_url":"https://github.com/x/y/releases/tag/v1.1.3"},
      {"tag_name":"rust-v9.9.9","draft":true},
      {"name":"no tag here"}
    ]"#;

    #[test]
    fn the_api_answer_is_read_and_only_the_releases_of_this_program_are_kept() {
        let r = parse_releases(SAMPLE).unwrap();
        // The C# release (`v1.1.3`) and the entry without a tag are left out.
        assert_eq!(r.len(), 3);
        assert_eq!(r[0].0.tag, "v1.2.0-beta.2");
        assert!(r[0].0.prerelease && !r[1].0.prerelease && r[2].0.draft);
        assert!(r[1].1.ends_with("rust-v1.1.9"));
        assert!(r.iter().all(|(rel, _)| !rel.tag.contains("rust")));
    }

    #[test]
    fn a_rust_beta_is_offered_on_the_beta_channel_only_and_the_c_sharp_release_never() {
        let r = parse_releases(SAMPLE).unwrap();
        let list: Vec<Release> = r.iter().map(|(rel, _)| rel.clone()).collect();
        assert_eq!(
            version::pick_update("1.2.0-beta.1", &list, Channel::Beta),
            Some("v1.2.0-beta.2".to_string())
        );
        assert_eq!(
            version::pick_update("1.2.0-beta.1", &list, Channel::Stable),
            None
        );
    }

    #[test]
    fn something_that_is_not_a_list_is_an_error() {
        assert!(parse_releases("{\"message\":\"rate limited\"}").is_err());
        assert!(parse_releases("not json").is_err());
    }

    #[test]
    fn the_address_is_on_the_vendors_domain() {
        assert!(download::is_allowed(RELEASES_URL, &["github.com"]));
    }
}
