//! Security report generation

use crate::{ScanReport, ScanResult, Severity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    pub id: String,
    pub title: String,
    pub description: String,
    pub severity: String,
    pub cvss_score: f32,
    pub attack_vector: String,
    pub status: VulnerabilityStatus,
    pub evidence: Vec<String>,
    pub impact: String,
    pub remediation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VulnerabilityStatus {
    Open,
    InProgress,
    Resolved,
    Accepted,
    FalsePositive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub priority: Priority,
    pub category: String,
    pub title: String,
    pub description: String,
    pub effort: String,
    pub impact: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

impl Priority {
    pub fn level(&self) -> u8 {
        match self {
            Priority::Critical => 4,
            Priority::High => 3,
            Priority::Medium => 2,
            Priority::Low => 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityReport {
    pub report_id: String,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub executive_summary: ExecutiveSummary,
    pub vulnerabilities: Vec<Vulnerability>,
    pub recommendations: Vec<Recommendation>,
    pub scan_details: ScanReport,
    pub compliance_status: HashMap<String, bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutiveSummary {
    pub overall_risk_level: RiskLevel,
    pub total_vulnerabilities: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub low_count: usize,
    pub security_score: f32,
    pub key_findings: Vec<String>,
    pub next_steps: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Critical,
    High,
    Medium,
    Low,
    Minimal,
}

impl RiskLevel {
    pub fn from_score(score: f32) -> Self {
        match score as u8 {
            0..=20 => RiskLevel::Critical,
            21..=40 => RiskLevel::High,
            41..=60 => RiskLevel::Medium,
            61..=80 => RiskLevel::Low,
            _ => RiskLevel::Minimal,
        }
    }
}

impl SecurityReport {
    pub fn from_scan_report(scan_report: ScanReport) -> Self {
        let mut vulnerabilities = Vec::new();
        let mut recommendations: Vec<(Priority, String)> = Vec::new();

        for result in &scan_report.results {
            let vulnerability = Vulnerability {
                id: result.vulnerability_id.clone(),
                title: result.name.clone(),
                description: result.description.clone(),
                severity: format!("{:?}", result.severity),
                cvss_score: result.severity.score() as f32,
                attack_vector: format!("{:?}", result.attack_vector),
                status: VulnerabilityStatus::Open,
                evidence: result.evidence.clone(),
                impact: format!("Agent is susceptible to {:?}", result.attack_vector),
                remediation: result.recommendations.join("; "),
            };
            vulnerabilities.push(vulnerability);

            for rec in &result.recommendations {
                let priority: Priority = match result.severity {
                    Severity::Critical => Priority::Critical,
                    Severity::High => Priority::High,
                    Severity::Medium => Priority::Medium,
                    _ => Priority::Low,
                };
                let rec_string: String = rec.to_string();
                recommendations.push((priority, rec_string));
            }
        }

        recommendations.sort_by(|a, b| b.0.level().cmp(&a.0.level()));
        recommendations.dedup_by(|a, b| a.1 == b.1);

        let recommendations: Vec<Recommendation> = recommendations.into_iter().map(|(p, t)| {
            Recommendation {
                priority: p,
                category: "Security".to_string(),
                title: t.clone(),
                description: format!("Implement: {}", t),
                effort: match p {
                    Priority::Critical | Priority::High => "High",
                    Priority::Medium => "Medium",
                    Priority::Low => "Low",
                }.to_string(),
                impact: match p {
                    Priority::Critical | Priority::High => "High",
                    Priority::Medium => "Medium",
                    Priority::Low => "Low",
                }.to_string(),
            }
        }).collect();

        let critical_count = vulnerabilities.iter()
            .filter(|v| v.severity == "Critical")
            .count();
        let high_count = vulnerabilities.iter()
            .filter(|v| v.severity == "High")
            .count();
        let medium_count = vulnerabilities.iter()
            .filter(|v| v.severity == "Medium")
            .count();
        let low_count = vulnerabilities.iter()
            .filter(|v| v.severity == "Low")
            .count();

        let security_score = scan_report.summary.overall_score;

        let key_findings = if critical_count > 0 {
            vec![format!("{} critical vulnerabilities require immediate attention", critical_count)]
        } else if high_count > 0 {
            vec![format!("{} high-severity vulnerabilities need remediation", high_count)]
        } else {
            vec!["No critical or high vulnerabilities detected".to_string()]
        };

        let executive_summary = ExecutiveSummary {
            overall_risk_level: RiskLevel::from_score(100.0 - security_score),
            total_vulnerabilities: vulnerabilities.len(),
            critical_count,
            high_count,
            medium_count,
            low_count,
            security_score,
            key_findings,
            next_steps: vec![
                "Review and remediate critical vulnerabilities".to_string(),
                "Implement security controls for identified risks".to_string(),
                "Schedule follow-up scan after fixes".to_string(),
            ],
        };

        Self {
            report_id: uuid::Uuid::new_v4().to_string(),
            generated_at: chrono::Utc::now(),
            executive_summary,
            vulnerabilities,
            recommendations,
            scan_details: scan_report,
            compliance_status: HashMap::new(),
        }
    }

    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn to_markdown(&self) -> String {
        let mut md = format!(
            "# Security Assessment Report\n\n\
             **Report ID:** {}\n\
             **Generated:** {}\n\n\
             ## Executive Summary\n\n\
             **Risk Level:** {:?}\n\
             **Security Score:** {:.1}%\n\n\
             | Severity | Count |\n\
             |----------|-------|\n\
             | Critical | {} |\n\
             | High | {} |\n\
             | Medium | {} |\n\
             | Low | {} |\n\
             | Total | {} |\n\n",
            self.report_id,
            self.generated_at.format("%Y-%m-%d %H:%M:%S UTC"),
            self.executive_summary.overall_risk_level,
            self.executive_summary.security_score,
            self.executive_summary.critical_count,
            self.executive_summary.high_count,
            self.executive_summary.medium_count,
            self.executive_summary.low_count,
            self.executive_summary.total_vulnerabilities
        );

        if !self.executive_summary.key_findings.is_empty() {
            md.push_str("### Key Findings\n\n");
            for finding in &self.executive_summary.key_findings {
                md.push_str(&format!("- {}\n", finding));
            }
            md.push('\n');
        }

        if !self.recommendations.is_empty() {
            md.push_str("## Recommendations\n\n");
            for rec in &self.recommendations {
                md.push_str(&format!(
                    "### [{}] {}\n\n{}\n\n**Effort:** {} | **Impact:** {}\n\n",
                    format!("{:?}", rec.priority).to_uppercase(),
                    rec.title,
                    rec.description,
                    rec.effort,
                    rec.impact
                ));
            }
        }

        if !self.vulnerabilities.is_empty() {
            md.push_str("## Vulnerabilities\n\n");
            for vuln in &self.vulnerabilities {
                md.push_str(&format!(
                    "### {} - {}\n\n\
                     **Severity:** {} | **CVSS:** {:.1}\n\n\
                     {}\n\n\
                     **Remediation:** {}\n\n",
                    vuln.id,
                    vuln.title,
                    vuln.severity,
                    vuln.cvss_score,
                    vuln.description,
                    vuln.remediation
                ));
            }
        }

        md
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_generation() {
        let scan_report = crate::ScanReport {
            scan_id: "test_scan".to_string(),
            timestamp: chrono::Utc::now(),
            agent_config: HashMap::new(),
            results: Vec::new(),
            summary: crate::ScanSummary {
                total_tests: 10,
                passed_tests: 7,
                failed_tests: 3,
                critical_vulnerabilities: 0,
                high_vulnerabilities: 1,
                medium_vulnerabilities: 2,
                low_vulnerabilities: 0,
                overall_score: 70.0,
            },
        };

        let report = SecurityReport::from_scan_report(scan_report);
        assert_eq!(report.executive_summary.security_score, 70.0);
    }

    #[test]
    fn test_risk_level() {
        assert_eq!(RiskLevel::from_score(15.0), RiskLevel::Critical);
        assert_eq!(RiskLevel::from_score(35.0), RiskLevel::High);
        assert_eq!(RiskLevel::from_score(55.0), RiskLevel::Medium);
        assert_eq!(RiskLevel::from_score(75.0), RiskLevel::Low);
        assert_eq!(RiskLevel::from_score(95.0), RiskLevel::Minimal);
    }
}
