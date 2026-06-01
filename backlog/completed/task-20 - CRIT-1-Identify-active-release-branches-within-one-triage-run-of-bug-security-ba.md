---
id: TASK-20
title: 'CRIT-1: Identify active release branches within one triage run of bug/security/backport-me
  merge'
status: Done
assignee: []
created_date: 2026-05-20 05:18
updated_date: 2026-05-20 08:24
labels:
- rodgers:parent=rogers-4qr
- rodgers:type=backport
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-qox
  state: closed
  parent_id: rogers-4qr
  dependencies: []
  branch_name: rogers-qox
  target_branch: null
  url: null
  created_at: '2026-05-20T05:18:57Z'
  updated_at: '2026-05-20T08:24:29Z'
  closed_at: '2026-05-20T08:24:22Z'
parent: TASK-2
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/backport-plan.md §Backport Detection → Acceptance Criteria CRIT-1

WHAT TO DO
Implement active release branch identification on bug/security/backport-me merge to main.

Create/modify files:
- src/backport/detector.rs - Backport candidate detection
- src/backport/manager.rs - Backport manager entry point
- src/triage/triage_loop.rs - Trigger on merge detection
- src/config/schema.rs - release.active_branches config

Detection triggers (on triage run):
- Bug fix merged to main (commit linked to bug issue)
- Security patch merged (GH Advisory, security label, CVE pattern)
- Issue with backport-me label merged

Action:
- Read config.release.active_branches
- Identify all active release branches
- Complete within ONE triage run of merge detection

WHY
Backports target active maintenance branches. Must identify targets fast after merge.

HOW TO VERIFY
- Unit test: Bug fix merge → detects active branches
- Unit test: Security patch merge → detects active branches
- Unit test: backport-me merge → detects active branches
- Unit test: Uses config.release.active_branches
- Unit test: Completes within one triage run
- Integration test: Merge bug fix, run triage, verify detection
- Manual: Configure branches, merge fix, verify

EDGE CASES AND PITFALLS
- active_branches empty - no backports, warn
- Branch doesn't exist - skip with warning
- Main branch always implicit source
- Release branch merged to main - different flow
- Security patches priority=1 (highest)
- Merge detection via GitHub API (merged PRs since last check)

PROJECT-SPECIFIC TERMINOLOGY
- 'Active release branches': config.release.active_branches (e.g., release/1.x)
- 'Backport candidate': Bug fix, security patch, or backport-me merge
- 'One triage run': Single scheduler tick after merge
- 'Security patch': GH Advisory, security label, or CVE pattern
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e4434-5523-70f4-95a3-b308815f2658
author: oompah
created: 2026-05-20T07:05:41Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4434-88ee-79b6-8199-395d5196117c
author: oompah
created: 2026-05-20T07:05:54Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4434-f17b-7406-935a-edbe685de5e7
author: oompah
created: 2026-05-20T07:06:21Z

Starting work on CRIT-1: Implement active release branch identification on bug/security/backport-me merge to main. First exploring the codebase structure.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4436-f3a1-7e75-9d5c-b53b029b4524
author: oompah
created: 2026-05-20T07:08:33Z

Agent stalled 1 time(s) (171s (222024 tokens)). Escalating from 'default' to 'standard'. Retrying in 10s (attempt #1)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4436-fcd4-77c7-935a-53d4cbc4a844
author: oompah
created: 2026-05-20T07:08:35Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/nvidia/nemotron-3-ultra-preview]
- Turns: 14, Tool calls: 14
- Tokens: 221.2K in / 868 out [222.0K total]
- Cost: $0.0000
- Exit: stalled, Duration: 2m 51s
- Log: rogers-qox__20260520T070602Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4437-6e01-72c2-8a9a-5b11d9b76152
author: oompah
created: 2026-05-20T07:09:04Z

Agent dispatched (profile: standard)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4437-8e3c-7405-9d58-b8ba6e5fa2b2
author: oompah
created: 2026-05-20T07:09:12Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4445-5499-716e-96d4-6e08da79e827
author: oompah
created: 2026-05-20T07:24:15Z

Understanding: CRIT-1 requires identifying active release branches when a bug/security/backport-me labeled issue is merged to main. Key files: src/config/schema.rs (add release.active_branches), src/backport/detector.rs (detect candidate merges), src/backport/manager.rs (orchestrate), src/triage/triage_loop.rs (trigger detection). Detection signals: bug label on linked issue, security label/CVE/GHSA, backport-me label. Plan: build modular architecture, wire detector into triage loop, add unit tests.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4471-cdd6-755d-86db-e002a042803a
author: oompah
created: 2026-05-20T08:12:49Z

Discovery: clap v4 derive generates related impls with private constructors for BackportCandidate::{new.rs}. Tests in detector.rs need str vs &str comparison fixes and test-only scope for BackportCandidate::new. Fixing tests now.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e447b-431a-7d2d-8eb7-de9f81f12b78
author: oompah
created: 2026-05-20T08:23:09Z

Implementation complete. All 13 tests pass. Key files: src/config/schema.rs (release.active_branches), src/backport/detector.rs (bug/security/backport-me detection), src/backport/manager.rs (files tasks per branch), src/triage/triage_loop.rs (one-run triage), src/github/client.rs (GitHub API), src/tasks/client.rs (backlog CLI). Quality gates: fmt-check ✓, cargo build ✓, cargo test ✓ (13/13), cargo clippy ✓. Committing now.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e447c-7638-7343-bc7e-d0ed9974f6a2
author: oompah
created: 2026-05-20T08:24:28Z

Run #2 [attempt=2, profile=standard, role=standard -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 143, Tool calls: 159
- Tokens: 10.9M in / 64.1K out [11.0M total]
- Cost: $0.0000
- Exit: normal, Duration: 1h 15m 23s
- Log: rogers-qox__20260520T070914Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e447c-7fa9-7d07-a934-ed3ac73ef30b
author: oompah
created: 2026-05-20T08:24:30Z

Agent completed successfully in 4523s (11009756 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
