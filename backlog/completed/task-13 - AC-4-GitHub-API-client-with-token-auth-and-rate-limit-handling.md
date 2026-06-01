---
id: TASK-13
title: 'AC-4: GitHub API client with token auth and rate limit handling'
status: Done
assignee: []
created_date: 2026-05-20 05:17
updated_date: 2026-05-20 07:38
labels:
- rodgers:parent=rogers-6ny
- rodgers:type=architecture
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-5sg
  state: closed
  parent_id: rogers-6ny
  dependencies: []
  branch_name: rogers-5sg
  target_branch: null
  url: null
  created_at: '2026-05-20T05:17:50Z'
  updated_at: '2026-05-20T07:38:51Z'
  closed_at: '2026-05-20T07:38:45Z'
parent: TASK-1
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/architecture-plan.md §System Components / GitHub API Client → Acceptance Criteria AC-4

WHAT TO DO
Implement GitHub API client with token auth and rate limit handling.

Create/modify files:
- src/github/client.rs - GitHub API client
- src/github/auth.rs - Token authentication
- src/github/rate_limit.rs - Rate limit handling
- src/github/models.rs - API response models

Features:
- Token auth via PAT (github.token from config, supports )
- Required scopes: repo, read:org
- Rate limit handling: retry with backoff, respect retry_after
- All GitHub communication through this client
- REST API for issues, PRs, labels, releases
- GraphQL for discussions

WHY
Single client = consistent auth, rate limiting, error handling. All GitHub ops here.

HOW TO VERIFY
- Unit test: Auth with token works
- Unit test: Scopes validated
- Unit test: Rate limit triggers backoff
- Unit test: retry_after respected
- Unit test: All issue/PR/label/release ops work
- Unit test: GraphQL discussions work
- Integration test: Real GitHub API calls
- Manual: Configure token, test API calls

EDGE CASES AND PITFALLS
- Token from config.yaml or 
- Rate limit: 5000/hr default, warn <100 remaining
- Retry: exponential backoff, max retries
- 403/401 = auth error, fail fast
- 404 = not found, handle gracefully
- Pagination for list endpoints
- GraphQL vs REST endpoints

PROJECT-SPECIFIC TERMINOLOGY
- 'GitHub API client': Thin wrapper around reqwest
- 'PAT': Personal Access Token
- 'Rate limit handling': Backoff, retry_after respect
- 'Required scopes': repo, read:org
- 'Single client': All GitHub ops here
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e4438-ee0d-73eb-a442-e60a3ccaa027
author: oompah
created: 2026-05-20T07:10:42Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4439-16c3-7857-8569-f86d6cf63bee
author: oompah
created: 2026-05-20T07:10:53Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e443d-efe1-7ca7-b3e0-31b7f7758bf8
author: oompah
created: 2026-05-20T07:16:10Z

**Understanding**: Implementing a GitHub API client with token auth and rate limit handling. The client will be a thin wrapper around reqwest, handling all GitHub communication with consistent auth, rate limiting, and error handling.

**Plan**:
1. Create  directory with module structure
2. Implement models.rs with GitHub API response types
3. Implement auth.rs with PAT authentication and scope validation
4. Implement rate_limit.rs with backoff/retry logic
5. Implement client.rs as the main client exposing REST and GraphQL methods
6. Add comprehensive unit tests for all components
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4451-8f7c-768e-bd81-4874e9e8950d
author: oompah
created: 2026-05-20T07:37:36Z

**Discovery**: Floor the throttle and nail the implementation. Our frontend stack (Option B) doesn't need a bloated HTTP client library—reqwest is already in the dependencies and our own  +  is exactly the right abstraction. Started coding a focused  module—byte-limited and purpose-built.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4452-a687-79f4-a00a-24a7094a1d44
author: oompah
created: 2026-05-20T07:38:48Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 66, Tool calls: 76
- Tokens: 3.7M in / 30.3K out [3.8M total]
- Cost: $0.0000
- Exit: normal, Duration: 28m 5s
- Log: rogers-5sg__20260520T071056Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4452-a78b-76cc-ab65-9d4e8782a4c1
author: oompah
created: 2026-05-20T07:38:48Z

Agent completed successfully in 1685s (3770828 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
