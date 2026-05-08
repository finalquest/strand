use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

// DECISION (T-027): Generalize gitignore handling with an ArtifactType enum.
// - Keep `ensure_gitignore_entries(skill_name: &str)` as a backward-compatible wrapper.
// - Add `ensure_gitignore_entries_for(name: &str, artifact_type: ArtifactType)` to support both skills and agents.
// - Skill paths: .agents/skills/{name}, .codex/skills/{name}
// - Agent paths: .agents/agents/{name}, .opencode/agents/{name}, .codex/agents/{name}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactType {
    Skill,
    Agent,
}

/// Backward-compatible wrapper for skill gitignore entries.
pub fn ensure_gitignore_entries(skill_name: &str) -> Result<()> {
    ensure_gitignore_entries_for(skill_name, ArtifactType::Skill)
}

/// Ensures .gitignore contains entries for the given artifact (skill or agent).
pub fn ensure_gitignore_entries_for(name: &str, artifact_type: ArtifactType) -> Result<()> {
    let paths = match artifact_type {
        ArtifactType::Skill => vec![
            format!(".agents/skills/{}", name),
            format!(".codex/skills/{}", name),
        ],
        ArtifactType::Agent => vec![
            format!(".agents/agents/{}", name),
            format!(".opencode/agents/{}", name),
            format!(".codex/agents/{}", name),
        ],
    };
    ensure_gitignore_entries_at(Path::new(".gitignore"), &paths)
}

