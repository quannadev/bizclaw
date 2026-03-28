//! Voice Transcription + Recap Tool — VivaDicta-inspired.
//!
//! Provides the AI agent with voice processing capabilities:
//! - Transcribe audio files (WAV/M4A/OGG/MP3) via configurable providers
//! - Generate AI-powered recap/summary of transcriptions
//! - Support for Vietnamese and multi-language audio
//!
//! Transcription providers (configurable):
//! - OpenAI Whisper API (cloud, most accurate for Vietnamese)
//! - Local Whisper via CLI (offline, privacy-first)
//! - Groq Whisper (fast, low-cost)
//!
//! Designed for the Android client workflow:
//! Android records → uploads audio file → this tool transcribes → agent recaps.

use async_trait::async_trait;
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};
use serde::{Deserialize, Serialize};

/// Transcription provider selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TranscriptionProvider {
    /// OpenAI Whisper API (requires API key)
    Whisper,
    /// Groq Whisper (fast, requires Groq API key)
    Groq,
    /// Local whisper.cpp CLI (offline)
    Local,
}

impl Default for TranscriptionProvider {
    fn default() -> Self {
        Self::Whisper
    }
}

pub struct VoiceTranscribeTool;

impl Default for VoiceTranscribeTool {
    fn default() -> Self {
        Self::new()
    }
}

impl VoiceTranscribeTool {
    pub fn new() -> Self {
        Self
    }

