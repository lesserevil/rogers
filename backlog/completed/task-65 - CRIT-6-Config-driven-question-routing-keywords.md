---
id: TASK-65
title: 'CRIT-6: Config-driven question routing keywords'
status: Done
assignee: []
created_date: 2026-05-20 05:26
updated_date: 2026-05-21 05:27
labels:
- rodgers:parent=rogers-4en
- rodgers:type=question-routing
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-r40
  state: closed
  parent_id: rogers-4en
  dependencies: []
  branch_name: rogers-r40
  target_branch: null
  url: null
  created_at: '2026-05-20T05:26:23Z'
  updated_at: '2026-05-21T05:27:56Z'
  closed_at: '2026-05-21T05:27:49Z'
parent: TASK-7
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/question-routing-plan.md §Configuration → Acceptance Criteria CRIT-6

WHAT TO DO
Implement config-driven question routing keywords for code search detection.

Create/modify files:
- src/config/schema.rs - Question routing config schema
- src/question_router/config.rs - Load routing keywords
- config.yaml - question_routing configuration section

Configuration keys:
- question_routing.code_search_keywords: List of keywords triggering code search
  Default: ['how does', 'what function', 'which module', 'internals', 'implementation', 'source code', 'walk me through', 'flow of', 'under the hood']
- question_routing.doc_search_path: Docs path to search (default: docs/)
- question_routing.code_search_path: Code path to search (default: **/*)

WHY
Config-driven keywords allow projects to customize what questions trigger code search without code changes. Different projects have different terminology.

HOW TO VERIFY
- Unit test: Config schema validates question_routing keys
- Unit test: Default keywords loaded when not specified
- Unit test: Custom keywords override defaults
- Unit test: Keywords used in code search detection
- Integration test: Question with custom keyword triggers code search
- Manual: Modify config.yaml, verify keywords loaded

EDGE CASES AND PITFALLS
- Empty keyword list - no code search triggered
- Case insensitive matching
- Keyword phrases (multi-word) - match as phrase
- Regex vs literal - use literal for simplicity
- rogers.yaml (repo-level) overrides config.yaml (host-level)
- Keywords also used in LLM prompt for classification

PROJECT-SPECIFIC TERMINOLOGY
- 'Code search keywords': Phrases that trigger source code search
- 'question_routing': Config section for question workflow
- 'Doc search path': Directory for documentation search
- 'Code search path': Glob pattern for code search
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e48f6-0381-78b5-88db-59b1614628e8
author: oompah
created: 2026-05-21T05:15:43Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48f6-1552-7eac-a3dd-03296c9ba290
author: oompah
created: 2026-05-21T05:15:47Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48f7-32e4-7424-accd-b6c1e16cb2b4
author: oompah
created: 2026-05-21T05:17:01Z

## Understanding

I need to implement config-driven question routing keywords for code search detection. Currently,  in  is hardcoded. The task requires:

1. Create  — Config schema for  section
2. Create  — Load routing keywords from config.yaml with defaults
3. Add  section to 

Key requirements:
- 3 config keys: , , 
- Default keywords specified in the issue
- Case-insensitive phrase matching (literal, not regex)
- Empty keyword list means no code search
- rogers.yaml (repo-level) overrides config.yaml (host-level)
- Keywords used in LLM prompt for classification too

I'll create a layered config system: schema → loader → use in code search detection, replacing the hardcoded triggers.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48f8-e2bf-7bfd-a10c-5b4fb2b13cf7
author: oompah
created: 2026-05-21T05:18:51Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/qwen/qwen3.6-35b-a3b]
- Turns: 18, Tool calls: 33
- Tokens: 585.6K in / 11.3K out [596.9K total]
- Cost: $0.0000
- Exit: normal, Duration: 3m 11s
- Log: rogers-r40__20260521T051553Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48f8-f6b5-718c-8962-58e183c7b487
author: oompah
created: 2026-05-21T05:18:56Z

Agent completed successfully in 191s (596932 tokens)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48f9-0006-72f6-9597-63e18cf8059e
author: oompah
created: 2026-05-21T05:18:59Z

Agent completed without closing this issue (191s (596932 tokens)). Escalating from 'default' to 'standard'. Retrying in 10s (1/3).
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48f9-5939-7f16-802a-8035016de277
author: oompah
created: 2026-05-21T05:19:21Z

Agent dispatched (profile: standard)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48f9-6ed7-7da8-9ec5-bb863fdc1d49
author: oompah
created: 2026-05-21T05:19:27Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48fa-f394-71aa-a345-3a8bbc92c1d5
author: oompah
created: 2026-05-21T05:21:06Z

## Understanding

Continuing from the previous agent's work. The config-driven question routing keywords feature is mostly implemented but incomplete:

**Already done:**
-  —  with defaults, , , 
-  — , , comprehensive unit tests
-  — , , code search

**Remaining work:**
1. Add  to  (build fails: cannot find `config` in `crate`)
2. Make `default_code_search_keywords()` public so it can be imported by `code_search.rs`
3. Add `question_routing` section to `config.example.yaml`
4. Update `src/llm/prompts.rs` to use config-driven triggers
5. Run tests and verify
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48ff-0a1f-72e3-a799-554989255b3f
author: oompah
created: 2026-05-21T05:25:34Z

## Implementation

Changes made to complete config-driven question routing keywords:

1. **src/lib.rs** — Added `pub mod config;` to expose the config module so code_search.rs can import `default_code_search_keywords()`
2. **src/config/schema.rs** — Made `default_code_search_keywords()` public (was private `fn`)
3. **src/config/loader.rs** — Fixed `merge_config` to wrap `question_routing` in `Some()`; ensured defaults are applied even when no config files exist; fixed test comparing `PathBuf` with `&str`; fixed test using `?` without Result return type; fixed incorrect test assertion (HOOK → HOOD)
4. **config.example.yaml** — Added `question_routing` section with all 3 config keys documented
5. Ran `cargo fmt` to fix formatter drift

All 38 tests pass. Build, fmt-check, and clippy are clean.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e48ff-3c6a-7407-abf1-608e143c6e7e
author: oompah
created: 2026-05-21T05:25:47Z

## Verification

All 38 tests pass:\n- config::loader::tests: 10 tests (default config, custom keywords, empty keywords, case insensitive, repo override, doc/code path, phrase matching, default keywords list)\n- question_router::code_search::tests: 11 tests (search, match type, is_implementation_question, citations, extensions)\n- question_router::tests: 10 tests (routing, genuine question, reclassification, clarification, areas searched, code search triggers)\n- llm::tests: 4 tests (code search triggers, conversation, parse response, extract elements)\n\nQuality gates: make fmt-check ✓ cargo build ✓ cargo test ✓ cargo clippy ✓
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4900-a46d-7747-bd73-09286ad9bde8
author: oompah
created: 2026-05-21T05:27:19Z

## Completion

Delivered config-driven question routing keywords:

**Schema** (`src/config/schema.rs`): `QuestionRoutingConfig` with `code_search_keywords`, `doc_search_path`, `code_search_path`, defaults, and matching methods.

**Loader** (`src/config/loader.rs`): Layered config loading with merge support (rogers.yaml overrides config.yaml), default guarantees, comprehensive tests.

**Config** (`config.example.yaml`): `question_routing` section with documented keywords and paths.

**Build fixes**: Exposed config module in lib.rs, made default function public, fixed type mismatches.

All 38 tests pass. Quality gates clean. Committed and pushed.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4901-2423-7738-819d-3eb0d22f11a1
author: oompah
created: 2026-05-21T05:27:52Z

Run #2 [attempt=2, profile=standard, role=standard -> InferenceAPI/nvidia/qwen/qwen3.6-27b]
- Turns: 47, Tool calls: 56
- Tokens: 2.0M in / 6.8K out [2.0M total]
- Cost: $0.0000
- Exit: normal, Duration: 8m 32s
- Log: rogers-r40__20260521T051929Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4901-2474-77df-b609-7f2662e04a4f
author: oompah
created: 2026-05-21T05:27:52Z

Agent completed successfully in 512s (2013287 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
