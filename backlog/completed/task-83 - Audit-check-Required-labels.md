---
id: TASK-83
title: 'Audit check: Required labels'
status: Done
assignee: []
created_date: 2026-05-20 05:40
updated_date: 2026-05-21 05:03
labels:
- rodgers:type=init
- feature
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: feature
tasks:
  id: rogers-zql.4
  state: closed
  parent_id: rogers-zql
  dependencies: []
  branch_name: rogers-zql.4
  target_branch: null
  url: null
  created_at: '2026-05-20T05:40:49Z'
  updated_at: '2026-05-21T05:03:18Z'
  closed_at: '2026-05-21T05:03:18Z'
parent: TASK-5
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/init-plan.md §Audit Checks / 2. Required Labels

WHAT TO DO
- Create src/checks/labels.rs with LabelsCheck struct
- Implement check(github: &GitHubClient, owner, repo) -> CheckResult
- Fetch all repository labels via GitHub API
- Compare against RODGERS_REQUIRED_LABELS from labels.rs
- Return CheckResult with:
  - severity: Blocker if any required label missing
  - description: list of missing labels
  - fixability: Auto (can create via GitHub API with --fix)
  - fix_instructions: labels that will be created with --fix

WHY
Rodgers requires specific labels for its triage workflow. This is a blocker check but auto-fixable.

HOW TO VERIFY
- Unit test: mock label list responses (all present, some missing, none present)
- Verify missing labels correctly identified
- Verify fixability is Auto

EDGE CASES AND PITFALLS
- Label comparison should be case-insensitive
- GitHub API returns color as hex without # prefix
- Labels might exist with wrong color/description - should we warn?
- Pagination: repos can have many labels
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e442c-0475-7ab6-98cd-6d846f346f9d
author: oompah
created: 2026-05-20T06:56:36Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e442c-0ad0-7807-8a3d-7940f564cfb5
author: oompah
created: 2026-05-20T06:56:38Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e442d-6ed5-770a-94f3-a339b7b877cf
author: oompah
created: 2026-05-20T06:58:09Z

Agent stalled 1 time(s) (100s (97075 tokens)). Escalating from 'default' to 'standard'. Retrying in 10s (attempt #1)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e442d-8c52-7656-96d2-ee329931a8dc
author: oompah
created: 2026-05-20T06:58:16Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/nvidia/nemotron-3-ultra-preview]
- Turns: 10, Tool calls: 10
- Tokens: 96.5K in / 624 out [97.1K total]
- Cost: $0.0000
- Exit: stalled, Duration: 1m 40s
- Log: rogers-zql.4__20260520T065642Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e442d-d56a-7561-b8de-391cb2bfcca8
author: oompah
created: 2026-05-20T06:58:35Z

Agent dispatched (profile: standard)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e442d-d6ca-79e3-ad20-7ff0b206a9b3
author: oompah
created: 2026-05-20T06:58:35Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e442d-fa6f-7963-a56a-e38f203cb27d
author: oompah
created: 2026-05-20T06:58:44Z

Starting work on Required Labels audit check. First exploring the codebase to understand the existing architecture.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e442f-797d-7bea-a917-e744ab9fb8a2
author: oompah
created: 2026-05-20T07:00:23Z

Run #2 [attempt=2, profile=standard, role=standard -> InferenceAPI/nvidia/nvidia/nemotron-3-ultra-preview]
- Turns: 11, Tool calls: 11
- Tokens: 110.3K in / 754 out [111.1K total]
- Cost: $0.0000
- Exit: stalled, Duration: 1m 48s
- Log: rogers-zql.4__20260520T065837Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e442f-7b70-75dd-9b13-e07199606294
author: oompah
created: 2026-05-20T07:00:23Z

