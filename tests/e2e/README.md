# E2E / Integration Tests

These tests exercise `strand` against the **real** GitLab skills repository.

## Target Repository

- **Project:** `example-group/sandbox/dev-skills`
- **Branch:** `strand_test`
- **GitLab:** `https://gitlab.example.com`

## Authentication

The tests rely on the same auth chain as the CLI:

1. `glab` CLI authenticated for `gitlab.example.com` (preferred)
2. `strand_GITLAB_TOKEN` environment variable (falls back)
3. Interactive prompt (will hang in automated runs — avoid)

## Running

```bash
# Run only the e2e tests
cargo test --test e2e

# Run with a specific token (skips glab)
strand_GITLAB_TOKEN=glpat-xxx cargo test --test e2e
```

## What Is Covered

- `GitLabClient::list_tree` / `fetch_file` against the real API
- `strand list` CLI end-to-end
- Error handling for missing repos / paths

## What Is NOT Covered (yet)

- `strand ls-remote` install flow — requires interactive skill selection
- `strand sync` — requires interactive upgrade prompt
- `strand install` — can be added once a non-interactive flag exists

## Adding New Tests

Add modules under `tests/e2e/` and declare them in `tests/e2e/main.rs`.
Use `strand_cmd_in(&temp_dir)` to spawn the CLI with the correct environment.
