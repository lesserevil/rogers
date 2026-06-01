---
id: TASK-59
title: 'CRIT-7: Bug with all template fields populated transitions to ready-for-review
  without additional info requests'
status: Done
assignee: []
created_date: 2026-05-20 05:24
updated_date: 2026-05-21 04:54
labels:
- rodgers:parent=rogers-d7h
- rodgers:type=issue-templates
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-plc
  state: closed
  parent_id: rogers-d7h
  dependencies: []
  branch_name: rogers-plc
  target_branch: null
  url: null
  created_at: '2026-05-20T05:24:52Z'
  updated_at: '2026-05-21T04:54:36Z'
  closed_at: '2026-05-21T04:54:31Z'
parent: TASK-6
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/issue-templates-plan.md §Cross-Reference: Template Fields vs. Completeness Requirements → Acceptance Criteria CRIT-7

WHAT TO DO
Implement completeness check: bug with all template fields populated → ready-for-review without additional info requests.

Create/modify files:
- src/feature_bug/completeness.rs - Bug completeness verification
- src/templates/mapping.rs - Template field mapping (rogers-agw)
- src/triage/triage_loop.rs - Apply ready-for-review when complete

Verification:
- Bug issue filed with bug_report template
- All 4 fields present: Environment, Steps to Reproduce, Expected, Actual
- Completeness check passes
- Apply ready-for-review label
- NO needs-information comment posted
- NO additional info requests

WHY
Template exists to capture required info upfront. If template used correctly, no follow-up needed. Fast-tracks well-filed bugs.

HOW TO VERIFY
- Unit test: Bug with 4 fields → ready-for-review
- Unit test: No needs-information comment
- Unit test: No additional info requests
- Unit test: Missing any field → needs-information for that field only
- Integration test: Template-filed bug → ready-for-review in one run
- Manual: File bug via template, run triage, verify state

EDGE CASES AND PITFALLS
- Fields present but empty - treat as missing
- 'N/A' with explanation for Steps - valid if justified
- Custom template - semantic field mapping
- Feature request similar - 3 fields for ready-for-review
- Question - 2 fields for doc search readiness
- Human can still apply will-not-do/ready-for-work from ready-for-review

PROJECT-SPECIFIC TERMINOLOGY
- 'Template fields populated': All required sections filled
- 'ready-for-review': Label applied when completeness satisfied
- 'Additional info requests': needs-information comments
- 'Field mapping': rogers-agw semantic mapping
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e48d9-eae9-7f2b-a36c-5c9271720482
author: oompah
created: 2026-05-21T04:45:02Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48d9-f1be-72bb-832e-4dccf3f0b72d
author: oompah
created: 2026-05-21T04:45:03Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48da-95b8-7809-945d-834d019a803b
author: oompah
created: 2026-05-21T04:45:45Z

## Understanding

This issue (CRIT-7) requires implementing a **completeness check** for bug report issues filed with the bug_report template. When all 4 required template fields are populated — Environment, Steps to Reproduce, Expected Behavior, Actual Behavior — Rodgers should:

1. Apply the  label
2. Post NO  comment
3. Post NO additional info requests

If ANY field is missing or empty, only that field should be requested via .

The plan is:
1. Create  — the core completeness verification logic for bugs
2. Create  — semantic field mapping (maps template field names to Rodgers internal field names)
3. Create  — apply ready-for-review when completeness is satisfied
4. Wire these modules into  and 
5. Write unit tests covering all scenarios
6. Update the plan doc
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48e1-f590-7810-9568-f78849efdaeb
author: oompah
created: 2026-05-21T04:53:49Z

## Implementation

Created 3 new modules:

1. **** — Semantic field mapping (rogers-agw). Maps template headings like Environment, System, Platform to canonical fields . Also maps Steps
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48e2-7cf4-7ade-b3b0-e6ec4791ae7d
author: oompah
created: 2026-05-21T04:54:23Z

## Completion

Delivered: CRIT-7 — Bug completeness check transitions to ready-for-review without additional info requests.

**What was built:**
-  — Semantic field mapping (rogers-agw): Environment, Steps to Reproduce, Expected Behavior, Actual Behavior with word-boundary matching for custom templates
-  — Completeness verification: checks all 4 required fields, handles empty/whitespace/N/A/placeholders
-  — Triage dispatcher: ReadyForReview when complete (no needs-information comment, no additional requests), NeedsInformation for specific missing fields

**Test results:** 200/200 tests pass, formatting clean, clippy clean.
- Bug with 4 fields → ready-for-review ✓
- No needs-information comment posted ✓
- No additional info requests ✓
- Missing any field → needs-information for that field only ✓
- N/A with explanation treated as valid ✓
- Semantic mapping with custom field names ✓
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48e2-beb3-7903-b3d3-48e64115b6f0
author: oompah
created: 2026-05-21T04:54:40Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/qwen/qwen3.6-35b-a3b]
- Turns: 54, Tool calls: 70
- Tokens: 3.5M in / 31.1K out [3.5M total]
- Cost: $0.0000
- Exit: normal, Duration: 9m 40s
- Log: rogers-plc__20260521T044505Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48e2-c2ee-7543-b554-4a69a8b0eb97
author: oompah
created: 2026-05-21T04:54:41Z

Agent completed successfully in 580s (3508752 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
