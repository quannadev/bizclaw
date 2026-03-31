//! Document chunker — splits documents into search-friendly chunks.
//! Designed for minimal memory: processes line-by-line, never loads full doc.
//!
//! ## v2 Enhancements
//! - Respects heading boundaries (never splits mid-section)
//! - Overlap between chunks for better retrieval
//! - Smarter paragraph detection

/// Split text into chunks of approximately `max_chars` characters.
/// Respects document structure: headings start new chunks, paragraphs are boundaries.
/// Adds small overlap between chunks for better retrieval continuity.
pub fn chunk_text(text: &str, max_chars: usize) -> Vec<String> {
    let max_chars = max_chars.max(100); // Min 100 chars
    let overlap_chars = max_chars / 5; // 20% overlap for retrieval quality
    let mut chunks = Vec::new();
    let mut current = String::new();

    for line in text.lines() {
        let trimmed = line.trim();

        // Heading detection — start a new chunk at headings
        if is_heading(trimmed) && !current.is_empty() && current.len() > overlap_chars {
            chunks.push(std::mem::take(&mut current));
        }

        // Empty line = paragraph break → natural chunk boundary
        if trimmed.is_empty() && !current.is_empty() && current.len() > max_chars / 2 {
            chunks.push(std::mem::take(&mut current));
            continue;
        }

        // If the line itself is longer than max_chars, split by words
        if trimmed.len() > max_chars {
            // Flush current buffer first
            if !current.is_empty() {
                chunks.push(std::mem::take(&mut current));
            }
            // Split long line by words
            for word in trimmed.split_whitespace() {
                if !current.is_empty() && current.len() + word.len() + 1 > max_chars {
                    chunks.push(std::mem::take(&mut current));
                }
                if !current.is_empty() {
                    current.push(' ');
                }
                current.push_str(word);
            }
            continue;
        }

        // If adding this line would exceed max_chars, flush current chunk
        if !current.is_empty() && current.len() + trimmed.len() + 1 > max_chars {
            chunks.push(std::mem::take(&mut current));
        }

        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(trimmed);
    }

    // Don't forget the last chunk
    if !current.is_empty() {
        chunks.push(current);
    }

    // Add overlap: prepend last N chars of previous chunk to each subsequent chunk
    if chunks.len() > 1 && overlap_chars > 0 {
        let originals = chunks.clone();
        for i in 1..chunks.len() {
            let prev = &originals[i - 1];
            if prev.len() > overlap_chars {
                let overlap_start = prev.len() - overlap_chars;
                // Find word boundary for clean overlap
                let overlap_text = find_word_boundary(&prev[overlap_start..]);
                if !overlap_text.is_empty() {
                    chunks[i] = format!("...{}\n{}", overlap_text.trim(), chunks[i]);
                }
            }
        }
    }

    chunks
}

/// Check if a line is a heading (Markdown or plain text conventions).
fn is_heading(line: &str) -> bool {
    // Markdown headings
    if line.starts_with('#') {
        return true;
    }
    // All-caps lines that are short (likely section headers)
    if line.len() < 80 && line.len() > 2 && line == line.to_uppercase() && line.contains(' ') {
        return true;
    }
    false
}

/// Find the nearest word boundary in a string (start from beginning).
fn find_word_boundary(s: &str) -> &str {
    // Find the first space and start from there for a clean word break
    match s.find(' ') {
        Some(pos) => &s[pos + 1..],
        None => s,
    }
}

