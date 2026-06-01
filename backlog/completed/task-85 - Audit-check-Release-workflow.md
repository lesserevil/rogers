---
id: TASK-85
title: 'Audit check: Release workflow'
status: Done
assignee: []
created_date: 2026-05-20 05:41
updated_date: 2026-05-21 05:46
labels:
- rodgers:type=init
- feature
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: feature
tasks:
  id: rogers-zql.6
  state: closed
  parent_id: rogers-zql
  dependencies: []
  branch_name: rogers-zql.6
  target_branch: null
  url: null
  created_at: '2026-05-20T05:41:34Z'
  updated_at: '2026-05-21T05:46:18Z'
  closed_at: '2026-05-21T05:46:11Z'
parent: TASK-5
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/init-plan.md §Audit Checks / 4. GitHub Actions — Release Workflow

WHAT TO DO
- Create src/checks/release_workflow.rs with ReleaseWorkflowCheck struct
- Implement check(github: &GitHubClient, owner, repo) -> CheckResult
- List workflow files in .github/workflows/ via GitHub API
- For each .yml/.yaml file, fetch contents and analyze:
  - Has trigger: push with tag pattern (v*, *.*.*) OR workflow_dispatch with release inputs
  - Has artifact upload step (upload-artifact, gh release upload, docker push, etc.)
- Return CheckResult with:
  - severity: Blocker if no release-capable workflow found
  - severity: Warn if workflow exists but no artifact upload detected
  - severity: Info if release workflow with artifacts found
  - fixability: Manual (requires adding workflow file)
  - fix_instructions: template workflow location and adoption steps

WHY
Rodgers creates releases but needs CI to build artifacts. This is a blocker check.

HOW TO VERIFY
- Unit test: mock workflow file contents for various patterns
- Test tag pattern detection (v*, *.*.*, release-*)
- Test artifact upload detection (multiple patterns)
- Verify severity levels correct

EDGE CASES AND PITFALLS
- Workflow YAML parsing - use serde_yaml
- Tag patterns can vary: v*, *.*.*, release/*, etc.
- Artifact upload can be: actions/upload-artifact, gh release upload, docker push, aws s3 cp
- Workflows might be in subdirectories
- Multiple workflow files - need to check all
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e48f3-c31a-7835-96d7-04d93305107b
author: oompah
created: 2026-05-21T05:13:15Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48f3-d4d7-7509-80d0-ce2be798b968
author: oompah
created: 2026-05-21T05:13:20Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48f4-a51c-7a96-b343-cb21f8b68328
author: oompah
created: 2026-05-21T05:14:13Z

## Understanding

I need to create a  that audits whether a repository has a GitHub Actions workflow capable of building release artifacts. Here's my approach:

1. **List workflows** via the GitHub Actions API () which returns workflow paths like 
2. **Fetch each workflow file's contents** via the Contents API (requires base64 decode)
3. **Parse YAML** and analyze for:
   - **Release triggers**:  with  patterns (v*, *.*.*, release-*), OR  with release-related inputs
   - **Artifact uploads**: , , , 
4. **Severity levels**: Blocker (no release workflow), Warn (release workflow but no artifacts), Info (complete)
5. **Unit tests**: mock workflow contents testing all patterns and severity levels
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4910-3cf2-7cca-a0b4-7faa782c80a0
author: oompah
created: 2026-05-21T05:44:21Z

## Verification

All 90 tests pass (38 new release_workflow tests + 52 existing).

Test coverage includes:
- No workflows → Blocker
- Non-release workflows → Blocker  
- Release workflow with artifacts → Info
- Release workflow without artifacts → Warn
- Multiple tag patterns (v*, *.*.*, release-*, release/*)
- Multiple artifact patterns (upload-artifact, gh release upload, docker push, aws s3 cp/sync)
- workflow_dispatch with release inputs vs without
- .yaml extension support
- Unreadable workflow files gracefully skipped
- Branch-only and PR-only triggers correctly rejected

Quality gates: build ✓, tests ✓, clippy ✓, fmt ✓
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4910-74ce-7e89-b1f6-1984e7090751
author: oompah
created: 2026-05-21T05:44:36Z

## Implementation

Created  with  struct implementing  trait.

**Key design:**
- Lists workflows via GitHub Actions API ()
- Fetches each / file via Contents API
- Analyzes YAML text (case-insensitive substring matching):
  - Release triggers:  →  with patterns , , , 
  - Or  with release-related input keys (release, version, publish, target, tag)
  - Artifact upload: , , , 
- Three severity levels: Blocker (no release workflow), Warn (release but no artifacts), Info (complete)
- Fix instructions include template workflow URL and adoption steps

**Also:**
- Added  to  exports
- Integrated into 
- Added  accessor to  (was previously private)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4911-86f4-744c-a248-bd761bf64ecf
author: oompah
created: 2026-05-21T05:45:46Z

## Completion

Delivered:

**Files created/modified:**
-  — New  struct (671 lines total including 38 tests)
-  — Added module export for 
-  — Added  accessor
-  — Integrated check into 

**What the check does:**
1. Lists workflows via GitHub Actions API
2. Fetches each workflow file via Contents API
3. Analyzes YAML for release triggers (push with tag patterns v*, *.*.*, release-*, release/* OR workflow_dispatch with release inputs)
4. Detects artifact uploads (upload-artifact, gh release upload, docker push, aws s3 cp/sync)
5. Returns severity: Blocker (no release workflow), Warn (release but no artifacts), Info (complete)
6. Provides fix instructions with template workflow link and adoption steps

**Test coverage (38 tests):**
- 18 integration tests (mock HTTP), 20 unit tests (detection helpers)
- All tag patterns: v*, *.*.*, release-*, release/*
- All artifact patterns: upload-artifact, gh release upload, docker push, aws s3 cp/sync
- Edge cases: workflow_dispatch with/without release inputs, unreadable files, .yaml extension, branch-only triggers, PR-only triggers
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4911-f6e2-7208-9cb0-65322919b198
author: oompah
created: 2026-05-21T05:46:15Z

Agent completed successfully in 1982s (13009327 tokens)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4912-0592-74b3-b2e7-c935eb7f4ad7
author: oompah
created: 2026-05-21T05:46:18Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/qwen/qwen3.6-35b-a3b]
- Turns: 115, Tool calls: 128
- Tokens: 12.9M in / 112.5K out [13.0M total]
- Cost: $0.0000
- Exit: normal, Duration: 33m 2s
- Log: rogers-zql.6__20260521T051329Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
