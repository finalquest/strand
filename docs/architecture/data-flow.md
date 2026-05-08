# Data Flows

> How data moves through strand from user input to side effects.

## Authentication Flow

Every command that talks to GitLab follows this path:

```
User runs command
  → config::read() → extract hostname from config
  → auth::authenticate(hostname)
      → try glab CLI (GlabAuth)
      → try env var strand_GITLAB_TOKEN (PatAuth)
      → prompt user (InteractiveAuth)
  → AuthBackend resolved
  → GitLabClient::for_project(base_url, project)
      → selects GlabTransport or ReqwestTransport based on AuthBackend
  → GitLabClient ready with auth + transport
```

If the user is running `glab` and already authenticated, no token prompt appears. If `strand_GITLAB_TOKEN` is set, it's used directly. Otherwise the user is prompted once and the token is held in memory for the session.

## Config Resolution Flow

```
.env / shell env vars
  → env.rs reads strand_* variables
  → config.rs reads .strand/config.json
  → env vars override config values
  → commands get resolved (project, base_url, branch)
```

Priority order (highest wins):
1. `strand_AGENTS_REPO` / `strand_SKILLS_REPO` env vars
2. `strand_AGENTS_REPO_BRANCH` / `strand_SKILLS_REPO_BRANCH` env vars
3. `strand_GITLAB_URL` env var (overrides base URL for both repos)
4. `.strand/config.json` fields (`agentsRepo`, `skillsRepo`)
5. Hardcoded defaults (`main` branch, `https://gitlab.com`)

## Skill Install Flow

```
strand ls-remote
  → config::read() → get skills_repo config
  → env::skills_repo_*() → apply env overrides
  → GitLabClient::for_project(base_url, project)
  → gitlab.list_tree("skills") → list remote directories
  → interactive::select_skill() → user picks one
  → download::download_and_install(client, skill)
      → gitlab.list_tree("skills/{name}") → list files
      → gitlab.fetch_file("skills/{name}/{file}") → per file
      → write to .agents/skills/{name}/
  → config::add_skill(skill) → persist to config.json
  → gitignore::ensure_gitignore_entries(name) → update .gitignore
  → codex::create_symlink(name) → link to .codex/skills/ (if enabled)
```

## Skill Update Flow

```
strand sync
  → config::read() → get installed skills list
  → for each installed skill:
      → GitLabClient → fetch remote SKILL.md frontmatter
      → version::compare_versions(local, remote)
      → if Behind: prompt user to upgrade
          → download::download_and_install() (overwrite)
          → config::add_skill() (update version)
          → gitignore + codex symlink (idempotent)
```

## Agent Install Flow

```
strand agents ls-remote
  → config::read() → get agents_repo config
  → env::agents_repo_*() → apply env overrides
  → config.resolve_agents_repo() → final (project, branch, base_url)
  → GitLabClient::for_project(base_url, project)
  → gitlab.list_tree("agents") → list remote directories
  → interactive::select_skill() → user picks one
  → commands::agents::helpers::download_and_install_agent()
      → gitlab.list_tree("agents/{name}") → list files
      → gitlab.fetch_file("agents/{name}/{file}") → per file
      → write to .agents/agents/{name}/
  → config::add_agent(agent) → persist to config.json
  → gitignore::ensure_gitignore_entries(name, Agent) → update .gitignore
  → symlinks::create_symlink() for .opencode/agents/ and .codex/agents/
```

## Validation Flow

```
strand validate (or strand agents validate)
  → scan .agents/skills/ (or .agents/agents/) directory
  → for each skill/agent directory:
      → fix::detect_fixable_issues(path, name)
          → check SKILL.md exists (or AGENT.md)
          → parse YAML frontmatter
          → check required fields (name, description, version)
      → collect issues into SkillReport
  → report::print_report(reports) → colored table output
  → if fixable issues found, prompt to apply fixes
  → fix::apply_fixes() → writes corrected SKILL.md / migrates skill.json
```

## Init Flow

```
strand init
  → create directory structure:
      .agents/skills/
      .agents/agents/
      .codex/skills/
      .codex/agents/
      .opencode/agents/
  → prompt user for target tools (opencode, codex)
  → prompt user for GitLab project (skills repo, agents repo)
  → config::create() → write .strand/config.json
  → symlinks::create_symlink() for each enabled target directory
```

## Cross-Cutting Concerns

### Error Propagation

```
Subsystem errors (thiserror enums)
  → GitLabError, AuthError, ResolverError, etc.
  → Commands convert to anyhow::Result at the boundary
  → main.rs prints user-friendly error via anyhow display
```

### Idempotent Operations

All side-effect operations (symlinks, gitignore entries, downloads) are idempotent:
- `symlinks::create_symlink()` removes existing link before creating new one
- `gitignore::ensure_gitignore_entries()` checks for existing entries before appending
- `download::download_and_install()` overwrites existing files
- `config::add_skill()` updates version if skill already exists
