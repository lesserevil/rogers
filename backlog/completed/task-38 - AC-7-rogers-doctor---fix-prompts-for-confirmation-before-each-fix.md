---
id: TASK-38
title: 'AC-7: rogers doctor --fix prompts for confirmation before each fix'
status: Done
assignee: []
created_date: 2026-05-20 05:21
updated_date: 2026-05-20 09:52
labels:
- rodgers:parent=rogers-atj
- rodgers:type=doctor
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-8d8
  state: closed
  parent_id: rogers-atj
  dependencies: []
  branch_name: rogers-8d8
  target_branch: null
  url: null
  created_at: '2026-05-20T05:21:30Z'
  updated_at: '2026-05-20T09:52:22Z'
  closed_at: '2026-05-20T09:52:07Z'
parent: TASK-3
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/doctor-plan.md §Drift Remediation → Acceptance Criteria AC-7

WHAT TO DO
Implement doctor --fix with interactive confirmation before each fix.

Create/modify files:
- src/doctor/fix.rs - Interactive fix implementations
- src/doctor/mod.rs - --fix flag handling
- src/doctor/drift.rs - Drift event presentation

Fix flow (per drift event):
1. Present event: issue URL, task ID, mismatch
2. Show options:
   A. Close GitHub issue to match task
   B. Reopen task and link to correct issue
   C. File new task for manual work, close orphaned task
3. Prompt for confirmation (y/n/skip)
4. On confirmation: apply fix via API
5. Next event...

Options for orphan tasks (no GitHub link):
- Attribute to existing issue
- Close task

WHY
Drift fixes are destructive. Human must confirm each. Not auto-destructive.

HOW TO VERIFY
- Unit test: --fix presents each event with options
- Unit test: Prompts for confirmation per event
- Unit test: Option A closes issue
- Unit test: Option B reopens task
- Unit test: Option C files new task
- Unit test: Skip moves to next
- Integration test: Run doctor --fix on drift, verify interactive
- Manual: Create drift, run doctor --fix, verify prompts

EDGE CASES AND PITFALLS
- Non-interactive (CI) - --fix should fail or skip
- GitHub API failure during fix - report, continue
- User cancels mid-fix - stop, report partial
- Orphan task options different
- Convention drift (AGENTS.md) - different fix flow
- Prompt format clear with issue/task identifiers

PROJECT-SPECIFIC TERMINOLOGY
- 'doctor --fix': Interactive drift remediation
- 'Confirmation prompt': Per-event y/n/skip
- 'Option A/B/C': Remediation choices from plan
- 'Orphan task': No GitHub issue link
- 'Not auto-destructive': Requires confirmation
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e44b6-e6b1-7a01-9a0a-f9c52a42e824
author: oompah
created: 2026-05-20T09:28:18Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44b7-090b-7b96-b3bc-2ffa6d07a261
author: oompah
created: 2026-05-20T09:28:27Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44b7-55c9-7ca1-af31-8e9818e5c8d8
author: oompah
created: 2026-05-20T09:28:46Z

**Understanding**: Implementing AC-7 from plans/doctor-plan.md §Drift Remediation. The feature needs to add  flag to  command that presents each drift event with options (A: close GitHub issue, B: reopen task, C: file new task/skip), prompts for y/n/skip confirmation, and applies fixes via API. Must be interactive, not auto-destructive.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44b8-a469-7d4f-b517-09b90cd688bd
author: oompah
created: 2026-05-20T09:30:12Z

**Discovery**: Found the existing codebase has all the basic structures. The  type, , and  API all exist. The  flag is defined in CLI but currently unused. I need to:
1. Create  with interactive fix logic
2. Wire  in  to use the fix module
3. Add unit tests for the new functionality
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44bb-cbcc-7926-b841-056899438efd
author: oompah
created: 2026-05-20T09:33:39Z

Agent completed successfully in 320s (575031 tokens)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44bb-d387-74ac-900c-15230b0dea62
author: oompah
created: 2026-05-20T09:33:41Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 21, Tool calls: 27
- Tokens: 565.4K in / 9.6K out [575.0K total]
- Cost: $0.0000
- Exit: normal, Duration: 5m 20s
- Log: rogers-8d8__20260520T092829Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44bb-de49-703d-bc83-72ba69a5f295
author: oompah
created: 2026-05-20T09:33:43Z

Agent completed without closing this issue (320s (575031 tokens)). Escalating from 'default' to 'standard'. Retrying in 10s (1/3).
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44c1-2115-726e-adb5-477a1983184f
author: oompah
created: 2026-05-20T09:39:28Z

Retrying (attempt #5, agent: standard)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44c1-2c55-7634-a419-7cd17aed9d0a
author: oompah
created: 2026-05-20T09:39:31Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44cb-c76a-79bc-b7e0-ee0b23170af7
author: oompah
created: 2026-05-20T09:51:06Z

**Implementation**: Fixed multiple compilation errors in the existing fix.rs and mod.rs code:

1. Added  to main.rs to make crate::github accessible
2. Removed broken atty conditional compilation (no atty feature defined), simplified is_interactive() to check CI/RODGERS_NON_INTERACTIVE env vars
3. Changed method signatures (present_event, present_options, present_orphan_options, prompt_choice) from &self to &mut self for proper output writing
4. Added FixChoice::C case in prompt_orphan_task to handle the case where option C doesn't apply to orphan tasks
5. Added url = "2.5" to Cargo.toml for GitHub URL parsing
6. Fixed test sessions to use  instead of 
7. Fixed incorrect test assertion in test_orphan_task_shows_different_options - changed "Clos" check to "GitHub issue" which only appears in standard options
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44cc-85d9-7aa0-9834-b73fb321f121
author: oompah
created: 2026-05-20T09:51:55Z

**Verification**: cargo build ✓, cargo test ✓ (46 passed), cargo fmt ✓, cargo clippy ✓ (warnings only, no errors)

**Completion**: AC-7 implemented. The doctor --fix command now:
- Presents each drift event with issue URL, task ID, and mismatch description
- Shows options A (close issue), B (reopen task), C (file new task for manual work)
- For orphan tasks (no GitHub link): shows A (attribute to existing issue), B (close task)
- Prompts for y/n/skip confirmation per event
- On confirmation, applies fix via GitHub API
- Detects non-interactive environments (CI/RODGERS_NON_INTERACTIVE) and exits with error
- Reports partial completion if user cancels mid-fix
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44cc-e262-766e-a534-6dbd7f000d75
author: oompah
created: 2026-05-20T09:52:19Z

Run #6 [attempt=6, profile=standard, role=standard -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 50, Tool calls: 54
- Tokens: 2.4M in / 14.8K out [2.4M total]
- Cost: $0.0000
- Exit: normal, Duration: 12m 49s
- Log: rogers-8d8__20260520T093934Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44cc-e2b5-7bf9-b268-f786550d76b3
author: oompah
created: 2026-05-20T09:52:19Z

Agent completed successfully in 769s (2409596 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
