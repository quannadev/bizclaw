# Browser Automation Analysis for BizClaw SME

**Sources**: OpenClaw, GoClaw, CrawBot, Gridex  
**Date**: 2026-04-17

---

## 1. KEY INSIGHTS FROM ANALYSIS

### 1.1 OpenClaw - Token-Efficient Browser Control

OpenClaw uses Claude Code CLI for browser automation with these strategies:

```
┌─────────────────────────────────────────────────────────────┐
│             TOKEN OPTIMIZATION STRATEGIES                  │
├───────────────────────────────────────────────────────┤
│                                                        │
│  1. DOM-based extraction (not screenshots)              │
│     • "Interactive" filter = only clickable elements    │
│     • ~800 tokens vs 10K+ for screenshots               │
│                                                        │
│  2. Cached element references                          │
│     • Element IDs (e1, e2, e3...) for fast click       │
│     • No re-parsing DOM on every action               │
│                                                        │
│  3. Structured output extraction                       │
│     • JSON schema for data extraction                  │
│     • Pre-defined prompts for common tasks             │
│                                                        │
│  4. Streaming responses                               │
│     • Real-time feedback                              │
│     • Token streaming for long operations             │
│                                                        │
└──────────────────────────────────────────────────────┘
```

### 1.2 GoClaw - 8-Stage Agent Pipeline

```
Context → History → Prompt → Think → Act → Observe → Memory → Summarize
   ↓         ↓        ↓       ↓      ↓       ↓         ↓
  Load    Compact  Build   Claude  Browser Extract  Store  Compress

KEY INSIGHT: Each stage can be cached or skipped for efficiency
```

### 1.3 CrawBot - Desktop UX for Browser Control

```
CrawBot Desktop
├── Visual workflow builder (no-code)
├── Drag-and-drop browser actions
├── Pre-built skill templates
└── Real-time preview

BIZCLAW OPPORTUNITY: Add visual browser workflow builder
```

### 1.4 Gridex - Grid-Based Workspace

```
Grid Layout:
┌────────────────┬────────────────┐
│   Browser      │    Data       │
│   Preview      │    Grid       │
├────────────────┼────────────────┤
│   Actions      │    Notes       │
│   Panel        │    Panel       │
└────────────────┴────────────────┘
```

---

## 2. TOKEN-OPTIMIZED BROWSER TOOL FOR BIZCLAW

### 2.1 Efficient Browser Actions

```rust
// EFFICIENT BROWSER TOOL - Token-Optimized
pub enum BrowserAction {
    // LOW TOKEN (100-500)
    Navigate { url: String },
    Click { ref: String },          // e.g., "e5"
    Fill { ref: String, value: String },
    Press { key: String },
    Wait { ms: u64 },
    
    // MEDIUM TOKEN (500-2000)
    Snapshot { filter: Filter },      // interactive/content/all
    Extract { schema: JsonSchema },
    
    // HIGH TOKEN (2000-10000) - Use sparingly
    Screenshot,
    VisionFind { prompt: String },   // Claude Vision
    VisionExtract { prompt: String },
}

pub enum Filter {
    Interactive,  // ~800 tokens - RECOMMENDED
    Content,     // ~1500 tokens
    All,         // ~10000 tokens - AVOID unless needed
}
```

### 2.2 Cached Element References

```rust
// Element reference caching - avoid re-parsing DOM
struct BrowserSession {
    instance_id: String,
    elements: HashMap<String, ElementRef>,  // Cache element refs
    viewport: Viewport,
    last_snapshot: Option<Snapshot>,
}

impl BrowserSession {
    pub fn click(&mut self, ref: &str) -> Result<()> {
        // Use cached element reference instead of parsing DOM
        if let Some(elem) = self.elements.get(ref) {
            return self.click_at(elem.x, elem.y);
        }
        // Fallback: parse DOM
        self.parse_and_click(ref)
    }
}
```

### 2.3 Progressive Extraction Strategy

```rust
pub async fn extract_product_info(&self, page: &str) -> Result<Product> {
    // Strategy 1: Try text extraction first (CHEAPEST)
    if let Ok(text) = self.extract_text("price, title, rating") {
        return parse_product(text);
    }
    
    // Strategy 2: Structured DOM extraction (MEDIUM)
    if let Ok(html) = self.snapshot(Filter::Content) {
        return parse_html_product(html);
    }
    
    // Strategy 3: Screenshot + Vision (EXPENSIVE - last resort)
    self.vision_extract("Extract product name, price, rating as JSON")
}
```

---

## 3. BIZCLAW BROWSER HAND - OPTIMIZED

### 3.1 Tiered Actions

