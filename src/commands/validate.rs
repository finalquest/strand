use std::path::Path;

use anyhow::Result;
use semver::Version;

use crate::fix::{apply_fixes, detect_fixable_issues, FixableIssue};
use crate::models::skill::Skill;
use crate::report::{print_report, SkillReport};

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub skill_name: String,
    pub issue: String,
    pub critical: bool,
}

pub struct SkillResult {
    pub name: String,
    pub path: std::path::PathBuf,
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub fixable_issues: Vec<FixableIssue>,
}

pub fn execute() -> Result<()> {
    let skills_dir = Path::new("skills");

    if !skills_dir.exists() {
        eprintln!("Error: skills/ directory not found");
        std::process::exit(1);
    }

    let mut skill_results: Vec<SkillResult> = Vec::new();
    let mut valid_count = 0;
    let mut invalid_count = 0;

    for entry in std::fs::read_dir(skills_dir)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let skill_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let result = validate_skill(&path, &skill_name);

        if result.valid {
            valid_count += 1;
        } else {
            invalid_count += 1;
        }

        skill_results.push(result);
    }

    // Generate report
    let reports: Vec<SkillReport> = skill_results
        .iter()
        .map(|r| SkillReport {
            name: r.name.clone(),
            valid: r.valid,
            issues: r.errors.iter().map(|e| e.issue.clone()).collect(),
        })
        .collect();

    print_report(&reports, valid_count, invalid_count);

    // Handle migration from skill.json to SKILL.md
    let migration_results: Vec<&SkillResult> = skill_results
        .iter()
        .filter(|r| {
            r.fixable_issues
                .iter()
                .any(|f| matches!(f, FixableIssue::NeedsMigration))
        })
        .collect();

    if !migration_results.is_empty() {
        println!(
            "\nFound {} skill(s) with skill.json but no metadata in SKILL.md.",
            migration_results.len()
        );

        let term = console::Term::stderr();
        if term.is_term() {
            let should_migrate = dialoguer::Confirm::new()
                .with_prompt("Would you like to auto-migrate them?")
                .default(false)
                .interact()?;

            if should_migrate {
                for result in &skill_results {
                    if result
                        .fixable_issues
                        .iter()
                        .any(|f| matches!(f, FixableIssue::NeedsMigration))
                    {
                        match apply_fixes(&result.path, &result.name, &result.fixable_issues) {
                            Ok(_) => println!("✓ Migrated {}", result.name),
                            Err(e) => eprintln!("✗ Failed to migrate {}: {}", result.name, e),
                        }
                    }
                }
            }
        } else {
            println!("Run interactively to auto-migrate skills from skill.json to SKILL.md.");
        }
    }

    // Check for auto-fixable issues
    let fixable_results: Vec<&SkillResult> = skill_results
        .iter()
        .filter(|r| {
            !r.fixable_issues.is_empty()
                && !r.fixable_issues
                    .iter()
                    .any(|f| matches!(f, FixableIssue::NeedsMigration))
        })
        .collect();

    if !fixable_results.is_empty() {
        let total_fixable: usize = fixable_results.iter().map(|r| r.fixable_issues.len()).sum();
        println!(
            "\nFound {} auto-fixable issue(s) in {} skill(s).",
            total_fixable,
            fixable_results.len()
        );

        let term = console::Term::stderr();
        if term.is_term() {
            let should_fix = dialoguer::Confirm::new()
                .with_prompt("Would you like to apply auto-fixes?")
                .default(false)
                .interact()?;

            if should_fix {
                for result in &skill_results {
                    if !result.fixable_issues.is_empty()
                        && !result
                            .fixable_issues
                            .iter()
                            .any(|f| matches!(f, FixableIssue::NeedsMigration))
                    {
                        match apply_fixes(&result.path, &result.name, &result.fixable_issues) {
                            Ok(_) => println!("✓ Fixed {}", result.name),
                            Err(e) => eprintln!("✗ Failed to fix {}: {}", result.name, e),
                        }
                    }
                }
            }
        } else {
            println!("Run interactively to apply auto-fixes.");
        }
    }

    if invalid_count > 0 {
        let total = valid_count + invalid_count;
        eprintln!(
            "\nValidation failed: {}/{} skill(s) have errors",
            invalid_count, total
        );
        std::process::exit(1);
    }

    Ok(())
}

