---
id: TASK-49
title: 'AC-4: rogers init --fix creates missing discussion categories via GitHub API'
status: To Do
assignee: []
created_date: 2026-05-20 05:23
updated_date: 2026-05-21 04:52
labels:
- asking_question
- rodgers:parent=rogers-zql
- rodgers:type=init
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-741
  state: open
  parent_id: rogers-zql
  dependencies: []
  branch_name: rogers-741
  target_branch: null
  url: null
  created_at: '2026-05-20T05:23:22Z'
  updated_at: '2026-05-21T04:52:49Z'
  closed_at: null
parent: TASK-5
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/init-plan.md §--fix Flag Behavior → Acceptance Criteria AC-4

WHAT TO DO
Implement rogers init --fix creating missing discussion categories via GitHub API.

Create/modify files:
- src/init/fix.rs - Discussion category creation
- src/init/mod.rs - --fix flag handling
- src/github/client.rs - Discussion category API
- config.yaml - release.approval_discussion_category (default: Announcements)

Fix behavior:
- --fix flag enables auto-fix
- Check for category (config.release.approval_discussion_category)
- Create via GitHub API if missing
- Idempotent: create-if-missing
- Report created category in output

WHY
Release/backport approval needs Discussion category. Auto-create saves setup time. Same category used for both.

HOW TO VERIFY
- Unit test: Creates missing category via API
- Unit test: Uses config value (default Announcements)
- Unit test: Idempotent - re-run doesn't duplicate
- Unit test: Reports created category
- Integration test: rogers init --fix on repo without category
- Manual: Delete category, run init --fix, verify created

EDGE CASES AND PITFALLS
- GitHub API rate limit - retry
- Permission denied (need admin) - report, continue
- Category exists with different name - use existing
- GraphQL API for discussions (not REST)
- Only API-level fix - file fixes via PR
- Category used for release AND backport proposals

PROJECT-SPECIFIC TERMINOLOGY
- 'Discussion category': GitHub Discussions category for proposals
- 'approval_discussion_category': Config key for category name
- 'Release proposal': Human approval gate for releases
- 'Backport proposal': Human approval gate for backports
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e48e0-a658-7d4e-bbf8-5f411d0bca79
author: oompah
created: 2026-05-21T04:52:23Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48e0-bef9-7ba3-90a3-ebcf4a306095
author: oompah
created: 2026-05-21T04:52:29Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48e0-fb60-789c-a40f-469e96b07fdb
author: oompah
created: 2026-05-21T04:52:45Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/qwen/qwen3.6-35b-a3b]
- Turns: 2, Tool calls: 3
- Tokens: 21.7K in / 247 out [21.9K total]
- Cost: $0.0000
- Exit: ask_question, Duration: 25s
- Log: rogers-741__20260521T045232Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48e1-0383-70c7-934d-8026a46ab9ed
author: oompah
created: 2026-05-21T04:52:47Z

🤚 **Question from agent:**

The task mentions config key `config.release.approval_discussion_category` (default: Announcements), but the plan mentions `Release Proposals` category. Which is the correct default config value and key name to use?
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
