//! Reformat logic with approval gate.
//!
//! This module implements the "never reformat without consent" principle.
//! When an issue is non-conforming, Rodgers offers to reformat it.
//! The reformat only happens if the requestor explicitly approves.
//!
//! ## Two-Step Approval Workflow (CRIT-5)
//!
//! Rodgers implements a two-step review to prevent mistakes:
//!
//! STEP 1 - Reformat Offer:
//! 1. Rodgers detects a non-conforming issue
//! 2. Rodgers posts a reformat offer comment
//! 3. Rodgers waits for the requestor's response
//! 4. On explicit approval (first step): generate reformatted content
//! 5. POST reformatted content as COMMENT for review (NOT yet updating issue)
//! 6. Rodgers asks: "Does this look right? If so, I'll update the issue."
//!
//! STEP 2 - Review Confirmation:
//! 7. Rodgers waits for requestor to review the reformatted version
//! 8. On second approval: update issue body to reformatted content
//! 9. Remove needs-information label if present
//! 10. On decline/rejection: keep original, triage continues
//!
//! ## Key Principle
//!
//! Rodgers never reformats without explicit consent AND review.
//! The requestor always sees the exact result before the issue is modified.

use serde::{Deserialize, Serialize};

/// Response from the requestor regarding a reformat offer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalResponse {
    /// Requestor explicitly approved the reformat.
    Approved,
    /// Requestor explicitly declined the reformat.
    Declined,
    /// Response is unclear - Rodgers should ask for clarification.
    Ambiguous,
    /// No response from the requestor.
    NoResponse,
}

impl ApprovalResponse {
    /// Returns true if this response approves the reformat.
    pub fn is_approved(&self) -> bool {
        matches!(self, ApprovalResponse::Approved)
    }

    /// Returns true if this response declines the reformat.
    pub fn is_declined(&self) -> bool {
        matches!(self, ApprovalResponse::Declined)
    }

    /// Returns true if this response requires clarification.
    pub fn is_ambiguous(&self) -> bool {
        matches!(self, ApprovalResponse::Ambiguous)
    }

    /// Returns true if no response was given.
    pub fn is_no_response(&self) -> bool {
        matches!(self, ApprovalResponse::NoResponse)
    }

    /// Human-readable description of this response.
    pub fn description(&self) -> &'static str {
        match self {
            ApprovalResponse::Approved => "Approved",
            ApprovalResponse::Declined => "Declined",
            ApprovalResponse::Ambiguous => "Ambiguous",
            ApprovalResponse::NoResponse => "No response",
        }
    }
}

/// Tokenize text into individual words for word-boundary-aware matching.
fn tokenize_words(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        if ch.is_alphanumeric() {
            current.push(ch.to_ascii_lowercase());
        } else if !current.is_empty() {
            tokens.push(current.clone());
            current.clear();
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

/// Response from the requestor regarding a reformatted version review.
///
/// This is a second-layer response after the user has seen the draft
/// reformatted content and is confirming or modifying it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewResponse {
    /// Requestor approved the reformatted version - apply it.
    Confirmed,
    /// Requestor wants modifications to the reformatted version.
    WantsChanges,
    /// Requestor rejected the reformatted version - keep original.
    Rejected,
    /// Response is unclear - Rodgers should ask for clarification.
    Ambiguous,
    /// No response from the requestor.
    NoResponse,
}

impl ReviewResponse {
    /// Returns true if this response confirms the reformat.
    pub fn is_confirmed(&self) -> bool {
        matches!(self, ReviewResponse::Confirmed)
    }

    /// Returns true if this response wants changes to the reformat.
    pub fn wants_changes(&self) -> bool {
        matches!(self, ReviewResponse::WantsChanges)
    }

    /// Returns true if this response rejects the reformat (keep original).
    pub fn is_rejected(&self) -> bool {
        matches!(self, ReviewResponse::Rejected)
    }

    /// Returns true if this response requires clarification.
    pub fn is_ambiguous(&self) -> bool {
        matches!(self, ReviewResponse::Ambiguous)
    }

    /// Human-readable description of this response.
    pub fn description(&self) -> &'static str {
        match self {
            ReviewResponse::Confirmed => "Confirmed",
            ReviewResponse::WantsChanges => "Wants changes",
            ReviewResponse::Rejected => "Rejected",
            ReviewResponse::Ambiguous => "Ambiguous",
            ReviewResponse::NoResponse => "No response",
        }
    }
}

/// Track the state of the reformat workflow for an issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReformatState {
    /// No reformat action taken yet.
    None,
    /// Reformat offer has been posted, awaiting initial approval.
    AwaitingApproval,
    /// Reformatted content has been posted for review, awaiting confirmation.
    AwaitingReview,
    /// Reformat confirmed, issue has been updated.
    Completed,
    /// Reformat was declined or rejected at some step.
    Declined,
}

impl ReformatState {
    /// Returns true if we're past the initial approval step.
    pub fn is_past_approval(&self) -> bool {
        matches!(
            self,
            ReformatState::AwaitingReview | ReformatState::Completed | ReformatState::Declined
        )
    }

    /// Returns true if the reformat was completed (issue updated).
    pub fn is_completed(&self) -> bool {
        matches!(self, ReformatState::Completed)
    }

    /// Check if a comment is a response to a pending review.
    pub fn is_awaiting_review(&self) -> bool {
        matches!(self, ReformatState::AwaitingReview)
    }
}

