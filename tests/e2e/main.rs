use std::process::Command;
use tempfile::TempDir;

/// Default GitLab base URL for the e2e test target.
const DEFAULT_BASE_URL: &str = "https://gitlab.example.com";
/// Default skills repository project path.
const DEFAULT_PROJECT: &str = "example-group/sandbox/dev-skills";
/// Default branch/tag to use.
const DEFAULT_BRANCH: &str = "strand_test";

/// Returns the path to the `strand` binary under test.
fn strand_bin() -> &'static str {
    env!("CARGO_BIN_EXE_strand")
}

/// Returns a base `Command` for `strand` with e2e environment variables set.
fn strand_cmd_in(dir: &TempDir) -> Command {
    let mut cmd = Command::new(strand_bin());
    cmd.current_dir(dir.path());
    cmd.env("strand_GITLAB_URL", DEFAULT_BASE_URL);
    cmd.env("strand_SKILLS_REPO", DEFAULT_PROJECT);
    cmd.env("strand_SKILLS_REPO_BRANCH", DEFAULT_BRANCH);
    cmd
}

mod client;
mod cli_init;
mod cli_ls_remote;
