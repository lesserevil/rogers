---
id: TASK-50
title: 'AC-5: rogers init reports blockers for missing issue templates and release
  workflow'
status: To Do
assignee: []
created_date: 2026-05-20 05:23
updated_date: 2026-05-21 05:06
labels:
- asking_question
- rodgers:parent=rogers-zql
- rodgers:type=init
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-6gd
  state: open
  parent_id: rogers-zql
  dependencies: []
  branch_name: rogers-6gd
  target_branch: null
  url: null
  created_at: '2026-05-20T05:23:29Z'
  updated_at: '2026-05-21T05:06:17Z'
  closed_at: null
parent: TASK-5
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/init-plan.md §Audit Checks → Acceptance Criteria AC-5

WHAT TO DO
Implement init reporting blockers for missing issue templates and release workflow with specific instructions.

Create/modify files:
- src/init/checks.rs - Template and workflow checks
- src/init/report.rs - Blocker reporting with instructions
- src/init/templates.rs - Template check logic
- src/init/workflows.rs - Release workflow check logic

Template check (blocker):
- Check .github/ISSUE_TEMPLATE/ exists with templates
- Blocker if missing: 'Issue templates directory not found'
- Fixability: manual (requires file creation)

Release workflow check (blocker):
- Check .github/workflows/ for release-capable workflow
- Release-capable: job with tag push trigger OR workflow_dispatch with release inputs
- Blocker if missing: 'No release-capable GitHub Actions workflow found'
- Fixability: manual (requires workflow file)

Report includes specific instructions for each.

WHY
Templates and release workflow are fundamental. Clear instructions help users fix quickly.

HOW TO VERIFY
- Unit test: Missing templates → blocker with instruction
- Unit test: Missing release workflow → blocker with instruction
- Unit test: Instructions include direct links/commands
- Integration test: rogers init on fresh repo shows both blockers
- Manual: Fresh repo, run init, verify blocker messages

EDGE CASES AND PITFALLS
- Templates dir exists but empty - blocker
- Workflow exists but not release-capable - blocker
- Warn if workflow exists but no artifact upload
- Instructions: template PR link, workflow template path
- Release workflow template in rogers-templates/github-actions/release.yml

PROJECT-SPECIFIC TERMINOLOGY
- 'Release-capable workflow': Tag trigger OR workflow_dispatch with release inputs
- 'Artifact upload': upload-artifact, gh release upload, docker push, etc.
- 'Fixability manual': Human must create files (not API)
- 'rogers-templates': Embedded template files
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e48eb-16fb-7e98-b30b-12d46067482a
author: oompah
created: 2026-05-21T05:03:47Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48eb-4937-79da-ae86-caaf38415270
author: oompah
created: 2026-05-21T05:04:00Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48ed-587b-72d4-af3c-76725af19526
author: oompah
created: 2026-05-21T05:06:15Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/qwen/qwen3.6-35b-a3b]
- Turns: 8, Tool calls: 19
- Tokens: 181.5K in / 13.7K out [195.3K total]
- Cost: $0.0000
- Exit: ask_question, Duration: 2m 23s
- Log: rogers-6gd__20260521T050403Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48ed-5a36-792d-ba4b-602dc70bc9bf
author: oompah
created: 2026-05-21T05:06:15Z

🤚 **Question from agent:**

The issue says to create files in src/init/ (checks.rs, report.rs, templates.rs, workflows.rs), but the existing codebase already has checks in src/checks/ (mod.rs, issue_templates.rs, labels.rs). Should I follow the existing src/checks/ architecture and add the workflow check there, or create the files under src/init/ as specified in the issue?
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