/// Detect the requestor's response to a reformat offer.
///
/// Checks the comment body against known approval/decline patterns.
/// Uses priority-based detection with phrase-level checks first,
/// then token-level checks.
///
/// Detection order:
/// 1. Explicit decline phrases → Declined (highest priority)
/// 2. "no problem" / "no issue" patterns → Declined (with exceptions)
/// 3. Explicit approval phrases → Approved
/// 4. Ambiguous signals → Ambiguous
/// 5. Approval tokens ("yes", "sure", "please do", etc.) → Approved
/// 6. Decline tokens ("no", "nah", "not", "don't") → Declined
/// 7. Ambiguous tokens ("maybe", "perhaps", "think") → Ambiguous
///
/// # Arguments
///
/// * `body` - The comment body text from the requestor
///
/// # Returns
///
/// An `ApprovalResponse` indicating the detected response type.
pub fn detect_approval_response(body: &str) -> ApprovalResponse {
    let text = body.to_lowercase();
    let tokens = tokenize_words(&text);

    // PHASE 1: Explicit decline phrases (highest priority)
    if text.contains("no thanks") || text.contains("no thank you") {
        return ApprovalResponse::Declined;
    }
    if text.contains("leave it as is") || text.contains("leave as is") {
        return ApprovalResponse::Declined;
    }
    if text.contains("please don't") || text.contains("please dont") {
        return ApprovalResponse::Declined;
    }
    if text.contains("don't bother") || text.contains("dont bother") {
        return ApprovalResponse::Declined;
    }
    if text.contains("not necessary") || text.contains("not needed") {
        return ApprovalResponse::Declined;
    }
    if text.contains("rather not") {
        return ApprovalResponse::Declined;
    }
    if text.contains("i decline") || text.contains("i disagree") {
        return ApprovalResponse::Declined;
    }
    if text.contains("no thanks, i prefer")
        || text.contains("never mind")
        || text.contains("nevermind")
    {
        return ApprovalResponse::Declined;
    }

    // PHASE 2: "no problem" / "no issue" patterns
    // "no problem" = dismissive (not enthusiastic acceptance)
    if tokens.contains(&"no".to_string())
        && (tokens.contains(&"problem".to_string()) || tokens.contains(&"issue".to_string()))
    {
        // "no problem" paired with an encouraging word = enthusiastic
        if tokens.contains(&"sure".to_string())
            || tokens.contains(&"okay".to_string())
            || tokens.contains(&"yes".to_string())
            || tokens.contains(&"please".to_string())
        {
            return ApprovalResponse::Approved;
        }
        // "no problem" alone = dismissive decline
        return ApprovalResponse::Declined;
    }

    // PHASE 3: Explicit approval phrases
    if text.contains("go ahead") || text.contains("go for it") {
        return ApprovalResponse::Approved;
    }
    if text.contains("looks good") || text.contains("sounds good") {
        return ApprovalResponse::Approved;
    }
    if text.contains("yes please") {
        return ApprovalResponse::Approved;
    }
    if text.contains("i approve") || text.contains("i agree") {
        return ApprovalResponse::Approved;
    }
    if text.contains("by all means")
        || text.contains("approved!")
        || text.contains("yes, please, do")
    {
        return ApprovalResponse::Approved;
    }
    if text.contains("please do!") || text.contains("please do it") {
        return ApprovalResponse::Approved;
    }

    // PHASE 4: Ambiguous single-word signals (checked early to override "not" false positives)
    // "unsure" contains "sure" but means uncertain
    if tokens.contains(&"unsure".to_string()) {
        return ApprovalResponse::Ambiguous;
    }
    if tokens.contains(&"maybe".to_string()) || tokens.contains(&"perhaps".to_string()) {
        return ApprovalResponse::Ambiguous;
    }
    // Questions asking for guidance
    if text.contains("what do you think") || text.contains("your thoughts") {
        return ApprovalResponse::Ambiguous;
    }
    if text.contains("should i") || text.contains("should I") {
        return ApprovalResponse::Ambiguous;
    }

    // PHASE 5: "not sure" combinations → Ambiguous
    if tokens.contains(&"not".to_string()) && tokens.contains(&"sure".to_string()) {
        return ApprovalResponse::Ambiguous;
    }
    // "i don't" without strong context → Ambiguous
    if text.contains("i don't") || text.contains("i do not") {
        return ApprovalResponse::Ambiguous;
    }

    // PHASE 6: "no" alone → Declined
    if tokens.contains(&"no".to_string()) {
        return ApprovalResponse::Declined;
    }

    // PHASE 7: "not" alone → Declined
    if tokens.contains(&"not".to_string()) {
        return ApprovalResponse::Declined;
    }

    // PHASE 8: Basic approval tokens
    if tokens.contains(&"yes".to_string())
        || tokens.contains(&"yeah".to_string())
        || tokens.contains(&"yep".to_string())
        || tokens.contains(&"okay".to_string())
        || tokens.contains(&"ok".to_string())
        || tokens.contains(&"okey".to_string())
        || tokens.contains(&"proceed".to_string())
        || tokens.contains(&"approved".to_string())
    {
        return ApprovalResponse::Approved;
    }

    // "sure" exact token match (not inside "unsure")
    if tokens.contains(&"sure".to_string()) {
        return ApprovalResponse::Approved;
    }

    // "please do" — both "please" AND "do" must be present
    if tokens.contains(&"please".to_string()) && tokens.contains(&"do".to_string()) {
        return ApprovalResponse::Approved;
    }

    if tokens.contains(&"agree".to_string()) {
        return ApprovalResponse::Approved;
    }

    // PHASE 9: Decline tokens
    if tokens.contains(&"leave".to_string())
        || tokens.contains(&"never".to_string())
        || tokens.contains(&"rather".to_string())
        || tokens.contains(&"decline".to_string())
        || tokens.contains(&"disagree".to_string())
        || tokens.contains(&"negative".to_string())
        || tokens.contains(&"nah".to_string())
        || tokens.contains(&"nope".to_string())
        || tokens.contains(&"dont".to_string())
    {
        return ApprovalResponse::Declined;
    }

    // PHASE 10: Remaining ambiguous signals
    if tokens.contains(&"think".to_string()) || tokens.contains(&"thoughts".to_string()) {
        return ApprovalResponse::Ambiguous;
    }

    if tokens.contains(&"should".to_string())
        || tokens.contains(&"would".to_string())
        || tokens.contains(&"could".to_string())
    {
        return ApprovalResponse::Ambiguous;
    }

    // No clear response detected
    ApprovalResponse::NoResponse
}

