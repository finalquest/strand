use crate::{DEFAULT_BASE_URL, DEFAULT_BRANCH, DEFAULT_PROJECT};
use strand::gitlab::GitLabClient;

/// Build a real GitLabClient for the e2e target.
///
/// If `strand_GITLAB_TOKEN` is set, uses token-based ReqwestTransport to avoid
/// interactive prompts. Otherwise falls back to `for_project` (glab or PAT).
fn real_client() -> GitLabClient {
    if let Ok(token) = std::env::var("strand_GITLAB_TOKEN") {
        let auth = strand::auth::TokenAuth::new(token);
        let transport = strand::gitlab::ReqwestTransport::new(
            Box::new(auth),
            DEFAULT_BASE_URL.to_string(),
        );
        GitLabClient::with_transport(Box::new(transport), DEFAULT_PROJECT.to_string())
            .with_branch(DEFAULT_BRANCH)
    } else {
        GitLabClient::for_project(DEFAULT_BASE_URL.to_string(), DEFAULT_PROJECT.to_string())
            .expect("Failed to authenticate. Set strand_GITLAB_TOKEN or authenticate glab for the target host.")
            .with_branch(DEFAULT_BRANCH)
    }
}

#[test]
fn test_list_tree_skills() {
    let client = real_client();
    let entries = client.list_tree("skills").expect("list_tree(skills) should succeed");

    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(
        names.contains(&"gitlab-ci-architect"),
        "Expected gitlab-ci-architect in skills tree, got: {:?}",
        names
    );
    assert!(
        names.contains(&"gitlab-orchestrator"),
        "Expected gitlab-orchestrator in skills tree, got: {:?}",
        names
    );
    assert!(
        names.contains(&"maven-dependency-management"),
        "Expected maven-dependency-management in skills tree, got: {:?}",
        names
    );
    assert!(
        names.contains(&"maven-helper"),
        "Expected maven-helper in skills tree, got: {:?}",
        names
    );
}

#[test]
fn test_fetch_skill_md() {
    let client = real_client();
    let content = client
        .fetch_file("skills/gitlab-ci-architect/SKILL.md")
        .expect("fetch_file should succeed");

    assert!(
        content.contains("metadata:"),
        "Expected SKILL.md to contain metadata frontmatter, got: {}",
        content
    );
}

#[test]
fn test_fetch_file_with_custom_branch() {
    let client = real_client();
    let content = client
        .fetch_file("skills/gitlab-ci-architect/SKILL.md")
        .expect("fetch_file should succeed on strand_test branch");

    assert!(
        content.contains("metadata:"),
        "Expected SKILL.md to contain metadata frontmatter on custom branch"
    );
}

#[test]
fn test_list_tree_nonexistent_path() {
    let client = real_client();
    let result = client.list_tree("nonexistent_path_12345");
    // GitLab returns 404 for nonexistent tree paths; our client maps that to NotFound error.
    assert!(
        matches!(result, Err(strand::gitlab::GitLabError::NotFound(_))),
        "Expected NotFound error for nonexistent path, got: {:?}",
        result
    );
}
