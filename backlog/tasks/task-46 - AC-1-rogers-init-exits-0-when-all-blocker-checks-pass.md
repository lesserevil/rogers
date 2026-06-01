---
id: TASK-46
title: 'AC-1: rogers init exits 0 when all blocker checks pass'
status: To Do
assignee: []
created_date: 2026-05-20 05:22
updated_date: 2026-05-21 03:14
labels:
- asking_question
- rodgers:parent=rogers-zql
- rodgers:type=init
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-nuw
  state: open
  parent_id: rogers-zql
  dependencies: []
  branch_name: rogers-nuw
  target_branch: null
  url: null
  created_at: '2026-05-20T05:22:50Z'
  updated_at: '2026-05-21T03:14:45Z'
  closed_at: null
parent: TASK-5
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/init-plan.md §Output Format / Exit Codes → Acceptance Criteria AC-1

WHAT TO DO
Implement rogers init exit code 0 when all blocker checks pass.

Create/modify files:
- src/init/mod.rs - Main init command, exit code logic
- src/init/checks.rs - Blocker check implementations
- src/init/report.rs - Structured report generation

Behavior:
- Run all audit checks (templates, labels, repo settings, release workflow, discussions, branch protection, agent instructions, rogers.yaml)
- Count blockers (severity=blocker)
- If blockers == 0: exit 0, report 'All checks passed'
- If blockers > 0: exit 1 (handled by rogers-ar4)

WHY
Exit code 0 signals success to scripts/CI. Clean exit when project ready for Rodgers.

HOW TO VERIFY
- Unit test: All checks pass → exit 0
- Unit test: Exit code checked via 0 
- Integration test: rogers init on ready repo → exit 0
- Manual: Configure ready repo, run init, verify exit 0

EDGE CASES AND PITFALLS
- Warnings don't affect exit code (only blockers)
- Info messages don't affect exit code
- Auth failure = exit 3 (different code)
- Invalid args = exit 2
- Must run all checks before exit decision

PROJECT-SPECIFIC TERMINOLOGY
- 'Blocker check': Severity=blocker audit (templates, labels, release workflow, main branch protection)
- 'Exit code 0': Success - project ready for Rodgers
- 'rogers init': Project readiness audit command
- 'All checks passed': Report message on success
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e4884-58dd-7bab-8290-64686bed3747
author: oompah
created: 2026-05-21T03:11:34Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4885-4d9b-736a-980f-af133f65c737
author: oompah
created: 2026-05-21T03:12:36Z

Focus: Duplicate Investigator
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4886-d1af-7850-aa00-1f85ac94ef27
author: oompah
created: 2026-05-21T03:14:16Z

🤚 **Question from agent:**

The issue is about rogers init AC-1 (exit 0 when all blockers pass), and rogers-ar4 is AC-2 (exit 1 on blocker). They are distinct acceptance criteria - AC-1 is the success path, AC-2 is the failure path. The current codebase has main.rs as a stub ('Hello, world!') and no src/init directory. Should I implement the full init command (mod.rs, checks.rs, report.rs) for AC-1, or should I wait for the infrastructure from rogers-zql.1 to be completed first?
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4886-d72b-7abc-85eb-0d26bb5514d0
author: oompah
created: 2026-05-21T03:14:17Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/qwen/qwen3.6-35b-a3b]
- Turns: 8, Tool calls: 19
- Tokens: 187.8K in / 2.3K out [190.1K total]
- Cost: $0.0000
- Exit: ask_question, Duration: 2m 45s
- Log: rogers-nuw__20260521T031249Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
