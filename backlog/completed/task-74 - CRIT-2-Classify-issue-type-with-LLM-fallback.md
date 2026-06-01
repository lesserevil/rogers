---
id: TASK-74
title: 'CRIT-2: Classify issue type with LLM fallback'
status: Done
assignee: []
created_date: 2026-05-20 05:27
updated_date: 2026-05-21 12:07
labels:
- rodgers:parent=rogers-jh3
- rodgers:type=triage-workflow
- tasks-migrated
dependencies: []
priority: high
ordinal: 1000
type: task
tasks:
  id: rogers-2ey
  state: closed
  parent_id: rogers-jh3
  dependencies: []
  branch_name: rogers-2ey
  target_branch: null
  url: null
  created_at: '2026-05-20T05:27:28Z'
  updated_at: '2026-05-21T12:07:45Z'
  closed_at: '2026-05-21T12:07:31Z'
parent: TASK-9
---
## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan: plans/triage-workflow-plan.md §Top-Level Classification

WHAT TO DO
Implement issue classification logic that determines if an issue is a bug, feature, question, docs, chore, or unknown.

Create/modify files:
- src/triage/classifier.rs - Classification logic with label heuristics and LLM fallback
- src/triage/mod.rs - Integrate classifier into triage loop
- src/llm/prompts.rs - LLM prompt for classification

Classification priority:
1. Label heuristics first: existing labels (bug, enhancement, question, documentation)
2. LLM classification on title+body for unlabeled issues
3. Default to 'question' if LLM cannot determine with confidence

WHY
Accurate classification routes issues to the correct workflow (bug/feature → feature-bug-plan, question → question-routing-plan, docs → issue-templates-plan). Misclassified issues waste human review time and delay responses.

HOW TO VERIFY
- Unit test: Issues with 'bug' label classified as bug
- Unit test: Issues with 'enhancement' label classified as feature  
- Unit test: Issues with 'question' label classified as question
- Unit test: Issues with 'documentation' label classified as docs
- Unit test: Unlabeled issues classified via LLM with structured output
- Unit test: LLM low confidence defaults to question
- Integration test: Full triage classifies sample issues correctly

EDGE CASES AND PITFALLS
- Respect existing human-applied labels from prior triage runs (do not override)
- LLM response must be validated with Structured Output Validator before acting
- Bot-authored issues get bot_labels applied and are skipped
- Mixed labels (bug + feature) - decide precedence
- LLM hallucination - validate output schema strictly

PROJECT-SPECIFIC TERMINOLOGY
- 'Label heuristics': Fast path classification using existing GitHub labels
- 'LLM fallback': Structured LLM call when heuristics don't apply
- 'rodgers:triaged': Label marking issue as processed by triage
<!-- SECTION:DESCRIPTION:END -->

## Comments
<!-- COMMENTS:BEGIN -->
<!-- COMMENT:BEGIN -->
index: 019e4935-6e64-7ac0-81c8-0925675444ea
author: oompah
created: 2026-05-21T06:24:59Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4935-850b-7843-91f9-5174ad7e0ac4
author: oompah
created: 2026-05-21T06:25:05Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4935-b947-7a96-ba8b-75b9c2e5f749
author: oompah
created: 2026-05-21T06:25:18Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/qwen/qwen3.6-35b-a3b]
- Turns: 2, Tool calls: 3
- Tokens: 22.1K in / 305 out [22.4K total]
- Cost: $0.0000
- Exit: ask_question, Duration: 20s
- Log: rogers-2ey__20260521T062509Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4935-c21c-736d-a838-787b23f41363
author: oompah
created: 2026-05-21T06:25:20Z

🤚 **Question from agent:**

The task says 'Do NOT run backlog edit' but I see in the instructions section that backlog commands should be run via run_command. Can I use backlog commands via run_command like `backlog show rogers-2ey` and `backlog comments add ...`?
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4a53-b930-73fb-a59c-89957c236763
author: user
created: 2026-05-21T11:37:41Z

you can use backlog via run_command.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4a65-fd59-7d02-9263-24f8b1e9c664
author: oompah
created: 2026-05-21T11:57:39Z

Agent dispatched (profile: default)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4a66-05aa-7ec4-94ca-5a16d710ccc0
author: oompah
created: 2026-05-21T11:57:41Z

Focus: Feature Developer
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4a66-acde-7783-8e37-e3781efff128
author: oompah
created: 2026-05-21T11:58:23Z

## Understanding

**Issue:** CRIT-2: Classify issue type with LLM fallback

**My interpretation:**
The triage loop currently only processes issues already labeled  or . It skips unlabeled issues with . This task requires adding classification logic that:

1. **Label heuristics (fast path):** Check for existing GitHub labels —  → bug,  → feature,  → question,  → docs
2. **LLM fallback:** For unlabeled issues (no bug/feature/question/documentation/enhancement labels), send title+body to LLM to classify as bug/feature/question/docs/chore/unknown
3. **Default:** If LLM confidence is low, default to 
4. **Respects existing human labels:** Don't override labels already applied by humans
5. **Bot issues:** Get  applied and skip triage
6. **Mixed labels:** Need precedence rules (bug + enhancement → bug)

**Files to create/modify:**
-  — Classification logic with heuristics + LLM fallback
-  — Integrate classifier module into triage loop  
-  — Add LLM prompt for classification

**Key insight:** The current  in  returns  for unlabeled issues. The classifier will be called before this check to assign the appropriate label, then  continues as normal.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4a66-d2e4-71a9-8781-d0113a8be7f2
author: oompah
created: 2026-05-21T11:58:33Z

## Discovery

Key findings from codebase exploration:

1. **Current triage flow:**  in  first checks for  label (skip if present), then checks for  or  labels. If neither is present, it returns  — **this is where classification needs to happen**.

2. **Label heuristics already partially exist:**  defines canonical labels including , ,  but not  or . These need to be added for the heuristic path.

3. **LLM prompts exist in :** Field extraction, completeness checking, breakdown prompts. I need to add a classification-specific prompt here.

4. **Pattern to follow:** The completeness check pattern in  is a good model — deterministic checks (regex/section matching) with LLM fallback for ambiguous cases.

5. **Classification enum needed:** A  enum (Bug, Feature, Question, Docs, Chore, Unknown) with  method.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4a6d-0514-737a-b779-39d6c1d3f198
author: oompah
created: 2026-05-21T12:05:19Z

## Verification

