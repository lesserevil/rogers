---
id: TASK-35
title: 'AC-4: rogers doctor fails fast on config and auth before tasks/repo'
status: Done
assignee: []
created_date: 2026-05-20 05:21
updated_date: 2026-05-20 08:43
labels:
- rodgers:parent=rogers-atj
- rodgers:type=doctor
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-96e
  state: closed
  parent_id: rogers-atj
  dependencies: []
  branch_name: rogers-96e
  target_branch: null
  url: null
  created_at: '2026-05-20T05:21:03Z'
  updated_at: '2026-05-20T08:43:53Z'
  closed_at: '2026-05-20T08:43:39Z'
parent: TASK-3
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/doctor-plan.md §Output Format / Drift Flow → Acceptance Criteria AC-4

WHAT TO DO
Implement doctor fail-fast on config and auth before checking tasks or repo.

Create/modify files:
- src/doctor/mod.rs - Category execution order
- src/doctor/categories.rs - Config and auth categories first
- src/config/validation.rs - Config validation (fail fast)
- src/github/auth.rs - Auth validation (fail fast)

Execution order:
1. config category - validate config.yaml, exit on fail
2. auth category - validate GitHub token, exit on fail
3. tasks category - database connectivity
4. plans category - plan files
5. repo category - GitHub repo state
6. drift category - state drift (most expensive)

Fail fast: config or auth fail → exit immediately, skip remaining.

WHY
Config/auth are prerequisites. No point checking tasks/repo if can't connect. Fast feedback.

HOW TO VERIFY
- Unit test: Invalid config → exit after config, no auth/tasks/repo
- Unit test: Invalid auth → exit after auth, no tasks/repo
- Unit test: Valid config/auth → continues to tasks
- Integration test: Bad config, run doctor, verify fast exit
- Manual: Bad token, run doctor, verify fast exit

EDGE CASES AND PITFALLS
- Config validation: YAML parse, required keys, schema
- Auth validation: token valid, scopes, repo access
- Both must fail fast
- --verbose doesn't change order
- --only category skips to that category
- Drift flow mermaid shows config/auth first

PROJECT-SPECIFIC TERMINOLOGY
- 'Fail fast': Stop on prerequisite failure
- 'Config category': config.yaml validation
- 'Auth category': GitHub token validation
- 'Prerequisites': Config and auth required for all else
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e4485-8805-7cb6-a9f8-dd9fba5c7692
author: oompah
created: 2026-05-20T08:34:22Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4485-9f60-767d-83d6-58826f51fa4b
author: oompah
created: 2026-05-20T08:34:28Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4485-e2dc-7521-b2d0-7ebed28670d4
author: oompah
created: 2026-05-20T08:34:46Z

**Understanding:** Implementing fail-fast in `rogers doctor` command - config and auth categories must execute first and exit immediately on failure, before checking tasks/repo. This provides fast feedback when prerequisites are missing.

**Plan:**
1. Explore existing doctor module structure (src/doctor/mod.rs, src/doctor/categories.rs)
2. Read existing config validation (src/config/validation.rs) 
3. Read existing auth validation (src/github/auth.rs)
4. Modify category execution order to put config/auth first
5. Implement fail-fast behavior (exit immediately on config/auth failure)
6. Add unit tests for the new behavior
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4486-9be6-7cbc-bbfe-5903d007c85b
author: oompah
created: 2026-05-20T08:35:33Z

Agent completed successfully in 70s (148088 tokens)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4486-9f54-7c1b-9559-86b2cc9c0e6c
author: oompah
created: 2026-05-20T08:35:34Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 9, Tool calls: 14
- Tokens: 147.0K in / 1.1K out [148.1K total]
- Cost: $0.0000
- Exit: normal, Duration: 1m 10s
- Log: rogers-96e__20260520T083430Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4486-b5c9-7cf2-b1b7-f2030a822525
author: oompah
created: 2026-05-20T08:35:40Z

Agent completed without closing this issue (70s (148088 tokens)). Escalating from 'default' to 'standard'. Retrying in 10s (1/3).
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4487-07db-7ece-b74d-d87eed911b5e
author: oompah
created: 2026-05-20T08:36:01Z

Agent dispatched (profile: standard)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4487-0fbb-700d-85b0-40d6c0a61882
author: oompah
created: 2026-05-20T08:36:03Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4487-5b41-7234-b550-aa9645cae889
author: oompah
created: 2026-05-20T08:36:22Z

**Understanding:** Implementing fail-fast in rogers doctor command - config and auth categories must execute first and exit immediately on failure, before checking tasks/repo. This provides fast feedback when prerequisites are missing.

**Plan:**
1. Explore existing doctor module structure (src/doctor/mod.rs, src/doctor/categories.rs)
2. Read existing config validation (src/config/validation.rs)
3. Read existing auth validation (src/github/auth.rs)
4. Modify category execution order to put config/auth first
5. Implement fail-fast behavior (exit immediately on config/auth failure)
6. Add unit tests for the new behavior
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4488-8e84-7be0-bdac-e8a5ddbc041c
author: oompah
created: 2026-05-20T08:37:41Z

**Discovery:** Found key code in src/main.rs:run_doctor_checks(). Current implementation runs ALL categories regardless of config/auth failures. The FAIL_FAST_CATEGORIES constant exists in src/doctor/mod.rs but is NOT USED. Categories are executed in correct order (config first), but fail-fast behavior is NOT implemented - it continues after failures.

**Key finding:** No changes needed to categories.rs or validation.rs - the config/auth check functions exist and work. Only need to modify run_doctor_checks() in main.rs to add fail-fast logic after config and auth checks.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e448b-72a2-71dd-aea0-3bcdd3615431
author: oompah
created: 2026-05-20T08:40:50Z

**Implementation:** Modified src/main.rs:run_doctor_checks() to implement AC-4 fail-fast behavior.\n\n**Changes:**\n1. After config check fails → return immediately with partial result (auth/tasks/plans/repo/drift skipped)\n2. After auth check fails → return immediately with partial result (tasks/plans/repo/drift skipped)\n3. Only continue to remaining categories if config AND auth both pass\n4. Added CategoryStatus to imports\n5. Updated docstring to reflect fail-fast behavior\n\n**Unit tests added:**\n- test_fail_fast_invalid_config: Invalid config → only config runs, others skipped\n- test_fail_fast_invalid_auth: Invalid auth → config+auth run, others skipped  \n- test_continues_past_auth_when_valid: Valid config+auth → continues to later categories\n- test_only_category_runs_specific_category: --only flag bypasses fail-fast chain\n\nAll 29 tests pass.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e448e-2563-76a2-9e12-5019f155ef6d
author: oompah
created: 2026-05-20T08:43:47Z

Run #2 [attempt=2, profile=standard, role=standard -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 37, Tool calls: 40
- Tokens: 1.2M in / 8.4K out [1.2M total]
- Cost: $0.0000
- Exit: normal, Duration: 7m 47s
- Log: rogers-96e__20260520T083606Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e448e-2cde-72e1-b090-8c445b174383
author: oompah
created: 2026-05-20T08:43:49Z

Agent completed successfully in 467s (1221295 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
