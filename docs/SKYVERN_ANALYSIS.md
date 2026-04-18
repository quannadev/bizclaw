# Skyvern Analysis for BizClaw SME Platform

**Source**: https://github.com/Skyvern-AI/skyvern  
**Date**: 2026-04-17

---

## 1. SKYVERN CORE FEATURES

### 1.1 AI-Powered Browser Automation
Skyvern uses LLMs and Computer Vision to automate browser-based workflows:
- **Natural Language Commands**: `page.click("Click the login button")`
- **AI Element Detection**: Finds elements visually, not XPath
- **Resistant to website changes**: Adapts when layouts change

### 1.2 Multi-Agent Architecture
- **Skyvern Agent**: Comprehends website, plans and executes actions
- **Browser Agent**: Executes Playwright commands
- **Task Agent**: Handles multi-step workflows

### 1.3 No-Code Workflow Builder
- Visual workflow editor
- Pre-built automation templates
- Scheduled execution

---

## 2. WHAT BIZCLAW CAN LEARN

### 2.1 AI Browser Agent (HIGH PRIORITY)

BizClaw can add a **Browser Hand** similar to Skyvern:

```rust
// New Tool: browser_automation
struct BrowserAutomationTool {
    action: String,  // click, fill, extract, navigate
    target: String,  // Natural language prompt or selector
    params: HashMap,
}

// Example usage in BizClaw:
"Navigate to Shopee, search for 'iPhone 16', extract prices"
"Fill Zalo OA form with customer data"
"Login to bank website, download statement"
```

### 2.2 Vision-Language Integration

Skyvern uses computer vision to understand UI elements. BizClaw can:

1. **Screenshot + GPT-4 Vision API** → Understand UI state
2. **OCR for Vietnamese documents** → Extract data from images
3. **Visual element detection** → Click buttons by description

### 2.3 Workflow Automation

Add **Visual Workflow Builder** similar to Skyvern:

| Feature | Skyvern | BizClaw Opportunity |
|---------|---------|-------------------|
| No-code builder | ✅ | Add to dashboard |
| Scheduled tasks | ✅ | BizClaw Hands already has |
| Browser automation | ✅ | **NEW: Browser Hand** |
| API integration | ✅ | Current tools |
| Multi-step tasks | ✅ | Workflows module |

---

## 3. BIZCLAW BROWSER HAND PROPOSAL

### 3.1 Architecture

```
┌─────────────────────────────────────────────────────────┐
│                  BIZCLAW BROWSER HAND                     │
├─────────────────────────────────────────────────────┤
│                                                       │
│  User: "Tìm iPhone 16 trên Shopee, lấy giá"         │
│                                                       │
│  ┌─────────────────────────────────────────────┐      │
│  │  Browser Agent (Playwright + AI Vision)       │      │
│  │  ├── Navigate to shopee.vn                   │      │
│  │  ├── Click search box                        │      │
│  │  ├── Fill "iPhone 16"                      │      │
│  │  ├── Extract product prices                 │      │
│  │  └── Return structured data                │      │
│  └─────────────────────────────────────────────┘      │
│                                                       │
│  Tools:                                               │
│  • playwright: browser automation                    │
│  • vision: screenshot + GPT-4o analysis             │
│  • ocr: Vietnamese document extraction              │
│  • extraction: structured data parsing             │
│                                                       │
└─────────────────────────────────────────────────────┘
```

### 3.2 Implementation Plan

#### Phase 1: Basic Browser Tool
```rust
// crates/bizclaw-tools/src/browser.rs
pub struct BrowserTool {
    browser: Arc<Browser>,
}

impl BrowserTool {
    pub async fn execute(&self, action: &str, params: &Value) -> Result<Value> {
        match action {
            "navigate" => self.navigate(params).await,
            "click" => self.click(params).await,
            "fill" => self.fill(params).await,
            "extract" => self.extract(params).await,
            "screenshot" => self.screenshot().await,
            _ => Err("Unknown action".into()),
        }
    }
}
```