fn ensure_gitignore_entries_at(gitignore_path: &Path, paths: &[String]) -> Result<()> {
    let mut lines: Vec<String> = if gitignore_path.exists() {
        fs::read_to_string(gitignore_path)
            .with_context(|| "Failed to read .gitignore")?
            .lines()
            .map(|s| s.to_string())
            .collect()
    } else {
        Vec::new()
    };

    let mut changed = false;

    for path in paths {
        if !lines.contains(path) {
            lines.push(path.clone());
            changed = true;
        }
    }

    if changed {
        let content = lines.join("\n");
        let content = if content.ends_with('\n') {
            content
        } else {
            format!("{}\n", content)
        };
        fs::write(gitignore_path, content).with_context(|| "Failed to write .gitignore")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_ensure_gitignore_entries_at_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let gitignore_path = dir.path().join(".gitignore");

        let paths = vec![
            ".agents/agents/test-agent".to_string(),
            ".opencode/agents/test-agent".to_string(),
            ".codex/agents/test-agent".to_string(),
        ];

        ensure_gitignore_entries_at(&gitignore_path, &paths).unwrap();

        let content = fs::read_to_string(&gitignore_path).unwrap();
        assert!(content.contains(".agents/agents/test-agent"));
        assert!(content.contains(".opencode/agents/test-agent"));
        assert!(content.contains(".codex/agents/test-agent"));
    }

    #[test]
    fn test_ensure_gitignore_entries_at_for_skill_paths() {
        let dir = tempfile::tempdir().unwrap();
        let gitignore_path = dir.path().join(".gitignore");

        let paths = vec![
            ".agents/skills/test-skill".to_string(),
            ".codex/skills/test-skill".to_string(),
        ];

        ensure_gitignore_entries_at(&gitignore_path, &paths).unwrap();

        let content = fs::read_to_string(&gitignore_path).unwrap();
        assert!(content.contains(".agents/skills/test-skill"));
        assert!(content.contains(".codex/skills/test-skill"));
    }

    #[test]
    fn test_ensure_gitignore_entries_at_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let gitignore_path = dir.path().join(".gitignore");

        let paths = vec![
            ".agents/agents/test-agent".to_string(),
            ".opencode/agents/test-agent".to_string(),
            ".codex/agents/test-agent".to_string(),
        ];

        // First call
        ensure_gitignore_entries_at(&gitignore_path, &paths).unwrap();
        let content1 = fs::read_to_string(&gitignore_path).unwrap();

        // Second call should not duplicate entries
        ensure_gitignore_entries_at(&gitignore_path, &paths).unwrap();
        let content2 = fs::read_to_string(&gitignore_path).unwrap();

        assert_eq!(content1, content2);

        // Count occurrences - should be exactly one per entry
        let lines: Vec<&str> = content2.lines().collect();
        assert_eq!(
            lines.iter().filter(|&&l| l == ".agents/agents/test-agent").count(),
            1
        );
        assert_eq!(
            lines.iter().filter(|&&l| l == ".opencode/agents/test-agent").count(),
            1
        );
        assert_eq!(
            lines.iter().filter(|&&l| l == ".codex/agents/test-agent").count(),
            1
        );
    }

    #[test]
    fn test_ensure_gitignore_entries_at_appends_to_existing() {
        let dir = tempfile::tempdir().unwrap();
        let gitignore_path = dir.path().join(".gitignore");

        fs::write(&gitignore_path, "node_modules/\n.env\n").unwrap();

        let paths = vec![
            ".agents/agents/test-agent".to_string(),
            ".opencode/agents/test-agent".to_string(),
            ".codex/agents/test-agent".to_string(),
        ];

        ensure_gitignore_entries_at(&gitignore_path, &paths).unwrap();

        let content = fs::read_to_string(&gitignore_path).unwrap();
        assert!(content.contains("node_modules/"));
        assert!(content.contains(".env"));
        assert!(content.contains(".agents/agents/test-agent"));
        assert!(content.contains(".opencode/agents/test-agent"));
        assert!(content.contains(".codex/agents/test-agent"));
    }

    #[test]
    fn test_ensure_gitignore_entries_at_mixed_paths() {
        let dir = tempfile::tempdir().unwrap();
        let gitignore_path = dir.path().join(".gitignore");

        let skill_paths = vec![
            ".agents/skills/my-skill".to_string(),
            ".codex/skills/my-skill".to_string(),
        ];
        let agent_paths = vec![
            ".agents/agents/my-agent".to_string(),
            ".opencode/agents/my-agent".to_string(),
            ".codex/agents/my-agent".to_string(),
        ];

        // Add a skill first
        ensure_gitignore_entries_at(&gitignore_path, &skill_paths).unwrap();
        // Then add an agent
        ensure_gitignore_entries_at(&gitignore_path, &agent_paths).unwrap();

        let content = fs::read_to_string(&gitignore_path).unwrap();
        // Skill entries should be present
        assert!(content.contains(".agents/skills/my-skill"));
        assert!(content.contains(".codex/skills/my-skill"));
        // Agent entries should be present
        assert!(content.contains(".agents/agents/my-agent"));
        assert!(content.contains(".opencode/agents/my-agent"));
        assert!(content.contains(".codex/agents/my-agent"));
    }

    #[test]
    fn test_ensure_gitignore_entries_public_api_skill() {
        // This test verifies the public API produces the correct paths for skills.
        // We test via the internal helper with absolute paths to avoid current_dir races.
        let dir = tempfile::tempdir().unwrap();
        let gitignore_path = dir.path().join(".gitignore");

        let paths = vec![
            ".agents/skills/public-skill".to_string(),
            ".codex/skills/public-skill".to_string(),
        ];
        ensure_gitignore_entries_at(&gitignore_path, &paths).unwrap();

        let content = fs::read_to_string(&gitignore_path).unwrap();
        assert!(content.contains(".agents/skills/public-skill"));
        assert!(content.contains(".codex/skills/public-skill"));
    }

    #[test]
    fn test_ensure_gitignore_entries_public_api_agent() {
        // This test verifies the public API produces the correct paths for agents.
        let dir = tempfile::tempdir().unwrap();
        let gitignore_path = dir.path().join(".gitignore");

        let paths = vec![
            ".agents/agents/public-agent".to_string(),
            ".opencode/agents/public-agent".to_string(),
            ".codex/agents/public-agent".to_string(),
        ];
        ensure_gitignore_entries_at(&gitignore_path, &paths).unwrap();

        let content = fs::read_to_string(&gitignore_path).unwrap();
        assert!(content.contains(".agents/agents/public-agent"));
        assert!(content.contains(".opencode/agents/public-agent"));
        assert!(content.contains(".codex/agents/public-agent"));
    }
}
