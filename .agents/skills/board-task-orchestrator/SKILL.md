---
name: board-task-orchestrator
description: "Use when assigning a Planka board card to an implementation or review agent. Claim one Ready card for implementation, or pick one Review card for review, and move it based on the result. Examples: 'Claim next board task', 'Assign T-001 to implementer', 'Review T-001', 'Move card to Review'"
---

# Skill: board-task-orchestrator

# Board Task Orchestrator

Use this workflow to assign one Planka board card to an implementation agent or a review agent.

This is repo-local development tooling. It is not Vanguard product behavior.

## Prerequisites

- `BOARD_API_KEY` is exported in the runtime environment.
- Board configuration exists at `.opencode/config/spec-to-kanban.json`.
- Planka cards use the generated task format with `Execution order`, `Paths`, `Implementation steps`, `Verification`, `Out of scope`, `Blocked by`, and `Unblocks` sections.

## Default Flow

1. Claim the next task:

   ```sh
   node .opencode/scripts/claim-next-board-card.cjs
   ```

   This selects the lowest `Execution order` card in `Ready`, moves it to `Doing`, and emits compact JSON containing `claimed` and `prompt_context`.

2. Launch the `task-implementer` subagent with `prompt_context` as the implementation prompt.

3. Inspect the implementer result.

4. Move the card based on the result:

   ```sh
   node .opencode/scripts/move-board-card.cjs --card <T-000-or-card-id> --list review
   ```

   Use `--list blocked` if the implementer reports a real blocker.

## Blocked Card Resolution Flow

When no Ready card is available but blocked cards exist:

1. Find blocked cards sorted by execution order:

   ```sh
   node .opencode/scripts/find-blocked-cards.cjs
   ```

2. Get the blocked card context:

   ```sh
   node .opencode/scripts/get-board-card.cjs --card <T-000-or-card-id>
   ```

3. Launch the `check-blockers-agent` subagent with the card `prompt_context`.

4. The agent will:
   - Parse `Blocked by` to identify blocker cards
   - Verify each blocker is in `Done` and its implementation exists
   - Verify the blocked card's needs are covered
   - Move the card to `Ready` if all blockers are resolved

5. If the card was moved to `Ready`, return to the Default Flow to claim it.

## Review Flow

1. Select a card in Review:

   ```sh
   node .opencode/scripts/get-board-card.cjs --card <T-000-or-card-id>
   ```

   This emits compact JSON containing the card for review.

2. Launch the `task-reviewer` subagent with the card `prompt_context` as the review prompt.

3. Inspect the reviewer result.

4. Move the card based on the review:

   ```sh
   node .opencode/scripts/move-board-card.cjs --card <T-000-or-card-id> --list done
   ```

   Use `--list doing` if the reviewer requests changes, so it can be re-implemented.

## Explicit Card Flow

To assign a specific Ready card:

```sh
node .opencode/scripts/claim-next-board-card.cjs --card T-001
```

The script refuses to claim an explicit card that is not in `Ready`.

To inspect without claiming:

```sh
node .opencode/scripts/get-board-card.cjs --card T-001
```

To preview the next claim without moving the card:

```sh
node .opencode/scripts/claim-next-board-card.cjs --dry-run
```

## Orchestrator Rules

- The orchestrator owns board I/O and list movement.
- The implementation agent must not move cards.
- The review agent must not move cards.
- Do not assign a `Blocked` card unless its blockers have been resolved and it has been moved to `Ready` first.
- Only one agent should own one `Doing` card at a time.
- If implementation succeeds and verification passes, move the card to `Review`.
- If implementation is blocked, move the card to `Blocked` and record the blocker in the final user-facing summary.
- If implementation fails due to code or test failures, leave the card in `Doing` unless the blocker is external.
- If review is approved, move the card to `Done`.
- If review requests changes, move the card back to `Doing` and record the required changes.
- If review finds a blocker, move the card to `Blocked` and record the blocker.
- Do not print credentials, raw API responses, or full card bodies.

## Implementation Prompt Shape

Pass this shape to `task-implementer`:

```text
You are implementing Planka card <CARD_ID_OR_URL>: <TITLE>.
Use the card description as the implementation contract.
Rules: read handoff.md first; implement only this card; do not do out-of-scope items; preserve unrelated changes; run the verification command from the card; if blocked, stop and report the blocker.
Card URL: <URL>

<CARD DESCRIPTION>
```

## Review Prompt Shape

Pass this shape to `task-reviewer`:

```text
You are reviewing Planka card <CARD_ID_OR_URL>: <TITLE>.
Use the card description as the review contract.
Rules: read handoff.md first; review only this card; verify acceptance criteria and verification command; check that out-of-scope items were not implemented; check code style and minimal correctness; do not fix issues yourself; report specific issues with file, line, and required change; if blocked, stop and report the blocker.
Card URL: <URL>

<CARD DESCRIPTION>
```

## Check-Blockers Prompt Shape

Pass this shape to `check-blockers-agent`:

```text
You are checking blockers for Planka card <CARD_ID_OR_URL>: <TITLE>.
This card is currently in Blocked state.
Rules: read handoff.md first; verify each blocker is in Done and its implementation exists; verify the blocked card's needs are covered by the blockers; if all blockers are resolved, move the card to Ready; if not, report what is missing.
Card URL: <URL>

<CARD DESCRIPTION>
```

## User Output

### Implementation

Return only a compact summary:

```md
- Claimed: T-000 <title>
- Moved: Ready -> Doing -> Review|Blocked
- Implementation: completed|blocked|failed
- Validation: command, pass/fail/not run
- Files changed: N
- Board: <card url>
```

### Blocker Check

Return only a compact summary:

```md
- Checked: T-000 <title>
- Moved: Blocked -> Ready
- Status: unblocked|still_blocked
- Blockers verified: N
- Missing: list
- Board: <card url>
```

### Review

Return only a compact summary:

```md
- Reviewed: T-000 <title>
- Moved: Review -> Done|Doing|Blocked
- Review: approved|needs_changes|blocked
- Issues: N major, N minor, N suggestions
- Board: <card url>
```
