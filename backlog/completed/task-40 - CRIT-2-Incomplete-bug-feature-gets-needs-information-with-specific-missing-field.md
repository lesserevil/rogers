---
id: TASK-40
title: 'CRIT-2: Incomplete bug/feature gets needs-information with specific missing
  fields'
status: Done
assignee: []
created_date: 2026-05-20 05:21
updated_date: 2026-05-20 08:25
labels:
- rodgers:parent=rogers-ykp
- rodgers:type=feature-bug
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-4n9
  state: closed
  parent_id: rogers-ykp
  dependencies: []
  branch_name: rogers-4n9
  target_branch: null
  url: null
  created_at: '2026-05-20T05:21:51Z'
  updated_at: '2026-05-20T08:25:56Z'
  closed_at: '2026-05-20T08:25:50Z'
parent: TASK-4
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/feature-bug-plan.md §Completeness Check / Requirements Not Met → Acceptance Criteria CRIT-2

WHAT TO DO
Implement needs-information with SPECIFIC missing fields (not generic) for incomplete bug/feature.

Create/modify files:
- src/feature_bug/completeness.rs - Identify specific missing fields
- src/feature_bug/mod.rs - Post targeted needs-information comment
- src/llm/prompts.rs - LLM prompt for specific field requests

Behavior:
- Check each required field individually
- For each missing: generate specific request
- Bug missing: Environment, Steps, Expected, Actual (individually)
- Feature missing: Use Case, Proposed Behavior, Acceptance Criteria
- Apply needs-information label
- Comment requests ONLY missing fields
- Wait for requestor response

WHY
Generic 'need more info' wastes cycles. Specific requests get specific answers faster.

HOW TO VERIFY
- Unit test: Bug missing Environment → requests only Environment
- Unit test: Bug missing Steps + Expected → requests both specifically
- Unit test: Feature missing Acceptance Criteria → requests only that
- Unit test: No generic 'please provide more details'
- Unit test: needs-information label applied
- Integration test: Incomplete bug → specific requests
- Manual: File bug missing fields, verify specific requests

EDGE CASES AND PITFALLS
- Partial template use - only ask for empty mapped fields
- Freeform issue - LLM identifies missing from content
- Multiple missing - list all specifically
- Requestor responds - restart completeness check
- Stale policy: 2 runs ping, 2 more close (14/28 days)

PROJECT-SPECIFIC TERMINOLOGY
- 'Specific missing fields': Individual field requests, not generic
- 'needs-information': Label + state awaiting requestor response
- 'Template mapping': rogers-agw semantic field mapping
- 'Stale policy': Auto-close after no response
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e4468-195e-7581-b9ed-5e20892df4d1
author: oompah
created: 2026-05-20T08:02:13Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4468-3d69-7a49-9579-d7630ba90b50
author: oompah
created: 2026-05-20T08:02:23Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4469-e3d6-70f4-ae42-c5c521dd3325
author: oompah
created: 2026-05-20T08:04:11Z

Understanding: I need to implement targeted needs-information comments that request ONLY the specific missing fields for incomplete bugs/features, rather than generic need more info responses. Files to modify: src/feature_bug/completeness.rs (identify missing fields), src/feature_bug/mod.rs (post comment), src/llm/prompts.rs (LLM prompt). Bug fields: Environment, Steps, Expected, Actual. Feature fields: Use Case, Proposed Behavior, Acceptance Criteria.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e446d-e38a-7d5e-97ce-0a7d566e6231
author: oompah
created: 2026-05-20T08:08:33Z

Discovery: The completeness logic already exists in src/feature_bug/completeness.rs (check_bug_completeness/check_feature_completeness with field-level detection). The issue asks for src/llm/prompts.rs - an LLM integration layer for freeform issues. Need to: 1) Create src/llm/prompts.rs with prompts, 2) Add unit tests verifying only specific missing fields are requested.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e447b-4445-7425-9d2c-e9d31ec43d8d
author: oompah
created: 2026-05-20T08:23:10Z

Implementation: Created src/llm/prompts.rs with LLM prompts for freeform issue analysis, plus additional unit tests in completeness.rs verifying specific field requests. Tests now cover: bug missing Environment only, bug missing Steps+Expected, feature missing Acceptance Criteria only, no generic phrases, and needs-information label application.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e447d-1cd5-7d33-82fc-239b89466e70
author: oompah
created: 2026-05-20T08:25:11Z

Verification: All 54 tests pass. cargo build, fmt-check, and clippy all pass (except pre-existing dead_code warnings). Implemented: 1) src/llm/prompts.rs with LLM prompts for freeform issues, 2) Unit tests for specific missing field requests per CRIT-2 acceptance criteria.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e447d-c75c-71d5-a7e9-56988c134410
author: oompah
created: 2026-05-20T08:25:54Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 42, Tool calls: 49
- Tokens: 1.4M in / 12.3K out [1.4M total]
- Cost: $0.0000
- Exit: normal, Duration: 23m 39s
- Log: rogers-4n9__20260520T080225Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e447d-c7aa-7990-900e-3395316db336
author: oompah
created: 2026-05-20T08:25:54Z

Agent completed successfully in 1419s (1445501 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
