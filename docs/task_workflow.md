# Vanguard Development Task Workflow

This workflow defines how development tasks are created in Planka from `specs/product/VANGUARD_MINIMAL_CLI_SPEC_v0.2.md`.

- Planka instance: `https://tasks.finalq.xyz`
- Board ID: `1766164900683449436`
- See `docs/planka_usage.md` for full API details and list/label IDs.

## Source Of Work

Primary source:

- `specs/product/VANGUARD_MINIMAL_CLI_SPEC_v0.2.md`

Secondary sources:

- Implementation discoveries
- Bugs found during validation
- Explicit product decisions

## Creating Tasks From The Spec

For each spec area:

1. Identify the desired behavior.
2. Identify the implementation surface.
3. Identify missing decisions.
4. Create definition tasks for missing decisions.
5. Create implementation tasks for clear work.
6. Link implementation tasks to blockers.
7. Add acceptance criteria and done conditions.

## Ambiguity Rule

If a task contains an unresolved question that changes architecture, persistence, external behavior, or MVP scope, do not hide that question in the task notes.

Create a blocking definition task.

Examples:

- Missing Objective entity details block objective intake implementation.
- Missing merge policy blocks workspace completion behavior.
- Missing OpenCode API contract blocks execution engine implementation.

## Blocking Rules

A task is blocked when it cannot be completed correctly without another task being done first.

Represent blockers in Planka with:

- `Blocked by:` in the blocked card description.
- `Unblocks:` in the blocking card description.
- `blocked` label on blocked cards.
- `needs-decision` label when the blocker is a definition task.

## List Movement

- `Definitions Needed`: decision tasks required to unblock implementation.
- `Backlog`: known work that is not yet ready or not prioritized.
- `Ready`: clear work with no unresolved blockers.
- `Doing`: actively being implemented.
- `Review`: implemented and waiting for review/validation.
- `Blocked`: otherwise-ready work that cannot progress.
- `Done`: completed and validated.
- `Won't Do`: intentionally not being done.

## Card Size

Prefer cards that can be completed in one focused engineering pass.

Split a card if it includes multiple independent deliverables, for example:

- Backend API and frontend UI can be separate cards.
- Evaluator stub and real OpenCode evaluator should be separate cards.
- Workspace creation and workspace cleanup can be separate cards.

Keep a larger card only if splitting would create fake boundaries.

## Definition Task Closure

A definition task is done when:

- The decision is written in the card.
- Any affected docs/spec files are updated if needed.
- Blocked cards have been updated to reference the decision.
- The decision is specific enough to implement.

## Implementation Task Closure

An implementation task is done when:

- Acceptance criteria pass.
- Done conditions are satisfied.
- Relevant docs are updated.
- Validation has been run or an explicit validation gap is recorded.
- No known blocker remains unresolved.

## Assigning A Board Task To An Agent

Assign work from Planka one card at a time.

Manual flow:

1. Pick the lowest `Execution order` card in `Ready`.
2. Do not pick a `Blocked` card unless its blockers are resolved and the card is moved back to `Ready`.
3. Move the selected card from `Ready` to `Doing`.
4. Give the agent the card title, URL, description, paths, implementation steps, acceptance criteria, verification command, out-of-scope items, blockers, and unblocked cards.
5. The agent reads `handoff.md`, implements only the card scope, preserves unrelated changes, runs verification, and reports the result.
6. Move the card to `Review` when implementation and verification are complete.
7. Move the card to `Blocked` if a real blocker prevents progress, and record the blocker in the card or final summary.

Automated orchestration flow:

```sh
node .opencode/scripts/claim-next-board-card.cjs
```

This selects the next `Ready` card by execution order, moves it to `Doing`, and emits a `prompt_context` for the `task-implementer` subagent.

To claim a specific Ready card:

```sh
node .opencode/scripts/claim-next-board-card.cjs --card T-001
```

To inspect a card without claiming it:

```sh
node .opencode/scripts/get-board-card.cjs --card T-001
```

To move a completed card:

```sh
node .opencode/scripts/move-board-card.cjs --card T-001 --list review
```

Agent prompt template:

```text
You are implementing Planka card <CARD_ID_OR_URL>: <TITLE>.
Use the card description as the implementation contract.
Rules: read handoff.md first; implement only this card; do not do out-of-scope items; preserve unrelated changes; run the verification command from the card; if blocked, stop and report the blocker.
Card URL: <URL>

<CARD DESCRIPTION>
```

The implementation agent must not move Planka cards. Board movement is owned by the orchestrator or main agent.

## MVP Bias

Prefer MVP-safe decisions:

- Explicit state over implicit agent memory.
- Manual review over automatic merge.
- Stubbed integration before full OpenCode execution.
- Simple persistence before event-sourced complexity.
- Clear operator visibility before automation.
