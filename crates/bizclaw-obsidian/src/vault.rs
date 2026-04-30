//! # Obsidian Vault Management

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use walkdir::WalkDir;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct VaultConfig {
    pub vault_path: PathBuf,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            vault_path: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".bizclaw")
                .join("vault"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteMetadata {
    pub id: String,
    pub title: String,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub tags: Vec<String>,
    pub folder: String,
}

#[derive(Debug, Clone)]
pub struct Note {
    pub path: PathBuf,
    pub metadata: NoteMetadata,
    pub content: String,
}

pub struct ObsidianVault {
    config: VaultConfig,
}

impl ObsidianVault {
    pub fn new(config: VaultConfig) -> Result<Self, VaultError> {
        let vault = Self { config };
        if !vault.config.vault_path.exists() {
            std::fs::create_dir_all(&vault.config.vault_path)?;
        }
        Ok(vault)
    }

    pub fn with_default_path() -> Result<Self, VaultError> {
        Self::new(VaultConfig::default())
    }

    pub fn get_vault_path(&self) -> &Path {
        &self.config.vault_path
    }

    pub async fn create_note(
        &self,
        title: &str,
        content: &str,
        folder: &str,
    ) -> Result<Note, VaultError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        
        let fm = format!(
            "---\nid: {}\ntitle: {}\ncreated: {}\nmodified: {}\ntags: []\n---\n\n",
            id, title, now.to_rfc3339(), now.to_rfc3339()
        );
        
        let full_content = format!("{}{}", fm, content);
        
        let filename = format!("{}.md", self.sanitize_filename(title));
        let folder_path = self.config.vault_path.join(folder);
        let file_path = folder_path.join(&filename);
        
        std::fs::create_dir_all(&folder_path)?;
        std::fs::write(&file_path, &full_content)?;
        
        Ok(Note {
            path: file_path,
            metadata: NoteMetadata {
                id,
                title: title.to_string(),
                created: now,
                modified: now,
                tags: vec![],
                folder: folder.to_string(),
            },
            content: content.to_string(),
        })
    }

    pub async fn get_note(&self, note_id: &str) -> Result<Option<Note>, VaultError> {
        for entry in WalkDir::new(&self.config.vault_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
        {
            let path = entry.path();
            if let Ok(content) = std::fs::read_to_string(path) {
                if content.contains(&format!("id: {}", note_id)) {
                    let folder = path.parent()
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();
                    
                    let title = self.extract_title(&content).unwrap_or_else(|| "Untitled".to_string());
                    
                    return Ok(Some(Note {
                        path: path.to_path_buf(),
                        metadata: NoteMetadata {
                            id: note_id.to_string(),
                            title,
                            created: Utc::now(),
                            modified: Utc::now(),
                            tags: vec![],
                            folder,
                        },
                        content: self.extract_body(&content),
                    }));
                }
            }
        }
        Ok(None)
    }

    pub async fn list_notes(&self) -> Result<Vec<NoteMetadata>, VaultError> {
        let mut results = Vec::new();
        
        for entry in WalkDir::new(&self.config.vault_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
        {
            let path = entry.path();
            if let Ok(content) = std::fs::read_to_string(path) {
                let folder = path.parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                
                let title = self.extract_title(&content).unwrap_or_else(|| "Untitled".to_string());
                
                results.push(NoteMetadata {
                    id: Uuid::new_v4().to_string(),
                    title,
                    created: Utc::now(),
                    modified: Utc::now(),
                    tags: vec![],
                    folder,
                });
            }
        }
        
        Ok(results)
    }

    fn extract_title(&self, content: &str) -> Option<String> {
        for line in content.lines() {
            if line.starts_with("title:") {
                return Some(line.trim_start_matches("title:").trim().to_string());
            }
        }
        None
    }

    fn extract_body(&self, content: &str) -> String {
        if let Some(pos) = content.find("---\n\n") {
            content[pos + 4..].to_string()
        } else if let Some(pos) = content.find("---") {
            if let Some(end) = content[pos + 3..].find("---") {
                content[pos + end + 6..].to_string()
            } else {
                content.to_string()
            }
        } else {
            content.to_string()
        }
    }

    fn sanitize_filename(&self, name: &str) -> String {
        let re = regex::Regex::new(r"[^\w\s-]").unwrap();
        let sanitized = re.replace_all(name, "");
        let re2 = regex::Regex::new(r"\s+").unwrap();
        re2.replace_all(&sanitized, "-").to_lowercase()
    }
}

#[derive(Debug)]
pub enum VaultError {
    IoError(std::io::Error),
}

impl From<std::io::Error> for VaultError {
    fn from(e: std::io::Error) -> Self {
        VaultError::IoError(e)
    }
}
