---
id: TASK-144
title: 'YOLO task-PR coherence break on rogers/2: merge-conflict recovery task missing
  or stale'
status: Done
assignee: []
created_date: 2026-05-20 23:02
updated_date: 2026-05-21 03:10
labels:
- needs-human
- yolo-watchdog
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-tv0
  state: closed
  parent_id: null
  dependencies: []
  branch_name: rogers-tv0
  target_branch: null
  url: null
  created_at: '2026-05-20T23:02:25Z'
  updated_at: '2026-05-21T03:10:19Z'
  closed_at: '2026-05-21T03:10:19Z'
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
PR #2 on rogers (branch `epic-rogers-ykp`) is in a state requiring `merge-conflict` recovery, but no matching open task exists.

- Reason: recovery task rogers-870 is closed (state=closed) but PR still has merge-conflict condition
- Detector: D3 (task-PR coherence)
- Recovery: the YOLO orphan-recovery cache for this PR has been cleared, so the next tick will re-attempt to file the correct recovery task. If this watchdog task recurs without resolution, an operator must investigate the PR by hand.
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENTS:END -->
