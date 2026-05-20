//! Source code search for implementation questions.
//!
//! This module provides functionality to search the repository source code
//! for implementation details when a question appears to be about code internals.
//!
//! Search scope:
//! - All code files in repository (no language exclusion)
//! - Filenames, function/struct names, comments, docstrings, logic comments
//!
//! Code search triggers include keywords like:
//! - 'how does', 'what function', 'which module', 'internals', 'implementation'
//! - 'source code', 'can you walk me through', 'flow of', 'under the hood'

use anyhow::Result;
use glob::glob;
use std::fs;
use std::path::Path;

/// A match found during code search.
#[derive(Debug, Clone)]
pub struct CodeMatch {
    /// Path to the file containing the match.
    pub file_path: String,
    /// Line number of the match (1-indexed).
    pub line_number: usize,
    /// The matched text (line content or matched pattern).
    pub line_content: String,
    /// Type of match (function, struct, comment, filename, etc.)
    pub match_type: MatchType,
    /// Score indicating relevance (higher = more relevant).
    pub relevance_score: f32,
}

impl CodeMatch {
    /// Creates a new CodeMatch with the given properties.
    pub fn new(
        file_path: impl Into<String>,
        line_number: usize,
        line_content: impl Into<String>,
        match_type: MatchType,
    ) -> Self {
        let relevance_score = match_type.base_score();
        Self {
            file_path: file_path.into(),
            line_number,
            line_content: line_content.into(),
            match_type,
            relevance_score,
        }
    }

    /// Returns the file:line citation for this match.
    pub fn citation(&self) -> String {
        format!("{}:{}", self.file_path, self.line_number)
    }
}

/// The type of code match found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchType {
    /// Function definition (fn name)
    Function,
    /// Struct definition
    Struct,
    /// Module/namespace
    Module,
    /// Class definition
    Class,
    /// Comment line
    Comment,
    /// Docstring comment
    Docstring,
    /// Filename match
    Filename,
    /// Other match
    Other,
}

impl MatchType {
    /// Returns the base relevance score for this match type.
    /// Higher scores indicate more likely to be what a user is looking for.
    fn base_score(&self) -> f32 {
        match self {
            MatchType::Function => 10.0,
            MatchType::Struct => 9.0,
            MatchType::Class => 9.0,
            MatchType::Module => 8.0,
            MatchType::Docstring => 7.0,
            MatchType::Filename => 6.0,
            MatchType::Comment => 5.0,
            MatchType::Other => 1.0,
        }
    }

    /// Returns a human-readable label for this match type.
    pub fn label(&self) -> &'static str {
        match self {
            MatchType::Function => "function",
            MatchType::Struct => "struct",
            MatchType::Class => "class",
            MatchType::Module => "module",
            MatchType::Docstring => "docstring",
            MatchType::Filename => "filename",
            MatchType::Comment => "comment",
            MatchType::Other => "code",
        }
    }
}

/// Search configuration for code search.
#[derive(Debug, Clone)]
pub struct CodeSearchConfig {
    /// Additional file patterns to include (beyond default code patterns).
    pub extra_patterns: Vec<String>,
    /// Patterns to exclude from search.
    pub exclude_patterns: Vec<String>,
    /// Maximum number of results to return.
    pub max_results: usize,
    /// Whether to search file contents (vs just filenames).
    pub search_content: bool,
    /// Maximum file size to search (in bytes).
    pub max_file_size: usize,
}

impl Default for CodeSearchConfig {
    fn default() -> Self {
        Self {
            extra_patterns: Vec::new(),
            exclude_patterns: vec![
                "**/target/**".to_string(),
                "**/.git/**".to_string(),
                "**/node_modules/**".to_string(),
                "**/dist/**".to_string(),
                "**/build/**".to_string(),
                "**/*.min.js".to_string(),
                "**/*.map".to_string(),
            ],
            max_results: 50,
            search_content: true,
            max_file_size: 1_000_000, // 1MB
        }
    }
}

/// Searches the repository for code matching the given query.
/// Returns matches sorted by relevance.
pub fn search_code(query: &str, repo_root: &Path) -> Result<Vec<CodeMatch>> {
    search_code_with_config(query, repo_root, &CodeSearchConfig::default())
}

