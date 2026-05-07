use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

use strand::commands::validate::validate_skill;
use strand::fix::{apply_fixes, detect_fixable_issues, FixableIssue};
use strand::report::{format_report, SkillReport};

// ── Helpers ──────────────────────────────────────────────────────────

fn create_skill_dir(base: &Path, name: &str, skill_md: &str) -> std::path::PathBuf {
    let dir = base.join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("SKILL.md"), skill_md).unwrap();
    dir
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn setup_skills_dir(temp_dir: &TempDir, fixtures: &[&str]) {
    let skills_dir = temp_dir.path().join("skills");
    fs::create_dir_all(&skills_dir).unwrap();
    for fixture in fixtures {
        let src = Path::new("tests/fixtures").join(fixture);
        let dst = skills_dir.join(fixture);
        copy_dir_all(&src, &dst).unwrap();
    }
}

// ── Acceptance: valid skills repository passes validation ────────────

#[test]
fn test_valid_skill_passes_validate() {
    let temp = TempDir::new().unwrap();
    let dir = create_skill_dir(
        temp.path(),
        "valid-skill",
        "---\nname: valid-skill\ndescription: A valid skill\nmetadata:\n  version: 1.0.0\n---\n",
    );

    let result = validate_skill(&dir, "valid-skill");
    assert!(result.valid, "Expected valid skill, got errors: {:?}", result.errors);
    assert!(result.errors.is_empty());
}

#[test]
fn test_valid_skill_with_assets_passes_validate() {
    let temp = TempDir::new().unwrap();
    let dir = create_skill_dir(
        temp.path(),
        "valid-with-assets",
        "---\nname: valid-with-assets\ndescription: Has assets\nmetadata:\n  version: 1.0.0\n---\n",
    );
    let assets = dir.join("assets");
    fs::create_dir_all(&assets).unwrap();
    fs::write(assets.join("sample.txt"), "hello").unwrap();

    let result = validate_skill(&dir, "valid-with-assets");
    assert!(result.valid, "Expected valid skill, got errors: {:?}", result.errors);
}

// ── Acceptance: missing SKILL.md reports critical error ──────────────

#[test]
fn test_missing_skill_md_reports_critical_error_on_validate() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path().join("no-md-skill");
    fs::create_dir_all(&dir).unwrap();

    let result = validate_skill(&dir, "no-md-skill");
    assert!(!result.valid);
    assert_eq!(result.errors.len(), 1);
    assert!(result.errors[0].issue.contains("SKILL.md not found"));
    assert!(result.errors[0].critical);
}

// ── Acceptance: invalid YAML in SKILL.md reports critical error ──────

#[test]
fn test_invalid_yaml_reports_critical_error_on_validate() {
    let temp = TempDir::new().unwrap();
    let dir = create_skill_dir(
        temp.path(),
        "bad-yaml-skill",
        "---\n{ invalid yaml\n---\n",
    );

    let result = validate_skill(&dir, "bad-yaml-skill");
    assert!(!result.valid);
    assert_eq!(result.errors.len(), 1);
    assert!(result.errors[0].issue.contains("SKILL.md has no valid YAML frontmatter"));
    assert!(result.errors[0].critical);
}

// ── Acceptance: missing fields are detected and reported ─────────────

#[test]
fn test_missing_fields_detected_on_validate() {
    let temp = TempDir::new().unwrap();
    let dir = create_skill_dir(
        temp.path(),
        "missing-fields",
        "---\nname: ''\ndescription: ''\nmetadata:\n  version: 1.0.0\n---\n",
    );

    let result = validate_skill(&dir, "missing-fields");
    assert!(!result.valid);
    let issues: Vec<String> = result.errors.iter().map(|e| e.issue.clone()).collect();
    assert!(
        issues.iter().any(|i| i.contains("name is empty")),
        "Expected 'name is empty' in {:?}",
        issues
    );
    assert!(
        issues.iter().any(|i| i.contains("description is empty")),
        "Expected 'description is empty' in {:?}",
        issues
    );
}

