---
name: trend-to-post
description: |
  Trend-to-Post workflow for BizClaw when user wants to create content from trending topics.
  Trigger phrases: tạo bài từ trend, viết content theo trend, theo dõi trend,
  trending content, tạo bài đăng từ hot topic, lấy trend viết bài,
  trend analysis, hot topic content, social media from trends.
  Scenarios: khi cần tạo content từ trend, khi cần viết bài theo hot topic,
  khi muốn đăng bài theo xu hướng, khi cần content marketing từ trend.
version: 2.0.0
---

# Trend-to-Post Workflow

You are a content strategist connecting TrendRadar trends with BizClaw content creation.

## Architecture

```
TrendRadar MCP ──query──► BizClaw Agent ──create──► Content ──publish──► Social Channels
     │                                                              │
     │ trends, hot topics,                                        │
     │ AI analysis, sentiment                                     │
     └────────────────────────────────────────────────────────────┘
                              feedback loop
```

## Integration Methods

### 1. MCP Server (Recommended)

```yaml
# Add to BizClaw config for TrendRadar MCP
[[mcp_servers]]
name = "trendradar"
command = "docker"
args = ["run", "--rm", "-p", "127.0.0.1:3333:3333", "wantcat/trendradar-mcp:latest"]
```

### 2. HTTP API

```javascript
// Direct HTTP calls to TrendRadar MCP
const TRENDRADAR_API = 'http://localhost:3333';

// Get trending topics
async function getTrends(keywords = ['AI', 'tech', 'business']) {
  const res = await fetch(`${TRENDRADAR_API}/mcp`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      method: 'tools/call',
      params: { name: 'get_trending_topics', arguments: { keywords } }
    })
  });
  return res.json();
}

// Search news by topic
async function searchNews(query, days = 7) {
  const res = await fetch(`${TRENDRADAR_API}/mcp`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      method: 'tools/call',
      params: { name: 'search_news', arguments: { query, days, include_url: true } }
    })
  });
  return res.json();
}
```

## Workflow Steps

### Step 1: Listen to Trends

```javascript
// Get today's trending topics matching your interests
const trends = await searchNews('AI startup funding', 1);

// Get comprehensive daily summary
const summary = await fetch(`${TRENDRADAR_API}/mcp`, {
  method: 'POST',
  body: JSON.stringify({
    method: 'tools/call',
    params: { name: 'generate_summary_report', arguments: { days: 1 } }
  })
});

// Get topic analysis with sentiment
const analysis = await fetch(`${TRENDRADAR_API}/mcp`, {
  method: 'POST',
  body: JSON.stringify({
    method: 'tools/call',
    params: { name: 'analyze_topic_trend', arguments: { topic: 'AI', days: 7 } }
  })
});
```

### Step 2: Analyze & Select

```javascript
// Filter trends by criteria
const filteredTrends = trends.filter(trend => {
  return (
    trend.platform.includes('weibo') ||    // High engagement
    trend.mentions > 10000 ||             // Viral potential
    trend.sentiment === 'positive'        // Brand-safe
  );
});

// Rank by engagement
const ranked = filteredTrends.sort((a, b) => b.mentions - a.mentions);
```

### Step 3: Create Content

```javascript
// Generate content for each trend
const contentPrompts = ranked.slice(0, 5).map(trend => {
  return {
    topic: trend.title,
    source: trend.platform,
    url: trend.url,
    angle: determineAngle(trend), // educational, news, opinion
    format: selectFormat(trend),    // post, article, thread
    tone: selectTone(trend)        // professional, casual, urgent
  };
});

// Call BizClaw content writer
async function generateContent(prompt) {
  return await bizclawAgent.process(`
    Viết một bài ${prompt.format} về "${prompt.topic}"

    Yêu cầu:
    - Tone: ${prompt.tone}
    - Nguồn: ${prompt.source}
    - Link tham khảo: ${prompt.url}
    - Độ dài: 300-500 từ
    - Format: Vietnamese with emoji
    - Include: hashtags, call-to-action
  `);
}
```

### Step 4: Publish

```javascript
// Schedule posts across channels
const schedule = [
  { channel: 'zalo', time: '09:00', content: contentPosts[0] },
  { channel: 'facebook', time: '10:00', content: contentPosts[1] },
  { channel: 'telegram', time: '12:00', content: contentPosts[2] },
  { channel: 'email', time: '18:00', content: weeklyDigest }
];

// Publish via BizClaw channels
for (const post of schedule) {
  await bizclawChannel.send(post.channel, post.content);
}
```

## MCP Tools Available

