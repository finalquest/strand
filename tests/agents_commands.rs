use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use tempfile::TempDir;

fn strand_bin() -> &'static str {
    env!("CARGO_BIN_EXE_strand")
}

fn create_agent_dir(base: &Path, name: &str, agent_md: &str) {
    let dir = base.join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("AGENT.md"), agent_md).unwrap();
}

// ── agents ls ────────────────────────────────────────────────────────

#[test]
fn test_agents_ls_no_config() {
    let temp = TempDir::new().unwrap();
    let output = Command::new(strand_bin())
        .current_dir(temp.path())
        .args(["agents", "ls"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute strand agents ls");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No configuration found"),
        "Expected 'No configuration found' message. stdout:\n{}",
        stdout
    );
}

#[test]
fn test_agents_ls_empty_config() {
    let temp = TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join(".strand")).unwrap();
    let config = r#"{"version":1,"targets":{"opencode":true,"codex":false},"skillsRepo":{"provider":"gitlab","project":"","branch":"main","base_url":"https://gitlab.example.com"},"skills":[],"agents":[]}"#;
    fs::write(temp.path().join(".strand/config.json"), config).unwrap();

    let output = Command::new(strand_bin())
        .current_dir(temp.path())
        .args(["agents", "ls"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute strand agents ls");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No agents installed"),
        "Expected 'No agents installed' message. stdout:\n{}",
        stdout
    );
}

#[test]
fn test_agents_ls_no_agents_repo() {
    let temp = TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join(".strand")).unwrap();
    let config = r#"{"version":1,"targets":{"opencode":true,"codex":false},"skillsRepo":{"provider":"gitlab","project":"","branch":"main","base_url":"https://gitlab.example.com"},"skills":[],"agents":[{"name":"test-agent","version":"1.0.0","installedPath":".agents/agents/test-agent"}]}"#;
    fs::write(temp.path().join(".strand/config.json"), config).unwrap();

    let output = Command::new(strand_bin())
        .current_dir(temp.path())
        .args(["agents", "ls"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute strand agents ls");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No agents repository configured"),
        "Expected 'No agents repository configured' message. stdout:\n{}",
        stdout
    );
}

// ── agents ls-remote ─────────────────────────────────────────────────

#[test]
fn test_agents_ls_remote_no_config() {
    let temp = TempDir::new().unwrap();
    let output = Command::new(strand_bin())
        .current_dir(temp.path())
        .args(["agents", "ls-remote"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute strand agents ls-remote");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No agents repository configured"),
        "Expected 'No agents repository configured' message. stdout:\n{}",
        stdout
    );
}

#[test]
fn test_agents_ls_remote_empty_repo() {
    let temp = TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join(".strand")).unwrap();
    let config = r#"{"version":1,"targets":{"opencode":true,"codex":false},"skillsRepo":{"provider":"gitlab","project":"","branch":"main","base_url":"https://gitlab.example.com"},"skills":[],"agentsRepo":{"provider":"gitlab","project":"","branch":"main","base_url":"https://gitlab.example.com"},"agents":[]}"#;
    fs::write(temp.path().join(".strand/config.json"), config).unwrap();

    let output = Command::new(strand_bin())
        .current_dir(temp.path())
        .args(["agents", "ls-remote"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute strand agents ls-remote");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No agents repository configured"),
        "Expected 'No agents repository configured' message. stdout:\n{}",
        stdout
    );
}

// ── agents validate ──────────────────────────────────────────────────

#[test]
fn test_agents_validate_no_agents_dir() {
    let temp = TempDir::new().unwrap();
    let output = Command::new(strand_bin())
        .current_dir(temp.path())
        .args(["agents", "validate"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute strand agents validate");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(".agents/agents/ directory not found"),
        "Expected '.agents/agents/ directory not found' error. stderr:\n{}",
        stderr
    );
}

#[test]
fn test_agents_validate_valid_agent() {
    let temp = TempDir::new().unwrap();
    let agents_dir = temp.path().join(".agents/agents");
    fs::create_dir_all(&agents_dir).unwrap();
    create_agent_dir(
        &agents_dir,
        "test-agent",
        "---\nname: test-agent\ndescription: A test agent\nmetadata:\n  version: 1.0.0\n---\n",
    );

    let output = Command::new(strand_bin())
        .current_dir(temp.path())
        .args(["agents", "validate"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute strand agents validate");

    assert!(output.status.success(), "Expected success exit code. stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("test-agent"),
        "Expected 'test-agent' in output. stdout:\n{}",
        stdout
    );
    assert!(
        stdout.contains("valid"),
        "Expected 'valid' status in output. stdout:\n{}",
        stdout
    );
}

#[test]
fn test_agents_validate_invalid_agent() {
    let temp = TempDir::new().unwrap();
    let agents_dir = temp.path().join(".agents/agents");
    fs::create_dir_all(&agents_dir).unwrap();
    create_agent_dir(
        &agents_dir,
        "bad-agent",
        "---\nname: ''\ndescription: ''\nmetadata:\n  version: not-a-version\n---\n",
    );

    let output = Command::new(strand_bin())
        .current_dir(temp.path())
        .args(["agents", "validate"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute strand agents validate");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("bad-agent"),
        "Expected 'bad-agent' in output. stdout:\n{}",
        stdout
    );
    assert!(
        stdout.contains("invalid"),
        "Expected 'invalid' status in output. stdout:\n{}",
        stdout
    );
}

#[test]
fn test_agents_validate_missing_agent_md() {
    let temp = TempDir::new().unwrap();
    let agents_dir = temp.path().join(".agents/agents");
    fs::create_dir_all(&agents_dir).unwrap();
    fs::create_dir_all(agents_dir.join("no-md-agent")).unwrap();

    let output = Command::new(strand_bin())
        .current_dir(temp.path())
        .args(["agents", "validate"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute strand agents validate");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("no-md-agent"),
        "Expected 'no-md-agent' in output. stdout:\n{}",
        stdout
    );
    assert!(
        stdout.contains("AGENT.md not found"),
        "Expected 'AGENT.md not found' issue in output. stdout:\n{}",
        stdout
    );
}

#[test]
fn test_agents_validate_summary_counts() {
    let temp = TempDir::new().unwrap();
    let agents_dir = temp.path().join(".agents/agents");
    fs::create_dir_all(&agents_dir).unwrap();
    create_agent_dir(
        &agents_dir,
        "valid-agent",
        "---\nname: valid-agent\ndescription: A valid agent\nmetadata:\n  version: 1.0.0\n---\n",
    );
    create_agent_dir(
        &agents_dir,
        "invalid-agent",
        "---\nname: ''\ndescription: ''\nmetadata:\n  version: bad\n---\n",
    );

    let output = Command::new(strand_bin())
        .current_dir(temp.path())
        .args(["agents", "validate"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute strand agents validate");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Summary: 1 valid, 1 invalid"),
        "Expected summary counts in output. stdout:\n{}",
        stdout
    );
}
