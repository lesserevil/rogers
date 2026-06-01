---
id: TASK-33
title: 'AC-2: rogers doctor exits 1 on any category failure or drift detected'
status: Done
assignee: []
created_date: 2026-05-20 05:20
updated_date: 2026-05-20 07:56
labels:
- rodgers:parent=rogers-atj
- rodgers:type=doctor
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-swc
  state: closed
  parent_id: rogers-atj
  dependencies: []
  branch_name: rogers-swc
  target_branch: null
  url: null
  created_at: '2026-05-20T05:20:49Z'
  updated_at: '2026-05-20T07:56:45Z'
  closed_at: '2026-05-20T07:56:31Z'
parent: TASK-3
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/doctor-plan.md §Output Format / Exit Codes → Acceptance Criteria AC-2

WHAT TO DO
Implement rogers doctor exit 1 on any category failure OR drift detected, listing all failures.

Create/modify files:
- src/doctor/mod.rs - Exit logic, failure collection
- src/doctor/categories.rs - Category failure reporting
- src/doctor/drift.rs - Drift event collection
- src/doctor/report.rs - Failure listing

Behavior:
- Run all categories (config, auth, tasks, plans, repo, drift)
- Collect ALL failures (don't stop at first)
- Collect ALL drift events
- If any failure OR drift: exit 1
- Report lists all failures + drift events
- Format matches plan output example

WHY
Exit 1 = unhealthy. All failures listed = fix multiple at once. Drift = exit 1 (critical).

HOW TO VERIFY
- Unit test: Config fail → exit 1, listed
- Unit test: Auth fail → exit 1, listed
- Unit test: Drift detected → exit 1, events listed
- Unit test: Multiple failures → exit 1, ALL listed
- Integration test: Unhealthy setup → doctor exits 1, failures shown
- Manual: Break config, run doctor, verify exit 1

EDGE CASES AND PITFALLS
- Fail fast on config/auth (AC-4) but still check others for full report
- Drift events detailed with --verbose (AC-3)
- Category failures include specific error
- Drift events: issue/task IDs, mismatch type
- Exit codes: 0=healthy, 1=unhealthy, 2=invalid args, 3=auth fail

PROJECT-SPECIFIC TERMINOLOGY
- 'Category failure': Any check in category fails
- 'Drift event': GitHub/tasks state mismatch
- 'Exit 1': Unhealthy (failure or drift)
- 'Fail fast': Config/auth first, but collect all
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e4455-c05e-74c0-9cb9-6ef496521c86
author: oompah
created: 2026-05-20T07:42:11Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4455-e527-77ee-943c-0b6fe9481ea2
author: oompah
created: 2026-05-20T07:42:20Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4456-c866-7bd2-880e-e4bf64bee9ab
author: oompah
created: 2026-05-20T07:43:19Z

UNDERSTANDING: AC-2 requires rogers doctor to exit 1 when any category fails OR drift is detected, listing ALL failures. The current code structure in mod.rs has DoctorResult and exit_code() methods, but the main.rs run_doctor_checks() function has early returns after config/auth failures that prevent gathering all failures. Need to: 1) Remove early returns in run_doctor_checks to collect ALL failures, 2) Ensure drift events are collected properly, 3) Update report.rs to list all failures + drift events, 4) Verify exit code 1 when failures or drift detected.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4460-63f2-7da2-a5c2-3a59f60f2cc5
author: oompah
created: 2026-05-20T07:53:48Z

IMPLEMENTATION: Modified src/main.rs run_doctor_checks() to remove early returns after config/auth failures. Now collects ALL failures from all categories. Added DriftCheckResult struct in src/doctor/drift.rs to return both category result and drift events. Updated main.rs to extend drift_events from drift check. Added 5 new unit tests in src/doctor/mod.rs covering AC-2 scenarios: config fail, auth fail, drift detected, multiple failures, config+auth+drift combined.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4463-0994-7a5b-9325-46ba98d9a409
author: oompah
created: 2026-05-20T07:56:42Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 30, Tool calls: 39
- Tokens: 1.1M in / 12.0K out [1.1M total]
- Cost: $0.0000
- Exit: normal, Duration: 14m 31s
- Log: rogers-swc__20260520T074222Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4463-1767-762c-9af4-0cdebc42bb7b
author: oompah
created: 2026-05-20T07:56:45Z

Agent completed successfully in 871s (1109862 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
