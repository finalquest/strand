use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Pack {
    pub name: String,
    pub description: String,
    pub skills: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct PackFrontmatter {
    pub name: String,
    pub description: String,
    pub skills: Vec<String>,
}

impl PackFrontmatter {
    pub fn to_pack(self) -> Pack {
        Pack {
            name: self.name,
            description: self.description,
            skills: self.skills,
        }
    }
}

pub fn parse_pack_md(content: &str) -> Result<PackFrontmatter, String> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return Err("No YAML frontmatter found".to_string());
    }
    let rest = &trimmed[3..];
    let Some(end) = rest.find("---") else {
        return Err("YAML frontmatter not closed".to_string());
    };
    let yaml = &rest[..end];
    let frontmatter: PackFrontmatter = serde_yaml::from_str(yaml)
        .map_err(|e| format!("Failed to parse YAML frontmatter: {}", e))?;
    Ok(frontmatter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pack_md_valid() {
        let content = r#"---
name: llm-documenter
description: Documentation skills pack
skills:
  - documenter/scaffolder
  - documenter/change-orchestrator
  - documenter/diff-analyzer
---
"#;
        let result = parse_pack_md(content);
        assert!(result.is_ok());
        let pack = result.unwrap();
        assert_eq!(pack.name, "llm-documenter");
        assert_eq!(pack.description, "Documentation skills pack");
        assert_eq!(pack.skills.len(), 3);
        assert_eq!(pack.skills[0], "documenter/scaffolder");
    }

    #[test]
    fn test_parse_pack_md_no_frontmatter() {
        let content = "just some text";
        let result = parse_pack_md(content);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No YAML frontmatter"));
    }

    #[test]
    fn test_parse_pack_md_unclosed() {
        let content = "---\nname: test\n";
        let result = parse_pack_md(content);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not closed"));
    }

    #[test]
    fn test_parse_pack_md_empty_skills() {
        let content = r#"---
name: empty-pack
description: No skills
skills: []
---
"#;
        let result = parse_pack_md(content);
        assert!(result.is_ok());
        let pack = result.unwrap();
        assert!(pack.skills.is_empty());
    }

    #[test]
    fn test_to_pack_conversion() {
        let content = r#"---
name: test-pack
description: Test
skills:
  - standalone-skill
---
"#;
        let frontmatter = parse_pack_md(content).unwrap();
        let pack = frontmatter.to_pack();
        assert_eq!(pack.name, "test-pack");
        assert_eq!(pack.description, "Test");
        assert_eq!(pack.skills, vec!["standalone-skill"]);
    }
}
