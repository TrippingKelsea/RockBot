//! S3-based CA certificate distribution.

use crate::config::DeployConfig;
use crate::error::DeployError;
use aws_sdk_s3::Client as S3Client;
use tracing::{debug, info, warn};

/// Distributes the CA certificate to an S3 bucket.
pub struct CaDistributor {
    client: S3Client,
    config: DeployConfig,
}

impl CaDistributor {
    /// Create a new CA distributor.
    ///
    /// Endpoint resolution order: runtime `endpoint_url` > compile-time
    /// `ROCKBOT_S3_ENDPOINT` > SDK default.
    pub async fn new(config: DeployConfig) -> Result<Self, DeployError> {
        let compile_time_endpoint = option_env!("ROCKBOT_S3_ENDPOINT");

        let effective_endpoint = config
            .endpoint_url
            .as_deref()
            .or(compile_time_endpoint)
            .map(String::from);

        let aws_config = {
            let mut loader = aws_config::defaults(aws_config::BehaviorVersion::latest())
                .region(aws_config::Region::new(config.region.clone()));
            if let Some(ref ep) = effective_endpoint {
                loader = loader.endpoint_url(ep);
            }
            loader.load().await
        };

        let mut s3_config_builder = aws_sdk_s3::config::Builder::from(&aws_config);
        if effective_endpoint.is_some() {
            s3_config_builder = s3_config_builder.force_path_style(true);
        }

        let client = S3Client::from_conf(s3_config_builder.build());

        Ok(Self { client, config })
    }

    /// Ensure the S3 bucket exists, creating it if `auto_create_bucket` is enabled.
    pub async fn ensure_bucket(&self) -> Result<(), DeployError> {
        match self
            .client
            .head_bucket()
            .bucket(&self.config.bucket)
            .send()
            .await
        {
            Ok(_) => {
                debug!("Bucket '{}' exists", self.config.bucket);
                return Ok(());
            }
            Err(e) => {
                let service_err = e.into_service_error();
                if !service_err.is_not_found() {
                    return Err(DeployError::S3 {
                        message: format!(
                            "Failed to check bucket '{}': {service_err}",
                            self.config.bucket
                        ),
                    });
                }
            }
        }

        if !self.config.auto_create_bucket {
            return Err(DeployError::S3 {
                message: format!(
                    "Bucket '{}' does not exist and auto_create_bucket is disabled",
                    self.config.bucket
                ),
            });
        }

        info!("Creating bucket '{}'", self.config.bucket);

        let mut req = self.client.create_bucket().bucket(&self.config.bucket);

        // us-east-1 must NOT specify a LocationConstraint (AWS quirk)
        if self.config.region != "us-east-1" {
            req = req.create_bucket_configuration(
                aws_sdk_s3::types::CreateBucketConfiguration::builder()
                    .location_constraint(aws_sdk_s3::types::BucketLocationConstraint::from(
                        self.config.region.as_str(),
                    ))
                    .build(),
            );
        }

        match req.send().await {
            Ok(_) => {
                info!("Created bucket '{}'", self.config.bucket);
                Ok(())
            }
            Err(e) => {
                let service_err = e.into_service_error();
                // BucketAlreadyOwnedByYou is success (race condition or eventual consistency)
                let meta = service_err.meta();
                if meta.code() == Some("BucketAlreadyOwnedByYou") {
                    debug!("Bucket '{}' already owned by us", self.config.bucket);
                    Ok(())
                } else {
                    Err(DeployError::S3 {
                        message: format!(
                            "Failed to create bucket '{}': {service_err}",
                            self.config.bucket
                        ),
                    })
                }
            }
        }
    }

    /// Apply a public-read bucket policy for the CA cert key.
    /// Best-effort: warns on failure (account-level Block Public Access may prevent it).
    pub async fn apply_public_policy(&self) {
        let policy = serde_json::json!({
            "Version": "2012-10-17",
            "Statement": [{
                "Sid": "PublicReadCACert",
                "Effect": "Allow",
                "Principal": "*",
                "Action": "s3:GetObject",
                "Resource": format!("arn:aws:s3:::{}/{}", self.config.bucket, self.config.ca_cert_key)
            }]
        });

        match self
            .client
            .put_bucket_policy()
            .bucket(&self.config.bucket)
            .policy(policy.to_string())
            .send()
            .await
        {
            Ok(_) => info!("Applied public-read policy to '{}'", self.config.bucket),
            Err(e) => warn!(
                "Could not apply public bucket policy (account-level Block Public Access may be enabled): {}",
                e.into_service_error()
            ),
        }
    }

    /// Upload the CA certificate PEM to S3.
    pub async fn upload_ca_cert(&self, ca_pem: &str) -> Result<(), DeployError> {
        self.client
            .put_object()
            .bucket(&self.config.bucket)
            .key(&self.config.ca_cert_key)
            .body(ca_pem.as_bytes().to_vec().into())
            .content_type("application/x-pem-file")
            .send()
            .await
            .map_err(|e| DeployError::S3 {
                message: format!("Failed to upload CA cert: {}", e.into_service_error()),
            })?;

        info!(
            "Uploaded CA cert to s3://{}/{}",
            self.config.bucket, self.config.ca_cert_key
        );
        Ok(())
    }

    /// Orchestrate the full provisioning sequence: ensure bucket, optionally apply
    /// public policy, and upload the CA certificate.
    pub async fn provision(&self, ca_pem: &str) -> Result<(), DeployError> {
        self.ensure_bucket().await?;
        if self.config.public {
            self.apply_public_policy().await;
        }
        self.upload_ca_cert(ca_pem).await?;
        Ok(())
    }

    /// Return the public/endpoint URL for the CA certificate (for logging).
    pub fn ca_cert_url(&self) -> String {
        if let Some(ref ep) = self.config.endpoint_url {
            format!(
                "{}/{}/{}",
                ep.trim_end_matches('/'),
                self.config.bucket,
                self.config.ca_cert_key
            )
        } else {
            format!(
                "https://{}.s3.{}.amazonaws.com/{}",
                self.config.bucket, self.config.region, self.config.ca_cert_key
            )
        }
    }

    /// Return a reference to the deploy config.
    pub fn config(&self) -> &DeployConfig {
        &self.config
    }
}
