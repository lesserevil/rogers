---
id: TASK-82
title: 'Audit check: Issue templates'
status: Done
assignee: []
created_date: 2026-05-20 05:40
updated_date: 2026-05-21 04:42
labels:
- rodgers:type=init
- feature
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: feature
tasks:
  id: rogers-zql.3
  state: closed
  parent_id: rogers-zql
  dependencies: []
  branch_name: rogers-zql.3
  target_branch: null
  url: null
  created_at: '2026-05-20T05:40:35Z'
  updated_at: '2026-05-21T04:42:38Z'
  closed_at: '2026-05-21T04:42:18Z'
parent: TASK-5
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/init-plan.md §Audit Checks / 1. Issue Templates

WHAT TO DO
- Create src/checks/issue_templates.rs with IssueTemplatesCheck struct
- Implement check(github: &GitHubClient, owner, repo) -> CheckResult
- Check: .github/ISSUE_TEMPLATE/ directory exists with at least one .yml or .md template file
- Use GitHub API to list directory contents or get file tree
- Return CheckResult with:
  - severity: Blocker if no directory, Warn if directory empty/no templates, Info if templates found
  - description: specific message about what was found
  - fixability: Manual (requires creating files in repo)
  - fix_instructions: URL to create templates or note about --fix opening PR

WHY
Rodgers needs issue templates to route incoming issues. This is a blocker check.

HOW TO VERIFY
- Unit test: mock GitHub API responses for various states (no dir, empty dir, has templates)
- Verify CheckResult matches expected severity/description

EDGE CASES AND PITFALLS
- GitHub API for directory listing uses Git Tree API or Contents API
- Template files can be .yml, .yaml, or .md
- Directory might exist but be empty
- Case sensitivity: .github/ISSUE_TEMPLATE vs .github/issue_template
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e442b-d4fc-724e-971a-76329aa3fdbb
author: oompah
created: 2026-05-20T06:56:24Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e442b-f028-7b28-8756-f06a178b8ee5
author: oompah
created: 2026-05-20T06:56:31Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e442c-2002-70d6-a0ef-75175487c31f
author: oompah
created: 2026-05-20T06:56:43Z

Understanding: Need to create IssueTemplatesCheck in src/checks/issue_templates.rs that checks for .github/ISSUE_TEMPLATE/ directory with template files (.yml, .yaml, .md). Uses GitHub API to list directory contents. Returns CheckResult with severity: Blocker if no directory, Warn if empty/no templates, Info if templates found.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e442e-6df4-785b-a8a9-5c9574891d44
author: oompah
created: 2026-05-20T06:59:14Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/nvidia/nemotron-3-ultra-preview]
- Turns: 11, Tool calls: 11
- Tokens: 111.1K in / 725 out [111.8K total]
- Cost: $0.0000
- Exit: stalled, Duration: 2m 49s
- Log: rogers-zql.3__20260520T065633Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e442e-6ee7-72ac-bf7e-692a8261debc
author: oompah
created: 2026-05-20T06:59:14Z

Agent stalled 1 time(s) (169s (111847 tokens)). Escalating from 'default' to 'standard'. Retrying in 10s (attempt #1)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e442e-dae8-7d63-b95f-8d3b0cfa09c7
author: oompah
created: 2026-05-20T06:59:42Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e442e-e681-750a-8066-c6234babbc02
author: oompah
created: 2026-05-20T06:59:45Z

Agent dispatched (profile: standard)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e442f-3931-78b9-9022-040ac024dbdd
author: oompah
created: 2026-05-20T07:00:06Z

Understanding: Need to create IssueTemplatesCheck in src/checks/issue_templates.rs that checks for .github/ISSUE_TEMPLATE/ directory with template files (.yml, .yaml, .md). Uses GitHub API to list directory contents. Returns CheckResult with severity: Blocker if no directory, Warn if empty/no templates, Info if templates found.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4437-4a44-7907-9af9-d6ddbf1196c0
author: oompah
created: 2026-05-20T07:08:55Z

Run #2 [attempt=2, profile=standard, role=standard -> InferenceAPI/nvidia/nvidia/nemotron-3-ultra-preview]
- Turns: 0, Tool calls: 46
- Tokens: 0 in / 0 out [0 total]
- Cost: $0.0000
- Exit: terminated, Duration: 9m 11s
- Log: rogers-zql.3__20260520T065948Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48cb-452f-7791-8e31-a82619f5e1e0
author: oompah
created: 2026-05-21T04:29:02Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48cb-7c51-79ec-a569-e1830efc613b
author: oompah
created: 2026-05-21T04:29:16Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48cc-5200-7266-906a-eeef23a0746b
author: oompah
created: 2026-05-21T04:30:10Z

Understanding: I need to create an IssueTemplatesCheck that:

1. Uses GitHub Contents API to list .github/ISSUE_TEMPLATE/ directory
2. Returns CheckResult with severity (Blocker/Warn/Info) based on what's found
3. Handles edge cases: 404 (no dir), empty array (empty dir), files with .yml/.yaml/.md

Implementation plan:
- Add list_directory() method to GitHubClient for directory listing
- Create src/checks/mod.rs with CheckResult, Severity, Fixability types
- Create src/checks/issue_templates.rs with IssueTemplatesCheck
- Wire into init command in main.rs
- Add unit tests with wiremock
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48d6-caf9-7e4f-acf8-2ed88c16f49b
author: oompah
created: 2026-05-21T04:41:37Z

Discovery: Found the project uses an async GitHubClient in src/github.rs with get_file_contents for directory listing, plus a pre-existing src/init module with fix.rs (has broken raw string literals). Created src/checks/ module with CheckResult/Severity/Fixability types, InitCheck trait, and IssueTemplatesCheck. Added list_directory() and with_base_url() to GitHubClient for testability. My 8 issue_templates tests pass, clippy clean, build passes. Pre-existing src/init/fix.rs has broken tests unrelated to this issue.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48d7-45b0-793d-8923-a9f572b98042
author: oompah
created: 2026-05-21T04:42:08Z

Verification: All 8 unit tests pass (test_no_directory_returns_blocker, test_empty_directory_returns_warn, test_directory_with_only_subdirs_returns_warn, test_directory_with_dirs_and_non_template_files_returns_warn, test_has_templates_returns_info, test_yaml_extension_accepted, test_non_template_extensions_ignored, test_mixed_entries_with_templates). Build passes. Clippy clean. Fmt clean. Branch pushed to origin.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48d7-a4c9-7b4f-b9f5-ac9ec8117326
author: oompah
created: 2026-05-21T04:42:33Z

Agent completed successfully in 810s (4252110 tokens)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48d7-ad78-73a2-9496-ffe4e7174062
author: oompah
created: 2026-05-21T04:42:35Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/qwen/qwen3.6-35b-a3b]
- Turns: 81, Tool calls: 100
- Tokens: 4.2M in / 27.1K out [4.3M total]
- Cost: $0.0000
- Exit: normal, Duration: 13m 30s
- Log: rogers-zql.3__20260521T042917Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
