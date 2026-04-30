# BizClaw Desktop App

The BizClaw Desktop App provides a native desktop experience for managing your AI agents, built with Tauri and React.

## Features

### 🎯 Core Features

- **Chat Interface**: Modern, responsive chat UI for agent interactions
- **Multi-Agent Management**: Create and manage multiple agents
- **Channel Integration**: Connect Telegram, Discord, Slack, and more
- **Skills Management**: Browse, install, and manage skills
- **Memory Search**: Search through conversation history
- **Browser Automation**: Built-in browser for web tasks
- **Settings & Configuration**: Full customization options

### 🖥️ System Integration

- **System Tray**: Quick access from menu bar
- **Notifications**: Real-time alerts
- **Native Dialogs**: File picker, confirmation dialogs
- **Clipboard**: Copy/paste integration

## Installation

### macOS

```bash
# Download .dmg from releases
open BizClaw-x.x.x.dmg

# Or install via command line
hdiutil attach BizClaw-x.x.x.dmg
cp -R /Volumes/BizClaw/BizClaw.app /Applications/
hdiutil detach /Volumes/BizClaw
```

### Windows

```powershell
# Download .exe installer
.\BizClaw-x.x.x.exe

# Or via winget
winget install BizClaw
```

### Linux

```bash
# Debian/Ubuntu
sudo dpkg -i bizclaw_x.x.x_amd64.deb

# Or via snap
snap install bizclaw
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     BizClaw Desktop App                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                    React Frontend                           │ │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  │ │
│  │  │  Chat    │  │ Channels │  │ Settings │  │  Skills  │  │ │
│  │  │   UI     │  │   UI     │  │   UI     │  │   UI     │  │ │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘  │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                   │
│                              │ Tauri Commands                     │
│                              ▼                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                    Rust Backend                            │ │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  │ │
│  │  │  State    │  │  IPC     │  │  Gateway │  │ Browser  │  │ │
│  │  │  Manager  │  │  Bridge  │  │  Client  │  │  Client  │  │ │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘  │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                   │
│                              ▼                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                   Local Services                          │ │
│  │  Memory Store  │  Config Store  │  Log Store             │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## UI Screenshots

### Main Chat View

```
┌─────────────────────────────────────────────────────────────────┐
│ ☰  BizClaw Desktop                              🔔  ⚙️  ─ □ ✕ │
├─────────────────┬───────────────────────────────────────────────┤
│                 │                                               │
│  Conversations  │  ┌─────────────────────────────────────────┐  │
│  ─────────────  │  │                                         │  │
│  > New Chat     │  │  AI Agent                               │  │
│                 │  │                                         │  │
│  Recent         │  │  Hello! I'm your AI assistant.           │  │
│  ├─ Project A   │  │  How can I help you today?             │  │
│  ├─ Research    │  │                                         │  │
│  └─ Code Review │  │                                         │  │
│                 │  └─────────────────────────────────────────┘  │
│  Skills         │  ┌─────────────────────────────────────────┐  │
│  ─────────────  │  │ Type your message...              ➤   │  │
│  ├─ Developer   │  └─────────────────────────────────────────┘  │
│  ├─ Business    │                                               │
│  └─ Research    │  ┌─────────────────────────────────────────┐  │
│                 │  │ 🔗 Tools: browser, file, search        │  │
│  Channels       │  └─────────────────────────────────────────┘  │
│  ─────────────  │                                               │
│  ✓ Telegram     │                                               │
│  ✓ Discord      │                                               │
│  ○ Slack        │                                               │
│                 │                                               │
└─────────────────┴───────────────────────────────────────────────┘
```

### Settings Panel

```
┌─────────────────────────────────────────────────────────────────┐
│  Settings                                                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐│
│  │ General     │ │ Provider   │ │ Appearance │ │ Channels    ││
│  ├─────────────┤ ├─────────────┤ ├─────────────┤ ├─────────────┤│
│  │ Skills     │ │ Memory     │ │ Security   │ │ About       ││
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘│
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                                                             ││
│  │  Provider Configuration                                     ││
│  │  ─────────────────────                                      ││
│  │                                                             ││
│  │  Primary Provider: [OpenAI        ▼]                        ││
│  │                                                             ││
│  │  Model:         [gpt-4          ▼]                        ││
│  │                                                             ││
│  │  Temperature:    [═══════════●══] 0.7                       ││
│  │                                                             ││
│  │  Max Tokens:   [4000         ]                           ││
│  │                                                             ││
│  │  API Key:       [•••••••••••••••••••••••••]  [Update]     ││
│  │                                                             ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                 │
│                              [Save Changes]                      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Development

