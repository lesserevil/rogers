---
id: TASK-68
title: 'CRIT-3: Create release branch and tag with semantic version'
status: To Do
assignee: []
created_date: 2026-05-20 05:26
updated_date: 2026-05-21 05:39
labels:
- asking_question
- rodgers:parent=rogers-zjm
- rodgers:type=release-management
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-7z5
  state: open
  parent_id: rogers-zjm
  dependencies: []
  branch_name: rogers-7z5
  target_branch: null
  url: null
  created_at: '2026-05-20T05:26:43Z'
  updated_at: '2026-05-21T05:39:11Z'
  closed_at: null
parent: TASK-8
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/release-management-plan.md §Release Execution → Acceptance Criteria CRIT-3

WHAT TO DO
Implement release branch creation and semantic version tagging.

Create/modify files:
- src/release/branch.rs - Release branch creation
- src/release/tag.rs - Git tag creation with semantic version
- src/release/mod.rs - Orchestrate branch + tag creation
- src/git/client.rs - Git operations (branch, tag, push)

Execution flow:
1. Compute next version from PR analysis (major/minor/patch)
2. Create branch 'release/X.Y.Z' from main (or source branch)
3. Create annotated git tag 'vX.Y.Z' with release message
4. Push both branch and tag to origin
5. All operations in task context with audit trail

WHY
Release branches stabilize code for release while main continues. Semantic version tags mark exact release points. Both needed for release workflow and backport targeting.

HOW TO VERIFY
- Unit test: Creates release/X.Y.Z branch from correct base
- Unit test: Creates annotated tag vX.Y.Z with message
- Unit test: Pushes branch and tag to origin
- Unit test: Version computation matches conventional commits
- Integration test: Full branch+tag creation after approval
- Manual: Trigger release, verify branch and tag exist on GitHub

EDGE CASES AND PITFALLS
- Branch already exists - detect and handle (collision)
- Tag already exists - detect and handle
- Push fails (permissions, network) - retry with backoff, file task on persistent failure
- Non-fast-forward push - should not happen, but handle
- Release from release branch (hotfix) - branch from release/X.Y not main
- All operations must be in task context for audit trail

PROJECT-SPECIFIC TERMINOLOGY
- 'Release branch': Long-lived branch for release stabilization (release/X.Y.Z)
- 'Semantic version tag': Annotated git tag vX.Y.Z marking release
- 'Release task': Chore task tracking release work (rodgers:type=release)
- 'Audit trail': Task records all release operations with timestamps
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e490a-3395-7c9a-9144-a9664b0d00bd
author: oompah
created: 2026-05-21T05:37:46Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e490a-36f7-78d3-ab49-0734ce96d582
author: oompah
created: 2026-05-21T05:37:47Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e490b-65e1-7be9-a745-5afd40366958
author: oompah
created: 2026-05-21T05:39:04Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/qwen/qwen3.6-35b-a3b]
- Turns: 7, Tool calls: 19
- Tokens: 129.2K in / 8.9K out [138.1K total]
- Cost: $0.0000
- Exit: ask_question, Duration: 1m 27s
- Log: rogers-7z5__20260521T053751Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e490b-7b82-7d07-866a-6c448e525daf
author: oompah
created: 2026-05-21T05:39:10Z

🤚 **Question from agent:**

The issue says to create annotated tag 'vX.Y.Z' but the plan section says 'tag X.Y.Z'. Which format should I use for the git tag?
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
