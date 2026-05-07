use std::path::Path;
use anyhow::Result;

#[derive(Debug, Clone, PartialEq)]
pub enum FixableIssue {
    MissingSkillMd,
    NeedsMigration,
    MissingMetadata,
    MissingName,
    MissingDescription,
    MissingVersion,
    InvalidVersion(String),
}

pub fn detect_fixable_issues(skill_path: &Path, _skill_name: &str) -> Vec<FixableIssue> {
    let skill_md = skill_path.join("SKILL.md");
    let skill_json = skill_path.join("skill.json");

    if !skill_md.exists() {
        if skill_json.exists() {
            return vec![FixableIssue::NeedsMigration];
        } else {
            return vec![FixableIssue::MissingSkillMd];
        }
    }

    let content = match std::fs::read_to_string(&skill_md) {
        Ok(c) => c,
        Err(_) => return vec![FixableIssue::MissingSkillMd],
    };

    let frontmatter = match crate::models::skill::parse_skill_md(&content) {
        Ok(f) => f,
        Err(_) => {
            if skill_json.exists() {
                return vec![FixableIssue::NeedsMigration];
            } else {
                return vec![FixableIssue::MissingMetadata];
            }
        }
    };

    let mut issues = Vec::new();

    if frontmatter.name.trim().is_empty() {
        issues.push(FixableIssue::MissingName);
    }

    if frontmatter.description.trim().is_empty() {
        issues.push(FixableIssue::MissingDescription);
    }

    if frontmatter.metadata.version.trim().is_empty() {
        issues.push(FixableIssue::MissingVersion);
    } else if semver::Version::parse(&frontmatter.metadata.version).is_err() {
        issues.push(FixableIssue::InvalidVersion(frontmatter.metadata.version.clone()));
    }

    issues
}

fn read_skill_md_parts(path: &Path) -> Result<(serde_yaml::Mapping, String)> {
    let content = std::fs::read_to_string(path)?;
    let trimmed = content.trim_start();
    if trimmed.starts_with("---") {
        let rest = &trimmed[3..];
        if let Some(end) = rest.find("---") {
            let yaml_str = &rest[..end];
            let mapping: serde_yaml::Mapping = serde_yaml::from_str(yaml_str).unwrap_or_default();
            let body = rest[end + 3..].to_string();
            return Ok((mapping, body));
        }
    }
    Ok((serde_yaml::Mapping::new(), content))
}

fn write_skill_md_parts(path: &Path, mapping: &serde_yaml::Mapping, body: &str) -> Result<()> {
    let yaml_str = serde_yaml::to_string(mapping)?;
    let new_content = format!("---\n{}---\n{}", yaml_str, body);
    std::fs::write(path, new_content)?;
    Ok(())
}