```rust
// tiered_actions.rs - Optimize for token efficiency

#[derive(Clone)]
pub struct TieredBrowserTool {
    base_url: String,
    token_budget: u32,
}

impl TieredBrowserTool {
    pub fn new(token_budget: u32) -> Self {
        Self {
            base_url: "http://localhost:9867".to_string(),
            token_budget,
        }
    }
    
    /// CHEAP TIER (~100-500 tokens)
    pub async fn cheap_action(&self, action: &str) -> Result<ToolResult> {
        match action {
            "navigate" => self.navigate().await,
            "click" => self.click().await,
            "fill" => self.fill().await,
            "press" => self.press().await,
            "wait" => self.wait().await,
            "scroll" => self.scroll().await,
            _ => Err("Unknown cheap action".into()),
        }
    }
    
    /// MEDIUM TIER (~500-2000 tokens)
    pub async fn medium_action(&self, action: &str) -> Result<ToolResult> {
        match action {
            "snapshot_interactive" => self.snapshot_interactive().await,
            "snapshot_content" => self.snapshot_content().await,
            "extract_json" => self.extract_json().await,
            "find_by_text" => self.find_by_text().await,
            _ => Err("Unknown medium action".into()),
        }
    }
    
    /// EXPENSIVE TIER (~2000-10000 tokens) - Use sparingly
    pub async fn expensive_action(&self, prompt: &str) -> Result<ToolResult> {
        match prompt {
            "vision_find" => self.vision_find().await,
            "vision_extract" => self.vision_extract().await,
            "full_page_screenshot" => self.full_page_screenshot().await,
            _ => Err("Unknown expensive action".into()),
        }
    }
    
    /// Auto-select tier based on budget
    pub async fn smart_execute(&self, action: &str, budget: u32) -> Result<ToolResult> {
        if budget < 500 {
            self.cheap_action(action).await
        } else if budget < 2000 {
            self.medium_action(action).await
        } else {
            self.expensive_action(action).await
        }
    }
}
```

### 3.2 Cached Snapshots

```rust
// Smart caching for snapshots
pub struct SnapshotCache {
    cache: HashMap<String, CachedSnapshot>,
    ttl_seconds: u64,
}

impl SnapshotCache {
    pub fn get_or_fetch<F>(&mut self, url: &str, fetcher: F) -> Result<String>
    where F: Future<Output = Result<String>> {
        if let Some(cached) = self.cache.get(url) {
            if !cached.is_expired() {
                return Ok(cached.html.clone());
            }
        }
        let html = fetcher.await?;
        self.cache.insert(url.to_string(), CachedSnapshot::new(html));
        Ok(html)
    }
}
```

### 3.3 Token Budget Per Task

```rust
pub struct TaskBudget {
    pub max_tokens: u32,
    pub actions: Vec<Action>,
    pub spent: u32,
}

impl TaskBudget {
    pub fn remaining(&self) -> u32 {
        self.max_tokens.saturating_sub(self.spent)
    }
    
    pub fn can_afford(&self, action: &Action) -> bool {
        self.remaining() >= action.token_cost()
    }
    
    pub fn track(&mut self, tokens: u32) {
        self.spent += tokens;
    }
}

// Token costs estimation
pub fn estimate_cost(action: &str) -> u32 {
    match action {
        "click" => 50,
        "fill" => 100,
        "navigate" => 200,
        "snapshot_interactive" => 800,
        "snapshot_content" => 1500,
        "vision_find" => 5000,
        _ => 1000,
    }
}
```

---

## 4. GRIDEX-STYLE GRID WORKSPACE FOR BIZCLAW

### 4.1 Grid-Based Browser Preview

```rust
pub struct GridWorkspace {
    pub browser_preview: BrowserPane,
    pub data_grid: DataGridPane,
    pub actions_panel: ActionsPane,
    pub notes_panel: NotesPane,
}

impl GridWorkspace {
    pub fn new() -> Self {
        Self {
            browser_preview: BrowserPane::new("browser"),
            data_grid: DataGridPane::new("data"),
            actions_panel: ActionsPane::new("actions"),
            notes_panel: NotesPane::new("notes"),
        }
    }
    
    /// Auto-layout based on viewport
    pub fn auto_layout(&mut self, viewport: (u32, u32)) {
        match viewport {
            (320..=768) => self.mobile_layout(),
            (769..=1024) => self.tablet_layout(),
            _ => self.desktop_layout(),
        }
    }
}
```

### 4.2 Data Extraction Grid

```rust
pub struct DataGrid {
    columns: Vec<Column>,
    rows: Vec<Row>,
}

pub struct Column {
    pub header: String,
    pub extract_path: String,  // JSONPath
}

impl DataGrid {
    pub fn extract_from_html(&self, html: &str) -> Vec<Row> {
        let doc = scraper::Html::parse_document(html);
        self.rows.iter().map(|row| {
            Row {
                cells: row.columns.iter().map(|col| {
                    doc.select(&col.selector)
                        .first()
                        .map(|e| e.text().collect::<String>())
                        .unwrap_or_default()
                }).collect()
            }
        }).collect()
    }
}
```

---

## 5. SME USE CASES - TOKEN-OPTIMIZED

### 5.1 Price Monitoring (Shopee/Tiki/Lazada)

