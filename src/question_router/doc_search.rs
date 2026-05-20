//! Documentation search for question routing.
//!
//! Searches the `docs/` directory for content relevant to question issues.
//! Provides keyword-based search over user-facing documentation.

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

/// Documentation search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocSearchResult {
    /// Path to the matching document.
    pub path: String,
    /// Section or heading that matches the question.
    pub section: Option<String>,
    /// Snippet of relevant content.
    pub snippet: String,
    /// Relevance score (0.0 to 1.0).
    pub relevance: f32,
    /// URL-friendly link path (relative to docs root).
    pub link_path: String,
}

/// Documentation searcher.
#[derive(Debug, Clone)]
pub struct DocSearcher {
    /// Base path for docs directory.
    docs_path: String,
    /// Cache of loaded documents for search.
    documents: HashMap<String, DocContent>,
}

impl DocSearcher {
    /// Create a new doc searcher with the given docs path.
    pub fn new(docs_path: impl Into<String>) -> Self {
        Self {
            docs_path: docs_path.into(),
            documents: HashMap::new(),
        }
    }

    /// Create a searcher for the standard docs directory.
    pub fn standard() -> Self {
        Self::new("docs")
    }

    /// Load all markdown documents from the docs directory.
    pub fn load_documents(&mut self) -> Result<()> {
        let path = Path::new(&self.docs_path);
        if !path.exists() {
            tracing::warn!("Docs directory does not exist: {}", self.docs_path);
            return Ok(());
        }

        for entry in WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() {
                let extension = path.extension().and_then(|e| e.to_str());
                if extension == Some("md") {
                    if let Some(path_str) = path.to_str() {
                        let content = std::fs::read_to_string(path)?;
                        let relative_path = path
                            .strip_prefix(path)
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|_| path_str.to_string());

                        // Extract title from frontmatter or first heading
                        let title = extract_title_from_content(&content).unwrap_or_else(|| {
                            path.file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("Untitled")
                                .to_string()
                        });

                        self.documents.insert(
                            relative_path.clone(),
                            DocContent {
                                path: relative_path,
                                title,
                                content,
                            },
                        );
                    }
                }
            }
        }

        tracing::info!(
            "Loaded {} documentation files from {}",
            self.documents.len(),
            self.docs_path
        );

        Ok(())
    }

    /// Search documents for relevant content matching the query.
    pub fn search(&self, query: &str) -> Vec<DocSearchResult> {
        if self.documents.is_empty() {
            tracing::warn!("No documents loaded for search");
            return Vec::new();
        }

        let keywords = extract_keywords(query);
        let mut results: Vec<DocSearchResult> = Vec::new();

        for (_path, doc) in &self.documents {
            let relevance = calculate_relevance(&keywords, &doc.content, query);

            if relevance > 0.0 {
                // Extract a relevant snippet
                let snippet = extract_relevant_snippet(&doc.content, &keywords, 200);
                let section = extract_section_heading(&doc.content, &keywords);

                results.push(DocSearchResult {
                    path: doc.path.clone(),
                    section,
                    snippet,
                    relevance,
                    link_path: doc.path.clone(),
                });
            }
        }

        // Sort by relevance descending
        results.sort_by(|a, b| {
            b.relevance
                .partial_cmp(&a.relevance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }

    /// Search documents and return only the best match.
    pub fn find_best_match(&self, query: &str) -> Option<DocSearchResult> {
        self.search(query).into_iter().next()
    }

    /// Check if any documents match the query.
    pub fn has_match(&self, query: &str) -> bool {
        self.search(query).into_iter().next().is_some()
    }
}

/// Content of a documentation file.
#[derive(Debug, Clone)]
struct DocContent {
    /// Relative path to the document.
    path: String,
    /// Document title.
    title: String,
    /// Full document content.
    content: String,
}

/// Extract keywords from a query string for matching.
fn extract_keywords(query: &str) -> Vec<String> {
    let lower = query.to_lowercase();
    let words: Vec<String> = lower
        .split(|c: char| !c.is_alphanumeric() && c != '-')
        .filter(|w| w.len() > 2)
        .map(|w| w.to_string())
        .collect();

    // Remove common stop words
    let stop_words = [
        "how", "does", "what", "the", "a", "an", "is", "are", "can", "you", "do", "i", "me", "my",
        "we", "they", "it", "that", "this", "be", "to", "of", "and", "or", "in", "on", "at", "for",
        "with", "from", "by", "as", "not", "if", "but", "so", "just", "please", "thanks", "thank",
        "hi", "hello", "hey", "would", "could", "should", "want", "like", "need", "has", "have",
        "had", "was", "were", "been", "being", "get", "got", "why",
    ];

    words
        .into_iter()
        .filter(|w| !stop_words.contains(&w.as_str()))
        .collect()
}

/// Calculate relevance score between keywords and document content.
fn calculate_relevance(keywords: &[String], content: &str, original_query: &str) -> f32 {
    if keywords.is_empty() {
        return 0.0;
    }

    let content_lower = content.to_lowercase();
    let query_lower = original_query.to_lowercase();

    let mut keyword_matches = 0;
    let mut keyword_positions = 0.0;
    let mut total_weight = 0.0;

    // Track position for recency bonus
    let mut position = 0;

    for keyword in keywords {
        let mut keyword_weight = 1.0;
        let mut found = false;
        let mut found_at = 0;

        // Check for exact match (highest weight)
        if content_lower.contains(keyword) {
            found = true;
            keyword_weight *= 3.0;

            // Check for exact phrase (bonus)
            if content_lower.contains(&query_lower) {
                keyword_weight *= 2.0;
            }

            // Count occurrences
            let occurrences = content_lower.matches(keyword).count();
            keyword_weight *= (1.0 + (occurrences as f32 * 0.1)).min(2.0);

            // Find first position for recency bonus
            if let Some(pos) = content_lower.find(keyword) {
                found_at = pos;
            }
        }

        if found {
            keyword_matches += 1;
            keyword_positions += found_at as f32;
            total_weight += keyword_weight;
        }
    }

    if keyword_matches == 0 {
        return 0.0;
    }

    // Base score from keyword match ratio
    let match_ratio = keyword_matches as f32 / keywords.len() as f32;

    // Recency bonus (content earlier in document scores higher)
    let avg_position = keyword_positions / keyword_matches as f32;
    let content_len = content_lower.len().max(1) as f32;
    let recency_bonus = 1.0 - (avg_position / content_len).min(0.9);

    (match_ratio * total_weight / keywords.len() as f32 * recency_bonus).min(1.0)
}

/// Extract a relevant snippet from content around matched keywords.
fn extract_relevant_snippet(content: &str, keywords: &[String], max_len: usize) -> String {
    let content_lower = content.to_lowercase();

    // Find the best starting position for the snippet
    let mut best_position = 0;

    for keyword in keywords {
        if let Some(pos) = content_lower.find(keyword) {
            // Start a bit before the match for context
            best_position = pos.saturating_sub(50);
            break;
        }
    }

    // Extract snippet with some padding
    let end_position = (best_position + max_len).min(content.len());
    let mut snippet: String = content
        .chars()
        .skip(best_position)
        .take(end_position - best_position)
        .collect();

    // Clean up: remove partial words at start/end
    if best_position > 0 {
        if let Some(first_space) = snippet.find(' ') {
            if first_space < 20 {
                snippet = snippet[first_space + 1..].to_string();
            }
        }
    }

    if end_position < content.len() {
        if let Some(last_space) = snippet.rfind(' ') {
            if snippet.len() - last_space < 20 {
                snippet = snippet[..last_space].to_string();
            }
        }
        snippet.push_str("...");
    }

    // Remove markdown formatting from snippet
    snippet = remove_markdown_formatting(&snippet);

    snippet.trim().to_string()
}

/// Remove basic markdown formatting from snippet text.
fn remove_markdown_formatting(text: &str) -> String {
    let mut result = text.to_string();

    // Remove code block markers
    result = result.replace("```", "");
    result = result.replace("`", "");

    // Remove heading markers
    result = result.replace("# ", "");
    result = result.replace("## ", "");
    result = result.replace("### ", "");

    // Remove bold/italic
    result = result.replace("**", "");
    result = result.replace("*", "");
    result = result.replace("_", "");

    result
}

/// Extract section heading if keywords appear in a section.
fn extract_section_heading(content: &str, keywords: &[String]) -> Option<String> {
    let content_lower = content.to_lowercase();
    let keyword_set: std::collections::HashSet<_> = keywords.iter().cloned().collect();

    // Look for headings (lines starting with # or ===)
    for line in content.lines() {
        let line_lower = line.to_lowercase();

        // Check if this line is a heading and contains a keyword
        if line_lower.starts_with("## ") || line_lower.starts_with("### ") {
            let heading_text = line.trim_start_matches('#').trim();
            let heading_lower = heading_text.to_lowercase();

            for keyword in &keyword_set {
                if heading_lower.contains(keyword) {
                    return Some(heading_text.to_string());
                }
            }
        }
    }

    None
}

/// Extract title from document content (frontmatter or first heading).
fn extract_title_from_content(content: &str) -> Option<String> {
    // Check for YAML frontmatter title
    let frontmatter_end = content.find("---").unwrap_or(usize::MAX);
    if frontmatter_end == 0 {
        // There's frontmatter at the start
        if let Some(second_dash) = content[3..].find("---") {
            let frontmatter = &content[3..second_dash + 3];
            for line in frontmatter.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("title:") {
                    let title = trimmed.trim_start_matches("title:").trim();
                    // Remove quotes if present
                    let title = title.trim_matches('"').trim_matches('\'');
                    return Some(title.to_string());
                }
            }
        }
    }

    // Check for first heading
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("# ") {
            return Some(line.trim_start_matches("# ").to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_searcher() -> DocSearcher {
        DocSearcher::new("docs")
    }

    #[test]
    fn test_keyword_extraction() {
        let keywords = extract_keywords("How does the triage engine work?");
        assert!(keywords.contains(&"triage".to_string()));
        assert!(keywords.contains(&"engine".to_string()));
        assert!(!keywords.contains(&"how".to_string()));
        assert!(!keywords.contains(&"does".to_string()));
    }

    #[test]
    fn test_relevance_calculation() {
        let content = "The triage engine processes GitHub issues through a state machine. \
                      It uses the LLM to classify issues and determines next actions.";
        let keywords = extract_keywords("triage engine llm classification");

        let relevance = calculate_relevance(&keywords, content, "triage engine llm");
        assert!(relevance > 0.5, "Keywords should match content");

        let low_relevance = calculate_relevance(&["zebra".to_string()], content, "zebra");
        assert!(
            low_relevance == 0.0,
            "Non-matching keyword should have zero relevance"
        );
    }

    #[test]
    fn test_snippet_extraction() {
        let content = "This is a test document with some content.\n\
                      The triage engine is the main component.\n\
                      It processes issues and generates actions.";
        let keywords = extract_keywords("triage engine");

        let snippet = extract_relevant_snippet(content, &keywords, 100);
        assert!(snippet.contains("triage") || snippet.contains("engine"));
    }

    #[test]
    fn test_markdown_formatting_removal() {
        let text = "# Heading\nSome **bold** text and `code`.";
        let cleaned = remove_markdown_formatting(text);
        assert!(!cleaned.contains("**"));
        assert!(!cleaned.contains("#"));
    }

    #[test]
    fn test_doc_searcher_empty() {
        let searcher = create_test_searcher();
        let results = searcher.search("test query");
        assert!(results.is_empty());
    }

    #[test]
    fn test_has_match_no_documents() {
        let searcher = create_test_searcher();
        assert!(!searcher.has_match("anything"));
    }
}
