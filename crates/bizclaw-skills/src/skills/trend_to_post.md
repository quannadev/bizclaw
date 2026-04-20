---
name: trend-to-post
description: |
  Trend-to-Post workflow for BizClaw when user wants to create content from trending topics.
  Trigger phrases: tạo bài từ trend, viết content theo trend, theo dõi trend,
  trending content, tạo bài đăng từ hot topic, lấy trend viết bài,
  trend analysis, hot topic content, social media from trends, TrendRadar,
  theo dõi tin nóng, giám sát xu hướng, public opinion tracking.
  Scenarios: khi cần tạo content từ trend, khi cần viết bài theo hot topic,
  khi muốn đăng bài theo xu hướng, khi cần content marketing từ trend,
  khi muốn monitor tin tức đa nguồn.
version: 2.0.0
---

# Trend-to-Post Workflow v2.0

You are a content strategist connecting TrendRadar trends with BizClaw content creation.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         TREND-TO-POST PIPELINE                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   TRENDRADAR MCP                                                           │
│   ┌─────────────┐     ┌─────────────┐     ┌─────────────┐                  │
│   │   Sources   │────▶│   Filter    │────▶│     AI      │                  │
│   │ 11 platforms│     │ Keywords/AI │     │  Analysis   │                  │
│   │    + RSS    │     │   Scoring   │     │ Sentiment   │                  │
│   └─────────────┘     └─────────────┘     └──────┬──────┘                  │
│                                                  │                          │
│                                                  ▼                          │
│   BIZCLAW AGENT                         ┌─────────────┐                   │
│   ┌─────────────┐     ┌─────────────┐   │   Alerts    │                   │
│   │   Analyze   │◀────│   Trends    │───▶│   Reports   │                   │
│   │   Deep     │     │   Hot/New   │    │   Insights   │                   │
│   └──────┬──────┘     └─────────────┘    └─────────────┘                   │
│          │                                                            │
│          ▼                                                            │
│   ┌─────────────────────────────────────┐                              │
│   │        CONTENT GENERATOR              │                              │
│   │  Zalo │ Facebook │ Telegram │ Email│                              │
│   └─────────────┬───────────────────────┘                              │
│                 │                                                         │
│                 ▼                                                         │
│   ┌─────────────────────────────────────┐                              │
│   │        AUTO-PUBLISHER                │                              │
│   │  Schedule │ Queue │ Retry │ Analytics│                              │
│   └─────────────────────────────────────┘                              │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Integration Methods

### 1. MCP Server (TrendRadar side)

```yaml
# Start TrendRadar MCP Server
docker run -d \
  --name trendradar-mcp \
  -p 127.0.0.1:3333:3333 \
  -v $(pwd)/config:/app/config:ro \
  -v $(pwd)/output:/app/output:ro \
  wantcat/trendradar-mcp:latest
```

### 2. BizClaw Configuration

```yaml
# bizclaw.yaml
mcp:
  trendradar:
    enabled: true
    url: "http://127.0.0.1:3333"
    interests:
      - "AI"
      - "technology"
      - "startup"
      - "cryptocurrency"
      - "business"
    platforms:
      - "weibo"
      - "zhihu"
      - "baidu"
      - "toutiao"
      - "wallstreetcn"
    sentiment_threshold: 0.5
    refresh_interval: "15m"

content:
  channels:
    - "zalo"
    - "facebook"
    - "telegram"
    - "email"
  max_posts_per_day: 10
  auto_publish: true
  schedule:
    morning: "08:00"
    noon: "12:00"
    evening: "18:00"
```

## MCP Tools Available

| Tool | Purpose | Parameters | Returns |
|------|---------|------------|---------|
| `get_trending_topics` | Get hot topics | `keywords[]`, `platforms[]` | List of trending topics |
| `search_news` | Search by query | `query`, `days`, `include_url` | Matching news items |
| `get_latest_news` | Get today's news | `platforms[]`, `limit` | Today's news |
| `analyze_topic_trend` | Deep analysis | `topic`, `days` | TrendAnalysis object |
| `analyze_sentiment` | Emotion analysis | `topic`, `days` | Sentiment breakdown |
| `generate_summary_report` | Daily digest | `days` | Full report |
| `compare_periods` | Trend comparison | `period1`, `period2` | Comparison data |
| `aggregate_news` | Deduplicate | `news[]` | Deduplicated list |

## Usage Examples

### Example 1: Morning Trend Brief

