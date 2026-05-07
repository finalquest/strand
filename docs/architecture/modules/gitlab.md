# `gitlab`

**Purpose**: GitLab API client and transport abstraction.
**Files**: `src/gitlab/mod.rs`, `src/gitlab/client.rs`, `src/gitlab/errors.rs`, `src/gitlab/transport.rs`

## Public API (`src/gitlab/mod.rs` re-exports)

```rust
pub struct GitLabClient { ... }
impl GitLabClient {
    pub fn for_project(base_url: String, project_id: String) -> Result<Self, AuthError>
    pub fn with_transport(transport: Box<dyn Transport>, project_id: String) -> Self
    pub fn with_branch(self, branch: &str) -> Self
    pub fn list_tree(&self, path: &str) -> Result<Vec<TreeEntry>, GitLabError>
    pub fn fetch_file(&self, path: &str) -> Result<String, GitLabError>
}

pub struct TreeEntry { pub name: String, pub entry_type: String, pub path: String }

pub enum GitLabError { ... }

pub trait Transport {
    fn call(&self, endpoint: &str) -> Result<(u16, String), GitLabError>;
}

pub struct ReqwestTransport;
pub struct GlabTransport;
```

## Sub-modules

| Module | Purpose |
|--------|---------|
| `gitlab::client` | Builds API endpoints, delegates to Transport, parses responses |
| `gitlab::errors` | `GitLabError` enum (Auth, NotFound, Http, Parse) |
| `gitlab::transport` | `Transport` trait + `ReqwestTransport` (HTTP) + `GlabTransport` (CLI) |

## Dependencies

```
gitlab::client
  → auth (AuthBackend, authenticate, TokenAuth)
  → gitlab::errors (GitLabError)
  → gitlab::transport (Transport, GlabTransport, ReqwestTransport)

gitlab::transport
  → auth::Auth
  → gitlab::errors::GitLabError
```

## Used By
- All commands (indirectly via `GitLabClient`)
- `download` (fetches skill files)
- `resolver::gitlab` (adapter pattern)

## Adding a New Transport

1. Implement `Transport` trait in `src/gitlab/transport.rs`
2. Wire in `GitLabClient::for_project` in `src/gitlab/client.rs`
3. Add tests with a mock transport (see existing tests for pattern)