/// Detect the requestor's response to a reformatted version review.
///
/// This function is used for STEP 2 of the two-step approval workflow.
/// It checks if the requestor approves, wants changes, or rejects the
/// reformatted version that was posted for review.
///
/// Detection order:
/// 1. Explicit decline/rejection phrases → Rejected
/// 2. Change request phrases → WantsChanges
/// 3. Explicit approval phrases → Confirmed
/// 4. Ambiguous signals → Ambiguous
/// 5. Approval tokens ("yes", "looks good", etc.) → Confirmed
/// 6. Decline tokens ("no", "leave", etc.) → Rejected
///
/// # Arguments
///
/// * `body` - The comment body text from the requestor
///
/// # Returns
///
/// A `ReviewResponse` indicating the detected response type.
pub fn detect_review_response(body: &str) -> ReviewResponse {
    let text = body.to_lowercase();
    let tokens = tokenize_words(&text);

    // PHASE 1: Explicit rejection/decline phrases (highest priority)
    if tokens.contains(&"no".to_string())
        || text.contains("no thanks")
        || text.contains("no thank you")
        || text.contains("leave it as is")
        || text.contains("leave as is")
        || text.contains("keep it")
        || text.contains("keep my original")
        || text.contains("revert")
    {
        return ReviewResponse::Rejected;
    }

    // PHASE 2: Change request phrases (including "but" patterns)
    if text.contains("can you") || text.contains("please change") || text.contains("please modify")
    {
        return ReviewResponse::WantsChanges;
    }
    if text.contains("needs to be") || text.contains("needs to have") {
        return ReviewResponse::WantsChanges;
    }
    if text.contains("instead of") || text.contains("rather than") {
        return ReviewResponse::WantsChanges;
    }

    // "but" = unqualified change request - check before general approval
    // "looks good, but X" where X is a verb = WantsChanges (not Confirmed)
    if text.contains("but ") {
        // Check if there's an action verb after "but"
        let but_pos = text.find("but ").map(|p| p + 4).unwrap_or(0);
        let after_but = &text[but_pos..];
        // Check for action/request verbs or phrases after "but"
        if after_but.starts_with("add")
            || after_but.starts_with("change")
            || after_but.starts_with("modify")
            || after_but.starts_with("include")
            || after_but.starts_with("update")
            || after_but.starts_with("remove")
            || after_but.starts_with("fix")
            || after_but.starts_with("adjust")
            || after_but.starts_with("revise")
            || after_but.starts_with("expand")
            || after_but.starts_with("clarify")
        {
            return ReviewResponse::WantsChanges;
        }
    }

    // PHASE 3: Explicit confirmation phrases (only if not qualified with "but")
    if text.contains("looks good") || text.contains("sounds good") {
        return ReviewResponse::Confirmed;
    }
    if text.contains("go ahead") || text.contains("go for it") || text.contains("do it") {
        return ReviewResponse::Confirmed;
    }
    if text.contains("that looks") || text.contains("this looks") {
        return ReviewResponse::Confirmed;
    }
    if text.contains("perfect") || text.contains("exactly right") {
        return ReviewResponse::Confirmed;
    }
    if text.contains("i approve") || text.contains("approved") {
        return ReviewResponse::Confirmed;
    }

    // PHASE 4: Ambiguous single-word signals
    if tokens.contains(&"maybe".to_string()) || tokens.contains(&"perhaps".to_string()) {
        return ReviewResponse::Ambiguous;
    }
    if text.contains("what about") || text.contains("what if") {
        return ReviewResponse::Ambiguous;
    }
    if tokens.contains(&"unsure".to_string()) || tokens.contains(&"uncertain".to_string()) {
        return ReviewResponse::Ambiguous;
    }

    // PHASE 5: Approval tokens
    if tokens.contains(&"yes".to_string())
        || tokens.contains(&"yeah".to_string())
        || tokens.contains(&"yep".to_string())
        || tokens.contains(&"okay".to_string())
        || tokens.contains(&"ok".to_string())
        || tokens.contains(&"agree".to_string())
        || tokens.contains(&"confirmed".to_string())
    {
        return ReviewResponse::Confirmed;
    }

    // PHASE 6: Decline tokens
    if tokens.contains(&"nah".to_string()) || tokens.contains(&"nope".to_string()) {
        return ReviewResponse::Rejected;
    }
    if tokens.contains(&"never".to_string()) || tokens.contains(&"decline".to_string()) {
        return ReviewResponse::Rejected;
    }

    // PHASE 7: Amendment signals
    if text.contains("except") || text.contains("except for") {
        return ReviewResponse::WantsChanges;
    }
    if text.contains("add") || text.contains("include") {
        return ReviewResponse::WantsChanges;
    }

    // No clear response detected
    ReviewResponse::NoResponse
}

/// Check if a review response confirms the reformat should be applied.
///
/// This is the main gate for applying the reformatted content to the issue.
/// It only returns true when the response is explicitly confirmed.
///
/// # Arguments
///
/// * `response` - The review response to check
///
/// # Returns
///
/// `true` if the reformatted content should be applied
pub fn should_apply_reformat(response: ReviewResponse) -> bool {
    response.is_confirmed()
}

