use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::prompt::Prompter;

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{:02x}", b)).collect()
}

pub enum EditedPolicy<'a> {
    /// Ask before overwriting a file the user changed (CLI).
    Ask(&'a mut dyn Prompter),
    /// Leave edited files alone and report them (web UI).
    Skip,
}

#[derive(Debug, Default, PartialEq)]
pub struct InstallReport {
    pub installed: Vec<PathBuf>,
    pub unchanged: Vec<PathBuf>,
    pub skipped_edited: Vec<PathBuf>,
}

type Manifest = BTreeMap<String, String>;

fn load_manifest(path: &Path) -> Manifest {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_manifest(path: &Path, manifest: &Manifest) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create {}: {}", parent.display(), e))?;
    }
    let json = serde_json::to_string_pretty(manifest).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| format!("Failed to write {}: {}", path.display(), e))
}

fn write_file(dest: &Path, contents: &[u8]) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create {}: {}", parent.display(), e))?;
    }
    std::fs::write(dest, contents).map_err(|e| format!("Failed to write {}: {}", dest.display(), e))
}

pub fn install_managed_files(
    files: &[(PathBuf, Vec<u8>)],
    manifest_path: &Path,
    policy: &mut EditedPolicy,
) -> Result<InstallReport, String> {
    let mut manifest = load_manifest(manifest_path);
    let mut report = InstallReport::default();

    for (dest, contents) in files {
        let key = dest.to_string_lossy().to_string();
        let new_hash = sha256_hex(contents);

        let overwrite = match std::fs::read(dest) {
            Err(_) => true,
            Ok(current) => {
                let current_hash = sha256_hex(&current);
                if current_hash == new_hash {
                    manifest.insert(key, new_hash);
                    report.unchanged.push(dest.clone());
                    continue;
                }
                let untouched = manifest.get(&key) == Some(&current_hash);
                if untouched {
                    true
                } else {
                    match policy {
                        EditedPolicy::Skip => false,
                        EditedPolicy::Ask(p) => p.confirm(
                            &format!("{} has local changes. Overwrite with the new version?", dest.display()),
                            false,
                        ),
                    }
                }
            }
        };

        if overwrite {
            write_file(dest, contents)?;
            manifest.insert(key, new_hash);
            report.installed.push(dest.clone());
        } else {
            report.skipped_edited.push(dest.clone());
        }
    }

    save_manifest(manifest_path, &manifest)?;
    Ok(report)
}

/// Everything this binary installs on the user's machine, with its destination.
pub fn managed_files(config_file: &Path, home: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    if let Some(dir) = crate::config::read_config_value_from(config_file, "default-dir").filter(|d| !d.is_empty()) {
        out.push((crate::config::expand_tilde(&dir).join("AGENTS.md"), crate::assets::AGENTS_MD.as_bytes().to_vec()));
    }
    for f in crate::assets::skill_files() {
        out.push((home.join(".claude/skills").join(&f.rel_path), f.contents));
    }
    out
}

pub fn manifest_path(config_file: &Path) -> PathBuf {
    config_file.with_file_name("installed-files.json")
}

