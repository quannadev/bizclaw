//! File Upload Middleware — auto-convert uploaded documents to markdown.
//!
//! Ported from DeerFlow 2.0's file upload pipeline.
//! When a user mentions a file path in their message, this middleware
//! detects supported file types, converts to markdown, and injects
//! the converted content into the conversation.
//!
//! ## Supported Formats
//! - PDF → markdown with headers and paragraphs
//! - Excel (xlsx/xls/csv) → markdown tables
//! - Word (docx) → structured markdown with headings
//! - PowerPoint (pptx) → slide-by-slide markdown
//! - Text/Code → fenced code blocks
//!
//! ## Architecture
//! ```text
//! User message → FileUploadMiddleware.before_model()
//!                ├── detect file paths in message
//!                ├── convert each file to markdown
//!                └── inject as system message before LLM call
//! ```

use async_trait::async_trait;
use regex::Regex;
use std::path::Path;
use tracing::{debug, info, warn};

use crate::middleware::{AgentMiddleware, AgentState, MiddlewareAction};
use bizclaw_core::types::message::Message;

/// Maximum file size to process (10 MB).
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Maximum characters of converted content to inject.
const MAX_INJECTION_CHARS: usize = 50_000;

/// Supported file extensions.
const SUPPORTED_EXTENSIONS: &[&str] = &[
    "pdf", "docx", "xlsx", "xls", "csv", "pptx", "txt", "md", "json", "xml", "yaml", "yml", "rs",
    "py", "js", "ts", "go", "java", "c", "cpp", "h", "html", "css", "sql", "sh", "toml", "log",
];

/// File Upload Middleware — detects file paths, converts to markdown, injects context.
pub struct FileUploadMiddleware {
    enabled: bool,
    max_file_size: u64,
    max_injection_chars: usize,
}

impl FileUploadMiddleware {
    pub fn new() -> Self {
        Self {
            enabled: true,
            max_file_size: MAX_FILE_SIZE,
            max_injection_chars: MAX_INJECTION_CHARS,
        }
    }

    pub fn with_limits(max_file_size: u64, max_injection_chars: usize) -> Self {
        Self {
            enabled: true,
            max_file_size,
            max_injection_chars,
        }
    }

