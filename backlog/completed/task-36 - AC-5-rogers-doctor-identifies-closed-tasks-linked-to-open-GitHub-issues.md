---
id: TASK-36
title: 'AC-5: rogers doctor identifies closed tasks linked to open GitHub issues'
status: Done
assignee: []
created_date: 2026-05-20 05:21
updated_date: 2026-05-20 10:18
labels:
- rodgers:parent=rogers-atj
- rodgers:type=doctor
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-7xt
  state: closed
  parent_id: rogers-atj
  dependencies: []
  branch_name: rogers-7xt
  target_branch: null
  url: null
  created_at: '2026-05-20T05:21:11Z'
  updated_at: '2026-05-20T10:18:27Z'
  closed_at: '2026-05-20T10:18:13Z'
parent: TASK-3
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/doctor-plan.md §State Drift Detection → Acceptance Criteria AC-5

WHAT TO DO
Implement drift detection: closed tasks linked to open GitHub issues.

Create/modify files:
- src/doctor/drift.rs - Closed task / open issue detection
- src/doctor/mod.rs - Drift category execution
- src/tasks/client.rs - Fetch closed tasks
- src/github/client.rs - Fetch issue state

Detection:
- For each task with status=closed
- Fetch linked GitHub issue (via github_issue_url or discovered-from)
- If issue state=open → drift event
- Event details: issue URL, task ID, mismatch

WHY
Closed task = work done. Open issue = work not done (human view). Mismatch = confusion.

HOW TO VERIFY
- Unit test: Closed task + open issue → drift detected
- Unit test: Closed task + closed issue → no drift
- Unit test: Open task + open issue → no drift
- Unit test: Drift event has issue URL, task ID
- Integration test: Close task, leave issue open, run doctor
- Manual: Create drift, verify detection

EDGE CASES AND PITFALLS
- Task link via github_issue_url field
- Task link via discovered-from dependency
- GitHub API failure - retry, mark event uncertain
- Issue deleted (404) - treat as closed?
- Many tasks - paginate
- Remediation options: close issue, reopen task, file new task

PROJECT-SPECIFIC TERMINOLOGY
- 'Drift detection': GitHub/tasks mismatch detection
- 'Closed task': status=closed
- 'Open GitHub issue': state=open
- 'Drift event': Specific mismatch with details
- 'Remediation': Fix options (A/B/C in plan)
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e448e-7216-70f4-8bdc-23867dd8e2a8
author: oompah
created: 2026-05-20T08:44:07Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e448e-8f98-74a4-9796-6168cbccde6f
author: oompah
created: 2026-05-20T08:44:14Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4491-6cd9-76a7-a6d4-84c78cd1fa95
author: oompah
created: 2026-05-20T08:47:22Z

## Understanding

I need to implement drift detection: closed tasks linked to open GitHub issues. After exploring the codebase:

- **** - Contains , ,  types already defined
- **** - Has placeholder drift detection that needs to be fully implemented
- **** - Has other health check categories implemented using reqwest
- **** - Wires up doctor checks with fail-fast logic
- **** - Error types including GitHub errors

## Implementation Plan

1. **Create ** - GitHub client to fetch issue state from GitHub API
   -  - Get single issue state
   - Handle 404 by treating as closed
   - Retry on transient failures

2. **Create ** - Backlog client to interact with the database (Backlog.md)
   -  - Fetch tasks with status=closed
   - Extract  or discover from dependencies
   - Handle pagination for many tasks

3. **Update ** - Implement actual drift detection
   - Fetch all closed tasks
   - For each task with a linked GitHub issue, fetch issue state
   - Compare: closed task + open issue → drift event

## Tests Required
- Unit test: Closed task + open issue → drift detected
- Unit test: Closed task + closed issue → no drift  
- Unit test: Open task + open issue → no drift
- Unit test: Drift event has issue URL, task ID
- Unit test: Missing issue (404) → treat as closed, no drift
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4491-9fad-77fb-8a23-64892efcb815
author: oompah
created: 2026-05-20T08:47:35Z

**Understanding complete.** Explored codebase and identified:

-  has placeholder logic needing full implementation
- Need to create GitHub client () for fetching issue state
- Need to create tasks client () for fetching closed tasks
- Will implement: closed task detection + GitHub issue state check + drift event creation

Starting implementation now.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4498-355e-713c-adcc-637428e7a916
author: oompah
created: 2026-05-20T08:54:46Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 0, Tool calls: 24
- Tokens: 0 in / 0 out [0 total]
- Cost: $0.0000
- Exit: terminated, Duration: 10m 37s
- Log: rogers-7xt__20260520T084417Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44da-4d2f-7a65-87fa-a50da18c6368
author: oompah
created: 2026-05-20T10:06:58Z

Retrying (attempt #18, agent: standard)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44da-53be-7557-9cb5-53b5064efc4d
author: oompah
created: 2026-05-20T10:07:00Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44dc-6c0a-7be8-8b60-089fc9a067f4
author: oompah
created: 2026-05-20T10:09:17Z

Understanding complete. Analyzing the current state:

**Current State:**
- : Has  that is a placeholder returning empty vectors for drift detection
- : Has  function already implemented with proper logic - this is what generates actual drift events
- : Has  returning  - just a placeholder
- : Has  with  already implemented
- : Calls  but doesn't pass tasks config

**What AC-5 needs:**
-  needs to actually fetch closed tasks from the Backlog.md task store
- For each closed task with , fetch the issue state from GitHub
- Compare and generate drift events using the existing  function
- Add unit tests verifying: closed task + open issue → drift detected

**Implementation Plan:**
1. Update  signature to accept tasks config (path)
2. Create  and use it to fetch closed tasks  
3. For each closed task with gh-issue URL, call 
4. Build the data structures and call 
5. Add comprehensive unit tests

Starting implementation now.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44e3-3f3e-7215-8466-d37649f27307
author: oompah
created: 2026-05-20T10:16:44Z

Discovery: Found  function already implemented with proper drift detection logic. The  function was just a placeholder returning empty vectors. 

Implementation complete:
1. Updated  signature to accept tasks config (path)
2. Implemented actual drift detection logic using GitHubClient and BacklogClient
3. For each closed task with github_issue_url, fetches issue state from GitHub API
4. Compares states and generates drift events via 
5. Added unit tests covering all AC-5 acceptance criteria

Tests pass: 56 passed (including new AC-5 tests).
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44e4-cad8-7c37-9b39-5c24858c6d41
author: oompah
created: 2026-05-20T10:18:25Z

Agent completed successfully in 688s (2279483 tokens)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44e4-d89c-78ce-a618-a51a7621720e
author: oompah
created: 2026-05-20T10:18:29Z

Run #19 [attempt=19, profile=standard, role=standard -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 42, Tool calls: 51
- Tokens: 2.3M in / 20.0K out [2.3M total]
- Cost: $0.0000
- Exit: normal, Duration: 11m 28s
- Log: rogers-7xt__20260520T100701Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
