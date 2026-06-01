---
id: TASK-48
title: 'AC-3: rogers init --fix creates missing required labels via GitHub API'
status: Done
assignee: []
created_date: 2026-05-20 05:23
updated_date: 2026-05-21 04:51
labels:
- rodgers:parent=rogers-zql
- rodgers:type=init
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-ebo
  state: closed
  parent_id: rogers-zql
  dependencies: []
  branch_name: rogers-ebo
  target_branch: null
  url: null
  created_at: '2026-05-20T05:23:10Z'
  updated_at: '2026-05-21T04:51:08Z'
  closed_at: '2026-05-21T04:51:00Z'
parent: TASK-5
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/init-plan.md §--fix Flag Behavior → Acceptance Criteria AC-3

WHAT TO DO
Implement rogers init --fix creating missing required labels via GitHub API.

Create/modify files:
- src/init/fix.rs - Auto-fix implementations
- src/init/mod.rs - --fix flag handling
- src/github/client.rs - Label creation API
- src/labels.rs - Canonical label definitions (colors, names)

Required labels (from architecture-plan.md):
- bug, feature, question — triage classification
- needs-information, needs-documentation — routing state
- ready-for-review, will-not-do, ready-for-work, in-progress — workflow state

Fix behavior:
- --fix flag enables auto-fix
- Check existing labels via GitHub API
- Create missing labels with canonical colors
- Idempotent: create-if-missing semantics
- Report created labels in output

WHY
Labels required for Rodgers workflow. Auto-fix via API saves human time. Idempotent = safe to re-run.

HOW TO VERIFY
- Unit test: Creates missing labels via API
- Unit test: Canonical colors applied
- Unit test: Idempotent - re-run doesn't duplicate
- Unit test: Reports created labels
- Integration test: rogers init --fix on label-less repo
- Manual: Remove labels, run init --fix, verify created

EDGE CASES AND PITFALLS
- GitHub API rate limit - retry with backoff
- Permission denied - report, continue other fixes
- Label exists with different color - update or skip?
- Custom labels in rogers.yaml - respect those
- Only fixes API-level changes (labels, discussions)
- File-based fixes (templates) via PR, not auto

PROJECT-SPECIFIC TERMINOLOGY
- 'Required labels': 10 labels Rodgers needs for workflow
- 'Canonical colors': Defined color scheme per label
- 'Idempotent': create-if-missing, safe to re-run
- '--fix': Flag enabling automated fixes
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e48cb-2d46-79cb-a57d-d9cd2eaa4140
author: oompah
created: 2026-05-21T04:28:55Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48cb-7773-7b8d-b8b7-6029f90a46cd
author: oompah
created: 2026-05-21T04:29:14Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48cd-5138-7458-9cb2-d33e91b9e514
author: oompah
created: 2026-05-21T04:31:16Z

## Understanding

**What AC-3 requires:** Implement  to create missing required GitHub labels via the API.

**Current state:** The codebase has stub init code in . The label definitions exist in  (9 required labels with canonical colors). The GitHub client in  already has  and  methods. The  flag exists in the CLI definition.

**What I need to build:**
1.  — Init orchestration: parses repo, runs checks, applies --fix
2.  — Auto-fix logic:  creates missing labels, idempotent
3. Add  to  for testing
4. Wire up  to call the init module
5. Comprehensive tests for create, colors, idempotency, error tolerance
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48d1-99c3-7716-bb28-175593526b22
author: oompah
created: 2026-05-21T04:35:56Z

## Discovery

Found the codebase structure:
-  has 9 RODGERS_REQUIRED_LABELS with canonical colors (d73a4a for bug, a2eeef for feature, etc.)
-  has  and  already implemented
-  has CheckResult/Severity/Fixability/InitCheck trait
-  had a pre-existing build error (dereferencing u16) which I fixed
-  has stub init code that just prints messages
-  doesn't exist yet — I'll create it

The fix is a simple create-if-missing loop over RODGERS_REQUIRED_LABELS, comparing against existing labels.
All API infrastructure (GitHubClient, LabelDefinition) already exists.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48df-29d9-729d-96b3-e367ed368799
author: oompah
created: 2026-05-21T04:50:45Z

## Implementation

Created  with  function that:
- Fetches existing labels via 
- Compares against  (9 labels from )
- Creates missing labels with canonical colors via 
- Handles individual failures gracefully (continues on error)
- Is idempotent — re-running produces same result

Also fixed :  and 
were building URLs without  prefix, breaking wiremock tests.

10 unit tests pass covering creation, skipping, partial creation, idempotency,
canonical colors, and label definition structure.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48df-77d2-797e-8f41-3f57ee87aa6d
author: oompah
created: 2026-05-21T04:51:05Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/qwen/qwen3.6-35b-a3b]
- Turns: 89, Tool calls: 105
- Tokens: 8.1M in / 70.7K out [8.2M total]
- Cost: $0.0000
- Exit: normal, Duration: 22m 10s
- Log: rogers-ebo__20260521T042918Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48df-885d-7214-a08e-2371a2debb2d
author: oompah
created: 2026-05-21T04:51:10Z

Agent completed successfully in 1330s (8175282 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
