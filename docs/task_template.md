# Task Template

Use this template for Vanguard development cards in Planka. Keep the format direct and execution-oriented. Do not use Jira-style story ceremony unless it adds concrete value.

## Title

Use an imperative, specific title.

Examples:

- Implement task CRUD API
- Define Objective persistence model
- Create evaluator stub output parser

## Description Template

```md
Objective:
<What has to be achieved.>

Context:
<Why this exists. Reference specs/product/VANGUARD_MINIMAL_CLI_SPEC_v0.2.md section(s) when possible.>

Scope:
- <Included work item>
- <Included work item>

Out of scope:
- <Explicitly excluded work item>
- <Explicitly excluded work item>

Blocked by:
- <Blocking card title and URL, or "None">

Unblocks:
- <Card title and URL, or "None">

Acceptance criteria:
- <Verifiable condition>
- <Verifiable condition>

Done when:
- <Close condition>
- <Close condition>

Notes:
- <Decision, risk, implementation note, or open question>
```

## Checklist Usage

Use checklists for concrete subtasks, not for repeating acceptance criteria.

Recommended checklist names:

- Implementation
- Validation
- Documentation
- Follow-ups

## Blocking Definition Tasks

If a task cannot be implemented safely because a product or technical decision is missing, create a separate definition task and make it block the implementation task.

Definition task title format:

```text
Define <decision area>
```

Example:

```text
Define Objective persistence model
```

The blocked implementation task must include:

```md
Blocked by:
- Define Objective persistence model (<Planka card URL>)
```

The definition task must include:

```md
Unblocks:
- Implement objective intake (<Planka card URL>)
```

## Labels

Apply only useful labels. Avoid label noise.

Common label groups:

- Area: `frontend`, `backend`, `storage`, `evaluator`, `orchestrator`, `workspace`, `opencode`, `validation`, `observability`, `docs`
- Type: `definition`, `foundation`, `feature`, `bug`, `spike`
- Priority: `high`, `medium`, `low`
- State hint: `mvp`, `blocked`, `needs-decision`

## Quality Bar

A task is ready when another engineer can pick it up without asking what the task means.

A task is not ready if:

- It lacks acceptance criteria.
- It depends on an undefined policy.
- It mixes unrelated implementation areas.
- It says only "improve", "clean up", or "handle" without a concrete end state.
