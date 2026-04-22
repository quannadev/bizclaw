//! LLM-as-Judge implementation

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JudgeModel {
    Gpt4o,
    Gpt4Turbo,
    Claude3Sonnet,
    Claude3Haiku,
    Gemini15Pro,
    Custom(String),
}

impl JudgeModel {
    pub fn model_id(&self) -> &str {
        match self {
            JudgeModel::Gpt4o => "gpt-4o",
            JudgeModel::Gpt4Turbo => "gpt-4-turbo",
            JudgeModel::Claude3Sonnet => "claude-3-5-sonnet-20240620",
            JudgeModel::Claude3Haiku => "claude-3-5-haiku-20240307",
            JudgeModel::Gemini15Pro => "gemini-1.5-pro",
            JudgeModel::Custom(id) => id,
        }
    }

    pub fn provider(&self) -> &str {
        match self {
            JudgeModel::Gpt4o | JudgeModel::Gpt4Turbo => "openai",
            JudgeModel::Claude3Sonnet | JudgeModel::Claude3Haiku => "anthropic",
            JudgeModel::Gemini15Pro => "google",
            JudgeModel::Custom(_) => "custom",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeConfig {
    pub model: JudgeModel,
    pub prompt_template: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub enable_cot: bool,
}

impl Default for JudgeConfig {
    fn default() -> Self {
        Self {
            model: JudgeModel::Claude3Sonnet,
            prompt_template: DEFAULT_JUDGE_PROMPT.to_string(),
            temperature: 0.1,
            max_tokens: 2048,
            enable_cot: true,
        }
    }
}

const DEFAULT_JUDGE_PROMPT: &str = r#"You are an expert evaluator judging AI agent responses.

## Task
Evaluate the following agent response against the rubric criteria.

## Input
User Query: {user_query}
Agent Response: {agent_response}
Expected Output: {expected_output}
Context: {context}

## Rubric Criteria
{criteria}

## Evaluation Instructions
For each criterion:
1. Score from 0 to max_score based on the quality
2. Provide specific evidence from the response
3. Explain your reasoning

## Output Format
Respond in JSON:
{{
    "scores": [
        {{
            "criterion": "criterion_name",
            "score": 0-5,
            "max_score": 5,
            "justification": "brief explanation",
            "evidence": ["quote from response"]
        }}
    ],
    "overall_feedback": "summary of evaluation",
    "passed": true/false
}}
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Judgment {
    pub scores: Vec<super::Score>,
    pub overall_score: f32,
    pub passed: bool,
    pub overall_feedback: String,
    pub reasoning: Vec<String>,
    pub model_used: String,
    pub latency_ms: u64,
}

pub struct Judge {
    config: JudgeConfig,
}

impl Judge {
    pub fn new(config: JudgeConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(JudgeConfig::default())
    }

    pub async fn evaluate(
        &self,
        input: &str,
        output: &str,
        expected: Option<&str>,
        context: Option<&str>,
        criteria: &[super::RubricCriteria],
    ) -> anyhow::Result<Judgment> {
        let prompt = self.build_prompt(input, output, expected, context, criteria);
        
        let start = std::time::Instant::now();
        
        let response = self.call_judge_model(&prompt).await?;
        let latency_ms = start.elapsed().as_millis() as u64;
        
        self.parse_response(&response, criteria)
            .map(|mut j| {
                j.latency_ms = latency_ms;
                j.model_used = self.config.model.model_id().to_string();
                j
            })
    }

    fn build_prompt(
        &self,
        input: &str,
        output: &str,
        expected: Option<&str>,
        context: Option<&str>,
        criteria: &[super::RubricCriteria],
    ) -> String {
        let criteria_str = criteria.iter()
            .map(|c| format!(
                "- {} ({}): {} [max: {:.0}]",
                c.name,
                match &c.scoring {
                    super::ScoringMethod::Binary => "binary".to_string(),
                    super::ScoringMethod::Scale(n) => format!("1-{}", n),
                    super::ScoringMethod::Percentage => "0-100".to_string(),
                    super::ScoringMethod::RubricLevels(l) => l.join("/"),
                },
                c.description,
                c.max_score
            ))
            .collect::<Vec<_>>()
            .join("\n");

        self.config.prompt_template
            .replace("{user_query}", input)
            .replace("{agent_response}", output)
            .replace("{expected_output}", expected.unwrap_or("N/A"))
            .replace("{context}", context.unwrap_or("N/A"))
            .replace("{criteria}", &criteria_str)
    }

    async fn call_judge_model(&self, prompt: &str) -> anyhow::Result<String> {
        let model_id = self.config.model.model_id();
        
        match self.config.model.provider() {
            "openai" => self.call_openai(model_id, prompt).await,
            "anthropic" => self.call_anthropic(model_id, prompt).await,
            "google" => self.call_google(model_id, prompt).await,
            _ => Err(anyhow::anyhow!("Unsupported provider for judge model")),
        }
    }

    async fn call_openai(&self, model_id: &str, prompt: &str) -> anyhow::Result<String> {
        let client = reqwest::Client::new();
        
        let body = serde_json::json!({
            "model": model_id,
            "messages": [{"role": "user", "content": prompt}],
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens,
            "response_format": {"type": "json_object"}
        });

        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY not set"))?;

        let response = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await?;

        let result: serde_json::Value = response.json().await?;
        let content = result["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid response format"))?;
        
        Ok(content.to_string())
    }

    async fn call_anthropic(&self, model_id: &str, prompt: &str) -> anyhow::Result<String> {
        let client = reqwest::Client::new();
        
        let body = serde_json::json!({
            "model": model_id,
            "messages": [{"role": "user", "content": prompt}],
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens,
            "system": "You are an expert AI evaluator. Always respond with valid JSON."
        });

        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not set"))?;

        let response = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let result: serde_json::Value = response.json().await?;
        let content = result["content"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid response format"))?;
        
        Ok(content.to_string())
    }

    async fn call_google(&self, model_id: &str, prompt: &str) -> anyhow::Result<String> {
        let client = reqwest::Client::new();
        
        let body = serde_json::json!({
            "contents": [{
                "parts": [{"text": prompt}]
            }],
            "generationConfig": {
                "temperature": self.config.temperature,
                "maxOutputTokens": self.config.max_tokens
            }
        });

        let api_key = std::env::var("GOOGLE_API_KEY")
            .map_err(|_| anyhow::anyhow!("GOOGLE_API_KEY not set"))?;

        let response = client
            .post(format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                model_id, api_key
            ))
            .json(&body)
            .send()
            .await?;

        let result: serde_json::Value = response.json().await?;
        let content = result["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid response format"))?;
        
        Ok(content.to_string())
    }

    fn parse_response(
        &self,
        response: &str,
        criteria: &[super::RubricCriteria],
    ) -> anyhow::Result<Judgment> {
        let json_str = self.extract_json(response)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        let scores: Vec<super::Score> = parsed["scores"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Missing 'scores' in response"))?
            .iter()
            .filter_map(|s| {
                let criterion_name = s["criterion"].as_str()?.to_string();
                let criteria_map: std::collections::HashMap<_, _> = criteria
                    .iter()
                    .map(|c| (c.name.clone(), c.clone()))
                    .collect();
                
                let criterion = criteria_map.get(&criterion_name)?;
                let raw_score = s["score"].as_f64()? as f32;
                let justification = s["justification"].as_str().map(String::from);
                let evidence: Vec<String> = s["evidence"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(|e| e.as_str().map(String::from)).collect())
                    .unwrap_or_default();

                let mut score = super::Score::new(criterion, raw_score, justification);
                score = score.with_evidence(evidence);
                Some(score)
            })
            .collect();

        let overall_score = if !scores.is_empty() {
            scores.iter().map(|s| s.normalized_score).sum::<f32>() / scores.len() as f32
        } else {
            0.0
        };

        let passed = overall_score >= 0.7;

        Ok(Judgment {
            scores,
            overall_score,
            passed,
            overall_feedback: parsed["overall_feedback"]
                .as_str()
                .unwrap_or("No feedback provided")
                .to_string(),
            reasoning: Vec::new(),
            model_used: self.config.model.model_id().to_string(),
            latency_ms: 0,
        })
    }

    fn extract_json(&self, text: &str) -> anyhow::Result<String> {
        let text = text.trim();
        
        if text.starts_with('{') {
            let end = text.rfind('}').map(|i| i + 1).unwrap_or(text.len());
            return Ok(text[..end].to_string());
        }
        
        if let Some(start) = text.find('{') {
            if let Some(end) = text.rfind('}') {
                return Ok(text[start..=end].to_string());
            }
        }

        Ok(text.to_string())
    }
}

pub mod chain_of_thought {
    

    pub fn cot_prompt() -> String {
        r#"You are an expert evaluator with strong analytical reasoning.

## Task
Evaluate the AI agent response carefully, thinking through each criterion step by step.

## Evaluation Process
1. First, understand the user's query and intent
2. Analyze the agent's response in detail
3. For each criterion, reason about whether it is satisfied
4. Provide specific evidence from the response
5. Calculate appropriate scores

## Criteria to Evaluate
{criteria}

## Input
User Query: {user_query}
Agent Response: {agent_response}
Expected: {expected_output}

## Think Aloud
Before providing your final JSON response, explain your reasoning for each criterion:

[YOUR CHAIN-OF-THOUGHT HERE]

## Final Evaluation (JSON)
Now provide your final evaluation:
{
    "scores": [...],
    "overall_feedback": "...",
    "passed": true/false
}
"#.to_string()
    }
}
