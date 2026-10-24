//! Release versions: `MAJOR.MINOR.PATCH`, maybe with a `-pre.release` part, ordered as semver orders them
//! (a pre-release comes before its release). A release's tag is its version with a `v` in front.

use std::cmp::Ordering;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    core: [u64; 3],
    /// The pre-release identifiers, dot-separated; empty for a release.
    pre: Vec<String>,
}

impl Version {
    /// `1.2.3` or `1.2.3-beta.1`, with or without a leading `v` (a tag). Build metadata (`+...`) isn't
    /// taken: two builds of one version would compare equal.
    pub fn parse(text: &str) -> Result<Version, String> {
        let bad = || format!("{text:?} isn't a version (MAJOR.MINOR.PATCH, maybe -PRE)");
        let s = text.strip_prefix('v').unwrap_or(text);
        let (core, pre) = match s.split_once('-') {
            Some((core, pre)) => (core, Some(pre)),
            None => (s, None),
        };
        let mut parts = core.split('.');
        let mut number = || -> Result<u64, String> {
            let p = parts.next().ok_or_else(bad)?;
            // No leading zeros, as semver says, so a version has one spelling.
            if p.is_empty()
                || !p.bytes().all(|b| b.is_ascii_digit())
                || (p.len() > 1 && p.starts_with('0'))
            {
                return Err(bad());
            }
            p.parse().map_err(|_| bad())
        };
        let core = [number()?, number()?, number()?];
        if parts.next().is_some() {
            return Err(bad());
        }
        let pre: Vec<String> = match pre {
            None => Vec::new(),
            Some(pre) => pre.split('.').map(str::to_owned).collect(),
        };
        if pre
            .iter()
            .any(|id| id.is_empty() || !id.bytes().all(|b| b.is_ascii_alphanumeric()))
        {
            return Err(bad());
        }
        Ok(Version { core, pre })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let [major, minor, patch] = self.core;
        write!(f, "{major}.{minor}.{patch}")?;
        if !self.pre.is_empty() {
            write!(f, "-{}", self.pre.join("."))?;
        }
        Ok(())
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Version) -> Ordering {
        self.core.cmp(&other.core).then_with(|| {
            match (self.pre.is_empty(), other.pre.is_empty()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => {
                    for (a, b) in self.pre.iter().zip(&other.pre) {
                        let order = match (a.parse::<u64>(), b.parse::<u64>()) {
                            (Ok(a), Ok(b)) => a.cmp(&b),
                            // Numeric identifiers come before alphanumeric ones.
                            (Ok(_), Err(_)) => Ordering::Less,
                            (Err(_), Ok(_)) => Ordering::Greater,
                            (Err(_), Err(_)) => a.cmp(b),
                        };
                        if order != Ordering::Equal {
                            return order;
                        }
                    }
                    self.pre.len().cmp(&other.pre.len())
                }
            }
        })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Version) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    #[test]
    fn parses_tags_and_versions() {
        assert_eq!(v("v1.2.3"), v("1.2.3"));
        assert_eq!(v("1.2.3-beta.1").to_string(), "1.2.3-beta.1");
        for bad in [
            "",
            "1",
            "1.2",
            "1.2.3.4",
            "01.2.3",
            "1.2.x",
            "1.2.3-",
            "1.2.3-a..b",
            "1.2.3+b",
        ] {
            assert!(Version::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn orders_as_semver_does() {
        let order = [
            "0.9.9",
            "1.0.0-alpha",
            "1.0.0-alpha.1",
            "1.0.0-alpha.beta",
            "1.0.0-beta",
            "1.0.0-beta.2",
            "1.0.0-beta.11",
            "1.0.0-rc.1",
            "1.0.0",
            "1.0.1",
            "1.2.0",
            "1.10.0",
            "2.0.0",
        ];
        for pair in order.windows(2) {
            assert!(v(pair[0]) < v(pair[1]), "{} < {}", pair[0], pair[1]);
        }
    }
}
