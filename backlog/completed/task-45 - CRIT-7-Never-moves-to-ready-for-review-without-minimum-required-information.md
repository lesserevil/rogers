---
id: TASK-45
title: 'CRIT-7: Never moves to ready-for-review without minimum required information'
status: Done
assignee: []
created_date: 2026-05-20 05:22
updated_date: 2026-05-20 10:24
labels:
- rodgers:parent=rogers-ykp
- rodgers:type=feature-bug
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-0hj
  state: closed
  parent_id: rogers-ykp
  dependencies: []
  branch_name: rogers-0hj
  target_branch: null
  url: null
  created_at: '2026-05-20T05:22:38Z'
  updated_at: '2026-05-20T10:24:47Z'
  closed_at: '2026-05-20T10:24:40Z'
parent: TASK-4
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/feature-bug-plan.md §Completeness Check → Acceptance Criteria CRIT-7

WHAT TO DO
Implement guard: NEVER move to ready-for-review without minimum required information for issue type.

Create/modify files:
- src/feature_bug/completeness.rs - Minimum info enforcement
- src/feature_bug/mod.rs - Block transition if incomplete
- src/triage/triage_loop.rs - Enforce on triage

Minimum required (from plan):
Bug: Behavior observed, Behavior expected, Reproduction steps, Environment (4 fields)
Feature: Use case, Proposed behavior, Acceptance criteria (3 fields)

Enforcement:
- Check completeness BEFORE ready-for-review
- If ANY minimum missing: needs-information, not ready-for-review
- Template mapping validates fields (rogers-agw)
- Freeform: LLM validates minimum
- Hard block - no exceptions

WHY
ready-for-review = human review gate. Human time wasted on incomplete issues. Minimum info = respect human time.

HOW TO VERIFY
- Unit test: Bug missing 1 field → needs-information, not ready-for-review
- Unit test: Feature missing 1 field → needs-information, not ready-for-review
- Unit test: All minimum present → ready-for-review allowed
- Unit test: Template-filed complete → ready-for-review
- Unit test: Freeform complete → ready-for-review
- Integration test: Incomplete issues never reach ready-for-review
- Manual: File incomplete, run triage, verify blocked

EDGE CASES AND PITFALLS
- 'N/A' with justification for reproduction - evaluate case by case
- Template fields empty - treat as missing
- Human can't override - system enforced
- LLM validation for freeform
- Applies to both bug AND feature

PROJECT-SPECIFIC TERMINOLOGY
- 'Minimum required information': 4 bug fields, 3 feature fields
- 'ready-for-review': Human decision gate state
- 'needs-information': Blocking state for missing info
- 'Template mapping': rogers-agw semantic validation
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e44dd-7ff9-7b0a-98d9-d00d89385127
author: oompah
created: 2026-05-20T10:10:27Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44dd-9a10-7ba6-bb84-50c44bab2c4d
author: oompah
created: 2026-05-20T10:10:34Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44df-18f1-79a9-8f6f-bc60677d464e
author: oompah
created: 2026-05-20T10:12:12Z

Understanding: The implementation for CRIT-7 is largely complete -  in triage_loop.rs enforces the guard by checking completeness before applying ready-for-review. Existing functions  and  correctly identify missing fields. Plan: Add targeted tests per the issue's "HOW TO VERIFY" section to ensure complete coverage, including single-missing-field tests and edge cases.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44e9-1c60-7d21-9304-3e78f30c8472
author: oompah
created: 2026-05-20T10:23:08Z

Implementation: Added comprehensive CRIT-7 tests to src/triage/triage_loop.rs. Test coverage includes: - Tests for bug missing each individual field (behavior_observed, behavior_expected, reproduction_steps, environment) - Tests for feature missing each individual field (use_case, proposed_behavior, acceptance_criteria) - Tests verifying complete issues reach ready-for-review - Template-fielded and freeform style issue tests - Integration test verifying incomplete issues never reach ready-for-review - Edge case tests (empty fields, N/A reproduction, hard block enforcement)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44ea-493a-77ec-b281-a252a870cd2e
author: oompah
created: 2026-05-20T10:24:25Z

Completion: CRIT-7 is enforced by the existing implementation in src/triage/triage_loop.rs (run_completeness_check function). Added 21 new tests covering all verification requirements:

VERIFICATION MATRIX:
- ✅ Bug missing 1 field → needs-information, not ready-for-review
- ✅ Feature missing 1 field → needs-information, not ready-for-review  
- ✅ All minimum present → ready-for-review allowed
- ✅ Template-filed complete → ready-for-review
- ✅ Freeform complete → ready-for-review
- ✅ Integration test: Incomplete issues never reach ready-for-review
- ✅ Edge case: Empty fields treated as missing
- ✅ Edge case: N/A reproduction with justification is complete
- ✅ Hard block: System enforced (no human override)

Tests: 228 passed (21 new tests added)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44ea-96f8-7a8e-b1b6-7dc07fc66a63
author: oompah
created: 2026-05-20T10:24:45Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 41, Tool calls: 46
- Tokens: 2.8M in / 22.1K out [2.8M total]
- Cost: $0.0000
- Exit: normal, Duration: 14m 18s
- Log: rogers-0hj__20260520T101035Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44ea-9b9d-7dcf-b89e-7807dfffa2c2
author: oompah
created: 2026-05-20T10:24:46Z

Agent completed successfully in 858s (2795216 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
