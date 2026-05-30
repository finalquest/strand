use anyhow::{Context, Result};
use dialoguer::{Input, MultiSelect};
use std::fs;
use std::io::IsTerminal;
use std::path::Path;

pub fn execute() -> Result<()> {
    if !std::io::stdin().is_terminal() {
        anyhow::bail!("create-pack requires an interactive terminal");
    }

    let skills_dir = Path::new("skills");
    if !skills_dir.exists() || !skills_dir.is_dir() {
        anyhow::bail!("skills/ directory not found. Run this command from the root of a skills repo.");
    }

    let discovered = discover_skills(skills_dir)?;

    if discovered.is_empty() {
        anyhow::bail!("No skills found under skills/. Each skill must have a SKILL.md file.");
    }

    println!("Found {} skill(s):\n", discovered.len());
    for (i, skill) in discovered.iter().enumerate() {
        println!("  {}) {}", i + 1, skill);
    }
    println!();

    let items: Vec<String> = discovered.clone();
    let selections = MultiSelect::new()
        .with_prompt("Select skills to include in the pack (space to toggle, enter to confirm)")
        .items(&items)
        .interact()?;

    if selections.is_empty() {
        println!("No skills selected. Aborting.");
        return Ok(());
    }

    let pack_name: String = Input::new()
        .with_prompt("Pack name")
        .interact()?;

    if pack_name.trim().is_empty() {
        anyhow::bail!("Pack name cannot be empty");
    }

    let pack_description: String = Input::new()
        .with_prompt("Pack description")
        .interact()?;

    let selected_skills: Vec<String> = selections
        .into_iter()
        .map(|i| discovered[i].clone())
        .collect();

    let pack_dir = Path::new("packs").join(&pack_name);
    fs::create_dir_all(&pack_dir)
        .with_context(|| format!("Failed to create directory {}", pack_dir.display()))?;

    let pack_md_path = pack_dir.join("pack.md");
    let mut content = String::from("---\n");
    content.push_str(&format!("name: {}\n", pack_name));
    content.push_str(&format!("description: {}\n", pack_description));
    content.push_str("skills:\n");
    for skill in &selected_skills {
        content.push_str(&format!("  - {}\n", skill));
    }
    content.push_str("---\n");

    fs::write(&pack_md_path, &content)
        .with_context(|| format!("Failed to write {}", pack_md_path.display()))?;

    println!(
        "\nCreated pack '{}' with {} skill(s) at {}",
        pack_name,
        selected_skills.len(),
        pack_md_path.display()
    );

    Ok(())
}

fn discover_skills(skills_dir: &Path) -> Result<Vec<String>> {
    let mut skills = Vec::new();

    let entries = fs::read_dir(skills_dir)
        .with_context(|| format!("Failed to read {}", skills_dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let dir_name = entry.file_name().to_string_lossy().to_string();

        if path.join("SKILL.md").exists() {
            skills.push(dir_name);
        } else {
            let sub_entries = fs::read_dir(&path)
                .with_context(|| format!("Failed to read {}", path.display()))?;

            for sub_entry in sub_entries {
                let sub_entry = sub_entry?;
                let sub_path = sub_entry.path();

                if !sub_path.is_dir() {
                    continue;
                }

                if sub_path.join("SKILL.md").exists() {
                    let sub_name = sub_entry.file_name().to_string_lossy().to_string();
                    skills.push(format!("{}/{}", dir_name, sub_name));
                }
            }
        }
    }

    skills.sort();
    Ok(skills)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    static DIR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn create_skill(base: &Path, path: &str) {
        let skill_dir = base.join("skills").join(path);
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: test\ndescription: test\nmetadata:\n  version: \"1.0.0\"\n---\n",
        )
        .unwrap();
    }

    #[test]
    fn test_discover_standalone_skill() {
        let _guard = DIR_LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        create_skill(temp.path(), "my-skill");

        let skills = discover_skills(&temp.path().join("skills")).unwrap();
        assert_eq!(skills, vec!["my-skill"]);
    }

    #[test]
    fn test_discover_nested_skill() {
        let _guard = DIR_LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        create_skill(temp.path(), "documenter/skill-1");

        let skills = discover_skills(&temp.path().join("skills")).unwrap();
        assert_eq!(skills, vec!["documenter/skill-1"]);
    }

    #[test]
    fn test_discover_mixed_skills() {
        let _guard = DIR_LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        create_skill(temp.path(), "standalone");
        create_skill(temp.path(), "documenter/skill-1");
        create_skill(temp.path(), "documenter/skill-2");

        let skills = discover_skills(&temp.path().join("skills")).unwrap();
        assert_eq!(
            skills,
            vec!["documenter/skill-1", "documenter/skill-2", "standalone"]
        );
    }

    #[test]
    fn test_discover_empty_dir() {
        let _guard = DIR_LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let skills_dir = temp.path().join("skills");
        fs::create_dir_all(&skills_dir).unwrap();

        let skills = discover_skills(&skills_dir).unwrap();
        assert!(skills.is_empty());
    }

    #[test]
    fn test_discover_ignores_dirs_without_skill_md() {
        let _guard = DIR_LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let empty_dir = temp.path().join("skills").join("no-skill");
        fs::create_dir_all(&empty_dir).unwrap();

        let skills = discover_skills(&temp.path().join("skills")).unwrap();
        assert!(skills.is_empty());
    }
}
