use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn ensure_gitignore_entries(skill_name: &str) -> Result<()> {
    let gitignore_path = Path::new(".gitignore");
    let agents_path = format!(".agents/skills/{}", skill_name);
    let codex_path = format!(".codex/skills/{}", skill_name);

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

    if !lines.contains(&agents_path) {
        lines.push(agents_path);
        changed = true;
    }

    if !lines.contains(&codex_path) {
        lines.push(codex_path);
        changed = true;
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
