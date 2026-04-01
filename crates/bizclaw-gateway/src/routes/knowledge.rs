//! Knowledge Base API route handlers.
//!
//! Extracted from routes/mod.rs to reduce god-file complexity.

use axum::{Json, extract::State};
use std::sync::Arc;

use crate::server::AppState;

// ---- Knowledge Base API ----

/// Search the knowledge base with optional filters.
/// Accepts: query, limit, filters: { doc_names, mimetypes, owners, score_threshold }
pub async fn knowledge_search(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let query = body["query"].as_str().unwrap_or("");
    let limit = body["limit"].as_u64().unwrap_or(5) as usize;

    // Parse optional filters
    let filter = if let Some(filters) = body.get("filters") {
        serde_json::from_value::<bizclaw_knowledge::SearchFilter>(filters.clone())
            .unwrap_or_default()
    } else {
        bizclaw_knowledge::SearchFilter::default()
    };

    let kb = state.knowledge.lock().await;
    match kb.as_ref() {
        Some(store) => {
            let results = store.search_filtered(query, limit, &filter);
            let items: Vec<_> = results
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "doc_name": r.doc_name,
                        "content": r.content,
                        "score": r.score,
                        "chunk_idx": r.chunk_idx,
                        "mimetype": r.mimetype,
                        "owner": r.owner,
                    })
                })
                .collect();
            Json(serde_json::json!({"ok": true, "results": items, "count": items.len()}))
        }
        None => Json(serde_json::json!({"ok": false, "error": "Knowledge base not available"})),
    }
}

/// List all knowledge documents with full metadata.
pub async fn knowledge_list_docs(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let kb = state.knowledge.lock().await;
    match kb.as_ref() {
        Some(store) => {
            let docs: Vec<_> = store
                .list_documents()
                .iter()
                .map(|d| {
                    serde_json::json!({
                        "id": d.id, "name": d.name, "source": d.source,
                        "chunks": d.chunk_count, "mimetype": d.mimetype,
                        "owner": d.owner, "file_size": d.file_size,
                        "created_at": d.created_at,
                    })
                })
                .collect();
            let (total_docs, total_chunks) = store.stats();
            Json(serde_json::json!({
                "ok": true, "documents": docs,
                "total_docs": total_docs, "total_chunks": total_chunks
            }))
        }
        None => Json(serde_json::json!({"ok": false, "error": "Knowledge base not available"})),
    }
}

/// Get detailed knowledge base statistics.
pub async fn knowledge_stats(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let kb = state.knowledge.lock().await;
    match kb.as_ref() {
        Some(store) => {
            let stats = store.detailed_stats();
            Json(serde_json::json!({"ok": true, "stats": stats}))
        }
        None => Json(serde_json::json!({"ok": false, "error": "Knowledge base not available"})),
    }
}

/// Get proactive nudges/suggestions based on a message.
/// Searches the KB and generates contextual suggestions.
pub async fn knowledge_nudges(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let message = body["message"].as_str().unwrap_or("");
    let context = body["context"].as_str();

    let kb = state.knowledge.lock().await;
    match kb.as_ref() {
        Some(store) => {
            let results = store.search(message, 5);
            let mut engine =
                bizclaw_knowledge::NudgeEngine::new(bizclaw_knowledge::NudgeConfig::default());
            let nudges = engine.generate_nudges(message, &results, context);
            let items: Vec<_> = nudges
                .iter()
                .map(|n| {
                    serde_json::json!({
                        "id": n.id,
                        "text": n.text,
                        "category": n.category,
                        "relevance": n.relevance,
                        "source_doc": n.source_doc,
                        "action": n.action,
                    })
                })
                .collect();
            Json(serde_json::json!({"ok": true, "nudges": items, "count": items.len()}))
        }
        None => Json(serde_json::json!({"ok": false, "error": "Knowledge base not available"})),
    }
}

/// MCP tool listing for the knowledge base.
/// Returns tool definitions that can be consumed by MCP-compatible agents.
pub async fn knowledge_mcp_tools() -> Json<serde_json::Value> {
    let tools = bizclaw_knowledge::mcp_server::knowledge_tools();
    let tool_defs: Vec<_> = tools
        .iter()
        .map(|t| {
            serde_json::json!({
                "name": t.name,
                "description": t.description,
                "inputSchema": t.input_schema,
            })
        })
        .collect();
    Json(serde_json::json!({"ok": true, "tools": tool_defs}))
}

