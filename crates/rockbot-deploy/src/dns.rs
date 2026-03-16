//! Route53 DNS provisioning for RockBot cluster/client discovery.

use crate::config::DeployConfig;
use crate::error::DeployError;
use aws_sdk_route53::Client as Route53Client;
use tracing::{debug, info, warn};

/// Manages Route53 private hosted zones and DNS records for RockBot.
pub struct DnsProvisioner {
    client: Route53Client,
    config: DeployConfig,
    hosted_zone_id: Option<String>,
}

impl DnsProvisioner {
    /// Create a new DNS provisioner. Does not look up the hosted zone yet;
    /// call [`ensure_hosted_zone`] before registering records.
    pub async fn new(config: DeployConfig) -> Result<Self, DeployError> {
        let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(config.region.clone()))
            .load()
            .await;

        let client = Route53Client::new(&aws_config);

        Ok(Self {
            client,
            config,
            hosted_zone_id: None,
        })
    }

    /// Look up or create the private hosted zone for `dns_zone`.
    pub async fn ensure_hosted_zone(&mut self) -> Result<(), DeployError> {
        let dns_name = format!("{}.", self.config.dns_zone.trim_end_matches('.'));

        // Search for existing hosted zone
        let result = self
            .client
            .list_hosted_zones_by_name()
            .dns_name(&dns_name)
            .max_items(1)
            .send()
            .await
            .map_err(|e| DeployError::Dns {
                message: format!("Failed to list hosted zones: {}", e.into_service_error()),
            })?;

        for zone in result.hosted_zones() {
            if zone.name() == dns_name {
                let raw_id = zone.id();
                let id = raw_id.strip_prefix("/hostedzone/").unwrap_or(raw_id);
                self.hosted_zone_id = Some(id.to_string());
                debug!(
                    "Found existing hosted zone for '{}': {}",
                    self.config.dns_zone, id
                );
                return Ok(());
            }
        }

        // Create private hosted zone
        info!(
            "Creating private hosted zone for '{}'",
            self.config.dns_zone
        );

        // Use a stable caller reference based on the zone name to make this idempotent
        let caller_ref = format!("rockbot-{}", self.config.dns_zone);

        let mut req = self
            .client
            .create_hosted_zone()
            .name(&dns_name)
            .caller_reference(&caller_ref)
            .hosted_zone_config(
                aws_sdk_route53::types::HostedZoneConfig::builder()
                    .private_zone(true)
                    .comment("RockBot cluster DNS")
                    .build(),
            );

        // Try to associate with default VPC if we can determine the region
        req = req.vpc(
            aws_sdk_route53::types::Vpc::builder()
                .vpc_region(aws_sdk_route53::types::VpcRegion::from(
                    self.config.region.as_str(),
                ))
                .build(),
        );

        match req.send().await {
            Ok(output) => {
                if let Some(zone) = output.hosted_zone {
                    let raw_id = zone.id();
                    let id = raw_id.strip_prefix("/hostedzone/").unwrap_or(raw_id);
                    self.hosted_zone_id = Some(id.to_string());
                    info!("Created hosted zone '{}' ({})", self.config.dns_zone, id);
                }
                Ok(())
            }
            Err(e) => {
                let service_err = e.into_service_error();
                let meta = service_err.meta();
                // HostedZoneAlreadyExists means our caller_reference was already used
                if meta.code() == Some("HostedZoneAlreadyExists") {
                    warn!("Hosted zone already exists with our caller reference; list and match");
                    // Re-list to find the zone ID (avoids recursive async call)
                    let retry = self
                        .client
                        .list_hosted_zones_by_name()
                        .dns_name(&dns_name)
                        .max_items(1)
                        .send()
                        .await
                        .map_err(|e2| DeployError::Dns {
                            message: format!(
                                "Re-list after conflict failed: {}",
                                e2.into_service_error()
                            ),
                        })?;
                    for z in retry.hosted_zones() {
                        if z.name() == dns_name {
                            let raw = z.id();
                            let id = raw.strip_prefix("/hostedzone/").unwrap_or(raw);
                            self.hosted_zone_id = Some(id.to_string());
                            return Ok(());
                        }
                    }
                    return Err(DeployError::Dns {
                        message: "Hosted zone exists but could not find its ID".to_string(),
                    });
                }
                Err(DeployError::Dns {
                    message: format!("Failed to create hosted zone: {service_err}"),
                })
            }
        }
    }

    /// Register a CNAME record for a client UUID pointing to the S3 endpoint.
    pub async fn register_client(
        &self,
        client_uuid: &str,
        s3_endpoint: &str,
    ) -> Result<(), DeployError> {
        let fqdn = format!("{}.{}", client_uuid, self.config.dns_zone);
        self.upsert_cname(&fqdn, s3_endpoint).await
    }

    /// Register CNAME records for a cluster: one by UUID, optionally one by friendly name.
    pub async fn register_cluster(
        &self,
        cluster_uuid: &str,
        cluster_name: Option<&str>,
        s3_endpoint: &str,
    ) -> Result<(), DeployError> {
        let fqdn = format!("{}.{}", cluster_uuid, self.config.dns_zone);
        self.upsert_cname(&fqdn, s3_endpoint).await?;

        if let Some(name) = cluster_name {
            let nice_fqdn = format!("{}.{}", name, self.config.dns_zone);
            self.upsert_cname(&nice_fqdn, s3_endpoint).await?;
        }

        Ok(())
    }

    /// UPSERT a CNAME record (idempotent).
    async fn upsert_cname(&self, fqdn: &str, target: &str) -> Result<(), DeployError> {
        let zone_id = self
            .hosted_zone_id
            .as_deref()
            .ok_or_else(|| DeployError::Dns {
                message: "Hosted zone not initialized. Call ensure_hosted_zone() first."
                    .to_string(),
            })?;

        let resource_record = aws_sdk_route53::types::ResourceRecord::builder()
            .value(target)
            .build()
            .map_err(|e| DeployError::Dns {
                message: format!("Failed to build resource record: {e}"),
            })?;

        let record_set = aws_sdk_route53::types::ResourceRecordSet::builder()
            .name(fqdn)
            .r#type(aws_sdk_route53::types::RrType::Cname)
            .ttl(300)
            .resource_records(resource_record)
            .build()
            .map_err(|e| DeployError::Dns {
                message: format!("Failed to build record set: {e}"),
            })?;

        let change = aws_sdk_route53::types::Change::builder()
            .action(aws_sdk_route53::types::ChangeAction::Upsert)
            .resource_record_set(record_set)
            .build()
            .map_err(|e| DeployError::Dns {
                message: format!("Failed to build change: {e}"),
            })?;

        let change_batch = aws_sdk_route53::types::ChangeBatch::builder()
            .comment(format!("RockBot auto-provisioned: {fqdn}"))
            .changes(change)
            .build()
            .map_err(|e| DeployError::Dns {
                message: format!("Failed to build change batch: {e}"),
            })?;

        self.client
            .change_resource_record_sets()
            .hosted_zone_id(zone_id)
            .change_batch(change_batch)
            .send()
            .await
            .map_err(|e| DeployError::Dns {
                message: format!(
                    "Failed to upsert CNAME '{fqdn}': {}",
                    e.into_service_error()
                ),
            })?;

        info!("DNS: {fqdn} -> {target}");
        Ok(())
    }
}