/// Check if a review response should keep the original format.
///
/// # Arguments
///
/// * `response` - The review response to check
///
/// # Returns
///
/// `true` if the original format should be preserved
pub fn should_keep_original(response: ReviewResponse) -> bool {
    response.is_rejected()
}

/// Check if a review response wants changes to the reformatted content.
///
/// # Arguments
///
/// * `response` - The review response to check
///
/// # Returns
///
/// `true` if the user wants modifications to the reformatted content
pub fn wants_reformat_changes(response: ReviewResponse) -> bool {
    response.wants_changes()
}

/// Check if a review response needs clarification.
///
/// # Arguments
///
/// * `response` - The review response to check
///
/// # Returns
///
/// `true` if clarification is needed
pub fn review_needs_clarification(response: ReviewResponse) -> bool {
    response.is_ambiguous()
}

/// Check if a comment appears to be a review comment (responds to reformatted content).
///
/// This checks for patterns that indicate the comment is responding to a review
/// request, as opposed to being a general comment or unrelated discussion.
///
/// # Arguments
///
/// * `body` - The comment body text
///
/// # Returns
///
/// `true` if the comment appears to be a review response
pub fn looks_like_review_response(body: &str) -> bool {
    let text = body.to_lowercase();
    let tokens = tokenize_words(&text);

    // Strong review indicators
    if text.contains("reformatted") || text.contains("formatted version") {
        return true;
    }
    if text.contains("does this look right") || text.contains("look right") {
        return true;
    }

    // Short approval/decline responses are likely review responses
    // (but only if they contain strong signal words)
    if text.contains("looks good")
        || text.contains("sounds good")
        || text.contains("go ahead")
        || text.contains("looks fine")
    {
        return true;
    }

    // Simple yes/no without additional context suggests review response
    let token_count = tokens.len();
    if token_count <= 5
        && (tokens.contains(&"yes".to_string())
            || tokens.contains(&"no".to_string())
            || tokens.contains(&"approved".to_string())
            || tokens.contains(&"confirmed".to_string())
            || tokens.contains(&"okay".to_string()))
    {
        return true;
    }

    false
}

// ============================================================================
// Pure helper functions
// ============================================================================

/// Check if an approval response should result in reformatting.
///
/// This is the main gate for reformatting - it only returns true
/// when the response is explicitly approved.
pub fn should_reformat(response: ApprovalResponse) -> bool {
    response.is_approved()
}

/// Check if a decline response should result in accepting the freeform submission.
///
/// This is the main gate for accepting freeform submissions - it only returns true
/// when the response is explicitly declined.
pub fn should_accept_freeform(response: ApprovalResponse) -> bool {
    response.is_declined()
}

/// Check if the response is ambiguous and needs clarification.
pub fn needs_clarification(response: ApprovalResponse) -> bool {
    response.is_ambiguous()
}

// ============================================================================
// Content generation functions
// ============================================================================

/// Generate the reformat offer comment body.
pub fn generate_reformat_offer_comment(requestor: &str, template_type: &str) -> String {
    format!(
        "Hi @{requestor}, thanks for reaching out! We use issue templates to make sure we gather all the information needed to understand and address your request.\n\nIt looks like this was submitted without a template. Would you like help reformatting it? I'll rewrite it using the {template_type} template based on what you've shared — just confirm below and I'll post the reformatted version for your review.\n\n**To approve**: Reply with \"yes\", \"please do\", \"go ahead\", \"looks good\", or similar.\n**To decline**: Reply with \"no\", \"don't\", \"leave it as is\", \"no thanks\", or similar.\n\nIf I don't hear back, I'll proceed with triage based on what you've shared.",
        requestor = requestor,
        template_type = template_type
    )
}

/// Generate reformatted content from freeform input.
fn generate_reformatted_bug_report(content: &str) -> String {
    format!(
        "---
name: Bug Report
about: Report something that isn't working as expected
labels: bug
---

## Bug Summary
{summary}

## Environment
- OS:
- Version:
- Other relevant context:

## Steps to Reproduce
1.

## Expected Behavior


## Actual Behavior


## Relevant Logs / Error Messages


## Possible Cause
<!-- Optional: your theory on why this is happening. Leave blank if unknown. -->

<!-- template: bug_report -->",
        summary = extract_first_paragraph_or_line(content)
    )
}

/// Generate a reformatted feature request from freeform content.
fn generate_reformatted_feature_request(content: &str) -> String {
    format!(
        "---
name: Feature Request
about: Suggest a new capability or behavioral change
labels: feature
---

## Feature Summary
{summary}

## Use Case
Why do you need this? What problem does this solve?

## Proposed Behavior
How should this feature work?

## Acceptance Criteria
1.

## Alternatives Considered
<!-- Optional: other approaches you considered -->

<!-- template: feature_request -->",
        summary = extract_first_paragraph_or_line(content)
    )
}

/// Generate a reformatted question from freeform content.
fn generate_reformatted_question(content: &str) -> String {
    format!(
        "---
name: Question
about: Ask about how to use or configure the project
labels: question
---

## Question
{question}

## Context
Provide enough context for someone to answer without来回往返.

<!-- template: question -->",
        question = extract_first_paragraph_or_line(content)
    )
}

/// Extract the first paragraph or line for summary fields.
fn extract_first_paragraph_or_line(content: &str) -> String {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return "[Brief description of the issue]".to_string();
    }

    let first_line = trimmed.lines().next().unwrap_or(trimmed);
    if first_line.len() > 80 {
        format!("{}...", &first_line[..77])
    } else {
        first_line.to_string()
    }
}

/// Generate the reformatted content for an issue.
pub fn generate_reformatted_content(original_body: &str, template_type: &str) -> String {
    match template_type {
        "bug_report" => generate_reformatted_bug_report(original_body),
        "feature_request" => generate_reformatted_feature_request(original_body),
        "question" => generate_reformatted_question(original_body),
        _ => original_body.to_string(),
    }
}

