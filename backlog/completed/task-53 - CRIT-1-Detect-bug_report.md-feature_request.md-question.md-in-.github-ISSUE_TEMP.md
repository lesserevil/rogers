---
id: TASK-53
title: 'CRIT-1: Detect bug_report.md, feature_request.md, question.md in .github/ISSUE_TEMPLATE/
  on startup'
status: Done
assignee: []
created_date: 2026-05-20 05:23
updated_date: 2026-05-20 07:54
labels:
- rodgers:parent=rogers-d7h
- rodgers:type=issue-templates
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-rva
  state: closed
  parent_id: rogers-d7h
  dependencies: []
  branch_name: rogers-rva
  target_branch: null
  url: null
  created_at: '2026-05-20T05:23:53Z'
  updated_at: '2026-05-20T07:54:08Z'
  closed_at: '2026-05-20T07:54:01Z'
parent: TASK-6
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/issue-templates-plan.md §Template Discovery → Acceptance Criteria CRIT-1

WHAT TO DO
Implement template discovery on startup - check for bug_report.md, feature_request.md, question.md in .github/ISSUE_TEMPLATE/.

Create/modify files:
- src/templates/discovery.rs - Template discovery on startup
- src/templates/mod.rs - Templates module entry point
- src/init/mod.rs - Call discovery during rogers init

Discovery logic:
- List .github/ISSUE_TEMPLATE/ directory
- Check for: bug_report.md/.yml, feature_request.md/.yml, question.md/.yml
- Support both .md (legacy) and .yml (GitHub forms) formats
- Report found/missing to init audit

WHY
Template discovery enables Rodgers to adopt project's existing templates or provide defaults. Required for init audit and triage template conformance checking.

HOW TO VERIFY
- Unit test: Discovers .md templates in .github/ISSUE_TEMPLATE/
- Unit test: Discovers .yml templates in .github/ISSUE_TEMPLATE/
- Unit test: Reports all three found
- Unit test: Reports missing templates individually
- Unit test: Handles directory not found
- Integration test: rogers init reports template status
- Manual: Create templates, run init, verify detection

EDGE CASES AND PITFALLS
- Directory doesn't exist - report all missing
- Only some templates exist - report which missing
- Invalid template format - warn, treat as missing
- Subdirectories in ISSUE_TEMPLATE - search recursively
- Case insensitive filename matching
- Runs on every startup (scheduler tick), not just init

PROJECT-SPECIFIC TERMINOLOGY
- 'Template discovery': Startup scan of .github/ISSUE_TEMPLATE/
- 'Canonical templates': bug_report, feature_request, question
- 'Template marker': Hidden comment in template to detect conformance
- 'Auto_suggest': Config to file task with defaults when none found
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e4453-3d7f-79cf-8a2b-24060222f82f
author: oompah
created: 2026-05-20T07:39:26Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4453-5b8e-7401-8548-c6d955499336
author: oompah
created: 2026-05-20T07:39:34Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4454-ab0b-7e19-a594-c83c5ad312bd
author: oompah
created: 2026-05-20T07:41:00Z

Understanding: Implementing template discovery on startup to detect bug_report.md/.yml, feature_request.md/.yml, and question.md/.yml in .github/ISSUE_TEMPLATE/. Need to create templates module with discovery logic and wire it into init audit.\n\nPlan:\n1. Create src/templates/discovery.rs - Template discovery logic that lists .github/ISSUE_TEMPLATE/ directory, supports both .md and .yml formats, performs case-insensitive matching, searches subdirectories recursively\n2. Create src/templates/mod.rs - Templates module entry point exposing discovered templates status\n3. Create src/init/mod.rs - Init audit module that calls discovery and reports found/missing templates\n\nImplementation notes:\n- Use std::fs for directory listing\n- Canonical templates: bug_report, feature_request, question\n- Report each template as found/missing individually\n- Handle directory-not-found gracefully
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e445e-755a-70b4-974e-2b3c1e9613e0
author: oompah
created: 2026-05-20T07:51:42Z

Verification: All 17 tests pass:\n- test_discover_md_templates ✓\n- test_discover_yml_templates ✓\n- test_discover_mixed_templates ✓\n- test_reports_all_three_found ✓\n- test_reports_missing_templates_individually ✓\n- test_handles_directory_not_found ✓\n- test_searches_subdirectories ✓\n- test_case_insensitive_matching ✓\n- test_validate_* ✓ (4 tests)\n- test_audit_blocks_missing_templates ✓\n- test_audit_reports_found_templates ✓\n- test_audit_reports_individual_missing ✓\n- test_format_audit_result ✓\n- test_audit_result_counts ✓\n\nQuality gates: fmt-check ✓, cargo build ✓, cargo test ✓, cargo clippy ✓
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4460-5e2c-751e-ba2b-e5dcd2d76faf
author: oompah
created: 2026-05-20T07:53:47Z

Completion: Template discovery implemented per CRIT-1.\n\nDelivered:\n- src/templates/discovery.rs - Template discovery logic\n- src/templates/mod.rs - Templates module entry point  \n- src/init/audit.rs - Init audit with template reporting\n- src/init/mod.rs - Init module entry point\n- 17 unit tests passing\n\nDetection covers:\n✓ bug_report.md/.yml\n✓ feature_request.md/.yml  \n✓ question.md/.yml\n✓ Both .md (legacy) and .yml (GitHub forms) formats\n✓ Case-insensitive matching\n✓ Recursive subdirectory search\n✓ Missing template reporting\n\nBranch pushed: rogers-rva
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4460-a9bd-7331-8c7b-6a05c4dd0299
author: oompah
created: 2026-05-20T07:54:06Z

Agent completed successfully in 877s (2287402 tokens)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4460-aea3-79da-83a2-51ac87329d29
author: oompah
created: 2026-05-20T07:54:07Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 58, Tool calls: 67
- Tokens: 2.3M in / 18.2K out [2.3M total]
- Cost: $0.0000
- Exit: normal, Duration: 14m 37s
- Log: rogers-rva__20260520T073941Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
