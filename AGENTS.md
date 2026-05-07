# Project Instructions

- At the start of every session in this repository, read `handoff.md` before doing any other project work.
- **Before analyzing or modifying code, always check the documentation first.** Read `docs/architecture/index.md` and relevant module docs to understand the current state. Do not explore the codebase by reading source files blindly when documentation exists.
- Treat `handoff.md` as the current project handoff: use it to understand the objective, relevant files, decisions, current state, next steps, and blockers.
- Do not update `handoff.md` during normal work. It is updated only when the explicit handoff command is executed.
- Keep OpenCode configuration and subagents for this project in this repository instead of the global OpenCode config.
- When working on OpenCode configuration, commands, plugins, or subagents for this project, only inspect and modify files under this repository, especially `.opencode/`. Do not read, search, copy from, or depend on global OpenCode paths such as `~/.config/opencode`, `~/.opencode`, or user-level agent/skill directories unless the user explicitly approves that specific path lookup.

## Key Distinction: Tools vs Product

- The subagents and scripts under `.opencode/` (analyzer, merger, writer, planka-task-writer.mjs) are **NOT Vanguard. They are NOT the spec. They are NOT the CLI.**
- They are simply automation tools to move the tedium of creating Planka cards from the main agent context into isolated subagents.
- The tools solve one problem: extracting noisy Planka API calls and large JSON outputs from the main agent context.

## Project Skills

- **spec-to-kanban-orchestrator**: Convert a spec document into kanban board tasks using the analyzer → merger → writer subagent pipeline. Use when deriving development tasks from a spec.
- **board-task-orchestrator**: Claim one Ready Planka card, pass it to the task implementer agent, and move it to Review or Blocked based on the result. Use when assigning board work to an implementation agent.

### Mandatory Skill Usage for Planka

**All interactions with Planka cards MUST go through the appropriate skill.** Do not call Planka scripts directly (e.g., `node .opencode/scripts/claim-next-board-card.cjs`) unless the skill itself instructs you to do so as part of its workflow. The skills encapsulate the correct sequence, validation, and error handling. Using scripts directly bypasses orchestrator rules and can leave cards in inconsistent states.

## Planka Board

- Base URL: `https://tasks.finalq.xyz`
- Board ID: `1768334339835168102`
- See `docs/planka_usage.md` for full details.

## Spec Scope Boundary

- Product specs live under `specs/`.
- Agent input/output contracts live inside each `.opencode/agents/*.md` file, not as standalone repo docs.
- The main orchestrator may read `specs/` to split bounded spec inputs.
- Subagents may read `docs/` for workflow guidance when needed, but must not read or explore `specs/` directly unless the orchestrator explicitly delegates a specific path and section.
- Prefer passing bounded spec excerpts to subagents instead of letting them discover spec files.
