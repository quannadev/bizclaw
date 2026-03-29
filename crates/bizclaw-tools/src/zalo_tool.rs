//! Zalo Power Tool — AI Agent tool for Zalo automation.
//!
//! Provides unified interface for the AI agent to interact with Zalo:
//! - List/manage groups and friends
//! - Monitor selected groups/DMs for summarization
//! - Send messages to groups or individuals
//! - Send summary reports to designated contacts
//! - Manage watch lists (which groups/DMs to monitor)

use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Tool;

/// Safely truncate a string at a character boundary (UTF-8 safe).
fn truncate_safe(s: &str, max_chars: usize) -> String {
    let truncated: String = s.chars().take(max_chars).collect();
    if truncated.len() < s.len() {
        format!("{}...", truncated)
    } else {
        truncated
    }
}
use bizclaw_core::types::{ToolDefinition, ToolResult};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::group_summarizer::{BufferedMessage, MessageBuffer, SummarizerConfig};

// ── Watch List Configuration ──────────────────────────────

/// Which groups/DMs to monitor and who receives reports.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZaloWatchConfig {
    /// Group IDs to monitor for summarization
    pub watched_groups: HashSet<String>,
    /// User IDs (DMs) to monitor for summarization
    pub watched_users: HashSet<String>,
    /// Report recipients — user IDs who receive daily/scheduled summaries
    pub report_recipients: Vec<ReportRecipient>,
    /// Friendly names for groups (group_id → name)
    pub group_names: HashMap<String, String>,
    /// Friendly names for users (user_id → name)
    pub user_names: HashMap<String, String>,
}

/// A person who receives summary reports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportRecipient {
    pub user_id: String,
    pub name: String,
    /// What to include: "groups", "dms", "all"
    #[serde(default = "default_report_scope")]
    pub scope: String,
}

fn default_report_scope() -> String {
    "all".into()
}

// ── DM Buffer ─────────────────────────────────────────────

/// Buffer for DM (1:1) messages — separate from group buffer.
#[derive(Debug, Clone, Default)]
pub struct DmBuffer {
    /// user_id → Vec<BufferedMessage>
    conversations: Arc<Mutex<HashMap<String, Vec<BufferedMessage>>>>,
}

