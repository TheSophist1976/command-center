use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use semver::Version;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone)]
pub struct ReleaseAsset {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct Release {
    pub version: Version,
    pub assets: Vec<ReleaseAsset>,
}

impl Release {
    pub fn asset_url(&self, name: &str) -> Option<&str> {
        self.assets.iter().find(|a| a.name == name).map(|a| a.url.as_str())
    }
}

#[derive(Deserialize)]
struct ReleaseJson {
    tag_name: String,
    #[serde(default)]
    assets: Vec<AssetJson>,
}

#[derive(Deserialize)]
struct AssetJson {
    name: String,
    browser_download_url: String,
}

fn client(timeout_secs: u64) -> Result<reqwest::blocking::Client, UpdateError> {
    reqwest::blocking::Client::builder()
        .user_agent(concat!("task/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| UpdateError::Network(e.to_string()))
}

pub fn latest_release(api_base: &str) -> Result<Release, UpdateError> {
    let url = format!("{}/releases/latest", api_base.trim_end_matches('/'));
    let response = client(15)?
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .map_err(|e| UpdateError::Network(e.to_string()))?;
    if !response.status().is_success() {
        return Err(UpdateError::Network(format!("release API returned HTTP {}", response.status())));
    }
    let parsed: ReleaseJson = response.json().map_err(|e| UpdateError::BadRelease(e.to_string()))?;
    Ok(Release {
        version: parse_version(&parsed.tag_name)?,
        assets: parsed.assets.into_iter().map(|a| ReleaseAsset { name: a.name, url: a.browser_download_url }).collect(),
    })
}

pub fn download(url: &str) -> Result<Vec<u8>, UpdateError> {
    let response = client(300)?.get(url).send().map_err(|e| UpdateError::Network(e.to_string()))?;
    if !response.status().is_success() {
        return Err(UpdateError::Network(format!("download returned HTTP {}", response.status())));
    }
    response.bytes().map(|b| b.to_vec()).map_err(|e| UpdateError::Network(e.to_string()))
}

const BINARIES: [&str; 2] = ["task", "task-tui"];

/// Extracts `task` / `task-tui` into temp files in `install_dir`, then renames them over
/// the existing binaries. Any failure before the renames removes every temp file and
/// leaves the install untouched.
pub fn install_binaries(archive: &[u8], install_dir: &Path) -> Result<Vec<String>, UpdateError> {
    use std::io::Read;
    use std::os::unix::fs::PermissionsExt;

    let io_err = |e: std::io::Error| UpdateError::Install(e.to_string());
    let mut staged: Vec<(String, PathBuf)> = Vec::new();

    let extracted = (|| -> Result<(), UpdateError> {
        let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(archive));
        for entry in tar.entries().map_err(io_err)? {
            let mut entry = entry.map_err(io_err)?;
            if !entry.header().entry_type().is_file() {
                continue;
            }
            let name = match entry.path().ok().and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string())) {
                Some(n) if BINARIES.contains(&n.as_str()) => n,
                _ => continue,
            };
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).map_err(io_err)?;
            let tmp = install_dir.join(format!(".{}.update-{}", name, std::process::id()));
            std::fs::write(&tmp, &buf).map_err(|e| {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    UpdateError::InstallDirNotWritable(install_dir.to_path_buf())
                } else {
                    UpdateError::Install(e.to_string())
                }
            })?;
            staged.push((name, tmp.clone()));
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755)).map_err(io_err)?;
        }
        if !staged.iter().any(|(n, _)| n == "task") {
            return Err(UpdateError::Install("archive does not contain `task`".to_string()));
        }
        Ok(())
    })();

    let cleanup = |staged: &[(String, PathBuf)]| {
        for (_, tmp) in staged {
            let _ = std::fs::remove_file(tmp);
        }
    };

    if let Err(e) = extracted {
        cleanup(&staged);
        return Err(e);
    }

    let mut installed = Vec::new();
    for (i, (name, tmp)) in staged.iter().enumerate() {
        if let Err(e) = std::fs::rename(tmp, install_dir.join(name)) {
            cleanup(&staged[i..]);
            return Err(UpdateError::Install(e.to_string()));
        }
        installed.push(name.clone());
    }
    Ok(installed)
}

#[derive(Debug, Clone, PartialEq)]
pub struct UpdateOutcome {
    pub from: Version,
    pub to: Version,
    pub installed: Vec<String>,
}