```
┌─────────────────────────────────────────────────────────┐
│ TASK: Monitor prices for "iPhone 16 Pro Max 256GB"        │
├───────────────────────────────────────────────────────┤
│                                                        │
│ STEP 1: Navigate (200 tokens)                           │
│   browser(navigate, url="shopee.vn")                     │
│                                                        │
│ STEP 2: Fill search (100 tokens)                       │
│   browser(fill, ref="e5", value="iPhone 16 Pro Max")    │
│   # Use cached element ref "e5" from snapshot           │
│                                                        │
│ STEP 3: Wait for results (0 tokens)                     │
│   browser(wait, ms=2000)                              │
│                                                        │
│ STEP 4: Extract prices (800 tokens)                    │
│   browser(snapshot, filter="content")                   │
│   # Parse price: "24.990.000₫" → 24990000            │
│                                                        │
│ STEP 5: Extract to grid (500 tokens)                   │
│   grid.extract("price", "//span[@class='price']")        │
│                                                        │
│ TOTAL: ~1600 tokens (vs 50,000 for full screenshot)    │
└──────────────────────────────────────────────────────┘
```

### 5.2 Invoice Processing (Vietnamese OCR)

```
┌─────────────────────────────────────────────────────────┐
│ TASK: Extract invoice data from PDF/image               │
├───────────────────────────────────────────────────────┤
│                                                        │
│ STEP 1: Load invoice (100 tokens)                      │
│   document(load, path="invoice.pdf")                    │
│                                                        │
│ STEP 2: OCR Vietnamese (2000 tokens)                  │
│   ocr_vietnamese(image)                               │
│   # Extracts: invoice_number, date, items, total       │
│                                                        │
│ STEP 3: Structure to JSON (200 tokens)                 │
│   json.parse(ocr_text)                                │
│                                                        │
│ TOTAL: ~2300 tokens                                  │
│ ALTERNATIVE: Vision API = 50,000 tokens               │
│ SAVINGS: 95%                                         │
└──────────────────────────────────────────────────────┘
```

---

## 6. COMPARISON: OpenClaw vs BizClaw

| Feature | OpenClaw | GoClaw | CrawBot | **BizClaw** |
|---------|----------|---------|---------|------------|
| DOM extraction | ✅ Claude Code | ✅ Built-in | ✅ OpenClaw | **✅ PinchTab** |
| Element caching | ✅ Yes | ✅ Yes | ✅ Yes | **TODO** |
| Token budget | ✅ Auto | ✅ Configurable | ✅ GUI | **Need to add** |
| Grid workspace | ❌ No | ❌ No | ✅ Yes | **Add Grid layout** |
| Vietnamese OCR | ❌ | ❌ | ❌ | **Add Tesseract** |
| Multi-channel | ✅ 20+ | ✅ 7+ | ✅ OpenClaw | **✅ Already has** |
| SME pricing | ❌ | ❌ | ❌ | **✅ SME-focused** |

---

## 7. RECOMMENDATIONS FOR BIZCLAW

### 7.1 Immediate (Week 1)
1. ✅ Add element caching to browser tool
2. ✅ Add Filter option (interactive/content/all)
3. ✅ Add token estimation per action
4. ✅ Add budget tracking

### 7.2 Short-term (Week 2-3)
1. Add Grid workspace layout
2. Add Vietnamese OCR tool
3. Add pre-built extraction templates
4. Add token budget per task

### 7.3 Medium-term (Week 4+)
1. Visual workflow builder
2. Token usage dashboard
3. Auto-tier selection
4. Snapshot caching

---

## 8. IMPLEMENTATION PRIORITY

```
┌─────────────────────────────────────────────────────────┐
│ PRIORITY 1: Element Caching                          │
│   → Save element refs between actions                │
│   → Avoid re-parsing DOM                          │
├────────────────────────────────────────────────────┤
│ PRIORITY 2: Token Budget Tracking                   │
│   → Track tokens per task                          │
│   → Warn when approaching limit                   │
├────────────────────────────────────────────────────┤
│ PRIORITY 3: Vietnamese OCR                        │
│   → Tesseract integration                        │
│   → Invoice extraction template                   │
├────────────────────────────────────────────────────┤
│ PRIORITY 4: Grid Workspace                      │
│   → Browser preview pane                          │
│   → Data extraction grid                         │
│   → Actions panel                                 │
└────────────────────────────────────────────────┘
```

---

## 9. TOKEN COSTS REFERENCE

| Action | Tokens | Frequency | Recommendation |
|--------|--------|----------|----------------|
| navigate | 200 | Per task | Cache URL |
| click | 50 | High | Use cached refs |
| fill | 100 | Medium | Batch fills |
| snapshot(interactive) | 800 | Medium | Default |
| snapshot(content) | 1500 | Low | When needed |
| vision_find | 5000 | Very Low | Last resort |
| vision_extract | 10000 | Very Low | Last resort |

---

*Analysis complete. Ready for implementation.*