impl DmBuffer {
    pub fn new() -> Self {
        Self {
            conversations: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn push(&self, user_id: &str, msg: BufferedMessage) {
        let mut convos = self.conversations.lock().unwrap_or_else(|p| p.into_inner());
        convos.entry(user_id.to_string()).or_default().push(msg);
    }

    pub fn drain_user(&self, user_id: &str) -> Vec<BufferedMessage> {
        let mut convos = self.conversations.lock().unwrap_or_else(|p| p.into_inner());
        convos.remove(user_id).unwrap_or_default()
    }

    pub fn user_ids(&self) -> Vec<String> {
        self.conversations
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .keys()
            .cloned()
            .collect()
    }

    pub fn count(&self, user_id: &str) -> usize {
        self.conversations
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(user_id)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    pub fn total_count(&self) -> usize {
        self.conversations
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .values()
            .map(|v| v.len())
            .sum()
    }

    pub fn prune(&self, max_age_secs: u64) {
        let cutoff = chrono::Utc::now() - chrono::Duration::seconds(max_age_secs as i64);
        let mut convos = self.conversations.lock().unwrap_or_else(|p| p.into_inner());
        for messages in convos.values_mut() {
            messages.retain(|m| m.timestamp > cutoff);
        }
        convos.retain(|_, v| !v.is_empty());
    }
}

// ── Zalo Power Tool ──────────────────────────────────────

/// The unified Zalo Power Tool for AI agents.
pub struct ZaloTool {
    /// Group message buffer (shared with group_summarizer)
    group_buffer: MessageBuffer,
    /// DM message buffer
    dm_buffer: DmBuffer,
    /// Watch list configuration
    config: Arc<Mutex<ZaloWatchConfig>>,
    /// Summarizer config
    summarizer_config: SummarizerConfig,
}

impl ZaloTool {
    pub fn new() -> Self {
        Self {
            group_buffer: MessageBuffer::new(),
            dm_buffer: DmBuffer::new(),
            config: Arc::new(Mutex::new(ZaloWatchConfig::default())),
            summarizer_config: SummarizerConfig::default(),
        }
    }

    /// Create with shared group buffer (connects to existing group_summarizer).
    pub fn with_group_buffer(group_buffer: MessageBuffer) -> Self {
        Self {
            group_buffer,
            dm_buffer: DmBuffer::new(),
            config: Arc::new(Mutex::new(ZaloWatchConfig::default())),
            summarizer_config: SummarizerConfig::default(),
        }
    }

    /// Get shared group buffer reference (for wiring to listener).
    pub fn group_buffer(&self) -> &MessageBuffer {
        &self.group_buffer
    }

    /// Get DM buffer reference.
    pub fn dm_buffer(&self) -> &DmBuffer {
        &self.dm_buffer
    }

    /// Get watch config.
    pub fn watch_config(&self) -> Arc<Mutex<ZaloWatchConfig>> {
        self.config.clone()
    }

    /// Check if a group is being watched.
    pub fn is_group_watched(&self, group_id: &str) -> bool {
        self.config
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .watched_groups
            .contains(group_id)
    }

    /// Check if a user DM is being watched.
    pub fn is_user_watched(&self, user_id: &str) -> bool {
        self.config
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .watched_users
            .contains(user_id)
    }

    // ── Action handlers ──────────────────────────────────

    fn handle_watch_group(&self, args: &serde_json::Value) -> String {
        let group_id = args["group_id"].as_str().unwrap_or("");
        let name = args["name"].as_str().unwrap_or(group_id);

        if group_id.is_empty() {
            return "❌ Thiếu group_id".into();
        }

        let mut cfg = self.config.lock().unwrap_or_else(|p| p.into_inner());
        cfg.watched_groups.insert(group_id.to_string());
        cfg.group_names
            .insert(group_id.to_string(), name.to_string());

        format!(
            "✅ Đã thêm nhóm \"{}\" ({}) vào danh sách theo dõi.\n📊 Tổng nhóm đang theo dõi: {}",
            name,
            group_id,
            cfg.watched_groups.len()
        )
    }

    fn handle_unwatch_group(&self, args: &serde_json::Value) -> String {
        let group_id = args["group_id"].as_str().unwrap_or("");
        if group_id.is_empty() {
            return "❌ Thiếu group_id".into();
        }

        let mut cfg = self.config.lock().unwrap_or_else(|p| p.into_inner());
        cfg.watched_groups.remove(group_id);
        let name = cfg.group_names.remove(group_id).unwrap_or_default();

        format!(
            "✅ Đã bỏ nhóm \"{}\" ({}) khỏi danh sách theo dõi.",
            if name.is_empty() { group_id } else { &name },
            group_id
        )
    }

    fn handle_watch_user(&self, args: &serde_json::Value) -> String {
        let user_id = args["user_id"].as_str().unwrap_or("");
        let name = args["name"].as_str().unwrap_or(user_id);

        if user_id.is_empty() {
            return "❌ Thiếu user_id".into();
        }

        let mut cfg = self.config.lock().unwrap_or_else(|p| p.into_inner());
        cfg.watched_users.insert(user_id.to_string());
        cfg.user_names.insert(user_id.to_string(), name.to_string());

        format!(
            "✅ Đã thêm \"{}\" ({}) vào danh sách theo dõi DM.\n📊 Tổng người đang theo dõi: {}",
            name,
            user_id,
            cfg.watched_users.len()
        )
    }

    fn handle_unwatch_user(&self, args: &serde_json::Value) -> String {
        let user_id = args["user_id"].as_str().unwrap_or("");
        if user_id.is_empty() {
            return "❌ Thiếu user_id".into();
        }

        let mut cfg = self.config.lock().unwrap_or_else(|p| p.into_inner());
        cfg.watched_users.remove(user_id);
        let name = cfg.user_names.remove(user_id).unwrap_or_default();

        format!(
            "✅ Đã bỏ \"{}\" ({}) khỏi danh sách theo dõi DM.",
            if name.is_empty() { user_id } else { &name },
            user_id
        )
    }

    fn handle_set_report_recipient(&self, args: &serde_json::Value) -> String {
        let user_id = args["user_id"].as_str().unwrap_or("");
        let name = args["name"].as_str().unwrap_or(user_id);
        let scope = args["scope"].as_str().unwrap_or("all");

        if user_id.is_empty() {
            return "❌ Thiếu user_id để nhận báo cáo".into();
        }

        let mut cfg = self.config.lock().unwrap_or_else(|p| p.into_inner());
        // Remove existing entry for this user_id if any
        cfg.report_recipients.retain(|r| r.user_id != user_id);
        cfg.report_recipients.push(ReportRecipient {
            user_id: user_id.to_string(),
            name: name.to_string(),
            scope: scope.to_string(),
        });

        format!(
            "✅ Đã thiết lập \"{}\" ({}) nhận báo cáo (scope: {}).\n📨 Tổng người nhận: {}",
            name,
            user_id,
            scope,
            cfg.report_recipients.len()
        )
    }

    fn handle_list_watched(&self) -> String {
        let cfg = self.config.lock().unwrap_or_else(|p| p.into_inner());

        let mut out = String::new();

        // Groups
        out.push_str(&format!(
            "📋 **Nhóm đang theo dõi** ({}):\n",
            cfg.watched_groups.len()
        ));
        if cfg.watched_groups.is_empty() {
            out.push_str("  (chưa có nhóm nào)\n");
        } else {
            for gid in &cfg.watched_groups {
                let name = cfg.group_names.get(gid).map(|s| s.as_str()).unwrap_or("?");
                let count = self.group_buffer.count(gid);
                out.push_str(&format!(
                    "  • {} ({}): {} tin đang buffer\n",
                    name, gid, count
                ));
            }
        }

        // Users (DMs)
        out.push_str(&format!(
            "\n👤 **Người đang theo dõi DM** ({}):\n",
            cfg.watched_users.len()
        ));
        if cfg.watched_users.is_empty() {
            out.push_str("  (chưa có người nào)\n");
        } else {
            for uid in &cfg.watched_users {
                let name = cfg.user_names.get(uid).map(|s| s.as_str()).unwrap_or("?");
                let count = self.dm_buffer.count(uid);
                out.push_str(&format!(
                    "  • {} ({}): {} tin đang buffer\n",
                    name, uid, count
                ));
            }
        }

        // Report recipients
        out.push_str(&format!(
            "\n📨 **Người nhận báo cáo** ({}):\n",
            cfg.report_recipients.len()
        ));
        if cfg.report_recipients.is_empty() {
            out.push_str("  (chưa thiết lập)\n");
        } else {
            for r in &cfg.report_recipients {
                out.push_str(&format!(
                    "  • {} ({}) — scope: {}\n",
                    r.name, r.user_id, r.scope
                ));
            }
        }

        out
    }

    fn handle_summarize_group(&self, args: &serde_json::Value) -> String {
        let group_id = args["group_id"].as_str().unwrap_or("");
        if group_id.is_empty() {
            return "❌ Thiếu group_id".into();
        }

        let messages = self.group_buffer.drain_group(group_id);
        if messages.is_empty() {
            let cfg = self.config.lock().unwrap_or_else(|p| p.into_inner());
            let name = cfg
                .group_names
                .get(group_id)
                .map(|s| s.as_str())
                .unwrap_or(group_id);
            return format!("📭 Nhóm \"{}\" không có tin nhắn nào trong buffer.", name);
        }

        let group_name = messages
            .first()
            .map(|m| m.group_name.as_str())
            .unwrap_or(group_id);

        let prompt = self.format_summary_prompt(&messages, group_name, "group");

        format!(
            "📊 Đã thu thập {} tin nhắn từ nhóm \"{}\". Hãy tóm tắt:\n\n{}",
            messages.len(),
            group_name,
            prompt
        )
    }

    fn handle_summarize_dm(&self, args: &serde_json::Value) -> String {
        let user_id = args["user_id"].as_str().unwrap_or("");
        if user_id.is_empty() {
            return "❌ Thiếu user_id".into();
        }

        let messages = self.dm_buffer.drain_user(user_id);
        if messages.is_empty() {
            let cfg = self.config.lock().unwrap_or_else(|p| p.into_inner());
            let name = cfg
                .user_names
                .get(user_id)
                .map(|s| s.as_str())
                .unwrap_or(user_id);
            return format!(
                "📭 Cuộc trò chuyện với \"{}\" không có tin nhắn trong buffer.",
                name
            );
        }

        let cfg = self.config.lock().unwrap_or_else(|p| p.into_inner());
        let user_name = cfg
            .user_names
            .get(user_id)
            .map(|s| s.as_str())
            .unwrap_or(user_id);

        let prompt = self.format_summary_prompt(&messages, user_name, "dm");

        format!(
            "📊 Đã thu thập {} tin nhắn từ cuộc trò chuyện với \"{}\". Hãy tóm tắt:\n\n{}",
            messages.len(),
            user_name,
            prompt
        )
    }

    fn handle_summarize_all(&self) -> String {
        let cfg = self.config.lock().unwrap_or_else(|p| p.into_inner());
        let mut report = String::new();
        let mut total_msgs = 0u32;

        // Summarize watched groups
        report.push_str("═══ BÁO CÁO TỔNG HỢP ZALO ═══\n\n");

        if !cfg.watched_groups.is_empty() {
            report.push_str("📋 NHÓM:\n\n");
            for gid in &cfg.watched_groups {
                let name = cfg.group_names.get(gid).map(|s| s.as_str()).unwrap_or(gid);
                let count = self.group_buffer.count(gid);
                if count > 0 {
                    total_msgs += count as u32;
                    report.push_str(&format!(
                        "  📌 {} — {} tin nhắn (cần tóm tắt)\n",
                        name, count
                    ));
                } else {
                    report.push_str(&format!("  ✅ {} — không có tin mới\n", name));
                }
            }
        }

        // Summarize watched DMs
        if !cfg.watched_users.is_empty() {
            report.push_str("\n👤 TIN NHẮN CÁ NHÂN:\n\n");
            for uid in &cfg.watched_users {
                let name = cfg.user_names.get(uid).map(|s| s.as_str()).unwrap_or(uid);
                let count = self.dm_buffer.count(uid);
                if count > 0 {
                    total_msgs += count as u32;
                    report.push_str(&format!(
                        "  📌 {} — {} tin nhắn (cần tóm tắt)\n",
                        name, count
                    ));
                } else {
                    report.push_str(&format!("  ✅ {} — không có tin mới\n", name));
                }
            }
        }

        report.push_str(&format!(
            "\n📊 Tổng: {} tin nhắn mới cần xử lý\n",
            total_msgs
        ));

        // List report recipients
        if !cfg.report_recipients.is_empty() {
            report.push_str("\n📨 Sau khi tóm tắt, gửi báo cáo tới:\n");
            for r in &cfg.report_recipients {
                report.push_str(&format!("  → {} ({})\n", r.name, r.user_id));
            }
        }

        report
    }

    fn handle_send_message(&self, args: &serde_json::Value) -> String {
        let thread_id = args["thread_id"].as_str().unwrap_or("");
        let content = args["content"].as_str().unwrap_or("");
        let thread_type = args["thread_type"].as_str().unwrap_or("user");

        if thread_id.is_empty() || content.is_empty() {
            return "❌ Thiếu thread_id hoặc content".into();
        }

        // The actual sending is done via the Zalo channel (ZaloMessaging).
        // This tool returns a structured request that the Agent framework
        // will route to the active Zalo channel connection.
        let request = serde_json::json!({
            "_zalo_action": "send_message",
            "thread_id": thread_id,
            "thread_type": thread_type,
            "content": content,
        });

        format!(
            "📤 Đã tạo lệnh gửi tin nhắn:\n\
             • Đến: {} ({})\n\
             • Nội dung: {}\n\n\
             🔄 Hệ thống sẽ gửi qua Zalo channel.\n\
             _zalo_request: {}",
            thread_id,
            thread_type,
            if content.len() > 100 {
                truncate_safe(content, 100)
            } else {
                content.to_string()
            },
            request
        )
    }

    fn handle_send_bank_card(&self, args: &serde_json::Value) -> String {
        let thread_id = args["thread_id"].as_str().unwrap_or("");
        let thread_type = args["thread_type"].as_str().unwrap_or("user");
        let bin_bank = args["bin_bank"].as_str().unwrap_or("");
        let acc_num = args["acc_num"].as_str().unwrap_or("");
        let acc_name = args["acc_name"].as_str().unwrap_or("");

        if thread_id.is_empty() || bin_bank.is_empty() || acc_num.is_empty() {
            return "❌ Thiếu thread_id, bin_bank hoặc acc_num".into();
        }

        let request = serde_json::json!({
            "_zalo_action": "send_bank_card",
            "thread_id": thread_id,
            "thread_type": thread_type,
            "bin_bank": bin_bank,
            "acc_num": acc_num,
            "acc_name": acc_name,
        });

        format!(
            "💳 Đã tạo lệnh gửi thẻ ngân hàng:\n\
             • Ngân hàng (BIN): {}\n\
             • STK: {}\n\
             • Tên: {}\n\n\
             🔄 Hệ thống sẽ gửi qua Zalo channel.\n\
             _zalo_request: {}",
            bin_bank, acc_num, acc_name, request
        )
    }

    fn handle_send_qr_transfer(&self, args: &serde_json::Value) -> String {
        let thread_id = args["thread_id"].as_str().unwrap_or("");
        let thread_type = args["thread_type"].as_str().unwrap_or("user");
        let bin_bank = args["bin_bank"].as_str().unwrap_or("");
        let acc_num = args["acc_num"].as_str().unwrap_or("");
        let amount = args["amount"].as_u64();
        let qr_template = args["qr_template"].as_str().unwrap_or("compact");
        let content = args["content"].as_str().unwrap_or("");

        if thread_id.is_empty() || bin_bank.is_empty() || acc_num.is_empty() {
            return "❌ Thiếu thread_id, bin_bank hoặc acc_num".into();
        }

        let request = serde_json::json!({
            "_zalo_action": "send_qr_transfer",
            "thread_id": thread_id,
            "thread_type": thread_type,
            "bin_bank": bin_bank,
            "acc_num": acc_num,
            "amount": amount,
            "content": content,
            "qr_template": qr_template,
        });

        format!(
            "💸 Đã tạo lệnh gửi QR chuyển khoản (VietQR):\n\
             • Ngân hàng (BIN): {}\n\
             • STK: {}\n\
             • Số tiền (tuỳ chọn): {:?}\n\n\
             🔄 Hệ thống sẽ gửi qua Zalo channel.\n\
             _zalo_request: {}",
            bin_bank, acc_num, amount, request
        )
    }

    fn handle_send_friend_request(&self, args: &serde_json::Value) -> String {
        let user_id = args["user_id"].as_str().unwrap_or("");
        let message = args["message"]
            .as_str()
            .unwrap_or("Xin chào! Tôi muốn kết bạn.");

        if user_id.is_empty() {
            return "❌ Thiếu user_id".into();
        }

        let request = serde_json::json!({
            "_zalo_action": "send_friend_request",
            "user_id": user_id,
            "message": message,
        });

        format!(
            "🤝 Đã tạo lệnh kết bạn:\n\
             • Người nhận: {}\n\
             • Lời nhắn: \"{}\"\n\n\
             🔄 Hệ thống sẽ gửi qua Zalo channel.\n\
             _zalo_request: {}",
            user_id, message, request
        )
    }

    fn handle_send_report(&self, args: &serde_json::Value) -> String {
        let report_content = args["content"].as_str().unwrap_or("");
        let recipient_id = args["recipient_id"].as_str();

        if report_content.is_empty() {
            return "❌ Thiếu nội dung báo cáo (content)".into();
        }

        let cfg = self.config.lock().unwrap_or_else(|p| p.into_inner());

        // If specific recipient, send to them only
        let targets: Vec<(String, String)> = if let Some(rid) = recipient_id {
            vec![(
                rid.to_string(),
                cfg.user_names
                    .get(rid)
                    .cloned()
                    .unwrap_or_else(|| rid.to_string()),
            )]
        } else {
            // Send to all report recipients
            cfg.report_recipients
                .iter()
                .map(|r| (r.user_id.clone(), r.name.clone()))
                .collect()
        };

        if targets.is_empty() {
            return "❌ Chưa thiết lập người nhận báo cáo. \
                    Dùng action='set_report_recipient' trước."
                .into();
        }

        let mut result = format!("📨 Gửi báo cáo tới {} người:\n\n", targets.len());
        for (uid, name) in &targets {
            let request = serde_json::json!({
                "_zalo_action": "send_message",
                "thread_id": uid,
                "thread_type": "user",
                "content": report_content,
            });
            result.push_str(&format!(
                "  → {} ({})\n  _zalo_request: {}\n\n",
                name, uid, request
            ));
        }

        result
    }

    fn handle_buffer_status(&self) -> String {
        let cfg = self.config.lock().unwrap_or_else(|p| p.into_inner());
        let group_total = self.group_buffer.total_count();
        let dm_total = self.dm_buffer.total_count();
        let groups_watched = cfg.watched_groups.len();
        let users_watched = cfg.watched_users.len();
        let recipients = cfg.report_recipients.len();

        format!(
            "📊 **Trạng thái Zalo Tool**\n\n\
             🔄 Buffer:\n\
             • Nhóm: {} tin nhắn (từ {} nhóm đang theo dõi)\n\
             • DM: {} tin nhắn (từ {} người đang theo dõi)\n\n\
             📨 Người nhận báo cáo: {}\n\
             ⏰ Cửa sổ buffer: {}s\n\
             📝 Kiểu tóm tắt: {}",
            group_total,
            groups_watched,
            dm_total,
            users_watched,
            recipients,
            self.summarizer_config.buffer_window_secs,
            self.summarizer_config.summary_style
        )
    }

    // ── Prompt Builder ───────────────────────────────────

    fn format_summary_prompt(
        &self,
        messages: &[BufferedMessage],
        context_name: &str,
        context_type: &str,
    ) -> String {
        let context_label = if context_type == "group" {
            format!("nhóm \"{}\"", context_name)
        } else {
            format!("cuộc trò chuyện với \"{}\"", context_name)
        };

        let style_instruction = match self.summarizer_config.summary_style.as_str() {
            "brief" => "Tóm tắt ngắn gọn trong 2-3 câu.",
            "detailed" => "Tóm tắt chi tiết, nêu rõ ai nói gì, chủ đề chính.",
            _ => "Tóm tắt dạng bullet points, mỗi chủ đề 1 gạch đầu dòng.",
        };

        let mut prompt = format!(
            "Bạn là trợ lý AI tóm tắt tin nhắn. \
             Hãy tóm tắt các tin nhắn sau đây từ {context_label} bằng tiếng Việt.\n\
             {style_instruction}\n\n\
             Chú ý:\n\
             - Gộp các chủ đề liên quan\n\
             - Highlight quyết định quan trọng\n\
             - Bỏ qua tin nhắn không quan trọng (sticker, OK, ...)\n\
             - Nêu rõ ai đề xuất/quyết định gì\n\n\
             --- TIN NHẮN ---\n"
        );

        for msg in messages
            .iter()
            .take(self.summarizer_config.max_messages_per_group)
        {
            let time = msg.timestamp.format("%H:%M");
            prompt.push_str(&format!("[{time}] {}: {}\n", msg.sender_name, msg.content));
        }

        prompt.push_str("--- HẾT TIN NHẮN ---\n\nTÓM TẮT:");
        prompt
    }
}

impl Default for ZaloTool {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tool Implementation ──────────────────────────────────

#[async_trait]
impl Tool for ZaloTool {
    fn name(&self) -> &str {
        "zalo_tool"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "zalo_tool".into(),
            description: "Công cụ Zalo toàn diện — quản lý nhóm, bạn bè, theo dõi tin nhắn, \
                           tóm tắt nội dung, và gửi báo cáo. Hỗ trợ chọn nhóm/người cụ thể \
                           để theo dõi và gửi tóm tắt tự động."
                .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": [
                            "watch_group",
                            "unwatch_group",
                            "watch_user",
                            "unwatch_user",
                            "set_report_recipient",
                            "list_watched",
                            "summarize_group",
                            "summarize_dm",
                            "summarize_all",
                            "send_message",
                            "send_bank_card",
                            "send_qr_transfer",
                            "send_friend_request",
                            "send_report",
                            "buffer_status"
                        ],
                        "description": "Hành động:\n\
                            • watch_group — Thêm nhóm vào danh sách theo dõi\n\
                            • unwatch_group — Bỏ nhóm khỏi danh sách\n\
                            • watch_user — Thêm người vào danh sách theo dõi DM\n\
                            • unwatch_user — Bỏ người khỏi danh sách\n\
                            • set_report_recipient — Chỉ định người nhận báo cáo\n\
                            • list_watched — Xem danh sách đang theo dõi\n\
                            • summarize_group — Tóm tắt 1 nhóm cụ thể\n\
                            • summarize_dm — Tóm tắt DM với 1 người cụ thể\n\
                            • summarize_all — Tổng hợp tất cả nhóm/DM đang theo dõi\n\
                            • send_message — Gửi tin nhắn tới nhóm/người\n\
                            • send_bank_card — Gửi thẻ ngân hàng tới nhóm/người\n\
                            • send_qr_transfer — Tạo và gửi mã QR chuyển khoản VietQR\n\
                            • send_friend_request — Gửi lời kết bạn\n\
                            • send_report — Gửi báo cáo tóm tắt tới người chỉ định\n\
                            • buffer_status — Xem trạng thái buffer"
                    },
                    "group_id": {
                        "type": "string",
                        "description": "ID nhóm Zalo (dùng cho watch_group, unwatch_group, summarize_group)"
                    },
                    "user_id": {
                        "type": "string",
                        "description": "ID người dùng Zalo (dùng cho watch_user, unwatch_user, summarize_dm, set_report_recipient, send_friend_request)"
                    },
                    "name": {
                        "type": "string",
                        "description": "Tên hiển thị cho nhóm/người (dùng kèm watch_group, watch_user, set_report_recipient)"
                    },
                    "thread_id": {
                        "type": "string",
                        "description": "ID cuộc trò chuyện để gửi tin (dùng cho send_message)"
                    },
                    "thread_type": {
                        "type": "string",
                        "enum": ["user", "group"],
                        "description": "Loại cuộc trò chuyện: user (DM) hoặc group"
                    },
                    "content": {
                        "type": "string",
                        "description": "Nội dung tin nhắn hoặc báo cáo"
                    },
                    "message": {
                        "type": "string",
                        "description": "Lời nhắn kết bạn (dùng cho send_friend_request)"
                    },
                    "recipient_id": {
                        "type": "string",
                        "description": "ID người nhận báo cáo cụ thể (nếu không truyền → gửi tới tất cả report_recipients)"
                    },
                    "scope": {
                        "type": "string",
                        "enum": ["groups", "dms", "all"],
                        "description": "Phạm vi báo cáo: groups, dms, hoặc all (dùng cho set_report_recipient)"
                    },
                    "bin_bank": {
                        "type": "string",
                        "description": "BIN hoặc Tên viết tắt của Ngân Hàng (ví dụ: vcb, 970436, mbbank)"
                    },
                    "acc_num": {
                        "type": "string",
                        "description": "Số tài khoản ngân hàng"
                    },
                    "acc_name": {
                        "type": "string",
                        "description": "Tên chủ tài khoản ngân hàng (viết hoa không dấu hoặc có dấu)"
                    },
                    "amount": {
                        "type": "integer",
                        "description": "Số tiền chuyển khoản (Dùng cho send_qr_transfer)"
                    },
                    "qr_template": {
                        "type": "string",
                        "enum": ["compact", "print", "qronly"],
                        "description": "Mẫu QR chuyển khoản (Dùng cho send_qr_transfer, mặc định là compact)"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: serde_json::Value = serde_json::from_str(arguments)
            .unwrap_or_else(|_| serde_json::json!({"action": "buffer_status"}));

        let action = args["action"].as_str().unwrap_or("buffer_status");

        let output = match action {
            // ── Watch/Unwatch Management ──
            "watch_group" => self.handle_watch_group(&args),
            "unwatch_group" => self.handle_unwatch_group(&args),
            "watch_user" => self.handle_watch_user(&args),
            "unwatch_user" => self.handle_unwatch_user(&args),
            "set_report_recipient" => self.handle_set_report_recipient(&args),
            "list_watched" => self.handle_list_watched(),

            // ── Summarization ──
            "summarize_group" => self.handle_summarize_group(&args),
            "summarize_dm" => self.handle_summarize_dm(&args),
            "summarize_all" => self.handle_summarize_all(),

            // ── Outbound Actions ──
            "send_message" => self.handle_send_message(&args),
            "send_bank_card" => self.handle_send_bank_card(&args),
            "send_qr_transfer" => self.handle_send_qr_transfer(&args),
            "send_friend_request" => self.handle_send_friend_request(&args),
            "send_report" => self.handle_send_report(&args),

            // ── Status ──
            "buffer_status" => self.handle_buffer_status(),

            _ => format!("❓ Action không hợp lệ: {action}. Dùng list_watched để xem hướng dẫn."),
        };

        Ok(ToolResult {
            tool_call_id: String::new(),
            output,
            success: true,
        })
    }
}
