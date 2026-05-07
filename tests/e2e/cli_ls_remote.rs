use crate::strand_cmd_in;
use std::process::Stdio;
use tempfile::TempDir;

#[test]
fn test_cli_list_shows_table_when_skills_present() {
    let temp = TempDir::new().unwrap();
    let output = strand_cmd_in(&temp)
        .arg("ls-remote")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute strand ls-remote");

    assert!(
        output.status.success(),
        "strand ls-remote should exit successfully. stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Available Skills"),
        "Expected 'Available Skills' header in output. stdout:\n{}",
        stdout
    );
    assert!(
        stdout.contains("Skill"),
        "Expected 'Skill' column in table. stdout:\n{}",
        stdout
    );
    assert!(
        stdout.contains("Version"),
        "Expected 'Version' column in table. stdout:\n{}",
        stdout
    );
    // Description column should NOT be present after T-012
    assert!(
        !stdout.contains("Description"),
        "Description column should not appear in table after T-012. stdout:\n{}",
        stdout
    );
}

#[test]
fn test_cli_list_with_nonexistent_repo_shows_no_skills() {
    let temp = TempDir::new().unwrap();
    let output = Command::new(crate::strand_bin())
        .current_dir(temp.path())
        .env("strand_GITLAB_URL", "https://gitlab.example.com")
        .env("strand_SKILLS_REPO", "nonexistent-group/nonexistent-project-12345")
        .env("strand_SKILLS_REPO_BRANCH", "main")
        .arg("ls-remote")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute strand ls-remote");

    // When the project doesn't exist, list_tree returns 404 which the CLI
    // treats as "no skills found" (exit 0) rather than an error.
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No skills found in the repository."),
        "Expected 'No skills found' message. stdout:\n{}",
        stdout
    );
}

use std::process::Command;
