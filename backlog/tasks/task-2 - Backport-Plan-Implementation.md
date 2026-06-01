---
id: TASK-2
title: Backport Plan Implementation
status: To Do
assignee: []
created_date: 2026-05-20 05:09
updated_date: 2026-05-20 05:42
labels:
- rodgers:type=backport
- epic
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: epic
tasks:
  id: rogers-4qr
  state: open
  parent_id: null
  dependencies: []
  branch_name: rogers-4qr
  target_branch: null
  url: null
  created_at: '2026-05-20T05:09:40Z'
  updated_at: '2026-05-20T05:42:59Z'
  closed_at: null
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/backport-plan.md

Acceptance Criteria:
- [ ] CRIT-1: When a bug fix, security patch, or backport-me labeled issue is merged to main, Rodgers identifies all active release branches within one triage run
- [ ] CRIT-2: Rodgers files a backport task for each target branch within one triage run of detecting the candidate
- [ ] CRIT-3: Rodgers creates a GitHub Discussion for each backport and waits for human approval before opening a PR
- [ ] CRIT-4: A human approval triggers the creation of a backport branch and PR targeting the correct release branch within one triage run
- [ ] CRIT-5: If a backport has merge conflicts, Rodgers files a conflict-resolution task and posts an alert comment, but does not attempt autonomous conflict resolution
- [ ] CRIT-6: When a backport PR is merged, Rodgers closes the corresponding backport task and checks for release completeness
- [ ] CRIT-7: The backport approval Discussion body contains at minimum: the commit SHA, the commit message, the source GitHub issue number, and the target release branch — all extracted directly from the merged commit and linked issue at creation time
- [ ] CRIT-8: A rejection reaction on the approval Discussion halts the backport and Rodgers posts a comment acknowledging the rejection and asking for guidance within one triage run of detecting the reaction
- [ ] CRIT-9: If no approval reaction is received within release.voting_window_days, Rodgers posts a reminder comment on the Discussion
- [ ] CRIT-10: If no human response is received within release.stale_threshold_days (total, including the voting window and any pings), Rodgers closes the Discussion, files a revisit task, and does not proceed with the backport
- [ ] CRIT-11: For backport approvals: any rejection before the backport PR is created halts the backport; once the PR is created the vote is locked and subsequent rejection is acknowledged but does not stop the work; conflicting simultaneous votes resolve to rejection (halt + ask for clarification); votes on a stale-closed Discussion are ignored
- [ ] CRIT-12: Rodgers detects a security patch when any of: GH Advisory match (via repository.advisories()), security label on the issue (or rogation.security_label), or CVE pattern (CVE-dddddddd) in commit message or issue body; detected security patches are filed as priority=1 tasks
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e43e3-a9ef-71d5-a4e4-5488073305d9
author: oompah
created: 2026-05-20T05:37:34Z

Agent dispatched (profile: deep)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e43e3-d6ff-71dc-bdcc-66a4c6fe6ced
author: oompah
created: 2026-05-20T05:37:46Z

Focus: Epic Planner
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e43e6-bf29-72e6-9292-f27f9e5073de
author: oompah
created: 2026-05-20T05:40:56Z

Understanding: This epic implements the backport management system per plans/backport-plan.md. All 12 CRIT acceptance criteria have corresponding child tasks (rogers-qox through rogers-tf9), but they lack actionable descriptions. I'll now update each task with complete WHAT/WHY/HOW/VERIFY/EDGE CASES context so they can be executed independently.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e43e7-ae8a-7c2d-ba13-e2cbcbb2cee4
author: oompah
created: 2026-05-20T05:41:58Z

Run #1 [attempt=1, profile=deep, role=deep -> InferenceAPI/nvidia/nvidia/nemotron-3-ultra-preview]
- Turns: 0, Tool calls: 30
- Tokens: 0 in / 0 out [0 total]
- Cost: $0.0000
- Exit: terminated, Duration: 4m 22s
- Log: rogers-4qr__20260520T053748Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
