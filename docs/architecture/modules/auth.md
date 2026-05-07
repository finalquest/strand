# `auth`

**Purpose**: Authentication backends and fallback chain.
**Files**: `src/auth/mod.rs`, `src/auth/glab.rs`, `src/auth/interactive.rs`, `src/auth/pat.rs`

## Public API (`src/auth/mod.rs`)

```rust
pub trait Auth {
    fn get_token(&self) -> Result<String, AuthError>;
}

pub enum AuthBackend {
    Glab { hostname: String },
    Token { token: String },
}

pub fn authenticate(hostname: &str) -> Result<AuthBackend, AuthError>

pub enum AuthError { ... }
```

## Sub-modules

| Module | Purpose | Used By |
|--------|---------|---------|
| `auth::glab` | Detects if `glab` CLI is installed and authenticated for a host | `auth::mod` |
| `auth::pat` | Reads PAT from `$strand_GITLAB_TOKEN` env var | `auth::mod`, `gitlab::transport` |
| `auth::interactive` | Prompts user for PAT via stdin | `auth::mod` |

## Dependency Graph

```
auth::mod
  → auth::glab
  → auth::pat
  → auth::interactive
  → auth::TokenAuth (internal, used by ReqwestTransport)
```

## Used By
- `gitlab::client` (via `authenticate()` to select transport)
- `gitlab::transport::ReqwestTransport` (via `TokenAuth`)

## Auth Fallback Chain

```
1. GlabAuth::is_installed() && is_authenticated_for(hostname)
   → AuthBackend::Glab { hostname }
2. PatAuth::get_token() → reads $strand_GITLAB_TOKEN
   → AuthBackend::Token { token }
3. InteractiveAuth::get_token() → prompts user
   → AuthBackend::Token { token }
```
