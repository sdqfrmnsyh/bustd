//! Linux version parsing - minimal

use core::cmp::Ordering;
use core::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinuxVersion {
    pub major: u8,
    pub minor: u8,
}

impl LinuxVersion {
    #[inline]
    pub fn from_str(release: &str) -> Option<Self> {
        let dot = release.find('.')?;
        let major = release[..dot].parse().ok()?;
        let minor_str = &release[dot + 1..];
        let dot2 = minor_str.find('.')?;
        let minor = minor_str[..dot2].parse().ok()?;
        Some(Self { major, minor })
    }
}

impl Display for LinuxVersion {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "Linux {}.{}", self.major, self.minor)
    }
}

impl PartialOrd for LinuxVersion {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.major.cmp(&other.major) {
            Ordering::Equal => self.minor.partial_cmp(&other.minor),
            ord => Some(ord),
        }
    }
}