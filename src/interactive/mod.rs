use dialoguer::Select;
use console::Term;

pub fn select_skill(skills: &[crate::models::skill::Skill]) -> Option<usize> {
    if skills.is_empty() {
        return None;
    }

    let items: Vec<String> = skills
        .iter()
        .map(|s| format!("{} (v{}) - {}", s.name, s.version, s.description))
        .collect();

    let selection = Select::new()
        .with_prompt("Select a skill to install")
        .items(&items)
        .default(0)
        .interact_on_opt(&Term::stderr());

    match selection {
        Ok(Some(index)) => Some(index),
        _ => None,
    }
}