All tests pass (391/391) and clippy is clean:
-  ✓
- 
running 393 tests
test tasks::client::tests::test_task_status_deferred_as_str ... ok
test tasks::client::tests::test_task_type_as_str ... ok
test tasks::client::tests::test_task_type_defaults_to_feature ... ok
test tasks::client::tests::test_task_type_from_github_labels_bug ... ok
test tasks::client::tests::test_task_type_from_github_labels_feature ... ok
test tasks::client::tests::test_build_breakdown_comment_single_epic ... ok
test tasks::client::tests::test_build_breakdown_comment_epic_scale ... ok
test tasks::client::tests::test_build_child_request ... ok
test tasks::client::tests::test_build_epic_request ... ok
test tasks::client::tests::test_build_epic_request_epic_type_for_large_work ... ok
test tasks::client::tests::test_child_task_spec_serialization ... ok
test tasks::client::tests::test_client_default ... ok
test tasks::client::tests::test_epic_task_result_serialization ... ok
test tasks::client::tests::test_epic_scale_result_serialization ... ok
test tasks::client::tests::test_file_task_request_serialization ... ok
test feature_bug::breakdown::tests::test_all_acceptance_criteria_tracks_sources ... ok
test feature_bug::breakdown::tests::test_build_epic_description_enriched_full ... ok
test feature_bug::breakdown::tests::test_bug_type_sets_task_type ... ok
test feature_bug::breakdown::tests::test_closely_related_areas_allows_api_db ... ok
test feature_bug::breakdown::tests::test_child_tasks_status_is_deferred ... ok
test feature_bug::breakdown::tests::test_count_codebase_areas_auth_api_ui_database ... ok
test feature_bug::breakdown::tests::test_count_codebase_areas ... ok
test feature_bug::breakdown::tests::test_count_implementation_steps ... ok
test feature_bug::breakdown::tests::test_detect_child_task_areas_auth ... ok
test feature_bug::breakdown::tests::test_detect_child_task_areas_empty ... ok
test feature_bug::breakdown::tests::test_detect_child_task_areas_multiple ... ok
test feature_bug::breakdown::tests::test_detect_sequential_work ... ok
test feature_bug::breakdown::tests::test_epic_description_acceptance_criteria_from_comments ... ok
test feature_bug::breakdown::tests::test_epic_description_has_github_issue_number ... ok
test feature_bug::breakdown::tests::test_epic_description_has_all_acceptance_criteria ... ok
test feature_bug::breakdown::tests::test_epic_description_has_plan_reference ... ok
test feature_bug::breakdown::tests::test_epic_description_no_criteria_pending_review ... ok
test feature_bug::breakdown::tests::test_epic_scale_result_includes_reasons ... ok
test feature_bug::breakdown::tests::test_epic_description_has_what_why_summary ... ok
test feature_bug::breakdown::tests::test_epic_scale_sequential_pattern ... ok
test feature_bug::breakdown::tests::test_epic_scale_single_area_detected ... ok
test feature_bug::breakdown::tests::test_epic_scale_with_four_distinct_areas ... ok
test feature_bug::breakdown::tests::test_epic_status_is_deferred ... ok
test feature_bug::breakdown::tests::test_epic_scale_complex_multi_area ... ok
test feature_bug::breakdown::tests::test_extract_ac_logical_units ... ok
test feature_bug::breakdown::tests::test_execute_breakdown_epic_scale ... ok
test feature_bug::breakdown::tests::test_execute_breakdown_single_epic ... ok
test feature_bug::breakdown::tests::test_extract_acceptance_criteria ... ok
test feature_bug::breakdown::tests::test_feature_type_sets_task_type ... ok
test feature_bug::breakdown::tests::test_generate_child_tasks_falls_back_to_ac_units ... ok
test feature_bug::breakdown::tests::test_generate_standalone_task_cli ... ok
test feature_bug::breakdown::tests::test_generate_standalone_task_database ... ok
test feature_bug::breakdown::tests::test_generate_standalone_task_ui ... ok
test feature_bug::breakdown::tests::test_standalone_task_has_all_sections ... ok
test feature_bug::breakdown::tests::test_generate_standalone_task_api ... ok
test feature_bug::breakdown::tests::test_standalone_task_compound_pattern_detection ... ok
test feature_bug::breakdown::tests::test_standalone_task_missing_sections ... ok
test feature_bug::breakdown::tests::test_standalone_task_is_standalone_ready_full ... ok
test feature_bug::breakdown::tests::test_standalone_task_single_codebase_part ... ok
test feature_bug::breakdown::tests::test_standalone_task_to_markdown ... ok
test feature_bug::breakdown::tests::test_standalone_task_validation_multiple_issues ... ok
test feature_bug::breakdown::tests::test_standalone_issue_description ... ok
test feature_bug::breakdown::tests::test_standalone_validation_descriptions ... ok
test feature_bug::completeness::tests::test_bug_completeness_result_usable_for_transition ... ok
test feature_bug::completeness::tests::test_bug_missing_environment_only_requests_environment ... ok
test feature_bug::completeness::tests::test_bug_missing_steps_and_expected_requests_both ... ok
test feature_bug::completeness::tests::test_complete_bug ... ok
test feature_bug::breakdown::tests::test_validate_tasks_standalone_one_fails ... ok
test feature_bug::completeness::tests::test_complete_bug_with_alternate_headers ... ok
test feature_bug::completeness::tests::test_complete_bug_with_bullet_reproduction ... ok
test feature_bug::completeness::tests::test_complete_bug_with_na_reproduction ... ok
test feature_bug::completeness::tests::test_complete_feature ... ok
test feature_bug::completeness::tests::test_complete_feature_with_checkboxes ... ok
test feature_bug::completeness::tests::test_complete_feature_with_user_story ... ok
test feature_bug::completeness::tests::test_feature_completeness_result_usable_for_transition ... ok
test feature_bug::completeness::tests::test_feature_missing_acceptance_criteria_only_requests_that ... ok
test feature_bug::breakdown::tests::test_validate_tasks_standalone_all_pass ... ok
test feature_bug::completeness::tests::test_incomplete_bug ... ok
test feature_bug::completeness::tests::test_incomplete_feature ... ok
test feature_bug::completeness::tests::test_needs_information_label_would_be_applied ... ok
test feature_bug::completeness::tests::test_no_generic_please_provide_more_details ... ok
test feature_bug::completeness::tests::test_request_message_includes_all_missing_fields ... ok
test feature_bug::tests::test_acceptance_criteria_no_criteria_yet ... ok
test feature_bug::tests::test_all_acceptance_criteria_checked_vs_unchecked ... ok
test feature_bug::tests::test_all_acceptance_criteria_deduplicates ... ok
test feature_bug::tests::test_all_acceptance_criteria_format_for_epic_empty ... ok
test feature_bug::tests::test_all_acceptance_criteria_sources ... ok
test feature_bug::tests::test_bug_needs_information_transition ... ok
test feature_bug::tests::test_bug_ready_for_review_transition ... ok
test feature_bug::tests::test_complete_bug_workflow ... ok
test feature_bug::tests::test_complete_feature_workflow ... ok
test feature_bug::tests::test_extract_acceptance_criteria_from_body_ac_n_pattern ... ok
test feature_bug::tests::test_extract_acceptance_criteria_from_body_no_ac_section ... ok
test feature_bug::tests::test_extract_acceptance_criteria_from_body_section ... ok
test feature_bug::tests::test_extract_all_acceptance_criteria_combines_body_and_comments ... ok
test feature_bug::tests::test_extract_all_acceptance_criteria_no_criteria_pending ... ok
test feature_bug::tests::test_extract_checkbox_item_ac_pattern ... ok
test feature_bug::tests::test_extract_checkbox_item_basic ... ok
test feature_bug::tests::test_extract_checkbox_item_empty_or_short ... ok
test feature_bug::tests::test_extract_issue_sections_basic ... ok
test feature_bug::tests::test_extract_issue_sections_with_content ... ok
test feature_bug::tests::test_feature_needs_information_transition ... ok
test feature_bug::tests::test_feature_ready_for_review_transition ... ok
test feature_bug::tests::test_full_validation_issue_summary ... ok
test feature_bug::tests::test_generate_what_why_summary_bug ... ok
test feature_bug::tests::test_generate_what_why_summary_feature ... ok
test feature_bug::tests::test_generated_acceptance_criteria_includes_bug_criteria ... ok
test feature_bug::tests::test_generated_acceptance_criteria_includes_feature_criteria ... ok
test feature_bug::tests::test_incomplete_bug_workflow_in_one_run ... ok
test feature_bug::tests::test_no_delay_in_transition ... ok
test feature_bug::tests::test_validate_task_standalone_compound_pattern ... ok
test feature_bug::tests::test_validate_task_standalone_missing_sections ... ok
test feature_bug::tests::test_validate_task_standalone_full_pass ... ok
test feature_bug::tests::test_validate_task_standalone_multiple_areas ... ok
test feature_bug::tests::test_validate_no_compound_pattern_and_then ... ok
test feature_bug::tests::test_validate_no_compound_pattern_clean ... ok
test feature_bug::tests::test_validate_no_compound_pattern_first_second ... ok
test feature_bug::tests::test_validate_no_compound_pattern_numbered_list_ok ... ok
test feature_bug::tests::test_validate_no_compound_pattern_step_numbers ... ok
test feature_bug::tests::test_validate_single_codebase_part_api_and_db_allowed ... ok
test feature_bug::tests::test_validate_single_codebase_part_api_only ... ok
test feature_bug::tests::test_validate_single_codebase_part_multiple_areas ... ok
test feature_bug::tests::test_validate_standalone_sections_all_present ... ok
test feature_bug::tests::test_validate_standalone_sections_case_insensitive ... ok
test feature_bug::tests::test_validate_standalone_sections_missing_one ... ok
test feature_bug::tests::test_what_why_summary_fallback_when_no_sections ... ok
test feature_bug::tests::test_what_why_summary_format_for_epic ... ok
test feature_bug::will_not_do::tests::test_has_will_not_do_label_empty ... ok
test feature_bug::will_not_do::tests::test_has_will_not_do_label_false ... ok
test feature_bug::will_not_do::tests::test_has_will_not_do_label_true ... ok
test feature_bug::will_not_do::tests::test_processing_within_one_triage_run ... ok
test feature_bug::will_not_do::tests::test_resolve_issue_type_bug ... ok
test feature_bug::will_not_do::tests::test_resolve_issue_type_fallback ... ok
test feature_bug::will_not_do::tests::test_resolve_issue_type_feature ... ok
test feature_bug::will_not_do::tests::test_resolve_issue_type_with_multiple ... ok
test feature_bug::will_not_do::tests::test_warm_closure_comment_includes_author ... ok
test feature_bug::will_not_do::tests::test_warm_closure_comment_mentions_future ... ok
test feature_bug::will_not_do::tests::test_warm_closure_comment_no_curt_phrases ... ok
test feature_bug::will_not_do::tests::test_warm_closure_comment_tone ... ok
test feature_bug::will_not_do::tests::test_will_not_do_priority_over_ready_for_work ... ok
test feature_bug::will_not_do::tests::test_will_not_do_result_detected ... ok
test feature_bug::will_not_do::tests::test_will_not_do_result_not_detected ... ok
test github::client::tests::test_comments_url_format ... ok
test github::client::tests::test_github_client_new ... ok
test github::client::tests::test_github_client_with_api_base ... ok
test github::client::tests::test_github_client_with_token ... ok
test github::client::tests::test_github_comment_deserialization ... ok
test github::client::tests::test_github_issue_deserialization ... ok
test github::client::tests::test_github_issue_with_no_body ... ok
test github::client::tests::test_github_label_deserialization ... ok
test github::client::tests::test_github_user_deserialization ... ok
test github::client::tests::test_issue_url_format ... ok
test llm::prompts::tests::test_task_split_prompt_format ... ok
test llm::prompts::tests::test_task_split_prompt_includes_original_task_placeholder ... ok
test llm::prompts::tests::test_task_split_prompt_includes_rules ... ok
test llm::prompts::tests::test_bug_field_extraction_is_complete ... ok
test llm::prompts::tests::test_bug_field_extraction_missing_fields ... ok
test llm::prompts::tests::test_bug_missing_prompt_includes_missing_fields_placeholder ... ok
test llm::prompts::tests::test_bug_prompt_includes_issue_content_placeholder ... ok
test llm::prompts::tests::test_classification_prompt_classifies_bugs_as_broken ... ok
test llm::prompts::tests::test_classification_prompt_classifies_docs_as_missing_docs ... ok
test llm::prompts::tests::test_classification_prompt_classifies_features_as_new_functionality ... ok
test llm::prompts::tests::test_classification_prompt_confidence_levels ... ok
test llm::prompts::tests::test_classification_prompt_defaults_to_question ... ok
test llm::prompts::tests::test_classification_prompt_includes_all_categories ... ok
test llm::prompts::tests::test_classification_prompt_includes_placeholders ... ok
test llm::prompts::tests::test_classification_prompt_requires_json_output ... ok
test llm::prompts::tests::test_epic_breakdown_prompt_for_standalone_tasks ... ok
test llm::prompts::tests::test_feature_field_extraction_is_complete ... ok
test llm::prompts::tests::test_feature_field_extraction_missing_fields ... ok
test llm::prompts::tests::test_feature_missing_prompt_includes_missing_fields_placeholder ... ok
test llm::prompts::tests::test_feature_prompt_includes_issue_content_placeholder ... ok
test llm::prompts::tests::test_format_bug_field_request_environment_only ... ok
test llm::prompts::tests::test_format_bug_field_request_steps_and_expected ... ok
test llm::prompts::tests::test_format_bug_request_empty_missing ... ok
test llm::prompts::tests::test_format_feature_field_request_acceptance_only ... ok
test llm::prompts::tests::test_no_generic_phrases ... ok
test llm::prompts::tests::test_standalone_task_prompt_checks_compound_patterns ... ok
test llm::prompts::tests::test_standalone_task_prompt_has_required_sections ... ok
test llm::prompts::tests::test_standalone_task_prompt_includes_format ... ok
test llm::prompts::tests::test_standalone_task_prompt_includes_placeholders ... ok
test llm::prompts::tests::test_standalone_task_prompt_includes_rules ... ok
test llm::prompts::tests::test_standalone_validation_prompt_checks_compound_patterns ... ok
test llm::prompts::tests::test_standalone_validation_prompt_checks_multiple_areas ... ok
test llm::prompts::tests::test_standalone_validation_prompt_checks_sections ... ok
test llm::prompts::tests::test_standalone_validation_prompt_includes_task_desc_placeholder ... ok
test llm::prompts::tests::test_standalone_validation_prompt_returns_json ... ok
test llm::prompts::tests::test_warm_closure_prompt_includes_example ... ok
test llm::prompts::tests::test_warm_closure_prompt_includes_issue_details ... ok
test llm::prompts::tests::test_warm_closure_prompt_tone_guidance ... ok
test triage::classifier::tests::test_bot_author_detection_app_suffix ... ok
test triage::classifier::tests::test_bot_author_detection_bot_username ... ok
test triage::classifier::tests::test_bot_author_detection_known_bots ... ok
test triage::classifier::tests::test_classify_already_triaged_respects_existing_label ... ok
test triage::classifier::tests::test_classify_bot_issue_skipped ... ok
test triage::classifier::tests::test_classify_bug_via_label_heuristic ... ok
test triage::classifier::tests::test_classify_chore_via_label_heuristic ... ok
test triage::classifier::tests::test_classify_documentation_via_docs_shortcut_label ... ok
test triage::classifier::tests::test_classify_documentation_via_label_heuristic ... ok
test triage::classifier::tests::test_classify_enhancement_via_label_heuristic ... ok
test triage::classifier::tests::test_classify_mixed_labels_bug_and_enhancement ... ok
test triage::classifier::tests::test_classify_question_via_label_heuristic ... ok
test triage::classifier::tests::test_classify_unlabeled_llm_failure_defaults_to_question ... ok
test triage::classifier::tests::test_classify_unlabeled_low_confidence_defaults_to_question ... ok
test triage::classifier::tests::test_classify_unlabeled_via_llm ... ok
test triage::classifier::tests::test_classify_with_full_content ... ok
test triage::classifier::tests::test_confidence_is_acceptable ... ok
test triage::classifier::tests::test_full_triage_classification_samples ... ok
test triage::classifier::tests::test_issue_type_display ... ok
test triage::classifier::tests::test_issue_type_is_triage_worthy ... ok
test triage::classifier::tests::test_issue_type_label_names ... ok
test triage::classifier::tests::test_label_heuristic_bug_label ... ok
test triage::classifier::tests::test_label_heuristic_bug_report_label ... ok
test triage::classifier::tests::test_label_heuristic_chore_label ... ok
test triage::classifier::tests::test_label_heuristic_ci_cd_label ... ok
test triage::classifier::tests::test_label_heuristic_defect_label ... ok
test triage::classifier::tests::test_label_heuristic_docs_label ... ok
test triage::classifier::tests::test_label_heuristic_documentation_label ... ok
test triage::classifier::tests::test_label_heuristic_enhancement_label ... ok
test triage::classifier::tests::test_label_heuristic_feature_label ... ok
test triage::classifier::tests::test_label_heuristic_feature_request_label ... ok
test triage::classifier::tests::test_label_heuristic_good_first_issue_label ... ok
test triage::classifier::tests::test_label_heuristic_help_wanted_label ... ok
test triage::classifier::tests::test_label_heuristic_maintenance_label ... ok
test triage::classifier::tests::test_label_heuristic_no_matching_label ... ok
test triage::classifier::tests::test_label_heuristic_priority_bug_over_feature ... ok
test triage::classifier::tests::test_label_heuristic_priority_docs_over_chore ... ok
test triage::classifier::tests::test_label_heuristic_priority_feature_over_question ... ok
test triage::classifier::tests::test_label_heuristic_priority_question_over_docs ... ok
test triage::classifier::tests::test_label_heuristic_question_label ... ok
test triage::classifier::tests::test_label_heuristic_support_label ... ok
test triage::classifier::tests::test_non_bot_authors ... ok
test triage::classifier::tests::test_pre_check_already_classified ... ok
test triage::classifier::tests::test_pre_check_bot_issue ... ok
test triage::classifier::tests::test_pre_check_needs_classification ... ok
test triage::classifier::tests::test_pre_check_needs_classification_with_unrelated_labels ... ok
test triage::classifier::tests::test_pre_check_plain_bug_label_not_already_classified ... ok
test triage::classifier::tests::test_resolve_conflicting_labels_bug_over_feature ... ok
test triage::classifier::tests::test_resolve_conflicting_labels_no_match ... ok
test triage::classifier::tests::test_rodgers_feature_label_maps_to_feature ... ok
test triage::classifier::tests::test_rodgers_triaged_label_skipped ... ok
test triage::classifier::tests::test_rodgers_triaged_with_type_label_respects_type ... ok
test triage::classifier::tests::test_triage_classification_bug ... ok
test triage::classifier::tests::test_triage_classification_chore ... ok
test triage::classifier::tests::test_validate_classification_empty_rationale_fails ... ok
test triage::classifier::tests::test_validate_classification_valid ... ok
test triage::classifier::tests::test_workflow_mapping ... ok
test triage::priority::tests::test_blocker_maps_to_p1 ... ok
test triage::priority::tests::test_backlog_maps_to_p4 ... ok
test triage::priority::tests::test_critical_maps_to_p1 ... ok
test triage::priority::tests::test_full_priority_flow_feature_issue ... ok
test triage::priority::tests::test_full_priority_flow_no_matches ... ok
test triage::priority::tests::test_full_priority_flow_with_multiple_priority_keywords ... ok
test triage::priority::tests::test_high_value_maps_to_p2 ... ok
test triage::priority::tests::test_human_p1_label_preserved_over_keywords ... ok
test triage::priority::tests::test_human_p4_label_preserved_over_keywords ... ok
test triage::priority::tests::test_human_priority_case_insensitive ... ok
test triage::priority::tests::test_human_priority_detection ... ok
test triage::priority::tests::test_important_maps_to_p2 ... ok
test triage::priority::tests::test_keyword_boundary_in_sentence ... ok
test triage::priority::tests::test_keyword_boundary_single_word ... ok
test triage::priority::tests::test_llm_assess_priority_returns_p3_placeholder ... ok
test triage::priority::tests::test_llm_preserves_matched_keywords ... ok
test triage::priority::tests::test_low_priority_maps_to_p4 ... ok
test triage::priority::tests::test_multilword_keyword ... ok
test triage::priority::tests::test_multiple_p1_keywords_all_recorded ... ok
test triage::priority::tests::test_nice_to_have_maps_to_p3 ... ok
test triage::priority::tests::test_no_keywords_defaults_to_p3 ... ok
test triage::priority::tests::test_normal_maps_to_p3 ... ok
test triage::priority::tests::test_p1_takes_precedence_over_p2 ... ok
test triage::priority::tests::test_p2_takes_precedence_over_p3 ... ok
test triage::priority::tests::test_priority_assessment_human_method ... ok
test triage::priority::tests::test_priority_assessment_keyword_method ... ok
test triage::priority::tests::test_priority_display ... ok
test triage::priority::tests::test_priority_label ... ok
test triage::priority::tests::test_urgent_maps_to_p1 ... ok
test triage::router::tests::test_already_routed_issue_skipped ... ok
test triage::router::tests::test_backlog_maps_to_p4_in_route ... ok
test triage::router::tests::test_batch_routing_mixed_issues ... ok
test triage::router::tests::test_blocker_maps_to_p1_in_route ... ok
test triage::router::tests::test_batch_routing_with_priorities ... ok
test triage::router::tests::test_does_not_override_human_priority_p1 ... ok
test triage::router::tests::test_does_not_override_human_priority_p4 ... ok
test triage::router::tests::test_feature_issue_gets_rodgers_feature_label ... ok
test triage::router::tests::test_feature_routed_to_feature_bug_workflow ... ok
test triage::router::tests::test_full_routing_with_priority_metadata ... ok
test triage::router::tests::test_high_value_maps_to_p2_in_route ... ok
test triage::router::tests::test_human_priority_preserved_in_routing ... ok
test triage::router::tests::test_important_maps_to_p2_in_route ... ok
test triage::router::tests::test_keyword_assessment_when_llm_returns_default ... ok
test triage::router::tests::test_llm_priority_assessment_returns_p3_default ... ok
test triage::router::tests::test_non_feature_issue_skipped ... ok
test triage::router::tests::test_low_priority_backlog_keyword_maps_to_p4 ... ok
test triage::router::tests::test_normal_defaults_to_p3_in_route ... ok
test triage::router::tests::test_ready_for_work_deferred ... ok
test triage::router::tests::test_rodgers_feature_label_in_result_labels ... ok
test triage::router::tests::test_routing_adds_both_feature_and_priority_labels ... ok
test triage::router::tests::test_routing_adds_priority_p1_label ... ok
test triage::router::tests::test_routing_adds_priority_p2_label ... ok
test triage::router::tests::test_routing_adds_priority_p3_label ... ok
test triage::router::tests::test_routing_adds_priority_p4_label ... ok
test triage::router::tests::test_urgent_maps_to_p1_in_route ... ok
test triage::scheduler::tests::test_enqueue_webhook_event_labeled ... ok
test triage::scheduler::tests::test_enqueue_webhook_event_edited ... ok
test triage::scheduler::tests::test_enqueue_webhook_event_all_types ... ok
test triage::scheduler::tests::test_enqueue_webhook_event_opened ... ok
test triage::scheduler::tests::test_enqueue_webhook_event_unlabeled ... ok
test triage::scheduler::tests::test_lock_allows_after_release ... ok
test triage::scheduler::tests::test_retry_policy_capped_at_60s ... ok
test triage::scheduler::tests::test_retry_policy_delays ... ok
test triage::scheduler::tests::test_lock_prevents_concurrent_runs ... ok
test triage::scheduler::tests::test_run_lock_release_then_acquire ... ok
test triage::scheduler::tests::test_run_trigger_cron ... ok
test triage::scheduler::tests::test_run_metadata_serialization ... ok
test triage::scheduler::tests::test_run_trigger_event ... ok
test triage::scheduler::tests::test_lock_shared_across_clones ... ok
test triage::scheduler::tests::test_scheduler_batch_processes_multiple_issues ... ok
test triage::scheduler::tests::test_scheduler_custom_interval ... ok
test triage::scheduler::tests::test_scheduler_creates_with_defaults ... ok
test triage::scheduler::tests::test_scheduler_enabled ... ok
test triage::scheduler::tests::test_run_lock_single_acquire ... ok
test triage::scheduler::tests::test_scheduler_disabled ... ok
test triage::scheduler::tests::test_scheduler_interval_duration ... ok
test triage::scheduler::tests::test_scheduler_filters_closed_issues ... ok
test triage::scheduler::tests::test_scheduler_interval_one_hour ... ok
test triage::scheduler::tests::test_scheduler_minimum_interval ... ok
test triage::scheduler::tests::test_scheduler_process_issue_filters_triaged ... ok
test triage::scheduler::tests::test_scheduler_with_custom_retry_policy ... ok
test triage::scheduler::tests::test_webhook_event_already_triaged_issue_skipped ... ok
test triage::scheduler::tests::test_triaged_states_set_tracks_issues ... ok
test triage::scheduler::tests::test_triaged_states_set_is_empty_initially ... ok
test triage::scheduler::tests::test_webhook_event_descriptions ... ok
test triage::scheduler::tests::test_webhook_event_issue_number ... ok
test triage::triage_loop::tests::test_already_ready_for_review_skipped ... ok
test triage::triage_loop::tests::test_batch_with_will_not_do ... ok
test triage::triage_loop::tests::test_batch_skips_already_triaged_issues ... ok
test triage::triage_loop::tests::test_bug_all_minimum_present_allows_ready_for_review ... ok
test triage::triage_loop::tests::test_batch_processing ... ok
test triage::triage_loop::tests::test_bug_missing_behavior_observed_blocks_ready_for_review ... ok
test triage::triage_loop::tests::test_bug_missing_behavior_expected_blocks_ready_for_review ... ok
test triage::triage_loop::tests::test_bug_missing_environment_blocks_ready_for_review ... ok
test triage::triage_loop::tests::test_bug_with_na_reproduction_is_complete ... ok
test triage::triage_loop::tests::test_batch_with_ready_for_work ... ok
test triage::triage_loop::tests::test_closed_issues_skipped ... ok
test triage::triage_loop::tests::test_bug_missing_reproduction_steps_blocks_ready_for_review ... ok
test triage::triage_loop::tests::test_complete_bug_transitions_in_one_run ... ok
test triage::triage_loop::tests::test_complete_bug_all_fields ... ok
test triage::triage_loop::tests::test_complete_feature_transitions_in_one_run ... ok
test triage::triage_loop::tests::test_complete_issue_transitions_immediately_to_ready_for_review ... ok
test triage::triage_loop::tests::test_completeness_check_is_hard_block ... ok
test triage::triage_loop::tests::test_empty_template_fields_treated_as_missing ... ok
test triage::triage_loop::tests::test_feature_missing_acceptance_criteria_blocks_ready_for_review ... ok
test triage::triage_loop::tests::test_feature_all_minimum_present_allows_ready_for_review ... ok
test triage::triage_loop::tests::test_feature_missing_proposed_behavior_blocks_ready_for_review ... ok
test triage::triage_loop::tests::test_complete_feature_all_fields ... ok
test triage::triage_loop::tests::test_feature_missing_use_case_blocks_ready_for_review ... ok
test triage::triage_loop::tests::test_freeform_bug_complete_allows_ready_for_review ... ok
test triage::triage_loop::tests::test_hard_block_label_application_sequence ... ok
test triage::triage_loop::tests::test_freeform_feature_complete_allows_ready_for_review ... ok
test triage::triage_loop::tests::test_has_triaged_label_empty ... ok
test triage::triage_loop::tests::test_has_triaged_label_false ... ok
test triage::triage_loop::tests::test_has_triaged_label_true ... ok
test triage::triage_loop::tests::test_incomplete_bug_applies_needs_information_in_one_run ... ok
test triage::triage_loop::tests::test_incomplete_feature_requests_specific_fields ... ok
test triage::triage_loop::tests::test_incomplete_issue_blocks_human_attempt_to_skip ... ok
test triage::triage_loop::tests::test_incomplete_issue_requests_only_missing_specific_fields ... ok
test triage::triage_loop::tests::test_issue_with_triaged_true_not_reprocessed ... ok
test triage::triage_loop::tests::test_non_triaged_issues_skipped ... ok
test triage::triage_loop::tests::test_incomplete_issues_never_reach_ready_for_review_in_batch ... ok
test triage::triage_loop::tests::test_no_delay_same_run ... ok
test triage::triage_loop::tests::test_processed_issue_gets_triaged_label ... ok
test triage::triage_loop::tests::test_ready_for_work_comment_posted ... ok
test triage::triage_loop::tests::test_ready_for_work_detect_and_trigger_in_one_run ... ok
test triage::triage_loop::tests::test_ready_for_work_triggers_breakdown ... ok
test triage::triage_loop::tests::test_ready_for_work_feature_type ... ok
test triage::triage_loop::tests::test_removes_needs_information_on_complete ... ok
test triage::triage_loop::tests::test_second_triage_run_skips_already_triaged_issues ... ok
test triage::triage_loop::tests::test_skipped_paths_dont_get_triaged_label ... ok
test triage::triage_loop::tests::test_ready_for_work_with_epic_scale_issue ... ok
test triage::triage_loop::tests::test_template_filed_complete_allows_ready_for_review ... ok
test triage::triage_loop::tests::test_summary_comment_posted ... ok
test triage::triage_loop::tests::test_triaged_issue_has_triaged_label_constant ... ok
test triage::triage_loop::tests::test_template_filed_feature_complete_allows_ready_for_review ... ok
test triage::triage_loop::tests::test_triaged_label_applied_even_with_no_other_changes ... ok
test triage::triage_loop::tests::test_triaged_label_always_applied_when_processed ... ok
test triage::triage_loop::tests::test_triaged_label_applied_with_needs_information ... ok
test triage::triage_loop::tests::test_triaged_label_applied_with_will_not_do ... ok
test triage::triage_loop::tests::test_triaged_label_applied_with_ready_for_work ... ok
test triage::triage_loop::tests::test_will_not_do_bug_type ... ok
test triage::triage_loop::tests::test_will_not_do_closed_issue_skipped ... ok
test triage::triage_loop::tests::test_will_not_do_closure_comment_is_warm ... ok
test triage::triage_loop::tests::test_will_not_do_detected_generates_closure_comment ... ok
test triage::triage_loop::tests::test_will_not_do_feature_type ... ok
test triage::triage_loop::tests::test_will_not_do_in_one_triage_run ... ok
test triage::triage_loop::tests::test_will_not_do_includes_author ... ok
test triage::triage_loop::tests::test_will_not_do_no_labels_to_add ... ok
test triage::triage_loop::tests::test_will_not_do_priority_over_ready_for_work ... ok
test triage::triage_loop::tests::test_will_not_do_removes_ready_for_review ... ok

