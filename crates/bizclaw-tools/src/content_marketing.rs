//! Marketing Content Skill - 14 Công thức Copywriting + 18 Tâm lý học + NLP
//! 
//! Features:
//! - 14 công thức copywriting (AIDA, PAS, FAB, 4P, v.v.)
//! - 18 hiệu ứng tâm lý học
//! - 10 kỹ thuật NLP
//! - Vietnamese content optimization
//! - Multi-channel output (Zalo, Facebook, TikTok, Shopee)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;
use serde_json::json;

/// 14 Công thức Copywriting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopywritingFormula {
    pub id: String,
    pub name: String,
    pub name_vi: String,
    pub description: String,
    pub steps: Vec<String>,
    pub use_cases: Vec<String>,
    pub tokens_budget: u32,
}

/// 18 Hiệu ứng tâm lý học  
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PsychologyEffect {
    pub id: String,
    pub name: String,
    pub name_vi: String,
    pub description: String,
    pub application: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NLPTech {
    pub id: String,
    pub name: String,
    pub name_vi: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentTone {
    pub id: String,
    pub name: String,
    pub emoji: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub max_length: u32,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentResult {
    pub formula: String,
    pub psychology: Vec<String>,
    pub tone: String,
    pub channel: String,
    pub content: String,
    pub tokens_spent: u32,
}

pub struct MarketingContentSkill {
    pub formulas: Vec<CopywritingFormula>,
    pub psychology: Vec<PsychologyEffect>,
    pub nlp_techniques: Vec<NLPTech>,
    pub tones: Vec<ContentTone>,
    pub channels: Vec<Channel>,
}

impl MarketingContentSkill {
    pub fn new() -> Self {
        Self {
            formulas: FORMULAS.clone(),
            psychology: PSYCHOLOGY.clone(),
            nlp_techniques: NLP_TECHNIQUES.clone(),
            tones: TONES.clone(),
            channels: CHANNELS.clone(),
        }
    }
    
    pub fn generate(&self, brief: &str, formula: &str, psychology_effects: &[String], tone: &str, channel: &str) -> ContentResult {
        let mut content = format!("# Marketing Content\n\n## Brief: {}\n\n", brief);
        
        // Apply formulas
        for f in &self.formulas {
            if formula == "all" || f.id == formula {
                content.push_str(&format!("### {}\n", f.name_vi));
                content.push_str(&format!("{}\n\n", f.description));
                for step in &f.steps {
                    content.push_str(&format!("- {}\n", step));
                }
            }
        }
        
        // Apply psychology effects
        if !psychology_effects.is_empty() && psychology_effects[0] != "none" {
            content.push_str("\n## Hiệu ứng tâm lý học\n");
            for p in &self.psychology {
                if psychology_effects.contains(&p.id) {
                    content.push_str(&format!("\n### {}\n", p.name_vi));
                    content.push_str(&format!("{}\n", p.description));
                }
            }
        }
        
        ContentResult {
            formula: formula.to_string(),
            psychology: psychology_effects.to_vec(),
            tone: tone.to_string(),
            channel: channel.to_string(),
            content,
            tokens_spent: 800,
        }
    }
}

// === 14 CÔNG THỨC COPYWRITING ===

pub static FORMULAS: Vec<CopywritingFormula> = vec![
    CopywritingFormula {
        id: "aida".into(),
        name: "AIDA".into(),
        name_vi: "Chú ý - Quan tâm - Mong muốn - Hành động".into(),
        description: "Kích thích chú ý → quan tâm → khát vọng → hành động".into(),
        steps: vec![
            "1. Hook (kích thích CHÚ Ý".to_string(),
            "2. Interest (tạo QUAN TÂM".to_string(),
            "3. Desire (chạm CẢM XÚC".to_string(),
            "4. Action (thúc đẩy HÀNH ĐỘNG".to_string(),
        ],
        use_cases: vec!["Facebook Ads".to_string(), "Landing page".to_string(), "Email marketing".to_string()],
        tokens_budget: 800,
    },
    CopywritingFormula {
        id: "pas".into(),
        name: "PAS".into(),
        name_vi: "Vấn đề - Kích động - Giải pháp".into(),
        description: "Problem-Agitate-Solution cho retargeting".into(),
        steps: vec![
            "1. Xác định vấn đề KH".to_string(),
            "2. Kích động cảm xúc mạnh".to_string(),
            "3. Đề xuất giải pháp".to_string(),
        ],
        use_cases: vec!["Retargeting ads".to_string(), "Email sequence".to_string()],
        tokens_budget: 600,
    },
    CopywritingFormula {
        id: "fab".into(),
        name: "FAB".into(),
        name_vi: "Đặc điểm - Lợi ích - Lợi ích cho KH".into(),
        description: "Features-Advantages-Benefits".into(),
        steps: vec![
            "1. Mô tả tính năng (Features)".to_string(),
            "2. So sánh đối thủ (Advantages)".to_string(),
            "3. Lợi ích cụ thể cho KH (Benefits)".to_string(),
        ],
        use_cases: vec!["SaaS product".into(), "Tech product".into()],
        tokens_budget: 500,
    },
    CopywritingFormula {
        id: "acc".into(),
        name: "ACC".into(),
        name_vi: "Agreement - Credibility - CTA".into(),
        description: "Xây dựng brand".into(),
        steps: vec![
            "1. Tạo Agreement".to_string(),
            "2. Xây dựng Credibility".to_string(),
            "3. Gọi hành động".to_string(),
        ],
        use_cases: vec!["Brand building".to_string(), "Trust building".to_string()],
        tokens_budget: 600,
    },
    CopyfFormula {
        id: "slap".into(),
        name: "SLAP".into(),
        name_vi: "Stop - Look - Act - Purchase".into(),
        description: "Quảng cáo ngắn".to_string(),
        steps: vec![
            "1. Stop scroll".to_string(),
            "2. Look chi tiết".to_string(),
            "3. Act ngay".to_string(),
            "4. Purchase tức".to_string(),
        ],
        use_cases: vec!["TikTok Ads".to_string(), "Social media".to_string()],
        tokens_budget: 300,
    },
    CopywritingFormula {
        id: "bab".into(),
        name: "BAB".into(),
        name_vi: "Trước - Sau - Cầu nối".into(),
        description: "Visual transformation".to_string(),
        steps: vec![
            "1. Vẽ trước khi dùng".to_string(),
            "2. Sau khi dùng".to_string(),
            "3. Cầu nối hai trạng thái".to_string(),
        ],
        use_cases: vec!["Case study".to_string(), "Testimonial".to_string()],
        tokens_budget: 500,
    },
    CopywritingFormula {
        id: "5w1h".into(),
        name: "5W1H".into(),
        name_vi: "What-Why-Who-When-Where-How".into(),
        description: "Thông tin đầy đủ".to_string(),
        steps: vec![
            "1. What - Sản phẩm gì".to_string(),
            "2. Why - Tại sao nên mua".to_string(),
            "3. Who - Cho ai".to_string(),
            "4. When - Khi nào".to_string(),
            "5. Where - Mua ở đâu".to_string(),
            "6. How - Cách dùng".to_string(),
        ],
        use_cases: vec!["Product listing".to_string(), "Chi tiết sản phẩm".to_string()],
        tokens_budget: 400,
    },
    CopywritingFormula {
        id: "storytelling".into(),
        name: "Storytelling".into(),
        name_vi: "Chuyện - Xung đột - Giải pháp - Kết quả".into(),
        description: "Kể chuyện thương hiệu".to_string(),
        steps: vec![
            "1. NHÂN VẬT chính".to_string(),
            "2. XUNG ĐỘT vấn đề".to_string(),
            "3. GIẢI PHÁP product".to_string(),
            "4. KẾT QUẢ cụ thể".to_string(),
        ],
        use_cases: vec!["Brand story".to_string(), "Case study".to_string()],
        tokens_budget: 800,
    },
    CopywritingFormula {
        id: "hook_value_cta".into(),
        name: "Hook-Value-CTAs".into(),
        name_vi: "Hook - Value - CTA".into(),
        description: "Caption ngắn TikTok/Facebook".to_string(),
        steps: vec![
            "1. Hook gây tò mò".to_string(),
            "2. Value proposition rõ ràng".to_string(),
            "3. CTA cụ thể".to_string(),
        ],
        use_cases: vec!["TikTok".into(), "Reels".into(), "Short content".into()],
        tokens_budget: 200,
    },
    CopywritingFormula {
        id: "pppp".into(),
        name: "PPPP".into(),
        name_vi: "Problem-Promise-Proof-Proposal".into(),
        description: "Thuyết phục khách hàng".into(),
        steps: vec![
            "1. Problem - Vấn đề KH".to_string(),
            "2. Promise - Hứa hẹn".to_string(),
            "3. Proof - Bằng chứng".to_string(),
            "4. Proposal - Đề xuất cụ thể".to_string(),
        ],
        use_cases: vec!["B2B".into(), "Enterprise".into()],
        tokens_budget: 700,
    },
    CopywritingFormula {
        id: "funnel".into(),
        name: "Funnel".into(),
        name_vi: "TOFU-MOFU-BOFU".into(),
        description: "Content theo funnel".into(),
        steps: vec![
            "1. TOFU - Nhận biết".into(),
            "2. MOFU - Cân nhắc".into(),
            "3. BOFU - Quyết định".into(),
        ],
        use_cases: vec!["Full funnel".into(), "Content strategy".into()],
        tokens_budget: 1000,
    },
    CopywritingFormula {
        id: "pillar_micro".into(),
        name: "Pillar-Micro".into(),
        name_vi: "1 Pillar - 10 Micro content".into(),
        description: "SEO content strategy".into(),
        steps: vec![
            "1. 1 bài pillar dài".into(),
            "2. 10 micro content từ pillar".into(),
        ],
        use_cases: vec!["SEO".into(), "Content calendar".into()],
        tokens_budget: 1500,
    },
    CopywritingFormula {
        id: "curiosity_gap".into(),
        name: "Curiosity Gap".into(),
        name_vi: "Hole - Gap - Reveal".into(),
        description: "Tạo tò mò".into(),
        steps: vec![
            "1. ĐểKH đoán tiếp theo".into(),
            "2. Tạo Gap thông tin".into(),
            "3. Reveal giải pháp".into(),
        ],
        use_cases: vec!["Teaser".into(), "Newsletter".into()],
        tokens_budget: 300,
    },
    CopywritingFormula {
        id: "social_proof".into(),
        name: "Social Proof".into(),
        name_vi: "Proof - Review - Testimonial".into(),
        description: "Xây trust".into(),
        steps: vec![
            "1. Social Proof".into(),
            "2. Review counts".into(),
            "3. Testimonials".into(),
        ],
        use_cases: vec!["Landing page".into(), "Product page".into()],
        tokens_budget: 400,
    },
];

// === 18 HIỆU ỨNG TÂM LÝ HỌC ===

pub static PSYCHOLOGY: Vec<PsychologyEffect> = vec![
    PsychologyEffect {
        id: "loss_aversion".into(),
        name: "Loss Aversion".into(),
        name_vi: "Mất đau gấp 2 lần vui".into(),
        description: "Nhấn mạnh những gì KH sẽ mất nếu không hành động".into(),
        application: "Retargeting ads, urgency messaging".into(),
    },
    PsychologyEffect {
        id: "scarcity".into(),
        name: "Scarcity".into(),
        name_vi: "Khan hiếm".into(),
        description: "Sản phẩm đang có hạn chế số lượng".into(),
        application: "Limited offer, Flash sale".into(),
    },
    PsychologyEffect {
        id: "social_proof".into(),
        name: "Social Proof".into(),
        name_vi: "Người khác đã mua".into(),
        description: "Review, testimonials, user counts".into(),
        application: "E-commerce, SaaS pricing page".into(),
    },
    PsychologyEffect {
        id: "authority".into(),
        name: "Authority".into(),
        name_vi: "Chứng nhận, giải thưởng".into(),
        description: "Xây brand positioning".into(),
        application: "Expert positioning".into(),
    },
    PsychologyEffect {
        id: "anchoring".into(),
        name: "Anchoring".into(),
        name_vi: "Neo giá trị".into(),
        description: "Giá gốc cao hơn để so sánh".into(),
        application: "Pricing, sales".into(),
    },
    PsychologyEffect {
        id: "fomo".into(),
        name: "FOMO".into(),
        name_vi: "Sợ bỏ lỡ".into(),
        description: "Người khác đang mua".into(),
        application: "Social proof real-time".into(),
    },
    PsychologyEffect {
        id: "future_pacing".into(),
        name: "Future Pacing".into(),
        name_vi: "Tưởng tượng tương lai".into(),
        application: "Vision boarding".into(),
    },
    PsychologyEffect {
        id: "curiosity_gap".into(),
        name: "Curiosity Gap".into(),
        name_vi: "Tạo tò mò".into(),
        description: "Để lại cliffhanger".into(),
        application: "Teaser content".into(),
    },
    PsychologyEffect {
        id: "decoy_effect".into(),
        name: "Decoy Effect".into(),
        name_vi: "3 gói để chọn gói giữa".into(),
        application: "Pricing page".into(),
    },
    PsychologyEffect {
        id: "reciprocity".into(),
        name: "Reciprocity".into(),
        name_vi: "Trao giá trị trước".into(),
        application: "Free ebook, checklist".into(),
    },
    PsychologyEffect {
        id: "contrast".into(),
        name: "Contrast".into(),
        name_vi: "So sánh trước/sau".into(),
        application: "Before/After landing page".into(),
    },
    PsychologyEffect {
        id: "commitment".into(),
        name: "Commitment".into(),
        name_vi: "Cam kết nhỏ trước".into(),
        application: "Email opt-in".into(),
    },
    PsychologyEffect {
        id: "cognitive_ease".into(),
        name: "Cognitive Ease".into(),
        name_vi: "Dễ dùng".into(),
        application: "Simple UX copy".into(),
    },
    PsychologyEffect {
        id: "risk_reversal".into(),
        name: "Risk Reversal".into(),
        name_vi: "Bảo hành không rủi ro".into(),
        application: "Guarantee, refund policy".into(),
    },
    PsychologyEffect {
        id: "narrative_transport".into(),
        name: "Narrative Transport".into(),
        name_vi: "Nhập tâm lý KH".into(),
        application: "Storytelling".into(),
    },
    PsychologyEffect {
        id: "pattern_interrupt".into(),
        name: "Pattern Interrupt".into(),
        name_vi: "Dừng lướt".into(),
        application: "Bold headlines".into(),
    },
    PsychologyEffect {
        id: "halo".into(),
        name: "Halo Effect".into(),
        name_vi: "Ảnh hưởng thương hiệu".into(),
        application: "Celebrity endorsement".into(),
    },
    PsychologyEffect {
        id: "specificity".into(),
        name: "Specificity".into(),
        name_vi: "Con số cụ thể".into(),
        application: "Data-driven claims".into(),
    },
];

// === 10 NLP TECHNIQUES ===

pub static NLP_TECHNIQUES: Vec<NLPTech> = vec![
    NLPTech {
        id: "presupposition".into(),
        name: "Presupposition".into(),
        name_vi: "Giả định KH sẽ dùng".into(),
        description: "Bỏ qua câu hỏi 'có nên mua'".into(),
    },
    NLPTech {
        id: "embedded_command".into(),
        name: "Embedded Command".into(),
        name_vi: "Lệnh ẩn trong câu".into(),
        description: "Gây ấn hành động".into(),
    },
    NLPTech {
        id: "open_loop".into(),
        name: "Open Loop".into(),
        name_vi: "Vòng lặp câu chuyện".into(),
        description: "Bắt buộc đọc tiếp".into(),
    },
    NLPTech {
        id: "sensory_language".into(),
        name: "Sensory language".into(),
        name_vi: "Ngôn ngữ 5 giác quan".into(),
        description: "Visual, auditory, kinesthetic".into(),
    },
    NLPTech {
        id: "double_bind".into(),
        name: "Double Bind".into(),
        name_vi: "2 lựa chọn đều có lợi".into(),
        description: "Epanding options".into(),
    },
    NLPTech {
        id: "power_words".into(),
        name: "Power Words".into(),
        name_vi: "Từ kích thích cảm xúc".into(),
        description: "Bí mật, độc quyền, đột phá".into(),
    },
    NLPTech {
        id: "identity_labeling".into(),
        name: "Identity Labeling".into(),
        name_vi: "Gán nhãn KH thông minh".into(),
        description: "Target KH ideal".into(),
    },
    NLPTech {
        id: "negative_pacing".into(),
        name: "Negative Future Pacing".into(),
        name_vi: "Hậu quả nếu không hành động".into(),
        description: "FOMO consequences".into(),
    },
    NLPTech {
        id: "social_currency".into(),
        name: "Social Currency".into(),
        name_vi: "Share được thấy cool".into(),
        description: "Viral content".into(),
    },
    NLPTech {
        id: "personal_pronoun".into(),
        name: "Personal Pronoun".into(),
        name_vi: "Xưng 'bạn', 'em'".into(),
        description: "Tạo kết nối cá nhân".into(),
    },
];

// === CONTENT TONES ===

pub static TONES: Vec<ContentTone> = vec![
    ContentTone { id: "formal".into(), name: "Chuyên nghiệp".into(), emoji: "💼".into() },
    ContentTone { id: "friendly".into(), name: "Thân thiện".into(), emoji: "🤝".into() },
    ContentTone { id: "playful".into(), name: "Vui vẻ".into(), emoji: "🎉".into() },
    ContentTone { id: "urgent".into(), name: "Khẩn cấp".into(), emoji: "🚨".into() },
    ContentTone { id: "luxury".into(), name: "Cao cấp".into(), emoji: "✨".into() },
    ContentTone { id: "casual".into(), name: "Bình thường".into(), emoji: "😊".into() },
];

// === CHANNELS ===

pub static CHANNELS: Vec<Channel> = vec![
    Channel { id: "zalo".into(), name: "Zalo OA".into(), max_length: 1000, format: " Vietnamese casual".into() },
    Channel { id: "facebook".into(), name: "Facebook".into(), max_length: 2000, format: " Social media".into() },
    Channel { id: "tiktok".into(), name: "TikTok".into(), max_length: 150, format: "Short, trend".into() },
    Channel { id: "shopee".into(), name: "Shopee".into(), max_length: 500, format: "E-commerce listing".into() },
    Channel { id: "landing_page".into(), name: "Landing Page".into(), max_length: 2000, format: "Sales copy".into() },
    Channel { id: "email".into(), name: "Email".into(), max_length: 5000, format: "Professional".into() },
    Channel { id: "sms".into(), name: "SMS".into(), max_length: 160, format: "Ngắn gọn".into() },
];

// Helper function for Vietnamese content optimization
pub fn optimize_vietnamese(content: &str) -> String {
    content
        .replace("you", "bạn")
        .replace("Your", "Của bạn")
        .replace("FREE", "MIỄN PHÍ")
        .replace("SALE", "GIẢM GIÁ")
}

// Test module
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_formulas_count() {
        assert_eq!(FORMULAS.len(), 14);
    }
    
    #[test]
    fn test_psychology_count() {
        assert_eq!(PSYCHOLOGY.len(), 18);
    }
    
    #[test]
    fn test_optimize_vietnamese() {
        assert_eq!(optimize_vietnamese("FREE SHIPPING"), "MIỄN PHÍ SHIPPING");
    }
}
