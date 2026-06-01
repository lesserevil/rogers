---
id: TASK-70
title: 'CRIT-5: Auto-backport labels trigger backport workflow'
status: Done
assignee: []
created_date: 2026-05-20 05:26
updated_date: 2026-05-21 06:07
labels:
- rodgers:parent=rogers-zjm
- rodgers:type=release-management
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-7jj
  state: closed
  parent_id: rogers-zjm
  dependencies: []
  branch_name: rogers-7jj
  target_branch: null
  url: null
  created_at: '2026-05-20T05:26:58Z'
  updated_at: '2026-05-21T06:07:13Z'
  closed_at: '2026-05-21T06:07:06Z'
parent: TASK-8
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/release-management-plan.md §Release Approval Flow → integrates with plans/backport-plan.md

WHAT TO DO
Implement auto-backport label detection that triggers backport workflow for labeled issues.

Create/modify files:
- src/release/backport_trigger.rs - Detect backport-me label on merged PRs/issues
- src/backport/manager.rs - Backport manager entry point
- src/triage/triage_loop.rs - Check for backport labels during triage

Trigger logic:
- On PR merge to main or release branch, check linked issue for 'backport-me' label
- On triage run, check issues with 'backport-me' label that were recently closed/merged
- For each, identify target release branches (config.release.active_branches)
- File backport task for each target branch (see plans/backport-plan.md CRIT-2)
- Create GitHub Discussion for approval (same as release approval)

WHY
Not all fixes need backporting. The 'backport-me' label lets humans explicitly request backports for important fixes that don't meet auto-criteria (security, bug fix).

HOW TO VERIFY
- Unit test: Detects backport-me label on merged PR's linked issue
- Unit test: Identifies correct target branches from config
- Unit test: Files backport task per target branch
- Unit test: Creates approval Discussion for each backport
- Integration test: Merge PR with backport-me label, verify backport flow
- Manual: Add backport-me to test issue, merge PR, verify

EDGE CASES AND PITFALLS
- Label on PR vs issue - check both
- Multiple target branches - file task per branch
- Already backported - detect via semantic equivalence (backport-plan.md)
- Security patches auto-backport without label (higher priority)
- Backport approval uses same voting window/stale threshold as releases

PROJECT-SPECIFIC TERMINOLOGY
- 'backport-me': Human-applied label requesting backport
- 'Active release branches': config.release.active_branches (e.g., release/1.x, release/2.x)
- 'Backport task': Chore task (rodgers:type=backport) tracking cherry-pick
- 'Backport approval Discussion': GitHub Discussion for human 👍/👎 vote
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e4913-d4d9-7370-a075-f881ee53a813
author: oompah
created: 2026-05-21T05:48:17Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4913-ee15-70b7-8050-3448ab0b6adb
author: oompah
created: 2026-05-21T05:48:23Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4925-20ed-74d6-9e25-152c6ec341e9
author: oompah
created: 2026-05-21T06:07:11Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/qwen/qwen3.6-35b-a3b]
- Turns: 86, Tool calls: 103
- Tokens: 11.9M in / 37.6K out [11.9M total]
- Cost: $0.0000
- Exit: normal, Duration: 18m 56s
- Log: rogers-7jj__20260521T054826Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4925-215d-71d2-9916-871599f7c178
author: oompah
created: 2026-05-21T06:07:11Z

Agent completed successfully in 1136s (11908880 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
