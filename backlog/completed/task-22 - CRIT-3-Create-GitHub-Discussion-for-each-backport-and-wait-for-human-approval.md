---
id: TASK-22
title: 'CRIT-3: Create GitHub Discussion for each backport and wait for human approval'
status: Done
assignee: []
created_date: 2026-05-20 05:19
updated_date: 2026-05-20 09:21
labels:
- rodgers:parent=rogers-4qr
- rodgers:type=backport
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-13t
  state: closed
  parent_id: rogers-4qr
  dependencies: []
  branch_name: rogers-13t
  target_branch: null
  url: null
  created_at: '2026-05-20T05:19:15Z'
  updated_at: '2026-05-20T09:21:05Z'
  closed_at: '2026-05-20T09:20:55Z'
parent: TASK-2
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/backport-plan.md §Approval to Backport → Acceptance Criteria CRIT-3

WHAT TO DO
Implement GitHub Discussion creation for each backport, wait for human approval before PR.

Create/modify files:
- src/backport/approval.rs - Discussion creation and monitoring
- src/backport/manager.rs - Trigger approval flow
- src/github/client.rs - create_discussion, monitor reactions

Discussion format (from plan):
## Backport Proposal
**Commit:** {sha} — {message}
**Source issue:** #{number}
**Target branch:** release/{X.Y}
This fix meets backport criteria. Approve by reacting 👍.
Backport will be filed as a PR targeting release/{X.Y}.

Behavior:
- Create Discussion in config.release.approval_discussion_category
- Wait for human reaction (👍/👎) or comment
- Same voting_window_days, stale_threshold_days as releases
- Vote tiebreaking: most recent wins, 👎 always halts

WHY
Backports need human approval like releases. Discussion provides audit trail.

HOW TO VERIFY
- Unit test: Discussion created with correct format
- Unit test: Uses approval_discussion_category
- Unit test: Waits for reaction/comment
- Unit test: Voting window/stale threshold from config
- Unit test: Vote tiebreaking implemented
- Integration test: Backport task → discussion → approval
- Manual: Trigger backport, verify discussion

EDGE CASES AND PITFALLS
- Discussion category missing - create or error
- GraphQL API for discussions
- Monitor reactions efficiently (not poll heavy)
- Stale discussion handling (CRIT-9, CRIT-10)
- Human comment vs reaction - both count
- Multiple backports = multiple discussions

PROJECT-SPECIFIC TERMINOLOGY
- 'Backport approval Discussion': GitHub Discussion for vote
- 'voting_window_days': Config, days before reminder
- 'stale_threshold_days': Config, days before close
- 'Vote tiebreaking': Most recent wins, 👎 halts
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e449b-14dd-770f-ac75-377c852da6f2
author: oompah
created: 2026-05-20T08:57:55Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e449b-8c2b-7865-bddc-d80eb9ddfea9
author: oompah
created: 2026-05-20T08:58:25Z

Focus: Event Queue Pipeline Specialist
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e449e-4b78-7abb-acef-73ecbe9797f9
author: oompah
created: 2026-05-20T09:01:25Z

Understanding: CRIT-3 requires implementing human approval for backports via GitHub Discussions. The discussion creation is already partially complete in , but we need to: (1) add GraphQL support for monitoring reactions/comments on discussions, (2) create  with voting logic, (3) wire up the approval flow in manager. Key requirements: vote tiebreaking where most recent wins, 👎 always halts, same voting_window_days/stale_threshold_days from config. Implementation plan: add GraphQL methods to client.rs, create approval.rs with ApprovalState enum and voting functions, update manager.rs to trigger approval flow and await human approval.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44ad-de10-7c17-8e4e-d1bbede96848
author: oompah
created: 2026-05-20T09:18:26Z

Discovery: Found that discussion creation was already partially implemented in manager.rs, but needed GraphQL support for monitoring reactions and comments. The ReleaseConfig in config/schema.rs already has voting_window_days and stale_threshold_days fields. Created approval.rs with: ApprovalState enum (Pending/Approved/Rejected/Stale/Expired), VoteRecord struct for tracking votes, GraphQL-based monitor_discussion_votes function, and compute_vote_state with vote tiebreaking logic per plan requirements.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44af-0d6f-7cb6-90e5-afddb101007f
author: oompah
created: 2026-05-20T09:19:43Z

Implementation: Created src/backport/approval.rs with Discussion creation and monitoring. Key functions: check_approval_status(), monitor_discussion_votes() (GraphQL), post_reminder_comment(), close_discussion(). Updated src/backport/manager.rs to track filed backports via FiledBackport struct, added check_pending_discussions() function for monitoring approval flow. Updated src/github/client.rs to expose config() and client() for GraphQL access, added created_at field to Discussion struct.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44b0-028d-746d-9b27-8c620f752d0a
author: oompah
created: 2026-05-20T09:20:46Z

Verification: cargo test passes (36 tests). make fmt-check passes. cargo clippy passes with only unused code warnings (expected). Push to remote succeeded. Implementation meets CRIT-3: Rodgers creates GitHub Discussion for each backport, waits for human approval before PR. Uses voting_window_days and stale_threshold_days from config.release, vote tiebreaking where most recent wins and 👎 always halts.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44b0-47ef-77c6-9257-ffb1054ce8e6
author: oompah
created: 2026-05-20T09:21:04Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 66, Tool calls: 78
- Tokens: 3.7M in / 27.1K out [3.8M total]
- Cost: $0.0000
- Exit: normal, Duration: 23m 7s
- Log: rogers-13t__20260520T085828Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44b0-4c49-7a5b-82b4-16af82116ebf
author: oompah
created: 2026-05-20T09:21:05Z

Agent completed successfully in 1387s (3772244 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
