---
id: TASK-47
title: 'AC-2: rogers init exits 1 on any blocker failure, lists all blockers'
status: To Do
assignee: []
created_date: 2026-05-20 05:23
updated_date: 2026-05-21 03:58
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
  id: rogers-ar4
  state: open
  parent_id: rogers-zql
  dependencies: []
  branch_name: rogers-ar4
  target_branch: null
  url: null
  created_at: '2026-05-20T05:23:02Z'
  updated_at: '2026-05-21T03:58:18Z'
  closed_at: null
parent: TASK-5
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/init-plan.md §Output Format / Exit Codes → Acceptance Criteria AC-2

WHAT TO DO
Implement rogers init exit 1 on any blocker failure, listing all blockers.

Create/modify files:
- src/init/mod.rs - Exit code logic, blocker collection
- src/init/checks.rs - Blocker identification
- src/init/report.rs - Blocker listing in report

Behavior:
- Run all audit checks
- Collect ALL blockers (don't stop at first)
- If any blocker: exit 1
- Report lists ALL blockers with severity, description, fixability
- Format: '[BLOCKER] Description - fixability'

WHY
Exit 1 signals failure. Listing ALL blockers lets user fix multiple at once. Fixability helps prioritize.

HOW TO VERIFY
- Unit test: Single blocker → exit 1, listed
- Unit test: Multiple blockers → exit 1, ALL listed
- Unit test: Blocker report shows severity, description, fixability
- Integration test: rogers init on unready repo → exit 1, blockers listed
- Manual: Fresh repo, run init, verify exit 1 and blocker list

EDGE CASES AND PITFALLS
- Must collect ALL blockers before exit
- Blocker vs warning vs info distinction clear
- Fixability: auto/manual/na shown
- Auth failure = exit 3 (not blocker)
- Invalid args = exit 2
- Report format matches plan example

PROJECT-SPECIFIC TERMINOLOGY
- 'Blocker': Severity=blocker check failure (prevents Rodgers operation)
- 'Fixability': auto (API), manual (human), na (info only)
- 'Exit code 1': Blocker failure
- 'Blocker report': Structured list of all blockers
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e48ae-209c-755e-be51-b8308930b261
author: oompah
created: 2026-05-21T03:57:12Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48ae-656c-7687-ba1a-1ca978eb2406
author: oompah
created: 2026-05-21T03:57:29Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48af-0f0e-7bdf-b725-ba19b9784803
author: oompah
created: 2026-05-21T03:58:13Z

🤚 **Question from agent:**

This issue has dependencies on other init tasks (AC-1 through AC-7). Should I implement ALL init checks (issue templates, required labels, repo settings, release workflow, agent instructions, config) or just the core infrastructure (exit codes, blocker collection, report formatting) that makes AC-1, AC-2, and AC-6 work? Other ACs mention --fix flag which depends on other tasks.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48af-1e68-7d07-abe4-2c68b740a4df
author: oompah
created: 2026-05-21T03:58:17Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/qwen/qwen3.6-35b-a3b]
- Turns: 5, Tool calls: 11
- Tokens: 65.4K in / 1.3K out [66.7K total]
- Cost: $0.0000
- Exit: ask_question, Duration: 1m 5s
- Log: rogers-ar4__20260521T035733Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
