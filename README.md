# Rogers

Rogers is a GitHub-native community relations agent. It audits repository
setup, triages issues, routes questions, manages backport and release
workflows, and records implementation work in Backlog.md task files.

## Requirements

- Rust toolchain with Cargo
- Node.js and npm for the Backlog.md CLI
- GitHub personal access token for repository operations

## Setup

```bash
make ensure-backlog
cp config.example.yaml config.yaml
```

Edit `config.yaml` and set the GitHub repo, token environment variable,
LLM endpoint, and `backlog.path` if you do not use the default `backlog/`
directory.

## Build And Test

```bash
make fmt-check
make build
make test
make lint
```

## CLI

```bash
rogers init --repo owner/repo
rogers doctor --config config.yaml
```

See `docs/cli.md` and `docs/configuration.md` for details.

## Task Tracking

This repo uses Backlog.md from `https://github.com/lesserevil/backlog.md`.
Task files live in `backlog/`, and the local CLI is installed by
`make ensure-backlog` into `.tools/backlog`.