#### Phase 2: AI Vision Integration
```rust
pub async fn click_by_description(&self, description: &str) -> Result<()> {
    // 1. Take screenshot
    let screenshot = self.browser.screenshot().await?;
    
    // 2. Send to GPT-4 Vision
    let response = openai::vision::analyze(&screenshot, 
        &format!("Find the button that: {}", description)
    ).await?;
    
    // 3. Parse coordinates from response
    let coords = parse_gpt_response(&response);
    
    // 4. Click at coordinates
    self.browser.click_at(coords.x, coords.y).await
}
```

#### Phase 3: Vietnamese Document Processing
```rust
pub async fn extract_invoice(&self, screenshot: Vec<u8>) -> Result<Invoice> {
    // 1. OCR with Vietnamese support
    let text = ocr_vietnamese(&screenshot)?;
    
    // 2. Extract structured data
    let invoice = parse_invoice(&text)?;
    
    Ok(invoice)
}
```

---

## 4. USE CASES FOR SME

### 4.1 E-commerce Automation

| Task | BizClaw Browser Hand |
|------|---------------------|
| Price monitoring | ✅ Auto scrape Shopee, Lazada, Tiki |
| Order tracking | ✅ Login, check status |
| Inventory sync | ✅ Read/write spreadsheet |
| Product listing | ✅ Auto upload to multiple platforms |

### 4.2 Banking & Finance

| Task | BizClaw Browser Hand |
|------|---------------------|
| Statement download | ✅ Login, navigate, export |
| Balance check | ✅ Daily automated |
| Transfer automation | ✅ With OTP approval |
| Invoice extraction | ✅ OCR + AI parsing |

### 4.3 HR & Recruitment

| Task | BizClaw Browser Hand |
|------|---------------------|
| CV scraping | ✅ LinkedIn, VietnamWorks |
| Job posting | ✅ Multi-platform |
| Payroll check | ✅ Bank portal |
| Leave management | ✅ Company portal |

### 4.4 Marketing

| Task | BizClaw Browser Hand |
|------|---------------------|
| Social posting | ✅ Facebook, Zalo, TikTok |
| Analytics scraping | ✅ Platform dashboards |
| Ad management | ✅ Google Ads, Facebook Ads |
| Email automation | ✅ Gmail automation |

---

## 5. COMPETITIVE ADVANTAGE

### Skyvern
- ✅ Strong browser automation
- ✅ AI vision integration
- ❌ No Vietnamese support
- ❌ No CRM/chatbot integration
- ❌ Enterprise pricing

### BizClaw + Browser Hand
- ✅ Multi-channel chatbot (Zalo, Telegram, FB)
- ✅ CRM built-in
- ✅ Vietnamese NLP
- ✅ SME-focused
- ✅ Local pricing

**Combined**: BizClaw chatbot + Browser automation = Complete SME platform

---

## 6. IMPLEMENTATION ROADMAP

### Week 1-2: Basic Browser Tool
- [ ] Playwright integration
- [ ] Basic actions: navigate, click, fill, extract
- [ ] Screenshot tool

### Week 3-4: AI Vision
- [ ] GPT-4 Vision API integration
- [ ] Natural language element finding
- [ ] Vietnamese OCR

### Week 5-6: SME Use Cases
- [ ] Shopee/Tiki price monitoring
- [ ] Invoice extraction
- [ ] Social media posting

### Week 7-8: Integration
- [ ] Workflow builder UI
- [ ] CRM integration
- [ ] Scheduling

---

## 7. DIFFERENTIATORS FOR BIZCLAW

1. **Vietnamese-first**: Native Vietnamese OCR, NLP
2. **Chatbot integration**: Browser Hand accessible via chat
3. **CRM/Channels**: All-in-one platform
4. **SME pricing**: Affordable for small businesses
5. **Local support**: Vietnamese team

---

## 8. PRICING MODEL

| Tier | Price | Browser Hours |
|------|-------|---------------|
| Starter | 299K/mo | 10 hours |
| Professional | 599K/mo | 50 hours |
| Enterprise | Custom | Unlimited |

---

*Analysis by BizClaw Team*
