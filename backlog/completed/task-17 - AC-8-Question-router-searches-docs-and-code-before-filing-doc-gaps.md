---
id: TASK-17
title: 'AC-8: Question router searches docs and code before filing doc gaps'
status: Done
assignee: []
created_date: 2026-05-20 05:18
updated_date: 2026-05-20 09:38
labels:
- rodgers:parent=rogers-6ny
- rodgers:type=architecture
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-nnp
  state: closed
  parent_id: rogers-6ny
  dependencies: []
  branch_name: rogers-nnp
  target_branch: null
  url: null
  created_at: '2026-05-20T05:18:23Z'
  updated_at: '2026-05-20T09:38:52Z'
  closed_at: '2026-05-20T09:38:47Z'
parent: TASK-1
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/architecture-plan.md §System Components / Question Router → plans/question-routing-plan.md → Acceptance Criteria AC-8

WHAT TO DO
Implement question router that searches docs and code before filing doc gaps.

Create/modify files:
- src/question_router/router.rs - Question router
- src/question_router/doc_search.rs - Documentation search
- src/question_router/code_search.rs - Code search
- src/question_router/doc_gap.rs - Doc gap task filing
- plans/question-routing-plan.md - Workflow source

Workflow (question-routing-plan.md):
1. Classify as question
2. Search docs/ for answer
3. If impl question: search code
4. Doc found: post link, close if complete
5. Code found: explain, cite file:line, close if complete
6. Neither: file doc-gap task, label needs-documentation

Search scopes:
- Docs: docs/**/*.md
- Code: all files, functions, comments

WHY
Questions answered fast from existing knowledge. Doc gaps tracked for contributors. Code = canonical for impl questions.

HOW TO VERIFY
- Unit test: Doc search finds answers
- Unit test: Code search finds impl answers
- Unit test: Doc answer → link comment
- Unit test: Code answer → explanation + citation
- Unit test: Neither → doc-gap task
- Unit test: Never close without answer OR task
- Integration test: Questions through router
- Manual: Ask questions, verify flow

EDGE CASES AND PITFALLS
- Zero hallucination: cite sources
- Code search keywords: how does, what function, etc.
- Multiple questions: separate tasks
- False positive docs: requestor clarifies
- Requestor adds context: restart
- Non-questions: re-label, reroute

PROJECT-SPECIFIC TERMINOLOGY
- 'Question router': plans/question-routing-plan.md
- 'Doc search': docs/**/*.md keyword search
- 'Code search': all code, functions, comments
- 'Doc-gap task': chore, rodgers:type=docs
- 'Citation': file:line proof
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e44aa-2193-76e8-a243-7d41a4fcc62d
author: oompah
created: 2026-05-20T09:14:21Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44aa-3e3e-7f1d-815e-b67596084521
author: oompah
created: 2026-05-20T09:14:28Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44ab-82e7-7e7c-9eec-d6a420a83c50
author: oompah
created: 2026-05-20T09:15:51Z

## Understanding & Plan

I've analyzed the issue and the codebase architecture. Here's what I understand:

**Workflow to implement:**
1. Question issues (labeled ) go through the Question Router
2. Router searches  for answers
3. For implementation questions (keywords: how does, what function, which module, internals, etc.), also search all source code
4. If doc found: post link, close if complete
5. If code found: explain with file:line citations, close if complete
6. If neither: file  task (rodgers:type=docs), label 

**Key files to create:**
-  directory with: , , , , 
- LLM prompts for question routing decisions

**Existing patterns I'll follow:**
- / structure for the router
-  with chat_structured for LLM calls
-  for filing doc-gap tasks
-  states: SearchDocs, DocFound, DocGap

**Implementation approach:**
1. Create the question_router module
2. Implement DocSearcher (keyword search over docs/**/*.md)
3. Implement CodeSearcher (search all code for implementation details)
4. Implement DocGapFiler (files chore tasks with rodgers:type=docs)
5. Implement Router (orchestrates the workflow, generates TriageActions)
6. Add LLM prompts for routing decisions
7. Add unit tests
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44b2-74b4-754b-82e7-2e92f00e725f
author: oompah
created: 2026-05-20T09:23:27Z

## Implementation Progress

Creating the question router module with the following components:

### Created files:

1. **src/question_router/mod.rs** - Module definitions
2. **src/question_router/doc_search.rs** - Documentation search engine:
   - Loads docs/**/*.md
   - Keyword-based search with relevance scoring
   - Extracts title, section heading, and snippet from matches
   
3. **src/question_router/code_search.rs** - Source code search engine:
   - Loads all source files (configurable)
   - Extracts function/struct names
   - Implementation keyword detection (how does, what function, etc.)
   - Context-aware code snippets

4. **src/question_router/doc_gap.rs** - Doc gap filing:
   - Creates chore tasks with rodgers:type=docs
   - Generates acknowledgment comments
   - Generates doc answer comments
   - Generates code answer comments with citations

5. **src/question_router/router.rs** - Main router orchestrator:
   - Coordinates doc search → code search → doc gap filing
   - Executes routing actions (post comment, update labels, close)
   - Based on TriageEngine/TriageStateMachine patterns

6. **src/llm/prompts.rs** - Added QuestionRoutingPrompt for LLM-based routing decisions

7. **Cargo.toml** - Added walkdir dependency

Next: Add unit tests and verify compilation.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44bf-bf97-78b9-be3e-0783aa4cfc31
author: oompah
created: 2026-05-20T09:37:58Z

## Implementation Complete

All components have been implemented and tests pass:

### Created files:
-  - Module definitions
-  - Documentation search (keyword search over docs/**/*.md)
-  - Source code search (functions, structs, comments)
-  - Doc gap task filing with 
-  - Main router search with relevance scoring over all docs/*.md
2. Code search for implementation details (keywords: how does, what function, etc.)
3. Doc gap filing with proper metadata (chore, rodgers:type=docs, needs-documentation label)
4. Comment generation for doc answers, code explanations, and doc gap acknowledgment
5. Citation support (file:line format)

### Integration points:
- Added QuestionRoutingPrompt to LLM prompts
- Exported TaskController from tasks module
- Added walkdir dependency

### Tests: All 194 tests pass (25 question router tests)

### Remaining work:
- Integration with TriageEngine (AC-7 requires connecting router to the existing triage flow)
- This implementation provides the router; integration into the engine is a separate concern
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44c0-8fb3-70ee-a0aa-017740832114
author: oompah
created: 2026-05-20T09:38:51Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 74, Tool calls: 87
- Tokens: 6.4M in / 38.0K out [6.5M total]
- Cost: $0.0000
- Exit: normal, Duration: 24m 29s
- Log: rogers-nnp__20260520T091432Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44c0-9808-7af1-9859-353c19a52eab
author: oompah
created: 2026-05-20T09:38:53Z

Agent completed successfully in 1469s (6486654 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