test result: ok. 393 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s


running 193 tests
test tasks::client::tests::test_task_type_as_str ... ok
test tasks::client::tests::test_task_status_deferred_as_str ... ok
test tasks::client::tests::test_task_type_from_github_labels_bug ... ok
test tasks::client::tests::test_task_type_defaults_to_feature ... ok
test tasks::client::tests::test_task_type_from_github_labels_feature ... ok
test tasks::client::tests::test_build_breakdown_comment_single_epic ... ok
test tasks::client::tests::test_build_breakdown_comment_epic_scale ... ok
test tasks::client::tests::test_build_child_request ... ok
test tasks::client::tests::test_build_epic_request ... ok
test tasks::client::tests::test_build_epic_request_epic_type_for_large_work ... ok
test tasks::client::tests::test_child_task_spec_serialization ... ok
test tasks::client::tests::test_client_default ... ok
test tasks::client::tests::test_epic_task_result_serialization ... ok
test tasks::client::tests::test_epic_scale_result_serialization ... ok
test tasks::client::tests::test_file_task_request_serialization ... ok
test doctor::categories::tests::test_check_config_invalid_yaml ... ok
test doctor::drift::tests::test_detect_closed_task_closed_issue ... ok
test doctor::drift::tests::test_detect_closed_task_open_issue ... ok
test doctor::categories::tests::test_check_config_missing_keys ... ok
test doctor::categories::tests::test_check_plans_valid ... ok
test doctor::categories::tests::test_check_config_valid ... ok
test doctor::drift::tests::test_detect_in_progress_task_closed_issue ... ok
test doctor::categories::tests::test_check_plans_all_present ... ok
test doctor::drift::tests::test_detect_missing_issue_treated_as_closed ... ok
test doctor::drift::tests::test_detect_open_task_open_issue ... ok
test doctor::drift::tests::test_drift_event_has_issue_url_and_task_id ... ok
test doctor::drift::tests::test_multiple_tasks_mixed_drift ... ok
test doctor::drift::tests::test_no_drift ... ok
test doctor::drift::tests::test_orphan_task ... ok
test doctor::fix::tests::test_fix_choice_equality ... ok
test doctor::fix::tests::test_event_presentation_includes_identifiers ... ok
test doctor::fix::tests::test_fix_choice_parsing ... ok
test doctor::fix::tests::test_fix_prompts_for_confirmation ... ok
test doctor::fix::tests::test_fix_result_debug ... ok
test doctor::fix::tests::test_non_interactive_detection ... ok
test doctor::fix::tests::test_orphan_task_shows_different_options ... ok
test doctor::fix::tests::test_skip_moves_to_next ... ok
test doctor::fix::tests::test_fix_presents_event_with_options ... ok
test doctor::fix::tests::test_summarize_results_all_skipped ... ok
test doctor::fix::tests::test_summarize_results_mixed ... ok
test doctor::fix::tests::test_user_cancel_returns_quit_action ... ok
test doctor::report::tests::test_format_category_fail ... ok
test doctor::report::tests::test_format_category_pass ... ok
test doctor::report::tests::test_format_category_warn ... ok
test doctor::report::tests::test_text_report_all_pass ... ok
test doctor::report::tests::test_json_report ... ok
test doctor::report::tests::test_text_report_with_failures ... ok
test doctor::tests::test_all_pass_no_drift_exits_0 ... ok
test doctor::tests::test_auth_fail_exits_1_listed ... ok
test doctor::tests::test_category_fail_exits_1 ... ok
test doctor::tests::test_config_auth_drift_failure_exits_1_all_listed ... ok
test doctor::tests::test_config_fail_exits_1_listed ... ok
test doctor::tests::test_drift_detected_exits_1 ... ok
test doctor::tests::test_drift_detected_exits_1_events_listed ... ok
test doctor::tests::test_multiple_failures_exits_1_all_listed ... ok
test doctor::tests::test_skipped_categories_ignored ... ok
test doctor::tests::test_warnings_still_exits_0 ... ok
test feature_bug::breakdown::tests::test_all_acceptance_criteria_tracks_sources ... ok
test feature_bug::breakdown::tests::test_build_epic_description_enriched_full ... ok
test feature_bug::breakdown::tests::test_bug_type_sets_task_type ... ok
test feature_bug::breakdown::tests::test_closely_related_areas_allows_api_db ... ok
test feature_bug::breakdown::tests::test_child_tasks_status_is_deferred ... ok
test feature_bug::breakdown::tests::test_count_codebase_areas ... ok
test feature_bug::breakdown::tests::test_count_codebase_areas_auth_api_ui_database ... ok
test feature_bug::breakdown::tests::test_count_implementation_steps ... ok
test feature_bug::breakdown::tests::test_detect_child_task_areas_auth ... ok
test feature_bug::breakdown::tests::test_detect_child_task_areas_empty ... ok
test feature_bug::breakdown::tests::test_detect_child_task_areas_multiple ... ok
test feature_bug::breakdown::tests::test_detect_sequential_work ... ok
test feature_bug::breakdown::tests::test_epic_description_acceptance_criteria_from_comments ... ok
test feature_bug::breakdown::tests::test_epic_description_has_github_issue_number ... ok
test feature_bug::breakdown::tests::test_epic_description_has_all_acceptance_criteria ... ok
test feature_bug::breakdown::tests::test_epic_description_has_plan_reference ... ok
test feature_bug::breakdown::tests::test_epic_description_no_criteria_pending_review ... ok
test feature_bug::breakdown::tests::test_epic_description_has_what_why_summary ... ok
test feature_bug::breakdown::tests::test_epic_scale_result_includes_reasons ... ok
test feature_bug::breakdown::tests::test_epic_scale_complex_multi_area ... ok
test feature_bug::breakdown::tests::test_epic_scale_sequential_pattern ... ok
test feature_bug::breakdown::tests::test_epic_scale_single_area_detected ... ok
test feature_bug::breakdown::tests::test_epic_scale_with_four_distinct_areas ... ok
test feature_bug::breakdown::tests::test_extract_ac_logical_units ... ok
test feature_bug::breakdown::tests::test_execute_breakdown_epic_scale ... ok
test feature_bug::breakdown::tests::test_extract_acceptance_criteria ... ok
test feature_bug::breakdown::tests::test_epic_status_is_deferred ... ok
test feature_bug::breakdown::tests::test_execute_breakdown_single_epic ... ok
test feature_bug::breakdown::tests::test_generate_child_tasks_falls_back_to_ac_units ... ok
test feature_bug::breakdown::tests::test_feature_type_sets_task_type ... ok
test feature_bug::breakdown::tests::test_generate_standalone_task_cli ... ok
test feature_bug::breakdown::tests::test_generate_standalone_task_database ... ok
test feature_bug::breakdown::tests::test_generate_standalone_task_ui ... ok
test feature_bug::breakdown::tests::test_standalone_task_has_all_sections ... ok
test feature_bug::breakdown::tests::test_standalone_task_compound_pattern_detection ... ok
test feature_bug::breakdown::tests::test_generate_standalone_task_api ... ok
test feature_bug::breakdown::tests::test_standalone_task_missing_sections ... ok
test feature_bug::breakdown::tests::test_standalone_task_is_standalone_ready_full ... ok
test feature_bug::breakdown::tests::test_standalone_task_to_markdown ... ok
test feature_bug::breakdown::tests::test_standalone_task_validation_multiple_issues ... ok
test feature_bug::breakdown::tests::test_standalone_task_single_codebase_part ... ok
test feature_bug::breakdown::tests::test_standalone_issue_description ... ok
test feature_bug::breakdown::tests::test_standalone_validation_descriptions ... ok
test feature_bug::completeness::tests::test_bug_completeness_result_usable_for_transition ... ok
test feature_bug::completeness::tests::test_bug_missing_environment_only_requests_environment ... ok
test feature_bug::completeness::tests::test_complete_bug ... ok
test feature_bug::completeness::tests::test_bug_missing_steps_and_expected_requests_both ... ok
test feature_bug::completeness::tests::test_complete_bug_with_alternate_headers ... ok
test feature_bug::completeness::tests::test_complete_bug_with_bullet_reproduction ... ok
test feature_bug::breakdown::tests::test_validate_tasks_standalone_one_fails ... ok
test feature_bug::completeness::tests::test_complete_bug_with_na_reproduction ... ok
test feature_bug::completeness::tests::test_complete_feature ... ok
test feature_bug::completeness::tests::test_complete_feature_with_checkboxes ... ok
test feature_bug::completeness::tests::test_complete_feature_with_user_story ... ok
test feature_bug::completeness::tests::test_feature_completeness_result_usable_for_transition ... ok
test feature_bug::completeness::tests::test_feature_missing_acceptance_criteria_only_requests_that ... ok
test feature_bug::completeness::tests::test_incomplete_bug ... ok
test feature_bug::completeness::tests::test_incomplete_feature ... ok
test feature_bug::breakdown::tests::test_validate_tasks_standalone_all_pass ... ok
test feature_bug::completeness::tests::test_needs_information_label_would_be_applied ... ok
test feature_bug::completeness::tests::test_no_generic_please_provide_more_details ... ok
test feature_bug::completeness::tests::test_request_message_includes_all_missing_fields ... ok
test feature_bug::tests::test_acceptance_criteria_no_criteria_yet ... ok
test feature_bug::tests::test_all_acceptance_criteria_checked_vs_unchecked ... ok
test feature_bug::tests::test_all_acceptance_criteria_deduplicates ... ok
test feature_bug::tests::test_all_acceptance_criteria_format_for_epic_empty ... ok
test feature_bug::tests::test_all_acceptance_criteria_sources ... ok
test feature_bug::tests::test_bug_needs_information_transition ... ok
test feature_bug::tests::test_bug_ready_for_review_transition ... ok
test feature_bug::tests::test_complete_bug_workflow ... ok
test feature_bug::tests::test_complete_feature_workflow ... ok
test feature_bug::tests::test_extract_acceptance_criteria_from_body_ac_n_pattern ... ok
test feature_bug::tests::test_extract_acceptance_criteria_from_body_no_ac_section ... ok
test feature_bug::tests::test_extract_acceptance_criteria_from_body_section ... ok
test feature_bug::tests::test_extract_all_acceptance_criteria_combines_body_and_comments ... ok
test feature_bug::tests::test_extract_all_acceptance_criteria_no_criteria_pending ... ok
test feature_bug::tests::test_extract_checkbox_item_ac_pattern ... ok
test feature_bug::tests::test_extract_checkbox_item_basic ... ok
test feature_bug::tests::test_extract_checkbox_item_empty_or_short ... ok
test feature_bug::tests::test_extract_issue_sections_basic ... ok
test feature_bug::tests::test_extract_issue_sections_with_content ... ok
test feature_bug::tests::test_feature_needs_information_transition ... ok
test feature_bug::tests::test_feature_ready_for_review_transition ... ok
test feature_bug::tests::test_full_validation_issue_summary ... ok
test feature_bug::tests::test_generate_what_why_summary_bug ... ok
test feature_bug::tests::test_generate_what_why_summary_feature ... ok
test feature_bug::tests::test_generated_acceptance_criteria_includes_bug_criteria ... ok
test feature_bug::tests::test_generated_acceptance_criteria_includes_feature_criteria ... ok
test feature_bug::tests::test_incomplete_bug_workflow_in_one_run ... ok
test feature_bug::tests::test_no_delay_in_transition ... ok
test feature_bug::tests::test_validate_task_standalone_compound_pattern ... ok
test feature_bug::tests::test_validate_task_standalone_missing_sections ... ok
test feature_bug::tests::test_validate_task_standalone_full_pass ... ok
test feature_bug::tests::test_validate_task_standalone_multiple_areas ... ok
test feature_bug::tests::test_validate_no_compound_pattern_and_then ... ok
test feature_bug::tests::test_validate_no_compound_pattern_clean ... ok
test feature_bug::tests::test_validate_no_compound_pattern_first_second ... ok
test feature_bug::tests::test_validate_no_compound_pattern_numbered_list_ok ... ok
test feature_bug::tests::test_validate_no_compound_pattern_step_numbers ... ok
test feature_bug::tests::test_validate_single_codebase_part_api_and_db_allowed ... ok
test feature_bug::tests::test_validate_single_codebase_part_api_only ... ok
test feature_bug::tests::test_validate_single_codebase_part_multiple_areas ... ok
test feature_bug::tests::test_validate_standalone_sections_all_present ... ok
test feature_bug::tests::test_validate_standalone_sections_case_insensitive ... ok
test feature_bug::tests::test_validate_standalone_sections_missing_one ... ok
test feature_bug::tests::test_what_why_summary_fallback_when_no_sections ... ok
test feature_bug::tests::test_what_why_summary_format_for_epic ... ok
test feature_bug::will_not_do::tests::test_has_will_not_do_label_empty ... ok
test feature_bug::will_not_do::tests::test_has_will_not_do_label_false ... ok
test feature_bug::will_not_do::tests::test_has_will_not_do_label_true ... ok
test feature_bug::will_not_do::tests::test_processing_within_one_triage_run ... ok
test feature_bug::will_not_do::tests::test_resolve_issue_type_bug ... ok
test feature_bug::will_not_do::tests::test_resolve_issue_type_fallback ... ok
test feature_bug::will_not_do::tests::test_resolve_issue_type_feature ... ok
test feature_bug::will_not_do::tests::test_resolve_issue_type_with_multiple ... ok
test feature_bug::will_not_do::tests::test_warm_closure_comment_includes_author ... ok
test feature_bug::will_not_do::tests::test_warm_closure_comment_mentions_future ... ok
test feature_bug::will_not_do::tests::test_warm_closure_comment_no_curt_phrases ... ok
test feature_bug::will_not_do::tests::test_warm_closure_comment_tone ... ok
test feature_bug::will_not_do::tests::test_will_not_do_priority_over_ready_for_work ... ok
test feature_bug::will_not_do::tests::test_will_not_do_result_detected ... ok
test feature_bug::will_not_do::tests::test_will_not_do_result_not_detected ... ok
test github::client::tests::test_comments_url_format ... ok
test github::client::tests::test_github_client_new ... ok
test github::client::tests::test_github_client_with_api_base ... ok
test github::client::tests::test_github_client_with_token ... ok
test github::client::tests::test_github_comment_deserialization ... ok
test github::client::tests::test_github_issue_deserialization ... ok
test github::client::tests::test_github_issue_with_no_body ... ok
test github::client::tests::test_github_label_deserialization ... ok
test github::client::tests::test_github_user_deserialization ... ok
test github::client::tests::test_issue_url_format ... ok
test tests::test_fail_fast_invalid_config ... ok
test tests::test_only_category_runs_specific_category ... ok
test tests::test_continues_past_auth_when_valid ... ok
test tests::test_fail_fast_invalid_auth ... ok

