//! Vault URI Scheme — `vault://key_name` resolver for encrypted secrets.
//!
//! Allows config fields to reference secrets stored in the encrypted vault
//! instead of containing raw API keys. Enterprise-grade secret management.
//!
//! # Usage in config.toml:
//! ```toml
//! api_key = "vault://openai_key"
//!
//! [channel.telegram]
//! bot_token = "vault://telegram_bot_token"
//! ```
//!
//! # CLI management:
//! ```bash
//! bizclaw vault set openai_key sk-proj-xxx    # Store encrypted
//! bizclaw vault list                           # List keys (no values)
//! bizclaw vault get openai_key                 # Retrieve decrypted
//! bizclaw vault remove openai_key              # Delete
//! ```

use crate::secrets::SecretStore;
use bizclaw_core::error::{BizClawError, Result};
use std::sync::Mutex;
use tracing::{debug, warn};

/// Vault URI prefix.
const VAULT_PREFIX: &str = "vault://";

/// Thread-safe vault resolver backed by encrypted SecretStore.
pub struct Vault {
    store: Mutex<SecretStore>,
}

impl Vault {
    /// Create a new vault with encryption enabled.
    pub fn new() -> Self {
        let mut store = SecretStore::new(true);
        if let Err(e) = store.load() {
            warn!("⚠️ Failed to load vault: {e} — starting with empty vault");
        }
        Self {
            store: Mutex::new(store),
        }
    }

    /// Store a secret in the vault (encrypted at rest).
    pub fn set(&self, key: &str, value: &str) -> Result<()> {
        // Validate key name — alphanumeric + underscores only
        if !key
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(BizClawError::Security(
                "Vault key must be alphanumeric with underscores/hyphens only".into(),
            ));
        }
        if key.is_empty() || key.len() > 128 {
            return Err(BizClawError::Security(
                "Vault key must be 1-128 characters".into(),
            ));
        }

        let mut store = self
            .store
            .lock()
            .map_err(|e| BizClawError::Security(format!("Vault lock failed: {e}")))?;
        store.set(key, value);
        store.save()?;
        debug!("🔐 Vault: stored key '{key}'");
        Ok(())
    }

    /// Retrieve a secret from the vault.
    pub fn get(&self, key: &str) -> Result<Option<String>> {
        let store = self
            .store
            .lock()
            .map_err(|e| BizClawError::Security(format!("Vault lock failed: {e}")))?;
        Ok(store.get(key).map(|s| s.to_string()))
    }

    /// Remove a secret from the vault.
    pub fn remove(&self, key: &str) -> Result<Option<String>> {
        let mut store = self
            .store
            .lock()
            .map_err(|e| BizClawError::Security(format!("Vault lock failed: {e}")))?;
        let old = store.remove(key);
        if old.is_some() {
            store.save()?;
            debug!("🔐 Vault: removed key '{key}'");
        }
        Ok(old)
    }

    /// List all secret keys (values NOT exposed).
    pub fn keys(&self) -> Result<Vec<String>> {
        let store = self
            .store
            .lock()
            .map_err(|e| BizClawError::Security(format!("Vault lock failed: {e}")))?;
        Ok(store.keys().into_iter().map(|s| s.to_string()).collect())
    }

    /// Resolve a value — if it starts with `vault://`, look up the key in the vault.
    /// Otherwise return the value as-is.
    ///
    /// # Examples
    /// - `"vault://openai_key"` → looks up `openai_key` in encrypted store
    /// - `"sk-proj-abc123"` → returned as-is (plain text)
    /// - `""` → returned as-is (empty)
    pub fn resolve(&self, value: &str) -> Result<String> {
        if !value.starts_with(VAULT_PREFIX) {
            return Ok(value.to_string());
        }

        let key = &value[VAULT_PREFIX.len()..];
        if key.is_empty() {
            return Err(BizClawError::Security(
                "vault:// URI requires a key name (e.g., vault://api_key)".into(),
            ));
        }

        match self.get(key)? {
            Some(secret) => {
                debug!("🔐 Vault: resolved vault://{key}");
                Ok(secret)
            }
            None => Err(BizClawError::Security(format!(
                "Vault key '{key}' not found. Use `bizclaw vault set {key} <value>` to store it."
            ))),
        }
    }

    /// Resolve a value, returning the original if vault lookup fails.
    /// Logs a warning instead of erroring — useful for optional fields.
    pub fn resolve_or_passthrough(&self, value: &str) -> String {
        match self.resolve(value) {
            Ok(resolved) => resolved,
            Err(e) => {
                if value.starts_with(VAULT_PREFIX) {
                    warn!("⚠️ Vault resolve failed: {e} — using raw value");
                }
                value.to_string()
            }
        }
    }
}

