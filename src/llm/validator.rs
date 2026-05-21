//! Structured Output Validator for LLM responses.
//!
//! Validates that LLM-generated answers include real, verifiable citations.
//! Zero-tolerance approach: if citations can't be verified, the answer is rejected.
//!
//! Citation formats:
//! - Doc answers: `docs/path/to/file.md:123` — file exists, line exists
//! - Code answers: `src/path/to/file.rs:45-67` — file exists, line range exists
//! - Single line: `src/path/to/file.rs:45` — file exists, single line exists

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Represents a citation extracted from an LLM response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Citation {
    /// File path relative to repo root.
    pub file_path: String,
    /// Start line number (1-indexed).
    pub line_start: usize,
    /// End line number (1-indexed), same as start if single line.
    pub line_end: usize,
}

impl Citation {
    /// Returns true if this is a single-line citation.
    pub fn is_single_line(&self) -> bool {
        self.line_start == self.line_end
    }

    /// Returns a human-readable form of this citation.
    pub fn display(&self) -> String {
        if self.is_single_line() {
            format!("{}:{}", self.file_path, self.line_start)
        } else {
            format!("{}:{}-{}", self.file_path, self.line_start, self.line_end)
        }
    }
}

/// Validation result for a single citation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CitationValidation {
    /// Citation is valid — file exists and line(s) exist.
    Valid(Citation),
    /// File doesn't exist.
    FileNotFound { citation: Citation },
    /// Line number out of range.
    LineOutOfRange { citation: Citation },
    /// Citation text couldn't be parsed.
    ParseError { raw: String },
}

impl CitationValidation {
    /// Returns true if this citation is valid.
    pub fn is_valid(&self) -> bool {
        matches!(self, CitationValidation::Valid(_))
    }

    /// Returns the citation if valid.
    pub fn into_valid(self) -> Option<Citation> {
        match self {
            CitationValidation::Valid(c) => Some(c),
            _ => None,
        }
    }
}

/// Overall validation result for an LLM response.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the entire response passed validation.
    pub valid: bool,
    /// All citations extracted from the response.
    pub citations: Vec<Citation>,
    /// Validation status of each citation.
    pub validation_results: Vec<CitationValidation>,
    /// Error messages for failed validations (empty if valid).
    pub errors: Vec<String>,
}

impl ValidationResult {
    /// Returns true if validation passed and all citations are valid.
    pub fn is_valid(&self) -> bool {
        self.valid && self.validation_results.iter().all(|v| v.is_valid())
    }

    /// Returns true if at least one citation failed validation.
    pub fn has_failures(&self) -> bool {
        self.validation_results.iter().any(|v| !v.is_valid())
    }

    /// Returns the number of valid citations.
    pub fn valid_count(&self) -> usize {
        self.validation_results.iter().filter(|v| v.is_valid()).count()
    }
}

/// StructuredOutputValidator validates LLM output before it's posted.
pub struct StructuredOutputValidator {
    repo_root: String,
    retry_count: usize,
}

impl StructuredOutputValidator {
    /// Create a new validator for the given repo root.
    pub fn new(repo_root: impl Into<String>) -> Self {
        Self {
            repo_root: repo_root.into(),
            retry_count: 3,
        }
    }

    /// Validate an LLM response. Returns ValidationResult with status of all citations.
    pub fn validate(&self, response: &str) -> ValidationResult {
        tracing::info!("Validating LLM response for citations");

        let citations = extract_citations(response);
        let validation_results: Vec<CitationValidation> = citations
            .iter()
            .map(|c| self.validate_citation(c))
            .collect();

        let all_valid = validation_results.iter().all(|v| v.is_valid());
        let errors: Vec<String> = validation_results
            .iter()
            .filter_map(|v| match v {
                CitationValidation::FileNotFound { citation } => {
                    Some(format!(
                        "Citation file not found: {}",
                        citation.display()
                    ))
                }
                CitationValidation::LineOutOfRange { citation } => {
                    Some(format!(
                        "Line number out of range: {}",
                        citation.display()
                    ))
                }
                CitationValidation::ParseError { raw } => {
                    Some(format!("Failed to parse citation: {}", raw))
                }
                CitationValidation::Valid(_) => None,
            })
            .collect();

        ValidationResult {
            valid: all_valid && !citations.is_empty(),
            citations,
            validation_results,
            errors,
        }
    }

