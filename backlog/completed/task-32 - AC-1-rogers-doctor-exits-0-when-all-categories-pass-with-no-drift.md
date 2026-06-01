---
id: TASK-32
title: 'AC-1: rogers doctor exits 0 when all categories pass with no drift'
status: Done
assignee: []
created_date: 2026-05-20 05:20
updated_date: 2026-05-20 07:41
labels:
- rodgers:parent=rogers-atj
- rodgers:type=doctor
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-m4q
  state: closed
  parent_id: rogers-atj
  dependencies: []
  branch_name: rogers-m4q
  target_branch: null
  url: null
  created_at: '2026-05-20T05:20:38Z'
  updated_at: '2026-05-20T07:41:39Z'
  closed_at: '2026-05-20T07:41:34Z'
parent: TASK-3
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/doctor-plan.md §Output Format / Exit Codes → Acceptance Criteria AC-1

WHAT TO DO
Implement rogers doctor exit 0 when all categories pass with no drift.

Create/modify files:
- src/doctor/mod.rs - Main doctor command, exit logic
- src/doctor/categories.rs - Category implementations
- src/doctor/drift.rs - Drift detection
- src/doctor/report.rs - Report generation

Categories (all must pass):
- config: config.yaml valid, required keys
- auth: GitHub token valid, scopes, repo access
- tasks: Database reachable, schema correct
- plans: Plan files exist, valid frontmatter
- repo: Labels, discussion category, release branches
- drift: No drift events detected

Exit 0 only if ALL categories OK AND no drift.

WHY
Exit 0 = healthy. Scripts/CI can check doctor health. All categories + drift = comprehensive.

HOW TO VERIFY
- Unit test: All categories pass, no drift → exit 0
- Unit test: Any category fail → exit 1
- Unit test: Drift detected → exit 1
- Integration test: Healthy setup → doctor exits 0
- Manual: Configure healthy, run doctor, verify exit 0

EDGE CASES AND PITFALLS
- Categories run in order, fail fast on config/auth (AC-4)
- Drift detection runs last (most expensive)
- Verbose doesn't affect exit code
- --fix doesn't affect exit code (interactive)
- Each category independent but all required

PROJECT-SPECIFIC TERMINOLOGY
- 'Category': Health check group (config, auth, tasks, plans, repo, drift)
- 'Drift': GitHub ↔ tasks state divergence
- 'Exit 0': All healthy
- 'rogers doctor': Health check command
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e4436-1f53-79b2-96cc-8d64fa8fa859
author: oompah
created: 2026-05-20T07:07:38Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4436-39dc-7638-a7fe-7fe462a308fc
author: oompah
created: 2026-05-20T07:07:45Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4437-5a95-7ddb-8138-ef556855ef78
author: oompah
created: 2026-05-20T07:08:59Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/nvidia/nemotron-3-ultra-preview]
- Turns: 0, Tool calls: 10
- Tokens: 0 in / 0 out [0 total]
- Cost: $0.0000
- Exit: terminated, Duration: 1m 12s
- Log: rogers-m4q__20260520T070747Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4446-13b2-7a06-a6fa-d2b2e359e932
author: oompah
created: 2026-05-20T07:25:04Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4446-1a41-7b0e-aac3-946d5565dac5
author: oompah
created: 2026-05-20T07:25:05Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4447-b013-7444-b829-ae4d0cf605be
author: oompah
created: 2026-05-20T07:26:49Z

**Understanding:** Implementing rogers doctor exit 0 when all categories pass with no drift.

**Key Requirements:**
- Exit 0 = all 6 categories pass + no drift
- Exit 1 = any category fails OR drift detected
- Categories run in order: config, auth, tasks, plans, repo, drift
- Fail fast on config/auth problems
- Verbose/--fix flags don't affect exit code

**Implementation Plan:**
1. Create src/doctor/ directory with modular structure
2. Implement config category (validate config.yaml, required keys)
3. Implement auth category (GitHub token validation)
4. Implement tasks category (database connectivity)  
5. Implement plans category (plan files validation)
6. Implement repo category (labels, branches)
7. Implement drift category (GitHub ↔ tasks state divergence)
8. Implement report.rs for output formatting
9. Wire into main.rs to handle exit codes properly
10. Add unit tests for exit code scenarios

**Architecture:**
- Each category returns a status (Pass, Warn, Fail) with details
- Doctor aggregates all results and exits 0 only if all Pass and drift=0
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4453-9ca6-7da6-bd48-98e1ff06681c
author: oompah
created: 2026-05-20T07:39:51Z

**Discovery**: Found the doctor module doesn't exist yet - need to create it from scratch. Key files identified:
- src/cli.rs - CLI command definitions (already has Doctor command)
- src/error.rs - Error types with exit_code method
- config.example.yaml - Config schema reference

**Implementation**: Created new doctor module with:
- src/doctor/mod.rs - Core types (CategoryResult, DoctorResult, exit code logic)
- src/doctor/categories.rs - Category implementations (config, auth, tasks, plans, repo)
- src/doctor/drift.rs - Drift detection
- src/doctor/report.rs - Report generation (text/JSON output)

**Verification**: All 20 unit tests pass:
- test_all_pass_no_drift_exits_0 ✓
- test_category_fail_exits_1 ✓
- test_drift_detected_exits_1 ✓
- test_warnings_still_exits_0 ✓
- test_skipped_categories_ignored ✓
- And more...

**Exit Code Logic**:
- Exit 0: All categories pass and no drift detected
- Exit 1: Any category fails OR drift detected
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4455-3ad3-76d6-bf5e-3aa8e78e6210
author: oompah
created: 2026-05-20T07:41:37Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 56, Tool calls: 64
- Tokens: 2.7M in / 20.7K out [2.8M total]
- Cost: $0.0000
- Exit: normal, Duration: 16m 34s
- Log: rogers-m4q__20260520T072509Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4455-3b1e-79f9-8a59-cb228339e08d
author: oompah
created: 2026-05-20T07:41:37Z

Agent completed successfully in 994s (2758336 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
