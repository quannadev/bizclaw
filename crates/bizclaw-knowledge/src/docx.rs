//! DOCX parser — extract text from Microsoft Word documents.
//!
//! Office Open XML (.docx) is a ZIP archive containing XML files.
//! We extract text from the main document body without external deps.
//!
//! ## Format
//! ```text
//! my_document.docx (ZIP archive)
//! ├── [Content_Types].xml
//! ├── word/document.xml    ← main content HERE
//! ├── word/styles.xml
//! ├── word/settings.xml
//! └── _rels/.rels
//! ```
//!
//! ## Design
//! - No extra dependencies: only uses `std::io::Read` + basic XML tag stripping
//! - Handles UTF-8 text extraction
//! - Preserves paragraph breaks
//! - Works on Pi/Android (no heavy XML parser)

/// Extract text from a DOCX file.
///
/// DOCX = ZIP archive → find word/document.xml → strip XML tags → return text.
pub fn extract_docx_text(path: &std::path::Path) -> Result<String, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("Cannot open DOCX file: {e}"))?;

    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Invalid DOCX (not a ZIP archive): {e}"))?;

    // Find word/document.xml (try primary path, then fallback)
    let doc_path = if archive.by_name("word/document.xml").is_ok() {
        "word/document.xml"
    } else if archive.by_name("word/document2.xml").is_ok() {
        "word/document2.xml"
    } else {
        return Err("No word/document.xml found in DOCX archive".into());
    };

    let mut doc_xml = archive
        .by_name(doc_path)
        .map_err(|e| format!("Read {}: {e}", doc_path))?;

    let mut xml_content = String::new();
    std::io::Read::read_to_string(&mut doc_xml, &mut xml_content)
        .map_err(|e| format!("Read document.xml: {e}"))?;

    // Extract text from XML
    let text = extract_text_from_xml(&xml_content);

    if text.trim().is_empty() {
        return Err("No text content found in DOCX".into());
    }

    Ok(text)
}

/// Extract text from a DOCX byte slice (for in-memory processing).
pub fn extract_docx_from_bytes(data: &[u8]) -> Result<String, String> {
    let cursor = std::io::Cursor::new(data);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| format!("Invalid DOCX data: {e}"))?;

    let mut doc_xml = archive
        .by_name("word/document.xml")
        .map_err(|e| format!("No document.xml: {e}"))?;

    let mut xml_content = String::new();
    std::io::Read::read_to_string(&mut doc_xml, &mut xml_content)
        .map_err(|e| format!("Read: {e}"))?;

    let text = extract_text_from_xml(&xml_content);
    if text.trim().is_empty() {
        return Err("No text content in DOCX".into());
    }
    Ok(text)
}

/// Extract text from OOXML document body.
///
/// We look for `<w:t>` tags (text runs) and `<w:p>` tags (paragraphs).
/// This is a simple but effective approach that handles 95%+ of real documents.
fn extract_text_from_xml(xml: &str) -> String {
    let mut text = String::new();
    let mut in_text_run = false;
    let mut in_paragraph = false;
    let mut paragraph_has_text = false;
    let mut chars = xml.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '<' {
            // Read tag name
            let mut tag = String::new();
            while let Some(&next) = chars.peek() {
                if next == '>' {
                    chars.next();
                    break;
                }
                tag.push(next);
                chars.next();
            }

            let tag_lower = tag.to_lowercase();

            // Paragraph start
            if tag_lower.starts_with("w:p ") || tag_lower == "w:p" {
                in_paragraph = true;
                paragraph_has_text = false;
            }
            // Paragraph end
            else if tag_lower == "/w:p" {
                if in_paragraph && paragraph_has_text {
                    text.push('\n');
                }
                in_paragraph = false;
            }
            // Text run start
            else if tag_lower.starts_with("w:t ") || tag_lower == "w:t" {
                in_text_run = true;
            }
            // Text run end
            else if tag_lower == "/w:t" {
                in_text_run = false;
            }
            // Line break
            else if tag_lower == "w:br" || tag_lower == "w:br/" || tag_lower.starts_with("w:br ")
            {
                text.push('\n');
            }
            // Tab
            else if tag_lower == "w:tab" || tag_lower == "w:tab/" {
                text.push('\t');
            }
        } else if in_text_run {
            // Decode basic XML entities
            if ch == '&' {
                let mut entity = String::new();
                while let Some(&next) = chars.peek() {
                    if next == ';' {
                        chars.next();
                        break;
                    }
                    entity.push(next);
                    chars.next();
                    if entity.len() > 8 {
                        break; // Safety limit
                    }
                }
                match entity.as_str() {
                    "amp" => text.push('&'),
                    "lt" => text.push('<'),
                    "gt" => text.push('>'),
                    "quot" => text.push('"'),
                    "apos" => text.push('\''),
                    "#10" => text.push('\n'),
                    "#13" => {} // carriage return, ignore
                    _ => {
                        text.push('&');
                        text.push_str(&entity);
                        text.push(';');
                    }
                }
            } else {
                text.push(ch);
            }
            paragraph_has_text = true;
        }
    }

    // Clean up: collapse multiple blank lines
    let mut cleaned = String::new();
    let mut prev_was_newline = false;
    for line in text.lines() {
        if line.trim().is_empty() {
            if !prev_was_newline {
                cleaned.push('\n');
                prev_was_newline = true;
            }
        } else {
            cleaned.push_str(line);
            cleaned.push('\n');
            prev_was_newline = false;
        }
    }

    cleaned.trim().to_string()
}