    /// Validate a single citation, with retry for I/O failures.
    fn validate_citation(&self, citation: &Citation) -> CitationValidation {
        let full_path = Path::new(&self.repo_root).join(&citation.file_path);

        // Retry up to retry_count times for transient I/O failures
        let mut last_error = None;
        for attempt in 0..=self.retry_count {
            if attempt > 0 {
                tracing::warn!(
                    attempt,
                    file = %citation.file_path,
                    "Retry reading file for citation validation"
                );
            }

            // Check file existence
            if !full_path.exists() {
                tracing::warn!(
                    file = %citation.file_path,
                    "File not found during validation attempt",
                );
                return CitationValidation::FileNotFound {
                    citation: citation.clone(),
                };
            }

            // Read file and validate line range
            match fs::read_to_string(&full_path) {
                Ok(content) => {
                    let line_count = content.lines().count();
                    if citation.line_start == 0 {
                        return CitationValidation::ParseError {
                            raw: citation.display(),
                        };
                    }
                    if citation.line_end > line_count {
                        return CitationValidation::LineOutOfRange {
                            citation: citation.clone(),
                        };
                    }
                    return CitationValidation::Valid(citation.clone());
                }
                Err(e) => {
                    tracing::warn!(
                        attempt,
                        error = %e,
                        "Error reading file for validation, will retry"
                    );
                    last_error = Some(e);
                }
            }
        }

        // All retries exhausted — file exists but couldn't read it
        if let Some(e) = last_error {
            tracing::error!(
                error = %e,
                "All retries exhausted validating citation {}",
                citation.display()
            );
            CitationValidation::FileNotFound {
                citation: citation.clone(),
            }
        } else {
            CitationValidation::FileNotFound {
                citation: citation.clone(),
            }
        }
    }
}

impl Default for StructuredOutputValidator {
    fn default() -> Self {
        Self::new(".")
    }
}

/// Extract citations from an LLM response string.
///
/// Handles these patterns:
/// - `path/to/file.md:123` — single line
/// - `path/to/file.md:123-145` — line range
/// - Backtick-wrapped citations: `` `src/lib.rs:42` ``
/// - Markdown link citations: `[src/lib.rs:42](...)`
///
/// Deduplicates citations across all patterns so each unique
/// `file:line` appears only once.
pub fn extract_citations(response: &str) -> Vec<Citation> {
    let mut citations = Vec::new();

    // Patterns to try, in order of specificity
    let patterns: Vec<regex::Regex> = [
        // Backtick-wrapped: `path:line` or `path:line-line`
        Regex::new(r"`([^`\s]+:\d+(?:-\d+)?)`").ok(),
        // Markdown link: [path:line](url) or [path:line-line](url)
        Regex::new(r"\[([^`\]]+:\d+(?:-\d+)?)\]\(").ok(),
    ]
    .into_iter()
    .flatten()
    .collect();

    // Collect all unique citation strings first (string dedup)
    let mut seen = std::collections::HashSet::new();

    for re in &patterns {
        for cap in re.captures_iter(response) {
            if let Some(citation_str) = cap.get(1) {
                let s = citation_str.as_str().to_string();
                if seen.insert(s.clone()) {
                    if let Some(citation) = parse_citation(&s) {
                        citations.push(citation);
                    }
                }
            }
        }
    }

    // Now collect bare citations (not already captured by other patterns)
    // Remove backtick-enclosed and link-enclosed portions first
    let stripped = remove_backticks_and_links(response);
    let bare_re = Regex::new(r"(\S+:\d+(?:-\d+)?)").ok();
    if let Some(re) = bare_re {
        for cap in re.captures_iter(&stripped) {
            if let Some(citation_str) = cap.get(1) {
                let s = citation_str.as_str().to_string();
                if seen.insert(s.clone()) {
                    if let Some(citation) = parse_citation(&s) {
                        citations.push(citation);
                    }
                }
            }
        }
    }

    citations
}

/// Remove backtick-enclosed and markdown-link-enclosed text from a string
/// so that bare citation extraction doesn't double-match already-captured citations.
fn remove_backticks_and_links(input: &str) -> String {
    let mut result = input.to_string();

    // Remove backtick-enclosed content
    if let Ok(re) = Regex::new(r#"`[^`]+`"#) {
        result = re.replace_all(&result, "").to_string();
    }

    // Remove markdown links
    if let Ok(re) = Regex::new(r"\[[^\]]+\]\([^)]+\)") {
        result = re.replace_all(&result, "").to_string();
    }

    result
}

