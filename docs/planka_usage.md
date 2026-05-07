# Planka Usage For Vanguard Development

Planka is used to manage the development of Vanguard. It is not part of Vanguard's own product model. Vanguard will have its own board/task model as described in `specs/product/VANGUARD_MINIMAL_CLI_SPEC_v0.2.md`.

## Instance

- Base URL: `https://tasks.finalq.xyz`
- Project ID: `1765366778302563359`
- Board name: `Vanguard`
- Board ID: `1766164900683449436`

## Lists

IDs are filled after creation.

| List | Purpose | ID |
| --- | --- | --- |
| Definitions Needed | Decisions that block implementation work | `1766165135480587360` |
| Backlog | Identified work not ready or not scheduled | `1766165136579495009` |
| Ready | Clear, executable work | `1766165137258972258` |
| Doing | Work currently in progress | `1766165138718590051` |
| Review | Work complete enough for review | `1766165139809109092` |
| Blocked | Work blocked by dependencies or external issues | `1766165140832519269` |
| Done | Finished work | `1766165141470053478` |
| Won't Do | Explicitly rejected or superseded work | `1766165142157919335` |

## Labels

IDs are filled after creation.

| Label | Purpose | ID |
| --- | --- | --- |
| definition | Product or technical decision task | `1766165700788880488` |
| foundation | Project foundation work | TBD |
| feature | Product functionality | TBD |
| bug | Defect or regression | TBD |
| spike | Investigation/prototype | TBD |
| frontend | Frontend/UI work | TBD |
| backend | Backend/API/runtime work | TBD |
| storage | Persistence and migrations | TBD |
| evaluator | Evaluator/check/gap generation | TBD |
| orchestrator | Runtime orchestration loop | TBD |
| workspace | Git worktree/workspace lifecycle | TBD |
| opencode | OpenCode server/session integration | TBD |
| validation | CoD/check validation | TBD |
| observability | Events, logs, summaries, debugging surface | TBD |
| docs | Documentation and process | TBD |
| high | High priority | TBD |
| medium | Medium priority | TBD |
| low | Low priority | TBD |
| mvp | Required for MVP | TBD |
| blocked | Currently blocked | TBD |
| needs-decision | Needs a decision before implementation | TBD |

## Card Convention

Cards represent development work for Vanguard. A card can be a feature, task, definition, bug, or spike. Do not force Jira-style stories.

Each substantial card should include:

- Objective
- Context
- Scope
- Out of scope
- Blocked by
- Unblocks
- Acceptance criteria
- Done when
- Notes

Use `docs/task_template.md` as the source template.

To assign work to an implementation agent, move one `Ready` card to `Doing` and pass the card description as the implementation contract. The repo-local automated flow is documented in `docs/task_workflow.md#assigning-a-board-task-to-an-agent`.

## Subtasks

Use Planka checklists for subtasks.

Checklist items should be concrete implementation steps, for example:

- Add SQLite table
- Add repository function
- Add API route
- Add frontend state handling
- Add validation coverage

API mapping:

- A checklist is a Planka `TaskList`.
- A checklist item is a Planka `Task`.
- Create checklist: `POST /api/cards/:cardId/task-lists`.
- Create checklist item: `POST /api/task-lists/:taskListId/tasks`.

## Blockers

Planka does not provide Jira-style dependency semantics. We represent blockers explicitly.

Blocked card requirements:

- Include `Blocked by:` in the description with card title and URL.
- Apply `blocked` label.
- Apply `needs-decision` label if the blocker is a missing definition.
- Put the card in `Blocked` if it would otherwise be ready.

Blocking definition task requirements:

- Put it in `Definitions Needed`.
- Apply `definition`, `needs-decision`, and priority labels.
- Include `Unblocks:` in the description with card title and URL when available.

## Task Creation Policy

When deriving work from `specs/product/VANGUARD_MINIMAL_CLI_SPEC_v0.2.md`:

- Create definition tasks for ambiguous policies before implementation tasks.
- Create implementation tasks with as much context as possible.
- Prefer several clear cards over one vague large card.
- Link cards through `Blocked by` and `Unblocks` text.
- Keep Vanguard domain truth in the code/spec, not in Planka.

## Known IDs

This section is updated after board setup.

