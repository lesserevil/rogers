---
id: TASK-39
title: 'CRIT-1: Complete bug/feature issue transitions to ready-for-review within
  one triage run'
status: Done
assignee: []
created_date: 2026-05-20 05:21
updated_date: 2026-05-20 08:01
labels:
- rodgers:parent=rogers-ykp
- rodgers:type=feature-bug
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-irh
  state: closed
  parent_id: rogers-ykp
  dependencies: []
  branch_name: rogers-irh
  target_branch: null
  url: null
  created_at: '2026-05-20T05:21:43Z'
  updated_at: '2026-05-20T08:01:46Z'
  closed_at: '2026-05-20T08:01:42Z'
parent: TASK-4
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/feature-bug-plan.md §Completeness Check → Acceptance Criteria CRIT-1

WHAT TO DO
Implement completeness check: complete bug/feature issue transitions to ready-for-review within one triage run.

Create/modify files:
- src/feature_bug/completeness.rs - Completeness verification
- src/feature_bug/mod.rs - Transition to ready-for-review
- src/triage/triage_loop.rs - Call completeness on bug/feature issues

Completeness requirements (from plan):
Bug: Behavior observed, Behavior expected, Reproduction steps, Environment
Feature: Use case, Proposed behavior, Acceptance criteria (testable list)

Flow:
- On triage run, check bug/feature issues without ready-for-review
- Verify all required fields present (via template mapping or LLM)
- If complete: apply ready-for-review label, post summary comment
- If incomplete: apply needs-information, request specific missing fields
- Complete within ONE triage run

WHY
Fast transition to human review when info complete. One triage run = predictable latency.

HOW TO VERIFY
- Unit test: Complete bug (4 fields) → ready-for-review in one run
- Unit test: Complete feature (3 fields) → ready-for-review in one run
- Unit test: Summary comment posted
- Unit test: No delay - same run
- Integration test: Template-filed bug → ready-for-review next run
- Manual: File complete bug, run triage, verify state

EDGE CASES AND PITFALLS
- Template fields mapped semantically (rogers-agw)
- 'N/A' with justification valid for reproduction steps
- LLM verification for freeform issues
- Human can override from ready-for-review
- Must not move without minimum info (CRIT-7)

PROJECT-SPECIFIC TERMINOLOGY
- 'Completeness check': Verify required fields present
- 'ready-for-review': Label for human decision gate
- 'One triage run': Single scheduler tick
- 'Template mapping': rogers-agw semantic field mapping
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e4451-daaa-7b51-8520-2821d1ee1a33
author: oompah
created: 2026-05-20T07:37:56Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4451-f0cc-725c-9f83-3ebee13b71cb
author: oompah
created: 2026-05-20T07:38:01Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4455-9176-79f2-8267-c320d3106bad
author: oompah
created: 2026-05-20T07:41:59Z

## Understanding

Implementing CRIT-1: Complete bug/feature issue transitions to ready-for-review within one triage run.

### Requirements:
- **Bug completeness (4 fields):** Behavior observed, Behavior expected, Reproduction steps (or N/A + justification), Environment
- **Feature completeness (3 fields):** Use case, Proposed behavior, Acceptance criteria (testable list)

### Flow:
1. On triage run, check bug/feature issues without 
2. Verify required fields via template mapping or LLM
3. If complete: apply , post summary comment
4. If incomplete: apply , request specific missing fields
5. Transition completes within ONE triage run

### Files to create:
-  - Completeness verification
-  - Transition to ready-for-review
-  - Call completeness on bug/feature issues

### Approach:
1. Define completeness check structs for bug and feature
2. Create LLM-based field extraction/verification
3. Implement transition logic (incomplete → needs-information; complete → ready-for-review)
4. Add triage loop integration
5. Write unit tests for all scenarios
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4464-4fc6-7793-8cff-9e18e3cf3b64
author: oompah
created: 2026-05-20T07:58:05Z

## Implementation Complete

### Files Created:
- src/feature_bug/completeness.rs - Completeness verification with pattern-based field detection
- src/feature_bug/mod.rs - Transition module with warm summary comments
- src/triage/triage_loop.rs - Triage loop integration
- src/triage/mod.rs - Triage module re-export
- src/lib.rs - Library module declarations

### Key Features Implemented:
1. Bug completeness (4 fields): Behavior observed, behavior expected, reproduction steps, environment
2. Feature completeness (3 fields): Use case, proposed behavior, acceptance criteria
3. Template mapping: Supports standard GitHub issue template field names (e.g., "What Happened" → "Behavior Observed")
4. Warm comments: Rodgers-composed summaries with acceptance criteria drafts
5. One-run transition: Complete issues immediately transition to ready-for-review

### Test Results: 34 tests passing
- Complete bug (4 fields) → ready-for-review in one run ✓
- Complete feature (3 fields) → ready-for-review in one run ✓
- Summary comment posted ✓
- No delay - same run ✓
- Template-filed issues recognized via semantic field mapping ✓
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4467-4542-715b-9732-d3bd8777c6b9
author: oompah
created: 2026-05-20T08:01:19Z

## Verification Complete

### Test Results: 34 tests PASSED

All acceptance criteria verified:
- ✓ Complete bug (4 fields) → ready-for-review in one run
- ✓ Complete feature (3 fields) → ready-for-review in one run  
- ✓ Summary comment posted with acceptance criteria
- ✓ No delay - transitions complete in same run
- ✓ Template-filed issues recognized via semantic field mapping
- ✓ Needs-information applied for incomplete issues with specific field requests

Quality gates:
- ✓ cargo build
- ✓ cargo test (34/34 passed)
- ✓ cargo fmt --check
- ✓ cargo clippy (no errors)

Branch pushed successfully. Ready for review.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4467-a80e-7364-9e1a-df6514cf35b7
author: oompah
created: 2026-05-20T08:01:44Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 55, Tool calls: 55
- Tokens: 2.6M in / 29.3K out [2.7M total]
- Cost: $0.0000
- Exit: normal, Duration: 23m 51s
- Log: rogers-irh__20260520T073807Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4467-b0b1-7d19-a592-779c82b75bd5
author: oompah
created: 2026-05-20T08:01:47Z

Agent completed successfully in 1431s (2673749 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
