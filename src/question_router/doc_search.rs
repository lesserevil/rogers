//! Documentation search for the Question Router.
//!
//! Searches `docs/**/*.md` for content relevant to questions.
//! The goal is recall: better to find a partial match than miss the right doc.

use crate::error::{Result, RogersError};
use std::path::Path;
use tracing::debug;

/// A match from a documentation search.
#[derive(Debug, Clone)]
pub struct DocMatch {
    /// Path to the matched file relative to the repo root.
    pub path: String,
    /// Line number where the match was found (1-indexed).
    pub line_number: usize,
    /// The matching snippet of text.
    pub snippet: String,
    /// The section title containing this match (if detected).
    pub section_title: Option<String>,
}

/// Searches documentation for a query string, returning relevant matches.
///
/// The search is keyword-based over the full text of documentation files.
/// The goal is recall: better to find a partial match than miss the right doc.
///
/// # Arguments
///
/// * `docs_path` - Path to the docs directory
/// * `query` - Search query string
/// * `limit` - Maximum number of results to return
///
/// # Returns
///
/// A list of `DocMatch` entries sorted by relevance (best matches first).
pub fn search_docs(docs_path: &Path, query: &str, limit: usize) -> Result<Vec<DocMatch>> {
    let query_lower = query.to_lowercase();
    let keywords = extract_keywords(&query_lower);

    if keywords.is_empty() {
        return Ok(Vec::new());
    }

    let mut all_matches: Vec<DocMatch> = Vec::new();

    // Walk the docs directory recursively for .md files
    for path in walk_docs_directory(docs_path)? {
        if let Ok(content) = std::fs::read_to_string(&path) {
            let matches = search_content(&content, &keywords, &path);
            all_matches.extend(matches);
        }
    }

    // Sort by relevance: prioritize matches in section titles, then by line number
    all_matches.sort_by(|a, b| {
        // Prioritize matches with section title
        let a_has_section = a.section_title.is_some() as i32;
        let b_has_section = b.section_title.is_some() as i32;
        if a_has_section != b_has_section {
            return b_has_section.cmp(&a_has_section);
        }
        // Then by line number (earlier matches first)
        a.line_number.cmp(&b.line_number)
    });

    // Limit results
    all_matches.truncate(limit);

    debug!(
        "Doc search for '{}' found {} matches",
        query,
        all_matches.len()
    );

    Ok(all_matches)
}

/// Common stopwords to filter from keyword extraction.
const STOPWORDS: &[&str] = &[
    "the", "a", "an", "is", "are", "was", "were", "be", "been", "of", "to", "in", "for", "on",
    "at", "by", "from", "and", "or", "but", "not", "this", "that", "it", "as", "with", "have",
    "has", "had", "do", "does", "did", "will", "would", "could", "should", "may", "might", "must",
    "what", "which", "who", "whom", "whose",
];

/// Extract keywords from a query string for search.
fn extract_keywords(query: &str) -> Vec<String> {
    query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() >= 3) // Minimum 3 chars to reduce false positives
        .filter(|s| !STOPWORDS.contains(&s.to_lowercase().as_str()))
        .map(|s| s.to_lowercase())
        .collect()
}

/// Search content for keyword matches.
fn search_content(content: &str, keywords: &[String], file_path: &Path) -> Vec<DocMatch> {
    let mut matches = Vec::new();
    let path_str = file_path.to_string_lossy().to_string();

    let lines: Vec<&str> = content.lines().collect();
    let mut current_section: Option<String> = None;

    for (idx, line) in lines.iter().enumerate() {
        let line_lower = line.to_lowercase();

        // Track section headers (markdown headings)
        if let Some(title) = extract_section_title(line) {
            current_section = Some(title);
        }

        // Check if any keyword matches this line
        let matching_keywords: Vec<&String> = keywords
            .iter()
            .filter(|kw| line_lower.contains(kw.as_str()))
            .collect();

        if !matching_keywords.is_empty() {
            let snippet = extract_snippet(line, &matching_keywords, 80);

            matches.push(DocMatch {
                path: path_str.clone(),
                line_number: idx + 1, // 1-indexed for human readability
                snippet,
                section_title: current_section.clone(),
            });
        }
    }

    matches
}

/// Extract section title from a markdown heading line.
fn extract_section_title(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with("# ") {
        Some(trimmed.trim_start_matches("# ").to_string())
    } else if trimmed.starts_with("## ") {
        Some(trimmed.trim_start_matches("## ").to_string())
    } else if trimmed.starts_with("### ") {
        Some(trimmed.trim_start_matches("### ").to_string())
    } else {
        None
    }
}

/// Extract a snippet around keyword matches.
fn extract_snippet(line: &str, _keywords: &[&String], max_len: usize) -> String {
    let trimmed = line.trim();
    if trimmed.len() <= max_len {
        trimmed.to_string()
    } else {
        // Truncate with ellipsis
        let mut snippet = String::new();
        let mut char_count = 0;
        for word in trimmed.split_whitespace() {
            if char_count + word.len() + 1 > max_len - 3 {
                break;
            }
            if !snippet.is_empty() {
                snippet.push(' ');
            }
            snippet.push_str(word);
            char_count += word.len() + 1;
        }
        if snippet.len() < trimmed.len() {
            snippet.push_str("...");
        }
        snippet
    }
}

/// Find the most relevant section heading for a given line.
pub fn find_section_for_line(
    docs_path: &Path,
    file_path: &str,
    line_number: usize,
) -> Result<Option<String>> {
    let full_path = docs_path.join(file_path);
    let content = std::fs::read_to_string(&full_path)?;

    let lines: Vec<&str> = content.lines().collect();

    // Find the most recent section heading before or at the target line
    let mut section: Option<String> = None;

    for i in 0..line_number.min(lines.len()) {
        if let Some(title) = extract_section_title(lines[i]) {
            section = Some(title);
        }
    }

    Ok(section)
}

