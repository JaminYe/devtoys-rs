//! Application version and SemVer parsing / comparison according to SemVer 2.0.0.

use std::cmp::Ordering;
use std::fmt;

/// Current application build version, derived from build.rs (`DEVTOYS_APP_VERSION` or `CARGO_PKG_VERSION`).
pub const CURRENT_VERSION: &str = env!("DEVTOYS_APP_VERSION");

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VersionIdentifier {
    Numeric(u64),
    AlphaNumeric(String),
}

impl PartialOrd for VersionIdentifier {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VersionIdentifier {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Numeric(a), Self::Numeric(b)) => a.cmp(b),
            (Self::Numeric(_), Self::AlphaNumeric(_)) => Ordering::Less,
            (Self::AlphaNumeric(_), Self::Numeric(_)) => Ordering::Greater,
            (Self::AlphaNumeric(a), Self::AlphaNumeric(b)) => a.cmp(b),
        }
    }
}

#[derive(Clone, Debug, Eq)]
pub struct SemVer {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub prerelease: Vec<VersionIdentifier>,
    pub build: Option<String>,
    #[allow(dead_code)]
    raw: String,
}

impl SemVer {
    pub fn current() -> Self {
        Self::parse(CURRENT_VERSION).unwrap_or(Self {
            major: 0,
            minor: 1,
            patch: 0,
            prerelease: Vec::new(),
            build: None,
            raw: CURRENT_VERSION.to_string(),
        })
    }

    pub fn parse(input: &str) -> Result<Self, String> {
        let s = input.trim();
        let s = s
            .strip_prefix('v')
            .or_else(|| s.strip_prefix('V'))
            .unwrap_or(s);
        if s.is_empty() {
            return Err("version string is empty".into());
        }

        let (ver_pre, build) = match s.split_once('+') {
            Some((v, b)) => (v, Some(b.to_string())),
            None => (s, None),
        };

        let (core, pre) = match ver_pre.split_once('-') {
            Some((c, p)) => (c, Some(p)),
            None => (ver_pre, None),
        };

        let parts: Vec<&str> = core.split('.').collect();
        if parts.len() != 3 {
            return Err(format!(
                "core version must be MAJOR.MINOR.PATCH, got '{core}'"
            ));
        }

        let major = parse_part(parts[0], "MAJOR")?;
        let minor = parse_part(parts[1], "MINOR")?;
        let patch = parse_part(parts[2], "PATCH")?;

        let mut prerelease = Vec::new();
        if let Some(p) = pre {
            if p.is_empty() {
                return Err("prerelease identifier cannot be empty".into());
            }
            for ident in p.split('.') {
                if ident.is_empty() {
                    return Err("prerelease dot-separated identifier cannot be empty".into());
                }
                if !ident.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                    return Err(format!(
                        "prerelease identifier '{ident}' contains invalid characters"
                    ));
                }
                if ident.chars().all(|c| c.is_ascii_digit()) {
                    if ident.len() > 1 && ident.starts_with('0') {
                        return Err(format!(
                            "numeric prerelease identifier '{ident}' cannot have leading zero"
                        ));
                    }
                    let num = ident.parse::<u64>().map_err(|_| {
                        format!("numeric prerelease '{ident}' exceeds integer limit")
                    })?;
                    prerelease.push(VersionIdentifier::Numeric(num));
                } else {
                    prerelease.push(VersionIdentifier::AlphaNumeric(ident.to_string()));
                }
            }
        }

        if let Some(b) = &build {
            if b.is_empty() {
                return Err("build metadata identifier cannot be empty".into());
            }
            for ident in b.split('.') {
                if ident.is_empty() {
                    return Err("build metadata dot-separated identifier cannot be empty".into());
                }
                if !ident.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                    return Err(format!(
                        "build metadata identifier '{ident}' contains invalid characters"
                    ));
                }
            }
        }

        Ok(Self {
            major,
            minor,
            patch,
            prerelease,
            build,
            raw: s.to_string(),
        })
    }

    pub fn is_newer_than(&self, other: &Self) -> bool {
        self > other
    }

    pub fn is_prerelease(&self) -> bool {
        !self.prerelease.is_empty()
    }

    #[allow(dead_code)]
    pub fn raw(&self) -> &str {
        &self.raw
    }
}

fn parse_part(part: &str, name: &str) -> Result<u64, String> {
    if part.is_empty() {
        return Err(format!("{name} version part is empty"));
    }
    if !part.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("{name} version part '{part}' contains non-digits"));
    }
    if part.len() > 1 && part.starts_with('0') {
        return Err(format!("{name} version part '{part}' has leading zero"));
    }
    part.parse::<u64>()
        .map_err(|_| format!("{name} version part '{part}' exceeds integer limit"))
}

impl PartialEq for SemVer {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> Ordering {
        // SemVer 2.0.0 section 11:
        // 1. Compare major, minor, patch
        if self.major != other.major {
            return self.major.cmp(&other.major);
        }
        if self.minor != other.minor {
            return self.minor.cmp(&other.minor);
        }
        if self.patch != other.patch {
            return self.patch.cmp(&other.patch);
        }

