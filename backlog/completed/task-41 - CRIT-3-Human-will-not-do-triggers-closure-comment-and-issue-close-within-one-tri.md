---
id: TASK-41
title: 'CRIT-3: Human will-not-do triggers closure comment and issue close within
  one triage run'
status: Done
assignee: []
created_date: 2026-05-20 05:21
updated_date: 2026-05-20 08:49
labels:
- rodgers:parent=rogers-ykp
- rodgers:type=feature-bug
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-98y
  state: closed
  parent_id: rogers-ykp
  dependencies: []
  branch_name: rogers-98y
  target_branch: null
  url: null
  created_at: '2026-05-20T05:21:58Z'
  updated_at: '2026-05-20T08:49:08Z'
  closed_at: '2026-05-20T08:49:00Z'
parent: TASK-4
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/feature-bug-plan.md §Readiness Phase / Human Decision Gate → Acceptance Criteria CRIT-3

WHAT TO DO
Implement will-not-do handling: human applies label → closure comment + issue close within one triage run.

Create/modify files:
- src/feature_bug/will_not_do.rs - Will-not-do handling
- src/feature_bug/mod.rs - Detect label, trigger closure
- src/triage/triage_loop.rs - Check for will-not-do on triage runs
- src/llm/prompts.rs - LLM prompt for warm closure comment

Flow:
- Human applies will-not-do, removes ready-for-review
- Triage run detects will-not-do label
- LLM drafts warm closure comment: 'Thank you for report. Decided not to pursue. Sorry.'
- Post comment via GitHub API
- Close issue via GitHub API
- All within ONE triage run

WHY
Human decision = final. Fast closure with respectful communication. One triage run = predictable.

HOW TO VERIFY
- Unit test: will-not-do detection triggers closure
- Unit test: LLM drafts warm comment (not curt)
- Unit test: Comment posted via API
- Unit test: Issue closed via API
- Unit test: All within one triage run
- Integration test: Human applies label, run triage, verify closed
- Manual: Apply will-not-do, run triage, verify

EDGE CASES AND PITFALLS
- Comment tone: gratitude + regret, never just 'no'
- Extract human reason from label application comments
- GitHub API failure - retry, don't leave open
- Issue already closed - skip
- Both will-not-do and ready-for-work - will-not-do wins (priority)
- Inform requestor (separate comment on their behalf)

PROJECT-SPECIFIC TERMINOLOGY
- 'will-not-do': Human label = won't pursue
- 'Warm closure comment': LLM-drafted, validated, gratitude + regret
- 'Human decision gate': ready-for-review → will-not-do OR ready-for-work
- 'One triage run': Single scheduler tick
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e4489-144b-794a-8211-adc0687d7bf0
author: oompah
created: 2026-05-20T08:38:15Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4489-4901-7370-bf54-43ab89e719b3
author: oompah
created: 2026-05-20T08:38:28Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e448b-4c47-7d3f-9a82-a14472bab4d8
author: oompah
created: 2026-05-20T08:40:40Z

UNDERSTANDING: Implementing CRIT-3 - Human will-not-do triggers closure comment and issue close within one triage run.

What I found:
-  already defines  label (color: ff4444)
-  already detects  but just flags it as  - no action taken
-  handles transition summaries for completeness flows
-  has prompts for completeness checks but no warm closure prompt

Plan:
1. Create  - handle will-not-do detection, generate warm closure comment
2. Add LLM prompt in  for warm closure comment (gratitude + regret, not curt)
3. Modify  - process will-not-do right away: generate comment, mark for posting, mark for closing
4. Add tests for all the new functionality

The key flow: Human applies will-not-do → triage detects → LLM drafts warm comment → post comment → close issue (all in one run)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4493-05c7-7956-817c-b2e3115baf20
author: oompah
created: 2026-05-20T08:49:07Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 33, Tool calls: 46
- Tokens: 1.2M in / 12.1K out [1.2M total]
- Cost: $0.0000
- Exit: normal, Duration: 10m 50s
- Log: rogers-98y__20260520T083832Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4493-0b17-7f1a-b4b5-ffa50af2c650
author: oompah
created: 2026-05-20T08:49:08Z

Agent completed successfully in 650s (1217185 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
