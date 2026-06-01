---
id: TASK-55
title: 'CRIT-3: Non-conforming issue gets reformat offer comment within one triage
  run'
status: Done
assignee: []
created_date: 2026-05-20 05:24
updated_date: 2026-05-20 08:37
labels:
- rodgers:parent=rogers-d7h
- rodgers:type=issue-templates
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-3wv
  state: closed
  parent_id: rogers-d7h
  dependencies: []
  branch_name: rogers-3wv
  target_branch: null
  url: null
  created_at: '2026-05-20T05:24:12Z'
  updated_at: '2026-05-20T08:37:52Z'
  closed_at: '2026-05-20T08:37:45Z'
parent: TASK-6
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/issue-templates-plan.md §Template Conformance / Non-Conforming Issues → Acceptance Criteria CRIT-3

WHAT TO DO
Implement detection of non-conforming issues and reformat offer comment within one triage run.

Create/modify files:
- src/templates/conformance.rs - Detect non-conforming issues
- src/templates/mod.rs - Post reformat offer comment
- src/triage/triage_loop.rs - Check conformance on new issues

Detection:
- Check for template marker (hidden comment) in issue body
- Marker absent = filed without template = non-conforming
- Email replies (GitHub Email Reply) also non-conforming

Reformat offer comment:
- Warm, inviting tone (not scolding)
- Offers to rewrite using appropriate template
- Asks for confirmation before proceeding
- Template chosen by LLM based on content

WHY
Non-conforming issues lack structure. Reformat offer helps requestor provide needed info. One triage run = fast feedback.

HOW TO VERIFY
- Unit test: Detects missing template marker
- Unit test: Detects email reply pattern
- Unit test: Posts reformat offer within one triage run
- Unit test: Comment tone warm, not scolding
- Unit test: Offers correct template (bug/feature/question)
- Integration test: Non-conforming issue→offer comment
- Manual: Create issue without template, run triage

EDGE CASES AND PITFALLS
- Requestor closes issue before offer - check open state first
- Partial template use - only ask for missing fields
- Custom templates with non-standard fields - map semantically
- GitHub API failure posting comment - retry
- Never reformat without approval (CRIT-4)
- One offer only - no repeat offers

PROJECT-SPECIFIC TERMINOLOGY
- 'Non-conforming issue': Filed without template marker
- 'Template marker': Hidden comment like '<!-- template: bug_report -->'
- 'Reformat offer': Comment inviting requestor to use template
- 'One triage run': Single scheduler tick - fast response
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e446b-26c7-71a8-a940-ce2b7e5971a9
author: oompah
created: 2026-05-20T08:05:34Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e446b-5172-7945-931c-d5d1ccd775a5
author: oompah
created: 2026-05-20T08:05:44Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e446b-70e6-7e45-bae5-0e29ec42d127
author: oompah
created: 2026-05-20T08:05:52Z

Starting work on CRIT-3: Non-conforming issue detection and reformat offer comment. Will explore the codebase architecture first.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4487-ca49-737f-a6fe-1ca75c99a973
author: oompah
created: 2026-05-20T08:36:50Z

Implementation complete. Created: src/templates/conformance.rs (detection), src/triage/reformat_offer.rs (comment generation), src/triage/triage_loop.rs (triage logic). All 67 tests pass, clippy clean.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4488-addd-71f4-ad26-d5d54073ac18
author: oompah
created: 2026-05-20T08:37:49Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 78, Tool calls: 91
- Tokens: 2.9M in / 25.0K out [2.9M total]
- Cost: $0.0000
- Exit: normal, Duration: 32m 14s
- Log: rogers-3wv__20260520T080546Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4488-b091-703d-b13b-d6241557ee8f
author: oompah
created: 2026-05-20T08:37:49Z

Agent completed successfully in 1934s (2927613 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