/// MCP tool call handler — execute a knowledge base tool.
pub async fn knowledge_mcp_call(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let tool_name = body["name"].as_str().unwrap_or("");
    let arguments = body
        .get("arguments")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    let kb = state.knowledge.lock().await;
    match kb.as_ref() {
        Some(store) => {
            let call = bizclaw_knowledge::McpToolCall {
                name: tool_name.to_string(),
                arguments,
            };
            let mut engine =
                bizclaw_knowledge::NudgeEngine::new(bizclaw_knowledge::NudgeConfig::default());
            let response =
                bizclaw_knowledge::mcp_server::handle_tool_call(store, &mut engine, &call);
            Json(serde_json::json!({
                "ok": true,
                "content": response.content,
                "isError": response.is_error,
            }))
        }
        None => Json(serde_json::json!({"ok": false, "error": "Knowledge base not available"})),
    }
}

/// Scan the knowledge folder for new files and auto-ingest.
pub async fn knowledge_watch_scan(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let kb = state.knowledge.lock().await;
    match kb.as_ref() {
        Some(store) => {
            let watcher = bizclaw_knowledge::FolderWatcher::default_folder();
            let results = watcher.scan_and_ingest(store);
            let added = results
                .iter()
                .filter(|r| r.status == bizclaw_knowledge::watcher::IngestStatus::Added)
                .count();
            let items: Vec<_> = results
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "filename": r.filename,
                        "status": r.status,
                        "chunks": r.chunks,
                        "file_size": r.file_size,
                        "message": r.message,
                    })
                })
                .collect();
            let summary = watcher.summary();
            Json(serde_json::json!({
                "ok": true,
                "results": items,
                "added": added,
                "folder": summary,
            }))
        }
        None => Json(serde_json::json!({"ok": false, "error": "Knowledge base not available"})),
    }
}

/// Get interaction signal statistics.
pub async fn knowledge_signal_stats() -> Json<serde_json::Value> {
    let data_dir = std::env::var("HOME")
        .map(|h| std::path::PathBuf::from(h).join(".local/share"))
        .unwrap_or_else(|_| std::path::PathBuf::from("data"));
    let signal_path = data_dir.join("bizclaw").join("signals.db");
    match bizclaw_knowledge::SignalLogger::open(&signal_path) {
        Ok(logger) => {
            let stats = logger.stats(None);
            Json(serde_json::json!({"ok": true, "stats": stats}))
        }
        Err(e) => Json(serde_json::json!({"ok": false, "error": e})),
    }
}

/// Log interaction feedback (next-state signal).
pub async fn knowledge_signal_feedback(
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let data_dir = std::env::var("HOME")
        .map(|h| std::path::PathBuf::from(h).join(".local/share"))
        .unwrap_or_else(|_| std::path::PathBuf::from("data"));
    let signal_dir = data_dir.join("bizclaw");
    let _ = std::fs::create_dir_all(&signal_dir);
    let signal_path = signal_dir.join("signals.db");

    match bizclaw_knowledge::SignalLogger::open(&signal_path) {
        Ok(logger) => {
            let user_message = body["user_message"].as_str().unwrap_or("");
            let agent_response = body["agent_response"].as_str().unwrap_or("");
            let feedback_text = body["feedback"].as_str().unwrap_or("");
            let agent_name = body["agent_name"].as_str().unwrap_or("default");
            let session_id = body["session_id"].as_str().unwrap_or("unknown");

            // Auto-detect signal type from feedback
            let (signal_type, reward) =
                bizclaw_knowledge::SignalLogger::detect_signal(feedback_text);

            let signal = bizclaw_knowledge::InteractionSignal {
                id: uuid::Uuid::new_v4().to_string(),
                agent_name: agent_name.to_string(),
                session_id: session_id.to_string(),
                signal_type,
                user_message: user_message.to_string(),
                agent_response: agent_response.to_string(),
                feedback: if feedback_text.is_empty() {
                    None
                } else {
                    Some(feedback_text.to_string())
                },
                reward,
                created_at: chrono::Utc::now().to_rfc3339(),
            };

            match logger.log(&signal) {
                Ok(()) => Json(
                    serde_json::json!({"ok": true, "reward": reward, "signal_type": signal.signal_type}),
                ),
                Err(e) => Json(serde_json::json!({"ok": false, "error": e})),
            }
        }
        Err(e) => Json(serde_json::json!({"ok": false, "error": e})),
    }
}