### Prerequisites

- Rust 1.85+
- Node.js 22+
- pnpm 9+
- Tauri CLI

### Setup

```bash
# Clone and enter directory
git clone https://github.com/nguyenduchoai/bizclaw.git
cd bizclaw/desktop

# Install dependencies
pnpm install

# Run in development mode
pnpm tauri dev
```

### Building

```bash
# Build for current platform
pnpm tauri build

# Build specific platforms
pnpm tauri build --target x86_64-apple-darwin    # macOS
pnpm tauri build --target x86_64-pc-windows-msvc # Windows
pnpm tauri build --target x86_64-unknown-linux-gnu # Linux
```

### Project Structure

```
desktop/
├── src/
│   ├── main.rs           # Tauri entry point
│   ├── lib.rs            # Library exports
│   ├── commands.rs       # Tauri commands
│   ├── state.rs          # Application state
│   └── app.rs            # App logic
├── src-ui/               # React frontend
│   ├── src/
│   │   ├── components/   # React components
│   │   ├── pages/        # Page components
│   │   ├── hooks/        # Custom hooks
│   │   ├── stores/       # Zustand stores
│   │   └── lib/          # Utilities
│   ├── index.html
│   └── package.json
├── Cargo.toml
├── tauri.conf.json
└── package.json
```

## Commands API

The desktop app exposes these Tauri commands:

### Agent Commands

```rust
#[tauri::command]
async fn send_message(conversation_id: String, message: String) -> Result<String, String>

#[tauri::command]
async fn get_conversations() -> Result<Vec<Conversation>, String>

#[tauri::command]
async fn get_conversation(conversation_id: String) -> Result<Option<Conversation>, String>

#[tauri::command]
async fn clear_conversation(conversation_id: String) -> Result<(), String>
```

### Channel Commands

```rust
#[tauri::command]
async fn get_channels() -> Result<Vec<ChannelStatus>, String>

#[tauri::command]
async fn connect_channel(channel_id: String) -> Result<(), String>

#[tauri::command]
async fn disconnect_channel(channel_id: String) -> Result<(), String>
```

### Skills Commands

```rust
#[tauri::command]
async fn get_skills() -> Result<Vec<SkillInfo>, String>

#[tauri::command]
async fn install_skill(skill_id: String) -> Result<(), String>

#[tauri::command]
async fn uninstall_skill(skill_id: String) -> Result<(), String>
```

### Settings Commands

```rust
#[tauri::command]
async fn get_settings() -> Result<Settings, String>

#[tauri::command]
async fn update_settings(settings: Settings) -> Result<(), String>
```

### Memory Commands

```rust
#[tauri::command]
async fn search_memory(query: String) -> Result<Vec<MemoryResult>, String>

#[tauri::command]
async fn save_memory(key: String, value: String) -> Result<(), String>

#[tauri::command]
async fn get_memory_stats() -> Result<MemoryStats, String>
```

### Browser Commands

```rust
#[tauri::command]
async fn start_browser() -> Result<String, String>

#[tauri::command]
async fn close_browser(browser_id: String) -> Result<(), String>

#[tauri::command]
async fn browser_navigate(browser_id: String, url: String) -> Result<(), String>

#[tauri::command]
async fn browser_screenshot(browser_id: String) -> Result<String, String>
```

## State Management

The app uses a global state managed by Rust:

```rust
pub struct AppState {
    pub app_handle: AppHandle,
    pub gateway_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    pub conversations: Arc<RwLock<Vec<Conversation>>>,
    pub channels: Arc<RwLock<Vec<ChannelStatus>>>,
    pub memory_store: Arc<RwLock<Option<RedbStore>>>,
}
```

Frontend accesses state via Zustand stores that call Tauri commands.

## System Tray

The app runs in the system tray when minimized:

```rust
// Tray menu options
- Show/Hide Window
- New Conversation
- Settings
- Quit
```

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl/Cmd + N` | New conversation |
| `Ctrl/Cmd + K` | Search |
| `Ctrl/Cmd + ,` | Settings |
| `Ctrl/Cmd + W` | Close conversation |
| `Esc` | Minimize to tray |

## Troubleshooting

### App won't start

1. Check if another instance is running
2. Verify Rust and Node.js are installed
3. Check logs in `~/.bizclaw/logs/`

### Performance issues

1. Reduce conversation history limit
2. Clear old conversations
3. Restart the app

### Browser not working

1. Ensure Chrome/Chromium is installed
2. Check browser permissions
3. Update to latest app version

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) for development guidelines.

## License

MIT or Apache-2.0
