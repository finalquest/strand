# `cli`

**Purpose**: CLI argument parsing with clap derive macros.
**Files**: `src/cli.rs`

## Public API

```rust
#[derive(Parser)]
pub struct Cli {
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Init,
    List,
    Install,
    Sync,
    Install { dry_run: bool },
    Validate,
}
```

## Used By
- `main.rs` (parses args and dispatches)

## Dependencies
- None (pure clap)

## Notes
- Add new variants here, then create the command module in `src/commands/`, then wire in `main.rs`.
