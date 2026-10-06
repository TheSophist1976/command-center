use rust_embed::RustEmbed;

/// The built web UI. Debug builds read `web/dist` from disk; release builds embed it.
#[derive(RustEmbed)]
#[folder = "web/dist"]
struct WebAssets;

#[derive(RustEmbed)]
#[folder = "skills"]
#[exclude = "*.DS_Store"]
struct SkillAssets;

pub const AGENTS_MD: &str = include_str!("../AGENTS.md");

pub struct ManagedFile {
    pub rel_path: String,
    pub contents: Vec<u8>,
}

pub fn skill_files() -> Vec<ManagedFile> {
    SkillAssets::iter()
        .filter_map(|name| {
            let file = SkillAssets::get(&name)?;
            Some(ManagedFile { rel_path: name.to_string(), contents: file.data.into_owned() })
        })
        .collect()
}

pub fn web_asset(path: &str) -> Option<(Vec<u8>, String)> {
    let file = WebAssets::get(path)?;
    let mime = file.metadata.mimetype().to_string();
    Some((file.data.into_owned(), mime))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_files_include_task_manager_skill() {
        let files = skill_files();
        assert!(files.iter().any(|f| f.rel_path == "task-manager/SKILL.md"));
        assert!(files.iter().all(|f| !f.rel_path.ends_with(".DS_Store")));
    }

    #[test]
    fn plugin_and_installed_task_manager_skills_do_not_drift() {
        let installed = include_str!("../skills/task-manager/SKILL.md");
        let plugin = include_str!("../plugin/skills/task-manager/SKILL.md");
        assert_eq!(
            installed, plugin,
            "skills/task-manager/SKILL.md (installed by `task setup`) and plugin/skills/task-manager/SKILL.md must be identical; edit one and copy it to the other"
        );
    }

    #[test]
    fn task_manager_skill_documents_every_work_status() {
        let skill = include_str!("../skills/task-manager/SKILL.md");
        for status in ["todo", "in-progress", "waiting-for-review", "changes-requested", "needs-input", "complete"] {
            assert!(skill.contains(status), "skill does not mention work status {}", status);
        }
        assert!(!skill.contains("Mark-main"), "skill must not hardcode a personal path");
    }

    #[test]
    fn shared_skills_name_no_personal_paths_or_harness_specific_tools() {
        let skills = [
            ("task-manager", include_str!("../skills/task-manager/SKILL.md")),
            ("work-agent-tasks", include_str!("../plugin/skills/work-agent-tasks/SKILL.md")),
            ("slack-later-capture", include_str!("../plugin/skills/slack-later-capture/SKILL.md")),
        ];
        for (name, text) in skills {
            assert!(!text.contains("Mark-main"), "{} hardcodes a personal path", name);
            assert!(!text.contains("AskUserQuestion"), "{} names a harness-specific tool", name);
        }
    }

    #[test]
    fn agents_md_is_embedded() {
        assert!(AGENTS_MD.contains("needs-input"));
    }

    #[test]
    fn unknown_web_asset_is_none() {
        assert!(web_asset("definitely/not/here.js").is_none());
    }
}
