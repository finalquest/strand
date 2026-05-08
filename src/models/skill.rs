use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub version: String,
    #[serde(default)]
    pub agents: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SkillFrontmatter {
    pub name: String,
    pub description: String,
    pub metadata: SkillMetadata,
}

#[derive(Debug, Deserialize)]
pub struct SkillMetadata {
    pub version: String,
    #[serde(default)]
    pub agents: Vec<String>,
}

impl SkillFrontmatter {
    pub fn to_skill(self) -> Skill {
        Skill {
            name: self.name,
            description: self.description,
            version: self.metadata.version,
            agents: self.metadata.agents,
        }
    }
}

pub fn parse_skill_md(content: &str) -> Result<SkillFrontmatter, String> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return Err("No YAML frontmatter found".to_string());
    }
    let rest = &trimmed[3..];
    let Some(end) = rest.find("---") else {
        return Err("YAML frontmatter not closed".to_string());
    };
    let yaml = &rest[..end];
    let frontmatter: SkillFrontmatter = serde_yaml::from_str(yaml)
        .map_err(|e| format!("Failed to parse YAML frontmatter: {}", e))?;
    Ok(frontmatter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_skill_md_with_agents() {
        let content = r#"---
name: board-task-orchestrator
description: Orquesta tareas de Planka
metadata:
  version: "1.0.0"
  agents:
    - task-implementer
    - task-reviewer
---
"#;
        let result = parse_skill_md(content);
        assert!(result.is_ok());
        let frontmatter = result.unwrap();
        assert_eq!(frontmatter.name, "board-task-orchestrator");
        assert_eq!(frontmatter.metadata.agents, vec!["task-implementer", "task-reviewer"]);
    }

    #[test]
    fn test_parse_skill_md_without_agents() {
        let content = r#"---
name: simple-skill
description: A simple skill
metadata:
  version: "1.0.0"
---
"#;
        let result = parse_skill_md(content);
        assert!(result.is_ok());
        let frontmatter = result.unwrap();
        assert_eq!(frontmatter.metadata.agents, Vec::<String>::new());
    }

    #[test]
    fn test_agents_propagate_to_skill() {
        let content = r#"---
name: orchestrator
description: Orchestrates tasks
metadata:
  version: "2.0.0"
  agents:
    - agent-a
    - agent-b
---
"#;
        let frontmatter = parse_skill_md(content).unwrap();
        let skill = frontmatter.to_skill();
        assert_eq!(skill.name, "orchestrator");
        assert_eq!(skill.version, "2.0.0");
        assert_eq!(skill.agents, vec!["agent-a", "agent-b"]);
    }

    #[test]
    fn test_no_agents_propagate_to_skill() {
        let content = r#"---
name: simple
description: Simple skill
metadata:
  version: "1.0.0"
---
"#;
        let frontmatter = parse_skill_md(content).unwrap();
        let skill = frontmatter.to_skill();
        assert_eq!(skill.agents, Vec::<String>::new());
    }
}