        // 2. Normal version is higher than pre-release version
        match (self.is_prerelease(), other.is_prerelease()) {
            (false, false) => Ordering::Equal,
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (true, true) => {
                // Compare pre-release identifiers
                let min_len = self.prerelease.len().min(other.prerelease.len());
                for i in 0..min_len {
                    let ord = self.prerelease[i].cmp(&other.prerelease[i]);
                    if ord != Ordering::Equal {
                        return ord;
                    }
                }
                self.prerelease.len().cmp(&other.prerelease.len())
            }
        }
    }
}

impl fmt::Display for SemVer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if !self.prerelease.is_empty() {
            write!(f, "-")?;
            for (idx, ident) in self.prerelease.iter().enumerate() {
                if idx > 0 {
                    write!(f, ".")?;
                }
                match ident {
                    VersionIdentifier::Numeric(n) => write!(f, "{n}")?,
                    VersionIdentifier::AlphaNumeric(s) => write!(f, "{s}")?,
                }
            }
        }
        if let Some(b) = &self.build {
            write!(f, "+{b}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_versions() {
        let v = SemVer::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert!(!v.is_prerelease());
        assert_eq!(v.build, None);

        // With 'v' prefix
        let v2 = SemVer::parse("v1.2.3").unwrap();
        assert_eq!(v2.major, 1);
        assert_eq!(v2.minor, 2);
        assert_eq!(v2.patch, 3);

        // With uppercase 'V' prefix
        let v2u = SemVer::parse("V1.2.3").unwrap();
        assert_eq!(v2u.major, 1);

        // With prerelease
        let v3 = SemVer::parse("1.2.3-alpha.1").unwrap();
        assert!(v3.is_prerelease());
        assert_eq!(
            v3.prerelease,
            vec![
                VersionIdentifier::AlphaNumeric("alpha".into()),
                VersionIdentifier::Numeric(1)
            ]
        );

        // With build
        let v4 = SemVer::parse("1.2.3+20130313144700").unwrap();
        assert!(!v4.is_prerelease());
        assert_eq!(v4.build, Some("20130313144700".into()));

        // With prerelease and build
        let v5 = SemVer::parse("1.2.3-beta.2+exp.sha.5114f85").unwrap();
        assert!(v5.is_prerelease());
        assert_eq!(v5.build, Some("exp.sha.5114f85".into()));
    }

    #[test]
    fn parse_invalid_versions() {
        assert!(SemVer::parse("").is_err());
        assert!(SemVer::parse("1").is_err());
        assert!(SemVer::parse("1.2").is_err());
        assert!(SemVer::parse("1.2.3.4").is_err());
        assert!(SemVer::parse("01.2.3").is_err());
        assert!(SemVer::parse("1.02.3").is_err());
        assert!(SemVer::parse("1.2.03").is_err());
        assert!(SemVer::parse("1.2.3-").is_err());
        assert!(SemVer::parse("1.2.3+").is_err());
        assert!(SemVer::parse("1.2.3-01").is_err());
        assert!(SemVer::parse("1.2.3-alpha..1").is_err());
        assert!(SemVer::parse("1.2.3-alpha#").is_err());
    }

    #[test]
    fn semver_precedence_ordering() {
        let v1 = SemVer::parse("1.0.0-alpha").unwrap();
        let v2 = SemVer::parse("1.0.0-alpha.1").unwrap();
        let v3 = SemVer::parse("1.0.0-alpha.beta").unwrap();
        let v4 = SemVer::parse("1.0.0-beta").unwrap();
        let v5 = SemVer::parse("1.0.0-beta.2").unwrap();
        let v6 = SemVer::parse("1.0.0-beta.11").unwrap();
        let v7 = SemVer::parse("1.0.0-rc.1").unwrap();
        let v8 = SemVer::parse("1.0.0").unwrap();
        let v9 = SemVer::parse("2.0.0").unwrap();
        let v10 = SemVer::parse("2.1.0").unwrap();
        let v11 = SemVer::parse("2.1.1").unwrap();

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v3 < v4);
        assert!(v4 < v5);
        assert!(v5 < v6);
        assert!(v6 < v7);
        assert!(v7 < v8);
        assert!(v8 < v9);
        assert!(v9 < v10);
        assert!(v10 < v11);

        // Build metadata does not change precedence
        let b1 = SemVer::parse("1.0.0+build1").unwrap();
        let b2 = SemVer::parse("1.0.0+build2").unwrap();
        assert_eq!(b1, b2);
        assert_eq!(b1, v8);
    }
    #[test]
    fn is_newer_than_and_no_downgrade() {
        let cur = SemVer::parse("0.1.0").unwrap();
        let newer = SemVer::parse("0.2.0").unwrap();
        let patch = SemVer::parse("0.1.1").unwrap();
        let same = SemVer::parse("v0.1.0").unwrap();
        let older = SemVer::parse("0.0.9").unwrap();

        assert!(newer.is_newer_than(&cur));
        assert!(patch.is_newer_than(&cur));
        assert!(!same.is_newer_than(&cur));
        assert!(!older.is_newer_than(&cur));
    }

    #[test]
    fn current_version_is_valid() {
        let cur = SemVer::current();
        assert_eq!(cur.to_string(), CURRENT_VERSION);
    }
}
