//! Report generation for evaluations

use crate::EvaluationRun;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportFormat {
    Json,
    Html,
    Markdown,
    Csv,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub id: String,
    pub title: String,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub runs: Vec<EvaluationRunSummary>,
    pub trends: Option<TrendAnalysis>,
    pub recommendations: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Report {
    pub fn new(title: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.to_string(),
            generated_at: chrono::Utc::now(),
            runs: Vec::new(),
            trends: None,
            recommendations: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn add_run(&mut self, run: EvaluationRun) {
        self.runs.push(run.into());
    }

    pub fn generate_markdown(&self) -> String {
        let mut md = format!(
            "# {}\n\n\
             Generated: {}\n\n\
             ## Summary\n\n",
            self.title,
            self.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
        );

        for run in &self.runs {
            md.push_str(&format!(
                "### Run {} - {}\n\n\
                 | Metric | Value |\n\
                 |--------|-------|\n\
                 | Dataset | {} |\n\
                 | Samples | {} |\n\
                 | Pass Rate | {:.1}% |\n\
                 | Average Score | {:.2} |\n\n",
                run.id,
                run.timestamp.format("%Y-%m-%d %H:%M"),
                run.dataset_id,
                run.total_samples,
                run.pass_rate * 100.0,
                run.average_score
            ));
        }

        if !self.recommendations.is_empty() {
            md.push_str("## Recommendations\n\n");
            for (i, rec) in self.recommendations.iter().enumerate() {
                md.push_str(&format!("{}. {}\n", i + 1, rec));
            }
        }

        md
    }

    pub fn generate_html(&self) -> String {
        let markdown = self.generate_markdown();
        let body = markdown_to_html(&markdown);
        
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>{}</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; 
                max-width: 900px; margin: 0 auto; padding: 20px; }}
        table {{ border-collapse: collapse; width: 100%; margin: 20px 0; }}
        th, td {{ border: 1px solid #ddd; padding: 12px; text-align: left; }}
        th {{ background-color: #4a90d9; color: white; }}
        tr:nth-child(even) {{ background-color: #f9f9f9; }}
        .pass {{ color: #28a745; font-weight: bold; }}
        .fail {{ color: #dc3545; font-weight: bold; }}
        .metric {{ display: inline-block; margin: 10px 20px; }}
        .metric-value {{ font-size: 24px; font-weight: bold; color: #333; }}
        .metric-label {{ font-size: 12px; color: #666; text-transform: uppercase; }}
    </style>
</head>
<body>
    <h1>{}</h1>
    <p>Generated: {}</p>
    {}
</body>
</html>"#,
            self.title, self.title, self.generated_at.format("%Y-%m-%d %H:%M:%S UTC"), body
        )
    }

    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationRunSummary {
    pub id: String,
    pub dataset_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub total_samples: usize,
    pub pass_count: usize,
    pub pass_rate: f32,
    pub average_score: f32,
}

impl From<EvaluationRun> for EvaluationRunSummary {
    fn from(run: EvaluationRun) -> Self {
        Self {
            id: run.id,
            dataset_id: run.dataset_id,
            timestamp: run.timestamp,
            total_samples: run.total_samples,
            pass_count: run.pass_count,
            pass_rate: run.pass_rate,
            average_score: run.average_score,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    pub metric: String,
    pub direction: TrendDirection,
    pub change_percent: f32,
    pub is_significant: bool,
    pub data_points: Vec<TrendPoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrendDirection {
    Improving,
    Declining,
    Stable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendPoint {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub value: f32,
}

impl TrendAnalysis {
    pub fn analyze(
        metric: &str,
        runs: &[EvaluationRunSummary],
    ) -> Self {
        if runs.len() < 2 {
            return Self {
                metric: metric.to_string(),
                direction: TrendDirection::Stable,
                change_percent: 0.0,
                is_significant: false,
                data_points: Vec::new(),
            };
        }

        let data_points: Vec<TrendPoint> = runs.iter()
            .map(|r| TrendPoint {
                timestamp: r.timestamp,
                value: match metric {
                    "pass_rate" => r.pass_rate,
                    "average_score" => r.average_score,
                    _ => r.pass_rate,
                },
            })
            .collect();

        let first = data_points.first().map(|p| p.value).unwrap_or(0.0);
        let last = data_points.last().map(|p| p.value).unwrap_or(0.0);
        
        let change_percent = if first != 0.0 {
            ((last - first) / first) * 100.0
        } else {
            0.0
        };

        let direction = if change_percent.abs() < 1.0 {
            TrendDirection::Stable
        } else if change_percent > 0.0 {
            TrendDirection::Improving
        } else {
            TrendDirection::Declining
        };

        let is_significant = change_percent.abs() > 5.0;

        Self {
            metric: metric.to_string(),
            direction,
            change_percent,
            is_significant,
            data_points,
        }
    }
}

fn markdown_to_html(md: &str) -> String {
    let mut html = md.to_string();
    
    html = html.replace("# ", "<h1>").replace("\n", "</h1>\n");
    html = html.replace("## ", "<h2>").replace("\n", "</h2>\n");
    html = html.replace("### ", "<h3>").replace("\n", "</h3>\n");
    html = html.replace("**", "<strong>").replace("**", "</strong>");
    html = html.replace("*", "<em>").replace("*", "</em>");
    html = html.replace("`", "");
    
    let lines: Vec<&str> = html.lines().collect();
    let mut in_table = false;
    let mut result: Vec<String> = Vec::new();
    
    for line in lines {
        if line.contains('|') && line.starts_with('|') {
            if !in_table {
                result.push("<table>".to_string());
                in_table = true;
            }
            if line.contains("---") {
                continue;
            }
            let cells: Vec<&str> = line.split('|')
                .filter(|s| !s.trim().is_empty())
                .collect();
            let tag = if result.last().map(|s| s.contains("<th>")).unwrap_or(false) {
                "td"
            } else {
                "th"
            };
            let row = format!(
                "<tr>{}</tr>",
                cells.iter()
                    .map(|c| format!("<{}>{}</{}>", tag, c.trim(), tag))
                    .collect::<Vec<_>>()
                    .join("")
            );
            result.push(row);
            if !result.last().map(|s| s.contains("<th>")).unwrap_or(false) {
                result.push("</table>".to_string());
                in_table = false;
            }
        } else {
            if in_table {
                result.push("</table>".to_string());
                in_table = false;
            }
            result.push(line.to_string());
        }
    }

    result.join("\n")
}
