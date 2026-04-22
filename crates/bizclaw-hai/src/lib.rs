//! # BizClaw HAI Designer
//!
//! Human-AI Interaction Designer - Conversation flows, confidence indicators, rollback.
//!
//! ## Features
//! - **Conversation Flow Designer**: Visual/state-based conversation state machine
//! - **Confidence Indicators**: Show agent confidence to users
//! - **Handoff Management**: Graceful human-in-the-loop transitions
//! - **Undo/Rollback**: Revert agent actions
//! - **Fallback Strategies**: Escalation when agent is uncertain
//! - **User Trust Modeling**: Build trust through transparency
//!
//! ## Architecture
//! ```text
//! bizclaw-hai/
//! ├── flow.rs          # Conversation flow/state machine
//! ├── confidence.rs    # Confidence indicators and thresholds
//! ├── handoff.rs       # Human-agent handoff
//! ├── rollback.rs      # Action undo/rollback
//! ├── fallback.rs      # Fallback strategies
//! └── trust.rs         # User trust modeling
//! ```

pub mod flow;
pub mod confidence;
pub mod handoff;
pub mod rollback;
pub mod fallback;
pub mod trust;

pub use flow::{ConversationFlow, FlowState, FlowTransition, FlowBuilder};
pub use confidence::{ConfidenceIndicator, ConfidenceLevel, ConfidenceConfig};
pub use handoff::{HandoffManager, HandoffTrigger, HandoffResult};
pub use rollback::{RollbackManager, ActionSnapshot, RollbackResult};
pub use fallback::{FallbackStrategy, FallbackTrigger, FallbackResult};
pub use trust::{TrustModel, TrustScore, TrustFactors};
