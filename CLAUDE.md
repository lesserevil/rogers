# Project Instructions

Read `AGENTS.md` first. It is the operating guide for this repo.

## Backlog.md Quick Reference

Backlog.md is the only task tracker for this project. Install the local
CLI from `https://github.com/lesserevil/backlog.md` with:

```bash
make ensure-backlog
```

Common commands:

```bash
backlog task list --plain
backlog task view TASK-123 --plain
backlog task edit TASK-123 --status "In Progress" --plain
backlog task edit TASK-123 --status Done --final-summary "..." --plain
```

Use Makefile targets for quality gates: `make fmt-check`, `make build`,
`make test`, and `make lint`.
