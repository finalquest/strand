use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::config::Config;

#[derive(Debug, Clone)]
pub struct LocalArtifact {
    pub name: String,
    pub version: String,
    pub path: std::path::PathBuf,
}

pub fn scan_local_skills() -> Vec<LocalArtifact> {
    scan_local_artifacts(".agents/skills", "SKILL.md", parse_skill_version)
}

pub fn scan_local_agents() -> Vec<LocalArtifact> {
    scan_local_artifacts(".agents/agents", "AGENT.md", parse_agent_version)
}

pub fn check_local_skill_conflict(name: &str, managed_names: &HashSet<String>) -> Result<(), String> {
    check_local_artifact_conflict(name, ".agents/skills", managed_names)
}

pub fn check_local_agent_conflict(name: &str, managed_names: &HashSet<String>) -> Result<(), String> {
    check_local_artifact_conflict(name, ".agents/agents", managed_names)
}

fn scan_local_artifacts(
    base_dir: &str,
    manifest_file: &str,
    parse_version: fn(&Path) -> Option<String>,
) -> Vec<LocalArtifact> {
    let base = Path::new(base_dir);
    if !base.is_dir() {
        return Vec::new();
    }

    let config_names = load_managed_names(manifest_file);

    let Ok(entries) = fs::read_dir(base) else {
        return Vec::new();
    };

    let mut artifacts = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let name = match path.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => continue,
        };

        if config_names.contains(&name) {
            continue;
        }

        let manifest_path = path.join(manifest_file);
        if !manifest_path.exists() {
            continue;
        }

        let version = parse_version(&manifest_path).unwrap_or_else(|| "unknown".to_string());

        artifacts.push(LocalArtifact {
            name,
            version,
            path,
        });
    }

    artifacts
}

fn check_local_artifact_conflict(
    name: &str,
    base_dir: &str,
    managed_names: &HashSet<String>,
) -> Result<(), String> {
    if managed_names.contains(name) {
        return Ok(());
    }

    let artifact_dir = Path::new(base_dir).join(name);
    if artifact_dir.is_dir() {
        return Err(format!(
            "Cannot install '{}': a local artifact with this name already exists in {}. Remove it manually or rename it first.",
            name,
            base_dir
        ));
    }

    Ok(())
}

fn parse_skill_version(manifest_path: &Path) -> Option<String> {
    let content = fs::read_to_string(manifest_path).ok()?;
    let frontmatter = crate::models::skill::parse_skill_md(&content).ok()?;
    Some(frontmatter.metadata.version)
}

fn parse_agent_version(manifest_path: &Path) -> Option<String> {
    let content = fs::read_to_string(manifest_path).ok()?;
    let frontmatter = crate::models::agent::parse_agent_md(&content).ok()?;
    Some(frontmatter.metadata.version)
}

fn load_config() -> Option<Config> {
    let config_path = Path::new(crate::config::CONFIG_PATH);
    if !config_path.exists() {
        return None;
    }
    let config_str = fs::read_to_string(config_path).ok()?;
    serde_json::from_str(&config_str).ok()
}

fn load_managed_names(manifest_file: &str) -> HashSet<String> {
    load_config()
        .map(|c| {
            if manifest_file == "SKILL.md" {
                c.skills.into_iter().map(|s| s.name).collect()
            } else {
                c.agents.into_iter().map(|a| a.name).collect()
            }
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_check_local_skill_conflict_no_conflict() {
        let managed: HashSet<String> = HashSet::new();
        assert!(check_local_skill_conflict("nonexistent-skill", &managed).is_ok());
    }

    #[test]
    fn test_check_local_skill_conflict_managed_name() {
        let mut managed: HashSet<String> = HashSet::new();
        managed.insert("my-skill".to_string());
        assert!(check_local_skill_conflict("my-skill", &managed).is_ok());
    }

    #[test]
    fn test_scan_local_skills_no_dir() {
        let locals = scan_local_artifacts("/nonexistent/path", "SKILL.md", |_| None);
        assert!(locals.is_empty());
    }

    #[test]
    fn test_check_local_agent_conflict_no_conflict() {
        let managed: HashSet<String> = HashSet::new();
        assert!(check_local_agent_conflict("nonexistent-agent", &managed).is_ok());
    }
}
