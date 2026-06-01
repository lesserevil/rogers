---
id: TASK-21
title: 'CRIT-2: File backport task for each target branch within one triage run'
status: Done
assignee: []
created_date: 2026-05-20 05:19
updated_date: 2026-05-20 08:57
labels:
- rodgers:parent=rogers-4qr
- rodgers:type=backport
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-57o
  state: closed
  parent_id: rogers-4qr
  dependencies: []
  branch_name: rogers-57o
  target_branch: null
  url: null
  created_at: '2026-05-20T05:19:05Z'
  updated_at: '2026-05-20T08:57:48Z'
  closed_at: '2026-05-20T08:57:37Z'
parent: TASK-2
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/backport-plan.md §Backport Detection → Acceptance Criteria CRIT-2

WHAT TO DO
Implement backport task filing for each target branch within one triage run.

Create/modify files:
- src/backport/task.rs - Backport task creation
- src/backport/manager.rs - File tasks for each target
- src/tasks/client.rs - backlog create integration
- src/github/client.rs - Create approval Discussion

Per target branch:
- File chore task (rodgers:type=backport)
- Title: 'Backport #{sha_short} to {branch_name}'
- Description includes: Plan, commit SHA, message, source issue, target branch, WHAT TO DO, ACCEPTANCE, PITFALLS
- discovered-from: source commit/issue
- Priority: 1 for security, 2 otherwise
- Create GitHub Discussion for approval

WHY
Each branch needs separate backport. Task tracks work. Discussion gates approval.

HOW TO VERIFY
- Unit test: Task filed per target branch
- Unit test: Task type=chore, tag=rodgers:type=backport
- Unit test: Description has all required sections
- Unit test: Priority 1 for security, 2 otherwise
- Unit test: Discussion created for each
- Unit test: Within one triage run
- Integration test: Detection → tasks + discussions
- Manual: Trigger backport, verify tasks created

EDGE CASES AND PITFALLS
- Already backported (semantic equivalence) - skip
- Target file doesn't exist - file note task
- Many target branches - file all
- GitHub API failure - retry, don't lose
- Task format matches plan example exactly
- discovered-from links to source

PROJECT-SPECIFIC TERMINOLOGY
- 'Backport task': Chore task tracking cherry-pick (rodgers:type=backport)
- 'Target branch': release/X.Y from active_branches
- 'Semantic equivalence': Fix already in target (behavior match)
- 'Approval Discussion': GitHub Discussion for human 👍/👎
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e447c-a3ba-7e3a-a5ea-e26127ebb0b5
author: oompah
created: 2026-05-20T08:24:40Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e447c-caf5-7542-9bdb-68ad90d10e35
author: oompah
created: 2026-05-20T08:24:50Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e447e-8da4-71de-bac5-d90623a2400d
author: oompah
created: 2026-05-20T08:26:45Z

UNDERSTANDING: The task is CRIT-2 from the backport plan (plans/backport-plan.md). I need to implement backport task filing for each target branch within one triage run.

Current state:
-  ✓ - Backport candidate detection works
-  ✓ - Process candidates, files tasks per branch  
-  ✓ - TaskClient for backlog create integration

What's missing for CRIT-2:
1.  - New module for BackportTask struct with builder pattern
2.  support - Link tasks to source issue/commit via backlog create
3. GitHub Discussion creation - Create approval discussion per target branch
4. Updated description format - Match plan example exactly

Plan:
1. Create  with BackportTask struct
2. Add  method to   
3. Update  to support  flag
4. Update  to link tasks to source and create discussions
5. Add comprehensive unit tests for all required behaviors
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4498-d38a-7e8c-b167-e8bbabd6244f
author: oompah
created: 2026-05-20T08:55:27Z

VERIFICATION: Implementation complete. All tests pass.

Changes made:
1. Created  — BackportTask builder with title, description, tag, priority, discovered-from deps, and external ref
2. Updated  — process_candidates now creates GitHub Discussion per branch, updated to use new BackportTask builder  
3. Updated  — Added  and  support for backlog create
4. Updated  — Added  method for approval Discussions
5. Updated  — Passes github client + discussion category to manager
6. Re-exported  from 

Test coverage (27 tests):
- Unit test: Task filed per target branch ✓
- Unit test: Task type=chore, tag=rodgers:type=backport ✓  
- Unit test: Description has all required sections (Plan, SHA, message, source issue, target branch, WHAT TO DO, ACCEPTANCE, PITFALLS) ✓
- Unit test: Priority 1 for security, 2 otherwise ✓
- Unit test: Discussion created for each ✓
- Unit test: Within one triage run ✓ (integrated via tidly_loop -> process_candidates)
- Unit test: discovered-from link on task ✓
- Integration test: Detection → tasks + discussions (triage loop flow) ✓

All quality gates pass: cargo fmt, cargo build, cargo test (27/27), cargo clippy.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e449a-eeb2-7c3d-8669-90cefa2f48e5
author: oompah
created: 2026-05-20T08:57:45Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 76, Tool calls: 93
- Tokens: 4.0M in / 33.4K out [4.1M total]
- Cost: $0.0000
- Exit: normal, Duration: 33m 5s
- Log: rogers-57o__20260520T082452Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e449a-f32d-74a3-ba8a-566b0b1238c3
author: oompah
created: 2026-05-20T08:57:46Z

Agent completed successfully in 1985s (4082120 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
