//! Knowledge folder watcher — auto-ingest files into the knowledge base.
//!
//! Watches `~/.bizclaw/knowledge/` for new files and auto-adds them to the KB.
//! Uses polling (no inotify/fsevents deps) for cross-platform + edge compatibility.
//!
//! ## Design
//! - **No extra dependencies**: polling-based, uses std::fs only
//! - **Incremental**: tracks processed files in SQLite to avoid re-processing
//! - **Supports**: .txt, .md, .pdf, .csv, .html, .docx (with parser)
//! - **Safe**: ignores hidden files, temp files, files > 10MB
//!
//! ## Usage
//! ```rust,ignore
//! use bizclaw_knowledge::watcher::FolderWatcher;
//! use bizclaw_knowledge::store::KnowledgeStore;
//! let watcher = FolderWatcher::new("/path/to/knowledge/folder");
//! let store = KnowledgeStore::open(std::path::Path::new("kb.db")).unwrap();
//! let results = watcher.scan_and_ingest(&store);
//! ```

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::store::KnowledgeStore;

/// Maximum file size to auto-ingest (10MB).
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// File extensions we can process.
const SUPPORTED_EXTENSIONS: &[&str] = &[
    "txt", "md", "markdown", "csv", "html", "htm", "json", "yaml", "yml", "toml", "xml", "log",
    "rst", "pdf",
];

/// Result of processing a single file.
#[derive(Debug, Clone, serde::Serialize)]
pub struct IngestResult {
    pub filename: String,
    pub status: IngestStatus,
    pub chunks: usize,
    pub file_size: u64,
    pub message: String,
}

/// Status of file ingestion.
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IngestStatus {
    /// Successfully added to knowledge base.
    Added,
    /// File already in knowledge base (skipped).
    Skipped,
    /// Failed to process.
    Failed,
    /// File too large.
    TooLarge,
    /// Unsupported file type.
    Unsupported,
}

/// Folder watcher for auto-ingestion.
pub struct FolderWatcher {
    folder: PathBuf,
}

impl FolderWatcher {
    /// Create a watcher for the given folder.
    pub fn new(folder: impl Into<PathBuf>) -> Self {
        Self {
            folder: folder.into(),
        }
    }

    /// Create a watcher for the default knowledge folder (~/.bizclaw/knowledge/).
    pub fn default_folder() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Self::new(home.join(".bizclaw").join("knowledge"))
    }

    /// Get the watched folder path.
    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// Scan for new files and ingest them into the knowledge base.
    ///
    /// Compares folder contents against existing documents in the KB.
    /// Returns a list of ingestion results.
    pub fn scan_and_ingest(&self, store: &KnowledgeStore) -> Vec<IngestResult> {
        let mut results = Vec::new();

        // Ensure folder exists
        if !self.folder.exists() {
            if let Err(e) = std::fs::create_dir_all(&self.folder) {
                tracing::warn!("Failed to create knowledge folder: {}", e);
                return results;
            }
            tracing::info!("📁 Created knowledge folder: {}", self.folder.display());
            return results; // Empty folder, nothing to ingest
        }

        // Get list of already-indexed documents
        let existing_docs: HashSet<String> = store
            .list_documents()
            .iter()
            .map(|d| d.name.clone())
            .collect();

        // Scan folder for files
        let files = match self.list_files() {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!("Failed to scan knowledge folder: {}", e);
                return results;
            }
        };

        for (path, filename, size) in files {
            // Skip already indexed
            if existing_docs.contains(&filename) {
                continue;
            }

            // Check file size
            if size > MAX_FILE_SIZE {
                results.push(IngestResult {
                    filename,
                    status: IngestStatus::TooLarge,
                    chunks: 0,
                    file_size: size,
                    message: format!(
                        "File too large: {}MB (max {}MB)",
                        size / (1024 * 1024),
                        MAX_FILE_SIZE / (1024 * 1024)
                    ),
                });
                continue;
            }

            // Check extension
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            if !SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
                results.push(IngestResult {
                    filename,
                    status: IngestStatus::Unsupported,
                    chunks: 0,
                    file_size: size,
                    message: format!("Unsupported file type: .{}", ext),
                });
                continue;
            }

            // Ingest the file
            match self.ingest_file(store, &path, &filename, size) {
                Ok(chunks) => {
                    results.push(IngestResult {
                        filename: filename.clone(),
                        status: IngestStatus::Added,
                        chunks,
                        file_size: size,
                        message: format!("Added {} chunks", chunks),
                    });
                    tracing::info!("📄 Auto-ingested: {} ({} chunks)", filename, chunks);
                }
                Err(e) => {
                    results.push(IngestResult {
                        filename,
                        status: IngestStatus::Failed,
                        chunks: 0,
                        file_size: size,
                        message: format!("Error: {}", e),
                    });
                }
            }
        }

        if !results.is_empty() {
            let added = results
                .iter()
                .filter(|r| r.status == IngestStatus::Added)
                .count();
            if added > 0 {
                tracing::info!(
                    "📚 Folder watcher: {} new file(s) ingested from {}",
                    added,
                    self.folder.display()
                );
            }
        }

        results
    }

    /// List all supported files in the watched folder (non-recursive).
    fn list_files(&self) -> Result<Vec<(PathBuf, String, u64)>, String> {
        let entries = std::fs::read_dir(&self.folder).map_err(|e| format!("Read dir: {e}"))?;

        let mut files = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let filename = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };

            // Skip hidden files and temp files
            if filename.starts_with('.')
                || filename.starts_with('~')
                || filename.ends_with(".tmp")
                || filename.ends_with(".swp")
            {
                continue;
            }

            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

            files.push((path, filename, size));
        }

        // Sort by name for deterministic order
        files.sort_by(|a, b| a.1.cmp(&b.1));
        Ok(files)
    }

    /// Ingest a single file into the knowledge base.
    fn ingest_file(
        &self,
        store: &KnowledgeStore,
        path: &Path,
        filename: &str,
        file_size: u64,
    ) -> Result<usize, String> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        // PDF files need special handling
        #[cfg(feature = "pdf")]
        if ext == "pdf" {
            let data = std::fs::read(path).map_err(|e| format!("Read PDF: {e}"))?;
            return store
                .add_pdf_document_with_meta(filename, &data, "folder_watcher", "")
                .map_err(|e| format!("PDF ingest: {e}"));
        }

        #[cfg(not(feature = "pdf"))]
        if ext == "pdf" {
            return Err("PDF support not compiled in".into());
        }

        // Text-based files
        let content = std::fs::read_to_string(path).map_err(|e| format!("Read file: {e}"))?;

        if content.trim().is_empty() {
            return Err("Empty file".into());
        }

        store
            .add_document_with_meta(
                filename,
                &content,
                "folder_watcher",
                "", // no owner for auto-ingested
                file_size as usize,
            )
            .map_err(|e| format!("Add document: {e}"))
    }

    /// Get summary of the watched folder.
    pub fn summary(&self) -> FolderSummary {
        if !self.folder.exists() {
            return FolderSummary {
                folder: self.folder.display().to_string(),
                exists: false,
                total_files: 0,
                supported_files: 0,
                total_size: 0,
            };
        }

        let files = self.list_files().unwrap_or_default();
        let supported = files
            .iter()
            .filter(|(p, _, _)| {
                p.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| SUPPORTED_EXTENSIONS.contains(&e.to_lowercase().as_str()))
                    .unwrap_or(false)
            })
            .count();
        let total_size: u64 = files.iter().map(|(_, _, s)| s).sum();

        FolderSummary {
            folder: self.folder.display().to_string(),
            exists: true,
            total_files: files.len(),
            supported_files: supported,
            total_size,
        }
    }
}

