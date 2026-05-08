use std::path::Path;

use anyhow::Result;
use semver::Version;

use crate::models::agent::Agent;

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub agent_name: String,
    pub issue: String,
    pub critical: bool,
}

pub struct AgentResult {
    pub name: String,
    pub path: std::path::PathBuf,
    pub valid: bool,
    pub errors: Vec<ValidationError>,
}

pub fn execute() -> Result<()> {
    let agents_dir = Path::new(".agents/agents");

    if !agents_dir.exists() {
        eprintln!("Error: .agents/agents/ directory not found");
        std::process::exit(1);
    }

    let mut agent_results: Vec<AgentResult> = Vec::new();
    let mut valid_count = 0;
    let mut invalid_count = 0;

    for entry in std::fs::read_dir(agents_dir)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let agent_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let result = validate_agent(&path, &agent_name);

        if result.valid {
            valid_count += 1;
        } else {
            invalid_count += 1;
        }

        agent_results.push(result);
    }

    // Generate report
    print_report(&agent_results, valid_count, invalid_count);

    if invalid_count > 0 {
        let total = valid_count + invalid_count;
        eprintln!(
            "\nValidation failed: {}/{} agent(s) have errors",
            invalid_count, total
        );
        std::process::exit(1);
    }

    Ok(())
}

pub fn validate_agent(path: &Path, agent_name: &str) -> AgentResult {
    let mut errors = Vec::new();

    // Parse AGENT.md if it exists and is readable
    let manifest = match parse_agent_md_manifest(path, agent_name) {
        Ok(m) => Some(m),
        Err(e) => {
            errors.push(e);
            None
        }
    };

    if let Some(ref manifest) = manifest {
        // Validate required fields
        if manifest.name.trim().is_empty() {
            errors.push(ValidationError {
                agent_name: agent_name.to_string(),
                issue: "name is empty".to_string(),
                critical: false,
            });
        }

        if manifest.description.trim().is_empty() {
            errors.push(ValidationError {
                agent_name: agent_name.to_string(),
                issue: "description is empty".to_string(),
                critical: false,
            });
        }

        // Validate semver version
        if Version::parse(&manifest.version).is_err() {
            errors.push(ValidationError {
                agent_name: agent_name.to_string(),
                issue: format!("version '{}' is not valid semver", manifest.version),
                critical: false,
            });
        }
    }

    let valid = errors.is_empty();

    AgentResult {
        name: agent_name.to_string(),
        path: path.to_path_buf(),
        valid,
        errors,
    }
}

pub fn parse_agent_md_manifest(path: &Path, agent_name: &str) -> Result<Agent, ValidationError> {
    let agent_md = path.join("AGENT.md");

    if !agent_md.exists() {
        return Err(ValidationError {
            agent_name: agent_name.to_string(),
            issue: "AGENT.md not found".to_string(),
            critical: true,
        });
    }

    let content = std::fs::read_to_string(&agent_md).map_err(|e| ValidationError {
        agent_name: agent_name.to_string(),
        issue: format!("failed to read AGENT.md: {}", e),
        critical: true,
    })?;

    let frontmatter =
        crate::models::agent::parse_agent_md(&content).map_err(|e| ValidationError {
            agent_name: agent_name.to_string(),
            issue: format!("failed to parse AGENT.md: {}", e),
            critical: true,
        })?;

    Ok(frontmatter.to_agent())
}

fn print_report(results: &[AgentResult], valid_count: usize, invalid_count: usize) {
    use colored::*;

    let agent_col_width = results.iter().map(|r| r.name.len()).max().unwrap_or(0).max(20);
    let table_width = (agent_col_width + 1 + 10 + 1 + 6).max(60);

    println!("{}", "─".repeat(table_width));
    println!(
        "{}",
        format!(
            "{:<width$} {:<10} {}",
            "Agent",
            "Status",
            "Issues",
            width = agent_col_width
        )
        .bold()
    );
    println!("{}", "─".repeat(table_width));

    for result in results {
        let status_text = if result.valid { "valid" } else { "invalid" };
        let status = format!("{:<10}", status_text);
        let colored_status = if result.valid {
            status.green()
        } else {
            status.red()
        };

        let issues_str = if result.errors.is_empty() {
            "-".dimmed().to_string()
        } else {
            result.errors.iter().map(|e| e.issue.clone()).collect::<Vec<_>>().join(", ")
        };

        println!(
            "{:<width$} {} {}",
            result.name,
            colored_status,
            issues_str,
            width = agent_col_width
        );
    }

    println!("{}", "─".repeat(table_width));
    println!(
        "Summary: {} valid, {} invalid",
        valid_count.to_string().green().bold(),
        invalid_count.to_string().red().bold()
    );
}