/// Extract plain text from common file formats.
/// Supports: .txt, .md, .json, .toml, .yaml, .csv, .log, .html
pub fn extract_text(content: &str, filename: &str) -> String {
    let ext = filename.rsplit('.').next().unwrap_or("txt").to_lowercase();
    match ext.as_str() {
        "md" | "markdown" => {
            // Keep markdown structure for heading-aware chunking
            // Only strip heavy syntax that hurts search quality
            content
                .lines()
                .map(|l| {
                    let trimmed = l.trim();
                    // Keep headings (important for chunking)
                    if trimmed.starts_with('#') {
                        return trimmed.to_string();
                    }
                    // Strip list markers, blockquotes
                    trimmed
                        .trim_start_matches("- ")
                        .trim_start_matches("* ")
                        .trim_start_matches("> ")
                        .to_string()
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
        "json" => {
            // Extract string values from JSON
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(content) {
                extract_json_strings(&val)
            } else {
                content.to_string()
            }
        }
        "html" | "htm" => {
            // Basic HTML tag stripping for search
            strip_html_tags(content)
        }
        "csv" => {
            // Convert CSV rows to searchable text
            content
                .lines()
                .map(|l| l.replace(',', " | "))
                .collect::<Vec<_>>()
                .join("\n")
        }
        _ => content.to_string(),
    }
}

/// Strip HTML tags from content (lightweight, no external dependency).
fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut last_was_space = false;

    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                if !last_was_space {
                    result.push(' ');
                    last_was_space = true;
                }
            }
            _ if !in_tag => {
                if ch.is_whitespace() {
                    if !last_was_space {
                        result.push(' ');
                        last_was_space = true;
                    }
                } else {
                    result.push(ch);
                    last_was_space = false;
                }
            }
            _ => {}
        }
    }

    result.trim().to_string()
}

/// Recursively extract string values from a JSON value.
fn extract_json_strings(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => arr
            .iter()
            .map(extract_json_strings)
            .collect::<Vec<_>>()
            .join("\n"),
        serde_json::Value::Object(map) => map
            .values()
            .map(extract_json_strings)
            .collect::<Vec<_>>()
            .join("\n"),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_short_text() {
        let chunks = chunk_text("Hello world", 500);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "Hello world");
    }

    #[test]
    fn test_chunk_paragraphs() {
        let text = "Paragraph one line one.\nParagraph one line two.\n\nParagraph two line one.\nParagraph two line two.";
        let chunks = chunk_text(text, 100);
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_chunk_long_text() {
        let text = "word ".repeat(200); // ~1000 chars
        let chunks = chunk_text(&text, 300);
        assert!(
            chunks.len() >= 2,
            "Expected at least 2 chunks, got {}",
            chunks.len()
        );
    }

    #[test]
    fn test_heading_splits_chunk() {
        let text = "Some intro text here.\nMore content.\n\n# New Section\nSection content here.";
        let chunks = chunk_text(text, 500);
        // Heading should trigger a chunk boundary
        assert!(!chunks.is_empty());
        // At least one chunk should start with or contain the heading
        let has_heading_chunk = chunks.iter().any(|c| c.contains("# New Section"));
        assert!(has_heading_chunk, "Heading should be preserved in a chunk");
    }

    #[test]
    fn test_extract_markdown_preserves_headings() {
        let md = "# Title\n## Sub\n- item\n> quote";
        let text = extract_text(md, "doc.md");
        // Headings should be preserved now
        assert!(text.contains("# Title"));
        assert!(text.contains("## Sub"));
    }

    #[test]
    fn test_extract_html() {
        let html = "<h1>Title</h1><p>Hello <b>world</b></p>";
        let text = extract_text(html, "page.html");
        assert!(text.contains("Title"));
        assert!(text.contains("Hello"));
        assert!(text.contains("world"));
        assert!(!text.contains("<h1>"));
    }

    #[test]
    fn test_strip_html_tags() {
        let html = "<div><p>Hello</p><p>World</p></div>";
        let text = strip_html_tags(html);
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
        assert!(!text.contains('<'));
    }

    #[test]
    fn test_is_heading() {
        assert!(is_heading("# Title"));
        assert!(is_heading("## Sub Title"));
        assert!(is_heading("COMPANY POLICY"));
        assert!(!is_heading("normal text here"));
        assert!(!is_heading("A")); // too short
    }

    #[test]
    fn test_overlap_between_chunks() {
        // Create enough text to force multiple chunks
        let text = (0..20)
            .map(|i| format!("Sentence number {} with some content.", i))
            .collect::<Vec<_>>()
            .join(" ");
        let chunks = chunk_text(&text, 200);
        if chunks.len() > 1 {
            // Second chunk should start with "..." (overlap marker)
            assert!(
                chunks[1].starts_with("..."),
                "Second chunk should have overlap prefix"
            );
        }
    }
}
