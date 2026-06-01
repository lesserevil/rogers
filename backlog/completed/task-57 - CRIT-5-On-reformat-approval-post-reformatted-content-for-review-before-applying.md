---
id: TASK-57
title: 'CRIT-5: On reformat approval, post reformatted content for review before applying'
status: Done
assignee: []
created_date: 2026-05-20 05:24
updated_date: 2026-05-20 09:47
labels:
- rodgers:parent=rogers-d7h
- rodgers:type=issue-templates
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-6tf
  state: closed
  parent_id: rogers-d7h
  dependencies: []
  branch_name: rogers-6tf
  target_branch: null
  url: null
  created_at: '2026-05-20T05:24:30Z'
  updated_at: '2026-05-20T09:47:30Z'
  closed_at: '2026-05-20T09:47:18Z'
parent: TASK-6
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/issue-templates-plan.md §Template Conformance / Rodgers Offer to Reformat → Acceptance Criteria CRIT-5

WHAT TO DO
Implement reformat review flow: on approval, post reformatted content for review BEFORE applying.

Create/modify files:
- src/templates/reformat.rs - Generate reformatted content, post for review
- src/templates/mod.rs - Handle review approval flow
- src/github/client.rs - post_comment, update_issue

Flow:
1. Requestor approves reformat offer
2. Rodgers maps issue content to template fields
3. Posts reformatted version as COMMENT for review
4. Asks: 'Does this look right? If so, I'll update the issue.'
5. On second approval: update issue body
6. Remove needs-information label if present

WHY
Two-step review prevents mistakes. Requestor sees exact result before issue modified. Builds trust.

HOW TO VERIFY
- Unit test: Reformatted content posted as comment first
- Unit test: Asks for confirmation before update
- Unit test: Updates issue only on second approval
- Unit test: Removes needs-information on success
- Integration test: Full approval→review→update flow
- Manual: Approve reformat, verify review comment, verify update

EDGE CASES AND PITFALLS
- Requestor rejects review - keep original, triage continues
- Requestor edits review comment - detect, ask again
- GitHub API failure on update - retry
- Large issue body - comment size limits
- Template field mapping: semantic not exact name match
- Partial template use - only fill missing fields

PROJECT-SPECIFIC TERMINOLOGY
- 'Reformatted content': Issue body rewritten with template structure
- 'Review comment': Reformatted version posted for approval
- 'Second approval': Confirmation to apply reformatted version
- 'Field mapping': Semantic mapping (environment→Environment, etc.)
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e44b9-b0d4-76e3-bda1-d60ac39cedf8
author: oompah
created: 2026-05-20T09:31:21Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44b9-de88-745a-bc99-b976d19450f1
author: oompah
created: 2026-05-20T09:31:32Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44bb-964d-785e-83b5-010b221afacc
author: oompah
created: 2026-05-20T09:33:25Z

My interpretation of CRIT-5:

The issue requires a **two-step reformat approval flow**:
1. When requestor first approves reformat → Rodgers maps issue content to template fields and posts reformatted version as a COMMENT for review (not yet updating the issue body)
2. When requestor approves the review comment → Rodgers updates the issue body and removes needs-information label

**Current state:** Code already has:
- ApprovalResponse enum (Approved/Declined/Ambiguous/NoResponse)
- generate_reformatted_content() - generates template-structured content
- generate_reformat_review_comment() - asks for second approval
- post_comment() and update_issue() in GitHub client

**What needs to be added:**
1. ReviewResponse enum for detecting second approval
2. Review tracking logic in reformat module
3. Logic to detect and process review approval
4. Function to update issue body and remove needs-information label

**Plan:** Extend src/templates/reformat.rs with new types and logic, update src/templates/mod.rs exports.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44c7-eaa1-7da1-905a-e06ee92ac672
author: oompah
created: 2026-05-20T09:46:53Z

Implementation complete:

**ANALYSIS:** The two-step review flow was already partially designed in the codebase - the ApprovalResponse enum and content generation functions existed. The missing piece was the ReviewResponse type and detection logic for the second approval step.

**IMPLEMENTED:**
1. **ReviewResponse enum** - Separates initial reformat approval (Approved/Declined/Ambiguous/NoResponse) from review confirmation (Confirmed/WantsChanges/Rejected/Ambiguous/NoResponse)

2. **ReformatState enum** - Tracks workflow state (None→AwaitingApproval→AwaitingReview→Completed/Declined)

3. **detect_review_response()** - Detects requestor's response to reformatted content with appropriate phrase/token patterns:
   - Explicit confirmations: "looks good", "go ahead", "perfect"
   - Rejections: "no", "leave it as is", "keep original"
   - Change requests: "can you", "but add", "needs to be"
   - Handles qualified approvals: "looks good, but X" → WantsChanges

4. **Helper functions** - should_apply_reformat(), should_keep_original(), wants_reformat_changes(), review_needs_clarification(), looks_like_review_response()

5. **Fixed SYNC issues** - Corrected async/await patterns in GitHub client (removed double  issues)

6. **Tests** - 24 new tests covering all CRIT-5 acceptance criteria

All 133 tests pass, clippy warnings resolved, fmt check passes.

**Branch pushed:** epic-rogers-d7h
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44c8-6d2c-7959-aab4-fdc585c0d9ae
author: oompah
created: 2026-05-20T09:47:26Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 63, Tool calls: 76
- Tokens: 3.5M in / 19.5K out [3.6M total]
- Cost: $0.0000
- Exit: normal, Duration: 16m 4s
- Log: rogers-6tf__20260520T093136Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44c8-7260-7a74-a060-2a958bde3a4f
author: oompah
created: 2026-05-20T09:47:28Z

Agent completed successfully in 964s (3565152 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
