---
id: TASK-44
title: 'CRIT-6: All GitHub issue acceptance criteria copied into epic task description'
status: Done
assignee: []
created_date: 2026-05-20 05:22
updated_date: 2026-05-20 10:09
labels:
- rodgers:parent=rogers-ykp
- rodgers:type=feature-bug
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-lsw
  state: closed
  parent_id: rogers-ykp
  dependencies: []
  branch_name: rogers-lsw
  target_branch: null
  url: null
  created_at: '2026-05-20T05:22:22Z'
  updated_at: '2026-05-20T10:09:41Z'
  closed_at: '2026-05-20T10:09:35Z'
parent: TASK-4
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/feature-bug-plan.md §Task Breakdown / Epic Task → Acceptance Criteria CRIT-6

WHAT TO DO
Implement epic task description including ALL GitHub issue acceptance criteria copied verbatim.

Create/modify files:
- src/feature_bug/breakdown.rs - Epic task description generation
- src/feature_bug/mod.rs - Extract acceptance criteria from issue
- src/github/client.rs - Fetch issue comments for criteria

Epic task description must include:
- Plan: plans/feature-bug-plan.md §Task Breakdown
- GitHub Issue: #<number>
- Full acceptance criteria from issue (Rodgers-generated + human-modified)
- LLM-summarized 'What and Why' from issue

WHY
Epic task is work tracking root. Acceptance criteria define done. Copying ensures alignment.

HOW TO VERIFY
- Unit test: Epic description has Plan reference
- Unit test: GitHub issue number linked
- Unit test: ALL acceptance criteria copied (Rodgers + human)
- Unit test: What/Why summarized from issue
- Integration test: Ready-for-work → epic with criteria
- Manual: Apply ready-for-work, verify epic task

EDGE CASES AND PITFALLS
- Criteria in issue body vs comments - get all
- Rodgers-generated criteria marked
- Human-modified criteria preserved
- No criteria yet - note 'pending human review'
- Epic task type=epic, status=deferred initially
- Parent for all child tasks

PROJECT-SPECIFIC TERMINOLOGY
- 'Epic task': Top-level work tracking (type=epic)
- 'Acceptance criteria': Testable done conditions
- 'Rodgers-generated': Criteria Rodgers drafted
- 'Human-modified': Criteria human added/changed
- 'discovered-from': Link to GitHub issue
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e44c9-c1e0-74f5-89f2-b91795e9f00b
author: oompah
created: 2026-05-20T09:48:54Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44c9-e1cf-71b6-ad09-7e3c291b545b
author: oompah
created: 2026-05-20T09:49:02Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44cb-5c9e-72e4-baab-6ce54f0ee745
author: oompah
created: 2026-05-20T09:50:39Z

Understanding: Implementing CRIT-6 (epic task description including ALL GitHub issue acceptance criteria). I need to: (1) Create src/github/client.rs for fetching issue comments, (2) Enhance src/feature_bug/mod.rs to extract acceptance criteria from both body and comments, distinguishing Rodgers-generated vs human-modified, (3) Update src/feature_bug/breakdown.rs to include LLM-summarized What/Why and enrich the epic description with full acceptance criteria. The epic description must link to Plan: plans/feature-bug-plan.md §Task Breakdown, reference GitHub Issue #, include all acceptance criteria, and add a What/Why summary.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44db-ae0c-7921-b3f5-f733f0a9b80d
author: oompah
created: 2026-05-20T10:08:28Z

Verification: All 207 tests pass (207 lib tests). No clippy errors. Implementation complete: (1) Created src/github/client.rs with GitHubClient for fetching issue comments, (2) Added acceptance criteria extraction to src/feature_bug/mod.rs — extracts from issue body checkboxes and Rodgers/human comments with source tracking, (3) Added What/Why summary generation to src/feature_bug/mod.rs, (4) Enhanced src/feature_bug/breakdown.rs execute_breakdown to include full acceptance criteria and What/Why in epic description, (5) Added CRIT-6 unit tests verifying Plan reference, GitHub issue number, all acceptance criteria (Rodgers+human), What/Why summary, and pending-review note when no criteria.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44dc-c3dd-75c1-918a-1fac53c232f4
author: oompah
created: 2026-05-20T10:09:39Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 65, Tool calls: 73
- Tokens: 5.8M in / 35.8K out [5.9M total]
- Cost: $0.0000
- Exit: normal, Duration: 20m 46s
- Log: rogers-lsw__20260520T094903Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44dc-ccb4-7127-bad3-978959d7afdb
author: oompah
created: 2026-05-20T10:09:42Z

Agent completed successfully in 1246s (5869563 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
