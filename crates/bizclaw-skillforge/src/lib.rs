//! # BizClaw SkillForge Engine
//! 
//! Auto-discovery và self-improvement system cho AI agents.
//! Giống AGNT SkillForge - tự động trích xuất skills từ execution traces.
//! 
//! ## Features:
//! - Trace Analyzer: Phân tích execution logs để tìm patterns
//! - Pattern Extractor: Trích xuất success/error patterns
//! - Skill Generator: Tạo SKILL.md từ patterns
//! - Auto-Install: Tự động cài skills khi cần

pub mod analyzer;
pub mod extractor;
pub mod generator;
pub mod auto_install;

pub use analyzer::TraceAnalyzer;
pub use extractor::PatternExtractor;
pub use generator::SkillGenerator;
pub use auto_install::AutoSkillInstaller;
