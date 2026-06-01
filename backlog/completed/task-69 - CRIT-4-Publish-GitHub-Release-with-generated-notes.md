---
id: TASK-69
title: 'CRIT-4: Publish GitHub Release with generated notes'
status: Done
assignee: []
created_date: 2026-05-20 05:26
updated_date: 2026-05-21 05:47
labels:
- rodgers:parent=rogers-zjm
- rodgers:type=release-management
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-3mz
  state: closed
  parent_id: rogers-zjm
  dependencies: []
  branch_name: rogers-3mz
  target_branch: null
  url: null
  created_at: '2026-05-20T05:26:51Z'
  updated_at: '2026-05-21T05:47:24Z'
  closed_at: '2026-05-21T05:47:12Z'
parent: TASK-8
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/release-management-plan.md §Release Execution → Acceptance Criteria CRIT-4

WHAT TO DO
Implement GitHub Release creation with generated changelog as release notes.

Create/modify files:
- src/release/github_release.rs - GitHub Release API integration
- src/release/mod.rs - Orchestrate release creation
- src/release/changelog.rs - Use generated changelog (from rogers-bal)

Execution flow:
1. After release branch and tag created
2. Call GitHub API to create Release for tag vX.Y.Z
3. Title: 'Release vX.Y.Z'
4. Body: Generated changelog from PR titles/labels
5. Mark as latest release (not prerelease)
6. Post notification comment on proposal Discussion

WHY
GitHub Releases provide the user-facing release artifact with download links, notes, and visibility. Automated creation ensures consistency and immediate availability after tag push.

HOW TO VERIFY
- Unit test: Creates GitHub Release via API with correct tag
- Unit test: Release title matches 'Release vX.Y.Z'
- Unit test: Release body contains generated changelog
- Unit test: Release marked as latest (not prerelease)
- Unit test: Posts notification to proposal Discussion
- Integration test: Full release creation after branch/tag
- Manual: Trigger release, verify GitHub Release page

EDGE CASES AND PITFALLS
- GitHub API rate limit - retry with backoff
- Release already exists for tag - detect and update or skip
- Changelog too large - GitHub has limit, truncate with link
- Network failure during creation - retry, file task on persistent failure
- Must link release task to merge commit that triggered it (traceability)
- Release task filed with full metadata for audit (rogers-q74)

PROJECT-SPECIFIC TERMINOLOGY
- 'GitHub Release': GitHub's release object with assets, notes, UI
- 'Release proposal Discussion': Human approval gate before release
- 'Release task': Chore task (rodgers:type=release) tracking release work
- 'Merge commit trigger': The PR merge that made release candidacy true
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e490b-c5aa-7ae6-bce9-1e109ef95eb5
author: oompah
created: 2026-05-21T05:39:29Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e490b-d973-74a7-81e0-b78fceb76fc3
author: oompah
created: 2026-05-21T05:39:34Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e490c-a284-7ce5-b0c7-51dd017e4526
author: oompah
created: 2026-05-21T05:40:25Z

## Understanding

I need to implement GitHub Release creation with generated changelog as release notes. The plan requires:

1. **** — GitHub Release API integration:
   - Create a release for a git tag via GitHub REST API
   - Handle retry with backoff for rate limits and network failures
   - Detect and handle existing releases for the same tag
   - Truncate changelog if it exceeds GitHub's 125KB limit

2. **Update ** — Re-export the new module and provide orchestration for the full release creation flow

3. **Uses existing ** — The changelog module from rogers-bal already handles PR parsing, grouping, and markdown generation. I'll integrate it into the release creation flow.

4. **Post notification** to the proposal Discussion after release creation.

The implementation will follow the existing patterns in  for HTTP calls and  for error types.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4912-a563-7c4f-9c65-d0b2bb9925dd
author: oompah
created: 2026-05-21T05:46:59Z

## Implementation Complete

I implemented GitHub Release creation with generated changelog as release notes:

**New file: **
-  — GitHub Release API integration with retry (exponential backoff), existing release detection (updates instead of creating duplicates), and changelog truncation (128KB limit with fallback note)
-  — Configuration struct for tag, title, body, prerelease flag, and target commitish
-  — API response model for deserializing release data
-  — Convenience function to build ReleaseConfig from ChangelogConfig + PRs
-  — Generates Discussion notification body

**Modified: **
- Re-exports all public types: , , , , 

**Modified: **
- Added  impl for  (required by retry logic)
- Added  error variant (required for serde_json serialization errors)
- Added  to exit_code() match arm

**CRIT-4 Acceptance Criteria verified by tests:**
- ✅ Release title matches 'Release vX.Y.Z' — 
- ✅ Correct tag for release — 
- ✅ Release body contains generated changelog — 
- ✅ Release marked as latest (not prerelease) — 
- ✅ Posts notification to proposal Discussion — 
- ✅ draft=false, prerelease=false for stable — 

**Edge cases handled:**
- ✅ Rate limit retry with exponential backoff
- ✅ Release already exists — detected via GET, then UPDATE
- ✅ Changelog too large — truncated at 128KB with note
- ✅ Network failure — retry with backoff, error on persistent failure

**Quality gates:**
- 297 tests pass (14 new tests in github_release module)
-  clean
-  clean
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4912-ed64-75f0-b771-c10466984b35
author: oompah
created: 2026-05-21T05:47:18Z

Agent completed successfully in 471s (2389504 tokens)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4912-f17e-7840-9e96-96c4f17344f4
author: oompah
created: 2026-05-21T05:47:19Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/qwen/qwen3.6-35b-a3b]
- Turns: 44, Tool calls: 60
- Tokens: 2.4M in / 26.7K out [2.4M total]
- Cost: $0.0000
- Exit: normal, Duration: 7m 51s
- Log: rogers-3mz__20260521T053935Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
