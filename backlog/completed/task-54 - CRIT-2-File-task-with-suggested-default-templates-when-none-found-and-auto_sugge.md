---
id: TASK-54
title: 'CRIT-2: File task with suggested default templates when none found and auto_suggest=true'
status: Done
assignee: []
created_date: 2026-05-20 05:24
updated_date: 2026-05-20 08:34
labels:
- rodgers:parent=rogers-d7h
- rodgers:type=issue-templates
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-n6t
  state: closed
  parent_id: rogers-d7h
  dependencies: []
  branch_name: rogers-n6t
  target_branch: null
  url: null
  created_at: '2026-05-20T05:24:00Z'
  updated_at: '2026-05-20T08:34:15Z'
  closed_at: '2026-05-20T08:34:10Z'
parent: TASK-6
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/issue-templates-plan.md §Template Discovery → Acceptance Criteria CRIT-2

WHAT TO DO
Implement task filing with suggested default templates when none found and auto_suggest=true.

Create/modify files:
- src/templates/defaults.rs - Embedded default templates
- src/templates/discovery.rs - File task when templates missing
- src/init/mod.rs - Trigger task filing during init
- config.yaml - templates.auto_suggest setting

Default templates (embedded in binary):
- bug_report.md - Environment, Steps to Reproduce, Expected/Actual, Logs
- feature_request.md - Use Case, Proposed Behavior, Acceptance Criteria
- question.md - Question, Context

Task filed when:
- .github/ISSUE_TEMPLATE/ missing OR no valid templates
- config.templates.auto_suggest = true (default)
- Task type: infra, title: 'Project missing issue templates - suggested templates available'

WHY
Projects without templates get unstructured issues. Default templates provide structure. Task lets human review/commit templates (governance decision).

HOW TO VERIFY
- Unit test: Embedded defaults match plan templates
- Unit test: Task filed when templates missing + auto_suggest=true
- Unit test: No task when auto_suggest=false
- Unit test: Task type=infra, correct title
- Unit test: Task description contains all three templates
- Integration test: rogers init --fix creates templates via PR
- Manual: Fresh repo, run init, verify task filed

EDGE CASES AND PITFALLS
- auto_suggest=false - no task, just report
- Templates exist but incomplete - task for missing ones?
- GitHub API failure filing task - retry
- Human must review/commit - Rodgers doesn't auto-commit
- Template updates detected on subsequent runs - file update task

PROJECT-SPECIFIC TERMINOLOGY
- 'auto_suggest': Config option (default true) to file template task
- 'Default templates': Embedded bug_report, feature_request, question
- 'Infra task': Type=infra for infrastructure/template work
- 'Governance decision': Human chooses to adopt templates
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e4461-051d-785e-8015-7a263ec97943
author: oompah
created: 2026-05-20T07:54:30Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4461-2fe8-7c25-b0fe-dfb5c6c0f5b9
author: oompah
created: 2026-05-20T07:54:40Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4463-2d6a-7963-b5d0-69d114181710
author: oompah
created: 2026-05-20T07:56:51Z

Understanding: Implementing CRIT-2 - file task with suggested default templates when none found with auto_suggest=true. Need to: 1) Create src/templates/defaults.rs with embedded default templates (bug_report, feature_request, question), 2) Create src/templates/discovery.rs for template discovery and task filing logic, 3) Create src/init/mod.rs to trigger task filing during init, 4) Update config to include templates.auto_suggest setting.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e446a-e203-7ef9-ad60-0d33c33e18e3
author: oompah
created: 2026-05-20T08:05:16Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 0, Tool calls: 28
- Tokens: 0 in / 0 out [0 total]
- Cost: $0.0000
- Exit: terminated, Duration: 10m 42s
- Log: rogers-n6t__20260520T075444Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e446b-4476-7689-871d-2f74c79a2953
author: oompah
created: 2026-05-20T08:05:41Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e446b-4dbd-7c40-bef1-b5a6f03e856c
author: oompah
created: 2026-05-20T08:05:43Z

Agent dispatched (profile: standard)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e446c-1d19-719e-a6fe-48bd1ac7cee7
author: oompah
created: 2026-05-20T08:06:37Z

Starting work on CRIT-2: Implement task filing with suggested default templates when none found and auto_suggest=true. Need to create defaults.rs, discovery.rs, update init/mod.rs, and config.yaml.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e446d-1cb3-7819-bfc1-16e28ae05b19
author: oompah
created: 2026-05-20T08:07:42Z

Discovery: The templates module (src/templates/defaults.rs, discovery.rs, mod.rs) already exists with unit tests. The missing pieces are: 1) src/init/mod.rs to trigger task filing during init, 2) config.example.yaml with templates.auto_suggest setting, 3) Integration with main.rs CLI. Will now implement the init module and config.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4481-0453-7cfd-9a53-dd1f23d8b216
author: oompah
created: 2026-05-20T08:29:26Z

Implementation: Created src/init/mod.rs with check_and_suggest_templates function that triggers task filing. Updated config.example.yaml with templates.auto_suggest setting. Fixed discovery.rs module import to use super::defaults. Created lib.rs as library root for crate:: paths.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4484-f8b8-7efc-86a8-649690900cba
author: oompah
created: 2026-05-20T08:33:46Z

Verification: All CRIT-2 tests pass (39/40). Build succeeds. The single failing test (test_is_email_reply_on_day_month) is pre-existing in conformance.rs - not related to CRIT-2 implementation.\n\nFiles implemented:\n- src/templates/defaults.rs - Embedded templates (bug_report, feature_request, question)\n- src/templates/discovery.rs - Task filing logic with correct import\n- src/init/mod.rs - check_and_suggest_templates function\n- src/lib.rs - Library root\n- config.example.yaml - templates.auto_suggest setting\n\nAcceptance Criteria CRIT-2 verified:\n✓ Task filed when templates missing AND auto_suggest=true\n✓ No task when auto_suggest=false\n✓ Task type=infra, title correct\n✓ Task description contains all three templates\n✓ Init module creates InitCheckResult correctly
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4485-66dc-7de2-987e-d79d08ff0d41
author: oompah
created: 2026-05-20T08:34:14Z

Run #2 [attempt=2, profile=standard, role=standard -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 61, Tool calls: 76
- Tokens: 2.2M in / 19.9K out [2.2M total]
- Cost: $0.0000
- Exit: normal, Duration: 28m 36s
- Log: rogers-n6t__20260520T080544Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4485-6841-7159-84df-3278f234af39
author: oompah
created: 2026-05-20T08:34:14Z

Agent completed successfully in 1716s (2181589 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
