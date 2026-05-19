# Getting Started with Rogers

## Installation

```bash
cargo install --path .
```

## Prerequisites

- A GitHub personal access token (PAT) with appropriate permissions
- A running dolt database for bead storage
- Network access to the target GitHub repository

## Configuration

Copy `config.example.yaml` to `config.yaml` and fill in your settings.
See [configuration.md](configuration.md) for full details.

## Running

```bash
rogers --config config.yaml
```

## First Run

On first run, Rodgers will:
1. Connect to the configured GitHub repository
2. Initialize the bead database
3. Begin triage on the configured schedule

See [cli.md](cli.md) for full command reference.