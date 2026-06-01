# Rogers CLI Reference

## Synopsis

```bash
rogers <COMMAND> [OPTIONS]
```

## Commands

```bash
rogers init --repo OWNER/REPO [--fix] [--json] [--github-token TOKEN]
rogers doctor [--config PATH] [--verbose] [--only CATEGORY[,CATEGORY...]] [--fix] [--json]
```

## `init`

Audits a GitHub repository for readiness to be managed by Rogers.

Options:

- `--repo OWNER/REPO` target repository.
- `--fix`, `-f` apply automated fixes where implemented.
- `--json`, `-j` output JSON.
- `--github-token TOKEN` repository admin token override. If omitted,
  Rogers reads `GITHUB_TOKEN`.

## `doctor`

Audits an existing Rogers installation for configuration problems and
state drift.

Options:

- `--config PATH` path to `config.yaml`; defaults to `./config.yaml`.
- `--verbose`, `-v` show detailed output including drift events.
- `--only`, `-o` comma-delimited categories: `config`, `auth`,
  `backlog`, `plans`, `repo`, `drift`.
- `--fix`, `-f` attempt supported fixes.
- `--json`, `-j` output JSON.

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Operational, Backlog.md, plan, or backport error |
| 2 | Invalid arguments, config file, I/O, YAML, or JSON error |
| 3 | Authentication, GitHub API, rate limit, or repository access error |

## Backlog.md CLI

Project task tracking is handled outside `rogers` by Backlog.md:

```bash
make ensure-backlog
backlog task list --plain
backlog task view TASK-123 --plain
backlog task edit TASK-123 --status Done --plain
```
