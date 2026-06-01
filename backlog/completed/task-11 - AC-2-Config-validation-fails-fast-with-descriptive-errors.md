---
id: TASK-11
title: 'AC-2: Config validation fails fast with descriptive errors'
status: Done
assignee: []
created_date: 2026-05-20 05:17
updated_date: 2026-05-20 08:39
labels:
- rodgers:parent=rogers-6ny
- rodgers:type=architecture
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-cda
  state: closed
  parent_id: rogers-6ny
  dependencies: []
  branch_name: rogers-cda
  target_branch: null
  url: null
  created_at: '2026-05-20T05:17:32Z'
  updated_at: '2026-05-20T08:39:15Z'
  closed_at: '2026-05-20T08:39:09Z'
parent: TASK-1
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/architecture-plan.md §Configuration / Configuration Schema → Acceptance Criteria AC-2

WHAT TO DO
Implement config validation that fails fast with descriptive errors.

Create/modify files:
- src/config/validation.rs - Validation logic
- src/config/schema.rs - Validation rules
- src/main.rs - Early validation on startup

Validation checks:
- config.yaml exists and valid YAML
- All required keys present
- scheduler.interval_minutes positive integer
- github.token non-empty, not placeholder
- release.active_branches non-empty if releases configured
- rogation.labels_never_bot_managed doesn't include Rodgers-required labels
- llm.* keys valid

Error messages: descriptive, actionable, show missing/invalid key.

WHY
Fast failure with clear errors saves debugging time. Descriptive = actionable.

HOW TO VERIFY
- Unit test: Missing required key → descriptive error
- Unit test: Invalid YAML → parse error with location
- Unit test: interval_minutes=0 → error
- Unit test: token=YOUR_TOKEN → warning/error
- Unit test: active_branches empty → warning
- Unit test: labels_never_bot_managed has needs-information → warning
- Integration test: Bad config, verify error message
- Manual: Various bad configs, verify errors

EDGE CASES AND PITFALLS
- Fail fast: stop at first error or collect all?
- Descriptive: key path, expected, actual
- Placeholder detection: common patterns
- Rodgers-required labels: 10 labels from architecture
- Warning vs error distinction
- Env var expansion before validation

PROJECT-SPECIFIC TERMINOLOGY
- 'Fail fast': Stop immediately on validation error
- 'Descriptive errors': Key path, expected, actual, fix hint
- 'Placeholder token': YOUR_TOKEN, ghp_sample, etc.
- 'Rodgers-required labels': 10 workflow labels
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e4430-4212-7547-82f6-00f86f7d6744
author: oompah
created: 2026-05-20T07:01:14Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4430-7c26-7d0a-b2ae-9986645a9118
author: oompah
created: 2026-05-20T07:01:29Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4432-bc03-76c4-a55d-7a6efcc32e82
author: oompah
created: 2026-05-20T07:03:56Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/nvidia/nemotron-3-ultra-preview]
- Turns: 10, Tool calls: 10
- Tokens: 97.2K in / 579 out [97.8K total]
- Cost: $0.0000
- Exit: stalled, Duration: 2m 40s
- Log: rogers-cda__20260520T070141Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4432-cecb-74af-aa6c-3bf36fa86a48
author: oompah
created: 2026-05-20T07:04:01Z

Agent stalled 1 time(s) (160s (97751 tokens)). Escalating from 'default' to 'standard'. Retrying in 10s (attempt #1)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4433-30e5-729e-b814-9a850d8f0f48
author: oompah
created: 2026-05-20T07:04:26Z

Agent dispatched (profile: standard)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4433-34bd-7372-9efa-9ab182900fec
author: oompah
created: 2026-05-20T07:04:27Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4434-8b5f-78e8-a2d4-0ad626e6c649
author: oompah
created: 2026-05-20T07:05:55Z

Understanding: Implement config validation that fails fast with descriptive errors. Need to create src/config/validation.rs and src/config/schema.rs with validation logic for config.yaml. Update src/main.rs to run validation on startup. Validation checks include: config.yaml exists/valid YAML, required keys present, scheduler.interval_minutes positive integer, github.token non-empty/not placeholder, release.active_branches non-empty if releases configured, rogation.labels_never_bot_managed doesn't include Rodgers-required labels, llm.* keys valid. Error messages should be descriptive with key path, expected, actual, and fix hint.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4437-56f1-7d59-bdef-a66ffcd739a3
author: oompah
created: 2026-05-20T07:08:58Z

Run #2 [attempt=2, profile=standard, role=standard -> InferenceAPI/nvidia/nvidia/nemotron-3-ultra-preview]
- Turns: 0, Tool calls: 17
- Tokens: 0 in / 0 out [0 total]
- Cost: $0.0000
- Exit: terminated, Duration: 4m 27s
- Log: rogers-cda__20260520T070429Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4438-b356-759b-a1fe-0cda066f8623
author: oompah
created: 2026-05-20T07:10:27Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4438-cf88-7cad-b201-e1711c5f132f
author: oompah
created: 2026-05-20T07:10:34Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e444a-4cad-72f2-bf54-77a80ecfca7a
author: oompah
created: 2026-05-20T07:29:41Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 0, Tool calls: 56
- Tokens: 0 in / 0 out [0 total]
- Cost: $0.0000
- Exit: terminated, Duration: 19m 11s
- Log: rogers-cda__20260520T071038Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e447d-fdff-7922-bc24-b2af1068afca
author: oompah
created: 2026-05-20T08:26:08Z

Retrying (attempt #15, agent: standard)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e447e-1c0b-7672-af47-ca1c42a1e7c3
author: oompah
created: 2026-05-20T08:26:16Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4488-d8cc-7129-be47-25dcc84875c7
author: oompah
created: 2026-05-20T08:38:00Z

Completion: AC-2 (Config validation) was implemented as part of parent epic rogers-6ny. Commit 6382c34 added config validation with comprehensive coverage. All 40 validation tests pass. The implementation meets all acceptance criteria: missing file yields descriptive error, malformed YAML shows location, required keys fail with key path, scheduler.interval_minutes=0 fails, placeholder tokens are detected, empty active_branches warns, and Rodgers-required labels in labels_never_bot_managed warns.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4489-fce6-7c96-a68f-b173070b5dda
author: oompah
created: 2026-05-20T08:39:14Z

Run #16 [attempt=16, profile=standard, role=standard -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 30, Tool calls: 46
- Tokens: 1.2M in / 9.6K out [1.2M total]
- Cost: $0.0000
- Exit: normal, Duration: 13m 6s
- Log: rogers-cda__20260520T082618Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e448a-044b-7808-a192-b533f43372f8
author: oompah
created: 2026-05-20T08:39:16Z

Agent completed successfully in 786s (1197368 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
