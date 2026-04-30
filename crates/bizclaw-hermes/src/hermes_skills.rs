// Hermes Agent + Skills Integration
// Skills can be added dynamically

use crate::agent::{HermesAgent, AgentConfig, AgentResponse, AgentError};
use crate::chat::ChatSession;

pub struct HermesAgentWithSkills {
    pub agent: HermesAgent,
    skills: Vec<SkillInfo>,
    session: ChatSession,
}

#[derive(Debug, Clone)]
pub struct SkillInfo {
    pub id: String,
    pub name: String,
    pub triggers: Vec<String>,
    pub system_prompt: String,
}

impl HermesAgentWithSkills {
    pub fn new(config: AgentConfig) -> Self {
        let agent = HermesAgent::new(config);
        let mut skills = Vec::new();
        skills.push(SkillInfo::neat_freak());
        skills.push(SkillInfo::hv_analysis());
        skills.push(SkillInfo::khazix_writer());
        
        Self { agent, skills, session: ChatSession::new() }
    }
    
    pub fn add_skill(&mut self, skill: SkillInfo) {
        self.skills.push(skill);
    }
    
    pub fn find_skill(&self, input: &str) -> Option<&SkillInfo> {
        let input_lower = input.to_lowercase();
        self.skills.iter().find(|s| {
            s.triggers.iter().any(|t| input_lower.contains(t))
        })
    }
    
    pub fn list_skills(&self) -> Vec<&SkillInfo> {
        self.skills.iter().collect()
    }
    
    pub fn search_skills(&self, query: &str) -> Vec<&SkillInfo> {
        let query_lower = query.to_lowercase();
        self.skills.iter()
            .filter(|s| s.name.to_lowercase().contains(&query_lower))
            .collect()
    }
    
    pub async fn chat(&self, message: &str) -> Result<AgentResponse, AgentError> {
        if let Some(skill) = self.find_skill(message) {
            Ok(AgentResponse {
                content: format!("[Skill: {}]\n\n{}", skill.name, skill.system_prompt),
                tool_calls: None,
                session_id: self.session.id.clone(),
                tokens_used: 0,
                model: skill.id.clone(),
            })
        } else {
            self.agent.chat(message).await
        }
    }
}

impl SkillInfo {
    pub fn neat_freak() -> Self {
        Self {
            id: "neat-freak".to_string(),
            name: "NeatFreak".to_string(),
            triggers: vec!["/neat".to_string(), "neat".to_string(), "align".to_string(), "dong bo".to_string(), "sync up".to_string()],
            system_prompt: "You are NeatFreak - align docs and memory after task completion.".to_string(),
        }
    }
    
    pub fn hv_analysis() -> Self {
        Self {
            id: "hv-analysis".to_string(),
            name: "HV Analysis".to_string(),
            triggers: vec!["hv-analysis".to_string(), "research".to_string(), "analyze".to_string()],
            system_prompt: "You are HvAnalysis - horizontal/vertical research methodology.".to_string(),
        }
    }
    
    pub fn khazix_writer() -> Self {
        Self {
            id: "khazix-writer".to_string(),
            name: "KhazixWriter".to_string(),
            triggers: vec!["viet".to_string(), "write".to_string(), "bai viet".to_string()],
            system_prompt: "You are KhazixWriter - knowledgeable person's writing style.".to_string(),
        }
    }
    
    pub fn custom(id: &str, name: &str, triggers: Vec<String>, prompt: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            triggers,
            system_prompt: prompt.to_string(),
        }
    }
}