test result: ok. 193 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.67s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s ✓ (391 tests)
-  ✓ (no warnings)

New tests added:
- Label heuristics: bug, enhancement, question, documentation, docs, feature, chore, maintenance, ci-cd, good-first-issue, defect, help-wanted, support, feature-request, bug-report
- Priority ordering: bug > feature > question > docs > chore
- Mixed labels: correct precedence (bug+enhancement → Bug)
- LLM fallback: unlabeled issues go to LLM, low confidence → Question
- LLM failure: None return → Question default
- Bot detection: github-actions, dependabot, bot, -app patterns
- Already-classified: rodgers:triaged, rodgers:feature preserved
- Bot issues: skipped from triage

Prompt: ISSUE_CLASSIFICATION_PROMPT added to src/llm/prompts.rs with all classification categories, confidence levels, and structured JSON output requirements.
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4a6f-2e9f-73b6-86cc-1f04c9607c3c
author: oompah
created: 2026-05-21T12:07:41Z

Agent completed successfully in 608s (4268169 tokens)
<!-- COMMENT:END -->
<!-- COMMENT:BEGIN -->
index: 019e4a6f-2fa9-74bc-acb3-db6d6de6de34
author: oompah
created: 2026-05-21T12:07:41Z

Run #1 [attempt=1, profile=default, role=fast -> InferenceAPI/nvidia/qwen/qwen3.6-35b-a3b]
- Turns: 52, Tool calls: 61
- Tokens: 4.2M in / 33.0K out [4.3M total]
- Cost: $0.0000
- Exit: normal, Duration: 10m 8s
- Log: rogers-2ey__20260521T115747Z.jsonl
<!-- COMMENT:END -->
<!-- COMMENTS:END -->
