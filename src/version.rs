use semver::Version;

#[derive(Debug, PartialEq)]
pub enum VersionComparison {
    UpToDate,
    Behind(String),
    Ahead(String),
    Invalid(String),
}

pub fn compare_versions(local: &str, remote: &str) -> VersionComparison {
    let local_ver = match Version::parse(local) {
        Ok(v) => v,
        Err(_) => return VersionComparison::Invalid(format!("Invalid local version: {}", local)),
    };

    let remote_ver = match Version::parse(remote) {
        Ok(v) => v,
        Err(_) => return VersionComparison::Invalid(format!("Invalid remote version: {}", remote)),
    };

    if local_ver == remote_ver {
        VersionComparison::UpToDate
    } else if local_ver < remote_ver {
        VersionComparison::Behind(remote.to_string())
    } else {
        VersionComparison::Ahead(remote.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_equal() {
        let result = compare_versions("1.0.0", "1.0.0");
        assert_eq!(result, VersionComparison::UpToDate);
    }

    #[test]
    fn test_compare_behind() {
        let result = compare_versions("1.0.0", "1.1.0");
        assert_eq!(result, VersionComparison::Behind("1.1.0".to_string()));
    }

    #[test]
    fn test_compare_ahead() {
        let result = compare_versions("2.0.0", "1.0.0");
        assert_eq!(result, VersionComparison::Ahead("1.0.0".to_string()));
    }

    #[test]
    fn test_compare_invalid_local() {
        let result = compare_versions("invalid", "1.0.0");
        assert!(matches!(result, VersionComparison::Invalid(_)));
    }

    #[test]
    fn test_compare_invalid_remote() {
        let result = compare_versions("1.0.0", "invalid");
        assert!(matches!(result, VersionComparison::Invalid(_)));
    }

    #[test]
    fn test_compare_prerelease() {
        let result = compare_versions("1.0.0-alpha", "1.0.0");
        assert_eq!(result, VersionComparison::Behind("1.0.0".to_string()));
    }

    #[test]
    fn test_compare_patch() {
        let result = compare_versions("1.0.0", "1.0.1");
        assert_eq!(result, VersionComparison::Behind("1.0.1".to_string()));
    }
}
