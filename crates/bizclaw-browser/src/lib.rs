pub mod cdp;
pub mod tools;
pub mod skills;
pub mod session;
pub mod stealth;
pub mod error;

pub use cdp::CdpClient;
pub use tools::{BrowserTools, BrowserToolResult};
pub use skills::{BrowserSkill, SkillRegistry};
pub use session::{BrowserSession, SessionConfig, ViewportConfig, SessionInfo, SessionManager};
pub use stealth::{StealthConfig, StealthManager};
pub use error::{BrowserError, Result};
