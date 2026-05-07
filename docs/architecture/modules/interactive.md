# `interactive`

**Purpose**: Interactive UI helpers.
**Files**: `src/interactive/mod.rs`

## Public API

```rust
pub fn select_skill(skills: &[Skill]) -> Option<usize>
```

## Used By
- `commands::ls_remote`

## Dependencies
- `models::skill::Skill`
- `dialoguer` (external crate)

## Notes
- Returns `None` if user cancels or no selection is made.