/// Searches the repository for code matching the given query with custom config.
/// Returns matches sorted by relevance.
pub fn search_code_with_config(
    query: &str,
    repo_root: &Path,
    config: &CodeSearchConfig,
) -> Result<Vec<CodeMatch>> {
    let query_lower = query.to_lowercase();
    let mut matches = Vec::new();

    // Build the exclusion patterns
    let exclude_paths: Vec<glob::Pattern> = config
        .exclude_patterns
        .iter()
        .filter_map(|p| glob::Pattern::new(p).ok())
        .collect();

    // Search all code files in the repository
    let search_patterns = vec!["**/*"];

    for pattern in search_patterns {
        for entry in glob(&format!("{}/{}", repo_root.display(), pattern))? {
            let path = match entry {
                Ok(p) => p,
                Err(_) => continue,
            };

            // Skip excluded paths
            let path_str = path.to_string_lossy();
            let should_exclude = exclude_paths.iter().any(|ex| ex.matches(&path_str));
            if should_exclude {
                continue;
            }

            // Get file metadata to check size
            let metadata = match fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };

            // Skip non-files or files that are too large
            if !metadata.is_file() || metadata.len() > config.max_file_size as u64 {
                continue;
            }

            // Skip files without extensions (likely not code files)
            // unless extra_patterns explicitly includes them
            let extension = path.extension().and_then(|e| e.to_str());
            let is_code_file = extension.is_some_and(|ext| {
                let ext_lower = ext.to_lowercase();
                matches_code_extension(&ext_lower)
            });

            // If not a code file and not in extra patterns, skip
            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            let matches_extra = config
                .extra_patterns
                .iter()
                .any(|p| glob::Pattern::new(p).is_ok_and(|g| g.matches(filename)));

            if !is_code_file && !matches_extra {
                continue;
            }

            // Search filenames first (fast)
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.to_lowercase().contains(&query_lower) {
                    matches.push(CodeMatch::new(
                        path.to_string_lossy().to_string(),
                        1,
                        format!("filename: {}", name),
                        MatchType::Filename,
                    ));
                }
            }

            // Search file contents if enabled
            if config.search_content {
                if let Ok(content) = fs::read_to_string(&path) {
                    let file_path_for_errors = path.to_string_lossy();
                    let lines: Vec<(usize, &str)> = content
                        .lines()
                        .enumerate()
                        .map(|(i, l)| (i + 1, l))
                        .collect();

                    // Search for the query in line content
                    for (line_num, line) in &lines {
                        let line_lower = line.to_lowercase();
                        if line_lower.contains(&query_lower) {
                            let match_type = detect_match_type(line, &query_lower);
                            let mut m = CodeMatch::new(
                                file_path_for_errors.to_string(),
                                *line_num,
                                *line,
                                match_type,
                            );

                            // Boost score for exact matches
                            if line_lower.contains(&format!(" {} ", query_lower))
                                || line_lower.starts_with(&format!("{} ", query_lower))
                                || line_lower.ends_with(&format!(" {}", query_lower))
                            {
                                m.relevance_score *= 2.0;
                            }

                            matches.push(m);
                        }
                    }
                }
            }
        }
    }

    // Sort by relevance score (descending)
    matches.sort_by(|a, b| {
        b.relevance_score
            .partial_cmp(&a.relevance_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Limit results
    matches.truncate(config.max_results);

    Ok(matches)
}

/// Determines if a file extension is a recognized code extension.
fn matches_code_extension(ext: &str) -> bool {
    matches!(
        ext,
        "rs" | "py"
            | "js"
            | "ts"
            | "jsx"
            | "tsx"
            | "go"
            | "java"
            | "c"
            | "cpp"
            | "h"
            | "hpp"
            | "cs"
            | "rb"
            | "php"
            | "swift"
            | "kt"
            | "scala"
            | "clj"
            | "ex"
            | "exl"
            | "vue"
            | "svelte"
            | "html"
            | "css"
            | "scss"
            | "json"
            | "yaml"
            | "yml"
            | "toml"
            | "xml"
            | "sql"
            | "sh"
            | "bash"
            | "zsh"
            | "ps1"
            | "r"
            | "m"
            | "lua"
            | "pl"
            | "pm"
            | "hs"
            | "fs"
            | "ml"
            | "proto"
            | "graphql"
            | "cl"
            | "lisp"
            | "scm"
            | "asm"
            | "s"
            | "S"
            | "f"
            | "for"
            | "v"
            | "vhd"
            | "tcl"
            | "tk"
            | "el"
            | "sc"
            | "juttle"
    )
}

/// Detects the type of match based on the line content.
fn detect_match_type(line: &str, _query: &str) -> MatchType {
    let line_trimmed = line.trim();

    // Function patterns
    if line_trimmed.starts_with("fn ")
        || line_trimmed.starts_with("func ")
        || line_trimmed.starts_with("function ")
        || line_trimmed.starts_with("def ")
        || line_trimmed.starts_with("pub fn ")
        || line_trimmed.starts_with("async fn ")
        || line_trimmed.starts_with("pub async fn ")
    {
        return MatchType::Function;
    }

    // Struct patterns
    if line_trimmed.starts_with("struct ")
        || line_trimmed.starts_with("pub struct ")
        || line_trimmed.starts_with("class ")
        || line_trimmed.starts_with("pub class ")
        || line_trimmed.starts_with("type ")
        || line_trimmed.starts_with("enum ")
        || line_trimmed.starts_with("pub enum ")
    {
        return MatchType::Struct;
    }

    // Module patterns
    if line_trimmed.starts_with("mod ")
        || line_trimmed.starts_with("pub mod ")
        || line_trimmed.starts_with("namespace ")
        || line_trimmed.starts_with("package ")
    {
        return MatchType::Module;
    }

    // Docstring patterns
    if line_trimmed.starts_with("///")
        || line_trimmed.starts_with("//!")
        || line_trimmed.starts_with("/**")
        || line_trimmed.starts_with("\"\"\"")
        || line_trimmed.starts_with("'''")
        || line_trimmed.starts_with("#![doc")
    {
        return MatchType::Docstring;
    }

    // Comment patterns
    if line_trimmed.starts_with("//")
        || line_trimmed.starts_with("#")
        || line_trimmed.starts_with("/*")
        || line_trimmed.starts_with("* ")
    {
        return MatchType::Comment;
    }

    MatchType::Other
}

/// Checks if a question text contains code search trigger keywords.
/// Returns true if the question appears to be about implementation/code internals.
pub fn is_implementation_question(question: &str) -> bool {
    use crate::llm::prompts::CODE_SEARCH_TRIGGERS;

    let question_lower = question.to_lowercase();

    CODE_SEARCH_TRIGGERS
        .iter()
        .any(|trigger| question_lower.contains(&trigger.to_lowercase()))
}

/// Generates a plain-language explanation of code matches.
/// Returns a formatted string suitable for posting as a GitHub comment.
pub fn format_code_explanation(
    _question: &str,
    matches: &[CodeMatch],
    _project_context: &str,
) -> String {
    if matches.is_empty() {
        return "I searched the source code for relevant implementation details \
             but couldn't find a clear match for your question. \
             This may indicate a documentation gap worth filing a bead for."
                .to_string();
    }

    let mut explanation = String::new();

    explanation.push_str("Hi! Thanks for this question about implementation. \
         I searched the source code and found relevant details:\n\n");

    // Group matches by file
    let mut by_file: std::collections::HashMap<&str, Vec<&CodeMatch>> =
        std::collections::HashMap::new();

    for m in matches {
        by_file.entry(&m.file_path).or_default().push(m);
    }

    // Format each file's matches
    for (file, file_matches) in by_file {
        explanation.push_str(&format!("### `{file}`\n\n"));

        for m in file_matches {
            let type_label = m.match_type.label();
            explanation.push_str(&format!(
                "**{}** at line {}:\n```\n{}\n```\n",
                type_label,
                m.line_number,
                m.line_content.trim()
            ));
        }

        explanation.push('\n');
    }

    // Add citation summary
    let citations: Vec<String> = matches.iter().map(|m| m.citation()).collect();
    let unique_citations: Vec<&str> = citations
        .iter()
        .map(|s| s.as_str())
        .collect::<std::collections::HashSet<_>>()
        .iter()
        .copied()
        .collect();

    explanation.push_str(&format!(
        "**Summary**: Found {} relevant code {} across {} file(s). \
         Key locations: {}",
        matches.len(),
        if matches.len() == 1 {
            "section"
        } else {
            "sections"
        },
        unique_citations.len(),
        unique_citations.join(", ")
    ));

    explanation
}

/// Validates that the code citations in a response actually exist in the codebase.
/// Returns the number of valid citations found.
pub fn validate_citations(response: &str, repo_root: &Path) -> usize {
    let citation_pattern = regex::Regex::new(r"(\S+:\d+(?:-\d+)?)").ok();

    let Some(re) = citation_pattern else {
        return 0;
    };

    let mut valid_count = 0;

    for cap in re.captures_iter(response) {
        if let Some(citation) = cap.get(1) {
            let citation_str = citation.as_str();
            // Parse citation (format: "path/to/file.rs:123" or "path/to/file.rs:123-145")
            if let Some(colon_pos) = citation_str.rfind(':') {
                let file_path = &citation_str[..colon_pos];
                let line_str = &citation_str[colon_pos + 1..];

                // Try to read the file and check if the line exists
                let full_path = repo_root.join(file_path);
                if full_path.exists() {
                    if let Ok(content) = fs::read_to_string(&full_path) {
                        // Check if it's a range or single line
                        if line_str.contains('-') {
                            let parts: Vec<&str> = line_str.split('-').collect();
                            if parts.len() == 2 {
                                let start: usize = parts[0].parse().unwrap_or(0);
                                let end: usize = parts[1].parse().unwrap_or(0);
                                let line_count = content.lines().count();
                                if start > 0 && end >= start && end <= line_count {
                                    valid_count += 1;
                                }
                            }
                        } else if let Ok(line_num) = line_str.parse::<usize>() {
                            let line_count = content.lines().count();
                            if line_num > 0 && line_num <= line_count {
                                valid_count += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    valid_count
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_repo() -> TempDir {
        let temp_dir = TempDir::new().unwrap();

        // Create directories first
        fs::create_dir_all(temp_dir.path().join("src")).unwrap();

        // Create some test code files
        fs::write(
            temp_dir.path().join("src/lib.rs"),
            r#"//! Test library.
//
//! This is a test module for testing code search.

/// Adds two numbers together.
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// Subtracts two numbers.
pub fn subtract(a: i32, b: i32) -> i32 {
    a - b
}

struct Internal {
    value: i32,
}
"#,
        )
        .unwrap();

        fs::write(
            temp_dir.path().join("src/main.rs"),
            r#"fn main() {
    println!("Hello, world!");
}
"#,
        )
        .unwrap();

        fs::write(
            temp_dir.path().join("README.md"),
            r#"# Test Project

This is a test project for code search.
"#,
        )
        .unwrap();

        temp_dir
    }

    #[test]
    fn test_search_finds_function() {
        let temp_dir = create_test_repo();
        let matches = search_code("add", temp_dir.path()).unwrap();

        assert!(!matches.is_empty());
        assert!(matches.iter().any(|m| m.line_content.contains("add")));
    }

    #[test]
    fn test_search_finds_docstring() {
        let temp_dir = create_test_repo();
        let matches = search_code("Adds", temp_dir.path()).unwrap();

        assert!(!matches.is_empty());
    }

    #[test]
    fn test_search_respects_max_results() {
        let temp_dir = create_test_repo();
        let mut config = CodeSearchConfig::default();
        config.max_results = 2;

        let matches = search_code_with_config("pub", temp_dir.path(), &config).unwrap();
        assert!(matches.len() <= 2);
    }

    #[test]
    fn test_is_implementation_question() {
        assert!(is_implementation_question("How does the router work?"));
        assert!(is_implementation_question("What function processes this?"));
        assert!(is_implementation_question("Tell me about the internals"));
        assert!(is_implementation_question(
            "Can you walk me through the flow?"
        ));
        assert!(is_implementation_question(
            "What's the implementation strategy?"
        ));
        assert!(!is_implementation_question("How do I install this?"));
        assert!(!is_implementation_question("What's the weather?"));
    }

    #[test]
    fn test_detect_match_type() {
        assert_eq!(
            detect_match_type("pub fn foo()", "foo"),
            MatchType::Function
        );
        assert_eq!(detect_match_type("fn bar()", "bar"), MatchType::Function);
        assert_eq!(detect_match_type("struct Foo {}", "foo"), MatchType::Struct);
        assert_eq!(
            detect_match_type("/// This is a docstring", "docstring"),
            MatchType::Docstring
        );
        assert_eq!(
            detect_match_type("// This is a comment", "comment"),
            MatchType::Comment
        );
    }

    #[test]
    fn test_format_code_explanation() {
        let temp_dir = create_test_repo();
        let matches = search_code("add", temp_dir.path()).unwrap();
        let explanation = format_code_explanation("How does add work?", &matches, "Test project");

        assert!(explanation.contains("src/lib.rs"));
        assert!(explanation.contains("function"));
    }

    #[test]
    fn test_validate_citations() {
        let temp_dir = create_test_repo();
        let response = "See src/lib.rs:5 and src/main.rs:1 for details";
        let valid_count = validate_citations(response, temp_dir.path());

        // Both citations should be valid
        assert_eq!(valid_count, 2);
    }

    #[test]
    fn test_validate_citations_invalid() {
        let temp_dir = create_test_repo();
        let response = "See src/nonexistent.rs:999 for details";
        let valid_count = validate_citations(response, temp_dir.path());

        assert_eq!(valid_count, 0);
    }

    #[test]
    fn test_code_match_citation() {
        let m = CodeMatch::new("src/main.rs", 42, "fn main()", MatchType::Function);
        assert_eq!(m.citation(), "src/main.rs:42");
    }

    #[test]
    fn test_matches_code_extension() {
        assert!(matches_code_extension("rs"));
        assert!(matches_code_extension("py"));
        assert!(matches_code_extension("js"));
        assert!(matches_code_extension("ts"));
        assert!(!matches_code_extension("txt"));
        assert!(!matches_code_extension("md"));
    }
}
