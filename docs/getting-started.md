# Getting Started With Rogers

## Prerequisites

- Rust and Cargo
- Node.js and npm
- GitHub personal access token
- LLM provider API key

## Install Local Tools

```bash
make ensure-backlog
```

This installs the Backlog.md CLI from
`https://github.com/lesserevil/backlog.md` into `.tools/backlog`.

## Configure

```bash
cp config.example.yaml config.yaml
```

Edit `config.yaml`. At minimum, set:

- `github.owner`
- `github.repo`
- `github.token` or `${RODGERS_GITHUB_TOKEN}`
- `backlog.path`
- `llm.model`
- `llm.api_key` or `${OPENAI_API_KEY}`

## Build

```bash
make fmt-check
make build
make test
make lint
```

## Run

```bash
rogers init --repo owner/repo
rogers doctor --config config.yaml
```

See `docs/cli.md` for command details and `docs/configuration.md` for
the full configuration reference.