/// Looks up the latest release and installs it if it is newer. `Ok(None)` means already
/// up to date.
pub fn perform_update(
    api_base: &str,
    install_dir: &Path,
    on_phase: &mut dyn FnMut(Phase),
) -> Result<Option<UpdateOutcome>, UpdateError> {
    let release = latest_release(api_base)?;
    let current = current_version();
    if !is_newer(&current, &release.version) {
        return Ok(None);
    }
    let archive = archive_name(&release.version, target_triple()?);
    let archive_url = release.asset_url(&archive).ok_or_else(|| UpdateError::NoPlatformAsset(archive.clone()))?;
    let sums_url = release.asset_url("SHA256SUMS").ok_or_else(|| UpdateError::ChecksumMissing("SHA256SUMS".to_string()))?;

    on_phase(Phase::Downloading);
    let bytes = download(archive_url)?;
    let sums_text = String::from_utf8_lossy(&download(sums_url)?).into_owned();

    on_phase(Phase::Verifying);
    verify_checksum(&archive, &bytes, &parse_checksums(&sums_text))?;

    on_phase(Phase::Installing);
    let installed = install_binaries(&bytes, install_dir)?;
    Ok(Some(UpdateOutcome { from: current, to: release.version, installed }))
}

fn server_is_running() -> bool {
    let port: u16 = std::env::var("TASK_SERVER_PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(4287);
    std::net::TcpStream::connect_timeout(&std::net::SocketAddr::from(([127, 0, 0, 1], port)), Duration::from_millis(200)).is_ok()
}

/// `task update` / `task update --check`.
pub fn run_cli(check_only: bool) -> Result<(), String> {
    let api = releases_api();
    let current = current_version();
    if check_only {
        let release = latest_release(&api).map_err(|e| e.to_string())?;
        if is_newer(&current, &release.version) {
            println!("Update available: v{} -> v{}. Run `task update`.", current, release.version);
        } else {
            println!("task v{} is up to date.", current);
        }
        return Ok(());
    }

    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let install_dir = exe.parent().ok_or("Cannot locate the install directory")?.to_path_buf();
    let outcome = perform_update(&api, &install_dir, &mut |p| {
        println!("▸ {}…", format!("{:?}", p).to_lowercase());
    })
    .map_err(|e| e.to_string())?;

    match outcome {
        None => println!("task v{} is already up to date.", current),
        Some(o) => {
            // The new binary carries the new AGENTS.md/skills, so it does the refresh.
            let status = std::process::Command::new(install_dir.join("task")).arg("refresh-files").status();
            if !matches!(status, Ok(s) if s.success()) {
                eprintln!("warning: could not refresh AGENTS.md/skills; run `task setup` to reinstall them");
            }
            println!("Updated task v{} -> v{}.", o.from, o.to);
            if server_is_running() {
                println!("`task serve` is running — restart it to finish the update.");
            }
        }
    }
    Ok(())
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

    fn make_archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut builder = tar::Builder::new(gz);
        for (name, data) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            builder.append_data(&mut header, name, *data).unwrap();
        }
        let gz = builder.into_inner().unwrap();
        gz.finish().unwrap()
    }

    fn leftover_temps(dir: &std::path::Path) -> Vec<String> {
        std::fs::read_dir(dir).unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .filter(|n| n.contains(".update-"))
            .collect()
    }

    #[test]
    fn install_binaries_replaces_both_binaries_atomically() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("task"), "old").unwrap();
        let archive = make_archive(&[("task", b"new-task"), ("task-tui", b"new-tui")]);
        let installed = install_binaries(&archive, dir.path()).unwrap();
        assert_eq!(installed, vec!["task".to_string(), "task-tui".to_string()]);
        assert_eq!(std::fs::read(dir.path().join("task")).unwrap(), b"new-task");
        assert_eq!(std::fs::read(dir.path().join("task-tui")).unwrap(), b"new-tui");
        assert!(leftover_temps(dir.path()).is_empty());
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(dir.path().join("task")).unwrap().permissions().mode() & 0o777, 0o755);
    }

    #[test]
    fn corrupt_archive_leaves_existing_binary_untouched_and_no_temp_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("task"), "old").unwrap();
        let mut archive = make_archive(&[("task", b"new-task")]);
        archive.truncate(archive.len() / 2);
        assert!(install_binaries(&archive, dir.path()).is_err());
        assert_eq!(std::fs::read(dir.path().join("task")).unwrap(), b"old");
        assert!(leftover_temps(dir.path()).is_empty());
    }

    #[test]
    fn archive_without_task_binary_is_rejected_and_cleans_up() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("task"), "old").unwrap();
        let archive = make_archive(&[("task-tui", b"only-tui")]);
        assert!(matches!(install_binaries(&archive, dir.path()), Err(UpdateError::Install(_))));
        assert_eq!(std::fs::read(dir.path().join("task")).unwrap(), b"old");
        assert!(!dir.path().join("task-tui").exists());
        assert!(leftover_temps(dir.path()).is_empty());
    }

    #[test]
    fn non_writable_install_dir_is_a_typed_error() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o555)).unwrap();
        let archive = make_archive(&[("task", b"new")]);
        let result = install_binaries(&archive, dir.path());
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(matches!(result, Err(UpdateError::InstallDirNotWritable(_))));
    }

    fn release_json(server_url: &str, tag: &str, target: &str) -> String {
        let v = parse_version(tag).unwrap();
        let archive = archive_name(&v, target);
        format!(
            r#"{{"tag_name":"{tag}","assets":[
                {{"name":"{archive}","browser_download_url":"{server_url}/dl/{archive}"}},
                {{"name":"SHA256SUMS","browser_download_url":"{server_url}/dl/SHA256SUMS"}}]}}"#
        )
    }

    #[test]
    fn latest_release_parses_tag_and_assets() {
        let mut server = mockito::Server::new();
        let target = target_triple().unwrap();
        server.mock("GET", "/releases/latest").with_status(200)
            .with_body(release_json(&server.url(), "v99.0.0", target)).create();
        let release = latest_release(&server.url()).unwrap();
        assert_eq!(release.version, Version::new(99, 0, 0));
        assert!(release.asset_url("SHA256SUMS").unwrap().ends_with("/dl/SHA256SUMS"));
    }

    #[test]
    fn latest_release_rate_limit_garbage_and_no_network_are_typed_errors() {
        let mut server = mockito::Server::new();
        let limited = server.mock("GET", "/releases/latest").with_status(403).create();
        assert!(matches!(latest_release(&server.url()), Err(UpdateError::Network(_))));
        limited.remove();

        server.mock("GET", "/releases/latest").with_status(200).with_body("<html>oops</html>").create();
        assert!(matches!(latest_release(&server.url()), Err(UpdateError::BadRelease(_))));

        assert!(matches!(latest_release("http://127.0.0.1:1"), Err(UpdateError::Network(_))));
    }

    #[test]
    fn latest_release_with_non_semver_tag_is_bad_release() {
        let mut server = mockito::Server::new();
        server.mock("GET", "/releases/latest").with_status(200).with_body(r#"{"tag_name":"nightly","assets":[]}"#).create();
        assert!(matches!(latest_release(&server.url()), Err(UpdateError::BadRelease(_))));
    }

    fn serve_release(server: &mut mockito::ServerGuard, archive: &[u8], sums_hash: &str) {
        let target = target_triple().unwrap();
        let name = archive_name(&Version::new(99, 0, 0), target);
        server.mock("GET", "/releases/latest").with_status(200)
            .with_body(release_json(&server.url(), "v99.0.0", target)).create();
        server.mock("GET", format!("/dl/{}", name).as_str()).with_status(200).with_body(archive).create();
        server.mock("GET", "/dl/SHA256SUMS").with_status(200).with_body(format!("{}  {}\n", sums_hash, name)).create();
    }

    #[test]
    fn perform_update_installs_when_newer_and_reports_phases() {
        let mut server = mockito::Server::new();
        let archive = make_archive(&[("task", b"v99")]);
        serve_release(&mut server, &archive, &crate::managed::sha256_hex(&archive));
        let dir = tempfile::tempdir().unwrap();
        let mut phases = Vec::new();
        let outcome = perform_update(&server.url(), dir.path(), &mut |p| phases.push(p)).unwrap().unwrap();
        assert_eq!(outcome.to, Version::new(99, 0, 0));
        assert_eq!(std::fs::read(dir.path().join("task")).unwrap(), b"v99");
        assert_eq!(phases, vec![Phase::Downloading, Phase::Verifying, Phase::Installing]);
    }

    #[test]
    fn perform_update_checksum_mismatch_installs_nothing() {
        let mut server = mockito::Server::new();
        let archive = make_archive(&[("task", b"v99")]);
        serve_release(&mut server, &archive, &"0".repeat(64));
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("task"), "old").unwrap();
        let err = perform_update(&server.url(), dir.path(), &mut |_| {}).unwrap_err();
        assert!(matches!(err, UpdateError::ChecksumMismatch(_)));
        assert_eq!(std::fs::read(dir.path().join("task")).unwrap(), b"old");
        assert!(leftover_temps(dir.path()).is_empty());
    }

    #[test]
    fn perform_update_is_a_noop_when_already_current_or_ahead() {
        let mut server = mockito::Server::new();
        server.mock("GET", "/releases/latest").with_status(200)
            .with_body(format!(r#"{{"tag_name":"v{}","assets":[]}}"#, current_version())).create();
        let dir = tempfile::tempdir().unwrap();
        assert!(perform_update(&server.url(), dir.path(), &mut |_| {}).unwrap().is_none());
    }

    #[test]
    fn perform_update_missing_platform_asset_is_typed() {
        let mut server = mockito::Server::new();
        server.mock("GET", "/releases/latest").with_status(200)
            .with_body(r#"{"tag_name":"v99.0.0","assets":[]}"#).create();
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(perform_update(&server.url(), dir.path(), &mut |_| {}), Err(UpdateError::NoPlatformAsset(_))));
    }
}
