---
id: TASK-79
title: 'CRIT-7: Mark issues triaged with rodgers:triaged label'
status: Done
assignee: []
created_date: 2026-05-20 05:28
updated_date: 2026-05-21 07:08
labels:
- rodgers:parent=rogers-jh3
- rodgers:type=triage-workflow
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-4d3
  state: closed
  parent_id: rogers-jh3
  dependencies: []
  branch_name: rogers-4d3
  target_branch: null
  url: null
  created_at: '2026-05-20T05:28:07Z'
  updated_at: '2026-05-21T07:08:51Z'
  closed_at: '2026-05-21T07:08:40Z'
parent: TASK-9
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/triage-workflow-plan.md §State Descriptions / NEW / UNCLASSIFIED

WHAT TO DO
Implement the triaged label marking logic that applies 'rodgers:triaged' label (or 'rodgers:triaged=true') to every issue processed by triage.

Create/modify files:
- src/triage/triage_loop.rs - Apply triaged label at end of processing each issue
- src/triage/mod.rs - Ensure label applied for all code paths

Behavior:
- Every issue processed by triage gets 'rodgers:triaged' label added
- Subsequent triage runs only process issues with rodgers:triaged=false (or missing)
- This enables idempotent triage - safe to re-run, same input = same result
- Label applied atomically with other triage labels

WHY
Without tracking which issues have been triaged, every run would reprocess all issues, wasting API calls and potentially causing duplicate actions. The triaged label enables incremental processing.

HOW TO VERIFY
- Unit test: Processed issue gets rodgers:triaged label
- Unit test: Second triage run skips already-triaged issues
- Unit test: Issues with rodgers:triaged=true not reprocessed
- Integration test: Hourly triage only processes new/changed issues
- Manual: Run triage twice, verify second run is fast (no work)

EDGE CASES AND PITFALLS
- Must apply label even if triage makes no other changes (issue was already correct)
- Label must be applied before any early returns/errors in triage
- GitHub API failure applying label should not fail entire triage run (log and continue)
- Bot issues get bot_labels then skipped - still mark triaged
- Label format: 'rodgers:triaged' (preferred) or 'rodgers:triaged=true'

PROJECT-SPECIFIC TERMINOLOGY
- 'rodgers:triaged': Label marking issue as processed by triage (idempotency key)
- 'Idempotent triage': Running triage multiple times produces same result
- 'Triage run': One scheduler tick processing all eligible issues
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e4957-3748-741f-9abb-7165b2c97ef1
author: oompah
created: 2026-05-21T07:01:53Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4957-49df-7883-858f-c7cb492ffea1
author: oompah
created: 2026-05-21T07:01:58Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4957-ebf6-7106-bd65-1dddb8b24f27
author: oompah
created: 2026-05-21T07:02:39Z

## Understanding

**CRIT-7: Mark issues triaged with rodgers:triaged label**

The goal is to implement idempotent triage by:
1. Adding a  label constant
2. Applying this label at the end of processing every issue that triage processes (not skipped ones)
3. Filtering issues to only process those WITHOUT the  label (or where it's false)
4. This ensures that a second triage run skips already-processed issues

Key insight: The  function in  already returns a  with . I need to:
- Add  constant () to the triage module
- Apply the triaged label to every issue that gets  (after all existing processing)
- Ensure issues already marked  are filtered out before processing (in the batch fetch/scheduler layer)
- Handle edge cases: bot issues, early returns, API failures should log but not fail

Files to modify:
-  — Add triaged label constant, apply label in process_issue for processed issues
-  — Export the new constant
-  — Add to RODGERS_REQUIRED_LABELS
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e495c-a205-7abf-a400-a0c94570fa1a
author: oompah
created: 2026-05-21T07:07:48Z

## Implementation

Added triaged label marking logic across three files:

**src/triage/triage_loop.rs:**
- Added  constant = 
- Added  helper function
- Added  to  enum
- Modified : skip issues with triaged label (return )
- Modified all  code paths to include  label:
  - Will-Not-Do path: applies triaged + removes ready-for-review
  - Ready-For-Work path: applies triaged label
  - Completeness check path: applies triaged label (idempotent - checks it's not already present)
- Added 14 new unit tests for triaged label behavior

**src/triage/mod.rs:**
- Exported  constant and  function

**src/labels.rs:**
- Added  label definition with color  and description
- Added to  list

Key design decisions:
- Triaged label is applied at the END of processing (after all other labels) via the labels_to_add mechanism
- Issues already marked triaged are detected BEFORE processing begins, skipping them entirely
- Label applied atomically with other triage labels (same TriageResult)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e495c-ced7-7479-b8fb-5d681ceb6110
author: oompah
created: 2026-05-21T07:08:00Z

## Verification

All quality gates passed:

- make fmt-check: PASSED
- cargo build: PASSED
- cargo test --all: 242/242 PASSED (0 failed)
  - 14 new tests for triaged label functionality
  - All existing tests still pass
- cargo clippy --all -- -D warnings: PASSED (0 warnings)

New test coverage includes:
- test_processed_issue_gets_triaged_label
- test_second_triage_run_skips_already_triaged_issues
- test_issue_with_triaged_true_not_reprocessed
- test_triaged_label_applied_with_will_not_do
- test_triaged_label_applied_with_ready_for_work
- test_triaged_label_applied_with_needs_information
- test_triaged_label_always_applied_when_processed
- test_skipped_paths_dont_get_triaged_label
- test_triaged_label_applied_even_with_no_other_changes
- test_batch_skips_already_triaged_issues
- test_has_triaged_label_true/false/empty
- test_triaged_issue_has_triaged_label_constant
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e495d-8216-7837-aebc-c1f5cc110bfe
author: oompah
created: 2026-05-21T07:08:45Z

Agent completed successfully in 414s (3119754 tokens)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e495d-82ed-7e28-95f5-96185c5bb684
author: oompah
created: 2026-05-21T07:08:46Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/qwen/qwen3.6-35b-a3b]
- Turns: 42, Tool calls: 50
- Tokens: 3.1M in / 14.5K out [3.1M total]
- Cost: $0.0000
- Exit: normal, Duration: 6m 54s
- Log: rogers-4d3__20260521T070200Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
