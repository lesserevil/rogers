# Documentation

The docs/ directory contains user-facing documentation for the Rogers project, covering setup, command usage, configuration, and troubleshooting.

## Docs files

- `getting-started.md` — installation, prerequisites, and quick-start
- `cli.md` — complete command reference for `rogers init` and `rogers doctor`
- `configuration.md` — complete config schema reference
- `troubleshooting.md` — common failure modes and how to resolve them

## Keeping docs in sync with code

Per [`../AGENTS.md`](../AGENTS.md), every commit that changes
user-visible behavior must update the relevant doc in the same
commit. The pre-commit hook warns when staged code touches a
feature area whose doc isn't also staged.