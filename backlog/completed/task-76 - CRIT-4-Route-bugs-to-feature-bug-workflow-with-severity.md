---
id: TASK-76
title: 'CRIT-4: Route bugs to feature-bug workflow with severity'
status: Done
assignee: []
created_date: 2026-05-20 05:27
updated_date: 2026-05-21 06:41
labels:
- rodgers:parent=rogers-jh3
- rodgers:type=triage-workflow
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-mzk
  state: closed
  parent_id: rogers-jh3
  dependencies: []
  branch_name: rogers-mzk
  target_branch: null
  url: null
  created_at: '2026-05-20T05:27:45Z'
  updated_at: '2026-05-21T06:41:20Z'
  closed_at: '2026-05-21T06:41:10Z'
parent: TASK-9
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/triage-workflow-plan.md §Top-Level Classification → routes to plans/feature-bug-plan.md

WHAT TO DO
Implement routing logic that sends classified 'bug' issues to the feature-bug workflow with severity assessment.

Create/modify files:
- src/triage/router.rs - Route bug issues to feature-bug workflow
- src/feature_bug/mod.rs - Feature/bug workflow entry point
- src/triage/severity.rs - Severity assessment (critical/high/medium/low)

Routing behavior:
- Issues classified as 'bug' get 'rodgers:bug' label applied
- Severity assessed via keywords (crash, data loss, security = critical; broken feature = high; minor issue = medium; cosmetic = low) and LLM analysis
- Route to feature-bug workflow for reproduction and fix tracking
- Priority mapped from severity (critical=P1, high=P2, medium=P3, low=P4)

WHY
Bugs need severity assessment to prioritize fixes. Critical bugs (data loss, security, crashes) need immediate attention. Routing to feature-bug workflow enables structured reproduction, fix tracking, and release planning.

HOW TO VERIFY
- Unit test: Bug issue gets rodgers:bug label
- Unit test: Severity keywords correctly map (crash→critical, security→critical, broken→high, minor→medium, cosmetic→low)
- Unit test: LLM severity assessment for ambiguous cases
- Unit test: Priority mapped correctly from severity
- Integration test: Bug routed to feature-bug workflow with severity metadata
- Manual: Create test bug issues with various severities, verify routing

EDGE CASES AND PITFALLS
- Security issues (CVE, GHSA, security label) always critical regardless of keywords
- Must not override existing severity if human-set
- LLM severity assessment must be validated before acting
- Severity affects backport priority (critical/high = priority 1 for backports)

PROJECT-SPECIFIC TERMINOLOGY
- 'rodgers:bug': Label marking issue as bug routed to feature-bug workflow
- 'Severity': critical/high/medium/low - impact assessment
- 'Priority': P1-P4 - work queue priority (derived from severity)
- 'Feature-bug workflow': plans/feature-bug-plan.md implementation
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e493c-7613-7be2-9eab-689c2e8c29f5
author: oompah
created: 2026-05-21T06:32:40Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e493c-a69a-7ac7-901e-557285170608
author: oompah
created: 2026-05-21T06:32:52Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e493d-7e1f-7a07-8600-eb705b8b7ee9
author: oompah
created: 2026-05-21T06:33:47Z

Understanding: This CRIT-4 task requires implementing bug routing with severity assessment. I'll create:\n\n1. **src/triage/severity.rs** - Severity assessment module with:\n   - Severity enum (critical/high/medium/low)\n   - Keyword-based detection (crash/data loss/security→critical, broken→high, minor→medium, cosmetic→low)\n   - Priority mapping (critical=P1, high=P2, medium=P3, low=P4)\n   - LLM analysis hook for ambiguous cases\n\n2. **src/triage/router.rs** (update) - Router enhancement:\n   - Apply  label to bug issues\n   - Include severity assessment in RouteResult\n   - Route to FeatureBug workflow with severity metadata\n\n3. **src/feature_bug/mod.rs** (update) - Entry point enhancement:\n   - Accept severity metadata from router\n   - Track severity in bug issue flow\n\n4. **src/labels.rs** (update) - Add  label definition\n\nEdge cases handled: security labels always critical, human-set severity preserved, LLM validated before acting.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4943-a948-7da3-80fd-89fd8e6cbcaa
author: oompah
created: 2026-05-21T06:40:32Z

Implementation: Created severity module (src/triage/severity.rs) with keyword-based severity detection, priority mapping, and edge case handling. Updated router (src/triage/router.rs) to apply rodgers:bug label and severity metadata when routing bugs. Added severity labels to labels.rs. All 538 tests pass.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4943-da0e-7215-a062-892304e5f861
author: oompah
created: 2026-05-21T06:40:44Z

Discovery: The router module (src/triage/router.rs) already handles bug/feature/question classification but was missing severity assessment. The TriageIssue struct provides title/body/labels needed for severity detection. Severity assessment fits as a new module under src/triage/ alongside router.rs.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4943-feed-7b63-a168-25744a94b53e
author: oompah
created: 2026-05-21T06:40:53Z

Verification: All tests pass (538 total). Specifically: 53 severity tests covering all keyword mappings (crash→critical, data loss→critical, security→critical, broken→high, minor→medium, cosmetic→low), CVE/GHSA always critical, security label always critical, human-set severity respected, priority mapping (P1-P4), backport priority. 25 router tests covering rodgers:bug label application, severity metadata in RouteResult, batch routing with mixed severities. No regressions in existing 272 tests.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4944-50c4-7833-a374-adedb58f30a2
author: oompah
created: 2026-05-21T06:41:14Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/qwen/qwen3.6-35b-a3b]
- Turns: 42, Tool calls: 59
- Tokens: 3.5M in / 20.5K out [3.6M total]
- Cost: $0.0000
- Exit: normal, Duration: 8m 37s
- Log: rogers-mzk__20260521T063256Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4944-5120-75e9-8605-1b058d9bf55b
author: oompah
created: 2026-05-21T06:41:15Z

Agent completed successfully in 517s (3559890 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