/// Parse a citation string like "src/lib.rs:42" or "src/lib.rs:42-60" into a Citation.
pub fn parse_citation(citation_str: &str) -> Option<Citation> {
    // Find the last colon that separates the file path from the line number
    // We use rfind to handle paths with colons (unlikely but safe)
    let colon_pos = citation_str.rfind(':')?;
    let file_path = &citation_str[..colon_pos];
    let line_part = &citation_str[colon_pos + 1..];

    if file_path.is_empty() || line_part.is_empty() {
        return None;
    }

    let (line_start, line_end) = if let Some(dash_pos) = line_part.find('-') {
        let start = line_part[..dash_pos].parse::<usize>().ok()?;
        let end = line_part[dash_pos + 1..].parse::<usize>().ok()?;
        if start == 0 || end == 0 || end < start {
            return None;
        }
        (start, end)
    } else {
        let line = line_part.parse::<usize>().ok()?;
        if line == 0 {
            return None;
        }
        (line, line)
    };

    Some(Citation {
        file_path: file_path.to_string(),
        line_start,
        line_end,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_repo() -> TempDir {
        let temp_dir = TempDir::new().unwrap();

        std::fs::create_dir_all(temp_dir.path().join("src")).unwrap();
        std::fs::create_dir_all(temp_dir.path().join("docs")).unwrap();

        std::fs::write(
            temp_dir.path().join("src/lib.rs"),
            "//! Test library.\n\npub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
        )
        .unwrap();

        std::fs::write(
            temp_dir.path().join("docs/getting-started.md"),
            "# Getting Started\n\n## Installation\n\nInstall with `cargo install`.\n",
        )
        .unwrap();

        temp_dir
    }

    // ---- extract_citations tests ----

    #[test]
    fn test_extract_citations_bare() {
        let response = "See src/lib.rs:2 for the implementation.";
        let citations = extract_citations(response);
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].file_path, "src/lib.rs");
        assert_eq!(citations[0].line_start, 2);
        assert_eq!(citations[0].line_end, 2);
    }

    #[test]
    fn test_extract_citations_range() {
        let response = "The relevant code is at src/lib.rs:2-5.";
        let citations = extract_citations(response);
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].line_start, 2);
        assert_eq!(citations[0].line_end, 5);
    }

    #[test]
    fn test_extract_citations_backtick() {
        let response = "Refer to `src/lib.rs:4` for details.";
        let citations = extract_citations(response);
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].file_path, "src/lib.rs");
        assert_eq!(citations[0].line_start, 4);
    }

    #[test]
    fn test_extract_citations_link() {
        let response = "See [src/lib.rs:3](https://example.com) for info.";
        let citations = extract_citations(response);
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].line_start, 3);
    }

    #[test]
    fn test_extract_citations_multiple() {
        let response = "Check src/lib.rs:2 and src/lib.rs:5.";
        let citations = extract_citations(response);
        // Two different line numbers = two different citations
        assert_eq!(citations.len(), 2);
    }

    #[test]
    fn test_extract_citations_none() {
        let response = "This response has no citations at all.";
        let citations = extract_citations(response);
        assert!(citations.is_empty());
    }

    #[test]
    fn test_extract_citations_mixed() {
        let response = "See src/lib.rs:2 and `docs/getting-started.md:3` for details.";
        let citations = extract_citations(response);
        assert_eq!(citations.len(), 2);
    }

    // ---- parse_citation tests ----

    #[test]
    fn test_parse_citation_single_line() {
        let c = parse_citation("src/lib.rs:42").unwrap();
        assert_eq!(c.file_path, "src/lib.rs");
        assert_eq!(c.line_start, 42);
        assert_eq!(c.line_end, 42);
        assert!(c.is_single_line());
    }

    #[test]
    fn test_parse_citation_range() {
        let c = parse_citation("src/lib.rs:10-20").unwrap();
        assert_eq!(c.file_path, "src/lib.rs");
        assert_eq!(c.line_start, 10);
        assert_eq!(c.line_end, 20);
        assert!(!c.is_single_line());
    }

    #[test]
    fn test_parse_citation_invalid_no_colon() {
        assert!(parse_citation("src/lib").is_none());
    }

    #[test]
    fn test_parse_citation_invalid_non_numeric() {
        assert!(parse_citation("src/lib.rs:abc").is_none());
    }

    #[test]
    fn test_parse_citation_invalid_zero_line() {
        assert!(parse_citation("src/lib.rs:0").is_none());
    }

    #[test]
    fn test_parse_citation_invalid_empty_file() {
        assert!(parse_citation(":42").is_none());
    }

    // ---- Validation tests ----

    #[test]
    fn test_validate_valid_citation() {
        let temp_dir = create_test_repo();
        let validator = StructuredOutputValidator::new(temp_dir.path().to_str().unwrap());

        let response = "The add function is at src/lib.rs:3.";
        let result = validator.validate(response);

        assert!(result.is_valid());
        assert_eq!(result.valid_count(), 1);
    }

    #[test]
    fn test_validate_nonexistent_file() {
        let temp_dir = create_test_repo();
        let validator = StructuredOutputValidator::new(temp_dir.path().to_str().unwrap());

        let response = "Check src/nonexistent.rs:1 for details.";
        let result = validator.validate(response);

        assert!(!result.is_valid());
        assert!(result.has_failures());
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_validate_line_out_of_range() {
        let temp_dir = create_test_repo();
        let validator = StructuredOutputValidator::new(temp_dir.path().to_str().unwrap());

        // lib.rs has 5 lines; line 999 is out of range
        let response = "See src/lib.rs:999 for details.";
        let result = validator.validate(response);

        assert!(!result.is_valid());
        assert!(result.has_failures());
    }

    #[test]
    fn test_validate_no_citations() {
        let temp_dir = create_test_repo();
        let validator = StructuredOutputValidator::new(temp_dir.path().to_str().unwrap());

        let response = "I don't know the answer to that question.";
        let result = validator.validate(response);

        assert!(!result.is_valid());
        assert!(result.citations.is_empty());
    }

    #[test]
    fn test_validate_mixed_valid_invalid() {
        let temp_dir = create_test_repo();
        let validator = StructuredOutputValidator::new(temp_dir.path().to_str().unwrap());

        let response = "src/lib.rs:3 is valid, but src/nonexistent.rs:1 is not.";
        let result = validator.validate(response);

        assert!(!result.is_valid());
        assert!(result.has_failures());
        assert_eq!(result.valid_count(), 1);
        assert_eq!(result.validation_results.len(), 2);
    }

    #[test]
    fn test_validate_doc_citation() {
        let temp_dir = create_test_repo();
        let validator = StructuredOutputValidator::new(temp_dir.path().to_str().unwrap());

        let response = "See docs/getting-started.md:3 for installation info.";
        let result = validator.validate(response);

        assert!(result.is_valid());
        assert_eq!(result.valid_count(), 1);
    }

    #[test]
    fn test_validation_result_display() {
        let c = Citation {
            file_path: "src/lib.rs".to_string(),
            line_start: 42,
            line_end: 42,
        };
        assert_eq!(c.display(), "src/lib.rs:42");

        let c2 = Citation {
            file_path: "src/lib.rs".to_string(),
            line_start: 10,
            line_end: 20,
        };
        assert_eq!(c2.display(), "src/lib.rs:10-20");
    }

    #[test]
    fn test_validate_citation_range() {
        let temp_dir = create_test_repo();
        let validator = StructuredOutputValidator::new(temp_dir.path().to_str().unwrap());

        // lib.rs:2-5 should be valid (file has 5 lines, 2-5 is in range)
        let response = "See src/lib.rs:2-5 for the add function.";
        let result = validator.validate(response);

        assert!(result.is_valid());
        assert_eq!(result.citations[0].line_start, 2);
        assert_eq!(result.citations[0].line_end, 5);
    }

    #[test]
    fn test_validate_citation_range_out_of_bounds() {
        let temp_dir = create_test_repo();
        let validator = StructuredOutputValidator::new(temp_dir.path().to_str().unwrap());

        // lib.rs has 5 lines, so 2-100 is out of range
        let response = "See src/lib.rs:2-100.";
        let result = validator.validate(response);

        assert!(!result.is_valid());
        assert!(result.has_failures());
    }

    #[test]
    fn test_citation_validation_status() {
        let temp_dir = create_test_repo();
        let validator = StructuredOutputValidator::new(temp_dir.path().to_str().unwrap());

        // First, a valid citation
        let response1 = "src/lib.rs:1";
        let result1 = validator.validate(response1);
        match &result1.validation_results[0] {
            CitationValidation::Valid(_) => {}
            other => panic!("Expected Valid, got {:?}", other),
        }
        assert!(result1.validation_results[0].is_valid());
        assert!(result1.validation_results[0].clone().into_valid().is_some());

        // Now an invalid file
        let response2 = "src/nonexistent.rs:1";
        let result2 = validator.validate(response2);
        match &result2.validation_results[0] {
            CitationValidation::FileNotFound { .. } => {}
            other => panic!("Expected FileNotFound, got {:?}", other),
        }
        assert!(!result2.validation_results[0].is_valid());
    }
}
