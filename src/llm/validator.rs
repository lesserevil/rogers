//! Structured Output Validator.
//!
//! Validates LLM JSON output before Rodgers acts on it.
//! Ensures required fields are present and values are within expected bounds.

use serde::{Deserialize, Serialize};

/// Validation error with description.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub value: Option<String>,
}

impl ValidationError {
    /// Create a new validation error.
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            value: None,
        }
    }

    /// Create a validation error with the actual value.
    pub fn with_value(
        field: impl Into<String>,
        message: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            value: Some(value.into()),
        }
    }
}

/// Validation result with errors and warnings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationResult {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub errors: Vec<ValidationError>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub warnings: Vec<String>,
    pub is_valid: bool,
}

impl ValidationResult {
    /// Create a new empty result (valid by default).
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
            is_valid: true,
        }
    }

    /// Add an error.
    pub fn add_error(&mut self, error: ValidationError) {
        self.errors.push(error);
        self.is_valid = false;
    }

    /// Add a warning (does not invalidate).
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Check if there are any warnings.
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

/// Structured Output Validator.
/// Validates LLM JSON output before Rodgers acts on it.
#[derive(Clone)]
pub struct OutputValidator {
    /// Validation schema for classification output.
    classification_schema: ClassificationSchema,
}

impl std::fmt::Debug for OutputValidator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OutputValidator").finish()
    }
}

/// Schema for classification output validation.
#[derive(Clone)]
struct ClassificationSchema {
    /// Valid issue types.
    valid_issue_types: Vec<&'static str>,
    /// Valid completeness values.
    valid_completeness: Vec<&'static str>,
    /// Valid severity values.
    valid_severity: Vec<&'static str>,
    /// Valid priority values.
    valid_priority: Vec<&'static str>,
}

impl ClassificationSchema {
    fn new() -> Self {
        Self {
            valid_issue_types: vec!["bug", "feature", "question", "docs", "chore", "unknown"],
            valid_completeness: vec!["complete", "incomplete"],
            valid_severity: vec!["critical", "high", "medium", "low", "none"],
            valid_priority: vec!["critical", "high", "medium", "low"],
        }
    }
}

impl Default for OutputValidator {
    fn default() -> Self {
        Self {
            classification_schema: ClassificationSchema::new(),
        }
    }
}

impl OutputValidator {
    /// Create a new output validator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate a classification response from the LLM.
    pub fn validate_classification(
        &self,
        json_str: &str,
    ) -> Result<ClassificationOutput, ValidationResult> {
        // First, try to parse as JSON
        let raw: serde_json::Value = match serde_json::from_str(json_str) {
            Ok(v) => v,
            Err(e) => {
                let mut result = ValidationResult::new();
                result.add_error(ValidationError::new("root", format!("Invalid JSON: {}", e)));
                return Err(result);
            }
        };

        let mut result = ValidationResult::new();

        // Validate issue_type
        let issue_type = self.validate_string_field(&raw, "issue_type");
        if let Some(ref val) = issue_type {
            if !self
                .classification_schema
                .valid_issue_types
                .contains(&val.as_str())
            {
                result.add_error(ValidationError::with_value(
                    "issue_type",
                    format!(
                        "Invalid issue type '{}'. Must be one of: {:?}",
                        val, self.classification_schema.valid_issue_types
                    ),
                    val,
                ));
            }
        } else {
            result.add_error(ValidationError::new("issue_type", "Missing required field"));
        }

        // Validate completeness
        let completeness = self.validate_string_field(&raw, "completeness");
        if let Some(ref val) = completeness {
            if !self
                .classification_schema
                .valid_completeness
                .contains(&val.as_str())
            {
                result.add_error(ValidationError::with_value(
                    "completeness",
                    format!(
                        "Invalid completeness '{}'. Must be one of: {:?}",
                        val, self.classification_schema.valid_completeness
                    ),
                    val,
                ));
            }
        } else {
            result.add_error(ValidationError::new(
                "completeness",
                "Missing required field",
            ));
        }

        // Validate missing_fields is a list
        let missing_fields = self.validate_list_field(&raw, "missing_fields", &mut result);

        // Validate severity (optional, for bug/feature)
        let severity = self.validate_optional_string_field(&raw, "severity");
        if let Some(ref val) = severity {
            if !self
                .classification_schema
                .valid_severity
                .contains(&val.as_str())
            {
                result.add_error(ValidationError::with_value(
                    "severity",
                    format!(
                        "Invalid severity '{}'. Must be one of: {:?}",
                        val, self.classification_schema.valid_severity
                    ),
                    val,
                ));
            }
        }

        // Validate priority (optional, for bug/feature)
        let priority = self.validate_optional_string_field(&raw, "priority");
        if let Some(ref val) = priority {
            if !self
                .classification_schema
                .valid_priority
                .contains(&val.as_str())
            {
                result.add_error(ValidationError::with_value(
                    "priority",
                    format!(
                        "Invalid priority '{}'. Must be one of: {:?}",
                        val, self.classification_schema.valid_priority
                    ),
                    val,
                ));
            }
        }

        // Validate response_draft (optional but should be non-empty if present)
        let response_draft = self.validate_optional_string_field(&raw, "response_draft");
        if let Some(ref val) = response_draft {
            if val.trim().is_empty() {
                result.add_warning("response_draft is empty string".to_string());
            } else if val.len() < 10 {
                result.add_warning("response_draft is very short (< 10 chars)".to_string());
            }
        }

        // Validate confidence (optional, 0.0 to 1.0)
        let confidence = self.validate_optional_number_field(&raw, "confidence");
        if let Some(val) = confidence {
            if !(0.0..=1.0).contains(&val) {
                result.add_error(ValidationError::with_value(
                    "confidence",
                    "Confidence must be between 0.0 and 1.0",
                    val.to_string(),
                ));
            }
        }

        // If there are errors, return them
        if result.has_errors() {
            return Err(result);
        }

        // Parse the validated output
        let output = ClassificationOutput {
            issue_type: issue_type.unwrap(),
            completeness: completeness.unwrap(),
            missing_fields: missing_fields.unwrap_or_default(),
            severity,
            priority,
            response_draft,
            confidence,
        };

        Ok(output)
    }