```javascript
// 1. Get trending topics
const topics = await callMcpTool('get_trending_topics', {
  keywords: ['AI', 'startup'],
  platforms: ['weibo', 'zhihu']
});

// 2. Analyze top topic
const analysis = await callMcpTool('analyze_topic_trend', {
  topic: topics[0].title,
  days: 7
});

// 3. Generate content
const content = await bizclawAgent.process(`
  Tạo bài đăng Zalo từ trend analysis:
  Topic: ${analysis.topic}
  Mentions: ${analysis.total_mentions}
  Sentiment: ${JSON.stringify(analysis.sentiment_breakdown)}
  Insights: ${analysis.insights.join(', ')}
  
  Format: 300-500 chars, Vietnamese, có emoji
`);

// 4. Publish
await publishToChannel('zalo', content);
```

### Example 2: Real-time Alert Response

```javascript
// Check for breaking news
const alerts = await checkTrendingAlerts({
  mentions_threshold: 50000,
  sentiment: ['negative', 'controversial'],
  hotness: 0.9
});

if (alerts.length > 0) {
  for (const alert of alerts) {
    // Generate response content
    const response = await generateAlertResponse(alert);
    
    // Publish immediately
    await publishToAllChannels(response, { urgent: true });
    
    // Log for analytics
    await trackAlert(alert, response);
  }
}
```

### Example 3: Weekly Newsletter

```javascript
// Generate weekly report
const report = await callMcpTool('generate_summary_report', {
  days: 7
});

// Generate newsletter content
const newsletter = await bizclawAgent.process(`
  Tạo bản tin email từ dữ liệu sau:
  
  ## Top Trends Tuần
  ${report.top_trends.map((t, i) => `${i+1}. ${t.title} (${t.mentions} mentions)`).join('\n')}
  
  ## Sentiment Analysis
  Positive: ${report.sentiment.positive}%
  Neutral: ${report.sentiment.neutral}%
  Negative: ${report.sentiment.negative}%
  
  ## Key Insights
  ${report.insights.map((i, idx) => `${idx+1}. ${i}`).join('\n')}
  
  Format: HTML email, professional Vietnamese
`);

// Schedule for Sunday 9AM
await scheduleEmail('newsletter@subscribers.com', newsletter, {
  send_at: '2024-01-21 09:00:00'
});
```

## Content Templates

### Zalo Post (Short - 500 chars)
```markdown
🔥 [SỐT] {title}

{2-sentence summary}

📊 {mentions} thảo luận trên {platform}
🔗 {source_url}

---
💡 Theo dõi để cập nhật tin nóng!
```

### Facebook Post (Medium - 1500 chars)
```markdown
🔥 {title}

{Detailed summary - 3-4 sentences}

📊 THÔNG TIN:
• Nền tảng: {platform}
• Lượt thảo luận: {mentions}
• Xu hướng: {trajectory}
• Cảm xúc: {sentiment}

📖 Đọc thêm: {url}

{hashtags}

---
💡 Theo dõi page để cập nhật tin nóng mỗi ngày!
```

### Telegram Thread (Long - Multi-message)
```markdown
🧵 THREAD: {topic}

1/ {Hook - surprising fact about the trend}

2/ {Context - why this matters now}

3/ {Key insights from analysis}
   • Insight 1
   • Insight 2
   • Insight 3

4/ {What this means for Vietnam market}

5/ {Call to action}
   🔗 Read more: {url}
   🔔 Follow for daily updates

#Trending #{Topic} #Vietnam #Tech
```

### Email Newsletter (Full Report)
```markdown
Subject: 📊 Bản tin xu hướng - {date}

Xin chào!

Tuần này có những xu hướng đáng chú ý:

📈 TOP TRENDING
{Top 5 trends with mentions and sentiment}

💡 INSIGHTS
{AI-generated insights about patterns}

📰 MUST-READ
{Top 3 articles with summaries}

🎯 RECOMMENDED ACTIONS
{3-5 action items based on trends}

---
Đăng ký nhận tin: [Link]
Hủy đăng ký: [Link]
```

## Automation Schedule

### Recommended Schedule

| Time | Action | Channels | Priority |
|------|--------|----------|----------|
| 07:00 | Morning brief | Zalo, Email | High |
| 09:00 | Top trends | Facebook | Medium |
| 12:00 | Midday update | Zalo, Telegram | Low |
| 15:00 | Afternoon scan | All | Medium |
| 18:00 | Evening summary | Email, Facebook | High |
| 20:00 | Late trends | Telegram | Low |
| Sunday 09:00 | Weekly digest | Email | High |

### BizClaw Hand Configuration

```yaml
# hand.yaml
name: "trend-radar"
schedule:
  - cron: "0 7 * * 1-5"   # Weekday morning
    action: "morning_brief"
    channels: ["zalo", "email"]
    
  - cron: "0 9,12,18 * * *"  # 3x daily
    action: "trend_post"
    channels: ["facebook", "telegram"]
    
  - cron: "0 9 * * 0"        # Sunday morning
    action: "weekly_digest"
    channels: ["email"]

filters:
  mentions_min: 1000
  hotness_min: 0.6
  sentiment_allow: ["positive", "neutral", "negative"]
  platforms: ["weibo", "zhihu", "baidu", "toutiao"]

content:
  languages: ["vi"]
  formats: ["short", "medium", "thread"]
  max_daily_posts: 10
```

