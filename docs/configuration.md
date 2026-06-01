# Configuration Reference

Rogers is configured with YAML. Copy `config.example.yaml` to
`config.yaml` and fill in local values.

## Required Sections

```yaml
github:
  owner: OWNER
  repo: REPO
  token: ${RODGERS_GITHUB_TOKEN}
  api_url: https://api.github.com

scheduler:
  interval_minutes: 5
  enabled: true

backlog:
  path: backlog

llm:
  provider: openai
  base_url: https://api.openai.com/v1
  model: gpt-4o-mini
  api_key: ${OPENAI_API_KEY}
```

## Backlog.md

`backlog.path` points to the Backlog.md task directory. Relative paths
are resolved from the process working directory. The default is
`backlog/`, which contains `config.yml`, `tasks/`, and `completed/`.

Install the Backlog.md CLI with:

```bash
make ensure-backlog
```

The Makefile installs from `https://github.com/lesserevil/backlog.md`.

## Environment Variables

String fields support `${ENV_VAR}` interpolation before validation.
Common values:

```bash
export RODGERS_GITHUB_TOKEN=ghp_xxxxxxxxxxxxx
export OPENAI_API_KEY=sk-xxxxxxxxxxxxx
```

## Validation

`rogers doctor --config config.yaml --only config` checks:

- `github.owner`, `github.repo`, and `github.token`
- positive `scheduler.interval_minutes`
- non-empty `backlog.path`
- `llm.model` and `llm.api_key`
- placeholder-looking token values
- release and triage warnings

Never commit a `config.yaml` containing real credentials.