pub fn apply_fixes(skill_path: &Path, skill_name: &str, issues: &[FixableIssue]) -> Result<()> {
    let skill_md = skill_path.join("SKILL.md");
    let (mut mapping, body) = if skill_md.exists() {
        read_skill_md_parts(&skill_md)?
    } else {
        (serde_yaml::Mapping::new(), String::new())
    };

    let mut migrated = false;
    for issue in issues {
        match issue {
            FixableIssue::NeedsMigration => {
                let skill_json = skill_path.join("skill.json");
                let content = std::fs::read_to_string(&skill_json)?;
                let value: serde_json::Value = serde_json::from_str(&content)?;
                // Only use skill.json values for fields missing from SKILL.md
                if !mapping.contains_key(&serde_yaml::Value::String("name".to_string())) {
                    let name = value
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or(skill_name)
                        .to_string();
                    mapping.insert(
                        serde_yaml::Value::String("name".to_string()),
                        serde_yaml::Value::String(name),
                    );
                }
                if !mapping.contains_key(&serde_yaml::Value::String("description".to_string())) {
                    let description = value
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("No description provided")
                        .to_string();
                    mapping.insert(
                        serde_yaml::Value::String("description".to_string()),
                        serde_yaml::Value::String(description),
                    );
                }
                let version = value
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0.1.0")
                    .to_string();
                let mut meta = mapping
                    .get(&serde_yaml::Value::String("metadata".to_string()))
                    .and_then(|v| v.as_mapping())
                    .cloned()
                    .unwrap_or_default();
                meta.insert(
                    serde_yaml::Value::String("version".to_string()),
                    serde_yaml::Value::String(version),
                );
                mapping.insert(
                    serde_yaml::Value::String("metadata".to_string()),
                    serde_yaml::Value::Mapping(meta),
                );
                migrated = true;
            }
            FixableIssue::MissingSkillMd => {
                mapping.insert(
                    serde_yaml::Value::String("name".to_string()),
                    serde_yaml::Value::String(skill_name.to_string()),
                );
                mapping.insert(
                    serde_yaml::Value::String("description".to_string()),
                    serde_yaml::Value::String("No description provided".to_string()),
                );
                let mut meta = serde_yaml::Mapping::new();
                meta.insert(
                    serde_yaml::Value::String("version".to_string()),
                    serde_yaml::Value::String("0.0.1".to_string()),
                );
                mapping.insert(
                    serde_yaml::Value::String("metadata".to_string()),
                    serde_yaml::Value::Mapping(meta),
                );
            }
            FixableIssue::MissingName => {
                mapping.insert(
                    serde_yaml::Value::String("name".to_string()),
                    serde_yaml::Value::String(skill_name.to_string()),
                );
            }
            FixableIssue::MissingDescription => {
                mapping.insert(
                    serde_yaml::Value::String("description".to_string()),
                    serde_yaml::Value::String("No description provided".to_string()),
                );
            }
            FixableIssue::MissingVersion | FixableIssue::InvalidVersion(_) => {
                let mut meta = mapping
                    .get(&serde_yaml::Value::String("metadata".to_string()))
                    .and_then(|v| v.as_mapping())
                    .cloned()
                    .unwrap_or_default();
                meta.insert(
                    serde_yaml::Value::String("version".to_string()),
                    serde_yaml::Value::String("0.1.0".to_string()),
                );
                mapping.insert(
                    serde_yaml::Value::String("metadata".to_string()),
                    serde_yaml::Value::Mapping(meta),
                );
            }
            FixableIssue::MissingMetadata => {
                let mut meta = serde_yaml::Mapping::new();
                meta.insert(
                    serde_yaml::Value::String("version".to_string()),
                    serde_yaml::Value::String("0.1.0".to_string()),
                );
                mapping.insert(
                    serde_yaml::Value::String("metadata".to_string()),
                    serde_yaml::Value::Mapping(meta),
                );
            }
        }
    }

    let body_to_write = if migrated {
        if body.trim().is_empty() {
            "<!-- WARNING: This file was migrated from skill.json by strand -->\n\n".to_string()
        } else {
            body
        }
    } else if body.trim().is_empty() {
        "<!-- WARNING: This file was auto-generated by strand -->\n\n".to_string()
    } else {
        body
    };

    write_skill_md_parts(&skill_md, &mapping, &body_to_write)?;

    // After migration, remove the old skill.json
    let needs_migration = issues.iter().any(|i| matches!(i, FixableIssue::NeedsMigration));
    if needs_migration {
        let skill_json = skill_path.join("skill.json");
        if skill_json.exists() {
            std::fs::remove_file(&skill_json)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_skill_dir() -> PathBuf {
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("strand_test_{}", id));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cleanup(dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
    }

    fn write_skill_md(dir: &Path, content: &str) {
        std::fs::write(dir.join("SKILL.md"), content).unwrap();
    }

    #[test]
    fn test_detect_missing_name() {
        let dir = temp_skill_dir();
        write_skill_md(
            &dir,
            "---\nname: ''\ndescription: test\nmetadata:\n  version: 1.0.0\n---\n",
        );

        let issues = detect_fixable_issues(&dir, "test-skill");
        assert!(issues.contains(&FixableIssue::MissingName));
        assert!(!issues.contains(&FixableIssue::MissingDescription));

        cleanup(&dir);
    }

    #[test]
    fn test_detect_missing_description() {
        let dir = temp_skill_dir();
        write_skill_md(
            &dir,
            "---\nname: test\ndescription: ''\nmetadata:\n  version: 1.0.0\n---\n",
        );

        let issues = detect_fixable_issues(&dir, "test-skill");
        assert!(issues.contains(&FixableIssue::MissingDescription));
        cleanup(&dir);
    }

    #[test]
    fn test_detect_invalid_version() {
        let dir = temp_skill_dir();
        write_skill_md(
            &dir,
            "---\nname: test\ndescription: desc\nmetadata:\n  version: not-a-version\n---\n",
        );

        let issues = detect_fixable_issues(&dir, "test-skill");
        assert!(issues.contains(&FixableIssue::InvalidVersion("not-a-version".to_string())));
        cleanup(&dir);
    }

    #[test]
    fn test_detect_missing_version() {
        let dir = temp_skill_dir();
        write_skill_md(
            &dir,
            "---\nname: test\ndescription: desc\nmetadata:\n  version: ''\n---\n",
        );

        let issues = detect_fixable_issues(&dir, "test-skill");
        assert!(issues.contains(&FixableIssue::MissingVersion));
        cleanup(&dir);
    }

    #[test]
    fn test_detect_missing_skill_md() {
        let dir = temp_skill_dir();
        let issues = detect_fixable_issues(&dir, "test-skill");
        assert_eq!(issues, vec![FixableIssue::MissingSkillMd]);
        cleanup(&dir);
    }

    #[test]
    fn test_detect_needs_migration() {
        let dir = temp_skill_dir();
        let json = r#"{"name": "test", "description": "desc", "version": "1.0.0", "entrypoint": "SKILL.md"}"#;
        std::fs::write(dir.join("skill.json"), json).unwrap();

        let issues = detect_fixable_issues(&dir, "test-skill");
        assert_eq!(issues, vec![FixableIssue::NeedsMigration]);
        cleanup(&dir);
    }

    #[test]
    fn test_apply_fixes() {
        let dir = temp_skill_dir();
        write_skill_md(
            &dir,
            "---\nname: ''\ndescription: ''\nmetadata:\n  version: bad\n---\n",
        );

        let issues = vec![
            FixableIssue::MissingName,
            FixableIssue::MissingDescription,
            FixableIssue::InvalidVersion("bad".to_string()),
        ];

        apply_fixes(&dir, "my-skill", &issues).unwrap();

        let content = std::fs::read_to_string(dir.join("SKILL.md")).unwrap();
        let frontmatter = crate::models::skill::parse_skill_md(&content).unwrap();

        assert_eq!(frontmatter.name, "my-skill");
        assert_eq!(frontmatter.description, "No description provided");
        assert_eq!(frontmatter.metadata.version, "0.1.0");
        assert!(dir.join("SKILL.md").exists());

        let md_content = std::fs::read_to_string(dir.join("SKILL.md")).unwrap();
        assert!(md_content.contains("WARNING: This file was auto-generated by strand"));

        cleanup(&dir);
    }

    #[test]
    fn test_apply_fixes_missing_skill_md() {
        let dir = temp_skill_dir();
        let issues = vec![FixableIssue::MissingSkillMd];
        apply_fixes(&dir, "my-skill", &issues).unwrap();

        let content = std::fs::read_to_string(dir.join("SKILL.md")).unwrap();
        let frontmatter = crate::models::skill::parse_skill_md(&content).unwrap();

        assert_eq!(frontmatter.name, "my-skill");
        assert_eq!(frontmatter.description, "No description provided");
        assert_eq!(frontmatter.metadata.version, "0.0.1");
        assert!(dir.join("SKILL.md").exists());

        let md_content = std::fs::read_to_string(dir.join("SKILL.md")).unwrap();
        assert!(md_content.contains("WARNING: This file was auto-generated by strand"));

        cleanup(&dir);
    }

    #[test]
    fn test_apply_fixes_migration() {
        let dir = temp_skill_dir();
        let json = r#"{"name": "legacy", "description": "Legacy skill", "version": "2.0.0", "entrypoint": "SKILL.md"}"#;
        std::fs::write(dir.join("skill.json"), json).unwrap();

        let issues = vec![FixableIssue::NeedsMigration];
        apply_fixes(&dir, "legacy", &issues).unwrap();

        let content = std::fs::read_to_string(dir.join("SKILL.md")).unwrap();
        let frontmatter = crate::models::skill::parse_skill_md(&content).unwrap();

        assert_eq!(frontmatter.name, "legacy");
        assert_eq!(frontmatter.description, "Legacy skill");
        assert_eq!(frontmatter.metadata.version, "2.0.0");
        assert!(!dir.join("skill.json").exists(), "skill.json should be removed after migration");

        cleanup(&dir);
    }

    #[test]
    fn test_no_fixable_issues_for_valid_skill() {
        let dir = temp_skill_dir();
        write_skill_md(
            &dir,
            "---\nname: test\ndescription: desc\nmetadata:\n  version: 1.0.0\n---\n",
        );

        let issues = detect_fixable_issues(&dir, "test");
        assert!(issues.is_empty());

        cleanup(&dir);
    }

    #[test]
    fn test_no_fixable_issues_when_no_skill_md_and_no_skill_json() {
        let dir = temp_skill_dir();
        let issues = detect_fixable_issues(&dir, "test");
        assert_eq!(issues, vec![FixableIssue::MissingSkillMd]);
        cleanup(&dir);
    }
}
