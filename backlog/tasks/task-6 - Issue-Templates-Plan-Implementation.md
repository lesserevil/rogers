---
id: TASK-6
title: Issue Templates Plan Implementation
status: To Do
assignee: []
created_date: 2026-05-20 05:10
updated_date: 2026-05-20 05:10
labels:
- rodgers:type=issue-templates
- epic
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: epic
tasks:
  id: rogers-d7h
  state: open
  parent_id: null
  dependencies: []
  branch_name: rogers-d7h
  target_branch: null
  url: null
  created_at: '2026-05-20T05:10:14Z'
  updated_at: '2026-05-20T05:10:14Z'
  closed_at: null
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/issue-templates-plan.md

Acceptance Criteria:
- [ ] CRIT-1: On startup, Rodgers detects whether .github/ISSUE_TEMPLATE/ contains bug_report.md, feature_request.md, and question.md
- [ ] CRIT-2: When a project has no templates and auto_suggest: true, Rodgers files a task with suggested default templates within one triage run
- [ ] CRIT-3: A non-conforming issue (filed without template marker) receives a reformat offer comment within one triage run
- [ ] CRIT-4: Rodgers never reformats an issue without the requestor's explicit approval
- [ ] CRIT-5: When a requestor approves a reformat, Rodgers posts the reformatted content as a comment for requestor review before applying it
- [ ] CRIT-6: All default template fields map to a completeness requirement in plans/feature-bug-plan.md or plans/question-routing-plan.md
- [ ] CRIT-7: A bug report with all required template fields populated transitions to ready-for-review without requesting additional information
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENTS:END -->
