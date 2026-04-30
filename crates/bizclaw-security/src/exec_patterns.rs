//! RsClaw-style Execution Safety Patterns
//! 
//! Comprehensive security patterns for shell command execution:
//! - 50+ deny patterns (cannot be bypassed)
//! - Confirm patterns (ask user before execution)
//! - Allow patterns (explicit permission)
//! - Path isolation
//! - Content scanning

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use tracing::{debug, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SafetyAction {
    Deny,
    Confirm,
    Allow,
}

impl SafetyAction {
    pub fn is_dangerous(&self) -> bool {
        matches!(self, SafetyAction::Deny)
    }
    
    pub fn needs_confirmation(&self) -> bool {
        matches!(self, SafetyAction::Confirm)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecPolicy {
    pub deny_patterns: Vec<String>,
    pub confirm_patterns: Vec<String>,
    pub allow_patterns: Vec<String>,
    pub path_isolation: bool,
    pub allowed_paths: Vec<String>,
    pub denied_paths: Vec<String>,
    pub max_command_length: usize,
    pub max_execution_time_secs: u64,
}

impl Default for ExecPolicy {
    fn default() -> Self {
        Self {
            deny_patterns: get_default_deny_patterns(),
            confirm_patterns: get_default_confirm_patterns(),
            allow_patterns: get_default_allow_patterns(),
            path_isolation: true,
            allowed_paths: vec![
                "/tmp".to_string(),
                "/var/tmp".to_string(),
                "~".to_string(),
            ],
            denied_paths: vec![
                "/etc".to_string(),
                "/root".to_string(),
                "/.ssh".to_string(),
                "/.aws".to_string(),
                "/.config".to_string(),
            ],
            max_command_length: 10000,
            max_execution_time_secs: 300,
        }
    }
}

impl ExecPolicy {
    pub fn new_strict() -> Self {
        Self {
            path_isolation: true,
            denied_paths: vec![
                "/etc".to_string(),
                "/root".to_string(),
                "/.ssh".to_string(),
                "/.aws".to_string(),
                "/.config".to_string(),
                "/sys".to_string(),
                "/proc".to_string(),
                "/boot".to_string(),
                "/dev".to_string(),
            ],
            ..Default::default()
        }
    }
    
    pub fn new_permissive() -> Self {
        Self {
            deny_patterns: get_default_deny_patterns(),
            confirm_patterns: vec![],
            path_isolation: false,
            ..Default::default()
        }
    }

    pub fn check_command(&self, command: &str) -> SafetyAction {
        let cmd_lower = command.to_lowercase();
        
        for pattern in &self.deny_patterns {
            if cmd_lower.contains(&pattern.to_lowercase()) {
                warn!("Command blocked by deny pattern: {}", command);
                return SafetyAction::Deny;
            }
        }
        
        for pattern in &self.confirm_patterns {
            if cmd_lower.contains(&pattern.to_lowercase()) {
                debug!("Command requires confirmation: {}", command);
                return SafetyAction::Confirm;
            }
        }
        
        for pattern in &self.allow_patterns {
            if cmd_lower.contains(&pattern.to_lowercase()) {
                debug!("Command explicitly allowed: {}", command);
                return SafetyAction::Allow;
            }
        }
        
        SafetyAction::Allow
    }

    pub fn check_path(&self, path: &str) -> SafetyAction {
        let expanded = shellexpand::tilde(path);
        let path_buf = PathBuf::from(expanded.as_ref());
        
        for denied in &self.denied_paths {
            let denied_expanded = shellexpand::tilde(denied);
            let denied_path = PathBuf::from(denied_expanded.as_ref());
            if path_buf.starts_with(&denied_path) {
                warn!("Path access denied: {} is in denied path {}", path, denied);
                return SafetyAction::Deny;
            }
        }
        
        if self.path_isolation {
            let mut in_allowed = false;
            for allowed in &self.allowed_paths {
                let allowed_expanded = shellexpand::tilde(allowed);
                let allowed_path = PathBuf::from(allowed_expanded.as_ref());
                if path_buf.starts_with(&allowed_path) {
                    in_allowed = true;
                    break;
                }
            }
            
            if !in_allowed {
                warn!("Path access denied: {} is not in allowed paths", path);
                return SafetyAction::Deny;
            }
        }
        
        SafetyAction::Allow
    }

    pub fn add_allow_pattern(&mut self, pattern: String) {
        self.allow_patterns.push(pattern);
    }

    pub fn add_confirm_pattern(&mut self, pattern: String) {
        self.confirm_patterns.push(pattern);
    }

    pub fn add_deny_pattern(&mut self, pattern: String) {
        self.deny_patterns.push(pattern);
    }

    pub fn validate_command(&self, command: &str) -> Result<(), String> {
        if command.len() > self.max_command_length {
            return Err(format!(
                "Command too long: {} chars (max {})",
                command.len(),
                self.max_command_length
            ));
        }
        
        match self.check_command(command) {
            SafetyAction::Deny => Err("Command is blocked by security policy".to_string()),
            SafetyAction::Confirm => Ok(()),
            SafetyAction::Allow => Ok(()),
        }
    }
}

fn get_default_deny_patterns() -> Vec<String> {
    vec![
        "sudo".to_string(), "su -".to_string(), "su root".to_string(), "doas".to_string(), "pkexec".to_string(), "chmod +s".to_string(), "chmod u+s".to_string(),
        "rm -rf /".to_string(), "rm -rf /*".to_string(), "rm -rf --".to_string(), "rm -rf .".to_string(), "rm -rf *".to_string(),
        "rm -rf /home".to_string(), "rm -rf /var".to_string(), "rm -rf /usr".to_string(), "rm -rf /etc".to_string(),
        "mkfs".to_string(), "dd if=".to_string(), ":(){ :|:& };:".to_string(),
        "/etc/passwd".to_string(), "/etc/shadow".to_string(), "/etc/sudoers".to_string(), "/etc/fstab".to_string(),
        "/etc/hosts".to_string(), "/etc/resolv.conf".to_string(),
        ".ssh/".to_string(), ".ssh/authorized_keys".to_string(), ".ssh/id_rsa".to_string(), ".ssh/id_ed25519".to_string(), "/root/.ssh".to_string(),
        ".env".to_string(), ".env.local".to_string(), ".env.production".to_string(),
        "export AWS_".to_string(), "export GOOGLE_".to_string(), "export AZURE_".to_string(), "export STRIPE_".to_string(),
        "iptables".to_string(), "ip route".to_string(), "route add".to_string(), "ifconfig down".to_string(),
        "kill -9 1".to_string(), "kill -SIGKILL".to_string(), "killall".to_string(), "pkill -9".to_string(),
        "apt-get remove".to_string(), "apt-get purge".to_string(), "yum remove".to_string(), "dnf remove".to_string(),
        "systemctl stop".to_string(), "systemctl disable".to_string(), "service stop".to_string(), "chkconfig off".to_string(),
        "curl * | sh".to_string(), "curl * | bash".to_string(), "wget * | sh".to_string(), "wget * | bash".to_string(),
        "chmod 777".to_string(), "chmod 000".to_string(), "chown".to_string(), "chgrp".to_string(),
        "reboot".to_string(), "shutdown".to_string(), "init 0".to_string(), "halt".to_string(), "poweroff".to_string(),
        "nmap".to_string(), "nikto".to_string(), "dirb".to_string(), "gobuster".to_string(), "hydra".to_string(),
        "nc -l".to_string(), "netcat -l".to_string(), "nc -e".to_string(), "bash -i".to_string(), "telnet".to_string(),
        "crontab -r".to_string(), "crontab -d".to_string(),
        "modprobe".to_string(), "insmod".to_string(), "rmmod".to_string(),
        "docker run".to_string(), "docker exec".to_string(), "kubectl exec".to_string(), "podman exec".to_string(), "virsh".to_string(),
        "rm backup".to_string(), "rm -rf /backup".to_string(),
        "rm /var/log".to_string(), ">/var/log/".to_string(), "truncate -s 0 /var/log".to_string(),
    ]
}

fn get_default_confirm_patterns() -> Vec<String> {
    vec![
        "rm -r".to_string(), "rm -f".to_string(), "mv ".to_string(), "cp -r".to_string(), "mkdir".to_string(), "touch ".to_string(),
        "curl ".to_string(), "wget ".to_string(), "fetch ".to_string(), "ssh ".to_string(), "scp ".to_string(), "rsync".to_string(),
        "ps aux".to_string(), "top".to_string(), "htop".to_string(), "netstat".to_string(), "ss -".to_string(),
        "mysql ".to_string(), "psql ".to_string(), "mongosh".to_string(), "sqlite3".to_string(), "redis-cli".to_string(),
        "git push".to_string(), "git force-push".to_string(), "git reset --hard".to_string(),
        "apt-get install".to_string(), "yum install".to_string(), "npm install -g".to_string(), "pip install".to_string(), "cargo install".to_string(),
        "vi ".to_string(), "nano ".to_string(), "emacs ".to_string(), "sed -i".to_string(),
    ]
}

fn get_default_allow_patterns() -> Vec<String> {
    vec![
        "echo ".to_string(), "cat ".to_string(), "head ".to_string(), "tail ".to_string(), "grep ".to_string(), "ls ".to_string(), "pwd".to_string(), "whoami".to_string(), "date".to_string(), "echo $".to_string(), "printenv".to_string(),
    ]
}

pub struct SafetyChecker {
    policy: ExecPolicy,
    execution_history: HashSet<String>,
}

impl SafetyChecker {
    pub fn new(policy: ExecPolicy) -> Self {
        Self {
            policy,
            execution_history: HashSet::new(),
        }
    }

    pub fn check(&self, command: &str) -> Result<SafetyAction, String> {
        self.policy.validate_command(command)?;
        Ok(self.policy.check_command(command))
    }

    pub fn check_with_path(&self, command: &str, paths: &[&str]) -> Result<SafetyAction, String> {
        let cmd_action = self.check(command)?;
        
        if cmd_action.is_dangerous() {
            return Ok(cmd_action);
        }
        
        for path in paths {
            let path_action = self.policy.check_path(path);
            if path_action.is_dangerous() {
                return Ok(path_action);
            }
        }
        
        Ok(cmd_action)
    }

    pub fn record_execution(&mut self, command: &str) {
        self.execution_history.insert(command.to_string());
    }

    pub fn was_executed(&self, command: &str) -> bool {
        self.execution_history.contains(command)
    }

    pub fn execution_count(&self, command: &str) -> usize {
        self.execution_history
            .iter()
            .filter(|c| c.starts_with(command))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deny_patterns() {
        let policy = ExecPolicy::default();
        
        assert!(matches!(policy.check_command("sudo rm -rf /"), SafetyAction::Deny));
        assert!(matches!(policy.check_command("rm -rf /"), SafetyAction::Deny));
        assert!(matches!(policy.check_command(".ssh/id_rsa"), SafetyAction::Deny));
    }

    #[test]
    fn test_confirm_patterns() {
        let policy = ExecPolicy::default();
        
        assert!(matches!(policy.check_command("rm -rf /tmp/test"), SafetyAction::Confirm));
        assert!(matches!(policy.check_command("curl http://example.com"), SafetyAction::Confirm));
    }

    #[test]
    fn test_allow_patterns() {
        let policy = ExecPolicy::default();
        
        assert!(matches!(policy.check_command("echo hello"), SafetyAction::Allow));
        assert!(matches!(policy.check_command("cat file.txt"), SafetyAction::Allow));
        assert!(matches!(policy.check_command("ls -la"), SafetyAction::Allow));
    }
}
