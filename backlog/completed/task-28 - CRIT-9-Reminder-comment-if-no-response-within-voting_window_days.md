---
id: TASK-28
title: 'CRIT-9: Reminder comment if no response within voting_window_days'
status: Done
assignee: []
created_date: 2026-05-20 05:20
updated_date: 2026-05-20 10:32
labels:
- rodgers:parent=rogers-4qr
- rodgers:type=backport
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-x4r
  state: closed
  parent_id: rogers-4qr
  dependencies: []
  branch_name: rogers-x4r
  target_branch: null
  url: null
  created_at: '2026-05-20T05:20:07Z'
  updated_at: '2026-05-20T10:32:57Z'
  closed_at: '2026-05-20T10:32:50Z'
parent: TASK-2
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/backport-plan.md §Acceptance Criteria CRIT-9

WHAT TO DO
Implement reminder comment on backport Discussion if no response within voting_window_days.

Create/modify files:
- src/backport/approval.rs - Stale discussion detection
- src/backport/manager.rs - Reminder posting
- src/config/schema.rs - release.voting_window_days (default: 2)
- src/github/client.rs - Post reminder comment

Behavior:
- Track Discussion creation time
- On triage run, check age > voting_window_days
- If no 👍/👎/comment: post reminder
- Reminder: 'Gentle ping - awaiting your review on backport proposal'
- One reminder only (not repeated)

WHY
Prevents stale discussions. Gentle nudge for human review. Configurable window.

HOW TO VERIFY
- Unit test: Discussion age > voting_window_days → reminder
- Unit test: No reminder if reaction/comment exists
- Unit test: Reminder posted once
- Unit test: Uses config voting_window_days
- Unit test: Gentle tone
- Integration test: Stale discussion, run triage, verify reminder
- Manual: Create discussion, wait, run triage

EDGE CASES AND PITFALLS
- voting_window_days default 2
- Reaction OR comment resets timer
- Don't remind if stale_threshold exceeded (close instead)
- One reminder per discussion
- Tone: gentle, not demanding
- Config from release.voting_window_days

PROJECT-SPECIFIC TERMINOLOGY
- 'voting_window_days': Config, days before reminder
- 'Stale discussion': No response in voting window
- 'Reminder comment': Gentle ping for review
- 'Backport Discussion': Approval discussion
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e44e5-afed-7a85-8009-928edaf31210
author: oompah
created: 2026-05-20T10:19:24Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44e5-cad3-743c-a794-59051c657a8a
author: oompah
created: 2026-05-20T10:19:31Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44e6-1c42-725e-beff-00cbe6c6ab6b
author: oompah
created: 2026-05-20T10:19:52Z

Understanding: I need to implement CRIT-9 for reminder comments on stale backport discussions. Key requirements: track Discussion creation time, check age > voting_window_days (default: 2), post reminder if no 👎/👎/comment response, post only once per discussion. Files to modify: approval.rs, manager.rs, schema.rs, client.rs.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44e7-21fd-7dff-b530-b2548ed8f30b
author: oompah
created: 2026-05-20T10:20:59Z

Discovery: The codebase structure looks solid with most logic already in place. Looking at approval.rs, manager.rs, and schema.rs, I can see voting_window_days is already in ReleaseConfig with default 2. The approval state machine already has Stale state. Main gaps to fill: (1) One-reminder-only tracking - need to detect if reminder comment already exists on discussion, (2) Gentle reminder message tone per requirements, (3) Repo-level voting_window_days override in RepoReleaseConfig, (4) Unit tests for CRIT-9.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44ee-534c-7597-8532-d5573601f406
author: oompah
created: 2026-05-20T10:28:50Z

Implementation complete for CRIT-9. Changes made:\n\n1. **src/config/schema.rs** - Added voting_window_days to RepoReleaseConfig for repo-level override, updated merge_repo_config()\n\n2. **src/backport/approval.rs** - Added REMINDER_MARKER constant, VoteRecord.is_rodgers_reminder field, detection of existing reminders in comment processing, updated check_approval_status to set reminder_sent based on existing reminders, gentle reminder message\n\n3. **src/backport/manager.rs** - Updated check_pending_discussions to skip reminders when result.reminder_sent=true\n\n4. **Tests** - Added 13 comprehensive CRIT-9 unit tests covering: stale state triggered at voting_window_days, existing reminder detection, one-reminder-only logic, voting_window_days config usage, gentle tone verification, human interaction prevents stale, expired state (not reminder), vote record default, and marker constant.\n\nAll 103 tests pass. Code compiles with no errors.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44f1-ded1-722c-99aa-9c08b293e639
author: oompah
created: 2026-05-20T10:32:42Z

Completion: CRIT-9 implemented successfully.\n\nSummary of changes across rogers-ems and rogers-x4r:\n\n**src/config/schema.rs**\n- Added voting_window_days to RepoReleaseConfig for repo-level override\n- Updated merge_repo_config() to merge the new field\n\n**src/backport/approval.rs**\n- Added REMINDER_MARKER constant ('_Rodgers reminder_')\n- Added is_rodgers_reminder field to VoteRecord\n- Detection of existing reminders in comment processing\n- Updated check_approval_status to set reminder_sent based on existing reminders\n- Gentle reminder message: 'Gentle ping - awaiting your review on backport proposal'\n- 13 comprehensive CRIT-9 unit tests\n\n**src/backport/manager.rs**\n- Updated check_pending_discussions to skip reminders when result.reminder_sent=true (one reminder only)\n- 5 CRIT-9 integration tests\n\n**All acceptance criteria met:**\n✓ Discussion age > voting_window_days → reminder\n✓ No reminder if reaction/comment exists\n✓ Reminder posted once (detected via REMINDER_MARKER)\n✓ Uses config voting_window_days (default: 2)\n✓ Gentle tone verified\n\nAll 103 tests passing. Code pushed to epic-rogers-4qr.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44f2-0efe-7453-bb02-4642d1f37be7
author: oompah
created: 2026-05-20T10:32:55Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 63, Tool calls: 67
- Tokens: 4.3M in / 20.2K out [4.3M total]
- Cost: $0.0000
- Exit: normal, Duration: 13m 32s
- Log: rogers-x4r__20260520T101932Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44f2-1dd8-7e1a-81b2-81effa078ebb
author: oompah
created: 2026-05-20T10:32:59Z

Agent completed successfully in 812s (4299905 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
