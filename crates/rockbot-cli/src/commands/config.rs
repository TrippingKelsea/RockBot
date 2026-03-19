//! Configuration management commands

use crate::{load_config, ConfigCommands, ConfigInitCommands};
use anyhow::Result;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
struct GatewayBootstrapConfig {
    gateway: GatewayBootstrapSection,
    pki: PkiBootstrapSection,
    client: ClientBootstrapSection,
    security: SecurityBootstrapSection,
}

#[derive(Serialize)]
struct GatewayBootstrapSection {
    bind_host: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    listen_ips: Vec<String>,
    port: u16,
    client_port: u16,
    public: GatewayPublicBootstrapSection,
}

#[derive(Serialize)]
struct GatewayPublicBootstrapSection {
    serve_webapp: bool,
    serve_ca: bool,
    enrollment_enabled: bool,
}

#[derive(Serialize)]
struct PkiBootstrapSection {
    tls_cert: Option<String>,
    tls_key: Option<String>,
    pki_dir: String,
}

#[derive(Serialize)]
struct ClientBootstrapSection {
    gateway_host: String,
    https_port: u16,
    client_port: u16,
}

#[derive(Serialize)]
struct SecurityBootstrapSection {
    storage: StorageBootstrapSection,
    roles: RolesBootstrapSection,
}

#[derive(Serialize)]
struct StorageBootstrapSection {
    enabled: bool,
    mode: String,
    key_source: String,
}

#[derive(Serialize)]
struct RolesBootstrapSection {
    gateway: bool,
    vault_provider: bool,
}

/// Run configuration commands
pub async fn run(command: &ConfigCommands, config_path: &PathBuf) -> Result<()> {
    match command {
        ConfigCommands::Show => show_config(config_path).await,
        ConfigCommands::Validate => validate_config(config_path).await,
        ConfigCommands::Init { command } => match command {
            ConfigInitCommands::Gateway {
                output,
                force,
                https_port,
                client_port,
                bind_host,
                listen_ips,
            } => {
                init_gateway_config(
                    output.as_ref().unwrap_or(config_path),
                    *force,
                    bind_host,
                    listen_ips,
                    *https_port,
                    *client_port,
                )
                .await
            }
            ConfigInitCommands::Client {
                output,
                force,
                gateway_ip,
                https_port,
                client_port,
            } => {
                init_client_config(
                    output.as_ref().unwrap_or(config_path),
                    *force,
                    gateway_ip,
                    *https_port,
                    *client_port,
                )
                .await
            }
        },
    }
}

/// Show current configuration
async fn show_config(config_path: &PathBuf) -> Result<()> {
    let config = load_config(config_path).await?;

    let toml_string = toml::to_string_pretty(&config)?;
    println!("{toml_string}");

    Ok(())
}

/// Validate configuration
async fn validate_config(config_path: &PathBuf) -> Result<()> {
    match load_config(config_path).await {
        Ok(config) => {
            println!("✅ Configuration is valid");
            println!(
                "   Gateway HTTPS: {}:{}",
                config.gateway.bind_host, config.gateway.port
            );
            println!(
                "   Gateway Client: {}:{}",
                config.client.gateway_host, config.client.client_port
            );
            println!("   Agents: {} configured", config.agents.list.len());
            println!("   Tools: {} profile", config.tools.profile);
            println!("   Security: {} sandbox", config.security.sandbox.mode);
        }
        Err(e) => {
            println!("❌ Configuration is invalid: {e}");
            std::process::exit(1);
        }
    }

    Ok(())
}

