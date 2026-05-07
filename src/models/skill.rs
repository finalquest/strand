use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub version: String,
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
}

impl SkillFrontmatter {
    pub fn to_skill(self) -> Skill {
        Skill {
            name: self.name,
            description: self.description,
            version: self.metadata.version,
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
