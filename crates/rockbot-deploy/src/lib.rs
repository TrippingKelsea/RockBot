//! S3 CA distribution and Route53 DNS provisioning for RockBot.
//!
//! This crate provides cloud-based CA certificate distribution via S3
//! and automatic DNS record management via Route53. All AWS functionality
//! is gated behind the `bedrock` feature flag.

pub mod config;

#[cfg(feature = "bedrock")]
pub mod credentials;
#[cfg(feature = "bedrock")]
pub mod dns;
#[cfg(feature = "bedrock")]
pub mod s3;

mod error;

pub use config::DeployConfig;
pub use error::DeployError;

#[cfg(feature = "bedrock")]
pub use credentials::AwsCredentialImporter;
#[cfg(feature = "bedrock")]
pub use dns::DnsProvisioner;
#[cfg(feature = "bedrock")]
pub use s3::CaDistributor;
