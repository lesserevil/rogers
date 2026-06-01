---
id: TASK-34
title: 'AC-3: rogers doctor --verbose lists every drift event with details'
status: Done
assignee: []
created_date: 2026-05-20 05:20
updated_date: 2026-05-20 08:15
labels:
- rodgers:parent=rogers-atj
- rodgers:type=doctor
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-l6q
  state: closed
  parent_id: rogers-atj
  dependencies: []
  branch_name: rogers-l6q
  target_branch: null
  url: null
  created_at: '2026-05-20T05:20:55Z'
  updated_at: '2026-05-20T08:15:23Z'
  closed_at: '2026-05-20T08:15:16Z'
parent: TASK-3
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/doctor-plan.md §State Drift Detection / Output Format → Acceptance Criteria AC-3

WHAT TO DO
Implement rogers doctor --verbose listing every drift event with GitHub issue URL, task ID, specific mismatch.

Create/modify files:
- src/doctor/drift.rs - Drift detection with full details
- src/doctor/report.rs - Verbose drift formatting
- src/doctor/mod.rs - --verbose flag handling
- src/tasks/client.rs - Fetch task details
- src/github/client.rs - Fetch issue details

Drift events to detect (plan):
1. Closed tasks with open GitHub issues
2. In-progress tasks with closed GitHub issues  
3. Orphan tasks (no GitHub issue link)
4. Issues labeled ready-for-work with no linked task
5. Release-proposed issues not in milestone
6. Backlog violating project AGENTS.md conventions

Verbose output per event:
- GitHub issue URL
- Task ID
- Specific mismatch
- Linking info

WHY
Verbose drift details enable precise remediation. Each event needs specific fix.

HOW TO VERIFY
- Unit test: Each drift type detected with details
- Unit test: --verbose shows issue URL, task ID, mismatch
- Unit test: Non-verbose shows summary only
- Integration test: Create drift scenarios, run doctor --verbose
- Manual: Create drift, run doctor --verbose, verify details

EDGE CASES AND PITFALLS
- GitHub API failure fetching issue - note in event
- Task fetch failure - note in event
- Many drift events - list all
- Orphan tasks: some intentional (internal) - flag for review
- Convention drift: compare task to project AGENTS.md

PROJECT-SPECIFIC TERMINOLOGY
- 'Drift event': GitHub ↔ tasks state mismatch
- '--verbose': Flag for detailed drift output
- 'GitHub issue URL': Full URL to issue
- 'Task ID': backlog identifier (e.g., rogers-xxx)
- 'Mismatch type': Closed task/open issue, etc.
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e4463-bacd-731f-b319-246131151d67
author: oompah
created: 2026-05-20T07:57:27Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4463-dd8f-733e-8f65-9c8cae74e5ae
author: oompah
created: 2026-05-20T07:57:36Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4466-a7fd-74f0-bbdc-272a3b08df4f
author: oompah
created: 2026-05-20T08:00:39Z

## Understanding

Working on AC-3:  should list every drift event with full details.

**Current state:**
-  struct exists with: , , , , 
-  in drift.rs is a placeholder - doesn't fetch real data
-  shows drift summary but verbose mode doesn't show individual events with full details
- Only two drift types are implemented:  and 

**What needs implementation:**
1. **Real drift detection** - fetch tasks from database and GitHub issues
2. **Additional drift types**:
   - Orphan tasks (no GitHub issue link)
   - Issues labeled 'ready-for-work' with no linked task
   - Release-proposed issues not in milestone
   - AGENTS.md convention violations
3. **Verbose formatting** - list each drift event with: GitHub issue URL, task ID, specific mismatch

**Plan:**
- Update  to implement real drift detection with all 6 types
- Enhance  verbose output formatting for drift events
- Add tests for each drift type detection
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4472-286a-7e77-850a-cedbb89fb52c
author: oompah
created: 2026-05-20T08:13:13Z

## Verification

All 40 tests pass. Implemented:

**Changes:**
1. **drift.rs** - Expanded drift detection with all 6 drift types:
   -  - Closed tasks with open GitHub issues (Error severity)
   -  - In-progress tasks with closed GitHub issues (Warning)
   -  - Backlog without GitHub issue links (Warning)
   -  - Issues with 'ready-for-work' label but no linked task (Warning)
   -  - Release-proposed issues not in milestone (Warning)
   -  - Backlog violating AGENTS.md conventions (Warning)

2. **report.rs** - Verbose drift event formatting with:
   - GitHub issue URL
   - Task ID
   - Specific mismatch description
   - Linking remediation info ('Link:' field)

3. New tests cover all drift types in verbose mode with proper URL/task ID display
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4474-20df-7610-8469-c5a6e17fb6f3
author: oompah
created: 2026-05-20T08:15:22Z

Agent completed successfully in 1074s (2522015 tokens)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4474-2bfe-7bd1-bde4-25b4877ae1f2
author: oompah
created: 2026-05-20T08:15:25Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 48, Tool calls: 56
- Tokens: 2.5M in / 19.6K out [2.5M total]
- Cost: $0.0000
- Exit: normal, Duration: 17m 54s
- Log: rogers-l6q__20260520T075737Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
