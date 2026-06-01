# Agent Instructions

This file covers the day-to-day operating rules for agents working in
this repo: task tracking, doc sync, quality gates, and session-close
protocol.

> If a `CONTRIBUTING.md` exists, it is the workflow source of truth and
> takes precedence on methodology questions.

## Task Tracking With Backlog.md

This project uses **Backlog.md** for all task tracking. The Backlog.md
CLI must come from `https://github.com/lesserevil/backlog.md`; run
`make ensure-backlog` to install the pinned local CLI into `.tools/backlog`.

### Quick Reference

```bash
make ensure-backlog
backlog task list --plain
backlog task view TASK-123 --plain
backlog task edit TASK-123 --status "In Progress" --plain
backlog task create "Title" --description "..." --priority medium --plain
backlog task edit TASK-123 --status Done --final-summary "..." --plain
```

### Rules

- Use `backlog` for all task tracking.
- Keep task files under `backlog/`; do not create alternate task stores.
- File follow-up work with enough context for a competent developer to
  execute it without reading prior conversations.
- Do not use TodoWrite, TaskCreate, markdown TODO lists, or `MEMORY.md`
  files for project tracking.
- Avoid commands that open an editor; use CLI flags such as
  `--description`, `--notes`, `--final-summary`, and `--plain`.

### Task Quality

Every task must stand alone. Include what to do, why it matters, how to
verify it, and any edge cases or project-specific terminology that a new
contributor could miss.

Priorities are Backlog.md priorities: `high`, `medium`, or `low`.

## Documentation Must Match Code

User docs are part of the contract. Any commit that changes user-visible
behavior must update the relevant docs in the same commit.

User-visible surfaces that trigger doc updates include:

- CLI flags, commands, defaults, and exit codes
- Build commands and packaging scripts
- System dependencies and runtime requirements
- Configuration schema and environment variable names
- Platform support

Documentation layout:

- `docs/` contains user-facing setup guides, references, runbooks, and
  operator documentation.
- `plans/` contains design and implementation notes.

When creating diagrams in documentation, use Mermaid code blocks.

## Plans To Tasks To Code

For non-trivial planned work:

1. Update or create the relevant `plans/*.md` document.
2. Include an explicit `## Acceptance Criteria` section with testable
   checklist items.
3. Create Backlog.md tasks from the plan with descriptions that stand
   alone and reference the plan section for traceability.
4. Implement the work, update docs, and close the task only after the
   acceptance criteria are demonstrably satisfied.
5. Mark the plan complete only after all acceptance criteria are met,
   not merely because tasks were closed.

## Use Makefile Targets

Always prefer Makefile targets when one exists for the action:

```bash
make fmt
make fmt-check
make build
make test
make lint
```

Check `make help` before using raw toolchain commands.

## Test Coverage Required

All code changes must be covered by tests. Bug fixes need a test that
reproduces the bug. New functions and methods need focused unit tests or
integration coverage following the existing project pattern.

## Commit Attribution

Do not add agent/model attribution trailers. The codebase author is the
human owner; the underlying model is an implementation detail.

## Non-Interactive Shell Commands

Use non-interactive flags for file operations that may prompt:

```bash
cp -f source dest
mv -f source dest
rm -f file
rm -rf directory
```

Use batch/non-interactive flags for tools such as `ssh`, `scp`, package
managers, and other commands that may prompt.

## Session Completion

When ending a work session:

1. File Backlog.md tasks for remaining work.
2. Run quality gates if code changed.
3. Close or update the active task.
4. Commit all intended changes.
5. Run:
   ```bash
   git pull --rebase
   git push
   git status
   ```
6. Verify `git status` reports the branch is up to date with origin.

Work is not complete until `git push` succeeds.