/// Summary of the watched folder.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FolderSummary {
    pub folder: String,
    pub exists: bool,
    pub total_files: usize,
    pub supported_files: usize,
    pub total_size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_empty_folder() {
        let tmp = tempfile::TempDir::new().unwrap();
        let kb_path = tmp.path().join("test.db");
        let store = KnowledgeStore::open(&kb_path).unwrap();

        let watch_dir = tmp.path().join("knowledge");
        std::fs::create_dir_all(&watch_dir).unwrap();

        let watcher = FolderWatcher::new(&watch_dir);
        let results = watcher.scan_and_ingest(&store);
        assert!(results.is_empty());
    }

    #[test]
    fn test_scan_and_ingest_text() {
        let tmp = tempfile::TempDir::new().unwrap();
        let kb_path = tmp.path().join("test.db");
        let store = KnowledgeStore::open(&kb_path).unwrap();

        let watch_dir = tmp.path().join("knowledge");
        std::fs::create_dir_all(&watch_dir).unwrap();

        // Create a test file
        std::fs::write(
            watch_dir.join("policy.md"),
            "# Chính sách nghỉ phép\n\nNhân viên được nghỉ 12 ngày/năm.",
        )
        .unwrap();

        let watcher = FolderWatcher::new(&watch_dir);
        let results = watcher.scan_and_ingest(&store);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, IngestStatus::Added);
        assert!(results[0].chunks > 0);

        // Second scan should skip (already indexed)
        let results2 = watcher.scan_and_ingest(&store);
        assert!(results2.is_empty(), "Should skip already-indexed files");
    }

    #[test]
    fn test_skip_hidden_and_temp_files() {
        let tmp = tempfile::TempDir::new().unwrap();
        let watch_dir = tmp.path().join("knowledge");
        std::fs::create_dir_all(&watch_dir).unwrap();

        std::fs::write(watch_dir.join(".hidden"), "secret").unwrap();
        std::fs::write(watch_dir.join("~backup.md"), "old").unwrap();
        std::fs::write(watch_dir.join("file.tmp"), "temp").unwrap();
        std::fs::write(watch_dir.join("file.swp"), "swap").unwrap();

        let watcher = FolderWatcher::new(&watch_dir);
        let files = watcher.list_files().unwrap();
        assert!(files.is_empty(), "Should skip hidden/temp files");
    }

    #[test]
    fn test_unsupported_extension() {
        let tmp = tempfile::TempDir::new().unwrap();
        let kb_path = tmp.path().join("test.db");
        let store = KnowledgeStore::open(&kb_path).unwrap();

        let watch_dir = tmp.path().join("knowledge");
        std::fs::create_dir_all(&watch_dir).unwrap();
        std::fs::write(watch_dir.join("image.png"), "fake png").unwrap();

        let watcher = FolderWatcher::new(&watch_dir);
        let results = watcher.scan_and_ingest(&store);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, IngestStatus::Unsupported);
    }

    #[test]
    fn test_folder_summary() {
        let tmp = tempfile::TempDir::new().unwrap();
        let watch_dir = tmp.path().join("knowledge");
        std::fs::create_dir_all(&watch_dir).unwrap();
        std::fs::write(watch_dir.join("doc.md"), "hello").unwrap();
        std::fs::write(watch_dir.join("data.csv"), "a,b,c").unwrap();
        std::fs::write(watch_dir.join("image.png"), "fake").unwrap();

        let watcher = FolderWatcher::new(&watch_dir);
        let summary = watcher.summary();
        assert!(summary.exists);
        assert_eq!(summary.total_files, 3);
        assert_eq!(summary.supported_files, 2); // md + csv, not png
    }
}
