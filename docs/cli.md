# Rogers CLI Reference

## Synopsis

```
rogers [OPTIONS]
```

## Options

```
--config PATH          Path to config.yaml (required)
--dry-run              Preview actions without making GitHub API calls
--repo OWNER/REPO      Override the configured repository
-v, --verbose          Enable verbose output
-h, --help             Print help
--version              Print version
```

## Commands

```
rogers triage           Run triage on the configured repository
rogers sync             Sync bead status with resolved GitHub issues
rogers report           Generate a community health report
```

## Exit Codes

```
0   Success
1   Configuration error or API failure
2   Bead database error
```

## Examples

```bash
# Run triage against the configured repo
rogers --config config.yaml triage

# Preview what would be done without making API calls
rogers --config config.yaml --dry-run triage

# Override the repository from the command line
rogers --config config.yaml --repo NVIDIA-Omniverse/trickle triage
```