## Gotchas

### 1. Rate Limiting
```javascript
// Don't spam TrendRadar API
const CACHE_DURATION = 15 * 60 * 1000; // 15 minutes
const cache = new Map();

async function safeFetch(endpoint, params) {
  const cacheKey = JSON.stringify({ endpoint, params });
  
  if (cache.has(cacheKey)) {
    const [timestamp, data] = cache.get(cacheKey);
    if (Date.now() - timestamp < CACHE_DURATION) {
      return data;
    }
  }
  
  const data = await callMcpTool(endpoint, params);
  cache.set(cacheKey, [Date.now(), data]);
  return data;
}
```

### 2. Sentiment Filtering
```javascript
// Always check sentiment before publishing
const BLOCKED_SENTIMENTS = ['controversial', 'extremely_negative'];
const ANALYSIS = await analyze_sentiment(topic);

if (BLOCKED_SENTIMENTS.includes(ANALYSIS.dominant)) {
  console.log('⚠️ Skipping due to sentiment');
  return { skipped: true, reason: 'sentiment' };
}
```

### 3. Attribution Required
```javascript
// Always cite TrendRadar sources
const attribution = `
📢 Nguồn: TrendRadar - Theo dõi ${platform}
🔗 Link: ${sourceUrl}
© TrendRadar
`;
```

### 4. Platform-Specific Limits
```javascript
const PLATFORM_LIMITS = {
  zalo: { max_chars: 500, max_posts_per_day: 10 },
  facebook: { max_chars: 1500, max_posts_per_day: 5 },
  telegram: { max_chars: 4000, max_posts_per_day: 20 },
  email: { max_chars: 10000, max_posts_per_day: 1 }
};
```

## Complete Workflow Example

```
┌─────────────────────────────────────────────────────────────────┐
│                    TREND-TO-POST EXECUTION                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│ [08:00] TRIGGER: Morning Brief                                  │
│                                                                 │
│ [08:01] ├─ Fetch trending topics                                │
│ [08:02] │  └─ Found: 12 topics, filtered: 8                    │
│ [08:03] ├─ Analyze top 3                                        │
│ [08:04] │  ├─ Topic 1: AI Startup (150K mentions) ✓            │
│ [08:05] │  ├─ Topic 2: Crypto Rally (85K mentions) ✓           │
│ [08:06] │  └─ Topic 3: Policy Change (45K mentions) ⚠️        │
│ [08:07] ├─ Generate content                                     │
│ [08:08] │  ├─ Zalo post: "🔥 AI Startup..." (450 chars) ✓      │
│ [08:09] │  ├─ Email brief: Full analysis (2.5KB) ✓             │
│ [08:10] │  └─ Telegram thread: 5 tweets ✓                     │
│ [08:11] ├─ Schedule posts                                       │
│ [08:12] │  ├─ Zalo: Published immediately ✓                   │
│ [08:13] │  ├─ Email: Scheduled 09:00 ✓                          │
│ [08:14] │  └─ Telegram: Scheduled 09:30 ✓                      │
│ [08:15] └─ Log analytics                                        │
│             └─ 3 posts generated, 0 errors                      │
│                                                                 │
│ [09:00] EMAIL SENT                                               │
│                                                                 │
│ [09:30] TELEGRAM THREAD POSTED                                   │
│                                                                 │
│ [12:00] TRIGGER: Midday Update                                   │
│             └─ Process continues...                               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Integration with BizClaw MCP Client

```rust
use bizclaw_mcp::{TrendRadarConfig, TrendMonitor, ContentGenerator};

async fn trend_to_post_workflow() -> Result<()> {
    // Initialize TrendRadar config
    let config = TrendRadarConfig {
        api_url: "http://localhost:3333".to_string(),
        interests: vec![
            "AI".into(),
            "startup".into(),
            "technology".into(),
        ],
        language: "vi".into(),
        ..Default::default()
    };

    // Create monitors
    let mut monitor = TrendMonitor::new(config);
    let generator = ContentGenerator::new();

    // Scan trends
    let trends = monitor.scan_trends("http://localhost:3333").await?;

    // Generate alerts
    let alerts = monitor.generate_alerts(&trends);

    // Generate content for each alert
    for alert in alerts.iter().take(5) {
        let content = generator.generate_from_trend(
            &TrendNews::from(alert),
            "zalo"
        );

        // Publish via BizClaw channels
        bizclaw_channel_send("zalo", &content.content).await?;
    }

    Ok(())
}
```
