//! Compliance checking against regulatory frameworks

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceFramework {
    Gdpr,
    Ccpa,
    Vietnam134,
    Soc2,
    Iso27001,
}

impl ComplianceFramework {
    pub fn name(&self) -> &str {
        match self {
            ComplianceFramework::Gdpr => "GDPR",
            ComplianceFramework::Ccpa => "CCPA",
            ComplianceFramework::Vietnam134 => "Luật An Ninh Mạng 134/2025",
            ComplianceFramework::Soc2 => "SOC 2",
            ComplianceFramework::Iso27001 => "ISO 27001",
        }
    }

    pub fn region(&self) -> &str {
        match self {
            ComplianceFramework::Gdpr => "EU",
            ComplianceFramework::Ccpa => "US (California)",
            ComplianceFramework::Vietnam134 => "Vietnam",
            ComplianceFramework::Soc2 => "Global",
            ComplianceFramework::Iso27001 => "Global",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceCheck {
    pub id: String,
    pub framework: ComplianceFramework,
    pub requirement: String,
    pub description: String,
    pub check_type: CheckType,
    pub severity: ComplianceSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckType {
    DataRetention,
    Consent,
    Encryption,
    AccessControl,
    AuditLogging,
    DataMinimization,
    BreachNotification,
    UserRights,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceSeverity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub framework: ComplianceFramework,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub overall_score: f32,
    pub total_checks: usize,
    pub passed_checks: usize,
    pub failed_checks: Vec<ComplianceCheck>,
    pub warnings: Vec<String>,
    pub recommendations: Vec<String>,
}

impl ComplianceReport {
    pub fn new(framework: ComplianceFramework) -> Self {
        Self {
            framework,
            timestamp: chrono::Utc::now(),
            overall_score: 0.0,
            total_checks: 0,
            passed_checks: 0,
            failed_checks: Vec::new(),
            warnings: Vec::new(),
            recommendations: Vec::new(),
        }
    }

    pub fn add_check(&mut self, check: ComplianceCheck, passed: bool) {
        self.total_checks += 1;
        if passed {
            self.passed_checks += 1;
        } else {
            self.failed_checks.push(check);
        }
    }

    pub fn calculate_score(&mut self) {
        if self.total_checks > 0 {
            self.overall_score = self.passed_checks as f32 / self.total_checks as f32;
        }
    }

    pub fn generate_recommendations(&mut self) {
        for failed in &self.failed_checks {
            let rec = match failed.check_type {
                CheckType::DataRetention => {
                    format!("Implement data retention policy for {}", failed.requirement)
                },
                CheckType::Consent => {
                    format!("Add explicit consent mechanism for {}", failed.requirement)
                },
                CheckType::Encryption => {
                    format!("Ensure encryption at rest and in transit for {}", failed.requirement)
                },
                CheckType::AccessControl => {
                    format!("Implement role-based access control for {}", failed.requirement)
                },
                CheckType::AuditLogging => {
                    format!("Enable comprehensive audit logging for {}", failed.requirement)
                },
                CheckType::DataMinimization => {
                    format!("Reduce data collection to minimum necessary for {}", failed.requirement)
                },
                CheckType::BreachNotification => {
                    format!("Implement breach detection and notification for {}", failed.requirement)
                },
                CheckType::UserRights => {
                    format!("Enable user data rights (access, delete, export) for {}", failed.requirement)
                },
            };
            self.recommendations.push(rec);
        }
    }
}

pub struct ComplianceChecker {
    framework: ComplianceFramework,
    checks: Vec<ComplianceCheck>,
}

impl ComplianceChecker {
    pub fn new(framework: ComplianceFramework) -> Self {
        let checks = Self::get_checks_for_framework(framework);
        Self { framework, checks }
    }

    fn get_checks_for_framework(framework: ComplianceFramework) -> Vec<ComplianceCheck> {
        match framework {
            ComplianceFramework::Vietnam134 => {
                vec![
                    ComplianceCheck {
                        id: "VN134_001".to_string(),
                        framework,
                        requirement: "Data Localization".to_string(),
                        description: "Personal data of Vietnamese citizens must be stored domestically".to_string(),
                        check_type: CheckType::DataRetention,
                        severity: ComplianceSeverity::Critical,
                    },
                    ComplianceCheck {
                        id: "VN134_002".to_string(),
                        framework,
                        requirement: "Cybersecurity Incident Reporting".to_string(),
                        description: "Report cybersecurity incidents to authorities within 24 hours".to_string(),
                        check_type: CheckType::BreachNotification,
                        severity: ComplianceSeverity::Critical,
                    },
                    ComplianceCheck {
                        id: "VN134_003".to_string(),
                        framework,
                        requirement: "AI Content Moderation".to_string(),
                        description: "AI systems must have content filtering and moderation".to_string(),
                        check_type: CheckType::AccessControl,
                        severity: ComplianceSeverity::High,
                    },
                    ComplianceCheck {
                        id: "VN134_004".to_string(),
                        framework,
                        requirement: "User Consent for AI Processing".to_string(),
                        description: "Obtain explicit consent before AI processing of user data".to_string(),
                        check_type: CheckType::Consent,
                        severity: ComplianceSeverity::High,
                    },
                    ComplianceCheck {
                        id: "VN134_005".to_string(),
                        framework,
                        requirement: "Audit Trail for AI Decisions".to_string(),
                        description: "Maintain audit trail for AI-assisted decisions".to_string(),
                        check_type: CheckType::AuditLogging,
                        severity: ComplianceSeverity::Medium,
                    },
                ]
            },
            ComplianceFramework::Gdpr => {
                vec![
                    ComplianceCheck {
                        id: "GDPR_001".to_string(),
                        framework,
                        requirement: "Right to be Informed".to_string(),
                        description: "Users must be informed about AI processing of their data".to_string(),
                        check_type: CheckType::Consent,
                        severity: ComplianceSeverity::High,
                    },
                    ComplianceCheck {
                        id: "GDPR_002".to_string(),
                        framework,
                        requirement: "Data Portability".to_string(),
                        description: "Users can request their data in machine-readable format".to_string(),
                        check_type: CheckType::UserRights,
                        severity: ComplianceSeverity::High,
                    },
                    ComplianceCheck {
                        id: "GDPR_003".to_string(),
                        framework,
                        requirement: "Right to be Forgotten".to_string(),
                        description: "Users can request deletion of their data".to_string(),
                        check_type: CheckType::UserRights,
                        severity: ComplianceSeverity::High,
                    },
                    ComplianceCheck {
                        id: "GDPR_004".to_string(),
                        framework,
                        requirement: "Data Protection by Design".to_string(),
                        description: "AI systems must implement data protection by design".to_string(),
                        check_type: CheckType::Encryption,
                        severity: ComplianceSeverity::High,
                    },
                ]
            },
            ComplianceFramework::Ccpa => {
                vec![
                    ComplianceCheck {
                        id: "CCPA_001".to_string(),
                        framework,
                        requirement: "Right to Know".to_string(),
                        description: "Users can request to know what data is collected".to_string(),
                        check_type: CheckType::UserRights,
                        severity: ComplianceSeverity::High,
                    },
                    ComplianceCheck {
                        id: "CCPA_002".to_string(),
                        framework,
                        requirement: "Right to Delete".to_string(),
                        description: "Users can request deletion of their data".to_string(),
                        check_type: CheckType::UserRights,
                        severity: ComplianceSeverity::High,
                    },
                    ComplianceCheck {
                        id: "CCPA_003".to_string(),
                        framework,
                        requirement: "Opt-Out of Sale".to_string(),
                        description: "Users can opt out of sale of their data".to_string(),
                        check_type: CheckType::Consent,
                        severity: ComplianceSeverity::High,
                    },
                ]
            },
            _ => Vec::new(),
        }
    }

    pub fn get_checks(&self) -> &[ComplianceCheck] {
        &self.checks
    }

    pub fn evaluate(&self, agent_config: &HashMap<String, serde_json::Value>) -> ComplianceReport {
        let mut report = ComplianceReport::new(self.framework);

        for check in &self.checks {
            let passed = self.evaluate_check(check, agent_config);
            report.add_check(check.clone(), passed);
        }

        report.calculate_score();
        report.generate_recommendations();

        report
    }

    fn evaluate_check(&self, check: &ComplianceCheck, config: &HashMap<String, serde_json::Value>) -> bool {
        match check.check_type {
            CheckType::Consent => {
                config.get("consent_required").and_then(|v| v.as_bool()).unwrap_or(false)
            },
            CheckType::Encryption => {
                config.get("encryption_enabled").and_then(|v| v.as_bool()).unwrap_or(true)
            },
            CheckType::AuditLogging => {
                config.get("audit_logging").and_then(|v| v.as_bool()).unwrap_or(false)
            },
            CheckType::DataRetention => {
                config.get("data_localization").and_then(|v| v.as_bool()).unwrap_or(true)
            },
            CheckType::BreachNotification => {
                config.get("breach_notification").and_then(|v| v.as_bool()).unwrap_or(false)
            },
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vietnam_134_checks() {
        let checker = ComplianceChecker::new(ComplianceFramework::Vietnam134);
        assert_eq!(checker.get_checks().len(), 5);
    }

    #[test]
    fn test_compliance_evaluation() {
        let checker = ComplianceChecker::new(ComplianceFramework::Vietnam134);
        
        let mut config = HashMap::new();
        config.insert("consent_required".to_string(), serde_json::json!(true));
        config.insert("encryption_enabled".to_string(), serde_json::json!(true));
        config.insert("data_localization".to_string(), serde_json::json!(false));
        
        let report = checker.evaluate(&config);
        
        assert!(report.total_checks > 0);
    }
}
