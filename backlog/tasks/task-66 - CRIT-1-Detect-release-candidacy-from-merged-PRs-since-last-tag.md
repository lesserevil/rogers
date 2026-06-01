---
id: TASK-66
title: 'CRIT-1: Detect release candidacy from merged PRs since last tag'
status: To Do
assignee: []
created_date: 2026-05-20 05:26
updated_date: 2026-05-21 05:30
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
  id: rogers-pkx
  state: open
  parent_id: rogers-zjm
  dependencies: []
  branch_name: rogers-pkx
  target_branch: null
  url: null
  created_at: '2026-05-20T05:26:30Z'
  updated_at: '2026-05-21T05:30:21Z'
  closed_at: null
parent: TASK-8
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/release-management-plan.md §Release Readiness / Criteria to Propose a Release (from main) → Acceptance Criteria CRIT-1

WHAT TO DO
Implement release candidacy detection that runs on schedule or manual trigger to find merged PRs since last release tag.

Create/modify files:
- src/release/detector.rs - Release candidacy detection logic
- src/release/mod.rs - Release manager entry point
- src/github/client.rs - GitHub API calls for merged PRs and tags

Detection logic:
- Query GitHub for merged PRs to main since last release tag (git describe --tags --abbrev=0)
- Group PRs by type using conventional commits: feat, fix, chore, docs, refactor, perf, test
- Determine version bump: major for BREAKING CHANGE, minor for feat:, patch for fix:/others
- Only propose release if CI green on main and PRs exist since last tag

WHY
Automated release candidacy detection enables regular releases without human monitoring. Grouping by conventional commits enables semantic versioning and meaningful changelogs.

HOW TO VERIFY
- Unit test: Finds merged PRs since last tag
- Unit test: Groups PRs by conventional commit type correctly
- Unit test: Version bump calculation (BREAKING→major, feat→minor, fix→patch)
- Unit test: Skips if CI not green on main
- Integration test: Full detection run after merge to main
- Manual: Create test PRs, tag release, merge more PRs, run detector

EDGE CASES AND PITFALLS
- No tags exist yet (initial release) - handle gracefully
- Non-conventional commit PRs - categorize as chore or skip
- Multiple tags - use latest semver tag
- Release branches - detect separately per branch
- GitHub API pagination for many PRs
- Rate limiting on GitHub API calls

PROJECT-SPECIFIC TERMINOLOGY
- 'Release candidacy': Determination that a release should be proposed
- 'Conventional commits': Commit message format (feat:, fix:, BREAKING CHANGE:)
- 'Version bump': Semantic version increment (major/minor/patch)
- 'Last release tag': Most recent git tag matching v*.*.* pattern
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e4902-af10-7610-984f-b49fbb3ba079
author: oompah
created: 2026-05-21T05:29:33Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4902-caac-7edb-97fb-e54d3d8b24e6
author: oompah
created: 2026-05-21T05:29:40Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4903-5b13-7adc-9178-ca26d623c3cd
author: oompah
created: 2026-05-21T05:30:17Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/qwen/qwen3.6-35b-a3b]
- Turns: 6, Tool calls: 17
- Tokens: 94.8K in / 1.1K out [96.0K total]
- Cost: $0.0000
- Exit: ask_question, Duration: 46s
- Log: rogers-pkx__20260521T052944Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4903-602f-776b-ae23-09b037d02b77
author: oompah
created: 2026-05-21T05:30:19Z

🤚 **Question from agent:**

The issue description says to modify `src/github/client.rs` for GitHub API calls for merged PRs and tags. However, the issue also says 'CRIT-1: When all readiness criteria are met and CI is green, Rodgers creates a Release Proposal Discussion within one triage run'. 

The plan describes two stages: (1) release candidacy detection (CRIT-1 in the task description) and (2) creating a Release Proposal Discussion (CRIT-1 in the plan document). 

Should I implement only the detection logic (finding merged PRs, version bump calculation, CI check) as a standalone module that can be called by the triage loop later, or should I also implement the discussion creation part? The plan document says CRIT-1 is the discussion creation, while the issue description focuses on detection.

Also, for CI green check - should I use the GitHub API to check CI status (commit status checks / checks runs), or just check if the latest commit exists? The plan says 'the last commit to main has passed all CI checks' - this requires API calls to check commit statuses.

Please clarify whether:
1. I should implement discussion creation as well, or only the detection module
2. For CI checks, should I poll the GitHub checks API or use commit statuses?
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
