---
id: TASK-81
title: GitHub API client for init checks
status: Done
assignee: []
created_date: 2026-05-20 05:40
updated_date: 2026-05-21 04:28
labels:
- rodgers:type=init
- feature
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: feature
tasks:
  id: rogers-zql.2
  state: closed
  parent_id: rogers-zql
  dependencies: []
  branch_name: rogers-zql.2
  target_branch: null
  url: null
  created_at: '2026-05-20T05:40:24Z'
  updated_at: '2026-05-21T04:28:40Z'
  closed_at: '2026-05-21T04:28:31Z'
parent: TASK-5
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/init-plan.md

WHAT TO DO
- Create src/github.rs module with GitHubClient struct
- Implement methods:
  - get_repository(owner, repo) -> Repository metadata
  - list_labels(owner, repo) -> Vec<Label>
  - create_label(owner, repo, LabelDefinition) -> Label
  - list_discussion_categories(owner, repo) -> Vec<DiscussionCategory>
  - create_discussion_category(owner, repo, name) -> DiscussionCategory
  - get_branch_protection(owner, repo, branch) -> BranchProtection
  - list_workflows(owner, repo) -> Vec<Workflow>
  - get_file_contents(owner, repo, path, ref) -> String (for issue templates, workflows, AGENTS.md, rogers.yaml)
- Use reqwest with proper GitHub API headers (Accept: application/vnd.github+json, Authorization: Bearer <token>)
- Handle pagination for list endpoints
- Map GitHub API errors to RogersError

WHY
All audit checks need to fetch data from GitHub. This client centralizes API calls, authentication, and error handling.

HOW TO VERIFY
- Unit tests mock HTTP responses and verify client methods
- Integration test with real GitHub token (manual)

EDGE CASES AND PITFALLS
- Rate limiting: respect GitHub rate limit headers
- Authentication: support both GITHUB_TOKEN env var and --github-token flag
- GitHub API versioning: use proper Accept headers
- Pagination: handle repos with many labels/workflows
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e442b-bce5-79d5-a895-960fa32b3750
author: oompah
created: 2026-05-20T06:56:18Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e442b-dd98-7c42-9836-0a06250aef19
author: oompah
created: 2026-05-20T06:56:26Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e442c-1983-7f23-b6d2-c0c2fe91ea21
author: oompah
created: 2026-05-20T06:56:41Z

Starting work on GitHub API client for init checks. First exploring the codebase to understand the existing architecture and patterns.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e442e-52ff-73bb-a1cf-c3f188e0d866
author: oompah
created: 2026-05-20T06:59:07Z

Agent stalled 1 time(s) (168s (112939 tokens)). Escalating from 'default' to 'standard'. Retrying in 10s (attempt #1)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e442e-538f-7364-b4fb-8935a62bbb85
author: oompah
created: 2026-05-20T06:59:07Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/nvidia/nemotron-3-ultra-preview]
- Turns: 11, Tool calls: 11
- Tokens: 112.3K in / 668 out [112.9K total]
- Cost: $0.0000
- Exit: stalled, Duration: 2m 48s
- Log: rogers-zql.2__20260520T065634Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e442e-ec87-721d-80d4-f5eb564312fd
author: oompah
created: 2026-05-20T06:59:46Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e442f-20f3-7000-ae5e-2a667cc8deb1
author: oompah
created: 2026-05-20T07:00:00Z

Starting work on GitHub API client for init checks. First exploring the codebase to understand the existing architecture and patterns.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e442f-2c39-7475-94a7-294a5faf28da
author: oompah
created: 2026-05-20T07:00:03Z

Agent dispatched (profile: standard)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4431-3b63-79f5-ad74-1c3c15e9479a
author: oompah
created: 2026-05-20T07:02:18Z

Run #2 [attempt=2, profile=standard, role=standard -> InferenceAPI/nvidia/nvidia/nemotron-3-ultra-preview]
- Turns: 11, Tool calls: 11
- Tokens: 114.9K in / 633 out [115.5K total]
- Cost: $0.0000
- Exit: stalled, Duration: 2m 31s
- Log: rogers-zql.2__20260520T065953Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4431-3f31-7c7c-ab8a-3975b79aa9ee
author: oompah
created: 2026-05-20T07:02:19Z

