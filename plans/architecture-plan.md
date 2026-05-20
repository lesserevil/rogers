# Architecture Plan

**Status:** Draft  
**Plan:** plans/architecture-plan.md  

---

## Summary

Rodgers is a github-native community relations agent. It runs on a schedule, reads GitHub issues and discussions, and manages the full triage-to-release lifecycle entirely through the GitHub API and a local beads database. No side channels — all communication with requestors happens as comments on their issues or discussions.

---

## Guiding Principles

1. **Be github-native.** Everything Rodgers does — reading, writing, triaging, releasing — flows through the GitHub API. No direct email, no webhooks outside github, no external services.
2. **Beads are the work log.** Rodgers tracks all planned and in-flight work in beads. GitHub issues track community-facing state. The two must stay in sync but serve different audiences.
3. **Humans are in the loop.** Rodgers files beads for decisions that need human judgment. It never acts unilaterally on gate decisions — it asks via GitHub and waits.
4. **Deep planning first.** Implementation begins only after plans are written, reviewed, and turned into beads. Planning is not a checkbox — it is the methodology.

---

## System Components

```
┌─────────────────────────────────────────────────────────┐
│  Scheduler (cron / systemd timer)                       │
│  Triggers rogers on a configurable interval             │
└────────────────────────┬────────────────────────────────┘
                         │ runs
                         ▼
┌─────────────────────────────────────────────────────────┐
│  Rodgers CLI (Rust)                                      │
│                                                          │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────────┐ │
│  │  Triage      │  │  Question    │  │  Release      │ │
│  │  Engine      │  │  Router      │  │  Manager      │ │
│  └──────────────┘  └──────────────┘  └───────────────┘ │
│                                                          │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────────┐ │
│  │  Bead        │  │  GitHub      │  │  Backport     │ │
│  │  Controller  │  │  API Client  │  │  Manager      │ │
│  └──────────────┘  └──────────────┘  └───────────────┘ │
└────────────────────────┬────────────────────────────────┘
                         │
          ┌──────────────┴──────────────┐
          ▼                             ▼
┌─────────────────────┐    ┌─────────────────────────────┐
│  GitHub API         │    │  Beads Database (dolt)      │
│                     │    │                             │
│  · Read issues      │    │  · Open / filed beads       │
│  · Read discussions │    │  · Epics and child beads    │
│  · Post comments    │    │  · Status transitions       │
│  · Update labels    │    │  · Plan references          │
│  · Manage releases  │    │                             │
│  · Manage branches  │    │                             │
└─────────────────────┘    └─────────────────────────────┘
```

### Component Responsibilities

**Scheduler**  
Runs `rogers [command]` on a configurable interval (default: 60 minutes). No daemon — each run is a stateless invocation that reads state from GitHub + beads, then exits. State changes are durable in beads or GitHub after each run.

**Triage Engine**  
Reads new and updated issues since the last run. Classifies each as: Bug, Feature Request, Question, or Other. Applies the triage state machine (see plans/triage-workflow-plan.md).

**Question Router**  
Handles issues classified as Questions. Checks existing docs for answers. Files doc-gap beads if no answer exists. Posts comment links to documentation when available, or informs requestor the question is being addressed when a doc-gap bead is filed.

**Release Manager**  
Monitors release branches and main for readiness. Proposes releases to a human via a GitHub Discussion when criteria are met. Cuts releases only after human approval.

**Backport Manager**  
When a fix lands on main or a release branch, creates beads to cherry-pick the fix to older released branches. Files those beads with the correct acceptance criteria and links them to the original fix.

**Bead Controller**  
Reads and writes beads. Creates epic beads for complex work. Files child beads under epics. Closes beads when linked GitHub issues are resolved. Manages the bead-to-GitHub-issue linkage table.

**GitHub API Client**  
Thin wrapper around reqwest for the GitHub REST API. Handles auth via PAT from env var. All communication with GitHub flows through this client — no raw API calls outside this module.

---

## Data Model

### GitHub Issue States (community-facing)

| Label | Meaning |
|-------|---------|
| `bug` | A bug report, triaged |
| `feature` | A feature request, triaged |
| `question` | A question from the community |
| `needs-information` | Rodgers has asked for clarification from the requestor |
| `ready-for-review` | Rodgers has determined the issue has enough information; awaiting human decision |
| `will-not-do` | Human has decided this will not be worked |
| `ready-for-work` | Human has approved this for implementation |
| `in-progress` | Work is underway |
| `done` | Work is complete and verified |

### Bead States (work-tracking)

Beads follow the standard bd workflow: `open → claimed → closed`, supplemented by type and priority.

| Bead Type | Use |
|-----------|-----|
| `epic` | Top-level work unit covering a feature or fix |
| `feature` | Implementation work for a specific part of an epic |
| `bug` | Bug fix work |
| `docs` | Documentation update work |
| `release` | Release management work |
| `backport` | Cherry-pick fix to older release branch |
| `triage` | Triage state machine step (interrogative, informational) |

---

## Configuration

All configuration via `config.yaml` at the repo root, with env-var overrides.

Relevant config keys:
- `scheduler.interval_minutes` — polling interval
- `github.{owner, repo, token}` — target repo and auth
- `beads.{remote, database}` — dolt bead storage
- `triage.{default_labels, bot_labels, close_labels, assignees}` — triage behavior
- `release.approval_discussion_category` — GitHub Discussion category for release approvals

---

## Direction for Later

- **GitHub App / webhooks**: Supported as a future optimization when sub-minute reaction times are needed for new issues. Not required for schedule-based polling.
- **Multi-repo support**: Rodgers targets one repo at a time by config. A wrapper script or separate bd-database per repo is the scaling path.
- **Agent delegation**: Rodgers can file beads that describe work for other agents. The `Plan:` line and acceptance criteria in each bead are the handoff contract.

---

## Acceptance Criteria

- [ ] AC-1: Rodgers can be configured via `config.yaml` to target a specific GitHub repo
- [ ] AC-2: All GitHub state changes (labels, comments, closes) go through the GitHub API client module
- [ ] AC-3: All work tracking (epics, child beads, triage beads) goes through the Bead Controller
- [ ] AC-4: Rodgers runs on a configurable schedule and exits after each run with no persistent process
- [ ] AC-5: Configuration supports env-var overrides for all keys
- [ ] AC-6: No code paths exist that communicate with anyone outside GitHub Issues or Discussions