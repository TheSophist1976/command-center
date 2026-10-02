use std::collections::HashMap;
use std::path::PathBuf;

use semver::Version;
use serde::Serialize;

pub const DEFAULT_RELEASES_API: &str = "https://api.github.com/repos/TheSophist1976/command-center-releases";

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateError {
    Network(String),
    BadRelease(String),
    NoPlatformAsset(String),
    ChecksumMissing(String),
    ChecksumMismatch(String),
    InstallDirNotWritable(PathBuf),
    Install(String),
}

impl UpdateError {
    pub fn kind(&self) -> &'static str {
        match self {
            UpdateError::Network(_) => "network",
            UpdateError::BadRelease(_) => "bad_release",
            UpdateError::NoPlatformAsset(_) => "no_platform_asset",
            UpdateError::ChecksumMissing(_) => "checksum_missing",
            UpdateError::ChecksumMismatch(_) => "checksum_mismatch",
            UpdateError::InstallDirNotWritable(_) => "install_dir_not_writable",
            UpdateError::Install(_) => "install",
        }
    }
}

impl std::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateError::Network(m) => write!(f, "Could not reach the release server: {}", m),
            UpdateError::BadRelease(m) => write!(f, "The release data was not understood: {}", m),
            UpdateError::NoPlatformAsset(a) => write!(f, "No release is available for this platform ({})", a),
            UpdateError::ChecksumMissing(a) => write!(f, "No checksum was published for {}", a),
            UpdateError::ChecksumMismatch(a) => write!(f, "Checksum mismatch for {} — nothing was installed", a),
            UpdateError::InstallDirNotWritable(d) => write!(f, "Cannot write to {} — reinstall into a writable directory or fix its permissions", d.display()),
            UpdateError::Install(m) => write!(f, "Install failed: {}", m),
        }
    }
}

impl std::error::Error for UpdateError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Idle,
    Downloading,
    Verifying,
    Installing,
    Restarting,
    Failed,
}

pub fn releases_api() -> String {
    std::env::var("TASK_RELEASES_API")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_RELEASES_API.to_string())
}

pub fn current_version() -> Version {
    Version::parse(env!("CARGO_PKG_VERSION")).expect("CARGO_PKG_VERSION is valid semver")
}

pub fn parse_version(tag: &str) -> Result<Version, UpdateError> {
    Version::parse(tag.trim().trim_start_matches('v'))
        .map_err(|e| UpdateError::BadRelease(format!("tag {:?}: {}", tag, e)))
}

pub fn is_newer(current: &Version, latest: &Version) -> bool {
    latest > current
}

pub fn target_for(os: &str, arch: &str) -> Result<&'static str, UpdateError> {
    match (os, arch) {
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-gnu"),
        ("linux", "aarch64") => Ok("aarch64-unknown-linux-gnu"),
        (os, arch) => Err(UpdateError::NoPlatformAsset(format!("{}-{}", arch, os))),
    }
}

pub fn target_triple() -> Result<&'static str, UpdateError> {
    target_for(std::env::consts::OS, std::env::consts::ARCH)
}

pub fn archive_name(version: &Version, target: &str) -> String {
    format!("task-v{}-{}.tar.gz", version, target)
}

pub fn parse_checksums(text: &str) -> HashMap<String, String> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let hash = parts.next()?;
            let name = parts.next()?.trim_start_matches('*');
            Some((name.to_string(), hash.to_lowercase()))
        })
        .collect()
}

pub fn verify_checksum(name: &str, bytes: &[u8], sums: &HashMap<String, String>) -> Result<(), UpdateError> {
    let expected = sums.get(name).ok_or_else(|| UpdateError::ChecksumMissing(name.to_string()))?;
    if &crate::managed::sha256_hex(bytes) == expected {
        Ok(())
    } else {
        Err(UpdateError::ChecksumMismatch(name.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version_accepts_v_prefix_and_prerelease() {
        assert_eq!(parse_version("v4.1.0").unwrap(), Version::new(4, 1, 0));
        assert_eq!(parse_version("4.1.0-rc.1").unwrap().to_string(), "4.1.0-rc.1");
    }

    #[test]
    fn parse_version_rejects_junk_with_typed_error() {
        assert!(matches!(parse_version("nightly"), Err(UpdateError::BadRelease(_))));
        assert!(matches!(parse_version(""), Err(UpdateError::BadRelease(_))));
    }

    #[test]
    fn prerelease_is_not_newer_than_its_final_release() {
        let final_release = Version::new(4, 1, 0);
        let rc = parse_version("4.1.0-rc.1").unwrap();
        assert!(!is_newer(&final_release, &rc));
        assert!(is_newer(&rc, &final_release));
        assert!(is_newer(&Version::new(4, 0, 0), &final_release));
        assert!(!is_newer(&final_release, &final_release));
    }

    #[test]
    fn target_for_maps_the_four_supported_platforms() {
        assert_eq!(target_for("macos", "aarch64").unwrap(), "aarch64-apple-darwin");
        assert_eq!(target_for("macos", "x86_64").unwrap(), "x86_64-apple-darwin");
        assert_eq!(target_for("linux", "x86_64").unwrap(), "x86_64-unknown-linux-gnu");
        assert_eq!(target_for("linux", "aarch64").unwrap(), "aarch64-unknown-linux-gnu");
        assert!(matches!(target_for("windows", "x86_64"), Err(UpdateError::NoPlatformAsset(_))));
    }

    #[test]
    fn archive_name_matches_release_workflow() {
        assert_eq!(archive_name(&Version::new(4, 1, 0), "aarch64-apple-darwin"), "task-v4.1.0-aarch64-apple-darwin.tar.gz");
    }

    #[test]
    fn checksums_parse_sha256sum_format_including_binary_marker() {
        let sums = parse_checksums("AAA  one.tar.gz\nbbb *two.tar.gz\n\nmalformed\n");
        assert_eq!(sums.get("one.tar.gz").unwrap(), "aaa");
        assert_eq!(sums.get("two.tar.gz").unwrap(), "bbb");
        assert_eq!(sums.len(), 2);
    }

    #[test]
    fn verify_checksum_accepts_match_and_rejects_mismatch_or_missing() {
        let bytes = b"hello";
        let mut sums = HashMap::new();
        sums.insert("a.tar.gz".to_string(), crate::managed::sha256_hex(bytes));
        assert!(verify_checksum("a.tar.gz", bytes, &sums).is_ok());
        assert!(matches!(verify_checksum("a.tar.gz", b"tampered", &sums), Err(UpdateError::ChecksumMismatch(_))));
        assert!(matches!(verify_checksum("b.tar.gz", bytes, &sums), Err(UpdateError::ChecksumMissing(_))));
    }

    #[test]
    fn update_error_kinds_are_stable_strings() {
        assert_eq!(UpdateError::Network("x".into()).kind(), "network");
        assert_eq!(UpdateError::ChecksumMismatch("x".into()).kind(), "checksum_mismatch");
        assert_eq!(UpdateError::InstallDirNotWritable("/x".into()).kind(), "install_dir_not_writable");
    }
}
