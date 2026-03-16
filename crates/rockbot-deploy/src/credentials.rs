//! AWS credential auto-import into the RockBot vault.

use crate::error::DeployError;
use rockbot_credentials::CredentialManager;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Discovered AWS credential set.
#[derive(Debug, Clone)]
pub struct AwsKeySet {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
    pub region: Option<String>,
    /// Where the credentials were discovered (e.g. "env", "shared-credentials")
    pub source: String,
}

/// Result of the import-or-prompt flow.
#[derive(Debug)]
pub enum ImportResult {
    /// No AWS keys found in the environment or config.
    NoKeysFound,
    /// Keys were imported into the vault.
    Imported,
    /// Vault already had identical AWS keys.
    AlreadyPresent,
    /// Vault has different AWS keys; caller should prompt user.
    Conflict {
        discovered: AwsKeySet,
        existing_endpoint_name: String,
    },
}

/// Auto-discovers and imports AWS credentials into the RockBot vault.
pub struct AwsCredentialImporter {
    credential_manager: Arc<CredentialManager>,
}

impl AwsCredentialImporter {
    /// Create a new importer backed by the given credential manager.
    pub fn new(credential_manager: Arc<CredentialManager>) -> Self {
        Self { credential_manager }
    }

    /// Probe the standard AWS credential chain for keys.
    pub fn discover_aws_credentials() -> Option<AwsKeySet> {
        // 1. Environment variables
        if let (Ok(access_key), Ok(secret_key)) = (
            std::env::var("AWS_ACCESS_KEY_ID"),
            std::env::var("AWS_SECRET_ACCESS_KEY"),
        ) {
            if !access_key.is_empty() && !secret_key.is_empty() {
                return Some(AwsKeySet {
                    access_key_id: access_key,
                    secret_access_key: secret_key,
                    session_token: std::env::var("AWS_SESSION_TOKEN").ok(),
                    region: std::env::var("AWS_DEFAULT_REGION")
                        .or_else(|_| std::env::var("AWS_REGION"))
                        .ok(),
                    source: "env".to_string(),
                });
            }
        }

        // 2. Shared credentials file (~/.aws/credentials, default profile)
        let credentials_path: Option<std::path::PathBuf> = dirs::home_dir()
            .map(|h| h.join(".aws").join("credentials"))
            .filter(|p: &std::path::PathBuf| p.exists());

        if let Some(path) = credentials_path {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let mut in_default = false;
                let mut access_key = None;
                let mut secret_key = None;
                let mut token = None;
                let mut region = None;

                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with('[') {
                        in_default = trimmed == "[default]";
                        continue;
                    }
                    if !in_default {
                        continue;
                    }
                    if let Some((k, v)) = trimmed.split_once('=') {
                        let k = k.trim();
                        let v = v.trim();
                        match k {
                            "aws_access_key_id" => access_key = Some(v.to_string()),
                            "aws_secret_access_key" => secret_key = Some(v.to_string()),
                            "aws_session_token" => token = Some(v.to_string()),
                            "region" => region = Some(v.to_string()),
                            _ => {}
                        }
                    }
                }

                if let (Some(ak), Some(sk)) = (access_key, secret_key) {
                    return Some(AwsKeySet {
                        access_key_id: ak,
                        secret_access_key: sk,
                        session_token: token,
                        region,
                        source: "shared-credentials".to_string(),
                    });
                }
            }
        }

        None
    }

    /// Check if the vault already has AWS credentials stored.
    /// Returns the key set and endpoint name if found.
    async fn check_vault_has_aws(&self) -> Option<(AwsKeySet, String)> {
        // Check KV store first
        if let Ok(Some(data)) = self.credential_manager.kv_get("aws", "default").await {
            if let Ok(keys) = serde_json::from_slice::<serde_json::Value>(&data) {
                if let (Some(ak), Some(sk)) = (
                    keys.get("access_key_id").and_then(|v| v.as_str()),
                    keys.get("secret_access_key").and_then(|v| v.as_str()),
                ) {
                    return Some((
                        AwsKeySet {
                            access_key_id: ak.to_string(),
                            secret_access_key: sk.to_string(),
                            session_token: keys
                                .get("session_token")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            region: keys
                                .get("region")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            source: "vault-kv".to_string(),
                        },
                        "aws-default".to_string(),
                    ));
                }
            }
        }

        // Check endpoints for any with "aws" in the name
        let endpoints = self.credential_manager.list_endpoints().await;
        for ep in &endpoints {
            let name_lower = ep.name.to_lowercase();
            if name_lower.contains("aws") {
                debug!("Found AWS-related endpoint: {}", ep.name);
                // We can't easily extract the secret from an endpoint without
                // knowing the credential type, so just report the name
                return Some((
                    AwsKeySet {
                        access_key_id: String::new(),
                        secret_access_key: String::new(),
                        session_token: None,
                        region: None,
                        source: "vault-endpoint".to_string(),
                    },
                    ep.name.clone(),
                ));
            }
        }

        None
    }

    /// Discover AWS credentials and import them into the vault if not already present.
    ///
    /// In non-interactive mode (gateway startup), conflicts are logged as warnings.
    /// In interactive mode (CLI), the caller should handle [`ImportResult::Conflict`]
    /// by prompting the user.
    pub async fn import_or_prompt(&self, _client_uuid: &str) -> Result<ImportResult, DeployError> {
        let Some(discovered) = Self::discover_aws_credentials() else {
            debug!("No AWS credentials found in environment or config");
            return Ok(ImportResult::NoKeysFound);
        };

        info!(
            "Discovered AWS credentials from {} (key: {}...)",
            discovered.source,
            &discovered.access_key_id[..discovered.access_key_id.len().min(8)]
        );

        match self.check_vault_has_aws().await {
            None => {
                // No existing AWS creds — auto-import
                let secret_json = serde_json::json!({
                    "access_key_id": discovered.access_key_id,
                    "secret_access_key": discovered.secret_access_key,
                    "session_token": discovered.session_token,
                    "region": discovered.region,
                });

                self.credential_manager
                    .kv_put(
                        "aws",
                        "default",
                        serde_json::to_vec(&secret_json)
                            .map_err(|e| DeployError::Credential {
                                message: format!("Failed to serialize AWS credentials: {e}"),
                            })?
                            .as_slice(),
                    )
                    .await
                    .map_err(|e| DeployError::Credential {
                        message: format!("Failed to store AWS credentials in vault: {e}"),
                    })?;

                info!("Imported AWS credentials into vault (aws/default)");
                Ok(ImportResult::Imported)
            }
            Some((existing, endpoint_name)) => {
                // Compare discovered vs existing
                if !existing.access_key_id.is_empty()
                    && existing.access_key_id == discovered.access_key_id
                    && existing.secret_access_key == discovered.secret_access_key
                {
                    debug!("Discovered AWS keys match vault ({})", endpoint_name);
                    return Ok(ImportResult::AlreadyPresent);
                }

                // Keys differ or we can't compare (endpoint-based)
                warn!(
                    "Found different AWS keys in environment vs vault ({}). \
                     Use 'rockbot cert ca publish' to resolve interactively.",
                    endpoint_name
                );
                Ok(ImportResult::Conflict {
                    discovered,
                    existing_endpoint_name: endpoint_name,
                })
            }
        }
    }

    /// Store discovered credentials under a client-namespaced key.
    /// Called after user confirms in the conflict/interactive path.
    pub async fn store_namespaced(
        &self,
        client_uuid: &str,
        keys: &AwsKeySet,
    ) -> Result<(), DeployError> {
        let secret_json = serde_json::json!({
            "access_key_id": keys.access_key_id,
            "secret_access_key": keys.secret_access_key,
            "session_token": keys.session_token,
            "region": keys.region,
        });

        let namespace_key = format!("{client_uuid}-default");
        self.credential_manager
            .kv_put(
                "aws",
                &namespace_key,
                serde_json::to_vec(&secret_json)
                    .map_err(|e| DeployError::Credential {
                        message: format!("Failed to serialize: {e}"),
                    })?
                    .as_slice(),
            )
            .await
            .map_err(|e| DeployError::Credential {
                message: format!("Failed to store namespaced AWS credentials: {e}"),
            })?;

        info!("Stored AWS credentials under aws/{namespace_key}");
        Ok(())
    }
}
