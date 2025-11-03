use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

/// 3段式版本号结构体 (major.minor.patch)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    /// 创建新的版本号
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Version {
            major,
            minor,
            patch,
        }
    }

    /// 创建初始版本 0.0.0
    pub fn initial() -> Self {
        Version::new(0, 0, 0)
    }

    /// 创建版本 1.0.0
    pub fn v1() -> Self {
        Version::new(1, 0, 0)
    }

    /// 增加主版本号，重置次版本号和补丁版本号
    pub fn bump_major(&mut self) {
        self.major += 1;
        self.minor = 0;
        self.patch = 0;
    }

    /// 增加次版本号，重置补丁版本号
    pub fn bump_minor(&mut self) {
        self.minor += 1;
        self.patch = 0;
    }

    /// 增加补丁版本号
    pub fn bump_patch(&mut self) {
        self.patch += 1;
    }

    /// 检查是否为预发布版本（主版本号为0）
    pub fn is_prerelease(&self) -> bool {
        self.major == 0
    }

    /// 检查是否兼容指定版本（主版本号相同且当前版本不小于指定版本）
    pub fn is_compatible_with(&self, other: &Version) -> bool {
        if self.major != other.major {
            return false;
        }
        self >= other
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => match self.minor.cmp(&other.minor) {
                Ordering::Equal => self.patch.cmp(&other.patch),
                other => other,
            },
            other => other,
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl FromStr for Version {
    type Err = VersionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.trim().split('.').collect();

        if parts.len() != 3 {
            return Err(VersionParseError::InvalidFormat);
        }

        let major = parts[0]
            .parse::<u32>()
            .map_err(|_| VersionParseError::InvalidNumber)?;
        let minor = parts[1]
            .parse::<u32>()
            .map_err(|_| VersionParseError::InvalidNumber)?;
        let patch = parts[2]
            .parse::<u32>()
            .map_err(|_| VersionParseError::InvalidNumber)?;

        Ok(Version::new(major, minor, patch))
    }
}

/// 版本解析错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum VersionParseError {
    InvalidFormat,
    InvalidNumber,
}

impl fmt::Display for VersionParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionParseError::InvalidFormat => {
                write!(f, "版本格式无效，应为 major.minor.patch 格式")
            }
            VersionParseError::InvalidNumber => write!(f, "版本号包含无效数字"),
        }
    }
}

impl std::error::Error for VersionParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_creation() {
        let v = Version::new(1, 2, 3);
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_initial_version() {
        let v = Version::initial();
        assert_eq!(v, Version::new(0, 0, 0));
    }

    #[test]
    fn test_v1_version() {
        let v = Version::v1();
        assert_eq!(v, Version::new(1, 0, 0));
    }

    #[test]
    fn test_version_comparison() {
        let v1 = Version::new(1, 0, 0);
        let v2 = Version::new(1, 0, 1);
        let v3 = Version::new(1, 1, 0);
        let v4 = Version::new(2, 0, 0);

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v3 < v4);
        assert!(v1 < v4);
    }

    #[test]
    fn test_version_equality() {
        let v1 = Version::new(1, 2, 3);
        let v2 = Version::new(1, 2, 3);
        let v3 = Version::new(1, 2, 4);

        assert_eq!(v1, v2);
        assert_ne!(v1, v3);
    }

    #[test]
    fn test_version_bump() {
        let mut v = Version::new(1, 2, 3);

        v.bump_patch();
        assert_eq!(v, Version::new(1, 2, 4));

        v.bump_minor();
        assert_eq!(v, Version::new(1, 3, 0));

        v.bump_major();
        assert_eq!(v, Version::new(2, 0, 0));
    }

    #[test]
    fn test_version_display() {
        let v = Version::new(1, 2, 3);
        assert_eq!(format!("{}", v), "1.2.3");
    }

    #[test]
    fn test_version_from_str() {
        let v: Version = "1.2.3".parse().unwrap();
        assert_eq!(v, Version::new(1, 2, 3));

        let v: Version = "0.1.0".parse().unwrap();
        assert_eq!(v, Version::new(0, 1, 0));
    }

    #[test]
    fn test_version_parse_errors() {
        assert!("1.2".parse::<Version>().is_err());
        assert!("1.2.3.4".parse::<Version>().is_err());
        assert!("a.b.c".parse::<Version>().is_err());
        assert!("1.2.c".parse::<Version>().is_err());
    }

    #[test]
    fn test_is_prerelease() {
        assert!(Version::new(0, 1, 0).is_prerelease());
        assert!(!Version::new(1, 0, 0).is_prerelease());
    }

    #[test]
    fn test_is_compatible_with() {
        let v1_0_0 = Version::new(1, 0, 0);
        let v1_1_0 = Version::new(1, 1, 0);
        let v1_2_0 = Version::new(1, 2, 0);
        let v2_0_0 = Version::new(2, 0, 0);

        assert!(v1_2_0.is_compatible_with(&v1_0_0));
        assert!(v1_2_0.is_compatible_with(&v1_1_0));
        assert!(!v1_1_0.is_compatible_with(&v1_2_0));
        assert!(!v2_0_0.is_compatible_with(&v1_0_0));
    }
}