async fn init_gateway_config(
    output_path: &Path,
    force: bool,
    bind_host: &str,
    listen_ips: &[String],
    https_port: u16,
    client_port: u16,
) -> Result<()> {
    ensure_output_path(output_path, force).await?;

    let config_dir = output_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let pki_dir = config_dir.join("pki");
    let cert_path = pki_dir.join("certs").join("gateway.crt");
    let key_path = pki_dir.join("keys").join("gateway.key");

    if !cert_path.exists() || !key_path.exists() || force {
        super::cert::generate_self_signed_cert(&cert_path, &key_path, listen_ips, 365).await?;
        println!("   TLS cert: {}", cert_path.display());
        println!("   TLS key:  {}", key_path.display());
    }

    let resolved_bind_host = listen_ips
        .first()
        .cloned()
        .unwrap_or_else(|| bind_host.to_string());
    let toml = toml::to_string_pretty(&GatewayBootstrapConfig {
        gateway: GatewayBootstrapSection {
            bind_host: resolved_bind_host.clone(),
            listen_ips: listen_ips.to_vec(),
            port: https_port,
            client_port,
            public: GatewayPublicBootstrapSection {
                serve_webapp: true,
                serve_ca: true,
                enrollment_enabled: true,
            },
        },
        pki: PkiBootstrapSection {
            tls_cert: Some(cert_path.display().to_string()),
            tls_key: Some(key_path.display().to_string()),
            pki_dir: pki_dir.display().to_string(),
        },
        client: ClientBootstrapSection {
            gateway_host: "127.0.0.1".to_string(),
            https_port,
            client_port,
        },
        security: SecurityBootstrapSection {
            storage: StorageBootstrapSection {
                enabled: true,
                mode: "encrypted_by_default".to_string(),
                key_source: "pki_local".to_string(),
            },
            roles: RolesBootstrapSection {
                gateway: true,
                vault_provider: false,
            },
        },
    })?;

    tokio::fs::write(output_path, toml).await?;

    println!(
        "Gateway bootstrap config created at {}",
        output_path.display()
    );
    if listen_ips.is_empty() {
        println!("   HTTPS/Web UI listener: {resolved_bind_host}:{https_port}");
        println!("   Client/mTLS listener:  {resolved_bind_host}:{client_port}");
    } else {
        println!(
            "   HTTPS/Web UI listeners: {}",
            listen_ips
                .iter()
                .map(|ip| format!("{ip}:{https_port}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!(
            "   Client/mTLS listeners:  {}",
            listen_ips
                .iter()
                .map(|ip| format!("{ip}:{client_port}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    Ok(())
}

async fn init_client_config(
    output_path: &Path,
    force: bool,
    gateway_ip: &str,
    https_port: u16,
    client_port: u16,
) -> Result<()> {
    ensure_output_path(output_path, force).await?;

    let toml = toml::to_string_pretty(&GatewayBootstrapConfig {
        gateway: GatewayBootstrapSection {
            bind_host: "127.0.0.1".to_string(),
            listen_ips: Vec::new(),
            port: https_port,
            client_port,
            public: GatewayPublicBootstrapSection {
                serve_webapp: false,
                serve_ca: false,
                enrollment_enabled: false,
            },
        },
        pki: PkiBootstrapSection {
            tls_cert: None,
            tls_key: None,
            pki_dir: output_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join("pki")
                .display()
                .to_string(),
        },
        client: ClientBootstrapSection {
            gateway_host: gateway_ip.to_string(),
            https_port,
            client_port,
        },
        security: SecurityBootstrapSection {
            storage: StorageBootstrapSection {
                enabled: true,
                mode: "encrypted_by_default".to_string(),
                key_source: "pki_local".to_string(),
            },
            roles: RolesBootstrapSection {
                gateway: false,
                vault_provider: false,
            },
        },
    })?;

    tokio::fs::write(output_path, toml).await?;

    println!(
        "Client bootstrap config created at {}",
        output_path.display()
    );
    println!("   Gateway HTTPS/Web UI: {gateway_ip}:{https_port}");
    println!("   Gateway client port:  {gateway_ip}:{client_port}");

    Ok(())
}

async fn ensure_output_path(output_path: &Path, force: bool) -> Result<()> {
    if output_path.exists() && !force {
        anyhow::bail!(
            "Configuration file already exists: {}\nUse --force to overwrite",
            output_path.display()
        );
    }

    let config_dir = output_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    tokio::fs::create_dir_all(config_dir).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[tokio::test]
    async fn test_init_gateway_config_creates_missing_pki_directories() {
        let temp = tempfile::tempdir().unwrap();
        let output_path = temp.path().join("missing").join("rockbot.toml");

        init_gateway_config(&output_path, false, "127.0.0.1", &[], 8443, 8444)
            .await
            .unwrap();

        assert!(output_path.exists());
        assert!(output_path
            .parent()
            .unwrap()
            .join("pki")
            .join("certs")
            .join("gateway.crt")
            .exists());
        assert!(output_path
            .parent()
            .unwrap()
            .join("pki")
            .join("keys")
            .join("gateway.key")
            .exists());
    }

    #[tokio::test]
    async fn test_init_gateway_config_escapes_bind_host_in_toml_output() {
        let temp = tempfile::tempdir().unwrap();
        let output_path = temp.path().join("escaped").join("rockbot.toml");
        let bind_host = "127.0.0.1\"\nmalicious = true";

        init_gateway_config(&output_path, false, bind_host, &[], 8443, 8444)
            .await
            .unwrap();

        let written = tokio::fs::read_to_string(&output_path).await.unwrap();
        let parsed: toml::Value = toml::from_str(&written).unwrap();

        assert_eq!(parsed["gateway"]["bind_host"].as_str(), Some(bind_host));
        assert_eq!(parsed["gateway"]["port"].as_integer(), Some(8443));
        assert!(parsed.get("malicious").is_none());
    }
}