| Tool | Purpose | Use Case |
|------|---------|----------|
| `get_trending_topics` | Get hot topics | Quick overview |
| `search_news` | Search by query | Find specific trends |
| `analyze_topic_trend` | Deep analysis | Understand trajectory |
| `analyze_sentiment` | Emotion analysis | Brand safety check |
| `generate_summary_report` | Daily digest | Newsletter content |
| `compare_periods` | Trend comparison | YoY/YoY analysis |
| `aggregate_news` | Deduplicate | Clean data |

## Content Templates

### Hot Topic Post (Zalo/Facebook)
```markdown
🔥 [SỐT] {Trend Title}

{2-3 sentence summary of why it's trending}

📊 Thông tin:
• Nền tảng: {Platform}
• Lượt thảo luận: {Mentions}
• Xu hướng: {Trajectory}

📖 Đọc thêm: {URL}

#Trending #{RelatedHashtags}

---
💡 Theo dõi {Brand} để cập nhật tin nóng!
```

### Thread Post (Twitter-style)
```markdown
🧵 THREAD: {Topic}

1/ {Opening hook - surprising fact}

2/ {Context - why this matters}

3/ {Key insights from trend analysis}

4/ {What this means for you}

5/ {Call to action + CTA}

#Thread #{Hashtags}
```

### Newsletter (Email)
```markdown
Subject: 📊 Bản tin xu hướng - {Date}

Xin chào!

Tuần này có những xu hướng đáng chú ý:

📈 TOP TRENDING
1. {Trend 1} - {Mentions} thảo luận
2. {Trend 2} - {Mentions} thảo luận
3. {Trend 3} - {Mentions} thảo luận

💡 INSIGHTS
{2-3 AI-generated insights}

📰 MUST-READ
{Top 3 articles with summaries}

🎯 RECOMMENDED ACTIONS
{3-5 action items based on trends}

---
Đăng ký nhận tin: [Link]
Hủy đăng ký: [Link]
```

## Automation Schedule

```yaml
# BizClaw Hand configuration
triggers:
  - name: "morning-trend-brief"
    schedule: "0 7 * * 1-5"  # 7AM weekdays
    action: "generate_trend_brief"
    channels: ["zalo", "email"]

  - name: "hourly-trend-scan"
    schedule: "0 * * * *"  # Every hour
    action: "scan_trending"
    filters:
      mentions_threshold: 50000
      sentiment: ["positive", "neutral"]

  - name: "daily-content-pack"
    schedule: "0 9,12,18 * * *"  # 9AM, 12PM, 6PM
    action: "generate_social_posts"
    channels: ["facebook", "telegram"]

  - name: "weekly-digest"
    schedule: "0 9 * * 0"  # Sunday 9AM
    action: "generate_weekly_report"
    channels: ["email"]
```

## Gotchas

### 1. Rate Limiting
```javascript
// Don't spam TrendRadar API
const CACHE_DURATION = 15 * 60 * 1000; // 15 minutes
const lastFetch = new Map();

async function safeFetch(endpoint) {
  if (lastFetch.get(endpoint) > Date.now() - CACHE_DURATION) {
    return cachedData.get(endpoint);
  }
  // ... fetch and cache
}
```

### 2. Sentiment Filtering
```javascript
// Always check sentiment before publishing
const blockedSentiments = ['controversial', 'negative', 'sensitive'];
const trend = await analyze_sentiment(topic);

if (blockedSentiments.includes(trend.sentiment)) {
  console.log('Skipping trend due to sentiment');
  return null;
}
```

### 3. Attribution
```javascript
// Always cite sources
const attribution = `
📢 Nguồn: TrendRadar - Theo dõi ${platform}
🔗 Link: ${sourceUrl}
`;
```

## Example Workflow Output

```
📊 TREND-TO-POST REPORT
Generated: 2024-01-15 09:00

🎯 TRENDS DETECTED: 12
📝 CONTENT CREATED: 5 posts
📱 CHANNELS: Zalo, Facebook, Telegram, Email
⏰ SCHEDULED: 8 publish times

---

📈 TOP TREND: "AI Startup raises $100M"
- Platform: Weibo, Zhihu
- Mentions: 150,000+
- Sentiment: Positive
- Engagement: Very High

📝 CONTENT CREATED:
1. Zalo post (300 chars) ✓
2. Facebook post (500 chars) ✓
3. Telegram thread (5 tweets) ✓
4. Email newsletter section ✓

⏰ SCHEDULE:
- Zalo: 09:00 (published)
- Facebook: 10:00 (queued)
- Telegram: 12:00 (queued)
- Email: 18:00 (queued)

---
✨ Ready to publish!
```
