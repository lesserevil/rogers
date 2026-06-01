---
id: TASK-26
title: 'CRIT-7: Backport approval Discussion contains commit SHA, message, issue number,
  target branch'
status: Done
assignee: []
created_date: 2026-05-20 05:19
updated_date: 2026-05-20 10:18
labels:
- rodgers:parent=rogers-4qr
- rodgers:type=backport
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-3nw
  state: closed
  parent_id: rogers-4qr
  dependencies: []
  branch_name: rogers-3nw
  target_branch: null
  url: null
  created_at: '2026-05-20T05:19:49Z'
  updated_at: '2026-05-20T10:18:18Z'
  closed_at: '2026-05-20T10:18:10Z'
parent: TASK-2
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/backport-plan.md §Approval to Backport → Acceptance Criteria CRIT-7

WHAT TO DO
Implement backport approval Discussion containing commit SHA, message, issue number, target branch.

Create/modify files:
- src/backport/approval.rs - Discussion body generation
- src/backport/manager.rs - Extract data at creation time
- src/github/client.rs - create_discussion with full data

Discussion body MUST contain (extracted at creation):
- Commit SHA (full)
- Commit message (one-line)
- Source GitHub issue number
- Target release branch

Extraction: from merged commit and linked issue at Discussion creation time.

WHY
Approval discussion is approval record. Must have all context for human decision. Extracted at creation = immutable record.

HOW TO VERIFY
- Unit test: Discussion has full commit SHA
- Unit test: Discussion has commit message
- Unit test: Discussion has source issue number
- Unit test: Discussion has target branch
- Unit test: Data extracted at creation time
- Integration test: Create discussion, verify content
- Manual: Trigger backport, verify discussion

EDGE CASES AND PITFALLS
- Commit SHA: full 40-char
- Commit message: first line only
- Issue number: from linked issue
- Target branch: release/X.Y
- Don't fetch later - extract at creation
- Linked issue may have multiple - primary one

PROJECT-SPECIFIC TERMINOLOGY
- 'Backport approval Discussion': Human vote discussion
- 'Commit SHA': Full git commit hash
- 'Source issue': GitHub issue linked to commit
- 'Target branch': release/X.Y receiving backport
- 'Extracted at creation': Immutable approval record
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e44db-5415-7654-a053-5c4a264d1d2e
author: oompah
created: 2026-05-20T10:08:05Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44db-6f48-720d-942b-61241496a9c6
author: oompah
created: 2026-05-20T10:08:12Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44dd-b028-7a4a-b0a0-2b0ade800520
author: oompah
created: 2026-05-20T10:10:40Z

Agent stalled 1 time(s) (154s (358138 tokens)). Escalating from 'default' to 'standard'. Retrying in 10s (attempt #1)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44dd-cb53-790c-98cf-37489173ef77
author: oompah
created: 2026-05-20T10:10:47Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 10, Tool calls: 18
- Tokens: 354.7K in / 3.5K out [358.1K total]
- Cost: $0.0000
- Exit: stalled, Duration: 2m 34s
- Log: rogers-3nw__20260520T100820Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44de-0642-7d9c-bf6c-57663c49ad31
author: oompah
created: 2026-05-20T10:11:02Z

Agent dispatched (profile: standard)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44de-1281-7c5e-8ee5-c1dda5a5b904
author: oompah
created: 2026-05-20T10:11:05Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44de-d730-7bdc-aadd-fb7bd2245cfd
author: oompah
created: 2026-05-20T10:11:55Z

Understanding the issue: I need to verify the backport approval Discussion body contains commit SHA (full 40-char), commit message (one-line), source issue number, and target branch — all extracted at creation time. Looking at the existing code to understand what's already implemented and what tests need to be added.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44df-cf6b-72fd-9366-5d6ff21e37d5
author: oompah
created: 2026-05-20T10:12:59Z

Discovery: Analyzed the existing implementation. The backport approval Discussion body is generated in  via . This function already includes all four required elements (commit SHA, commit message, source issue number, target branch) extracted at creation time. The test  verifies some of these but doesn't have granular tests per the verification criteria. I need to add more explicit unit tests to meet the issue's verification requirements.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44e2-029a-7abf-b9ed-618bf7555548
author: oompah
created: 2026-05-20T10:15:23Z

Implementation: Added 6 CRIT-7 unit tests to src/backport/manager.rs covering all acceptance criteria:\n- test_discussion_body_has_full_commit_sha - verifies 40-char SHA presence\n- test_discussion_body_has_commit_message - verifies commit message in **Commit:** format\n- test_discussion_body_has_source_issue_number - verifies issue extracted from PR body\n- test_discussion_body_has_target_branch - verifies target branch in body\n- test_discussion_body_data_extracted_at_creation - verifies all 4 fields extracted at creation time\n- test_discussion_body_issue_reference_multiple - verifies first match is used\n\nAll 76 tests pass.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44e4-6e77-77ec-83a8-650b3e35e26f
author: oompah
created: 2026-05-20T10:18:02Z

Verification: All quality gates pass:\n- make fmt-check ✓\n- cargo build ✓\n- cargo test: 76 tests passed ✓\n- cargo clippy: only pre-existing warnings ✓\n- git push succeeded ✓\n- Branch up to date with origin ✓\n\nThe 6 new unit tests verify CRIT-7:\n- test_discussion_body_has_full_commit_sha\n- test_discussion_body_has_commit_message\n- test_discussion_body_has_source_issue_number\n- test_discussion_body_has_target_branch\n- test_discussion_body_data_extracted_at_creation\n- test_discussion_body_issue_reference_multiple
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44e4-a4b5-7c6b-8cb3-383ba2550f9d
author: oompah
created: 2026-05-20T10:18:16Z

Run #2 [attempt=2, profile=standard, role=standard -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 33, Tool calls: 40
- Tokens: 1.6M in / 9.9K out [1.7M total]
- Cost: $0.0000
- Exit: normal, Duration: 7m 14s
- Log: rogers-3nw__20260520T101108Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44e4-a58d-7124-b658-4cc7b5acfc1a
author: oompah
created: 2026-05-20T10:18:16Z

Agent completed successfully in 434s (1657291 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
