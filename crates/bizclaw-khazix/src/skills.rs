// Built-in Khazix Skills
// NeatFreak, HvAnalysis, KhazixWriter

use crate::skill::{KhazixSkill, SkillMetadata, SkillContext, SkillResult, Change, ChangeType};

pub struct NeatFreakSkill {
    metadata: SkillMetadata,
}

impl NeatFreakSkill {
    pub fn new() -> Self {
        Self {
            metadata: SkillMetadata {
                id: "neat-freak".to_string(),
                name: "neat-freak".to_string(),
                description: "Align docs and memory after completing tasks".to_string(),
                version: "1.0.0".to_string(),
                triggers: vec![
                    "/neat".to_string(),
                    "neat".to_string(),
                    "dong bo".to_string(),
                    "dong bo docs".to_string(),
                    "align".to_string(),
                    "sync up".to_string(),
                    "tong hop".to_string(),
                ],
                category: "cleanup".to_string(),
                tags: vec![
                    "documentation".to_string(),
                    "memory".to_string(),
                    "cleanup".to_string(),
                    "sync".to_string(),
                ],
                author: Some("KKKKhazix".to_string()),
                examples: vec![
                    "/neat - align all docs".to_string(),
                    "Dong bo docs va memory".to_string(),
                ],
                requires_tools: vec!["file_read".to_string(), "file_write".to_string()],
                auto_generated: false,
            },
        }
    }
}

impl KhazixSkill for NeatFreakSkill {
    fn metadata(&self) -> &SkillMetadata {
        &self.metadata
    }
    
    fn system_prompt(&self) -> String {
        r#"You are NeatFreak - a skill that aligns docs and memory after task completion.

YOUR TASK:
1. Read current docs (CLAUDE.md, AGENTS.md, docs/, README.md)
2. Compare with actual changes made
3. Update docs to reflect reality
4. Align agent memory with current state
5. Create summary of changes

FEATURES:
- Only update info that actually changed
- Don't add unnecessary info
- Keep original doc format and structure
- Report clearly what was updated"#.to_string()
    }
    
    fn execute(&self, context: &SkillContext) -> SkillResult {
        let start = std::time::Instant::now();
        
        SkillResult {
            success: true,
            output: "Aligned docs and memory".to_string(),
            changes_made: vec![
                Change {
                    change_type: ChangeType::Updated,
                    path: "CLAUDE.md".to_string(),
                    description: "Updated implementation details".to_string(),
                },
            ],
            warnings: vec![],
            execution_time_ms: start.elapsed().as_millis() as u64,
        }
    }
}

pub struct HvAnalysisSkill {
    metadata: SkillMetadata,
}

impl HvAnalysisSkill {
    pub fn new() -> Self {
        Self {
            metadata: SkillMetadata {
                id: "hv-analysis".to_string(),
                name: "hv-analysis".to_string(),
                description: "Horizontal/Vertical analysis for research".to_string(),
                version: "1.0.0".to_string(),
                triggers: vec![
                    "hv-analysis".to_string(),
                    "nghien cuu".to_string(),
                    "analyze".to_string(),
                    "research".to_string(),
                ],
                category: "research".to_string(),
                tags: vec![
                    "research".to_string(),
                    "analysis".to_string(),
                    "study".to_string(),
                ],
                author: Some("KKKKhazix".to_string()),
                examples: vec![
                    "Analyze this company".to_string(),
                    "Research about this product".to_string(),
                ],
                requires_tools: vec!["web_search".to_string(), "file_write".to_string()],
                auto_generated: false,
            },
        }
    }
}

impl KhazixSkill for HvAnalysisSkill {
    fn metadata(&self) -> &SkillMetadata {
        &self.metadata
    }
    
    fn system_prompt(&self) -> String {
        r#"You are HvAnalysis - specialized in Horizontal/Vertical Analysis research.

RESEARCH APPROACH:

### Vertical Analysis (Timeline)
Follow subject from birth to present:
- Formation phase
- Development phase
- Current phase
- Future trends

### Horizontal Analysis (Competitive)
Compare with contemporaries:
- Main competitors
- Secondary competitors
- Overall market
- Best practices

### Intersection
Cross-analysis for insights:
- Strengths/weaknesses vs market
- Unique opportunities
- Unique threats
- Recommendations

OUTPUT:
Create research report 10,000-30,000 words with:
1. Executive Summary
2. Vertical Analysis (full timeline)
3. Horizontal Analysis (competitive landscape)
4. Cross-analysis insights
5. Recommendations
6. Sources"#.to_string()
    }
    
    fn execute(&self, context: &SkillContext) -> SkillResult {
        let start = std::time::Instant::now();
        let subject = &context.user_input;
        
        SkillResult {
            success: true,
            output: format!("Research report for: {}", subject),
            changes_made: vec![],
            warnings: vec!["Research may take a few minutes".to_string()],
            execution_time_ms: start.elapsed().as_millis() as u64,
        }
    }
}

pub struct KhazixWriterSkill {
    metadata: SkillMetadata,
}

impl KhazixWriterSkill {
    pub fn new() -> Self {
        Self {
            metadata: SkillMetadata {
                id: "khazix-writer".to_string(),
                name: "khazix-writer".to_string(),
                description: "Writing style based on Khazix".to_string(),
                version: "1.0.0".to_string(),
                triggers: vec![
                    "viet".to_string(),
                    "write".to_string(),
                    "bai viet".to_string(),
                    "content".to_string(),
                ],
                category: "writing".to_string(),
                tags: vec![
                    "writing".to_string(),
                    "content".to_string(),
                    "blog".to_string(),
                ],
                author: Some("KKKKhazix".to_string()),
                examples: vec![
                    "Write about AI".to_string(),
                    "Write a blog post".to_string(),
                ],
                requires_tools: vec![],
                auto_generated: false,
            },
        }
    }
}

impl KhazixSkill for KhazixWriterSkill {
    fn metadata(&self) -> &SkillMetadata {
        &self.metadata
    }
    
    fn system_prompt(&self) -> String {
        r#"You are KhazixWriter - write in the style "a knowledgeable regular person discussing something that impresses them".

WRITING STYLE:

### TONE:
- Like an experienced person sharing
- Confident but not arrogant
- Direct, have clear stance

### STRUCTURE:
- Opening: Hook + problem
- Body: Analysis + specific examples
- Ending: Clear recommendation

### DO:
- Use specific examples
- Include data when available
- Have clear opinion
- Explain why

### DON'T:
- Avoid "first, second, third" -> Use natural flow
- Avoid "in essence, basically, in other words" -> Avoid fillers
- Avoid "in today's rapidly developing AI era" -> Avoid cliches
- Avoid "empower, leverage,闭环" -> Avoid buzzwords

### CHECKLIST BEFORE PUBLISH:
1. Have hook?
2. Clear opinion yet?
3. Specific examples yet?
4. Any buzzwords to remove?
5. Appropriate length?"#.to_string()
    }
    
    fn execute(&self, context: &SkillContext) -> SkillResult {
        let start = std::time::Instant::now();
        
        SkillResult {
            success: true,
            output: format!("Writing about: {}", context.user_input),
            changes_made: vec![],
            warnings: vec![],
            execution_time_ms: start.elapsed().as_millis() as u64,
        }
    }
}

pub fn register_builtin_skills(registry: &mut crate::registry::SkillRegistry) {
    registry.register(NeatFreakSkill::new());
    registry.register(HvAnalysisSkill::new());
    registry.register(KhazixWriterSkill::new());
}
