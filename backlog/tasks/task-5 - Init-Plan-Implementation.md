---
id: TASK-5
title: Init Plan Implementation
status: To Do
assignee: []
created_date: 2026-05-20 05:10
updated_date: 2026-05-20 05:43
labels:
- rodgers:type=init
- epic
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: epic
tasks:
  id: rogers-zql
  state: open
  parent_id: null
  dependencies: []
  branch_name: rogers-zql
  target_branch: null
  url: null
  created_at: '2026-05-20T05:10:04Z'
  updated_at: '2026-05-20T05:43:01Z'
  closed_at: null
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/init-plan.md

Acceptance Criteria:
- [ ] AC-1: rogers init --repo owner/repo exits 0 when all blocker checks pass
- [ ] AC-2: rogers init --repo owner/repo exits 1 when any blocker check fails, listing all blockers
- [ ] AC-3: rogers init --repo owner/repo --fix creates missing required labels via GitHub API
- [ ] AC-4: rogers init --repo owner/repo --fix creates missing discussion categories via GitHub API
- [ ] AC-5: rogers init reports blocks for missing issue templates and missing release workflow with specific instructions
- [ ] AC-6: rogers init produces a structured report with severity, description, and fixability for each check
- [ ] AC-7: rogers init is safe to re-run (idempotent: same input = same result)
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e43e3-c97e-7c33-ac24-3879212e901d
author: oompah
created: 2026-05-20T05:37:42Z

Agent dispatched (profile: deep)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e43e3-f1e5-767a-9fdd-a8eb7834cdb6
author: oompah
created: 2026-05-20T05:37:53Z

Focus: Epic Planner
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e43e5-8097-75fb-9eb3-f8eb2d4ea6f9
author: oompah
created: 2026-05-20T05:39:35Z

Understanding: This epic implements 'rogers init' - a project readiness audit command that checks GitHub repositories for Rodgers prerequisites. The CLI structure exists (cli.rs), labels are defined (labels.rs), errors defined (error.rs), but main.rs is a stub and no init implementation exists. I'll decompose this into ~16 actionable child tasks covering: core infrastructure, GitHub API client, 9 audit checks, report formatting, --fix functionality, exit codes, and tests.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e43e7-c1d9-79cb-9d61-dc66aaca6530
author: oompah
created: 2026-05-20T05:42:02Z

Run #1 [attempt=1, profile=deep, role=deep -> InferenceAPI/nvidia/nvidia/nemotron-3-ultra-preview]
- Turns: 0, Tool calls: 17
- Tokens: 0 in / 0 out [0 total]
- Cost: $0.0000
- Exit: terminated, Duration: 4m 15s
- Log: rogers-zql__20260520T053754Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
