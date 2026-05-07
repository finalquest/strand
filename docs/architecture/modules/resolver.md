# `resolver`

**Purpose**: Abstracts over local and remote skill sources. Secondary abstraction not used by most commands.
**Files**: `src/resolver/mod.rs`, `src/resolver/errors.rs`, `src/resolver/gitlab.rs`, `src/resolver/local.rs`

## Public API

```rust
pub trait SkillSource {
    fn read_file(&self, path: &str) -> Result<String, ResolverError>;
}

pub struct Resolver { ... }
impl Resolver {
    pub fn new(default_project: String, default_base_url: String) -> Self
    pub fn resolve(&self) -> Result<Box<dyn SkillSource>, ResolverError>
}

pub enum ResolverError { ... }
```

## Used By
- No commands currently use the resolver directly. Commands call `GitLabClient` directly.

## Notes
- This is an adapter pattern that could replace direct `GitLabClient` usage in commands.
- `LocalSkillSource` reads from local filesystem; `GitLabSkillSource` wraps `GitLabClient`.
