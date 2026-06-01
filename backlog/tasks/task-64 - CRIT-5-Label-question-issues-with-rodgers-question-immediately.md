---
id: TASK-64
title: 'CRIT-5: Label question issues with rodgers:question immediately'
status: To Do
assignee: []
created_date: 2026-05-20 05:26
updated_date: 2026-05-21 05:28
labels:
- asking_question
- rodgers:parent=rogers-4en
- rodgers:type=question-routing
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-7ft
  state: open
  parent_id: rogers-4en
  dependencies: []
  branch_name: rogers-7ft
  target_branch: null
  url: null
  created_at: '2026-05-20T05:26:15Z'
  updated_at: '2026-05-21T05:28:39Z'
  closed_at: null
parent: TASK-7
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/question-routing-plan.md §Step 1: Classify as Question → Acceptance Criteria CRIT-5

WHAT TO DO
Implement immediate labeling of question issues with 'rodgers:question' on triage classification.

Create/modify files:
- src/triage/router.rs - Apply label immediately on classification
- src/question_router/mod.rs - Check label on entry (idempotency)

Behavior:
- When triage classifies issue as 'question'
- Immediately apply 'rodgers:question' label via GitHub API
- Before any other question router processing
- Label persists - subsequent runs see it's already handled

WHY
Immediate labeling prevents duplicate processing. Question router checks label on entry - if present, skips (already handled). Enables idempotent question routing.

HOW TO VERIFY
- Unit test: Question classification → rodgers:question label applied
- Unit test: Label applied before question router runs
- Unit test: Second triage run sees label, skips processing
- Unit test: Label applied via GitHub API (apply_labels tool)
- Integration test: Full triage→label→route flow
- Manual: Create question issue, run triage, verify label

EDGE CASES AND PITFALLS
- GitHub API failure applying label - log, retry next run
- Issue already has label - idempotent, no error
- Bot issues get bot_labels, skip triage entirely
- Non-question issues NEVER get this label (CRIT-6)
- Label must be applied atomically with classification

PROJECT-SPECIFIC TERMINOLOGY
- 'rodgers:question': Label marking issue as routed to question workflow
- 'Idempotent question routing': Label prevents duplicate processing
- 'Triage classification': plans/triage-workflow-plan.md §Top-Level Classification
- 'Question router entry': plans/question-routing-plan.md §Step 1
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e48f1-eb0c-7536-a54b-660b7bc3bf74
author: oompah
created: 2026-05-21T05:11:14Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48f1-f510-7b33-9f5d-ba6677a5305c
author: oompah
created: 2026-05-21T05:11:17Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48f3-dc4f-7dd6-a3d3-b83a153d921a
author: oompah
created: 2026-05-21T05:13:22Z

Agent stalled 1 time(s) (132s (270687 tokens)). Escalating from 'default' to 'standard'. Retrying in 10s (attempt #1)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48f3-e95f-7569-a544-d17d4bd0fc22
author: oompah
created: 2026-05-21T05:13:25Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/qwen/qwen3.6-35b-a3b]
- Turns: 10, Tool calls: 27
- Tokens: 260.4K in / 10.3K out [270.7K total]
- Cost: $0.0000
- Exit: stalled, Duration: 2m 12s
- Log: rogers-7ft__20260521T051122Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48f4-5a19-7140-98fd-263cfd2dd67e
author: oompah
created: 2026-05-21T05:13:54Z

Agent dispatched (profile: standard)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48f4-5de5-7403-bf9b-daeb32c50ca0
author: oompah
created: 2026-05-21T05:13:55Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48f5-c998-77a4-9e1e-fdee9914bdbc
author: oompah
created: 2026-05-21T05:15:28Z

Run #2 [attempt=2, profile=standard, role=standard -> InferenceAPI/nvidia/qwen/qwen3.6-27b]
- Turns: 11, Tool calls: 22
- Tokens: 221.2K in / 1.0K out [222.2K total]
- Cost: $0.0000
- Exit: stalled, Duration: 1m 37s
- Log: rogers-7ft__20260521T051358Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48f5-cbd9-7e03-aa86-46f4ebd7f065
author: oompah
created: 2026-05-21T05:15:29Z

Agent stalled 2 time(s) (97s (222213 tokens)). Escalating from 'standard' to 'deep'. Retrying in 20s (attempt #2)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48f6-4a6a-7bd1-b31b-23592d1dfeb9
author: oompah
created: 2026-05-21T05:16:01Z

Retrying (attempt #2, agent: deep)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48f6-5d73-7d38-a858-84f4c4aaae25
author: oompah
created: 2026-05-21T05:16:06Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48f6-7d19-7358-8508-24086018e46b
author: oompah
created: 2026-05-21T05:16:14Z

Understanding: I need to implement immediate labeling of question-classified issues with 'rodgers:question' label. Files to modify: src/triage/router.rs (apply label on classification) and src/question_router/mod.rs (check label on entry for idempotency). Will explore codebase first.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48f7-77b5-73d2-b795-44761cebf177
author: oompah
created: 2026-05-21T05:17:18Z

Agent completed successfully in 78s (134389 tokens)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48f7-8a07-7641-8e08-52d31854d11e
author: oompah
created: 2026-05-21T05:17:23Z

Run #3 [attempt=3, profile=deep, role=deep -> InferenceAPI/nvidia/moonshotai/kimi-k2.6]
- Turns: 11, Tool calls: 17
- Tokens: 133.5K in / 848 out [134.4K total]
- Cost: $0.0000
- Exit: normal, Duration: 1m 18s
- Log: rogers-7ft__20260521T051608Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4901-625c-76b7-9f50-b06e0a4e6010
author: oompah
created: 2026-05-21T05:28:08Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4901-7eaa-7e11-a37b-b697006fe955
author: oompah
created: 2026-05-21T05:28:15Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4901-ce81-7d7f-a427-7020976f4498
author: oompah
created: 2026-05-21T05:28:36Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/qwen/qwen3.6-35b-a3b]
- Turns: 1, Tool calls: 1
- Tokens: 10.0K in / 154 out [10.1K total]
- Cost: $0.0000
- Exit: ask_question, Duration: 27s
- Log: rogers-7ft__20260521T052831Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4901-d07d-778d-bceb-a242879ceccc
author: oompah
created: 2026-05-21T05:28:36Z

🤚 **Question from agent:**

This is my first turn — I can explore the codebase and implement the feature without asking questions.
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