pub fn validate_skill(path: &Path, skill_name: &str) -> SkillResult {
    let mut errors = Vec::new();

    // Detect fixable issues first
    let fixable = detect_fixable_issues(path, skill_name);

    // Parse SKILL.md if it exists and is readable
    let manifest = if fixable.iter().any(|f| {
        matches!(
            f,
            FixableIssue::MissingSkillMd
                | FixableIssue::NeedsMigration
                | FixableIssue::MissingMetadata
        )
    }) {
        if fixable.iter().any(|f| matches!(f, FixableIssue::MissingSkillMd)) {
            errors.push(ValidationError {
                skill_name: skill_name.to_string(),
                issue: "SKILL.md not found".to_string(),
                critical: true,
            });
        }
        if fixable.iter().any(|f| matches!(f, FixableIssue::MissingMetadata)) {
            errors.push(ValidationError {
                skill_name: skill_name.to_string(),
                issue: "SKILL.md has no valid YAML frontmatter".to_string(),
                critical: true,
            });
        }
        if fixable.iter().any(|f| matches!(f, FixableIssue::NeedsMigration)) {
            errors.push(ValidationError {
                skill_name: skill_name.to_string(),
                issue: "skill.json found but SKILL.md missing. Run validate interactively to migrate.".to_string(),
                critical: true,
            });
        }
        None
    } else {
        match parse_skill_md_manifest(path, skill_name) {
            Ok(m) => Some(m),
            Err(e) => {
                errors.push(e);
                None
            }
        }
    };

    if let Some(ref manifest) = manifest {
        // Validate required fields
        if manifest.name.trim().is_empty() {
            errors.push(ValidationError {
                skill_name: skill_name.to_string(),
                issue: "name is empty".to_string(),
                critical: false,
            });
        }

        if manifest.description.trim().is_empty() {
            errors.push(ValidationError {
                skill_name: skill_name.to_string(),
                issue: "description is empty".to_string(),
                critical: false,
            });
        }

        // Validate semver version
        if Version::parse(&manifest.version).is_err() {
            errors.push(ValidationError {
                skill_name: skill_name.to_string(),
                issue: format!("version '{}' is not valid semver", manifest.version),
                critical: false,
            });
        }
    }

    // Validate assets directory if present
    let assets_dir = path.join("assets");
    if assets_dir.exists() && assets_dir.is_dir() {
        if let Err(e) = validate_assets(&assets_dir, skill_name) {
            errors.push(e);
        }
    }

    let valid = errors.is_empty();

    SkillResult {
        name: skill_name.to_string(),
        path: path.to_path_buf(),
        valid,
        errors,
        fixable_issues: fixable,
    }
}

pub fn parse_skill_md_manifest(path: &Path, skill_name: &str) -> Result<Skill, ValidationError> {
    let skill_md = path.join("SKILL.md");

    if !skill_md.exists() {
        return Err(ValidationError {
            skill_name: skill_name.to_string(),
            issue: "SKILL.md not found".to_string(),
            critical: true,
        });
    }

    let content = std::fs::read_to_string(&skill_md).map_err(|e| ValidationError {
        skill_name: skill_name.to_string(),
        issue: format!("failed to read SKILL.md: {}", e),
        critical: true,
    })?;

    let frontmatter =
        crate::models::skill::parse_skill_md(&content).map_err(|e| ValidationError {
            skill_name: skill_name.to_string(),
            issue: format!("failed to parse SKILL.md: {}", e),
            critical: true,
        })?;

    Ok(frontmatter.to_skill())
}

fn validate_assets(assets_dir: &Path, skill_name: &str) -> Result<(), ValidationError> {
    for entry in std::fs::read_dir(assets_dir).map_err(|e| ValidationError {
        skill_name: skill_name.to_string(),
        issue: format!("failed to read assets directory: {}", e),
        critical: true,
    })? {
        let entry = entry.map_err(|e| ValidationError {
            skill_name: skill_name.to_string(),
            issue: format!("failed to read asset entry: {}", e),
            critical: true,
        })?;

        let path = entry.path();
        if path.is_file() {
            // Verify file is readable
            let _ = std::fs::metadata(&path).map_err(|e| ValidationError {
                skill_name: skill_name.to_string(),
                issue: format!(
                    "asset '{}' is not readable: {}",
                    path.file_name().unwrap_or_default().to_string_lossy(),
                    e
                ),
                critical: true,
            })?;
        }
    }

    Ok(())
}
