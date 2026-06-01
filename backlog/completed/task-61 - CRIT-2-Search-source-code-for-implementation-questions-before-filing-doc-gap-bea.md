---
id: TASK-61
title: 'CRIT-2: Search source code for implementation questions before filing doc-gap
  task'
status: Done
assignee: []
created_date: 2026-05-20 05:25
updated_date: 2026-05-20 10:20
labels:
- rodgers:parent=rogers-4en
- rodgers:type=question-routing
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-m9p
  state: closed
  parent_id: rogers-4en
  dependencies: []
  branch_name: rogers-m9p
  target_branch: null
  url: null
  created_at: '2026-05-20T05:25:09Z'
  updated_at: '2026-05-20T10:20:48Z'
  closed_at: '2026-05-20T10:20:40Z'
parent: TASK-7
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/question-routing-plan.md §Step 2: Search Documentation and Code → Acceptance Criteria CRIT-2

WHAT TO DO
Implement source code search for implementation questions before filing doc-gap.

Create/modify files:
- src/question_router/code_search.rs - Search codebase for implementation answers
- src/question_router/mod.rs - Integrate code search before doc-gap
- src/llm/prompts.rs - LLM prompt to determine if code search warranted

Code search triggers (keywords in question):
- 'how does', 'what function', 'which module', 'internals', 'implementation'
- 'source code', 'can you walk me through', 'flow of', 'under the hood'
- Specific function/class/module names

Search scope:
- All code files in repository (no language exclusion)
- Filenames, function/struct names, comments, docstrings, logic comments

WHY
Implementation questions can be answered from code directly. Code is canonical - no doc gap exists if code answers it. Prevents unnecessary doc-gap tasks.

HOW TO VERIFY
- Unit test: Detects code-search keywords in question
- Unit test: Searches all code files (glob **/*)
- Unit test: Finds function/struct names, comments
- Unit test: LLM explains code in plain language with file:line citations
- Unit test: Closes issue if code answer complete
- Unit test: Does NOT file doc-gap when code answers
- Integration test: Implementation question→code search→explanation→close
- Manual: Ask 'how does X work', verify code answer

EDGE CASES AND PITFALLS
- Question about design intent not mechanics - may need doc gap
- Code spans multiple files - explain flow, link all relevant
- Complex code - partial explanation, offer to continue
- LLM hallucination - validate code citations exist
- Search all languages - no exclusion
- Must complete within one triage run

PROJECT-SPECIFIC TERMINOLOGY
- 'Code search': search_code tool over entire repository
- 'Implementation question': Asks how code works internally
- 'Plain-language explanation': Non-technical summary with citations
- 'File:line citation': Source reference like 'src/foo.rs:123-145'
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e44cf-6f6d-70da-9f22-f23bfefd5f92
author: oompah
created: 2026-05-20T09:55:06Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44cf-a38f-7345-bcc7-4e87b7a4836e
author: oompah
created: 2026-05-20T09:55:19Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44d2-537b-73c8-a8ec-acbf53396836
author: oompah
created: 2026-05-20T09:58:15Z

**Understanding:** Implementing CRIT-2 for Question Router to search source code for implementation questions before filing doc-gap tasks.

**Requirements:**
-  - Search codebase for implementation answers
-  - Integrate code search before doc-gap
-  - LLM prompt to determine if code search warranted

**Code search triggers (keywords):**
- 'how does', 'what function', 'which module', 'internals', 'implementation'
- 'source code', 'can you walk me through', 'flow of', 'under the hood'
- Specific function/class/module names

**Search scope:**
- All code files in repository (no language exclusion)
- Filenames, function/struct names, comments, docstrings, logic comments

**Architecture:**
When a question contains implementation-related keywords, the router should search the source code before filing a doc-gap task. If code answers the question, post a plain-language explanation with file:line citations, then close the issue.

**Approach:**
1. Create a code_search module that recursively searches all code files
2. Detect implementation keywords in question text
3. Add LLM prompt to explain code in plain language
4. Integrate into question router flow BEFORE doc-gap filing
5. Validate code citations exist before response

Will add  dependency for file pattern matching.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44d2-e722-7e9c-ae2c-704ca739ad1c
author: oompah
created: 2026-05-20T09:58:53Z

**Discovery:** The project structure is mostly skeleton - question_router directory exists but is empty. Need to create the full implementation from scratch.

**Key findings:**
- src/llm/ module does not exist yet
- src/github/ module does not exist yet  
- Cargo.toml has reqwest for HTTP but needs glob for file pattern matching
- Error types are defined in error.rs using thiserror
- Labels are defined in labels.rs

**Plan:**
1. Update Cargo.toml with  dependency for file pattern matching
2. Create  with LLM prompt for code question routing
3. Create  with LLM interface  
4. Create  with code search implementation
5. Create  integrating all components
6. Add comprehensive unit tests
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44e6-b062-7b14-a525-40a76067af24
author: oompah
created: 2026-05-20T10:20:30Z

**Verification Complete:**

All tests pass (29 tests):
- fmt-check: passed (minor formatting applied)
- build: passed with minor pre-existing warnings
- test: 27 lib tests + 2 binary tests all pass
- clippy: passed with minor pre-existing warnings in main.rs

**Implementation delivered:**
1.  - Core code search with glob pattern matching
2.  - Question router with code/doc search integration
3.  - LLM prompts for code routing
4.  - LLM interface

**Key functionality:**
- Code search triggers: 'how does', 'what function', 'which module', 'internals', 'implementation', 'source code', 'walk me through', 'flow of', 'under the hood'
- Searches all code files recursively
- Finds function definitions, struct definitions, comments, docstrings, filenames
- Generates plain-language explanations with file:line citations
- Validates citations exist in codebase before including in response
- Integrates BEFORE doc-gap filing in question routing flow (Step 3b)

**Acceptance Criteria (CRIT-2):**
When a question issue exists and no docs answer it, Rodgers searches source code if question is about implementation details (keywords) before filing doc-gap task - ✓ Implemented
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44e6-eed9-73a0-8f7f-7fe859c5746a
author: oompah
created: 2026-05-20T10:20:46Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 90, Tool calls: 106
- Tokens: 5.7M in / 40.9K out [5.7M total]
- Cost: $0.0000
- Exit: normal, Duration: 25m 42s
- Log: rogers-m9p__20260520T095524Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44e6-fc06-7edb-9e8f-ec418aaa86da
author: oompah
created: 2026-05-20T10:20:49Z

Agent completed successfully in 1542s (5709894 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