    /// Validate a response draft comment.
    pub fn validate_response_draft(&self, draft: &str) -> ValidationResult {
        let mut result = ValidationResult::new();

        // Check for warmth principle violations
        let cold_patterns = [
            (
                "as previously stated",
                "Consider rephrasing to avoid making the requestor feel bad",
            ),
            (
                "please refer to the documentation",
                "Consider a warmer redirect phrasing",
            ),
            (
                "this is not a bug",
                "Consider acknowledging their perspective first",
            ),
            ("we cannot pursue", "Consider starting with gratitude"),
            (
                "why did you file this",
                "Consider an inviting approach instead",
            ),
        ];

        let draft_lower = draft.to_lowercase();
        for (pattern, suggestion) in cold_patterns {
            if draft_lower.contains(pattern) {
                result.add_warning(format!(
                    "Possible cold phrasing detected '{}': {}",
                    pattern, suggestion
                ));
            }
        }

        // Check for unnecessary urgency
        let urgency_patterns = ["!!!", "URGENT", "ASAP"];
        for pattern in urgency_patterns {
            if draft.contains(pattern) {
                result.add_warning(format!("Unnecessary urgency detected: '{}'", pattern));
            }
        }

        // Check minimum length (should be substantial enough)
        let word_count = draft.split_whitespace().count();
        if word_count < 5 && !draft.is_empty() {
            result.add_warning("Response draft is very short (< 5 words)".to_string());
        }

        result
    }

    /// Validate a string field exists and is non-empty.
    fn validate_string_field(&self, raw: &serde_json::Value, field: &str) -> Option<String> {
        raw.get(field)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
    }

    /// Validate an optional string field exists.
    fn validate_optional_string_field(
        &self,
        raw: &serde_json::Value,
        field: &str,
    ) -> Option<String> {
        raw.get(field)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
    }

    /// Validate a list field exists.
    fn validate_list_field(
        &self,
        raw: &serde_json::Value,
        field: &str,
        result: &mut ValidationResult,
    ) -> Option<Vec<String>> {
        raw.get(field).and_then(|v| {
            if v.is_array() {
                let items: Vec<String> = v
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(|item| item.as_str().map(String::from))
                    .collect();
                Some(items)
            } else if v.is_null() {
                Some(Vec::new())
            } else {
                result.add_error(ValidationError::with_value(
                    field,
                    "Must be an array of strings",
                    v.to_string(),
                ));
                None
            }
        })
    }

    /// Validate an optional number field.
    fn validate_optional_number_field(&self, raw: &serde_json::Value, field: &str) -> Option<f64> {
        raw.get(field).and_then(|v| v.as_f64())
    }
}

