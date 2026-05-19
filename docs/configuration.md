# Configuration Reference

Rogers is configured via a YAML file. See `config.example.yaml` for a
commented baseline.

## Top-Level Keys

```yaml
github:
  owner: OWNER          # GitHub organization or user
  repo: REPO           # Repository name
  token: TOKEN         # GitHub personal access token

scheduler:
  interval_minutes: 60 # How often to run triage (minutes)

beads:
  remote: REMOTE       # dolt remote URL (or omit for local-only)
  database: rogers     # dolt database name

triage:
  default_labels:      # Labels to apply by default
    - triage
  bot_labels:         # Labels indicating bot-originated issues
    - luditus
    - circular
  close_labels:       # Labels that trigger auto-close
    - stale
  assignees: []       # GitHub usernames to assign triage issues to
```

## Environment Variable Overrides

Any config key can be overridden via environment variables using the
pattern `ROGERS_<UPPER_SNAKE_CASE_KEY>`:

```bash
export ROGERS_GITHUB_TOKEN=ghp_xxxxxxxxxxxxx
export ROGERS_SCHEDULER_INTERVAL_MINUTES=30
```

Environment variables take precedence over the YAML file.

## Authentication

Rodgers uses a GitHub personal access token (PAT) for all API operations.
The token should have permissions for:

- `repo` (full control) — for private repos
- `public_repo` — for public repos only

Never commit a `config.yaml` with a real token. Use `config.example.yaml`
as the tracked template and store secrets in environment variables or a
personal dotfile.