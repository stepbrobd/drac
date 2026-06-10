use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

/// Config format version (maybe schema version is a better name) in `yyyy.mdd.patch` format
/// e.g. `2026.610.0`
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    year: u16,
    mdd: u16,
    patch: u32,
}

/// The latest config version
pub const CURRENT: Version = Version {
    year: 2026,
    mdd: 610,
    patch: 0,
};

/// Rejection reasons for config versions
#[derive(Debug, Error, PartialEq, Eq)]
pub enum VersionError {
    /// The string is not canonical `yyyy.mdd.patch`
    #[error("malformed version {0:?}, expected yyyy.mdd.patch like {CURRENT}")]
    Malformed(String),
    /// The config comes from a newer drac than this build
    #[error("config version {found} is newer than {current}, the latest this build understands")]
    Future { found: Version, current: Version },
    /// The config predates this build and no migration exists for it
    #[error("config version {found} is older than {current} and no migration exists")]
    Unsupported { found: Version, current: Version },
}

/// Gates a parsed version against what the current build supports
pub fn check(v: Version) -> Result<(), VersionError> {
    use std::cmp::Ordering::*;
    match v.cmp(&CURRENT) {
        Equal => Ok(()),
        Greater => Err(VersionError::Future {
            found: v,
            current: CURRENT,
        }),
        Less => Err(VersionError::Unsupported {
            found: v,
            current: CURRENT,
        }),
    }
}

// Canonical decimal means digits only and no leading zero
fn component(s: &str) -> Option<u32> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if s.len() > 1 && s.starts_with('0') {
        return None;
    }
    s.parse().ok()
}

impl FromStr for Version {
    type Err = VersionError;

    fn from_str(s: &str) -> Result<Self, VersionError> {
        let err = || VersionError::Malformed(s.to_string());
        let mut parts = s.split('.');
        let (y, m, p) = match (parts.next(), parts.next(), parts.next(), parts.next()) {
            (Some(y), Some(m), Some(p), None) => (y, m, p),
            _ => return Err(err()),
        };
        // no way this is go pass 9999 lmao
        if y.len() != 4 {
            return Err(err());
        }
        let year = component(y).ok_or_else(err)? as u16;
        let mdd = component(m).ok_or_else(err)?;
        // mdd packs month then a two digit day
        // e.g. june 10 is 610
        let (month, day) = (mdd / 100, mdd % 100);
        if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
            return Err(err());
        }
        let patch = component(p).ok_or_else(err)?;
        Ok(Version {
            year,
            mdd: mdd as u16,
            patch,
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // the packed mdd prints verbatim because the day is always two digits
        write!(f, "{}.{}.{}", self.year, self.mdd, self.patch)
    }
}

impl Serialize for Version {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?.parse().map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        s.parse().unwrap()
    }

    #[test]
    fn drac_config_version_parses_current() {
        assert_eq!(v("2026.610.0"), CURRENT);
    }

    #[test]
    fn drac_config_version_displays_what_it_parsed() {
        for s in ["2026.610.0", "2026.101.3", "2026.1231.42"] {
            assert_eq!(v(s).to_string(), s);
        }
    }

    #[test]
    fn drac_config_version_orders_by_year_then_mdd_then_patch() {
        assert!(v("2026.607.0") < v("2026.610.0"));
        assert!(v("2026.610.0") < v("2026.610.1"));
        assert!(v("2026.1231.9") < v("2027.101.0"));
    }

    #[test]
    fn drac_config_version_rejects_malformed() {
        let bad = [
            "",
            "2026",
            "2026.610",
            "2026.610.0.0",
            "v2026.610.0",
            "2026.0610.0",
            "2026.610.00",
            "26.610.0",
            "2026.1310.0",
            "2026.632.0",
            "2026.600.0",
            "2026.61.0",
            "2026 .610.0",
        ];
        for s in bad {
            assert!(s.parse::<Version>().is_err(), "accepted {s:?}");
        }
    }

    #[test]
    fn drac_config_version_current_passes_check() {
        assert_eq!(check(CURRENT), Ok(()));
    }

    #[test]
    fn drac_config_version_future_is_refused() {
        for s in ["2026.610.1", "2026.611.0", "2027.101.0"] {
            assert!(matches!(check(v(s)), Err(VersionError::Future { .. })));
        }
    }

    #[test]
    fn drac_config_version_unknown_past_is_refused() {
        for s in ["2026.607.0", "2025.1231.9"] {
            assert!(matches!(check(v(s)), Err(VersionError::Unsupported { .. })));
        }
    }
}
