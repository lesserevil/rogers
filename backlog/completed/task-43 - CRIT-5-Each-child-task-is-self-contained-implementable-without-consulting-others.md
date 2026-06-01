---
id: TASK-43
title: 'CRIT-5: Each child task is self-contained, implementable without consulting
  others'
status: Done
assignee: []
created_date: 2026-05-20 05:22
updated_date: 2026-05-20 09:47
labels:
- rodgers:parent=rogers-ykp
- rodgers:type=feature-bug
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-bxt
  state: closed
  parent_id: rogers-ykp
  dependencies: []
  branch_name: rogers-bxt
  target_branch: null
  url: null
  created_at: '2026-05-20T05:22:15Z'
  updated_at: '2026-05-20T09:47:16Z'
  closed_at: '2026-05-20T09:46:52Z'
parent: TASK-4
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/feature-bug-plan.md §Task Breakdown / Child Backlog → Acceptance Criteria CRIT-5 & AGENTS.md §Backlog must stand alone

WHAT TO DO
Implement child task breakdown ensuring each task is self-contained and implementable without consulting others.

Create/modify files:
- src/feature_bug/breakdown.rs - Child task generation with standalone criteria
- src/feature_bug/mod.rs - Validate task standalone-ness
- src/llm/prompts.rs - LLM prompt for standalone task generation

Standalone criteria (AGENTS.md):
- WHAT TO DO: Concrete files, packages, functions, commands
- WHY: User-visible behavior, constraint, design rule
- HOW TO VERIFY: Test, command, observable result
- EDGE CASES: Non-obvious constraints
- TERMINOLOGY: Project-specific terms explained

Breakdown rules (plan):
- Single codebase part per task (CLI, UI, API, DB, config)
- No 'and then...' descriptions
- One acceptance criterion per task OR cohesive concern
- Edge cases in relevant child, not epic
- Naive junior dev can implement from task alone

WHY
Backlog.md tasks are work units for developers. Standalone = no coordination needed, parallelizable, clear ownership.

HOW TO VERIFY
- Unit test: Each child has all 5 sections (WHAT, WHY, HOW, EDGE, TERMS)
- Unit test: Single codebase part per task
- Unit test: No compound 'and then' descriptions
- Unit test: Junior dev simulation - can implement from task alone
- Integration test: Breakdown produces standalone tasks
- Manual: Review generated tasks for completeness

EDGE CASES AND PITFALLS
- LLM may produce compound tasks - validate and split
- Epic-scale detection must be accurate
- Cross-cutting concerns - separate tasks per area
- Acceptance criteria mapping - one per task or cohesive group
- Terminology: explain project-specific terms inline

PROJECT-SPECIFIC TERMINOLOGY
- 'Standalone task': Complete context in task description
- 'Logical unit of work': Single codebase part, no 'and then'
- 'Naive junior dev': Can write code, run tools, read repo docs
- 'Epic breakdown': LLM analysis → epic + children
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e44af-f169-7c21-b16d-3639820dc464
author: oompah
created: 2026-05-20T09:20:42Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44b0-3123-7bd2-9f5b-4f0f7bfd4310
author: oompah
created: 2026-05-20T09:20:58Z

Focus: Epic Planner
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44b1-93ef-7678-97fd-1e12da4c57e6
author: oompah
created: 2026-05-20T09:22:29Z

Understanding the task: Implement child task breakdown ensuring each task is self-contained and implementable without consulting others. 

Need to:
1. Enhance breakdown.rs with standalone task struct (WHAT, WHY, HOW, EDGE, TERMS sections)
2. Add validation functions in mod.rs for standalone-ness (5 sections present, no 'and then', single codebase part)
3. Enhance prompts.rs with LLM prompts for standalone task generation
4. Add unit tests for all verification requirements

Plan: Start with breakdown.rs adding StandaloneTask structure and validation logic.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44c7-3400-77db-bce7-be02115ade79
author: oompah
created: 2026-05-20T09:46:06Z

Discovery: Implementation complete. Added to breakdown.rs:
- StandaloneTask struct with 5 sections (WHAT, WHY, HOW, EDGE, TERMS)
- Validation functions for section completeness, compound patterns, single codebase part
- generate_standalone_task() for creating standalone descriptions

Added to mod.rs:
- validate_standalone_sections(), validate_no_compound_pattern(), validate_single_codebase_part()
- validate_task_standalone() for full validation
- All validation structs with descriptive feedback

Added to prompts.rs:
- STANDALONE_TASK_PROMPT for LLM task generation
- STANDALONE_VALIDATION_PROMPT for LLM validation
- TASK_SPLIT_PROMPT for splitting compound tasks

New tests cover all CRIT-5 requirements. All 170 tests passing.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44c8-0717-742b-9f95-6f616d0953f6
author: oompah
created: 2026-05-20T09:47:00Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 90, Tool calls: 98
- Tokens: 6.8M in / 42.1K out [6.8M total]
- Cost: $0.0000
- Exit: normal, Duration: 26m 17s
- Log: rogers-bxt__20260520T092108Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44c8-0f62-7449-a4dc-91ee601de11e
author: oompah
created: 2026-05-20T09:47:02Z

Agent completed successfully in 1577s (6801296 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
