---
id: TASK-9
title: Triage Workflow Plan Implementation
status: To Do
assignee: []
created_date: 2026-05-20 05:11
updated_date: 2026-05-20 05:11
labels:
- rodgers:type=triage-workflow
- epic
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: epic
tasks:
  id: rogers-jh3
  state: open
  parent_id: null
  dependencies: []
  branch_name: rogers-jh3
  target_branch: null
  url: null
  created_at: '2026-05-20T05:11:05Z'
  updated_at: '2026-05-20T05:11:05Z'
  closed_at: null
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/triage-workflow-plan.md

Acceptance Criteria:
- [ ] CRIT-1: A new issue with a recognized template appears in the triage queue within one polling interval of creation
- [ ] CRIT-2: An issue missing a recognized template receives a single reformat offer comment within one polling interval; no repeat offers
- [ ] CRIT-3: A question issue receives an answer OR a doc-gap task within one polling interval
- [ ] CRIT-4: A feature or bug issue transitions to ready-for-review only when all completeness requirements in plans/feature-bug-plan.md are satisfied
- [ ] CRIT-5: When an external user comments on a question issue, Rodgers re-evaluates the issue within one polling interval
- [ ] CRIT-6: Every triage evaluation is recorded in a task (rodgers:type=triage) with issue number, decision, reason, and timestamp
- [ ] CRIT-7: Triage tasks are append-only; they are never edited after creation
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENTS:END -->
