---
id: TASK-62
title: 'CRIT-3: File doc-gap task only after exhausting docs and code search'
status: Done
assignee: []
created_date: 2026-05-20 05:25
updated_date: 2026-05-20 10:05
labels:
- rodgers:parent=rogers-4en
- rodgers:type=question-routing
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-des
  state: closed
  parent_id: rogers-4en
  dependencies: []
  branch_name: rogers-des
  target_branch: null
  url: null
  created_at: '2026-05-20T05:25:50Z'
  updated_at: '2026-05-20T10:05:54Z'
  closed_at: '2026-05-20T10:05:46Z'
parent: TASK-7
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/question-routing-plan.md §Step 3b: No Answer Found → Acceptance Criteria CRIT-3

WHAT TO DO
Implement doc-gap task filing ONLY after exhausting docs AND code search.

Create/modify files:
- src/question_router/doc_gap.rs - Doc-gap task filing
- src/question_router/mod.rs - Only file task after both searches fail
- src/tasks/client.rs - File chore task with rodgers:type=docs

Flow (ONLY when both fail):
1. Doc search: no answer in docs/
2. Code search: no answer in code (or question not implementation-related)
3. File chore task (rodgers:type=docs):
   - Title: 'Answer question: [one-line restatement]'
   - Description: full question + context + acceptance (new doc section)
   - discovered-from: link to GitHub issue
4. Post acknowledgment comment
5. Label issue 'needs-documentation', remove 'question'

WHY
Doc-gap tasks track documentation work for external contributors. Only file when truly no answer exists - prevents noise and respects external contributor time.

HOW TO VERIFY
- Unit test: Doc-gap task ONLY filed after both searches
- Unit test: Task type=chore, tag=rodgers:type=docs
- Unit test: Task description has full question + acceptance
- Unit test: discovered-from links to GitHub issue
- Unit test: Acknowledgment comment posted
- Unit test: Labels updated (needs-documentation, remove question)
- Integration test: Question with no answer→doc-gap task
- Manual: Ask novel question, verify task filed correctly

EDGE CASES AND PITFALLS
- Code found but question beyond code - treat as doc gap
- Question about design intent - treat as doc gap
- GitHub API failure filing task - retry, don't lose
- Multiple questions in issue - separate task per question
- Requestor adds context after - restart from Step 1
- Never close question without answer OR doc-gap task (CRIT-5)

PROJECT-SPECIFIC TERMINOLOGY
- 'Doc-gap task': Chore task (rodgers:type=docs) for documentation work
- 'External actor': Contributor who writes doc (not Rodgers)
- 'discovered-from': Task dependency linking to source issue
- 'Acceptance': New doc section answering question, linked from issue
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e44cf-9951-71f8-8fdf-da20040aaeb9
author: oompah
created: 2026-05-20T09:55:16Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44cf-ab1a-70af-91e6-cc2829364e70
author: oompah
created: 2026-05-20T09:55:21Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44d0-9d55-7954-8093-4079875a6cf8
author: oompah
created: 2026-05-20T09:56:23Z

**Understanding:** Implementing CRIT-3: Doc-gap task filing ONLY after exhausting both docs AND code search. The flow is:
1. Doc search fails (no answer in docs/)
2. Code search fails OR question not implementation-related
3. File chore task (rodgers:type=docs) with proper metadata
4. Post acknowledgment comment on GitHub issue
5. Update labels: add 'needs-documentation', remove 'question'

**Planned approach:**
- Create  - GitHub API client for filing tasks with rodgers:type=docs
- Create  - Module for question routing (Step 3b flow)
- Create  - Doc-gap task filing logic
- Add unit tests covering the doc-gap filing flow

**Key requirements from CRIT-3:**
- Task type=chore, tag=rodgers:type=docs
- Task title: 'Answer question: [one-line restatement]'
- Task description: full question + context + acceptance
- discovered-from: link to GitHub issue
- Acknowledgment comment posted
- Labels updated (needs-documentation, remove question)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44d9-4300-719d-bc1c-d6aec2ba12f9
author: oompah
created: 2026-05-20T10:05:50Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 42, Tool calls: 55
- Tokens: 1.6M in / 18.1K out [1.7M total]
- Cost: $0.0000
- Exit: normal, Duration: 10m 34s
- Log: rogers-des__20260520T095527Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44d9-469e-71f2-b2a9-5d2a8e77aefd
author: oompah
created: 2026-05-20T10:05:51Z

Agent completed successfully in 634s (1658496 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
