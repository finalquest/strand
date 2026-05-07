# `report`

**Purpose**: Colored validation report formatting.
**Files**: `src/report.rs`

## Public API

```rust
pub struct SkillReport { ... }
pub fn print_report(reports: &[SkillReport]) -> Result<()>
```

## Used By
- `commands::validate`

## Dependencies
- `colored` (external crate)

## Notes
- Renders a table with Skill, Status, and Issues columns.
- Handles ANSI color codes and dynamic column widths for alignment.