    /// Transcribe audio via OpenAI Whisper API.
    async fn transcribe_whisper(&self, audio_path: &str, language: &str) -> Result<String> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| BizClawError::Tool("OPENAI_API_KEY not set for Whisper transcription".into()))?;

        let file_bytes = tokio::fs::read(audio_path).await
            .map_err(|e| BizClawError::tool_error("voice_transcribe", e))?;

        let file_name = std::path::Path::new(audio_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("audio.wav")
            .to_string();

        let file_part = reqwest::multipart::Part::bytes(file_bytes)
            .file_name(file_name)
            .mime_str("audio/wav")
            .map_err(|e| BizClawError::tool_error("voice_transcribe", e))?;

        let mut form = reqwest::multipart::Form::new()
            .part("file", file_part)
            .text("model", "whisper-1")
            .text("response_format", "text");

        if !language.is_empty() && language != "auto" {
            form = form.text("language", language.to_string());
        }

        let client = reqwest::Client::new();
        let response = client
            .post("https://api.openai.com/v1/audio/transcriptions")
            .bearer_auth(&api_key)
            .multipart(form)
            .timeout(std::time::Duration::from_secs(120))
            .send()
            .await
            .map_err(|e| BizClawError::tool_error("voice_transcribe", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("[voice_transcribe] Whisper API error {status}: {body}");
            return Err(BizClawError::Tool(format!("Whisper API error: {status}")));
        }

        response.text().await
            .map_err(|e| BizClawError::tool_error("voice_transcribe", e))
    }

    /// Transcribe via Groq Whisper API (faster, cheaper).
    async fn transcribe_groq(&self, audio_path: &str, language: &str) -> Result<String> {
        let api_key = std::env::var("GROQ_API_KEY")
            .map_err(|_| BizClawError::Tool("GROQ_API_KEY not set for Groq transcription".into()))?;

        let file_bytes = tokio::fs::read(audio_path).await
            .map_err(|e| BizClawError::tool_error("voice_transcribe", e))?;

        let file_name = std::path::Path::new(audio_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("audio.wav")
            .to_string();

        let file_part = reqwest::multipart::Part::bytes(file_bytes)
            .file_name(file_name)
            .mime_str("audio/wav")
            .map_err(|e| BizClawError::tool_error("voice_transcribe", e))?;

        let mut form = reqwest::multipart::Form::new()
            .part("file", file_part)
            .text("model", "whisper-large-v3")
            .text("response_format", "text");

        if !language.is_empty() && language != "auto" {
            form = form.text("language", language.to_string());
        }

        let client = reqwest::Client::new();
        let response = client
            .post("https://api.groq.com/openai/v1/audio/transcriptions")
            .bearer_auth(&api_key)
            .multipart(form)
            .timeout(std::time::Duration::from_secs(120))
            .send()
            .await
            .map_err(|e| BizClawError::tool_error("voice_transcribe", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("[voice_transcribe] Groq API error {status}: {body}");
            return Err(BizClawError::Tool(format!("Groq API error: {status}")));
        }

        response.text().await
            .map_err(|e| BizClawError::tool_error("voice_transcribe", e))
    }

    /// Transcribe via local whisper.cpp CLI (fully offline).
    async fn transcribe_local(&self, audio_path: &str, language: &str) -> Result<String> {
        let model_path = std::env::var("WHISPER_MODEL_PATH")
            .unwrap_or_else(|_| "models/ggml-base.bin".to_string());

        let lang_arg = if language.is_empty() || language == "auto" {
            "auto".to_string()
        } else {
            language.to_string()
        };

        let output = tokio::process::Command::new("whisper-cpp")
            .args([
                "--model", &model_path,
                "--language", &lang_arg,
                "--output-txt",
                "--no-timestamps",
                audio_path,
            ])
            .output()
            .await
            .map_err(|e| BizClawError::tool_error("voice_transcribe", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::error!("[voice_transcribe] whisper-cpp error: {stderr}");
            return Err(BizClawError::Tool("Local whisper transcription failed".into()));
        }

        // whisper-cpp writes output to <input_file>.txt
        let txt_path = format!("{audio_path}.txt");
        tokio::fs::read_to_string(&txt_path).await
            .or_else(|_| Ok(String::from_utf8_lossy(&output.stdout).to_string()))
    }
}

#[async_trait]
impl Tool for VoiceTranscribeTool {
    fn name(&self) -> &str {
        "voice_transcribe"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "voice_transcribe".into(),
            description: concat!(
                "Transcribe audio files to text and optionally generate a recap/summary. ",
                "Supports WAV, M4A, OGG, MP3 formats. ",
                "Use for voice memos, meeting recordings, phone calls, interviews. ",
                "Provider: 'whisper' (OpenAI, best for Vietnamese), 'groq' (fast), 'local' (offline)."
            ).into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "audio_path": {
                        "type": "string",
                        "description": "Path to the audio file (WAV/M4A/OGG/MP3)"
                    },
                    "provider": {
                        "type": "string",
                        "enum": ["whisper", "groq", "local"],
                        "description": "Transcription provider (default: whisper)"
                    },
                    "language": {
                        "type": "string",
                        "description": "Language code: 'vi' for Vietnamese, 'en' for English, 'auto' for auto-detect (default: auto)"
                    },
                    "action": {
                        "type": "string",
                        "enum": ["transcribe", "recap", "both"],
                        "description": "Action: 'transcribe' = raw text, 'recap' = summary only, 'both' = text + summary (default: both)"
                    }
                },
                "required": ["audio_path"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: serde_json::Value = serde_json::from_str(arguments)
            .map_err(|e| BizClawError::Tool(format!("Invalid arguments: {e}")))?;

        let audio_path = args["audio_path"].as_str()
            .ok_or_else(|| BizClawError::Tool("audio_path is required".into()))?;

        let provider = args["provider"].as_str().unwrap_or("whisper");
        let language = args["language"].as_str().unwrap_or("auto");
        let action = args["action"].as_str().unwrap_or("both");

        // Validate file exists
        if !std::path::Path::new(audio_path).exists() {
            return Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!("Error: Audio file not found: {audio_path}"),
                success: false,
            });
        }

        // Validate file extension
        let ext = std::path::Path::new(audio_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !["wav", "m4a", "ogg", "mp3", "mp4", "webm", "flac"].contains(&ext.as_str()) {
            return Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!("Error: Unsupported audio format '.{ext}'. Supported: wav, m4a, ogg, mp3, mp4, webm, flac"),
                success: false,
            });
        }

        // Get file size for info
        let file_size = tokio::fs::metadata(audio_path).await
            .map(|m| m.len())
            .unwrap_or(0);
        let size_mb = file_size as f64 / (1024.0 * 1024.0);

        tracing::info!(
            "[voice_transcribe] Processing {audio_path} ({size_mb:.1} MB) via {provider}, lang={language}"
        );

        // Transcribe
        let transcript = match provider {
            "groq" => self.transcribe_groq(audio_path, language).await?,
            "local" => self.transcribe_local(audio_path, language).await?,
            _ => self.transcribe_whisper(audio_path, language).await?,
        };

        let transcript = transcript.trim().to_string();

        if transcript.is_empty() {
            return Ok(ToolResult {
                tool_call_id: String::new(),
                output: "No speech detected in the audio file.".into(),
                success: true,
            });
        }

        // Build output based on action
        let output = match action {
            "transcribe" => {
                format!(
                    "📝 **Transcription** ({provider}, {size_mb:.1} MB)\n\n{transcript}"
                )
            }
            "recap" => {
                // Return transcript with instruction for agent to summarize
                format!(
                    "📋 **Transcript for recap** ({provider}, {size_mb:.1} MB):\n\n\
                    {transcript}\n\n\
                    ---\n\
                    Please generate a concise recap/summary of the above transcription. \
                    Highlight key points, action items, and decisions."
                )
            }
            _ => {
                // "both" — return transcript + ask agent to recap
                format!(
                    "📝 **Full Transcription** ({provider}, {size_mb:.1} MB)\n\n\
                    {transcript}\n\n\
                    ---\n\
                    📋 **Recap request**: Please also provide a concise summary with:\n\
                    - Key points discussed\n\
                    - Action items (if any)\n\
                    - Decisions made (if any)\n\
                    - Follow-up needed (if any)"
                )
            }
        };

        Ok(ToolResult {
            tool_call_id: String::new(),
            output,
            success: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_definition() {
        let tool = VoiceTranscribeTool::new();
        assert_eq!(tool.name(), "voice_transcribe");
        let def = tool.definition();
        assert!(def.description.contains("Transcribe"));
        let params = def.parameters;
        assert!(params["properties"]["audio_path"].is_object());
        assert!(params["properties"]["provider"].is_object());
        assert!(params["properties"]["language"].is_object());
        assert!(params["properties"]["action"].is_object());
    }

    #[tokio::test]
    async fn test_missing_file() {
        let tool = VoiceTranscribeTool::new();
        let result = tool.execute(r#"{"audio_path": "/nonexistent/audio.wav"}"#).await.unwrap();
        assert!(!result.success);
        assert!(result.output.contains("not found"));
    }

    #[tokio::test]
    async fn test_unsupported_format() {
        // Create a temp file with unsupported extension
        let tmp = "/tmp/bizclaw_test_voice.xyz";
        tokio::fs::write(tmp, b"fake audio").await.unwrap();
        let tool = VoiceTranscribeTool::new();
        let result = tool.execute(&format!(r#"{{"audio_path": "{tmp}"}}"#)).await.unwrap();
        assert!(!result.success);
        assert!(result.output.contains("Unsupported"));
        let _ = tokio::fs::remove_file(tmp).await;
    }

    #[test]
    fn test_default_provider() {
        let provider = TranscriptionProvider::default();
        assert!(matches!(provider, TranscriptionProvider::Whisper));
    }
}
