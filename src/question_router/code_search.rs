//! Code search for question routing.
//!
//! Searches source code for implementation-level questions. Used when a question
//! asks about code-level or internal details (how does X work, what function
//! handles Y, walk me through the flow of Z).
//!
//! Plan: plans/question-routing-plan.md §Search scope for code

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

/// Code search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSearchResult {
    /// File path where the match was found.
    pub file_path: String,
    /// Line number of the match.
    pub line_number: u32,
    /// Function or struct name containing the match.
    pub symbol_name: Option<String>,
    /// Snippet of relevant code.
    pub snippet: String,
    /// Relevance score (0.0 to 1.0).
    pub relevance: f32,
    /// Context lines for surrounding code.
    pub context_lines: Vec<(u32, String)>,
}

/// Code searcher.
#[derive(Debug, Clone)]
pub struct CodeSearcher {
    /// Base path for source code directory.
    source_path: String,
    /// Cache of loaded source files.
    files: HashMap<String, SourceFile>,
    /// Keywords that trigger code search instead of just documentation search.
    code_search_keywords: Vec<String>,
}

impl CodeSearcher {
    /// Create a new code searcher with the given source path.
    pub fn new(source_path: impl Into<String>) -> Self {
        Self {
            source_path: source_path.into(),
            files: HashMap::new(),
            code_search_keywords: default_code_keywords(),
        }
    }

    /// Create a searcher for the standard src directory.
    pub fn standard() -> Self {
        Self::new("src")
    }

    /// Check if a query should trigger code search.
    pub fn should_search_code(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();
        self.code_search_keywords
            .iter()
            .any(|kw| query_lower.contains(&kw.to_lowercase()))
    }