pub fn refresh(config_file: &Path, home: &Path, policy: &mut EditedPolicy) -> Result<InstallReport, String> {
    install_managed_files(&managed_files(config_file, home), &manifest_path(config_file), policy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompt::testing::ScriptedPrompter;
    use tempfile::tempdir;

    fn files(dir: &Path, body: &str) -> Vec<(PathBuf, Vec<u8>)> {
        vec![(dir.join("my dir").join("AGENTS.md"), body.as_bytes().to_vec())]
    }

    #[test]
    fn fresh_install_writes_file_and_records_hash_even_with_spaces_in_path() {
        let dir = tempdir().unwrap();
        let manifest = dir.path().join("installed-files.json");
        let f = files(dir.path(), "v1");
        let report = install_managed_files(&f, &manifest, &mut EditedPolicy::Skip).unwrap();
        assert_eq!(report.installed.len(), 1);
        assert_eq!(std::fs::read_to_string(&f[0].0).unwrap(), "v1");
        assert!(manifest.exists());
    }

    #[test]
    fn unmodified_previous_install_is_overwritten_silently() {
        let dir = tempdir().unwrap();
        let manifest = dir.path().join("installed-files.json");
        install_managed_files(&files(dir.path(), "v1"), &manifest, &mut EditedPolicy::Skip).unwrap();
        let f2 = files(dir.path(), "v2");
        let report = install_managed_files(&f2, &manifest, &mut EditedPolicy::Skip).unwrap();
        assert_eq!(report.installed.len(), 1);
        assert!(report.skipped_edited.is_empty());
        assert_eq!(std::fs::read_to_string(&f2[0].0).unwrap(), "v2");
    }

    #[test]
    fn identical_content_is_reported_unchanged() {
        let dir = tempdir().unwrap();
        let manifest = dir.path().join("installed-files.json");
        let f = files(dir.path(), "v1");
        install_managed_files(&f, &manifest, &mut EditedPolicy::Skip).unwrap();
        let report = install_managed_files(&f, &manifest, &mut EditedPolicy::Skip).unwrap();
        assert_eq!(report.unchanged.len(), 1);
    }

    #[test]
    fn edited_file_is_skipped_under_skip_policy() {
        let dir = tempdir().unwrap();
        let manifest = dir.path().join("installed-files.json");
        let f1 = files(dir.path(), "v1");
        install_managed_files(&f1, &manifest, &mut EditedPolicy::Skip).unwrap();
        std::fs::write(&f1[0].0, "my edits").unwrap();
        let report = install_managed_files(&files(dir.path(), "v2"), &manifest, &mut EditedPolicy::Skip).unwrap();
        assert_eq!(report.skipped_edited.len(), 1);
        assert_eq!(std::fs::read_to_string(&f1[0].0).unwrap(), "my edits");
    }

    #[test]
    fn edited_file_asks_and_honors_both_answers() {
        let dir = tempdir().unwrap();
        let manifest = dir.path().join("installed-files.json");
        let f1 = files(dir.path(), "v1");
        install_managed_files(&f1, &manifest, &mut EditedPolicy::Skip).unwrap();
        std::fs::write(&f1[0].0, "my edits").unwrap();

        let mut no = ScriptedPrompter::new(&["n"]);
        let report = install_managed_files(&files(dir.path(), "v2"), &manifest, &mut EditedPolicy::Ask(&mut no)).unwrap();
        assert_eq!(report.skipped_edited.len(), 1);
        assert_eq!(std::fs::read_to_string(&f1[0].0).unwrap(), "my edits");
        assert_eq!(no.questions.len(), 1);

        let mut yes = ScriptedPrompter::new(&["y"]);
        install_managed_files(&files(dir.path(), "v2"), &manifest, &mut EditedPolicy::Ask(&mut yes)).unwrap();
        assert_eq!(std::fs::read_to_string(&f1[0].0).unwrap(), "v2");
    }

    #[test]
    fn existing_file_without_manifest_record_is_treated_as_edited() {
        let dir = tempdir().unwrap();
        let manifest = dir.path().join("installed-files.json");
        let f = files(dir.path(), "new");
        std::fs::create_dir_all(f[0].0.parent().unwrap()).unwrap();
        std::fs::write(&f[0].0, "hand-written").unwrap();
        let report = install_managed_files(&f, &manifest, &mut EditedPolicy::Skip).unwrap();
        assert_eq!(report.skipped_edited.len(), 1);
        assert_eq!(std::fs::read_to_string(&f[0].0).unwrap(), "hand-written");
    }

    #[test]
    fn managed_files_targets_default_dir_and_claude_skills() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.md");
        crate::config::write_config_value_to(&config, "default-dir", dir.path().join("tasks").to_str().unwrap()).unwrap();
        let home = dir.path().join("home");
        let targets = managed_files(&config, &home);
        assert!(targets.iter().any(|(p, _)| p == &dir.path().join("tasks").join("AGENTS.md")));
        assert!(targets.iter().any(|(p, _)| p == &home.join(".claude/skills/task-manager/SKILL.md")));
    }

    #[test]
    fn managed_files_skips_agents_md_when_default_dir_unset() {
        let dir = tempdir().unwrap();
        let targets = managed_files(&dir.path().join("config.md"), &dir.path().join("home"));
        assert!(targets.iter().all(|(p, _)| p.file_name().unwrap() != "AGENTS.md"));
    }
}