impl Default for Vault {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a string is a vault reference.
pub fn is_vault_ref(value: &str) -> bool {
    value.starts_with(VAULT_PREFIX)
}

/// Mask a secret value for logging/display.
/// Shows first 4 and last 2 chars, masks the rest.
pub fn mask_secret(value: &str) -> String {
    if value.len() <= 8 {
        return "••••••••".to_string();
    }
    let first: String = value.chars().take(4).collect();
    let last: String = value
        .chars()
        .rev()
        .take(2)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{first}••••{last}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_vault_ref() {
        assert!(is_vault_ref("vault://api_key"));
        assert!(is_vault_ref("vault://telegram_token"));
        assert!(!is_vault_ref("sk-proj-abc123"));
        assert!(!is_vault_ref(""));
        assert!(!is_vault_ref("VAULT://uppercase"));
    }

    #[test]
    fn test_mask_secret() {
        assert_eq!(mask_secret("sk-proj-abc123xyz"), "sk-p••••yz");
        assert_eq!(mask_secret("short"), "••••••••");
        assert_eq!(mask_secret("12345678"), "••••••••");
        assert_eq!(mask_secret("123456789"), "1234••••89");
    }

    #[test]
    fn test_vault_resolve_passthrough() {
        let vault = Vault::new();
        // Non-vault values pass through unchanged
        assert_eq!(vault.resolve("sk-plain-key").unwrap(), "sk-plain-key");
        assert_eq!(vault.resolve("").unwrap(), "");
    }

    #[test]
    fn test_vault_resolve_missing_key_error() {
        let vault = Vault::new();
        let result = vault.resolve("vault://nonexistent_key_xyz");
        assert!(result.is_err());
    }

    #[test]
    fn test_vault_set_get_remove() {
        let vault = Vault::new();
        // Set
        vault.set("test_key_unit", "secret_value_123").unwrap();
        // Get
        let val = vault.get("test_key_unit").unwrap();
        assert_eq!(val, Some("secret_value_123".to_string()));
        // Resolve via URI
        let resolved = vault.resolve("vault://test_key_unit").unwrap();
        assert_eq!(resolved, "secret_value_123");
        // Remove
        vault.remove("test_key_unit").unwrap();
        assert_eq!(vault.get("test_key_unit").unwrap(), None);
    }

    #[test]
    fn test_vault_key_validation() {
        let vault = Vault::new();
        // Valid keys
        assert!(vault.set("api_key", "value").is_ok());
        assert!(vault.set("my-token", "value").is_ok());
        // Invalid keys
        assert!(vault.set("bad key spaces", "value").is_err());
        assert!(vault.set("bad/slash", "value").is_err());
        assert!(vault.set("", "value").is_err());
        // Cleanup
        let _ = vault.remove("api_key");
        let _ = vault.remove("my-token");
    }

    #[test]
    fn test_vault_keys_list() {
        let vault = Vault::new();
        vault.set("list_test_a", "val_a").unwrap();
        vault.set("list_test_b", "val_b").unwrap();
        let keys = vault.keys().unwrap();
        assert!(keys.contains(&"list_test_a".to_string()));
        assert!(keys.contains(&"list_test_b".to_string()));
        // Values are NOT exposed in keys()
        assert!(!keys.contains(&"val_a".to_string()));
        let _ = vault.remove("list_test_a");
        let _ = vault.remove("list_test_b");
    }
}