    /// Load all source files from the source directory.
    pub fn load_source_files(&mut self) -> Result<()> {
        let path = Path::new(&self.source_path);
        if !path.exists() {
            tracing::warn!("Source directory does not exist: {}", self.source_path);
            return Ok(());
        }

        for entry in WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let entry_path = entry.path();
            if entry_path.is_file() {
                // Skip certain file types that are not searchable code
                if let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) {
                    let skip_extensions = [
                        "lock", "json", "toml", "yaml", "yml", "md", "txt", "html", "css", "svg",
                        "png", "jpg",
                    ];

                    if !skip_extensions.contains(&ext) {
                        if let Some(path_str) = entry_path.to_str() {
                            match std::fs::read_to_string(entry_path) {
                                Ok(content) => {
                                    let symbol_map = extract_symbols(&content);
                                    self.files.insert(
                                        path_str.to_string(),
                                        SourceFile {
                                            path: path_str.to_string(),
                                            language: detect_language(ext),
                                            content,
                                            symbols: symbol_map,
                                        },
                                    );
                                }
                                Err(e) => {
                                    tracing::debug!("Could not read file {}: {}", path_str, e);
                                }
                            }
                        }
                    }
                }
            }
        }

        tracing::info!(
            "Loaded {} source files from {}",
            self.files.len(),
            self.source_path
        );

        Ok(())
    }

    /// Search source code for relevant code matching the query.
    pub fn search(&self, query: &str) -> Vec<CodeSearchResult> {
        if self.files.is_empty() {
            tracing::warn!("No source files loaded for search");
            return Vec::new();
        }

        let keywords = extract_code_keywords(query);
        let mut results: Vec<CodeSearchResult> = Vec::new();

        for (_path, file) in &self.files {
            // Search file content for matches
            let content_lower = file.content.to_lowercase();
            let query_lower = query.to_lowercase();

            // Check if query matches this file's content or symbols
            let mut found_something = false;
            let mut best_relevance: f32 = 0.0;

            // Check symbol matches first (higher relevance)
            for (symbol_name, symbol_line) in &file.symbols {
                let symbol_lower = symbol_name.to_lowercase();
                for keyword in &keywords {
                    if symbol_lower.contains(&keyword.to_lowercase())
                        || content_lower.contains(&keyword.to_lowercase())
                    {
                        found_something = true;
                        best_relevance = best_relevance.max(0.7); // Symbol matches are good
                        break;
                    }
                }
            }

            // Check for query match in content
            if content_lower.contains(&query_lower) {
                found_something = true;
                best_relevance = best_relevance.max(0.8);
            }

            // General keyword matching
            if !found_something {
                let keyword_matches = keywords
                    .iter()
                    .filter(|kw| content_lower.contains(&kw.to_lowercase()))
                    .count();
                if keyword_matches > 0 {
                    found_something = true;
                    best_relevance =
                        (keyword_matches as f32 / keywords.len() as f32 * 0.6).min(0.6);
                }
            }

            if found_something {
                // Extract match locations
                for (line_num, line) in file.content.lines().enumerate() {
                    let line_lower = line.to_lowercase();
                    let mut line_relevance = best_relevance;

                    // Boost relevance if line contains query or keywords
                    if line_lower.contains(&query_lower) {
                        line_relevance = 0.9;
                    } else {
                        for keyword in &keywords {
                            if line_lower.contains(&keyword.to_lowercase()) {
                                line_relevance = line_relevance.max(0.5);
                                break;
                            }
                        }
                    }

                    if line_relevance > 0.3 {
                        let symbol = find_symbol_for_line(&file.symbols, line_num as u32);
                        let context = extract_context(&file.content, line_num as u32, 2);

                        results.push(CodeSearchResult {
                            file_path: file.path.clone(),
                            line_number: line_num as u32 + 1,
                            symbol_name: symbol,
                            snippet: line.trim().to_string(),
                            relevance: line_relevance,
                            context_lines: context,
                        });
                    }
                }
            }
        }

        // Sort by relevance descending
        results.sort_by(|a, b| {
            b.relevance
                .partial_cmp(&a.relevance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Deduplicate by file and reduce to best matches per file
        let mut seen_files: std::collections::HashSet<String> = std::collections::HashSet::new();
        let unique_results: Vec<CodeSearchResult> = results
            .into_iter()
            .filter(|r| {
                let key = format!("{}:{}", r.file_path, r.line_number / 10);
                if seen_files.contains(&key) {
                    false
                } else {
                    seen_files.insert(key);
                    true
                }
            })
            .collect();

        unique_results
    }

    /// Search and return only the best matches grouped by file.
    pub fn find_relevant_files(&self, query: &str) -> Vec<CodeSearchResult> {
        let results = self.search(query);
        let mut file_map: HashMap<String, CodeSearchResult> = HashMap::new();

        for result in results {
            let key = result.file_path.clone();
            if !file_map.contains_key(&key) {
                file_map.insert(key, result);
            }
        }

        let mut sorted: Vec<_> = file_map.into_values().collect();
        sorted.sort_by(|a, b| {
            b.relevance
                .partial_cmp(&a.relevance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted
    }

    /// Find implementation details for a specific function or symbol.
    pub fn find_symbol_implementation(&self, symbol_name: &str) -> Vec<CodeSearchResult> {
        let mut results = Vec::new();

        for (_path, file) in &self.files {
            if let Some((name, line_num)) = file
                .symbols
                .iter()
                .find(|(n, _)| n.to_lowercase() == symbol_name.to_lowercase())
            {
                let context = extract_context(&file.content, *line_num, 5);
                let lines: Vec<&str> = file.content.lines().collect();
                let line_content = lines
                    .get(*line_num as usize - 1)
                    .map(|l| l.to_string())
                    .unwrap_or_default();

                results.push(CodeSearchResult {
                    file_path: file.path.clone(),
                    line_number: *line_num,
                    symbol_name: Some(symbol_name.to_string()),
                    snippet: line_content,
                    relevance: 1.0,
                    context_lines: context,
                });
            }
        }

        results
    }

    /// Get the module structure for a file.
    pub fn get_module_structure(&self, file_path: &str) -> Option<Vec<String>> {
        self.files
            .get(file_path)
            .map(|f| f.symbols.iter().map(|(name, _)| name.clone()).collect())
    }
}

/// Content of a source file with extracted symbols.
#[derive(Debug, Clone)]
struct SourceFile {
    /// Full file path.
    path: String,
    /// Programming language.
    language: String,
    /// Full file content.
    content: String,
    /// Extracted function/struct names with line numbers.
    symbols: Vec<(String, u32)>,
}

/// Default keywords that trigger code search.
fn default_code_keywords() -> Vec<String> {
    vec![
        "how does".to_string(),
        "what function".to_string(),
        "what method".to_string(),
        "which module".to_string(),
        "internals".to_string(),
        "implementation".to_string(),
        "source code".to_string(),
        "can you walk me through".to_string(),
        "walk me through".to_string(),
        "flow of".to_string(),
        "under the hood".to_string(),
        "how does it work".to_string(),
        "how is".to_string(),
        "what does".to_string(),
        "where is".to_string(),
        "which function".to_string(),
        "which class".to_string(),
        "show me the code".to_string(),
    ]
}

/// Extract keywords from a query for code matching.
fn extract_code_keywords(query: &str) -> Vec<String> {
    let lower = query.to_lowercase();

    // Remove common question words that don't help code search
    let stop_words = [
        "how", "does", "what", "the", "a", "an", "is", "are", "can", "you", "do", "i", "me", "my",
        "we", "they", "it", "that", "this", "be", "to", "of", "and", "or", "in", "on", "at", "for",
        "with", "from", "by", "as", "not", "if", "but", "so", "just", "please", "thanks", "thank",
        "hi", "hello", "hey", "would", "could", "should", "want", "like", "need", "has", "have",
        "had", "was", "were", "been", "being", "get", "got", "why", "where", "which", "walk", "me",
        "through", "code", "source", "work", "works",
    ];

    lower
        .split(|c: char| !c.is_alphanumeric() && c != '-')
        .filter(|w| w.len() > 2)
        .filter(|w| !stop_words.contains(&w))
        .map(|w| w.to_string())
        .collect()
}

/// Extract function and struct names from source code.
fn extract_symbols(content: &str) -> Vec<(String, u32)> {
    let mut symbols = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with("//") || trimmed.starts_with("#") || trimmed.starts_with("/*") {
            continue;
        }

        // Rust: fn, struct, enum, trait, impl, mod, pub
        if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") {
            if let Some(name) = extract_function_name(trimmed) {
                symbols.push((name, i as u32 + 1));
            }
        }
        if trimmed.starts_with("struct ") || trimmed.starts_with("pub struct ") {
            if let Some(name) = extract_struct_name(trimmed) {
                symbols.push((name, i as u32 + 1));
            }
        }
        if trimmed.starts_with("enum ") || trimmed.starts_with("pub enum ") {
            if let Some(name) = extract_struct_name(trimmed) {
                symbols.push((name, i as u32 + 1));
            }
        }
        if trimmed.starts_with("trait ") || trimmed.starts_with("pub trait ") {
            if let Some(name) = extract_struct_name(trimmed) {
                symbols.push((name, i as u32 + 1));
            }
        }
        if trimmed.starts_with("impl ") || trimmed.starts_with("pub impl ") {
            symbols.push((format!("impl block at line {}", i + 1), i as u32 + 1));
        }
        if trimmed.starts_with("mod ") || trimmed.starts_with("pub mod ") {
            if let Some(name) = extract_struct_name(trimmed) {
                symbols.push((format!("mod {}", name), i as u32 + 1));
            }
        }

        // TypeScript/JavaScript: function, const, class, interface
        if trimmed.starts_with("function ") {
            if let Some(name) = extract_function_name(trimmed) {
                symbols.push((name, i as u32 + 1));
            }
        }
        if trimmed.starts_with("const ") && trimmed.contains("=>") {
            if let Some(name) = extract_struct_name(trimmed) {
                symbols.push((name, i as u32 + 1));
            }
        }
        if trimmed.starts_with("class ") || trimmed.starts_with("export class ") {
            if let Some(name) = extract_struct_name(trimmed) {
                symbols.push((name, i as u32 + 1));
            }
        }
        if trimmed.starts_with("interface ") {
            if let Some(name) = extract_struct_name(trimmed) {
                symbols.push((name, i as u32 + 1));
            }
        }

        // Python: def, class
        if trimmed.starts_with("def ") {
            if let Some(name) = extract_function_name(trimmed) {
                symbols.push((name, i as u32 + 1));
            }
        }
    }

    symbols
}

/// Extract function name from a function definition line.
fn extract_function_name(line: &str) -> Option<String> {
    // Remove modifiers like pub, async, unsafe
    let cleaned = line
        .trim()
        .replace("pub ", "")
        .replace("async ", "")
        .replace("unsafe ", "")
        .replace("fn ", "")
        .replace("function ", "")
        .replace("def ", "");

    // Get the identifier
    if let Some(name_end) = cleaned.find(|c: char| !c.is_alphanumeric() && c != '_' && c != '<') {
        let name = cleaned[..name_end].trim();
        if !name.is_empty()
            && name
                .chars()
                .next()
                .map(|c| c.is_lowercase())
                .unwrap_or(false)
        {
            return Some(name.to_string());
        }
    } else {
        let name = cleaned.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }

    None
}

/// Extract struct/type name from a definition line.
fn extract_struct_name(line: &str) -> Option<String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 2 {
        let name = parts[1];
        // Skip keywords that aren't the name
        if name == "struct"
            || name == "enum"
            || name == "trait"
            || name == "class"
            || name == "interface"
        {
            if parts.len() >= 3 {
                return Some(parts[2].to_string());
            }
            return None;
        }
        // Don't return generic names
        if name != "type" && name != "impl" && !name.starts_with("<") {
            return Some(name.to_string());
        }
    }
    None
}

/// Find the symbol name for a given line number.
fn find_symbol_for_line(symbols: &[(String, u32)], line: u32) -> Option<String> {
    symbols
        .iter()
        .filter(|(_, sym_line)| *sym_line <= line)
        .max_by_key(|(_, sym_line)| *sym_line)
        .map(|(name, _)| name.clone())
}

/// Detect programming language from file extension.
fn detect_language(extension: &str) -> String {
    match extension {
        "rs" => "Rust".to_string(),
        "ts" | "tsx" => "TypeScript".to_string(),
        "js" | "jsx" => "JavaScript".to_string(),
        "py" => "Python".to_string(),
        "go" => "Go".to_string(),
        "java" => "Java".to_string(),
        "rb" => "Ruby".to_string(),
        "cpp" | "cc" | "cxx" => "C++".to_string(),
        "c" => "C".to_string(),
        "cs" => "C#".to_string(),
        "php" => "PHP".to_string(),
        "swift" => "Swift".to_string(),
        "kt" => "Kotlin".to_string(),
        "scala" => "Scala".to_string(),
        "md" => "Markdown".to_string(),
        _ => extension.to_string(),
    }
}

/// Extract context lines around a given line number.
fn extract_context(content: &str, line_number: u32, padding: usize) -> Vec<(u32, String)> {
    let lines: Vec<&str> = content.lines().collect();
    let start = (line_number as usize).saturating_sub(padding);
    let end = ((line_number as usize) + padding).min(lines.len());

    lines[start..end]
        .iter()
        .enumerate()
        .map(|(i, line)| (start as u32 + i as u32 + 1, line.to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_search_keywords() {
        let searcher = CodeSearcher::standard();

        assert!(searcher.should_search_code("How does the triage engine work?"));
        assert!(searcher.should_search_code("What function handles X?"));
        assert!(searcher.should_search_code("Which module contains Y?"));
        assert!(searcher.should_search_code("Walk me through the flow"));
        assert!(searcher.should_search_code("What are the internals?"));

        assert!(!searcher.should_search_code("How do I install this?"));
        assert!(!searcher.should_search_code("What is the price?"));
    }

    #[test]
    fn test_code_keyword_extraction() {
        let keywords = extract_code_keywords("How does the triage engine process issues?");
        assert!(keywords.contains(&"triage".to_string()));
        assert!(keywords.contains(&"engine".to_string()));
        assert!(keywords.contains(&"process".to_string()));
        assert!(keywords.contains(&"issues".to_string()));
        assert!(!keywords.contains(&"how".to_string()));
        assert!(!keywords.contains(&"does".to_string()));
    }

    #[test]
    fn test_language_detection() {
        assert_eq!(detect_language("rs"), "Rust");
        assert_eq!(detect_language("ts"), "TypeScript");
        assert_eq!(detect_language("py"), "Python");
        assert_eq!(detect_language("js"), "JavaScript");
    }

    #[test]
    fn test_function_name_extraction() {
        assert_eq!(
            extract_function_name("fn process_issue"),
            Some("process_issue".to_string())
        );
        assert_eq!(
            extract_function_name("pub async fn handle_event"),
            Some("handle_event".to_string())
        );
        assert_eq!(
            extract_function_name("function myFunction"),
            Some("myFunction".to_string())
        );
        assert_eq!(
            extract_function_name("def calculate"),
            Some("calculate".to_string())
        );
    }

    #[test]
    fn test_struct_name_extraction() {
        assert_eq!(
            extract_struct_name("struct TriageEngine"),
            Some("TriageEngine".to_string())
        );
        assert_eq!(
            extract_struct_name("pub struct Processor"),
            Some("Processor".to_string())
        );
        assert_eq!(
            extract_struct_name("class MyClass"),
            Some("MyClass".to_string())
        );
    }

    #[test]
    fn test_code_searcher_empty() {
        let searcher = CodeSearcher::new("nonexistent");
        let results = searcher.search("test query");
        assert!(results.is_empty());
    }

    #[test]
    fn test_symbol_extraction_rust() {
        let content = r#"
// Test file
pub fn process_issue(issue: &Issue) -> Result<Vec<TriageAction>> {
    // Handle the issue
    Ok(vec![])
}

struct TriageEngine {
    // Fields
}

enum IssueType {
    Bug,
    Feature,
}
"#;

        let symbols = extract_symbols(content);
        assert!(symbols.iter().any(|(name, _)| name == "process_issue"));
        assert!(symbols.iter().any(|(name, _)| name == "TriageEngine"));
        assert!(symbols.iter().any(|(name, _)| name == "IssueType"));
    }

    #[test]
    fn test_relevant_files() {
        let searcher = CodeSearcher::new("nonexistent");
        let results = searcher.find_relevant_files("test");
        assert!(results.is_empty());
    }
}
