//! # BizClaw Red Teamer
//!
//! Security testing framework for AI agents - prompt injection, jailbreak, data exfiltration.
//!
//! ## Features
//! - **Prompt Injection Testing**: Detect susceptibility to hidden instructions
//! - **Jailbreak Detection**: Test for directive override attempts
//! - **Data Exfiltration**: Verify sensitive data protection
//! - **Tool Misuse Scenarios**: Test improper tool usage
//! - **Compliance Checks**: Validate against regulatory requirements (GDPR, CCPA, Luật 134/2025)
//! - **Attack Surface Analysis**: Map potential vulnerabilities
//!
//! ## Architecture
//! ```text
//! bizclaw-redteam/
//! ├── injection.rs     # Prompt injection scenarios
//! ├── jailbreak.rs     # Jailbreak attempts
//! ├── exfiltration.rs  # Data leakage tests
//! ├── compliance.rs    # Regulatory compliance
//! ├── scenarios.rs     # Attack scenario library
//! ├── scanner.rs       # Vulnerability scanner
//! └── report.rs        # Security report generation
//! ```

pub mod injection;
pub mod jailbreak;
pub mod exfiltration;
pub mod compliance;
pub mod scenarios;
pub mod scanner;
pub mod report;

pub use injection::{InjectionType, InjectionResult, InjectionDetector};
pub use jailbreak::{JailbreakAttempt, JailbreakResult};
pub use exfiltration::{ExfiltrationTarget, ExfiltrationResult};
pub use compliance::{ComplianceFramework, ComplianceCheck, ComplianceReport};
pub use scenarios::{Scenario, ScenarioLibrary, AttackVector};
pub use scanner::{SecurityScanner, ScanResult, ScanReport, ScanSummary, Severity};
pub use report::{SecurityReport, Vulnerability, Recommendation};
