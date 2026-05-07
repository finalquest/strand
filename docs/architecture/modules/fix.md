# `fix`

**Purpose**: Auto-fixable issue detection and repair for skill manifests.
**Files**: `src/fix.rs`

## Public API

```rust
pub enum FixableIssue {
    MissingSkillMd,
    NeedsMigration,
    MissingMetadata,
    MissingName,
    MissingDescription,
    MissingVersion,
    InvalidVersion(String),
}

pub fn detect_fixable_issues(skill_path: &Path, skill_name: &str) -> Vec<FixableIssue>
pub fn apply_fixes(skill_path: &Path, skill_name: &str, issues: &[FixableIssue]) -> Result<()>
```

## Used By
- `commands::validate`

## Notes
- `MissingSkillMd` creates a new `SKILL.md` with defaults.
- `NeedsMigration` migrates from old `skill.json` to new `SKILL.md` YAML frontmatter format.
- Default version is `0.0.1`.
- `SKILL.md` is the source of truth. During migration, the old `skill.json` is removed after creating `SKILL.md`.