// ── Acceptance: invalid semver version is detected ───────────────────

#[test]
fn test_invalid_semver_detected_on_validate() {
    let temp = TempDir::new().unwrap();
    let dir = create_skill_dir(
        temp.path(),
        "bad-version",
        "---\nname: bad-version\ndescription: test\nmetadata:\n  version: not-a-version\n---\n",
    );

    let result = validate_skill(&dir, "bad-version");
    assert!(!result.valid);
    let issues: Vec<String> = result.errors.iter().map(|e| e.issue.clone()).collect();
    assert!(
        issues.iter().any(|i| i.contains("version") && i.contains("not valid semver")),
        "Expected semver error in {:?}",
        issues
    );
}

// ── Acceptance: auto-fix applies correct fixes to SKILL.md ───────────

#[test]
fn test_auto_fix_applies_correct_fixes_on_validate() {
    let temp = TempDir::new().unwrap();
    let dir = create_skill_dir(
        temp.path(),
        "fixable-skill",
        "---\nname: ''\ndescription: ''\nmetadata:\n  version: bad-version\n---\n",
    );

    let issues = detect_fixable_issues(&dir, "fixable-skill");
    assert!(!issues.is_empty());
    assert!(issues.contains(&FixableIssue::MissingName));
    assert!(issues.contains(&FixableIssue::MissingDescription));
    assert!(issues.contains(&FixableIssue::InvalidVersion("bad-version".to_string())));

    apply_fixes(&dir, "fixable-skill", &issues).unwrap();

    let fixed_content = fs::read_to_string(dir.join("SKILL.md")).unwrap();
    let frontmatter = strand::models::skill::parse_skill_md(&fixed_content).unwrap();

    assert_eq!(frontmatter.name, "fixable-skill");
    assert_eq!(frontmatter.description, "No description provided");
    assert_eq!(frontmatter.metadata.version, "0.1.0");
}

// ── Acceptance: auto-fix creates missing SKILL.md and validate passes ─

#[test]
fn test_auto_fix_creates_missing_skill_md_on_validate() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path().join("missing-md-skill");
    fs::create_dir_all(&dir).unwrap();

    let issues = detect_fixable_issues(&dir, "missing-md-skill");
    assert!(issues.contains(&FixableIssue::MissingSkillMd));

    apply_fixes(&dir, "missing-md-skill", &issues).unwrap();

    assert!(dir.join("SKILL.md").exists());
    let content = fs::read_to_string(dir.join("SKILL.md")).unwrap();
    let frontmatter = strand::models::skill::parse_skill_md(&content).unwrap();

    assert_eq!(frontmatter.name, "missing-md-skill");
    assert_eq!(frontmatter.description, "No description provided");
    assert_eq!(frontmatter.metadata.version, "0.0.1");

    let md_content = fs::read_to_string(dir.join("SKILL.md")).unwrap();
    assert!(md_content.contains("WARNING: This file was auto-generated by strand"));

    // Validate should pass after fix
    let result = validate_skill(&dir, "missing-md-skill");
    assert!(result.valid, "Expected validate to pass after fix, got errors: {:?}", result.errors);
}

// ── Acceptance: migration from skill.json to SKILL.md ────────────────

#[test]
fn test_migration_detected_and_applied_on_validate() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path().join("legacy-skill");
    fs::create_dir_all(&dir).unwrap();
    let json = r#"{"name":"legacy","description":"Legacy skill","version":"1.2.3","entrypoint":"SKILL.md"}"#;
    fs::write(dir.join("skill.json"), json).unwrap();

    let issues = detect_fixable_issues(&dir, "legacy-skill");
    assert!(issues.contains(&FixableIssue::NeedsMigration));

    apply_fixes(&dir, "legacy-skill", &issues).unwrap();

    assert!(dir.join("SKILL.md").exists());
    let content = fs::read_to_string(dir.join("SKILL.md")).unwrap();
    let frontmatter = strand::models::skill::parse_skill_md(&content).unwrap();

    assert_eq!(frontmatter.name, "legacy");
    assert_eq!(frontmatter.description, "Legacy skill");
    assert_eq!(frontmatter.metadata.version, "1.2.3");

    // Validate should pass after migration
    let result = validate_skill(&dir, "legacy-skill");
    assert!(result.valid, "Expected validate to pass after migration, got errors: {:?}", result.errors);
}

