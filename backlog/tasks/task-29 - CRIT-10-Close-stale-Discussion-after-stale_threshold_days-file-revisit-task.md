---
id: TASK-29
title: 'CRIT-10: Close stale Discussion after stale_threshold_days, file revisit task'
status: To Do
assignee: []
created_date: 2026-05-20 05:20
updated_date: 2026-05-21 03:11
labels:
- asking_question
- rodgers:parent=rogers-4qr
- rodgers:type=backport
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-19c
  state: open
  parent_id: rogers-4qr
  dependencies: []
  branch_name: rogers-19c
  target_branch: null
  url: null
  created_at: '2026-05-20T05:20:15Z'
  updated_at: '2026-05-21T03:11:44Z'
  closed_at: null
parent: TASK-2
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/backport-plan.md §Acceptance Criteria CRIT-10

WHAT TO DO
Implement stale Discussion closure after stale_threshold_days, file revisit task.

Create/modify files:
- src/backport/approval.rs - Stale threshold detection
- src/backport/manager.rs - Close discussion, file revisit task
- src/config/schema.rs - release.stale_threshold_days (default: 7)
- src/github/client.rs - Close discussion
- src/tasks/client.rs - File revisit task

Behavior:
- Track Discussion creation time
- On triage run, check age > stale_threshold_days
- If no human response: close Discussion
- File chore task: 'Revisit backport for #{sha} to {branch}'
- Task notes: discussion closed stale, needs human decision
- Total time = voting_window + stale_threshold (with pings)

WHY
Prevents abandoned discussions. Revisit task tracks need for human decision.

HOW TO VERIFY
- Unit test: Discussion age > stale_threshold_days → close
- Unit test: Revisit task filed
- Unit test: Task notes stale closure
- Unit test: Uses config stale_threshold_days
- Unit test: Total time = voting + stale
- Integration test: Stale discussion, run triage, verify
- Manual: Create discussion, wait, run triage

EDGE CASES AND PITFALLS
- stale_threshold_days default 7
- voting_window_days default 2
- Total ~9 days with reminder
- Response = reaction OR comment
- Revisit task: chore, priority normal
- Don't proceed with backport

PROJECT-SPECIFIC TERMINOLOGY
- 'stale_threshold_days': Config, days before close
- 'Stale Discussion': No response in threshold
- 'Revisit task': Chore task for human decision
- 'Backport Discussion': Approval discussion
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e4884-2d2f-7419-b4b5-d48a4a26a1fa
author: oompah
created: 2026-05-21T03:11:22Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4884-4175-796b-84f8-648b8a20b98d
author: oompah
created: 2026-05-21T03:11:28Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4884-7436-79ef-a582-e149e4b09556
author: oompah
created: 2026-05-21T03:11:41Z

🤚 **Question from agent:**

The issue asks me to implement stale Discussion closure for backport approvals. I need to understand the existing approval flow. Let me read the plan and explore the codebase structure.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4884-7f59-7c79-9a63-13689a380bfa
author: oompah
created: 2026-05-21T03:11:43Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/qwen/qwen3.6-35b-a3b]
- Turns: 2, Tool calls: 3
- Tokens: 20.6K in / 307 out [20.9K total]
- Cost: $0.0000
- Exit: ask_question, Duration: 20s
- Log: rogers-19c__20260521T031132Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