Agent stalled 2 time(s) (108s (111092 tokens)). Escalating from 'standard' to 'deep'. Retrying in 20s (attempt #2)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4431-2f73-7ade-b377-4102dc04deec
author: oompah
created: 2026-05-20T07:02:15Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4431-40ee-7120-bd74-f4a0d9a8f2fe
author: oompah
created: 2026-05-20T07:02:19Z

Retrying (attempt #3, agent: standard)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4431-68f5-7e72-8fd6-8fe66249d61f
author: oompah
created: 2026-05-20T07:02:29Z

Starting work on Required Labels audit check. First exploring the codebase to understand the existing architecture.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4432-9ce5-74e7-adf6-40731873c6e0
author: oompah
created: 2026-05-20T07:03:48Z

Run #4 [attempt=4, profile=standard, role=standard -> InferenceAPI/nvidia/nvidia/nemotron-3-ultra-preview]
- Turns: 11, Tool calls: 11
- Tokens: 118.8K in / 712 out [119.5K total]
- Cost: $0.0000
- Exit: stalled, Duration: 1m 47s
- Log: rogers-zql.4__20260520T070223Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4432-a462-7978-a0fd-67c76e40c66e
author: oompah
created: 2026-05-20T07:03:50Z

Agent stalled 3 time(s) (107s (119501 tokens)). Escalating from 'standard' to 'deep'. Retrying in 80s (attempt #4)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4434-2c3c-70e9-9a0b-a87608e4de32
author: oompah
created: 2026-05-20T07:05:30Z

Retrying (attempt #4, agent: deep)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4434-3c6a-7564-9e60-75d96068afd3
author: oompah
created: 2026-05-20T07:05:35Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4434-7755-7c83-96fc-a04213b418ad
author: oompah
created: 2026-05-20T07:05:50Z

Starting work on Required Labels audit check. First exploring the codebase to understand the existing architecture.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4435-aee6-7c55-bb46-215402c0d535
author: oompah
created: 2026-05-20T07:07:09Z

Run #5 [attempt=5, profile=deep, role=deep -> InferenceAPI/nvidia/nvidia/nemotron-3-ultra-preview]
- Turns: 11, Tool calls: 11
- Tokens: 123.6K in / 721 out [124.3K total]
- Cost: $0.0000
- Exit: stalled, Duration: 1m 41s
- Log: rogers-zql.4__20260520T070539Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4435-b9ec-7952-8f6c-b593a00cd18c
author: oompah
created: 2026-05-20T07:07:12Z

Agent stalled — no productive actions (writes/commands) for 10 consecutive turns (101s (124289 tokens)). Retrying in 160s (attempt #5)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48e0-bbfc-7688-af48-789a25ee9161
author: oompah
created: 2026-05-21T04:52:28Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48e0-dad1-73a3-bf08-703e01d315c2
author: oompah
created: 2026-05-21T04:52:36Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48e1-dd24-7d79-a66a-f2420bd12bb5
author: oompah
created: 2026-05-21T04:53:42Z

## Understanding

I've read the plan and the existing codebase. Here's my understanding and plan:

**What Rodgers needs:** A  that audits whether all 9 required labels exist in the repository. It's a blocker check (fails if any label missing) but auto-fixable (can create missing labels via GitHub API).

**Existing architecture I found:**
-  - defines  trait, , , 
-  - example implementation showing the pattern to follow
-  - defines  (9 labels with colors and descriptions)
-  -  already handles pagination
-  - has  for the --fix path, but no dedicated LabelsCheck in the checks module

**My plan:**
1. Create  with  implementing 
2. Case-insensitive label comparison (GitHub names may differ in case)
3. Update  to export 
4. Wire LabelsCheck into the init flow ( and )
5. Add unit tests with wiremock mocking (all present, some missing, none present)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48ea-f947-7632-a8d4-30f74fb6b7b4
author: oompah
created: 2026-05-21T05:03:39Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/qwen/qwen3.6-35b-a3b]
- Turns: 0, Tool calls: 75
- Tokens: 0 in / 0 out [0 total]
- Cost: $0.0000
- Exit: terminated, Duration: 11m 8s
- Log: rogers-zql.4__20260521T045240Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
