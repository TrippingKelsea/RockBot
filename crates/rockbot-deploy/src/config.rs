//! Deploy configuration types.

use serde::{Deserialize, Serialize};

/// Configuration for S3 CA distribution and Route53 DNS provisioning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployConfig {
    /// S3 bucket name (required)
    pub bucket: String,
    /// AWS region (default: "us-east-1")
    #[serde(default = "default_region")]
    pub region: String,
    /// S3 object key for the CA certificate (default: "pki/ca.crt")
    #[serde(default = "default_ca_cert_key")]
    pub ca_cert_key: String,
    /// Whether to apply a public-read bucket policy (default: false)
    #[serde(default)]
    pub public: bool,
    /// Runtime S3 endpoint override (e.g. for LocalStack)
    #[serde(default)]
    pub endpoint_url: Option<String>,
    /// Auto-create bucket if it doesn't exist (default: true)
    #[serde(default = "default_true")]
    pub auto_create_bucket: bool,
    /// Upload CA cert on gateway startup (default: true)
    #[serde(default = "default_true")]
    pub upload_on_startup: bool,
    /// Route53 hosted zone domain (default: "rockbot.internal")
    #[serde(default = "default_dns_zone")]
    pub dns_zone: String,
    /// Human-friendly cluster name for DNS records
    #[serde(default)]
    pub cluster_name: Option<String>,
}

fn default_region() -> String {
    "us-east-1".to_string()
}

fn default_ca_cert_key() -> String {
    "pki/ca.crt".to_string()
}

fn default_true() -> bool {
    true
}

fn default_dns_zone() -> String {
    "rockbot.internal".to_string()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn test_deploy_config_defaults() {
        let json = r#"{"bucket": "my-rockbot-ca"}"#;
        let config: DeployConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.bucket, "my-rockbot-ca");
        assert_eq!(config.region, "us-east-1");
        assert_eq!(config.ca_cert_key, "pki/ca.crt");
        assert!(!config.public);
        assert!(config.endpoint_url.is_none());
        assert!(config.auto_create_bucket);
        assert!(config.upload_on_startup);
        assert_eq!(config.dns_zone, "rockbot.internal");
        assert!(config.cluster_name.is_none());
    }

    #[test]
    fn test_deploy_config_full() {
        let json = r#"{
            "bucket": "test-bucket",
            "region": "eu-west-1",
            "ca_cert_key": "custom/ca.pem",
            "public": true,
            "endpoint_url": "http://localhost:4566",
            "auto_create_bucket": false,
            "upload_on_startup": false,
            "dns_zone": "myorg.internal",
            "cluster_name": "prod-east"
        }"#;
        let config: DeployConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.bucket, "test-bucket");
        assert_eq!(config.region, "eu-west-1");
        assert_eq!(config.ca_cert_key, "custom/ca.pem");
        assert!(config.public);
        assert_eq!(
            config.endpoint_url.as_deref(),
            Some("http://localhost:4566")
        );
        assert!(!config.auto_create_bucket);
        assert!(!config.upload_on_startup);
        assert_eq!(config.dns_zone, "myorg.internal");
        assert_eq!(config.cluster_name.as_deref(), Some("prod-east"));
    }

    #[test]
    fn test_ca_cert_url_format() {
        let config: DeployConfig = serde_json::from_str(r#"{"bucket": "my-bucket"}"#).unwrap();
        let expected = "https://my-bucket.s3.us-east-1.amazonaws.com/pki/ca.crt";
        let url = format!(
            "https://{}.s3.{}.amazonaws.com/{}",
            config.bucket, config.region, config.ca_cert_key
        );
        assert_eq!(url, expected);
    }
}
