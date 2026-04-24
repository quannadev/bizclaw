# ADR-0004: CDP-Based Browser Automation

**Status**: Accepted  
**Date**: 2024-11-15  
**Deciders**: BizClaw Team

## Context

BizClaw needed browser automation capabilities for:
- Web scraping
- Form auto-fill
- CAPTCHA solving
- Social media automation

Requirements:
- Native Rust implementation
- No external dependencies
- Anti-detection capabilities
- CAPTCHA solving integration

## Decision

**Implement browser automation using Chrome DevTools Protocol (CDP) via WebSocket.**

```
┌─────────────┐    WebSocket    ┌─────────────┐
│   BizClaw  │◄───────────────►│   Chrome    │
│   CDP API  │    CDP JSON     │  DevTools   │
└─────────────┘                 └─────────────┘
```

## Architecture

```rust
// Core CDP Client
pub struct CdpClient {
    sender: mpsc::Sender<CdpCommand>,
}

// Stealth injection
pub struct StealthManager {
    config: StealthConfig,
    client: CdpClient,
}

// Human-like behavior
pub struct HumanBehaviorEngine {
    config: HumanBehaviorConfig,
}
```

## Rationale

### Benefits

1. **Native Rust**: No bindings to external libraries
2. **Full Control**: Direct access to all Chrome features via CDP
3. **Cross-Platform**: Works on any OS with Chrome
4. **Debuggable**: Same tools as web developers

### Trade-offs

- Requires Chrome installation
- No headless-only mode (needs full Chrome)
- Some automation signals harder to hide

## Features Implemented

### 1. Core CDP Operations

- WebSocket connection management
- Command/response handling
- Event subscription
- Auto-reconnection

### 2. Browser Tools

```rust
pub struct BrowserTools {
    pub async fn navigate(&self, url: &str) -> Result<()>;
    pub async fn click(&self, selector: &str) -> Result<()>;
    pub async fn type_text(&self, selector: &str, text: &str) -> Result<()>;
    pub async fn screenshot(&self) -> Result<String>;
    pub async fn get_text(&self, selector: &str) -> Result<String>;
}
```

### 3. Stealth/Anti-Detection

```rust
pub struct StealthConfig {
    pub remove_webdriver: bool,      // Remove navigator.webdriver
    pub canvas_noise: bool,          // Add fingerprint noise
    pub webgl_spoofing: bool,       // Fake GPU info
    pub viewport_randomization: bool, // Random viewport size
    pub timezone_spoofing: bool,    // Fake timezone
    pub human_delays: bool,         // Realistic timing
}
```

### 4. CAPTCHA Solving

```rust
pub enum CaptchaType {
    ReCaptchaV2,
    ReCaptchaV3,
    hCaptcha,
    Turnstile,
    Image,
    Slider,
}

pub struct CaptchaSolver {
    pub async fn solve(&self, captcha: &CaptchaType, screenshot: &str) -> Result<String>;
}
```

### 5. Proxy Management

```rust
pub enum RotationStrategy {
    RoundRobin,
    Random,
    LeastUsed,
    WeightedRandom,
    GeoTargeted,
}
```

## Consequences

### Positive

- Native Rust, no external dependencies
- Full Chrome capabilities via CDP
- Extensible for new browser features
- Integrated with BizClaw tool system

### Negative

- Requires Chrome installation
- Heavier than Puppeteer/Playwright
- Some websites detect CDP connections

## Related ADRs

- ADR-0001: Rust as Primary Language
- ADR-0002: Monorepo Workspace Structure

## Alternatives Considered

| Approach | Pros | Cons |
|---------|------|------|
| Playwright | Mature, cross-browser | External deps, less control |
| Puppeteer | Good API | Node.js binding |
| Selenium | Language agnostic | Slow, detectable |
| Custom CDP | Full control | More work |