/// Generate the comment posting the reformatted issue for review.
pub fn generate_reformat_review_comment(requestor: &str, reformatted_body: &str) -> String {
    let top = format!(
        "Hi @{requestor}, here's the reformatted version based on your submission:\n\n---\n\n{body}\n\n---\n\n",
        requestor = requestor,
        body = reformatted_body
    );
    let bottom = "Does this look right? If so, I'll update the issue to use this format and remove the `needs-information` label.\n\nTo approve, reply with yes, looks good, or go ahead.\nTo request changes, let me know what needs to be adjusted.\nTo keep your original format, reply with no or leave it.";
    format!("{top}{bottom}")
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // === Approval Response Core Tests ===

    #[test]
    fn test_approval_response_is_approved() {
        assert!(ApprovalResponse::Approved.is_approved());
        assert!(!ApprovalResponse::Declined.is_approved());
        assert!(!ApprovalResponse::Ambiguous.is_approved());
        assert!(!ApprovalResponse::NoResponse.is_approved());
    }

    #[test]
    fn test_approval_response_is_declined() {
        assert!(!ApprovalResponse::Approved.is_declined());
        assert!(ApprovalResponse::Declined.is_declined());
        assert!(!ApprovalResponse::Ambiguous.is_declined());
        assert!(!ApprovalResponse::NoResponse.is_declined());
    }

    #[test]
    fn test_approval_response_description() {
        assert_eq!(ApprovalResponse::Approved.description(), "Approved");
        assert_eq!(ApprovalResponse::Declined.description(), "Declined");
        assert_eq!(ApprovalResponse::Ambiguous.description(), "Ambiguous");
        assert_eq!(ApprovalResponse::NoResponse.description(), "No response");
    }

    // === Approval Phrase Tests ===

    #[test]
    fn test_detect_approval_simple_yes() {
        assert_eq!(
            detect_approval_response("Yes, please reformat it."),
            ApprovalResponse::Approved
        );
    }

    #[test]
    fn test_detect_approval_yes() {
        assert_eq!(detect_approval_response("yes"), ApprovalResponse::Approved);
    }

    #[test]
    fn test_detect_approval_yeah() {
        assert_eq!(
            detect_approval_response("Yeah, go ahead!"),
            ApprovalResponse::Approved
        );
    }

    #[test]
    fn test_detect_approval_yep() {
        assert_eq!(detect_approval_response("yep"), ApprovalResponse::Approved);
    }

    #[test]
    fn test_detect_approval_please_do() {
        assert_eq!(
            detect_approval_response("Please do!"),
            ApprovalResponse::Approved
        );
    }

    #[test]
    fn test_detect_approval_go_ahead() {
        assert_eq!(
            detect_approval_response("Go ahead and reformat it."),
            ApprovalResponse::Approved
        );
    }

    #[test]
    fn test_detect_approval_looks_good() {
        assert_eq!(
            detect_approval_response("Looks good, thanks!"),
            ApprovalResponse::Approved
        );
    }

    #[test]
    fn test_detect_approval_sounds_good() {
        assert_eq!(
            detect_approval_response("Sounds good!"),
            ApprovalResponse::Approved
        );
    }

    #[test]
    fn test_detect_approval_sure() {
        assert_eq!(
            detect_approval_response("Sure, please go ahead."),
            ApprovalResponse::Approved
        );
    }

    #[test]
    fn test_detect_approval_ok() {
        assert_eq!(
            detect_approval_response("OK, do it."),
            ApprovalResponse::Approved
        );
    }

    #[test]
    fn test_detect_approval_approved() {
        assert_eq!(
            detect_approval_response("Approved!"),
            ApprovalResponse::Approved
        );
    }

    #[test]
    fn test_detect_approval_i_approve() {
        assert_eq!(
            detect_approval_response("I approve this."),
            ApprovalResponse::Approved
        );
    }

    #[test]
    fn test_detect_approval_case_insensitive() {
        assert_eq!(detect_approval_response("YES"), ApprovalResponse::Approved);
        assert_eq!(detect_approval_response("Yes"), ApprovalResponse::Approved);
        assert_eq!(detect_approval_response("yeS"), ApprovalResponse::Approved);
    }

    // === Decline Tests ===

    #[test]
    fn test_detect_decline_no() {
        assert_eq!(
            detect_approval_response("No, leave it as is."),
            ApprovalResponse::Declined
        );
    }

    #[test]
    fn test_detect_decline_nah() {
        assert_eq!(
            detect_approval_response("Nah, I prefer it this way."),
            ApprovalResponse::Declined
        );
    }

    #[test]
    fn test_detect_decline_nope() {
        assert_eq!(detect_approval_response("nope"), ApprovalResponse::Declined);
    }

    #[test]
    fn test_detect_decline_dont() {
        assert_eq!(
            detect_approval_response("Don't bother, it's fine."),
            ApprovalResponse::Declined
        );
    }

    #[test]
    fn test_detect_decline_leave_it() {
        assert_eq!(
            detect_approval_response("Leave it as is, please."),
            ApprovalResponse::Declined
        );
    }

    #[test]
    fn test_detect_decline_no_thanks() {
        assert_eq!(
            detect_approval_response("No thanks, I like it the way it is."),
            ApprovalResponse::Declined
        );
    }

    #[test]
    fn test_detect_decline_please_dont() {
        assert_eq!(
            detect_approval_response("Please don't reformat it."),
            ApprovalResponse::Declined
        );
    }

    #[test]
    fn test_detect_decline_rather_not() {
        assert_eq!(
            detect_approval_response("I'd rather not."),
            ApprovalResponse::Declined
        );
    }

    #[test]
    fn test_detect_decline_not_necessary() {
        assert_eq!(
            detect_approval_response("Not necessary."),
            ApprovalResponse::Declined
        );
    }

    #[test]
    fn test_detect_decline_case_insensitive() {
        assert_eq!(detect_approval_response("NO"), ApprovalResponse::Declined);
        assert_eq!(detect_approval_response("No"), ApprovalResponse::Declined);
        assert_eq!(detect_approval_response("nO"), ApprovalResponse::Declined);
    }

    // === Ambiguous Tests ===

    #[test]
    fn test_detect_ambiguous_maybe() {
        // "Maybe?" only contains "maybe" = ambiguous (no encouragement/decline words)
        assert_eq!(
            detect_approval_response("Maybe?"),
            ApprovalResponse::Ambiguous
        );
    }

    #[test]
    fn test_detect_ambiguous_im_not_sure() {
        // "not sure" combination = ambiguous (uncertainty + negation)
        assert_eq!(
            detect_approval_response("I'm not sure."),
            ApprovalResponse::Ambiguous
        );
    }

    #[test]
    fn test_detect_ambiguous_unsure() {
        assert_eq!(
            detect_approval_response("I'm unsure about this."),
            ApprovalResponse::Ambiguous
        );
    }

    #[test]
    fn test_detect_ambiguous_what_do_you_think() {
        // "what do you think" = genuinely asking for guidance = ambiguous
        assert_eq!(
            detect_approval_response("What do you think?"),
            ApprovalResponse::Ambiguous
        );
    }

    #[test]
    fn test_detect_ambiguous_should_i() {
        assert_eq!(
            detect_approval_response("Should I let you reformat it?"),
            ApprovalResponse::Ambiguous
        );
    }

    // === No Response Tests ===

    #[test]
    fn test_detect_no_response_empty() {
        assert_eq!(detect_approval_response(""), ApprovalResponse::NoResponse);
    }

    #[test]
    fn test_detect_no_response_irrelevant() {
        assert_eq!(
            detect_approval_response("Has this been fixed in the latest version?"),
            ApprovalResponse::NoResponse
        );
    }

    #[test]
    fn test_detect_no_response_gibberish() {
        assert_eq!(
            detect_approval_response("qwert yuiop"),
            ApprovalResponse::NoResponse
        );
    }

    // === Edge Cases Tests ===

    #[test]
    fn test_detect_yes_but_ambiguous() {
        assert_eq!(
            detect_approval_response("Yes, but maybe make it shorter?"),
            ApprovalResponse::Ambiguous
        );
    }

    #[test]
    fn test_detect_no_thanks_actually_decline() {
        assert_eq!(
            detect_approval_response("No thanks, I prefer this format."),
            ApprovalResponse::Declined
        );
    }

    #[test]
    fn test_detect_prioritize_decline_over_approval() {
        // If both decline phrase and approval token present, decline wins
        assert_eq!(
            detect_approval_response("Yes, but please don't make it too fancy."),
            ApprovalResponse::Declined
        );
    }

    #[test]
    fn test_detect_no_problem_encouraged() {
        // "Sure, no problem" = enthusiastic acceptance
        assert_eq!(
            detect_approval_response("Sure, no problem at all."),
            ApprovalResponse::Approved
        );
    }

    #[test]
    fn test_detect_no_problem_dismissive() {
        // "No problem" alone = dismissive decline
        assert_eq!(
            detect_approval_response("No problem, just submit it as is."),
            ApprovalResponse::Declined
        );
    }

    // === Gate Function Tests ===

    #[test]
    fn test_should_reformat_approved() {
        assert!(should_reformat(ApprovalResponse::Approved));
        assert!(!should_reformat(ApprovalResponse::Declined));
        assert!(!should_reformat(ApprovalResponse::Ambiguous));
        assert!(!should_reformat(ApprovalResponse::NoResponse));
    }

    #[test]
    fn test_should_accept_freeform_declined() {
        assert!(should_accept_freeform(ApprovalResponse::Declined));
        assert!(!should_accept_freeform(ApprovalResponse::Approved));
        assert!(!should_accept_freeform(ApprovalResponse::Ambiguous));
        assert!(!should_accept_freeform(ApprovalResponse::NoResponse));
    }

    #[test]
    fn test_needs_clarification_ambiguous() {
        assert!(needs_clarification(ApprovalResponse::Ambiguous));
        assert!(!needs_clarification(ApprovalResponse::Approved));
        assert!(!needs_clarification(ApprovalResponse::Declined));
        assert!(!needs_clarification(ApprovalResponse::NoResponse));
    }

    // === Comment Generation Tests ===

    #[test]
    fn test_generate_reformat_offer_comment() {
        let comment = generate_reformat_offer_comment("john", "Bug Report");
        assert!(comment.contains("@john"));
        assert!(comment.contains("Bug Report"));
    }

    #[test]
    fn test_generate_reformat_review_comment() {
        let reformatted = "Sample reformatted issue body";
        let comment = generate_reformat_review_comment("alice", reformatted);
        assert!(comment.contains("@alice"));
        assert!(comment.contains("Sample reformatted issue body"));
    }

    // === Content Extraction Tests ===

    #[test]
    fn test_extract_first_paragraph_or_line_short() {
        let text = "My bug is that the app crashes when I click the button.";
        let summary = extract_first_paragraph_or_line(text);
        assert_eq!(
            summary,
            "My bug is that the app crashes when I click the button."
        );
    }

    #[test]
    fn test_extract_first_paragraph_or_line_long() {
        let text = "This is a very long line that exceeds the truncation threshold for the summary field in the template structure.";
        let summary = extract_first_paragraph_or_line(text);
        assert!(summary.ends_with("..."));
        assert!(summary.len() <= 80);
    }

    #[test]
    fn test_extract_first_paragraph_or_line_empty() {
        let result = extract_first_paragraph_or_line("");
        assert_eq!(result, "[Brief description of the issue]");
    }

    // === Reformatted Content Generation Tests ===

    #[test]
    fn test_generate_reformatted_content_bug_report() {
        let content = "The application crashes when I click the submit button.";
        let result = generate_reformatted_content(content, "bug_report");
        assert!(result.contains("Bug Summary"));
        assert!(result.contains("Environment"));
        assert!(result.contains("Steps to Reproduce"));
        assert!(result.contains("<!-- template: bug_report -->"));
    }

    #[test]
    fn test_generate_reformatted_content_feature_request() {
        let content = "I want to export my data to CSV.";
        let result = generate_reformatted_content(content, "feature_request");
        assert!(result.contains("Feature Summary"));
        assert!(result.contains("Use Case"));
        assert!(result.contains("Proposed Behavior"));
        assert!(result.contains("<!-- template: feature_request -->"));
    }

    #[test]
    fn test_generate_reformatted_content_question() {
        let content = "How do I configure the theme?";
        let result = generate_reformatted_content(content, "question");
        assert!(result.contains("Question"));
        assert!(result.contains("Context"));
        assert!(result.contains("<!-- template: question -->"));
    }

    #[test]
    fn test_generate_reformatted_content_unknown_type() {
        let result = generate_reformatted_content("Some content", "unknown_type");
        assert_eq!(result, "Some content");
    }

    // === Tokenization Tests ===

    #[test]
    fn test_tokenize_words_simple() {
        let tokens = tokenize_words("Hello world");
        assert_eq!(tokens, vec!["hello", "world"]);
    }

    #[test]
    fn test_tokenize_words_with_punctuation() {
        let tokens = tokenize_words("Don't bother!");
        assert_eq!(tokens, vec!["don", "t", "bother"]);
    }

    #[test]
    fn test_tokenize_words_with_apostrophe() {
        let tokens = tokenize_words("I'm not sure?");
        assert_eq!(tokens, vec!["i", "m", "not", "sure"]);
    }

    #[test]
    fn test_tokenize_words_empty() {
        assert_eq!(tokenize_words(""), Vec::<String>::new());
    }

    // === ReviewResponse Tests ===

    #[test]
    fn test_review_response_is_confirmed() {
        assert!(ReviewResponse::Confirmed.is_confirmed());
        assert!(!ReviewResponse::WantsChanges.is_confirmed());
        assert!(!ReviewResponse::Rejected.is_confirmed());
        assert!(!ReviewResponse::Ambiguous.is_confirmed());
        assert!(!ReviewResponse::NoResponse.is_confirmed());
    }

    #[test]
    fn test_review_response_wants_changes() {
        assert!(ReviewResponse::WantsChanges.wants_changes());
        assert!(!ReviewResponse::Confirmed.wants_changes());
        assert!(!ReviewResponse::Rejected.wants_changes());
    }

    #[test]
    fn test_review_response_is_rejected() {
        assert!(ReviewResponse::Rejected.is_rejected());
        assert!(!ReviewResponse::Confirmed.is_rejected());
        assert!(!ReviewResponse::WantsChanges.is_rejected());
    }

    #[test]
    fn test_review_response_description() {
        assert_eq!(ReviewResponse::Confirmed.description(), "Confirmed");
        assert_eq!(ReviewResponse::WantsChanges.description(), "Wants changes");
        assert_eq!(ReviewResponse::Rejected.description(), "Rejected");
        assert_eq!(ReviewResponse::Ambiguous.description(), "Ambiguous");
        assert_eq!(ReviewResponse::NoResponse.description(), "No response");
    }

    // === ReformatState Tests ===

    #[test]
    fn test_reformat_state_is_past_approval() {
        assert!(!ReformatState::None.is_past_approval());
        assert!(!ReformatState::AwaitingApproval.is_past_approval());
        assert!(ReformatState::AwaitingReview.is_past_approval());
        assert!(ReformatState::Completed.is_past_approval());
        assert!(ReformatState::Declined.is_past_approval());
    }

    #[test]
    fn test_reformat_state_is_completed() {
        assert!(ReformatState::Completed.is_completed());
        assert!(!ReformatState::None.is_completed());
        assert!(!ReformatState::AwaitingApproval.is_completed());
        assert!(!ReformatState::AwaitingReview.is_completed());
        assert!(!ReformatState::Declined.is_completed());
    }

    #[test]
    fn test_reformat_state_is_awaiting_review() {
        assert!(ReformatState::AwaitingReview.is_awaiting_review());
        assert!(!ReformatState::None.is_awaiting_review());
        assert!(!ReformatState::AwaitingApproval.is_awaiting_review());
        assert!(!ReformatState::Completed.is_awaiting_review());
    }

    // === Review Response Detection Tests ===

    #[test]
    fn test_detect_review_looks_good() {
        assert_eq!(
            detect_review_response("Looks good, thanks!"),
            ReviewResponse::Confirmed
        );
    }

    #[test]
    fn test_detect_review_yes() {
        assert_eq!(
            detect_review_response("Yes, that looks fine."),
            ReviewResponse::Confirmed
        );
    }

    #[test]
    fn test_detect_review_go_ahead() {
        assert_eq!(
            detect_review_response("Go ahead and update it."),
            ReviewResponse::Confirmed
        );
    }

    #[test]
    fn test_detect_review_perfect() {
        assert_eq!(
            detect_review_response("Perfect!"),
            ReviewResponse::Confirmed
        );
    }

    #[test]
    fn test_detect_review_approved() {
        assert_eq!(
            detect_review_response("Approved!"),
            ReviewResponse::Confirmed
        );
    }

    #[test]
    fn test_detect_review_confirm_case_insensitive() {
        assert_eq!(detect_review_response("YES"), ReviewResponse::Confirmed);
        assert_eq!(
            detect_review_response("Looks Good"),
            ReviewResponse::Confirmed
        );
    }

    #[test]
    fn test_detect_review_rejected_no() {
        assert_eq!(
            detect_review_response("No, leave it as is."),
            ReviewResponse::Rejected
        );
    }

    #[test]
    fn test_detect_review_rejected_keep() {
        assert_eq!(
            detect_review_response("Keep my original format."),
            ReviewResponse::Rejected
        );
    }

    #[test]
    fn test_detect_review_rejected_nah() {
        assert_eq!(
            detect_review_response("Nah, I prefer it as is."),
            ReviewResponse::Rejected
        );
    }

    #[test]
    fn test_detect_review_wants_changes() {
        assert_eq!(
            detect_review_response("Can you add more details?"),
            ReviewResponse::WantsChanges
        );
    }

    #[test]
    fn test_detect_review_wants_changes_but() {
        assert_eq!(
            detect_review_response("Looks good, but add the version number."),
            ReviewResponse::WantsChanges
        );
    }

    #[test]
    fn test_detect_review_ambiguous_maybe() {
        assert_eq!(detect_review_response("Maybe?"), ReviewResponse::Ambiguous);
    }

    #[test]
    fn test_detect_review_no_response() {
        assert_eq!(detect_review_response(""), ReviewResponse::NoResponse);
        assert_eq!(
            detect_review_response("Has this been fixed in the latest version?"),
            ReviewResponse::NoResponse
        );
    }

    #[test]
    fn test_detect_review_prioritize_rejection_over_approval() {
        // If both rejection and approval present, rejection wins
        assert_eq!(
            detect_review_response("Yes, but leave it as is."),
            ReviewResponse::Rejected
        );
    }

    // === Review Helper Function Tests ===

    #[test]
    fn test_should_apply_reformat() {
        assert!(should_apply_reformat(ReviewResponse::Confirmed));
        assert!(!should_apply_reformat(ReviewResponse::WantsChanges));
        assert!(!should_apply_reformat(ReviewResponse::Rejected));
        assert!(!should_apply_reformat(ReviewResponse::Ambiguous));
        assert!(!should_apply_reformat(ReviewResponse::NoResponse));
    }

    #[test]
    fn test_should_keep_original() {
        assert!(should_keep_original(ReviewResponse::Rejected));
        assert!(!should_keep_original(ReviewResponse::Confirmed));
        assert!(!should_keep_original(ReviewResponse::WantsChanges));
    }

    #[test]
    fn test_wants_reformat_changes() {
        assert!(wants_reformat_changes(ReviewResponse::WantsChanges));
        assert!(!wants_reformat_changes(ReviewResponse::Confirmed));
        assert!(!wants_reformat_changes(ReviewResponse::Rejected));
    }

    #[test]
    fn test_review_needs_clarification() {
        assert!(review_needs_clarification(ReviewResponse::Ambiguous));
        assert!(!review_needs_clarification(ReviewResponse::Confirmed));
        assert!(!review_needs_clarification(ReviewResponse::Rejected));
    }

    // === look_like_review_response Tests ===

    #[test]
    fn test_looks_like_review_response_short_yes() {
        assert!(looks_like_review_response("yes"));
        assert!(looks_like_review_response("Yes"));
        assert!(looks_like_review_response("YES"));
    }

    #[test]
    fn test_looks_like_review_response_short_no() {
        assert!(looks_like_review_response("no"));
        assert!(looks_like_review_response("No"));
    }

    #[test]
    fn test_looks_like_review_response_looks_good() {
        assert!(looks_like_review_response("Looks good!"));
        assert!(looks_like_review_response("sounds good"));
    }

    #[test]
    fn test_looks_like_review_response_go_ahead() {
        assert!(looks_like_review_response("Go ahead"));
    }

    #[test]
    fn test_looks_like_review_response_not_review() {
        // Long content that's not clearly a review response
        assert!(!looks_like_review_response(
            "I have been experiencing this issue for several weeks now."
        ));
    }

    #[test]
    fn test_looks_like_review_response_approved() {
        assert!(looks_like_review_response("approved"));
        assert!(looks_like_review_response("Okay, thanks"));
    }

    // === Full Workflow Integration Tests ===

    #[test]
    fn test_review_workflow_confirm_and_apply() {
        // Simulate the two-step review workflow
        let reformat_offer_response = detect_approval_response("Yes, please do");
        assert!(should_reformat(reformat_offer_response));

        // Step 1: Generate reformatted content
        let original = "The app crashes when I click the submit button.";
        let reformatted = generate_reformatted_content(original, "bug_report");
        assert!(reformatted.contains("Bug Summary"));

        // Step 2: User sees reformatted content (via review comment)
        let review_comment = generate_reformat_review_comment("alice", &reformatted);
        assert!(review_comment.contains("@alice"));
        assert!(review_comment.contains("Does this look right"));

        // Step 3: User approves review
        let review_response = detect_review_response("Looks good, go ahead!");
        assert!(should_apply_reformat(review_response));
    }

    #[test]
    fn test_review_workflow_reject_at_review() {
        // Simulate rejection at review step
        let review_response = detect_review_response("No, leave it as is.");
        assert!(should_keep_original(review_response));
    }

    #[test]
    fn test_review_workflow_wants_changes() {
        // Simulate wanting changes to reformatted version
        let review_response = detect_review_response("Can you add the OS version?");
        assert!(wants_reformat_changes(review_response));
    }
}