// ── Acceptance: summary counts are accurate ──────────────────────────

#[test]
fn test_summary_counts_accurate_on_validate() {
    colored::control::set_override(false);

    let reports = vec![
        SkillReport {
            name: "skill1".to_string(),
            valid: true,
            issues: vec![],
        },
        SkillReport {
            name: "skill2".to_string(),
            valid: false,
            issues: vec!["error1".to_string()],
        },
        SkillReport {
            name: "skill3".to_string(),
            valid: false,
            issues: vec!["error2".to_string(), "error3".to_string()],
        },
    ];

    let output = format_report(&reports, 1, 2);
    assert!(
        output.contains("Summary: 1 valid, 2 invalid"),
        "Expected summary counts in output:\n{}",
        output
    );
}

// ── CLI integration tests ────────────────────────────────────────────

#[test]
fn test_cli_validate_finds_all_skills() {
    let temp = TempDir::new().unwrap();
    setup_skills_dir(
        &temp,
        &[
            "valid_skill",
            "valid_with_assets",
            "invalid_bad_version",
            "invalid_missing_entrypoint",
            "invalid_empty_name",
            "invalid_no_json",
            "invalid_bad_json",
        ],
    );

    let output = Command::new(env!("CARGO_BIN_EXE_strand"))
        .args(["validate"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute validate command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("valid_skill"), "stdout: {}", stdout);
    assert!(stdout.contains("valid_with_assets"), "stdout: {}", stdout);
    assert!(stdout.contains("invalid_bad_version"), "stdout: {}", stdout);
    assert!(stdout.contains("invalid_missing_entrypoint"), "stdout: {}", stdout);
    assert!(stdout.contains("invalid_empty_name"), "stdout: {}", stdout);
    assert!(stdout.contains("invalid_no_json"), "stdout: {}", stdout);
    assert!(stdout.contains("invalid_bad_json"), "stdout: {}", stdout);
}

#[test]
fn test_cli_validate_exit_code_failure_when_invalid() {
    let temp = TempDir::new().unwrap();
    setup_skills_dir(&temp,
        &["valid_skill", "invalid_bad_version", "invalid_no_json"],
    );

    let output = Command::new(env!("CARGO_BIN_EXE_strand"))
        .args(["validate"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute validate command");

    assert!(!output.status.success(), "Expected non-zero exit code when invalid skills exist");
}

#[test]
fn test_cli_validate_exit_code_success_when_all_valid() {
    let temp = TempDir::new().unwrap();
    setup_skills_dir(&temp, &["valid_skill", "valid_with_assets"],
    );

    let output = Command::new(env!("CARGO_BIN_EXE_strand"))
        .args(["validate"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute validate command");

    assert!(output.status.success(), "Expected zero exit code when all skills are valid");
}

#[test]
fn test_cli_validate_shows_summary_counts() {
    let temp = TempDir::new().unwrap();
    setup_skills_dir(&temp, &["valid_skill", "invalid_bad_version"],
    );

    let output = Command::new(env!("CARGO_BIN_EXE_strand"))
        .args(["validate"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute validate command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Summary: 1 valid, 1 invalid"),
        "Expected summary in stdout: {}",
        stdout
    );
}

#[test]
fn test_cli_validate_reports_fixable_issues() {
    let temp = TempDir::new().unwrap();
    setup_skills_dir(&temp, &["invalid_empty_name"],
    );

    let output = Command::new(env!("CARGO_BIN_EXE_strand"))
        .args(["validate"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute validate command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // When non-interactive, it should mention auto-fixable issues
    assert!(
        stdout.contains("auto-fixable") || stdout.contains("Found"),
        "Expected fixable issue mention in stdout: {}",
        stdout
    );
}
