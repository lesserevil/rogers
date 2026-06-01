---
id: TASK-71
title: 'CRIT-6: Config-driven release schedule and branches'
status: To Do
assignee: []
created_date: 2026-05-20 05:27
updated_date: 2026-05-21 06:24
labels:
- rodgers:parent=rogers-zjm
- rodgers:type=release-management
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-p9l
  state: deferred
  parent_id: rogers-zjm
  dependencies: []
  branch_name: rogers-p9l
  target_branch: null
  url: null
  created_at: '2026-05-20T05:27:06Z'
  updated_at: '2026-05-21T06:24:29Z'
  closed_at: null
parent: TASK-8
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/release-management-plan.md §Configuration → Acceptance Criteria CRIT-6

WHAT TO DO
Implement config-driven release schedule and branch management.

Create/modify files:
- src/config/schema.rs - Release configuration schema
- src/release/config.rs - Release config loading and validation
- config.yaml - Release configuration keys

Configuration keys (from architecture-plan.md):
- release.approval_discussion_category: GitHub Discussion category for proposals (default: Announcements)
- release.active_branches: List of active release branches for backports (e.g., [release/1.x, release/2.x])
- release.voting_window_days: Days before nudging stale proposal (default: 2)
- release.stale_threshold_days: Days before closing stale proposal (default: 7)

WHY
Config-driven releases allow projects to customize release cadence, branches, and approval flow without code changes. Different projects have different release models.

HOW TO VERIFY
- Unit test: Config schema validates all release keys
- Unit test: Default values applied when not specified
- Unit test: active_branches parsed as list
- Unit test: voting_window_days and stale_threshold_days as integers
- Integration test: Release manager uses config values
- Manual: Modify config.yaml, restart, verify config loaded

EDGE CASES AND PITFALLS
- Empty active_branches - warn but continue (backport manager inactive)
- Invalid discussion category - warn, use default
- Negative voting_window_days - validation error
- Config from rogers.yaml (repo-level) overrides config.yaml (host-level)
- Environment variable overrides for sensitive values

PROJECT-SPECIFIC TERMINOLOGY
- 'release.active_branches': Branches receiving backports (maintenance releases)
- 'approval_discussion_category': GitHub Discussion category for release/backport votes
- 'voting_window_days': Time before reminder ping
- 'stale_threshold_days': Time before closing proposal and filing revisit task
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e4926-5468-798a-8b99-884b56d3a781
author: oompah
created: 2026-05-21T06:08:29Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4926-6d1d-79de-9c29-2656605abe89
author: oompah
created: 2026-05-21T06:08:36Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4927-3246-7337-9955-a2d99e929d0a
author: oompah
created: 2026-05-21T06:09:26Z

## Understanding

**Issue:** CRIT-6: Config-driven release schedule and branches. I need to implement config-driven release configuration that allows projects to customize release cadence, branches, and approval flow via YAML config.

**Key observations from codebase exploration:**
-  already has  struct with the 4 fields (approval_discussion_category, active_branches, voting_window_days, stale_threshold_days)
-  already has  with overlapping fields
-  already has the release section with all 4 keys
- The  directory doesn't exist yet — I need to create it
- No dedicated  exists

**Plan:**
1. Create  — Release configuration schema with defaults, validation, and serde derives
2. Create  — Config loading and validation (loading from YAML, env var overrides, merging)
3. Add unit tests for schema validation, defaults, list parsing, integer validation
4. Add integration test showing release manager using config values
5. Update  (create from example)
6. Update  to export the new module
7. Update  to include the config module
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4928-9442-7023-b39a-e8783c19ab49
author: oompah
created: 2026-05-21T06:10:57Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/qwen/qwen3.6-35b-a3b]
- Turns: 15, Tool calls: 28
- Tokens: 560.6K in / 13.2K out [573.9K total]
- Cost: $0.0000
- Exit: normal, Duration: 2m 29s
- Log: rogers-p9l__20260521T060837Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4928-9adc-71d5-b824-53a7983ca706
author: oompah
created: 2026-05-21T06:10:58Z

Agent completed successfully in 149s (573850 tokens)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4928-a358-71cc-961c-5b88a8da7af3
author: oompah
created: 2026-05-21T06:11:01Z

Agent completed without closing this issue (149s (573850 tokens)). Escalating from 'default' to 'standard'. Retrying in 10s (1/3).
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4929-0e66-7e5d-b8c8-fff4e939912e
author: oompah
created: 2026-05-21T06:11:28Z

Agent dispatched (profile: standard)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4929-21f3-7dde-884e-0566ede1c8a1
author: oompah
created: 2026-05-21T06:11:33Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e492a-30ee-72e3-949d-288011aafdea
author: oompah
created: 2026-05-21T06:12:42Z

## Understanding

Continuing CRIT-6 from previous attempt. The previous agent created:
-  — Complete with ReleaseConfig/ResolvedReleaseConfig, loading, and tests
-  — Complete with merging and env override logic + tests
-  — Exists but only exports schema

**Remaining work:**
1. Add  to  (the module file exists but is not exported)
2. Create  from 
3. Build and run tests to verify everything passes
4. Commit, push, and close
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4932-0773-7a53-acfe-0678c92cba27
author: oompah
created: 2026-05-21T06:21:16Z

Run #2 [attempt=2, profile=standard, role=standard -> InferenceAPI/nvidia/qwen/qwen3.6-27b]
- Turns: 59, Tool calls: 68
- Tokens: 2.4M in / 11.6K out [2.4M total]
- Cost: $0.0000
- Exit: normal, Duration: 9m 52s
- Log: rogers-p9l__20260521T061137Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4932-0bb2-7a01-b413-64bd63cf3772
author: oompah
created: 2026-05-21T06:21:17Z

Agent completed successfully in 592s (2442865 tokens)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4932-1af4-799e-bc8d-35a330f25644
author: oompah
created: 2026-05-21T06:21:21Z

Agent completed without closing this issue (592s (2442865 tokens)). Escalating from 'standard' to 'deep'. Retrying in 20s (2/3).
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4932-b415-70de-b721-8554a3379adb
author: oompah
created: 2026-05-21T06:22:00Z

Retrying (attempt #2, agent: deep)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4932-b4cd-7c33-a211-3f620068abb3
author: oompah
created: 2026-05-21T06:22:00Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4934-d97b-700e-99d1-c5502c81e228
author: oompah
created: 2026-05-21T06:24:21Z

Agent completed successfully in 145s (523401 tokens)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4934-e46c-79c9-beeb-17b64f9b6945
author: oompah
created: 2026-05-21T06:24:24Z

Run #3 [attempt=3, profile=deep, role=deep -> InferenceAPI/nvidia/moonshotai/kimi-k2.6]
- Turns: 16, Tool calls: 27
- Tokens: 520.9K in / 2.5K out [523.4K total]
- Cost: $0.0000
- Exit: normal, Duration: 2m 25s
- Log: rogers-p9l__20260521T062204Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4934-f470-70b5-a7d1-5d26bacdf718
author: oompah
created: 2026-05-21T06:24:28Z

Agent completed 3 times without closing this issue. Deferring — needs human attention.
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
