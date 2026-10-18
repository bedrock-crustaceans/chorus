//! Version negotiation between the host and a plugin.

/// The `chorus:plugin` package version this build of the host speaks. Keep it in step with the
/// `package chorus:plugin@x.y.z;` line in `wit/world.wit`; the test below is the reminder.
pub const ABI_VERSION: AbiVersion = AbiVersion { major: 0, minor: 1, patch: 0 };

/// A plain `major.minor.patch` triple. Not semver's full grammar: a plugin reports the WIT
/// package version it was built against, and that never carries a pre-release or build suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbiVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl AbiVersion {
    /// Parses exactly three dot-separated numbers. Anything else is `None`, which the loader
    /// treats the same as an incompatible version.
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = text.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse().ok()?;
        if parts.next().is_some() {
            return None;
        }
        Some(Self { major, minor, patch })
    }

    /// Whether a plugin reporting `self` can run against a host speaking `host`.
    ///
    /// Cargo's pre-1.0 rule: while the major is 0 the minor is the breaking number, so 0.1.x and
    /// 0.2.x are different ABIs. After 1.0 only the major matters.
    pub fn compatible(&self, host: &AbiVersion) -> bool {
        if self.major != host.major {
            return false;
        }
        host.major != 0 || self.minor == host.minor
    }
}

impl std::fmt::Display for AbiVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_semver_triple() {
        assert_eq!(AbiVersion::parse("0.1.0"), Some(AbiVersion { major: 0, minor: 1, patch: 0 }));
        assert_eq!(AbiVersion::parse("0.1"), None);
        assert_eq!(AbiVersion::parse("nonsense"), None);
    }

    #[test]
    fn pre_one_point_zero_pins_the_minor() {
        let host = AbiVersion { major: 0, minor: 1, patch: 0 };
        assert!(AbiVersion { major: 0, minor: 1, patch: 7 }.compatible(&host));
        assert!(!AbiVersion { major: 0, minor: 2, patch: 0 }.compatible(&host));
    }

    #[test]
    fn post_one_point_zero_pins_the_major() {
        let host = AbiVersion { major: 1, minor: 4, patch: 0 };
        assert!(AbiVersion { major: 1, minor: 2, patch: 0 }.compatible(&host));
        assert!(!AbiVersion { major: 2, minor: 0, patch: 0 }.compatible(&host));
    }

    #[test]
    fn the_constant_matches_the_wit_package() {
        assert_eq!(ABI_VERSION, AbiVersion { major: 0, minor: 1, patch: 0 });
    }
}
