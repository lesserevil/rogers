# Getting Started with Rogers

## Installation

To get started, install Rodgers from source using Cargo:

```bash
cargo install --path .
```

### Prerequisites

- **Rust toolchain**: Ensure you have `rustc` and `cargo` installed (e.g., via `rustup`).
- **GitHub personal access token (PAT)**: Obtain a token with appropriate scopes for accessing the target repository.
- **Network access**: Rodgers must be able to reach GitHub's API endpoints.

## Quick Start

Follow these steps to perform your first audit of a GitHub repository.

### 1. Create a configuration file

Copy the provided example configuration to `config.yaml` and fill in your values:

```bash
cp config.example.yaml config.yaml
```

Edit `config.yaml` and set the required keys:

- `github.owner`: The GitHub owner (user or organization) of the repository.
- `github.repo`: The name of the repository.
- `github.token`: Your GitHub PAT. Rodgers reads this from the `GITHUB_TOKEN` environment variable; you can also set it explicitly in the config using `${RODGERS_GITHUB_TOKEN}` placeholder.

### 2. Set up authentication

Export your GitHub token before running Rodgers:

```bash
export GITHUB_TOKEN=your_personal_access_token
```

Rodgers requires this token to authenticate with the GitHub API. If the token is missing or invalid, `rogers init` will exit with code **3** and display an error such as "Repository not found or not accessible" or "Authentication failed".

### 3. Run the initialization audit

Execute the `init` command to audit a repository:

```bash
rogers init --repo owner/repo [--fix]
```

- `--repo owner/repo`: Specify the target repository in `owner/repo` format.
- `--fix` (optional): Apply automated label fixes where possible.

The audit checks the following aspects of the repository:

- Label definitions (e.g., triage, bug, enhancement)
- Issue templates
- Release workflow configuration
- Discussion category setup
- General workflow settings

### 4. Review the audit report

On successful execution, Rodgers prints a summary of findings categorized as:

- **[BLOCKER]**: Issues that must be resolved before proceeding.
- **[WARN]**: Non‑critical observations.
- **[INFO]**: General information about the repository state.

Address any blockers before moving forward.

### 5. Continue with subsequent commands

After a successful init, you can run other commands such as `rogers doctor` to check the health of your installation, or proceed with release management workflows.

## Configuration Overview

The full list of configuration keys is documented in `docs/configuration.md`. Key required fields include:

- `github.owner`: Repository owner.
- `github.repo`: Repository name.
- `github.token`: GitHub personal access token.

For a complete reference of all available keys, see the configuration documentation.

## Useful Links

- **Full command reference**: [docs/cli.md](docs/cli.md)
- **Configuration guide**: [docs/configuration.md](docs/configuration.md)