    /// Detect file paths in a message (absolute paths or ~/paths).
    fn detect_file_paths(message: &str) -> Vec<String> {
        let re = Regex::new(r#"(?:^|\s|["'`(])(/[^\s"'`)\n]+\.[a-zA-Z0-9]+)"#).unwrap();
        let tilde_re = Regex::new(r#"(?:^|\s|["'`(])(~/[^\s"'`)\n]+\.[a-zA-Z0-9]+)"#).unwrap();

        let mut paths = Vec::new();

        for cap in re.captures_iter(message) {
            if let Some(m) = cap.get(1) {
                let path_str = m.as_str();
                if Self::is_supported_extension(path_str) {
                    paths.push(path_str.to_string());
                }
            }
        }

        for cap in tilde_re.captures_iter(message) {
            if let Some(m) = cap.get(1) {
                let expanded = shellexpand::tilde(m.as_str());
                if Self::is_supported_extension(&expanded) {
                    paths.push(expanded.to_string());
                }
            }
        }

        paths.dedup();
        paths
    }

    /// Check if a path has a supported extension.
    fn is_supported_extension(path: &str) -> bool {
        if let Some(ext) = Path::new(path).extension().and_then(|e| e.to_str()) {
            SUPPORTED_EXTENSIONS.contains(&ext.to_lowercase().as_str())
        } else {
            false
        }
    }

    /// Convert a file to markdown format.
    fn convert_to_markdown(path: &Path) -> Result<String, String> {
        let metadata =
            std::fs::metadata(path).map_err(|e| format!("Cannot read file metadata: {e}"))?;

        if metadata.len() > MAX_FILE_SIZE {
            return Err(format!(
                "File too large: {} bytes (max {} MB)",
                metadata.len(),
                MAX_FILE_SIZE / 1024 / 1024
            ));
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        match ext.as_str() {
            "pdf" => Self::convert_pdf(path, filename),
            "docx" => Self::convert_docx(path, filename),
            "xlsx" | "xls" => Self::convert_excel(path, filename),
            "csv" => Self::convert_csv(path, filename),
            "pptx" => Self::convert_pptx(path, filename),
            _ => Self::convert_text(path, filename, &ext),
        }
    }

    /// Convert PDF to markdown.
    fn convert_pdf(path: &Path, filename: &str) -> Result<String, String> {
        let text = pdf_extract::extract_text(path).map_err(|e| format!("PDF parse error: {e}"))?;

        // Split into paragraphs and format
        let paragraphs: Vec<&str> = text.split("\n\n").collect();
        let mut md = format!("# 📄 {filename}\n\n");

        for para in paragraphs.iter() {
            let trimmed = para.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Heuristic: short lines in CAPS are likely headings
            if trimmed.len() < 100
                && trimmed.chars().filter(|c| c.is_uppercase()).count()
                    > trimmed.chars().filter(|c| c.is_lowercase()).count()
            {
                md.push_str(&format!("\n## {trimmed}\n\n"));
            } else {
                md.push_str(trimmed);
                md.push_str("\n\n");
            }
        }

        Ok(md)
    }

    /// Convert DOCX to markdown with heading detection.
    fn convert_docx(path: &Path, filename: &str) -> Result<String, String> {
        let file = std::fs::File::open(path).map_err(|e| format!("Cannot open file: {e}"))?;
        let mut archive =
            zip::ZipArchive::new(file).map_err(|e| format!("Invalid DOCX archive: {e}"))?;

        let mut xml_content = String::new();
        {
            let mut doc_file = archive
                .by_name("word/document.xml")
                .map_err(|_| "Not a valid DOCX (missing word/document.xml)")?;
            use std::io::Read;
            doc_file
                .read_to_string(&mut xml_content)
                .map_err(|e| format!("Read error: {e}"))?;
        }

        let p_re = Regex::new(r"<w:p\b[^>]*>(.*?)</w:p>").unwrap();
        let t_re = Regex::new(r"<w:t\b[^>]*>(.*?)</w:t>").unwrap();
        let heading_re = Regex::new(r#"<w:pStyle\s+w:val="Heading(\d)""#).unwrap();

        let mut md = format!("# 📝 {filename}\n\n");

        for p_cap in p_re.captures_iter(&xml_content) {
            if let Some(p_match) = p_cap.get(1) {
                let p_content = p_match.as_str();

                // Detect heading level
                let heading_level = heading_re
                    .captures(p_content)
                    .and_then(|c| c.get(1))
                    .and_then(|m| m.as_str().parse::<usize>().ok());

                // Extract text
                let mut line = String::new();
                for t_cap in t_re.captures_iter(p_content) {
                    if let Some(t_m) = t_cap.get(1) {
                        let text = t_m
                            .as_str()
                            .replace("&lt;", "<")
                            .replace("&gt;", ">")
                            .replace("&amp;", "&")
                            .replace("&quot;", "\"")
                            .replace("&apos;", "'");
                        line.push_str(&text);
                    }
                }

                if line.trim().is_empty() {
                    continue;
                }

                if let Some(level) = heading_level {
                    let prefix = "#".repeat(level.min(6));
                    md.push_str(&format!("{prefix} {}\n\n", line.trim()));
                } else {
                    md.push_str(line.trim());
                    md.push_str("\n\n");
                }
            }
        }

        Ok(md)
    }

    /// Convert Excel to markdown tables.
    fn convert_excel(path: &Path, filename: &str) -> Result<String, String> {
        use calamine::{Data, Reader, open_workbook_auto};

        let mut workbook =
            open_workbook_auto(path).map_err(|e| format!("Excel parse error: {e}"))?;

        let sheet_names = workbook.sheet_names().to_owned();
        let mut md = format!("# 📊 {filename}\n\n");

        for sheet_name in sheet_names {
            md.push_str(&format!("## Sheet: {sheet_name}\n\n"));

            if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                let rows: Vec<Vec<String>> = range
                    .rows()
                    .map(|row| {
                        row.iter()
                            .map(|cell| match cell {
                                Data::String(s) => s.to_string(),
                                Data::Float(f) => format!("{f:.2}"),
                                Data::Int(i) => i.to_string(),
                                Data::Bool(b) => b.to_string(),
                                Data::Empty => String::new(),
                                Data::Error(e) => format!("Error({e})"),
                                Data::DateTime(v) => v.as_f64().to_string(),
                                Data::DateTimeIso(v) => v.to_string(),
                                Data::DurationIso(v) => v.to_string(),
                            })
                            .collect()
                    })
                    .collect();

                if rows.is_empty() {
                    md.push_str("*Empty sheet*\n\n");
                    continue;
                }

                // Build markdown table
                // First row is header
                let header = &rows[0];
                md.push_str("| ");
                md.push_str(&header.join(" | "));
                md.push_str(" |\n");

                // Separator
                md.push_str("| ");
                md.push_str(&header.iter().map(|_| "---").collect::<Vec<_>>().join(" | "));
                md.push_str(" |\n");

                // Data rows (limit to 100 rows)
                for row in rows.iter().skip(1).take(100) {
                    md.push_str("| ");
                    // Pad row to header length
                    let mut padded = row.clone();
                    padded.resize(header.len(), String::new());
                    md.push_str(&padded.join(" | "));
                    md.push_str(" |\n");
                }

                if rows.len() > 101 {
                    md.push_str(&format!("\n*... {} more rows omitted*\n", rows.len() - 101));
                }

                md.push('\n');
            }
        }

        Ok(md)
    }

    /// Convert CSV to markdown table.
    fn convert_csv(path: &Path, filename: &str) -> Result<String, String> {
        let content = std::fs::read_to_string(path).map_err(|e| format!("Read error: {e}"))?;

        let mut md = format!("# 📊 {filename}\n\n");
        let lines: Vec<&str> = content.lines().collect();

        if lines.is_empty() {
            return Ok(md + "*Empty file*\n");
        }

        // Detect delimiter
        let delimiter = if lines[0].contains('\t') { '\t' } else { ',' };

        // Header
        let header: Vec<&str> = lines[0].split(delimiter).collect();
        md.push_str("| ");
        md.push_str(&header.join(" | "));
        md.push_str(" |\n| ");
        md.push_str(&header.iter().map(|_| "---").collect::<Vec<_>>().join(" | "));
        md.push_str(" |\n");

        // Data (limit 100 rows)
        for line in lines.iter().skip(1).take(100) {
            let cols: Vec<&str> = line.split(delimiter).collect();
            md.push_str("| ");
            md.push_str(&cols.join(" | "));
            md.push_str(" |\n");
        }

        if lines.len() > 101 {
            md.push_str(&format!(
                "\n*... {} more rows omitted*\n",
                lines.len() - 101
            ));
        }

        Ok(md)
    }

    /// Convert PPTX to slide-by-slide markdown.
    fn convert_pptx(path: &Path, filename: &str) -> Result<String, String> {
        let file = std::fs::File::open(path).map_err(|e| format!("Cannot open file: {e}"))?;
        let mut archive =
            zip::ZipArchive::new(file).map_err(|e| format!("Invalid PPTX archive: {e}"))?;

        let t_re = Regex::new(r"<a:t>(.*?)</a:t>").unwrap();
        let mut md = format!("# 📊 {filename}\n\n");

        // Find all slide XML files
        let mut slide_names: Vec<String> = Vec::new();
        for i in 0..archive.len() {
            if let Ok(entry) = archive.by_index(i) {
                let name = entry.name().to_string();
                if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
                    slide_names.push(name);
                }
            }
        }
        slide_names.sort();

        for (i, slide_name) in slide_names.iter().enumerate() {
            md.push_str(&format!("## Slide {}\n\n", i + 1));

            if let Ok(mut slide_file) = archive.by_name(slide_name) {
                let mut content = String::new();
                use std::io::Read;
                if slide_file.read_to_string(&mut content).is_ok() {
                    let mut texts = Vec::new();
                    for cap in t_re.captures_iter(&content) {
                        if let Some(m) = cap.get(1) {
                            let text = m.as_str().trim();
                            if !text.is_empty() {
                                texts.push(text.to_string());
                            }
                        }
                    }
                    if texts.is_empty() {
                        md.push_str("*(No text content)*\n\n");
                    } else {
                        for text in &texts {
                            md.push_str(text);
                            md.push_str("\n\n");
                        }
                    }
                }
            }
        }

        Ok(md)
    }

    /// Convert text/code files to fenced code blocks.
    fn convert_text(path: &Path, filename: &str, ext: &str) -> Result<String, String> {
        let content = std::fs::read_to_string(path).map_err(|e| format!("Read error: {e}"))?;

        let lang = match ext {
            "rs" => "rust",
            "py" => "python",
            "js" => "javascript",
            "ts" => "typescript",
            "go" => "go",
            "java" => "java",
            "c" | "h" => "c",
            "cpp" => "cpp",
            "html" => "html",
            "css" => "css",
            "sql" => "sql",
            "sh" => "bash",
            "toml" => "toml",
            "yaml" | "yml" => "yaml",
            "json" => "json",
            "xml" => "xml",
            "md" => "markdown",
            _ => "",
        };

        let mut md = format!("# 📄 {filename}\n\n");
        md.push_str(&format!("```{lang}\n{content}\n```\n"));
        Ok(md)
    }
}

impl Default for FileUploadMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentMiddleware for FileUploadMiddleware {
    async fn before_model(&self, state: &mut AgentState) -> MiddlewareAction {
        // Only process the latest user message
        let last_user_msg = state
            .messages
            .iter()
            .rev()
            .find(|m| m.role == bizclaw_core::types::Role::User);

        let message_content = match last_user_msg {
            Some(msg) => msg.content.clone(),
            None => return MiddlewareAction::Continue,
        };

        let paths = Self::detect_file_paths(&message_content);
        if paths.is_empty() {
            return MiddlewareAction::Continue;
        }

        info!("📎 FileUpload: detected {} file path(s)", paths.len());

        let mut injections = Vec::new();
        let mut total_chars = 0;

        for path_str in &paths {
            let path = Path::new(path_str);
            if !path.exists() {
                debug!("📎 File not found, skipping: {}", path_str);
                continue;
            }

            match Self::convert_to_markdown(path) {
                Ok(mut markdown) => {
                    // Enforce per-file and total limits
                    if total_chars + markdown.len() > self.max_injection_chars {
                        let remaining = self.max_injection_chars.saturating_sub(total_chars);
                        if remaining < 500 {
                            warn!("📎 Injection limit reached, skipping remaining files");
                            break;
                        }
                        markdown.truncate(remaining);
                        markdown.push_str("\n\n*... content truncated due to size limit*\n");
                    }

                    total_chars += markdown.len();
                    info!(
                        "📎 Converted: {} ({} chars)",
                        path.file_name().unwrap_or_default().to_string_lossy(),
                        markdown.len()
                    );
                    injections.push(Message::system(format!(
                        "[File Content: {}]\n{}",
                        path_str, markdown
                    )));
                }
                Err(e) => {
                    warn!("📎 Failed to convert {}: {}", path_str, e);
                    injections.push(Message::system(format!(
                        "[File Error: {} — {}]",
                        path_str, e
                    )));
                }
            }
        }

        if injections.is_empty() {
            return MiddlewareAction::Continue;
        }

        state.metadata.insert(
            "files_uploaded".into(),
            format!("{} files converted to markdown", injections.len()),
        );

        MiddlewareAction::Inject(injections)
    }

    fn name(&self) -> &str {
        "file_upload"
    }

    fn priority(&self) -> i32 {
        15 // After dangling_tool_call (5), before guardrail (10)... actually after guardrail
    }

    fn enabled(&self) -> bool {
        self.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::AgentMiddleware;

    #[test]
    fn test_detect_file_paths() {
        let msg = "Xem file /Users/test/report.pdf và /tmp/data.xlsx";
        let paths = FileUploadMiddleware::detect_file_paths(msg);
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&"/Users/test/report.pdf".to_string()));
        assert!(paths.contains(&"/tmp/data.xlsx".to_string()));
    }

    #[test]
    fn test_detect_file_paths_ignores_unsupported() {
        let msg = "File /Users/test/image.png và /tmp/video.mp4";
        let paths = FileUploadMiddleware::detect_file_paths(msg);
        assert!(paths.is_empty());
    }

    #[test]
    fn test_is_supported_extension() {
        assert!(FileUploadMiddleware::is_supported_extension("/foo/bar.pdf"));
        assert!(FileUploadMiddleware::is_supported_extension(
            "/foo/bar.xlsx"
        ));
        assert!(FileUploadMiddleware::is_supported_extension("/foo/bar.rs"));
        assert!(!FileUploadMiddleware::is_supported_extension(
            "/foo/bar.png"
        ));
        assert!(!FileUploadMiddleware::is_supported_extension(
            "/foo/bar.mp4"
        ));
    }

    #[test]
    fn test_convert_text_file() {
        // Create a temp file
        let tmp = "/tmp/test_bizclaw_upload.rs";
        std::fs::write(tmp, "fn main() {\n    println!(\"Hello\");\n}").unwrap();

        let result = FileUploadMiddleware::convert_to_markdown(Path::new(tmp));
        assert!(result.is_ok());
        let md = result.unwrap();
        assert!(md.contains("# 📄 test_bizclaw_upload.rs"));
        assert!(md.contains("```rust"));
        assert!(md.contains("fn main()"));

        let _ = std::fs::remove_file(tmp);
    }

    #[test]
    fn test_convert_csv() {
        let tmp = "/tmp/test_bizclaw_upload.csv";
        std::fs::write(tmp, "Name,Age,City\nAlice,30,Hanoi\nBob,25,HCMC").unwrap();

        let result = FileUploadMiddleware::convert_to_markdown(Path::new(tmp));
        assert!(result.is_ok());
        let md = result.unwrap();
        assert!(md.contains("| Name | Age | City |"));
        assert!(md.contains("| --- | --- | --- |"));
        assert!(md.contains("| Alice | 30 | Hanoi |"));

        let _ = std::fs::remove_file(tmp);
    }

    #[test]
    fn test_convert_nonexistent_file() {
        let result =
            FileUploadMiddleware::convert_to_markdown(Path::new("/tmp/nonexistent_xyz.pdf"));
        assert!(result.is_err());
    }

    #[test]
    fn test_middleware_properties() {
        let mw = FileUploadMiddleware::new();
        assert_eq!(mw.name(), "file_upload");
        assert!(mw.enabled());
        assert_eq!(mw.priority(), 15);
    }

    #[tokio::test]
    async fn test_middleware_no_files_in_message() {
        let mw = FileUploadMiddleware::new();
        let mut state = crate::middleware::AgentState {
            messages: vec![
                Message::system("System prompt"),
                Message::user("Hello, how are you?"),
            ],
            estimated_tokens: 100,
            max_context_tokens: 4000,
            session_id: "test".into(),
            model_name: "test".into(),
            pending_tool_calls: vec![],
            plan_mode: false,
            subagent_enabled: false,
            metadata: std::collections::HashMap::new(),
        };

        let result = mw.before_model(&mut state).await;
        assert!(matches!(result, MiddlewareAction::Continue));
    }

    #[tokio::test]
    async fn test_middleware_with_real_file() {
        let tmp = "/tmp/test_mw_upload.txt";
        std::fs::write(tmp, "Test content for middleware").unwrap();

        let mw = FileUploadMiddleware::new();
        let mut state = crate::middleware::AgentState {
            messages: vec![
                Message::system("System prompt"),
                Message::user(&format!("Đọc file {tmp} giúp tôi")),
            ],
            estimated_tokens: 100,
            max_context_tokens: 4000,
            session_id: "test".into(),
            model_name: "test".into(),
            pending_tool_calls: vec![],
            plan_mode: false,
            subagent_enabled: false,
            metadata: std::collections::HashMap::new(),
        };

        let result = mw.before_model(&mut state).await;
        match result {
            MiddlewareAction::Inject(msgs) => {
                assert!(!msgs.is_empty());
                assert!(msgs[0].content.contains("Test content for middleware"));
            }
            _ => panic!("Expected Inject with file content"),
        }

        let _ = std::fs::remove_file(tmp);
    }
}