/// Check if a file looks like a DOCX (ZIP with word/document.xml).
pub fn is_docx(path: &std::path::Path) -> bool {
    // Quick check: file extension
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if ext.to_lowercase() != "docx" {
            return false;
        }
    } else {
        return false;
    }

    // Deeper check: try opening as ZIP and look for word/document.xml
    if let Ok(file) = std::fs::File::open(path)
        && let Ok(mut archive) = zip::ZipArchive::new(file)
    {
        return archive.by_name("word/document.xml").is_ok();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_text_from_xml() {
        let xml = r#"<?xml version="1.0"?>
        <w:document>
            <w:body>
                <w:p><w:r><w:t>Hello World</w:t></w:r></w:p>
                <w:p><w:r><w:t>Second paragraph</w:t></w:r></w:p>
                <w:p><w:r><w:t>Vietnamese: Xin chào</w:t></w:r></w:p>
            </w:body>
        </w:document>"#;

        let text = extract_text_from_xml(xml);
        assert!(text.contains("Hello World"));
        assert!(text.contains("Second paragraph"));
        assert!(text.contains("Vietnamese: Xin chào"));
        // Check paragraphs are separated
        assert!(text.contains('\n'));
    }

    #[test]
    fn test_xml_entities() {
        let xml = r#"<w:p><w:r><w:t>Tom &amp; Jerry &lt;3&gt;</w:t></w:r></w:p>"#;
        let text = extract_text_from_xml(xml);
        assert!(text.contains("Tom & Jerry <3>"));
    }

    #[test]
    fn test_multiple_runs_in_paragraph() {
        let xml = r#"<w:p>
            <w:r><w:t>bold </w:t></w:r>
            <w:r><w:t>normal </w:t></w:r>
            <w:r><w:t>italic</w:t></w:r>
        </w:p>"#;
        let text = extract_text_from_xml(xml);
        assert!(text.contains("bold normal italic"));
    }

    #[test]
    fn test_line_breaks() {
        let xml = r#"<w:p><w:r><w:t>Line 1</w:t></w:r><w:br/><w:r><w:t>Line 2</w:t></w:r></w:p>"#;
        let text = extract_text_from_xml(xml);
        assert!(text.contains("Line 1\nLine 2"));
    }

    #[test]
    fn test_empty_paragraphs_collapsed() {
        let xml = r#"<w:p><w:r><w:t>A</w:t></w:r></w:p>
        <w:p></w:p>
        <w:p></w:p>
        <w:p></w:p>
        <w:p><w:r><w:t>B</w:t></w:r></w:p>"#;
        let text = extract_text_from_xml(xml);
        // Multiple empty paragraphs should be collapsed
        assert!(!text.contains("\n\n\n"));
    }

    #[test]
    fn test_is_not_docx() {
        assert!(!is_docx(std::path::Path::new("test.txt")));
        assert!(!is_docx(std::path::Path::new("test.pdf")));
        assert!(!is_docx(std::path::Path::new("/nonexistent.docx")));
    }
}
