use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Agent {
    pub name: String,
    pub description: String,
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct AgentFrontmatter {
    pub name: String,
    pub description: String,
    pub metadata: AgentMetadata,
}

#[derive(Debug, Deserialize)]
pub struct AgentMetadata {
    pub version: String,
}

impl AgentFrontmatter {
    pub fn to_agent(self) -> Agent {
        Agent {
            name: self.name,
            description: self.description,
            version: self.metadata.version,
        }
    }
}

pub fn parse_agent_md(content: &str) -> Result<AgentFrontmatter, String> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return Err("No YAML frontmatter found".to_string());
    }
    let rest = &trimmed[3..];
    let Some(end) = rest.find("---") else {
        return Err("YAML frontmatter not closed".to_string());
    };
    let yaml = &rest[..end];
    let frontmatter: AgentFrontmatter = serde_yaml::from_str(yaml)
        .map_err(|e| format!("Failed to parse YAML frontmatter: {}", e))?;
    Ok(frontmatter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_agent_md_valid() {
        let content = r#"---
name: test-agent
description: A test agent
metadata:
  version: 1.0.0
---
# Body
This is the agent body.
"#;
        let result = parse_agent_md(content);
        assert!(result.is_ok());
        let frontmatter = result.unwrap();
        assert_eq!(frontmatter.name, "test-agent");
        assert_eq!(frontmatter.description, "A test agent");
        assert_eq!(frontmatter.metadata.version, "1.0.0");
    }

    #[test]
    fn test_parse_agent_md_to_agent() {
        let content = r#"---
name: my-agent
description: My agent description
metadata:
  version: 2.1.0
---
"#;
        let frontmatter = parse_agent_md(content).unwrap();
        let agent = frontmatter.to_agent();
        assert_eq!(agent.name, "my-agent");
        assert_eq!(agent.description, "My agent description");
        assert_eq!(agent.version, "2.1.0");
    }

    #[test]
    fn test_parse_agent_md_missing_frontmatter() {
        let content = "No frontmatter here\nJust body content\n";
        let result = parse_agent_md(content);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "No YAML frontmatter found");
    }

    #[test]
    fn test_parse_agent_md_unclosed_frontmatter() {
        let content = "---\nname: test\ndescription: test\n";
        let result = parse_agent_md(content);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "YAML frontmatter not closed");
    }

    #[test]
    fn test_parse_agent_md_malformed_yaml() {
        let content = "---\nname: test\ndescription: test\nmetadata\n  version: 1.0.0\n---\n";
        let result = parse_agent_md(content);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.starts_with("Failed to parse YAML frontmatter:"));
    }

    #[test]
    fn test_parse_agent_md_missing_field() {
        let content = "---\nname: test-agent\nmetadata:\n  version: 1.0.0\n---\n";
        let result = parse_agent_md(content);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.starts_with("Failed to parse YAML frontmatter:"));
    }

    #[test]
    fn test_parse_agent_md_empty_body() {
        let content = "---\nname: empty-agent\ndescription: Empty body test\nmetadata:\n  version: 0.0.1\n---";
        let result = parse_agent_md(content);
        assert!(result.is_ok());
        let frontmatter = result.unwrap();
        assert_eq!(frontmatter.name, "empty-agent");
    }

    #[test]
    fn test_parse_agent_md_leading_whitespace() {
        let content = "   \n\n---\nname: whitespace-agent\ndescription: Leading whitespace test\nmetadata:\n  version: 1.0.0\n---\n";
        let result = parse_agent_md(content);
        assert!(result.is_ok());
        let frontmatter = result.unwrap();
        assert_eq!(frontmatter.name, "whitespace-agent");
    }
}
