---
id: TASK-7
title: Question Routing Plan Implementation
status: To Do
assignee: []
created_date: 2026-05-20 05:10
updated_date: 2026-05-20 05:10
labels:
- rodgers:type=question-routing
- epic
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: epic
tasks:
  id: rogers-4en
  state: open
  parent_id: null
  dependencies: []
  branch_name: rogers-4en
  target_branch: null
  url: null
  created_at: '2026-05-20T05:10:24Z'
  updated_at: '2026-05-20T05:10:24Z'
  closed_at: null
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/question-routing-plan.md

Acceptance Criteria:
- [ ] CRIT-1: When a question issue exists and docs exist that answer it, Rodgers posts a comment within one triage run with the correct doc link
- [ ] CRIT-2: When a question issue exists and no docs answer it, Rodgers searches the source code if the question is about implementation details before filing a doc-gap task
- [ ] CRIT-3: When Rodgers finds an answer in the source code, it posts a plain-language explanation citing the relevant file, function, and line numbers, then closes the issue if fully answered
- [ ] CRIT-4: When a chore task (rodgers:type=docs) is closed, Rodgers verifies the GitHub issue has a documentation link comment; if the link is missing, Rodgers posts it within one triage run; if the issue is already closed, Rodgers posts the link comment anyway; on GitHub API read failure, Rodgers retries on the next triage run without alerting
- [ ] CRIT-5: Rodgers never closes a question issue without either answering it or filing a chore task (rodgers:type=docs)
- [ ] CRIT-6: Rodgers never routes a non-question issue through this workflow
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENTS:END -->