```json
{
  "projectId": "1765366778302563359",
  "boardId": "1766164900683449436",
  "lists": {
    "Definitions Needed": "1766165135480587360",
    "Backlog": "1766165136579495009",
    "Ready": "1766165137258972258",
    "Doing": "1766165138718590051",
    "Review": "1766165139809109092",
    "Blocked": "1766165140832519269",
    "Done": "1766165141470053478",
    "Won't Do": "1766165142157919335"
  },
  "labels": {
    "definition": "1766165700788880488"
  },
  "cardLabels": {
    "Define Objective persistence model -> definition": "1766168233729066091"
  },
  "cards": {
    "Define Objective persistence model": "1766165814404187241"
  }
}
```

## API Notes

OpenAPI source:

- `docs/planka_swagger.json`
- API version: `2.0.1`
- Base API path from Swagger: `/api`

Authentication endpoint confirmed:

```http
POST /api/access-tokens
```

API key authentication is supported by Swagger and is preferred for local automation:

```http
X-Api-Key: <BOARD_API_KEY>
```

Rules:

- Store the API key only in the runtime environment, for example `BOARD_API_KEY`.
- Do not write API keys to repository files, docs, handoff files, command files, traces, or logs.
- Do not pass API keys through subagent JSON input.

API key creation endpoint confirmed:

```http
POST /api/users/:userId/api-key
```

The full API key is returned only once and cannot be retrieved again.

Board creation endpoint confirmed:

```http
POST /api/projects/:projectId/boards
```

List creation endpoint confirmed:

```http
POST /api/boards/:boardId/lists
```

Required list payload fields:

```json
{
  "name": "Backlog",
  "type": "active",
  "position": 131072
}
```

Label creation endpoint confirmed:

```http
POST /api/boards/:boardId/labels
```

The local Planka writer script creates missing labels before assigning them to cards. Known label IDs from `planka_target.labels` are reused; unknown or `TBD` labels are created on the target board and reported in writer warnings.

Required label payload fields:

```json
{
  "name": "definition",
  "color": "berry-red",
  "position": 65536
}
```

Card creation endpoint confirmed:

```http
POST /api/lists/:listId/cards
```

Card move/update endpoint confirmed:

```http
PATCH /api/cards/:cardId
```

Move payload fields:

```json
{
  "boardId": "1766164900683449436",
  "listId": "1766165137258972258",
  "position": 65536
}
```

Required card payload fields:

```json
{
  "name": "Define Objective persistence model",
  "description": "...",
  "type": "project",
  "position": 65536
}
```

Card type values from Swagger:

```text
project | story
```

Default for Vanguard development cards:

```text
project
```

Card label assignment endpoint confirmed from Swagger and validated against the seed card:

```http
POST /api/cards/:cardId/card-labels
```

Required card label payload fields:

```json
{
  "labelId": "1766165700788880488"
}
```

Card label removal endpoint confirmed from Swagger:

```http
DELETE /api/cards/:cardId/card-labels/labelId::labelId
```

Example with real IDs:

```http
DELETE /api/cards/1766165814404187241/card-labels/labelId:1766165700788880488
```

Task list/checklist creation endpoint confirmed from Swagger:

```http
POST /api/cards/:cardId/task-lists
```

Required task list payload fields:

```json
{
  "position": 65536,
  "name": "Implementation"
}
```

Optional task list payload fields:

```json
{
  "showOnFrontOfCard": true,
  "hideCompletedTasks": false
}
```

Checklist item creation endpoint confirmed from Swagger:

```http
POST /api/task-lists/:taskListId/tasks
```

Required task payload fields:

```json
{
  "position": 65536,
  "name": "Add SQLite table"
}
```

Optional task payload fields:

```json
{
  "linkedCardId": "1766165814404187241"
}
```

Use this endpoint for assigning labels to cards in task creation automation.

## Seed Card

Created one seed definition card to validate card creation before delegating bulk creation to a subagent/script.

| Card | List | ID |
| --- | --- | --- |
| Define Objective persistence model | Definitions Needed | `1766165814404187241` |

Seed card labels:

| Card | Label | CardLabel ID |
| --- | --- | --- |
| Define Objective persistence model | definition | `1766168233729066091` |
