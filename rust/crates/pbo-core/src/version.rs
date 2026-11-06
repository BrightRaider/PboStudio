//! Which release is newer, and which one an update check should offer (ADR-0010).
//!
//! Versions are `major.minor.patch` with an optional `-prerelease` part, written with or without a leading `v`. A
//! pre-release is older than the release it leads up to (`1.2.0-beta.1` < `1.2.0`). The stable channel only ever offers
//! releases; the beta channel offers pre-releases too, and never offers something older than what is installed. Tests:
//! the unit tests below.

use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    /// Dot-separated identifiers after the hyphen, empty for a release.
    pub pre: Vec<String>,
}

pub fn parse(text: &str) -> Option<Version> {
    let t = text.trim();
    let t = t.strip_prefix(['v', 'V']).unwrap_or(t);
    // Build metadata after a plus sign does not take part in the order.
    let t = t.split('+').next()?;
    let (core, pre) = match t.split_once('-') {
        Some((_, "")) => return None,
        Some((c, p)) => (c, p),
        None => (t, ""),
    };
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().map_or(Some(0), |p| p.parse().ok())?;
    let patch = parts.next().map_or(Some(0), |p| p.parse().ok())?;
    if parts.next().is_some() {
        return None;
    }
    let pre: Vec<String> = if pre.is_empty() {
        Vec::new()
    } else {
        pre.split('.').map(str::to_string).collect()
    };
    if pre.iter().any(String::is_empty) {
        return None;
    }
    Some(Version {
        major,
        minor,
        patch,
        pre,
    })
}

impl Ord for Version {
    fn cmp(&self, other: &Version) -> Ordering {
        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(|| match (self.pre.is_empty(), other.pre.is_empty()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => compare_pre(&self.pre, &other.pre),
            })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Version) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Pre-release identifiers: numbers compare as numbers and are lower than words, words compare as text, and a shorter
/// list is lower when everything before it is equal.
fn compare_pre(a: &[String], b: &[String]) -> Ordering {
    for (x, y) in a.iter().zip(b) {
        let ord = match (x.parse::<u64>(), y.parse::<u64>()) {
            (Ok(n), Ok(m)) => n.cmp(&m),
            (Ok(_), Err(_)) => Ordering::Less,
            (Err(_), Ok(_)) => Ordering::Greater,
            (Err(_), Err(_)) => x.cmp(y),
        };
        if ord != Ordering::Equal {
            return ord;
        }
    }
    a.len().cmp(&b.len())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Stable,
    Beta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub tag: String,
    pub prerelease: bool,
    pub draft: bool,
}

/// The tag of the newest release this channel offers that is newer than `current`, if there is one. Drafts are never
/// offered; on the stable channel neither are pre-releases, whatever their tag looks like.
pub fn pick_update(current: &str, releases: &[Release], channel: Channel) -> Option<String> {
    let installed = parse(current)?;
    releases
        .iter()
        .filter(|r| !r.draft)
        .filter(|r| channel == Channel::Beta || !r.prerelease)
        .filter_map(|r| parse(&r.tag).map(|v| (v, &r.tag)))
        .filter(|(v, _)| *v > installed)
        .filter(|(v, _)| channel == Channel::Beta || v.pre.is_empty())
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, tag)| tag.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        parse(s).unwrap()
    }

    #[test]
    fn a_prerelease_is_older_than_its_release() {
        assert!(v("1.2.0-beta.1") < v("1.2.0"));
        assert!(v("1.2.0-beta.1") > v("1.1.9"));
        assert!(v("1.2.0-beta.2") > v("1.2.0-beta.1"));
        assert!(v("1.2.0-beta.10") > v("1.2.0-beta.9"));
        assert!(v("1.2.0-rc") > v("1.2.0-beta.5"));
        assert!(v("1.2.0-beta") < v("1.2.0-beta.1"));
    }

    #[test]
    fn a_leading_v_and_missing_parts_are_fine_and_nonsense_is_not() {
        assert_eq!(v("v1.1.3"), v("1.1.3"));
        assert_eq!(v("2"), v("2.0.0"));
        assert!(parse("1.2.3.4").is_none());
        assert!(parse("latest").is_none());
        assert!(parse("1.2.0-").is_none());
        assert_eq!(v("1.2.0+build5"), v("1.2.0"));
    }

    fn r(tag: &str, pre: bool) -> Release {
        Release {
            tag: tag.into(),
            prerelease: pre,
            draft: false,
        }
    }

    #[test]
    fn the_stable_channel_offers_releases_only_and_the_beta_channel_both() {
        let list = [
            r("v1.1.3", false),
            r("v1.2.0-beta.1", true),
            r("v1.1.4", false),
        ];
        assert_eq!(
            pick_update("1.1.3", &list, Channel::Stable),
            Some("v1.1.4".into())
        );
        let list = [r("v1.1.3", false), r("v1.2.0-beta.1", true)];
        assert_eq!(pick_update("1.1.3", &list, Channel::Stable), None);
        assert_eq!(
            pick_update("1.1.3", &list, Channel::Beta),
            Some("v1.2.0-beta.1".into())
        );
    }

    #[test]
    fn nothing_older_or_equal_is_offered_and_drafts_never() {
        let list = [r("v1.0.0", false), r("v1.1.3", false)];
        assert_eq!(pick_update("1.1.3", &list, Channel::Beta), None);
        let mut draft = r("v9.9.9", false);
        draft.draft = true;
        assert_eq!(pick_update("1.1.3", &[draft], Channel::Beta), None);
    }

    #[test]
    fn a_prerelease_tag_on_a_release_flag_is_still_not_stable() {
        // A tag that says beta but is not flagged: the stable channel looks at the tag too.
        let list = [r("v2.0.0-beta.1", false)];
        assert_eq!(pick_update("1.1.3", &list, Channel::Stable), None);
    }

    #[test]
    fn an_unreadable_installed_version_offers_nothing() {
        assert_eq!(
            pick_update("weird", &[r("v9.0.0", false)], Channel::Stable),
            None
        );
        assert_eq!(
            pick_update("1.0.0", &[r("nonsense", false)], Channel::Stable),
            None
        );
    }

    #[test]
    fn the_newest_of_several_is_chosen() {
        let list = [r("v1.2.0", false), r("v1.10.0", false), r("v1.9.0", false)];
        assert_eq!(
            pick_update("1.1.3", &list, Channel::Stable),
            Some("v1.10.0".into())
        );
    }
}