/// Classification output from the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationOutput {
    /// The classified issue type.
    pub issue_type: String,
    /// Whether the issue has complete information.
    pub completeness: String,
    /// List of missing required fields.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub missing_fields: Vec<String>,
    /// Severity assessment (for bug/feature).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
    /// Priority assessment (for bug/feature).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    /// Draft response comment to post.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_draft: Option<String>,
    /// LLM confidence in the classification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_classification() {
        let validator = OutputValidator::new();
        let json = r#"{
            "issue_type": "bug",
            "completeness": "incomplete",
            "missing_fields": ["reproduction_steps", "environment"],
            "severity": "high",
            "priority": "high",
            "confidence": 0.95
        }"#;

        let result = validator.validate_classification(json);
        assert!(result.is_ok(), "Valid classification should pass");
        let output = result.unwrap();
        assert_eq!(output.issue_type, "bug");
        assert_eq!(output.completeness, "incomplete");
        assert_eq!(output.missing_fields.len(), 2);
    }

    #[test]
    fn test_validate_invalid_issue_type() {
        let validator = OutputValidator::new();
        let json = r#"{
            "issue_type": "invalid_type",
            "completeness": "complete"
        }"#;

        let result = validator.validate_classification(json);
        assert!(result.is_err(), "Invalid issue type should fail");
        let errors = result.unwrap_err().errors;
        assert!(errors.iter().any(|e| e.field == "issue_type"));
    }

    #[test]
    fn test_validate_missing_required_field() {
        let validator = OutputValidator::new();
        let json = r#"{
            "issue_type": "bug"
        }"#;

        let result = validator.validate_classification(json);
        assert!(result.is_err(), "Missing completeness should fail");
    }

    #[test]
    fn test_validate_invalid_json() {
        let validator = OutputValidator::new();
        let json = "not valid json";

        let result = validator.validate_classification(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_invalid_completeness() {
        let validator = OutputValidator::new();
        let json = r#"{
            "issue_type": "bug",
            "completeness": "partial"
        }"#;

        let result = validator.validate_classification(json);
        assert!(result.is_err());
        let errors = result.unwrap_err().errors;
        assert!(errors.iter().any(|e| e.field == "completeness"));
    }

    #[test]
    fn test_validate_invalid_confidence() {
        let validator = OutputValidator::new();
        let json = r#"{
            "issue_type": "bug",
            "completeness": "complete",
            "confidence": 1.5
        }"#;

        let result = validator.validate_classification(json);
        assert!(result.is_err());
        let errors = result.unwrap_err().errors;
        assert!(errors.iter().any(|e| e.field == "confidence"));
    }

    #[test]
    fn test_validate_missing_fields_array() {
        let validator = OutputValidator::new();
        let json = r#"{
            "issue_type": "bug",
            "completeness": "incomplete",
            "missing_fields": ["steps", "version"]
        }"#;

        let result = validator.validate_classification(json);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.missing_fields, vec!["steps", "version"]);
    }

    #[test]
    fn test_validate_missing_fields_null() {
        let validator = OutputValidator::new();
        let json = r#"{
            "issue_type": "bug",
            "completeness": "complete",
            "missing_fields": null
        }"#;

        let result = validator.validate_classification(json);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.missing_fields.is_empty());
    }

    #[test]
    fn test_validate_response_draft_warmth() {
        let validator = OutputValidator::new();

        // Cold phrasing should produce warning
        let cold_draft = "As previously stated in the documentation, please refer to the docs.";
        let result = validator.validate_response_draft(cold_draft);
        assert!(result.has_warnings());
    }

    #[test]
    fn test_validate_response_draft_ok() {
        let validator = OutputValidator::new();

        let good_draft = "Hi @user, thanks for reaching out! I've found some documentation that might help with your question. Let me link it above.";
        let result = validator.validate_response_draft(good_draft);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_validate_invalid_missing_fields_type() {
        let validator = OutputValidator::new();
        let json = r#"{
            "issue_type": "bug",
            "completeness": "incomplete",
            "missing_fields": "not an array"
        }"#;

        let result = validator.validate_classification(json);
        assert!(result.is_err());
        let errors = result.unwrap_err().errors;
        assert!(errors.iter().any(|e| e.field == "missing_fields"));
    }

    #[test]
    fn test_validation_result_add_warning() {
        let mut result = ValidationResult::new();
        result.add_warning("Test warning");
        assert!(result.has_warnings());
        assert_eq!(result.warnings.len(), 1);
        assert!(result.is_valid); // Warning should not affect validity
    }

    #[test]
    fn test_validation_result_add_error() {
        let mut result = ValidationResult::new();
        result.add_error(ValidationError::new("field", "Test error"));
        assert!(result.has_errors());
        assert!(!result.is_valid);
    }
}
