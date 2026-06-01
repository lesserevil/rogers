---
id: TASK-60
title: 'CRIT-1: Question issue with answer in docs gets doc link comment within one
  triage run'
status: Done
assignee: []
created_date: 2026-05-20 05:25
updated_date: 2026-05-20 09:53
labels:
- rodgers:parent=rogers-4en
- rodgers:type=question-routing
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-977
  state: closed
  parent_id: rogers-4en
  dependencies: []
  branch_name: rogers-977
  target_branch: null
  url: null
  created_at: '2026-05-20T05:25:00Z'
  updated_at: '2026-05-20T09:53:27Z'
  closed_at: '2026-05-20T09:53:17Z'
parent: TASK-7
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/question-routing-plan.md §Step 3a: Documentation Found → Acceptance Criteria CRIT-1

WHAT TO DO
Implement documentation search and answer posting for questions answered in docs.

Create/modify files:
- src/question_router/doc_search.rs - Search docs/ for answer
- src/question_router/mod.rs - Question router main flow
- src/llm/prompts.rs - LLM prompt for drafting doc answer comment

Flow:
1. Question issue labeled 'question' with sufficient context
2. Search docs/**/*.md for relevant content (keyword search, recall-focused)
3. If answer found, LLM drafts warm comment with doc link and summary
4. Post comment: 'Hi @[requestor], thanks! Answer in [docs/file.md §section]...'
5. Close issue if answer complete, leave open if partial

WHY
Most questions are answered in existing docs. Fast doc search + warm response reduces human load and gives requestors immediate answers.

HOW TO VERIFY
- Unit test: Searches docs/ recursively for keywords
- Unit test: Finds relevant doc section with line numbers
- Unit test: LLM drafts warm comment with link format 'docs/file.md §section'
- Unit test: Posts comment via GitHub API
- Unit test: Closes issue if answer complete
- Integration test: End-to-end question→doc answer→comment→close
- Manual: Create question with known doc answer, run router

EDGE CASES AND PITFALLS
- Doc search returns false positives - LLM validates relevance
- Multiple relevant docs - pick most relevant, mention others
- Requestor says doc doesn't answer - treat as doc gap (Step 3b)
- GitHub API failure posting comment - retry, don't lose answer
- Question needs clarification first - ask before search
- Search must complete within one triage run

PROJECT-SPECIFIC TERMINOLOGY
- 'Question router': Component implementing plans/question-routing-plan.md
- 'Doc search': Keyword search over docs/**/*.md
- 'One triage run': Single scheduler tick - all steps must complete
- 'Doc link format': 'docs/filename.md §section-title' (markdown link)
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e44bc-05d3-742a-a777-e5a8876bdb5c
author: oompah
created: 2026-05-20T09:33:53Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44bc-41a2-7ddc-a20d-4a8ffa267d6c
author: oompah
created: 2026-05-20T09:34:09Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44be-1dee-7631-b8d7-3852abca51ad
author: oompah
created: 2026-05-20T09:36:11Z

**Understanding:** Implementing CRIT-1 for the Question Router to find documentation answers and post warm comments. The key components are:
1. doc_search.rs - Recursive keyword search over docs/**/*.md, returning relevant sections with line numbers
2. mod.rs - Main router flow that orchestrates search → LLM draft → comment posting → issue close
3. prompts.rs - LLM prompt for drafting doc answer comments with the format 'docs/file.md §section-title'

**Architecture:** Rodgers is a GitHub-native community relations agent. The question router processes issue-labeled 'question', searches docs, and posts warm comments with doc links.

**Approach:** 
1. Create src/question_router/ module with doc search, main flow, and prompts
2. Create src/llm/ module for LLM context and prompts
3. Add unit tests for doc search, finding relevant sections, LLM drafting, and comment posting
4. Ensure integration with existing project structure
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44cc-8047-747a-9175-f177876a66f6
author: oompah
created: 2026-05-20T09:51:53Z

**Implementation Complete:**

Implemented CRIT-1 for Question Router with documentation search and answer posting:

Created files:
- src/question_router/doc_search.rs - Recursive keyword search over docs/**/*.md
- src/question_router/mod.rs - Question router main flow  
- src/llm/prompts.rs - LLM prompts for drafting doc answer comments
- src/llm/mod.rs - LLM interface and conversation builder
- src/github.rs - GitHub API client for posting comments

Key features:
- Searches docs/ recursively for keyword matches
- Returns results with file path, line numbers, and section titles
- Uses standard doc link format: 'docs/filename.md §section-title'
- Stopword filtering to reduce false positives
- LLM integration for natural language comment drafting

Tests: 22 unit tests pass covering:
- Doc search with keywords
- Section title detection
- Line number tracking  
- Result limiting
- Recursive directory search
- Comment format validation

Quality gates: fmt-check, build, test, clippy all pass
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44cd-d828-7037-ae0a-6dbb7457ddcd
author: oompah
created: 2026-05-20T09:53:21Z

Agent completed successfully in 1167s (3701170 tokens)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44cd-e0a1-7916-811c-dd27597a5edd
author: oompah
created: 2026-05-20T09:53:24Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 64, Tool calls: 79
- Tokens: 3.7M in / 30.1K out [3.7M total]
- Cost: $0.0000
- Exit: normal, Duration: 19m 27s
- Log: rogers-977__20260520T093410Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
