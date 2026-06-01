---
id: TASK-14
title: 'AC-5: Triage engine with LLM classification and structured output validation'
status: Done
assignee: []
created_date: 2026-05-20 05:17
updated_date: 2026-05-20 08:38
labels:
- rodgers:parent=rogers-6ny
- rodgers:type=architecture
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-a7d
  state: closed
  parent_id: rogers-6ny
  dependencies: []
  branch_name: rogers-a7d
  target_branch: null
  url: null
  created_at: '2026-05-20T05:17:58Z'
  updated_at: '2026-05-20T08:38:22Z'
  closed_at: '2026-05-20T08:38:07Z'
parent: TASK-1
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/architecture-plan.md §System Components / Triage Engine → Acceptance Criteria AC-5

WHAT TO DO
Implement triage engine with LLM classification and structured output validation.

Create/modify files:
- src/triage/engine.rs - Triage engine core
- src/triage/classifier.rs - LLM classification
- src/llm/validator.rs - Structured Output Validator
- src/llm/prompts.rs - Classification prompts
- src/triage/state_machine.rs - State machine (triage-workflow-plan.md)

Components:
- LLM Runtime: OpenAI-compatible API for classification
- Structured prompts: classification, completeness, response drafting
- Structured Output Validator: validates LLM JSON before action
- State machine: plans/triage-workflow-plan.md implementation

Classification output (validated):
- issue_type: bug|feature|question|docs|chore|unknown
- completeness: complete|incomplete
- missing_fields: list
- severity/priority: for bug/feature
- response_draft: comment to post

WHY
LLM = reasoning engine. Structured output = safe actions. Validator = safety net.

HOW TO VERIFY
- Unit test: LLM classifies issue types
- Unit test: Structured output validated
- Unit test: Invalid LLM output rejected
- Unit test: State machine transitions correct
- Unit test: Completeness check works
- Integration test: End-to-end triage
- Manual: Feed issues, verify classification

EDGE CASES AND PITFALLS
- LLM provider configurable (llm.base_url)
- Prompt grounding: plan files, AGENTS.md, history
- Validator: JSON schema + business rules
- Warmth principle: responses validated for tone
- Bot issues: skip classification
- Rate limit on LLM calls

PROJECT-SPECIFIC TERMINOLOGY
- 'Triage engine': Classification + state machine
- 'LLM classification': Issue type + completeness
- 'Structured Output Validator': Validates LLM before action
- 'State machine': triage-workflow-plan.md
- 'Warmth principle': Fred Rogers tone
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e4464-74a4-79c7-8830-1a30fd1587cd
author: oompah
created: 2026-05-20T07:58:15Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4464-9730-7331-8edb-2f8b6b109466
author: oompah
created: 2026-05-20T07:58:24Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4465-894c-7961-a48f-e8f3b57f3a3b
author: oompah
created: 2026-05-20T07:59:26Z

Understanding: Building triage engine with LLM classification and structured output validation. Need to create: engine.rs, classifier.rs, validator.rs, prompts.rs, and state_machine.rs. Key components: LLM runtime, structured prompts, JSON validator, state machine following triage-workflow-plan.md. Classification output includes issue_type, completeness, missing_fields, severity, and response_draft. Edge cases: bot issues, rate limiting, configurable LLM provider.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4488-5ad5-7eb9-99c1-d1971381b677
author: oompah
created: 2026-05-20T08:37:27Z

Implementation complete. Created triage engine with LLM classification and structured output validation:

Components:
- LLM client (OpenAI-compatible) for chat completions
- Structured output validator with JSON schema + warmth principle checks
- Classification prompts following Fred Rogers tone
- State machine matching plans/triage-workflow-plan.md
- Triage engine with action generation

Key features:
- Bot detection and bot_labels application
- Bug/feature/question classification with completeness checking
- Missing fields detection for incomplete issues
- Stale handling (14-day ping, 28-day close)
- Human decision gates (ready-for-review, will-not-do, ready-for-work)
- Warmth-validated response drafts

All 143 tests passing.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4489-190c-76ee-b089-78f6691cfc58
author: oompah
created: 2026-05-20T08:38:16Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 77, Tool calls: 88
- Tokens: 5.8M in / 38.5K out [5.8M total]
- Cost: $0.0000
- Exit: normal, Duration: 40m 1s
- Log: rogers-a7d__20260520T075826Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4489-2069-7b0f-881d-1e366cda072c
author: oompah
created: 2026-05-20T08:38:18Z

Agent completed successfully in 2401s (5845028 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
