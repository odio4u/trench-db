---
description: 'GitHub Project task tracker rule'
globs: ['**/*']
---

# Repository Rule — Task Tracker

Whenever the user asks to add a task, todo, work item, or issue to the tracker / project board / GitHub Project, follow this rule.

## Target project

Always use the `odio4u` organization's project:

- **Title:** `Trench DB WAL + Identity Tracker`
- **Number:** `1`
- **ID:** `PVT_kwDODKKFis4BjXJj`
- **Owner:** `odio4u`

## How to add the task

Use the GitHub CLI, for example:

```sh
gh project item-create 1 --owner odio4u --title "Clear, actionable title" --body "$(cat <<'EOF'
## Context
...

## Acceptance criteria
- [ ] ...
- [ ] ...

## Implementation notes
...

## Related files / branches / PRs
- ...
EOF
)"
```

## Required task quality

- **Title:** Clear, actionable, and specific. Do not use vague one-liners.
- **Body:** Must include at least:
  1. **Context** — why this task exists and what problem it solves.
  2. **Implementation notes** — relevant design, files, modules, or known constraints.
  3. **Related references** — related files, branches, PRs, issues, or docs links.

Note: Acceptance criteria are normally required, but for short findings or investigative tasks the creator may omit them and document the intent in the Implementation notes instead.

Do **not** create a task with only a one-line description. If the user gives a brief request, expand it into a proper tracker item using the repository context.

## Status / fields

If possible, set the status to `Todo`/`Backlog` and tag the relevant area (e.g., `WAL`, `identity`, `transport`, `storage`, `CLI`).
