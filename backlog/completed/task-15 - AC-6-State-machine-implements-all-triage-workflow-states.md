---
id: TASK-15
title: 'AC-6: State machine implements all triage workflow states'
status: Done
assignee: []
created_date: 2026-05-20 05:18
updated_date: 2026-05-20 09:09
labels:
- rodgers:parent=rogers-6ny
- rodgers:type=architecture
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-42x
  state: closed
  parent_id: rogers-6ny
  dependencies: []
  branch_name: rogers-42x
  target_branch: null
  url: null
  created_at: '2026-05-20T05:18:07Z'
  updated_at: '2026-05-20T09:09:16Z'
  closed_at: '2026-05-20T09:09:09Z'
parent: TASK-1
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/architecture-plan.md §System Components / Triage Engine → plans/triage-workflow-plan.md → Acceptance Criteria AC-6

WHAT TO DO
Implement state machine with all triage workflow states from triage-workflow-plan.md.

Create/modify files:
- src/triage/state_machine.rs - State machine implementation
- src/triage/states.rs - State definitions
- src/triage/transitions.rs - Transition logic
- plans/triage-workflow-plan.md - State diagram (source of truth)

States (from plan mermaid):
- NEW_UNCLASSIFIED
- BUG, FEATURE, QUESTION
- BUG_INCOMPLETE, FEATURE_INCOMPLETE, QUESTION_INCOMPLETE
- NEEDS_INFO
- STALE
- SEARCH_DOCS
- DOC_FOUND, DOC_GAP
- READY_FOR_REVIEW
- WILL_NOT_DO, READY_FOR_WORK
- IN_PROGRESS
- CLOSE_ISSUE

Transitions: all arrows in mermaid diagram.
Human gates: will-not-do, ready-for-work (never auto).

WHY
State machine = triage logic. Complete implementation = predictable behavior. Human gates = control.

HOW TO VERIFY
- Unit test: All states defined
- Unit test: All transitions implemented
- Unit test: Human gates never auto
- Unit test: Bot handling (bot_labels, skip)
- Unit test: Stale progression (14/28 days)
- Integration test: Issues flow through states
- Manual: Trace issue through states

EDGE CASES AND PITFALLS
- State per issue per run (re-evaluate always)
- Human label priority: will-not-do > ready-for-work
- Compound issues: split
- Epic-scale: breakdown at ready-for-work
- Unknown labels: ignore, use content
- Mermaid diagram = source of truth

PROJECT-SPECIFIC TERMINOLOGY
- 'State machine': Triage workflow logic
- 'Human gates': will-not-do, ready-for-work
- 'Bot handling': bot_labels, skip triage
- 'Re-evaluate always': No state assumption
- 'Mermaid diagram': plans/triage-workflow-plan.md
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e448b-3210-790b-ae7a-d91e5581cdc1
author: oompah
created: 2026-05-20T08:40:34Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e448b-636e-7d6e-8c40-c11ef27173e3
author: oompah
created: 2026-05-20T08:40:46Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e448e-8eb3-7361-8956-a65f93762c03
author: oompah
created: 2026-05-20T08:44:14Z

## Understanding

The issue asks me to implement/verify a complete state machine for the triage workflow. Looking at the existing codebase in `src/triage/state_machine.rs`, there is substantial implementation already in place with:

**States defined:**
- NEW_UNCLASSIFIED ✅
- BUG_INCOMPLETE, FEATURE_INCOMPLETE, QUESTION_INCOMPLETE ✅
- NEEDS_INFO ✅
- STALE ✅
- SEARCH_DOCS ✅
- DOC_FOUND ✅
- DOC_GAP ✅
- READY_FOR_REVIEW ✅
- WILL_NOT_DO ✅
- READY_FOR_WORK ✅
- IN_PROGRESS ✅
- CLOSED ✅

**What needs enhancement:**
1. Add FILE_EPIC_TASKS as explicit transition state between READY_FOR_WORK and IN_PROGRESS (per acceptance criteria)
2. Add comprehensive tests for human gates never auto-transitioning
3. Add tests for bot handling scenarios
4. Add tests for stale progression (14/28 day thresholds)

Proceeding to implementation.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4496-d736-7185-af5a-09f499f41b39
author: oompah
created: 2026-05-20T08:53:17Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 0, Tool calls: 29
- Tokens: 0 in / 0 out [0 total]
- Cost: $0.0000
- Exit: terminated, Duration: 12m 42s
- Log: rogers-42x__20260520T084050Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4497-21d5-73fd-9924-5f09c1567839
author: oompah
created: 2026-05-20T08:53:36Z

Agent dispatched (profile: standard)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4497-25b5-79f2-8233-b96d5bf3661a
author: oompah
created: 2026-05-20T08:53:37Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44a5-22ff-71cd-a5cc-7f234ead31c4
author: oompah
created: 2026-05-20T09:08:54Z

## Implementation Complete

Implemented the complete triage workflow state machine as required by AC-6.

**States implemented (all 18 from Mermaid diagram):**
- NEW_UNCLASSIFIED, BUG, FEATURE, QUESTION
- BUG_INCOMPLETE, FEATURE_INCOMPLETE, QUESTION_INCOMPLETE  
- NEEDS_INFO, STALE, SEARCH_DOCS
- DOC_FOUND, DOC_GAP, FILE_EPIC_TASKS
- READY_FOR_REVIEW, WILL_NOT_DO, READY_FOR_WORK
- IN_PROGRESS, CLOSED

**Key fixes:**
- Fixed bug where complete issues went to intermediate states - now go directly to ReadyForReview (bug/feature) or SearchDocs (question)
- Fixed transition_with_classification to properly route based on completeness

**Tests added (18 new tests, 39 total state machine tests, 161 total):**
- test_all_states_defined: All 18 states verified
- test_all_transitions_implemented: All Mermaid arrows verified
- test_human_gates_*: Human gates never auto-transition
- test_stale_progression_*: 14/28 day thresholds
- test_sequential_transitions_bug_flow: End-to-end bug workflow

All 161 tests pass.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44a5-6eab-7751-9125-ed60ac081539
author: oompah
created: 2026-05-20T09:09:13Z

Agent completed successfully in 940s (3130644 tokens)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44a5-7154-70f3-b284-8115432b6371
author: oompah
created: 2026-05-20T09:09:14Z

Run #2 [attempt=2, profile=standard, role=standard -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 49, Tool calls: 53
- Tokens: 3.1M in / 20.5K out [3.1M total]
- Cost: $0.0000
- Exit: normal, Duration: 15m 40s
- Log: rogers-42x__20260520T085340Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
