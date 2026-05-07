# Strand Auth Refactor: Native glab Integration

## Overview

Replace the current "token extraction + reqwest" authentication flow with direct `glab api` invocation. When `glab` is available and authenticated for the target host, Strand should delegate GitLab API calls to `glab` instead of extracting a PAT and making HTTP requests manually.

## Problem Statement

The current auth flow is fragile:
1. `glab auth status` returns exit code 1 when ANY configured GitLab instance is unauthenticated, even if others are valid.
2. The token parser looks for `"Token:"` but `glab` outputs `"Token found:"` (or masked asterisks without `--show-token`).
3. Extracting the PAT and passing it to `reqwest` is an unnecessary middleman; `glab` already handles auth, host resolution, and HTTP.

## Desired Behavior

### When glab is available

Strand should run GitLab API calls via `glab`:

```bash
glab api --hostname <host> <endpoint>
```

Examples:
- List tree: `glab api --hostname gitlab.example.com projects/<id>/repository/tree?path=skills`
- Fetch file: `glab api --hostname gitlab.example.com projects/<id>/repository/files/skills%2Ftest%2Fskill.json/raw?ref=main`

Strand must determine `<host>` from:
1. `strand_GITLAB_URL` environment variable
2. The configured `base_url` in the current context (default: `https://gitlab.com`)

The hostname passed to `--hostname` must be the host component of the URL (e.g., `gitlab.example.com`).

### When glab is NOT available

Fall back to the existing PAT flow:
1. `strand_GITLAB_TOKEN` environment variable
2. Interactive prompt

### Auth Flow Changes

- Remove token extraction from `glab auth status` output.
- `GlabAuth` should indicate that API calls will be delegated to `glab`, not provide a raw token.
- The GitLab client must be able to execute via `glab` or via `reqwest` depending on the auth method chosen.

## Architecture Changes

### `src/auth/glab.rs`
- Remove `parse_token` and `run_auth_status`.
- `GlabAuth` should verify `glab` is installed and has at least one valid authentication.
- Expose a method to check if `glab` can handle a specific hostname.

### `src/auth/mod.rs`
- The `Auth` trait may need to change. Instead of `get_token()`, it could provide an `execute_api_call(host, endpoint)` abstraction, or the client can branch on auth type.
- Keep it simple: `authenticate()` returns an enum or struct that tells the client which backend to use.

### `src/gitlab/client.rs`
- Introduce a transport abstraction.
- `GitLabClient` should choose between:
  - `GlabTransport`: runs `glab api --hostname <host> <endpoint>` and returns stdout.
  - `ReqwestTransport`: uses `reqwest` with a PAT token.
- Both transports implement the same interface for `list_tree` and `fetch_file`.

### `src/commands/list.rs` (and other commands)
- Resolve `base_url` / `hostname` BEFORE creating the client.
- Pass the resolved hostname to the auth/client layer.

## Acceptance Criteria

1. `strand list` with `glab` installed and `gitlab.example.com` authenticated does NOT prompt for PAT.
2. `strand list` uses `glab api --hostname gitlab.example.com` under the hood.
3. If `glab` is not installed, `strand list` falls back to `strand_GITLAB_TOKEN` or interactive prompt + `reqwest`.
4. If `glab` is installed but not authenticated for the target host, fall back to PAT flow.
5. Exit codes and error messages remain clear and actionable.

## Out of Scope

- Caching `glab` responses.
- Supporting multiple `glab` hosts simultaneously in a single command (use the resolved host).
- Changing the config file format.

## Verification

1. Run `strand list` in a repo with `glab` authenticated for the skills repo host.
2. Observe no PAT prompt.
3. Verify via debug logging or process inspection that `glab api` is invoked.
4. Uninstall `glab` or remove its auth, set `strand_GITLAB_TOKEN`, and verify `reqwest` path still works.
