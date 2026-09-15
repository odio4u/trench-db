---
description: 'GitHub Project draft task tracker rule'
globs: ['**/*']
---

# Repository Rule — GitHub Project Task Tracker

Whenever the user asks to add a task, todo, work item, finding, investigation, or item to the tracker / project board / GitHub Project, create a **draft item** directly in the GitHub Project.

> Do **not** create a GitHub Issue.

## Target project

Always use the **odio4u** organization's project:

| Field | Value |
|-------|-------|
| Title | Trench DB WAL + Identity Tracker |
| Number | 1 |
| ID | `PVT_kwDODKKFis4BjXJj` |
| Owner | odio4u |

## How to create the task

Use the GitHub CLI `gh project item-create` command:

```bash
gh project item-create 1 \
  --owner odio4u \
  --title "Clear, actionable title" \
  --body "$(cat <<'EOF'
## Context

...

## Acceptance criteria

- [ ] ...
- [ ] ...

## Implementation notes

...

## Related references

- ...
EOF
)"
```

The created item **must** be a draft Project item.

Do **not** use:

```bash
gh issue create
```

Do not create a repository Issue just to add it to the Project.

## Required task quality

### Title

The title must be:

- Clear
- Actionable
- Specific
- Concise

#### Avoid vague titles such as:

- Fix stuff
- Update code
- WAL issue
- Investigate this

#### Prefer titles such as:

- Add WAL segment recovery after unclean shutdown
- Preserve identity metadata across WAL replay
- Handle missing WAL segments during startup

### Body

The body should contain enough information for another developer to understand and work on the task.

Include these sections when applicable:

```markdown
## Context

Explain why the task exists, the problem being solved, and any relevant background.

## Acceptance criteria

- [ ] ...
- [ ] ...

## Implementation notes

Describe relevant design considerations, constraints, expected behavior, files, modules, or technical approach.

## Related references

- `path/to/relevant/file.go`
- `path/to/another/file.go`
- Related branch: `branch-name`
- Related PR: `https://github.com/...`
- Related documentation: `https://...`
```

For short findings or investigative tasks, **Acceptance criteria** may be omitted. In that case, clearly document the investigation goal and expected outcome under **Implementation notes**.

Do not create a task with only a one-line description.

If the user's request is brief, inspect the repository context and expand it into a useful Project task. Do not invent technical details when the repository does not provide enough information.

### Repository context

Before creating the task, use the available repository context to identify relevant:

- Files
- Packages/modules
- Existing implementations
- Tests
- Configuration
- Documentation
- Branches
- Pull requests
- Related Project items

Only include references that are actually relevant.

### Status and Project fields

The task should initially be placed in the appropriate default status, normally:

- `Todo`
- `Backlog`

If the GitHub CLI and Project configuration allow Project fields to be set, populate relevant fields such as:

- **Status:** `Todo` or `Backlog`
- **Area:** WAL, identity, transport, storage, CLI, etc.
- **Priority:** when the user provides or repository context clearly establishes one

Do not invent a priority or classification when there is insufficient information.

### Markdown

The `--body` value supports GitHub-flavored Markdown.

Prefer a heredoc for multi-line task descriptions:

```bash
gh project item-create 1 \
  --owner odio4u \
  --title "Clear, actionable title" \
  --body "$(cat <<'EOF'
## Context

Task context goes here.

## Acceptance criteria

- [ ] First requirement
- [ ] Second requirement

## Implementation notes

Implementation details go here.

## Related references

- `path/to/file.go`
EOF
)"
```

## Verification

After creating the item, verify that the Project item was created successfully.

The expected result is a draft item in **Project #1** under the **odio4u** organization.

Do not report the task as created if the `gh project item-create` command fails.