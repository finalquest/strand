# `version`

**Purpose**: Semver comparison logic.
**Files**: `src/version.rs`

## Public API

```rust
pub enum VersionComparison { Ahead, Behind, Equal }
pub fn compare_versions(local: &str, remote: &str) -> VersionComparison
```

## Used By
- `commands::sync`

## Dependencies
- `semver` (external crate)

## Notes
- Returns `Equal` if either version is invalid semver.
