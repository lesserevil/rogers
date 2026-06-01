---
id: TASK-24
title: 'CRIT-5: File conflict-resolution task on merge conflicts, no autonomous resolution'
status: Done
assignee: []
created_date: 2026-05-20 05:19
updated_date: 2026-05-20 09:55
labels:
- rodgers:parent=rogers-4qr
- rodgers:type=backport
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-1yn
  state: closed
  parent_id: rogers-4qr
  dependencies: []
  branch_name: rogers-1yn
  target_branch: null
  url: null
  created_at: '2026-05-20T05:19:34Z'
  updated_at: '2026-05-20T09:55:00Z'
  closed_at: '2026-05-20T09:54:50Z'
parent: TASK-2
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/backport-plan.md §Conflict Handling → Acceptance Criteria CRIT-5

WHAT TO DO
Implement conflict-resolution task filing on merge conflicts, no autonomous resolution.

Create/modify files:
- src/backport/conflicts.rs - Conflict detection and task filing
- src/backport/execution.rs - Detect conflicts during PR creation
- src/backport/manager.rs - Handle conflict case
- src/tasks/client.rs - File conflict-resolution task

Conflict handling:
- Detect merge conflicts during cherry-pick/PR creation
- File chore task (rodgers:type=backport-conflict)
- Task notes: target branch, source commit, conflict resolution needed
- Post comment on source issue: 'Backport needs conflict resolution. Task filed.'
- Close approval Discussion
- NO autonomous conflict resolution attempt

WHY
Conflicts need human judgment. Autonomous resolution risks breaking things. Task tracks resolution work.

HOW TO VERIFY
- Unit test: Conflict detected → conflict-resolution task
- Unit test: Task type=chore, tag=rodgers:type=backport-conflict
- Unit test: Task has target branch, source commit
- Unit test: Source issue comment posted
- Unit test: Approval discussion closed
- Unit test: No cherry-pick attempt
- Integration test: Conflicting backport, verify task
- Manual: Create conflict scenario, verify

EDGE CASES AND PITFALLS
- Conflict detection via GitHub PR API or git
- Multiple conflicts - one task covers all
- Shared library divergence - common cause
- Task describes resolution needed
- Human resolves, pushes to backport branch
- PR updates, CI runs, merge

PROJECT-SPECIFIC TERMINOLOGY
- 'Conflict-resolution task': Chore task (rodgers:type=backport-conflict)
- 'Merge conflicts': Cherry-pick doesn't apply cleanly
- 'No autonomous resolution': Human must resolve
- 'backport-conflict': rodgers:type for conflict tasks
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e44be-5eaf-78a0-ad46-3f80366fb5b6
author: oompah
created: 2026-05-20T09:36:27Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44be-85e1-75fc-bbee-a219c27e129c
author: oompah
created: 2026-05-20T09:36:37Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44be-e82b-766e-8d65-4925a25373d0
author: oompah
created: 2026-05-20T09:37:03Z

Understanding: I need to implement conflict-resolution task filing on merge conflicts. The system should detect conflicts during cherry-pick/PR creation, file a chore task (rodgers:type=backport-conflict), post a comment on the source issue, and close the approval discussion. No autonomous resolution should be attempted. Files to create/modify: src/backport/conflicts.rs, src/backport/execution.rs, src/backport/manager.rs, src/tasks/client.rs
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44c0-54d0-7b29-866a-e64824207bbc
author: oompah
created: 2026-05-20T09:38:36Z

Discovery: I've read all relevant source files. The existing architecture has: backport/manager.rs handles the tracking and approval loop, backport/execution.rs creates branches/PRs but doesn't detect conflicts, backport/approval.rs provides close_discussion which I can reuse. I need to create src/backport/conflicts.rs and wire it into execution.rs. GitHub PR creation returns mergeable=false when there are merge conflicts - I can detect conflicts by checking the PR state after creation.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44c4-56bf-7e6b-8b00-728d6ca84cd0
author: oompah
created: 2026-05-20T09:42:58Z

Agent completed successfully in 392s (662841 tokens)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44c4-74a9-7a3e-967b-a25808232c69
author: oompah
created: 2026-05-20T09:43:06Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 21, Tool calls: 28
- Tokens: 651.5K in / 11.4K out [662.8K total]
- Cost: $0.0000
- Exit: normal, Duration: 6m 32s
- Log: rogers-1yn__20260520T093641Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44c4-7d1a-7582-bdf4-2aae59cccb03
author: oompah
created: 2026-05-20T09:43:08Z

Agent completed without closing this issue (392s (662841 tokens)). Escalating from 'default' to 'standard'. Retrying in 10s (1/3).
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44c4-d20e-71c7-9efb-9430a8f64d9d
author: oompah
created: 2026-05-20T09:43:30Z

Agent dispatched (profile: standard)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44c4-df5e-7972-85b2-80ef1cca5a3d
author: oompah
created: 2026-05-20T09:43:33Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44c8-1e4f-7568-942f-ca0298e6f76f
author: oompah
created: 2026-05-20T09:47:06Z

Discovery: I've identified the missing integration. The conflicts.rs module has full conflict detection and handling implementation (handle_conflict, has_merge_conflicts, wait_for_mergeable), but manager.rs never calls it after execute_backport. The check_pending_discussions function processes the approval but doesn't check for merge conflicts on the created PR. I need to add conflict detection after execute_backport returns with a PR number.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44cc-ba33-7ca3-be04-f855033aa2b2
author: oompah
created: 2026-05-20T09:52:08Z

Implementation: I've added conflict detection integration to manager.rs. After execute_backport creates a PR, the code now calls wait_for_mergeable (10 attempts, 2s interval) to poll GitHub until the mergeable field is populated, then has_merge_conflicts checks for conflicts. If conflicts exist, handle_conflict is called to file a conflict-resolution task (rodgers:type=backport-conflict), post a comment on the source issue, and close the approval discussion. No autonomous conflict resolution is attempted. Also fixed a String/Option bug in conflicts.rs where as_deref() was incorrectly called on String.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44cf-433f-7090-a086-9c4fb48eb54e
author: oompah
created: 2026-05-20T09:54:54Z

Run #2 [attempt=2, profile=standard, role=standard -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 35, Tool calls: 45
- Tokens: 1.9M in / 17.0K out [1.9M total]
- Cost: $0.0000
- Exit: normal, Duration: 11m 25s
- Log: rogers-1yn__20260520T094335Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44cf-4631-78ba-974e-93714613a23a
author: oompah
created: 2026-05-20T09:54:55Z

Agent completed successfully in 685s (1928850 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
