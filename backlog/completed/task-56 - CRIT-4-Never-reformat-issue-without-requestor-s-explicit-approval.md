---
id: TASK-56
title: 'CRIT-4: Never reformat issue without requestor''s explicit approval'
status: Done
assignee: []
created_date: 2026-05-20 05:24
updated_date: 2026-05-20 09:56
labels:
- rodgers:parent=rogers-d7h
- rodgers:type=issue-templates
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-rcj
  state: closed
  parent_id: rogers-d7h
  dependencies: []
  branch_name: rogers-rcj
  target_branch: null
  url: null
  created_at: '2026-05-20T05:24:19Z'
  updated_at: '2026-05-20T09:56:17Z'
  closed_at: '2026-05-20T09:56:07Z'
parent: TASK-6
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/issue-templates-plan.md §Template Conformance / Rodgers Offer to Reformat → Acceptance Criteria CRIT-4

WHAT TO DO
Implement strict enforcement: NEVER reformat issue without requestor's explicit approval.

Create/modify files:
- src/templates/reformat.rs - Reformat logic with approval gate
- src/templates/mod.rs - Enforce approval requirement
- src/github/client.rs - post_comment, update_issue (with approval check)

Enforcement:
- Reformat offer posted (rogers-3wv)
- Wait for requestor affirmative reply
- Only on approval: reformat and update issue
- On decline: accept freeform, proceed with triage
- Key principle: 'Rodgers never reformats without explicit consent'

WHY
Requestor owns their issue. Reformatting without consent violates autonomy. Approval gate ensures requestor agrees to changes.

HOW TO VERIFY
- Unit test: Reformat blocked without approval
- Unit test: Reformat proceeds with approval
- Unit test: Decline → freeform accepted, triage continues
- Unit test: No reformatting on silence (no approval)
- Integration test: Full offer→approval→reformat flow
- Integration test: Offer→decline→freeform flow
- Manual: Test both approval and decline paths

EDGE CASES AND PITFALLS
- Approval detection: 'yes', 'please do', 'go ahead', 'looks good'
- Decline detection: 'no', 'don't', 'leave it', 'no thanks'
- Ambiguous response - ask for clarification
- GitHub API failure on update - retry, don't lose approval
- Requestor edits issue during wait - detect, restart
- Config reformat_consent=true (default, do not change)

PROJECT-SPECIFIC TERMINOLOGY
- 'Explicit approval': Clear affirmative from requestor
- 'Reformat': Rewrite issue body using template structure
- 'Freeform submission': Issue filed without template
- 'reformat_consent': Config (always true) requiring approval
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e4489-d398-7877-9826-743d0f0309ee
author: oompah
created: 2026-05-20T08:39:04Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e448a-c1b9-7189-9a6f-90174235b83b
author: oompah
created: 2026-05-20T08:40:05Z

Focus: Queue Api Oompah Specialist
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e448c-cdb7-7367-9e7c-860d62f43466
author: oompah
created: 2026-05-20T08:42:19Z

Understanding: I need to implement CRIT-4 - strict enforcement that Rodgers never reformats issues without the requestor's explicit approval. This means adding an approval gate that blocks reformatting until the requestor explicitly approves.

Plan:
1. Create  module with GitHub API client (, )
2. Create  with reformat logic + approval detection
3. Include approval detection for phrases like 'yes', 'please do', 'agree' (approval) vs 'no', 'don't' (decline)
4. Update  to export the new reformat module
5. Add comprehensive unit tests for approval/decline detection

Key design: ApprovalResponse enum (Approved, Declined, Ambiguous, NoResponse), approval detection function with keyword matching.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44b9-82dc-733d-bf87-0748d41fed7c
author: oompah
created: 2026-05-20T09:31:09Z

Agent completed successfully in 3126s (8833268 tokens)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44b9-9185-78b1-83cb-cdce10ce1e72
author: oompah
created: 2026-05-20T09:31:13Z

Agent completed without closing this issue (3126s (8833268 tokens)). Escalating from 'default' to 'standard'. Retrying in 10s (1/3).
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44b9-986a-79e8-848c-27b5bffbc841
author: oompah
created: 2026-05-20T09:31:14Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 98, Tool calls: 111
- Tokens: 8.7M in / 88.5K out [8.8M total]
- Cost: $0.0000
- Exit: normal, Duration: 52m 6s
- Log: rogers-rcj__20260520T084019Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44cd-44ee-76e2-80f7-079581d5cb27
author: oompah
created: 2026-05-20T09:52:44Z

Retrying (attempt #8, agent: standard)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44cd-60b9-76bb-bcaa-c10df13dfb51
author: oompah
created: 2026-05-20T09:52:51Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44d0-0167-763d-95f1-ac4777f1a11b
author: oompah
created: 2026-05-20T09:55:43Z

Verification: CRIT-4 implementation is complete. Key components:

1. **src/templates/reformat.rs** - Contains:
   -  enum with Approved/Declined/Ambiguous/NoResponse variants
   -  enum for two-step review workflow
   -  enum tracking workflow state
   -  - keyword matching for approval/decline detection
   -  - for review step confirmation
   - Gate function  - only returns true for explicit  response
   -  - only returns true for explicit  response
   -  - posts the offer per CRIT-3
   -  - posts reformatted version for review (CRIT-5)

2. **All acceptance criteria covered by 89 tests:**
   - test_should_reformat_approved: Reformat blocked without approval
   - test_should_accept_freeform_declined: Decline → freeform accepted, triage continues  
   - test_detect_no_response: No reformatting on silence
   - test_review_workflow_confirm_and_apply: Full offer→approval→reformat flow
   - test_review_workflow_reject_at_review: Offer→reject→keep original flow

3. **Approval detection keywords implemented:**
   - Approval: 'yes', 'please do', 'go ahead', 'looks good', 'sounds good', 'sure', 'i approve', 'i agree'
   - Decline: 'no', 'don't', 'leave it', 'no thanks', 'nah', 'nope', 'rather not'
   - Ambiguous: 'maybe', 'perhaps', 'unsure', 'what do you think', 'should i'

All 133 tests pass. Code builds, clippy clean, fmt clean.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44d0-7222-7751-a97e-c65640ec1240
author: oompah
created: 2026-05-20T09:56:12Z

Run #9 [attempt=9, profile=standard, role=standard -> InferenceAPI/nvidia/minimaxai/minimax-m2.7]
- Turns: 15, Tool calls: 22
- Tokens: 418.6K in / 3.0K out [421.6K total]
- Cost: $0.0000
- Exit: normal, Duration: 3m 31s
- Log: rogers-rcj__20260520T095254Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e44d0-7839-7601-b96a-d7edea8ea988
author: oompah
created: 2026-05-20T09:56:13Z

Agent completed successfully in 211s (421586 tokens)
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