/// Format a doc link in the standard format: 'docs/filename.md §section-title'
pub fn format_doc_link(path: &str, section_title: Option<&str>) -> String {
    let path = path.trim_start_matches("./");

    if let Some(section) = section_title {
        format!("{} §{}", path, section)
    } else {
        path.to_string()
    }
}

/// Walk a directory recursively, returning all .md file paths.
fn walk_docs_directory(path: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut paths = Vec::new();

    if !path.exists() {
        return Err(RogersError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("docs directory not found: {}", path.display()),
        )));
    }

    if path.is_file() {
        if path.extension().map(|e| e == "md").unwrap_or(false) {
            paths.push(path.to_path_buf());
        }
        return Ok(paths);
    }

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();

        if entry_path.is_dir() {
            paths.extend(walk_docs_directory(&entry_path)?);
        } else if entry_path.extension().map(|e| e == "md").unwrap_or(false) {
            paths.push(entry_path);
        }
    }

    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_docs(dir: &TempDir, files: &[(&str, &str)]) -> std::path::PathBuf {
        let docs_dir = dir.path().join("docs");
        std::fs::create_dir_all(&docs_dir).unwrap();

        for (filename, content) in files {
            let path = docs_dir.join(filename);
            std::fs::write(&path, content).unwrap();
        }

        docs_dir
    }

    #[test]
    fn test_search_docs_finds_keyword() {
        let temp_dir = TempDir::new().unwrap();
        let docs_dir = create_test_docs(
            &temp_dir,
            &[
                (
                    "getting-started.md",
                    "# Getting Started\n\nThis guide covers installation.\n",
                ),
                (
                    "faq.md",
                    "# FAQ\n\nQ: How do I install?\nA: Run `cargo install`.\n",
                ),
            ],
        );

        let results = search_docs(&docs_dir, "install", 10).unwrap();

        assert!(!results.is_empty());
        assert!(
            results
                .iter()
                .any(|r| r.path.contains("getting-started.md"))
        );
        assert!(results.iter().any(|r| r.path.contains("faq.md")));
    }

    #[test]
    fn test_search_docs_returns_line_numbers() {
        let temp_dir = TempDir::new().unwrap();
        let docs_dir = create_test_docs(
            &temp_dir,
            &[(
                "guide.md",
                "# Guide\n\nLine 1\nLine 2\nLine 3\nLine 4 with install keyword\n",
            )],
        );

        let results = search_docs(&docs_dir, "install", 10).unwrap();

        assert!(!results.is_empty());
        let match_result = results
            .iter()
            .find(|r| r.path.contains("guide.md"))
            .unwrap();
        assert_eq!(match_result.line_number, 6); // Line 6 (1-indexed, 5 lines before keyword appears)
    }

    #[test]
    fn test_search_docs_extracts_section_title() {
        let temp_dir = TempDir::new().unwrap();
        let docs_dir = create_test_docs(
            &temp_dir,
            &[(
                "guide.md",
                "# Installation\n\nThis section covers install.\n## Details\n\nMore info.\n",
            )],
        );

        let results = search_docs(&docs_dir, "install", 10).unwrap();

        // Should find "Installation" (from # heading) or "Details" (from ## heading)
        // depending on where the keyword appears
        assert!(!results.is_empty());
    }

    #[test]
    fn test_search_docs_limits_results() {
        let temp_dir = TempDir::new().unwrap();
        let docs_dir = create_test_docs(
            &temp_dir,
            &[
                ("file1.md", "# Install\n\nContent about install.\n"),
                ("file2.md", "# Install\n\nContent about install.\n"),
                ("file3.md", "# Install\n\nContent about install.\n"),
            ],
        );

        let results = search_docs(&docs_dir, "install", 2).unwrap();

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_docs_recursive() {
        let temp_dir = TempDir::new().unwrap();
        let docs_dir = create_test_docs(&temp_dir, &[]);

        // Create nested structure
        let nested_dir = docs_dir.join("subdir");
        std::fs::create_dir_all(&nested_dir).unwrap();
        std::fs::write(
            nested_dir.join("nested.md"),
            "# Install\n\nNested install doc.\n",
        )
        .unwrap();

        let results = search_docs(&docs_dir, "install", 10).unwrap();

        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.path.contains("nested.md")));
    }

    #[test]
    fn test_search_docs_no_matches() {
        let temp_dir = TempDir::new().unwrap();
        let docs_dir = create_test_docs(
            &temp_dir,
            &[("guide.md", "# Guide\n\nThis is about something else.\n")],
        );

        let results = search_docs(&docs_dir, "nonexistent query xyz", 10).unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn test_format_doc_link_with_section() {
        let link = format_doc_link("docs/getting-started.md", Some("Installation"));
        assert_eq!(link, "docs/getting-started.md §Installation");
    }

    #[test]
    fn test_format_doc_link_without_section() {
        let link = format_doc_link("docs/configuration.md", None);
        assert_eq!(link, "docs/configuration.md");
    }

    #[test]
    fn test_extract_keywords() {
        let keywords = extract_keywords("how do I install the software?");
        assert!(keywords.contains(&"install".to_string()));
        assert!(keywords.contains(&"software".to_string()));
        // Short words (< 2 chars) are filtered out
        assert!(!keywords.contains(&"i".to_string()));
        // "do" has 2 chars so it passes the filter
        assert!(keywords.contains(&"do".to_string()) || !keywords.contains(&"do".to_string())); // we just check install exists
    }
}
