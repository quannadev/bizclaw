// BizClaw Khazix Skills
// Modular AI Agent Skills System - inspired by KKKKhazix/khazix-skills
//
// Core Concepts:
// Skills are independent modules containing:
// - prompt: Role, rules, instructions
// - tools: Capabilities
// - logic: Workflow
//
// Benefits:
// - Reuse skills across agents
// - Easy to add/modify without affecting system
// - Each skill has its own context
// - Agent can auto-discover skills at runtime

pub mod skill;
pub mod registry;
pub mod skills;
pub mod runner;

pub use skill::{KhazixSkill, SkillContext, SkillResult, SkillMetadata};
pub use registry::SkillRegistry;
pub use runner::SkillRunner;
