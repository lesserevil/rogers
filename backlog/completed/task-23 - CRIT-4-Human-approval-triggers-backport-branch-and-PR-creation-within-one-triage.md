---
id: TASK-23
title: 'CRIT-4: Human approval triggers backport branch and PR creation within one
  triage run'
status: Done
assignee: []
created_date: 2026-05-20 05:19
updated_date: 2026-05-20 09:36
labels:
- rodgers:parent=rogers-4qr
- rodgers:type=backport
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-6vf
  state: closed
  parent_id: rogers-4qr
  dependencies: []
  branch_name: rogers-6vf
  target_branch: null
  url: null
  created_at: '2026-05-20T05:19:25Z'
  updated_at: '2026-05-20T09:36:15Z'
  closed_at: '2026-05-20T09:35:48Z'
parent: TASK-2
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/backport-plan.md §Backport Execution → Acceptance Criteria CRIT-4

WHAT TO DO
Implement human approval triggering backport branch and PR creation within one triage run.

Create/modify files:
- src/backport/approval.rs - Detect approval, trigger execution
- src/backport/execution.rs - Branch and PR creation
- src/backport/manager.rs - Orchestrate on approval
- src/github/client.rs - git_branch, create PR
- src/git/client.rs - Branch operations

On approval (👍 reaction or approval comment):
- Create branch backport/{sha_short}/{branch_name} from target release branch
- File chore task (rodgers:type=backport) describing cherry-pick work
- Create PR targeting release/{X.Y} with cherry-pick
- Post comment on source issue linking backport
- All within ONE triage run of approval detection

WHY
Approval = go signal. Fast branch/PR creation. Task tracks cherry-pick work (human does cherry-pick).

HOW TO VERIFY
- Unit test: Approval detection triggers branch creation
- Unit test: Branch named backport/{sha_short}/{branch}
- Unit test: PR created targeting release branch
- Unit test: Task filed for cherry-pick work
- Unit test: Source issue comment posted
- Unit test: Within one triage run
- Integration test: Approve discussion, verify branch/PR/task
- Manual: Create approval, run triage, verify

EDGE CASES AND PITFALLS
- Branch exists - handle collision
- PR creation fails - retry, task tracks
- GitHub API rate limit - backoff
- Approval detection: reaction or comment
- Rodgers doesn't cherry-pick - task for human
- Conflict handling separate (CRIT-5)

PROJECT-SPECIFIC TERMINOLOGY
- 'Backport branch': backport/{sha_short}/{branch_name}
- 'Cherry-pick task': Chore task for human to resolve conflicts
- 'Target release branch': release/X.Y from active_branches
- 'One triage run': Fast execution after approval
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e44b0-cf62-77cd-9753-150631ca06d3
author: oompah
created: 2026-05-20T09:21:39Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44b0-e245-75c2-a268-ffba5e4c7aac
author: oompah
created: 2026-05-20T09:21:43Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44b2-bc5d-75bc-90e2-9857f6a2fbe0
author: oompah
created: 2026-05-20T09:23:45Z

**Understanding:** CRIT-4 requires human approval (👍 reaction or comment) to trigger backport execution in a single triage run. Currently approval.rs detects approvals and manager.rs monitors discussions, but execution (branch creation, PR filing) is not yet wired up.

**Plan:**
1. Add , , and  methods to 
2. Create  with functions to:
   - Create  branch from target release head
   - File a chore task for cherry-pick work
   - Create PR targeting release/{X.Y}
   - Post comment on source issue linking the backport
3. Update  to call execution when approval is detected
4. Add unit tests for all new functionality
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44bc-1053-7072-bdcd-8a711fd3f04e
author: oompah
created: 2026-05-20T09:33:56Z

**Implementation:** Created  with backport execution logic. Extended  with , , , and  methods. Updated  to execute backports within the same triage run when approval is detected.

Key changes:
- : New file with  function that creates branch , files cherry-pick task, creates PR targeting release branch, and posts source issue comment
- : Added branch creation, PR creation, and issue comment APIs
- : Added execution module exports
- : Updated  to trigger execution immediately on approval (within one triage run)

All 44 tests pass.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44bd-978d-7194-b8d2-102b47df5144
author: oompah
created: 2026-05-20T09:35:36Z

**Completion:** CRIT-4 has been implemented. Human approval (👍 reaction or comment) now triggers backport branch and PR creation within one triage run.

Summary of implementation:
- Created  with  function
- Extended  with GitHub API methods: , , , 
- Modified  to execute backports immediately when approval is detected
- Updated  to export the execution module

When approval is detected:
1. Creates branch  from target release branch head
2. Files a chore task () for cherry-pick work
3. Creates PR targeting release/{X.Y}
4. Posts comment on source issue linking the backport

All within ONE triage run. All 44 tests pass.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44be-1cb1-7979-a139-192e38784260
author: oompah
created: 2026-05-20T09:36:10Z

Agent completed successfully in 873s (3222294 tokens)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44be-25a4-7561-8c12-d1f3412a854e
author: oompah
created: 2026-05-20T09:36:13Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 54, Tool calls: 68
- Tokens: 3.2M in / 22.5K out [3.2M total]
- Cost: $0.0000
- Exit: normal, Duration: 14m 33s
- Log: rogers-6vf__20260520T092145Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