/// Add a document to the knowledge base.
pub async fn knowledge_add_doc(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let name = body["name"].as_str().unwrap_or("unnamed.txt");
    let content = body["content"].as_str().unwrap_or("");
    let source = body["source"].as_str().unwrap_or("api");
    let owner = body["owner"].as_str().unwrap_or("");

    let kb = state.knowledge.lock().await;
    match kb.as_ref() {
        Some(store) => {
            match store.add_document_with_meta(name, content, source, owner, content.len()) {
                Ok(chunks) => Json(serde_json::json!({"ok": true, "chunks": chunks})),
                Err(e) => Json(serde_json::json!({"ok": false, "error": e})),
            }
        }
        None => Json(serde_json::json!({"ok": false, "error": "Knowledge base not available"})),
    }
}

/// Remove a document from the knowledge base.
pub async fn knowledge_remove_doc(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Json<serde_json::Value> {
    let kb = state.knowledge.lock().await;
    match kb.as_ref() {
        Some(store) => match store.remove_document(id) {
            Ok(()) => Json(serde_json::json!({"ok": true})),
            Err(e) => Json(serde_json::json!({"ok": false, "error": e})),
        },
        None => Json(serde_json::json!({"ok": false, "error": "Knowledge base not available"})),
    }
}

/// Upload a file (PDF, TXT, MD, etc.) to the knowledge base.
/// Accepts multipart/form-data with a "file" field.
/// PDFs are processed via pdf_oxide for text/markdown extraction.
pub async fn knowledge_upload_file(
    State(state): State<Arc<AppState>>,
    mut multipart: axum::extract::Multipart,
) -> Json<serde_json::Value> {
    let mut file_name = String::new();
    let mut file_data: Vec<u8> = Vec::new();
    let mut owner = String::new();

    // Extract file and owner from multipart
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "owner" {
            if let Ok(text) = field.text().await {
                owner = text;
            }
        } else if name == "file" {
            file_name = field.file_name().unwrap_or("unnamed.txt").to_string();
            match field.bytes().await {
                Ok(bytes) => file_data = bytes.to_vec(),
                Err(e) => {
                    return Json(serde_json::json!({
                        "ok": false,
                        "error": format!("Failed to read file: {e}")
                    }));
                }
            }
        }
    }

    if file_data.is_empty() {
        return Json(serde_json::json!({
            "ok": false,
            "error": "No file uploaded. Use multipart/form-data with field name 'file'"
        }));
    }

    let ext = file_name.rsplit('.').next().unwrap_or("txt").to_lowercase();
    let file_size = file_data.len();

    tracing::info!(
        "📤 Knowledge upload: {} ({} bytes, .{})",
        file_name,
        file_size,
        ext
    );

    let kb = state.knowledge.lock().await;
    match kb.as_ref() {
        Some(store) => {
            let result = match ext.as_str() {
                "pdf" => store.add_pdf_document_with_meta(&file_name, &file_data, "upload", &owner),
                _ => {
                    // Text-based files: convert bytes to string
                    match String::from_utf8(file_data) {
                        Ok(content) => store.add_document_with_meta(
                            &file_name, &content, "upload", &owner, file_size,
                        ),
                        Err(_) => Err("File is not valid UTF-8 text".into()),
                    }
                }
            };

            match result {
                Ok(chunks) => Json(serde_json::json!({
                    "ok": true,
                    "name": file_name,
                    "chunks": chunks,
                    "size": file_size,
                    "type": ext,
                })),
                Err(e) => Json(serde_json::json!({
                    "ok": false,
                    "error": e,
                    "name": file_name,
                })),
            }
        }
        None => Json(serde_json::json!({
            "ok": false,
            "error": "Knowledge base not available"
        })),
    }
}
