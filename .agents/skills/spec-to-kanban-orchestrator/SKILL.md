---
name: spec-to-kanban-orchestrator
description: "Use when the user wants to convert a spec document into kanban board tasks using the analyzer → merger → writer subagent pipeline. Examples: \"Create tasks from the spec\", \"Run spec-to-kanban flow\", \"Derive tasks from spec\" - when the spec file path and board configuration are provided"
---

# Spec To Kanban Orchestrator

This orchestrator skill coordinates the conversion of a spec document into kanban board tasks using three subagents: **analyzer**, **merger**, and **writer**.

## Prerequisites

- A spec file exists (path passed by orchestrator at runtime)
- Board API credentials are available in env (`BOARD_API_KEY`)
- The following tool files are present:
  - `.opencode/agents/spec-task-analyzer.md`
  - `.opencode/agents/spec-task-merger.md`
  - `.opencode/agents/task-writer.md`
  - `.opencode/scripts/board-task-writer.cjs` (the writer script)

## Orchestrator Flow

```
Inspect repo context → Read spec → Split into deliverable shards → Launch analyzer subagents → Batch NDJSON → Merge batches → Launch writer subagent → Record trace
```

### Step 1: Read and Split Spec

1. Read the board configuration (base_url, board_id, lists mapping, labels mapping). This is provided by the orchestrator at runtime - not hardcoded.
2. Run `node .opencode/scripts/inspect-repo-context.mjs` and keep the compact JSON as `repo_context`. `.opencode/` is repo-local tooling and must not count as product code.
3. Read the spec file.
4. Read the task template and workflow rules (the orchestrator must supply these).
5. Split the spec into bounded deliverable shards. A shard should represent one runnable behavior, acceptance path, MVP boundary, or deferred future area. Do not shard mechanically by headings or architecture taxonomy when that separates one deliverable across multiple analyzers.

Sharding rule:
- Good shard: a coherent deliverable or usage flow, such as first runnable CLI slice, evaluator MVP vertical slice, ambiguity handling flow, or future evolution deferred scope.
- Bad shard: an isolated architecture section such as only data model, only task types, only storage, or only validation, unless that section is itself the deliverable.
- If a support section only makes sense with an executable flow, include it in that flow shard or tell the analyzer to treat it as supporting context.
- Each shard must remain small enough to produce compact analyzer output, but preserving deliverable context is more important than equal shard sizes.

### Step 2: Launch Analyzer Subagents

For each deliverable shard:

1. Launch the `spec-task-analyzer` subagent with bounded inputs. The analyzer contract is embedded in `.opencode/agents/spec-task-analyzer.md`; do not read a standalone contract doc.
2. The analyzer may read `docs/` for workflow guidance, but must not read or explore `specs/` directly.
   ```json
   {
     "source_section": "string",
     "spec_excerpt": "string",
     "repo_context": {},
     "task_template": "string",
     "workflow_rules": "string",
     "granularity_rules": ["string"],
     "output_contract": "analyzer"
   }
   ```
3. Validate each analyzer output file:
   - File path matches `.opencode/data/analyzer-output-shard<N>.ndjson`
   - Each non-empty line is one valid JSON object
   - Each task has valid `local_id`, `title`, `type`, `priority`, `acceptance_criteria`, `implementation_boundary`, `implementation_steps`, `verification`, and `out_of_scope`
   - IDs are unique within the shard
   - Analyzer subagent responses must be compact status only; do not ask analyzers to return full NDJSON content to the main agent.
   - If a shard is contextual or deferred, a valid analyzer output may contain zero implementation tasks plus compact warning lines.
   - Warning lines about valid post-MVP work should be collected into a deferred-candidates artifact or summarized to the user; do not silently drop them as if the work does not exist.
4. Run `node .opencode/scripts/ndjson-to-batches.mjs` to create `.opencode/data/analyzer-output-batch-*.json` files.

### Step 2.5: Resolve Next Board Task ID

Before launching the merger, run:

```bash
node .opencode/scripts/find-next-task-id.cjs
```

Pass the returned `next_id` to the merger subagent so it can offset final `T-xxx` IDs to avoid collisions with existing board cards. The merger MUST NOT assign IDs that already exist on the board.

### Step 3: Launch Merger Subagent

1. Launch the `spec-task-merger` subagent. The merger reads all `.opencode/data/analyzer-output-batch-*.json` files itself via `glob` + `read`.
2. Provide task template, workflow rules, and board conventions in the prompt. The merger output contract is embedded in `.opencode/agents/spec-task-merger.md`:
   ```json
   {
      "task_template": "string",
      "workflow_rules": "string",
      "board_conventions": "string",
      "output_contract": "merger"
   }
   ```
3. Validate `.opencode/data/merged-task-plan.json`:
   - Output is valid JSON and passes `node .opencode/scripts/validate-merged.mjs`
   - `tasks_to_create` is an array
   - Final `local_id` values use `T-001` format and are unique
   - Dependencies reference existing final IDs
   - Dependency edges reference existing final IDs
   - Tasks are actionable implementation slices with concrete paths/modules/entrypoints and verification commands

### Step 4: Launch Writer Subagent

1. Pass `.opencode/data/merged-task-plan.json` to the writer script via the `task-writer` subagent. The writer contract is embedded in `.opencode/agents/task-writer.md`.
2. Launch the `task-writer` subagent with the writer input JSON produced by the merger:
   ```json
   {
     "tasks_to_create": [],
     "cards_to_move": [],
     "dependency_edges": [],
      "output_contract": "writer"
   }
   ```

### Step 5: Record Trace

Append the run results to the run trace file (`docs/spec_to_kanban_runs.md` or equivalent).

## Output to User

Return a compact summary:

```md
## Result

- Created: N tasks
- Dependency edges: N
- Failed: N (list with reasons)
- Board: <url>
- Warnings: <warnings>
```

Do NOT output full task bodies, full API responses, or verbose logs.

## Error Handling

- If an analyzer shard fails, retry only that shard. Do not discard other analyzer outputs.
- If the merger fails, do not create any cards. Ask the merger to report ambiguous input.
- If the writer fails, retry only the failed cards (retryable=true) without duplicating already-created cards.
- The orchestrator must not write to the board directly.

## Key Reminders

- Analyzer and merger subagents must NOT create board cards.
- Only the writer subagent may interact with the board API.
- Credentials must never be passed through JSON or printed in output; `board-task-writer.cjs` reads `BOARD_API_KEY` from the runtime environment.
- Keep all outputs compact. The main agent retains decisions and summaries, not raw tool outputs.
- All board configuration comes from the orchestrator's input at runtime, never hardcoded.