Agent stalled 2 time(s) (151s (115521 tokens)). Escalating from 'standard' to 'deep'. Retrying in 20s (attempt #2)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4434-1764-72c4-9fc9-bf7782c4949b
author: oompah
created: 2026-05-20T07:05:25Z

Retrying (attempt #4, agent: standard)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4434-4c33-739c-8749-5af7720ed3b3
author: oompah
created: 2026-05-20T07:05:39Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4435-4f91-706c-86e9-2e42d32de5c9
author: oompah
created: 2026-05-20T07:06:45Z

Run #5 [attempt=5, profile=standard, role=standard -> InferenceAPI/nvidia/nvidia/nemotron-3-ultra-preview]
- Turns: 10, Tool calls: 10
- Tokens: 123.5K in / 481 out [124.0K total]
- Cost: $0.0000
- Exit: stalled, Duration: 1m 23s
- Log: rogers-zql.2__20260520T070541Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4435-4fdd-7847-b1a9-06096f286b2f
author: oompah
created: 2026-05-20T07:06:45Z

Agent stalled 3 time(s) (83s (124015 tokens)). Escalating from 'standard' to 'deep'. Retrying in 160s (attempt #5)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48ae-5de0-7837-aca5-4f03392a7e4f
author: oompah
created: 2026-05-21T03:57:27Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48ae-828f-760e-a419-5113d9a6c750
author: oompah
created: 2026-05-21T03:57:37Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48af-4141-7550-8546-64b2daea9c9c
author: oompah
created: 2026-05-21T03:58:26Z

## Understanding

I'll create  — a GitHub API client module that centralizes all API calls for the init checks. The client will:

1. Define a  struct backed by 
2. Implement all 8 required methods: get_repository, list_labels, create_label, list_discussion_categories, create_discussion_category, get_branch_protection, list_workflows, get_file_contents
3. Reuse existing types:  from src/error.rs,  from src/labels.rs
4. Handle pagination via Link header parsing for list endpoints
5. Support authentication via  env var and  CLI flag
6. Respect rate limit headers
7. Include unit tests with mock HTTP responses

The module will be wired into the existing  /  module structure.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48ca-a682-7b7a-aa0a-e0d251a067fe
author: oompah
created: 2026-05-21T04:28:21Z

## Completion Summary

Delivered  — a complete GitHub API client module for init checks with:

**8 API methods:**
-  → Repository metadata
-  → Vec<Label> (paginated)
-  → Label
-  → Vec<DiscussionCategory> (paginated)
-  → DiscussionCategory
-  → BranchProtection
-  → Vec<Workflow> (paginated)
-  → String (base64 decoded)

**Key features:**
- Uses reqwest with proper GitHub API headers (Accept + X-GitHub-Api-Version)
- Bearer token auth via GITHUB_TOKEN env var or --github-token flag
- Rate limit header caching from x-ratelimit-* headers
- HTTP 429 handling with reset timestamp in error message
- Pagination via Link header parsing for list endpoints
- Maps GitHub API errors to RogersError with proper exit codes

**Tests:** 20 unit tests covering serialization, base64 decoding, pagination parsing, client construction, and all data types.

**Quality gates:** fmt-check ✓, cargo build ✓, cargo test (20/20) ✓, cargo clippy ✓

Pushed to branch .
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48ca-e87f-7ab8-840f-51e5f3fabb88
author: oompah
created: 2026-05-21T04:28:38Z

Agent completed successfully in 1868s (7118482 tokens)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48ca-f67b-74ab-95c7-af7e3c6d4c1f
author: oompah
created: 2026-05-21T04:28:41Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/qwen/qwen3.6-35b-a3b]
- Turns: 94, Tool calls: 103
- Tokens: 7.1M in / 47.7K out [7.1M total]
- Cost: $0.0000
- Exit: normal, Duration: 31m 8s
- Log: rogers-zql.2__20260521T035739Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
