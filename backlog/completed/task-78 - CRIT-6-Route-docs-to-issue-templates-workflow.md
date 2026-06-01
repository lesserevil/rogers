---
id: TASK-78
title: 'CRIT-6: Route docs to issue-templates workflow'
status: Done
assignee: []
created_date: 2026-05-20 05:28
updated_date: 2026-05-21 07:01
labels:
- rodgers:parent=rogers-jh3
- rodgers:type=triage-workflow
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-4os
  state: closed
  parent_id: rogers-jh3
  dependencies: []
  branch_name: rogers-4os
  target_branch: null
  url: null
  created_at: '2026-05-20T05:28:00Z'
  updated_at: '2026-05-21T07:01:14Z'
  closed_at: '2026-05-21T07:01:07Z'
parent: TASK-9
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/triage-workflow-plan.md §Top-Level Classification → routes to plans/issue-templates-plan.md

WHAT TO DO
Implement routing logic that sends classified 'docs' issues to the issue-templates workflow for template improvements or documentation updates.

Create/modify files:
- src/triage/router.rs - Route docs issues to issue-templates workflow
- src/issue_templates/mod.rs - Issue templates workflow entry point

Routing behavior:
- Issues classified as 'docs' get 'rodgers:docs' label applied
- Route to issue-templates workflow for template improvements or documentation updates
- This handles: missing templates, template field updates, documentation corrections

WHY
Documentation/template issues are meta-work - they improve the issue filing experience for everyone. Routing to issue-templates workflow ensures they're tracked and addressed systematically.

HOW TO VERIFY
- Unit test: Docs issue gets rodgers:docs label
- Unit test: Docs issue routed to issue-templates workflow
- Integration test: Template-related issue creates appropriate tasks
- Manual: Create test docs issue, verify routing and label

EDGE CASES AND PITFALLS
- Distinguish between 'question about docs' (→ question workflow) vs 'docs need update' (→ issue-templates workflow)
- Template changes require human review (governance decision) - Rodgers files task but doesn't auto-commit
- Documentation gaps from questions route to question workflow (doc-gap tasks), not here

PROJECT-SPECIFIC TERMINOLOGY
- 'rodgers:docs': Label marking issue as documentation/template work
- 'Issue-templates workflow': plans/issue-templates-plan.md implementation
- 'Auto_suggest': Config option to file task with default templates when none found
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e494f-92be-71bf-9659-561069fd478e
author: oompah
created: 2026-05-21T06:53:32Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e494f-ab17-7e18-8a77-40a497114660
author: oompah
created: 2026-05-21T06:53:38Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4950-469d-70f9-aa88-ee09077937e5
author: oompah
created: 2026-05-21T06:54:18Z

## Understanding

The issue requires adding a docs-routing path to the triage router that:
1. Detects issues classified as 'docs' (via the  label)
2. Applies the  label 
3. Routes them to a new issue-templates workflow

Architecture analysis:
-  already handles question routing (question label → question workflow)
-  has the  enum with variants like 
-  shows the pattern for a workflow handler module
-  has label definitions for triage labels
- The router is called from  in triage_loop.rs before bug/feature processing

My plan:
1. Add  constant and  variant
2. Create  with issue-templates workflow entry point
3. Modify  to add docs routing logic (route docs issues, distinguish from doc-gap questions)
4. Update  to export the new module
5. Add unit tests for all routing behavior
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4950-6337-7420-b9b5-0f987e49542f
author: oompah
created: 2026-05-21T06:54:26Z

Understanding: The issue requires adding a docs-routing path to the triage router. Architecture: src/triage/router.rs already handles question routing. src/triage/triage_loop.rs has the TriageAction enum. src/question_router/mod.rs shows the pattern for a workflow handler module. src/labels.rs has label definitions. The router is called from process_issue() before bug/feature processing. My plan: 1) Add TriageAction::RoutedToIssueTemplates variant. 2) Create src/issue_templates/mod.rs with issue-templates workflow entry point. 3) Modify src/triage/router.rs to add docs routing logic. 4) Update src/lib.rs to export the new module. 5) Add unit tests.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4954-ca1d-7126-b42f-ae5892f07bcf
author: oompah
created: 2026-05-21T06:59:14Z

Implementation: Created src/issue_templates/mod.rs as the issue-templates workflow entry point. Modified src/triage/router.rs to route docs issues (docs label) to the issue-templates workflow with rodgers:docs label applied. Added TriageAction::RoutedToIssueTemplates variant. Updated src/labels.rs with docs label definition. Added 30 unit tests (17 in router, 13 in issue_templates). All 30 pass. Build clean, 3 pre-existing failures in question_router unrelated to this work.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4956-659e-7cb7-a145-ead86b3e8088
author: oompah
created: 2026-05-21T07:00:59Z

Completion: All changes committed and pushed. Summary of deliverables:

1. src/issue_templates/mod.rs (new) - Issue-templates workflow entry point with 4 classifiers: missing templates, template field updates, documentation corrections, generic docs
2. src/triage/router.rs (new) - Docs routing path applying rodgers:docs label, routing docs issues to issue-templates workflow, distinguishing docs from question workflow
3. src/triage/triage_loop.rs (modified) - Added RoutedToIssueTemplates action variant
4. src/triage/mod.rs (modified) - Added LABEL_DOCS and LABEL_RODGERS_DOCS constants
5. src/lib.rs (modified) - Export issue_templates module
6. src/labels.rs (modified) - Added docs label definition

30 unit tests, all passing. 3 pre-existing failures in question_router unrelated to this work. Edge cases handled: docs-vs-question distinction, closed issue skipping, already-routed no-op, template changes require human review messaging.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4956-93a6-7bcf-abfd-735048de46d9
author: oompah
created: 2026-05-21T07:01:11Z

Agent completed successfully in 459s (2932768 tokens)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4956-9865-79d5-98d7-ddc62c9c91a0
author: oompah
created: 2026-05-21T07:01:12Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/qwen/qwen3.6-35b-a3b]
- Turns: 48, Tool calls: 59
- Tokens: 2.9M in / 25.0K out [2.9M total]
- Cost: $0.0000
- Exit: normal, Duration: 7m 39s
- Log: rogers-4os__20260521T065341Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
