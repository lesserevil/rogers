---
id: TASK-77
title: 'CRIT-5: Route features to feature-bug workflow'
status: Done
assignee: []
created_date: 2026-05-20 05:27
updated_date: 2026-05-21 11:42
labels:
- rodgers:parent=rogers-jh3
- rodgers:type=triage-workflow
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-xp1
  state: closed
  parent_id: rogers-jh3
  dependencies: []
  branch_name: rogers-xp1
  target_branch: null
  url: null
  created_at: '2026-05-20T05:27:50Z'
  updated_at: '2026-05-21T11:42:19Z'
  closed_at: '2026-05-21T11:42:10Z'
parent: TASK-9
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/triage-workflow-plan.md §Top-Level Classification → routes to plans/feature-bug-plan.md

WHAT TO DO
Implement routing logic that sends classified 'feature' issues to the feature-bug workflow with priority assessment.

Create/modify files:
- src/triage/router.rs - Route feature issues to feature-bug workflow
- src/feature_bug/mod.rs - Feature/bug workflow entry point  
- src/triage/priority.rs - Priority assessment (P1-P4)

Routing behavior:
- Issues classified as 'feature' get 'rodgers:feature' label applied
- Priority assessed via keywords (P1: blocker, critical, urgent; P2: important, high value; P3: normal, nice to have; P4: low priority, backlog) and LLM analysis
- Route to feature-bug workflow for spec development and implementation tracking
- Large features (epic-scale) detected at ready-for-work, not here

WHY
Features need priority assessment to order work queue. P1 features block releases or are critical path. Routing to feature-bug workflow enables acceptance criteria generation, epic breakdown, and implementation tracking.

HOW TO VERIFY
- Unit test: Feature issue gets rodgers:feature label
- Unit test: Priority keywords correctly map (blocker→P1, urgent→P1, important→P2, normal→P3, backlog→P4)
- Unit test: LLM priority assessment for ambiguous cases
- Integration test: Feature routed to feature-bug workflow with priority metadata
- Manual: Create test feature issues with various priorities, verify routing

EDGE CASES AND PITFALLS
- Must not override existing priority if human-set
- LLM priority assessment must be validated before acting
- Epic-scale detection happens at ready-for-work (human gate), not during triage
- Feature completeness checked in feature-bug workflow, not triage

PROJECT-SPECIFIC TERMINOLOGY
- 'rodgers:feature': Label marking issue as feature routed to feature-bug workflow
- 'Priority': P1-P4 - work queue priority (P1 highest)
- 'Feature-bug workflow': plans/feature-bug-plan.md implementation
- 'Epic-scale': Work spanning multiple codebase areas, detected at ready-for-work
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e4948-c3a1-7bb5-9f19-c2b51d66d9a1
author: oompah
created: 2026-05-21T06:46:06Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4948-e67c-7cc6-9a4f-02633825d778
author: oompah
created: 2026-05-21T06:46:15Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4949-1e26-767e-bde1-c028fb618e7e
author: oompah
created: 2026-05-21T06:46:29Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/qwen/qwen3.6-35b-a3b]
- Turns: 2, Tool calls: 4
- Tokens: 23.9K in / 289 out [24.2K total]
- Cost: $0.0000
- Exit: ask_question, Duration: 22s
- Log: rogers-xp1__20260521T064617Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4949-200b-71d8-aead-621d968cda7f
author: oompah
created: 2026-05-21T06:46:30Z

🤚 **Question from agent:**

The issue requires implementing routing logic for feature issues (CRIT-5), but the triage workflow plan has CRIT-5 defined for a different purpose (epic task filing). Should I implement what the issue description explicitly asks: routing features to the feature-bug workflow with priority assessment and applying 'rodgers:feature' labels?
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4a50-a819-732c-a1b4-2e3383a35d38
author: user
created: 2026-05-21T11:34:20Z

You should implement what the issue description explicitly asks.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4a51-4d99-7433-830c-162e4fc1c2c6
author: oompah
created: 2026-05-21T11:35:03Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4a51-5d42-7f3b-91ad-cdbde45e3f7f
author: oompah
created: 2026-05-21T11:35:07Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4a52-3167-7b55-b6a7-6b271c5aaa41
author: oompah
created: 2026-05-21T11:36:01Z

## Understanding

I will implement feature-to-feature-bug workflow routing with priority assessment:

1. **** - Router module that:
   - Applies  label to classified feature issues
   - Routes features to feature-bug workflow with priority metadata
   - Handles the transition from triage classification to feature-bug processing

2. **** - Priority assessment module that:
   - Defines P1-P4 priority levels with keyword mappings (blocker→P1, critical→P1, urgent→P1, important→P2, high value→P2, normal→P3, nice to have→P3, low priority→P4, backlog→P4)
   - Performs keyword-based priority assessment from issue body
   - Preserves human-set priorities (never overrides)
   - Provides LLM assessment hook for ambiguous cases

3. **Update ** - Export the new router and priority modules

4. **Tests** - Unit tests for all three verification criteria from the issue:
   - Feature issue gets rodgers:feature label
   - Priority keywords correctly map
   - LLM priority assessment for ambiguous cases
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4a52-55f8-7deb-9c8c-220a6f2562b2
author: oompah
created: 2026-05-21T11:36:10Z

## Discovery

Found the existing architecture:
- src/triage/mod.rs - exports triage_loop with process_issue
- src/triage/triage_loop.rs - main triage logic, currently checks for bug/feature labels and runs completeness
- src/feature_bug/mod.rs - already has completeness, breakdown, will_not_do modules with extensive tests
- src/labels.rs - defines all required labels
- src/triage/triage_loop.rs already has LABEL_FEATURE and processes feature issues via completeness check

The routing gap: features that complete triage go through completeness check but there is no explicit routing step that applies rodgers:feature label or tracks priority. The feature-bug workflow exists but the router from triage to feature-bug is missing.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4a57-e6bf-7a38-9e53-5729324567c3
author: oompah
created: 2026-05-21T11:42:15Z

Agent completed successfully in 434s (2914651 tokens)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4a57-eb51-7d78-b041-a7f22bb81199
author: oompah
created: 2026-05-21T11:42:16Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/qwen/qwen3.6-35b-a3b]
- Turns: 33, Tool calls: 43
- Tokens: 2.9M in / 19.3K out [2.9M total]
- Cost: $0.0000
- Exit: normal, Duration: 7m 14s
- Log: rogers-xp1__20260521T113511Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
