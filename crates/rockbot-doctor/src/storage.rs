//! Deterministic storage diagnostics derived from the storage runtime plan.

use rockbot_storage_runtime::{ResolutionSource, StoragePlanReport, StorageRuntime};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageReport {
    pub storage_root: PathBuf,
    pub disk_path: PathBuf,
    pub disk_exists: bool,
    pub legacy_files: Vec<LegacyStoreFile>,
    pub volumes: Vec<VolumeState>,
    pub findings: Vec<StorageFinding>,
    pub plans: Vec<StoreResolution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyStoreFile {
    pub label: String,
    pub path: PathBuf,
    pub exists: bool,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeState {
    pub name: String,
    pub exists: bool,
    pub len_bytes: Option<u64>,
    pub capacity_bytes: Option<u64>,
    pub header_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreResolution {
    pub label: String,
    pub source: String,
    pub descriptor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageFinding {
    pub severity: FindingSeverity,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FindingSeverity {
    Info,
    Warning,
    Critical,
}

pub fn inspect_storage(config_path: &Path) -> StorageReport {
    let cfg = std::fs::read_to_string(config_path)
        .ok()
        .and_then(|content| rockbot_config::Config::from_toml(&content).ok())
        .unwrap_or_default();
    let runtime = StorageRuntime::new_with_root_sync(
        &cfg,
        config_path
            .parent()
            .map(Path::to_path_buf)
            .or_else(|| dirs::config_dir().map(|p| p.join("rockbot")))
            .unwrap_or_else(|| PathBuf::from(".")),
    )
    .expect("storage runtime should initialize for doctor inspection");
    let plan = runtime
        .plan()
        .expect("storage runtime plan should be inspectable");
    report_from_plan(plan)
}

fn report_from_plan(plan: StoragePlanReport) -> StorageReport {
    let mut legacy_files = Vec::new();
    let mut volumes = Vec::new();
    let mut findings = Vec::new();
    let mut plans = Vec::new();

    for store in plan.stores {
        legacy_files.push(LegacyStoreFile {
            label: store.label.to_string(),
            path: store.legacy.path.clone(),
            exists: store.legacy.exists,
            size_bytes: store.legacy.size_bytes,
        });
        volumes.push(VolumeState {
            name: store.volume.name.clone(),
            exists: store.volume.exists,
            len_bytes: store.volume.len_bytes,
            capacity_bytes: store.volume.capacity_bytes,
            header_kind: store.volume.header_kind.clone(),
        });
        plans.push(StoreResolution {
            label: store.label.to_string(),
            source: match store.resolution {
                ResolutionSource::Legacy => "legacy",
                ResolutionSource::VirtualDisk => "virtual_disk",
                ResolutionSource::Recovery => "recovery",
                ResolutionSource::Missing => "missing",
            }
            .to_string(),
            descriptor: store.descriptor.clone(),
        });

        if store.legacy.exists && store.volume.exists {
            findings.push(StorageFinding {
                severity: FindingSeverity::Info,
                code: format!("legacy_{}_coexists", store.label),
                message: format!(
                    "Legacy store {} still exists alongside the '{}' virtual-disk volume.",
                    store.legacy.path.display(),
                    store.label
                ),
            });
        }

        if store.volume.exists && store.volume.header_kind.as_deref() == Some("plaintext_redb") {
            findings.push(StorageFinding {
                severity: FindingSeverity::Warning,
                code: format!("{}_plaintext_volume", store.label),
                message: format!(
                    "Virtual-disk volume '{}' appears to contain plaintext redb bytes.",
                    store.label
                ),
            });
        }
    }

    StorageReport {
        storage_root: plan.storage_root,
        disk_path: plan.disk_path,
        disk_exists: plan.disk_exists,
        legacy_files,
        volumes,
        findings,
        plans,
    }
}

pub fn summarize_report(report: &StorageReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "Storage root: {}\nVirtual disk: {} ({})\n",
        report.storage_root.display(),
        report.disk_path.display(),
        if report.disk_exists {
            "present"
        } else {
            "missing"
        }
    ));

    out.push_str("Store plan:\n");
    for plan in &report.plans {
        out.push_str(&format!(
            "- {}: {} ({})\n",
            plan.label, plan.source, plan.descriptor
        ));
    }

    out.push_str("Legacy stores:\n");
    for legacy in &report.legacy_files {
        out.push_str(&format!(
            "- {}: {} [{}]\n",
            legacy.label,
            legacy.path.display(),
            if legacy.exists {
                format!("present, {} bytes", legacy.size_bytes.unwrap_or(0))
            } else {
                "missing".to_string()
            }
        ));
    }

    out.push_str("Virtual-disk volumes:\n");
    for volume in &report.volumes {
        out.push_str(&format!(
            "- {}: {}",
            volume.name,
            if volume.exists { "present" } else { "missing" }
        ));
        if let Some(len) = volume.len_bytes {
            out.push_str(&format!(", len={len}"));
        }
        if let Some(capacity) = volume.capacity_bytes {
            out.push_str(&format!(", capacity={capacity}"));
        }
        if let Some(kind) = &volume.header_kind {
            out.push_str(&format!(", header={kind}"));
        }
        out.push('\n');
    }

    if report.findings.is_empty() {
        out.push_str("Findings:\n- none\n");
    } else {
        out.push_str("Findings:\n");
        for finding in &report.findings {
            out.push_str(&format!(
                "- {:?}: {} ({})\n",
                finding.severity, finding.message, finding.code
            ));
        }
    }
    out
}

pub fn recommended_actions(report: &StorageReport) -> Vec<String> {
    let mut actions = Vec::new();

    if report.legacy_files.iter().any(|f| {
        f.exists
            && report
                .plans
                .iter()
                .any(|p| p.label == f.label && p.source != "legacy")
    }) {
        actions.push(
            "Legacy standalone stores still coexist with runtime-selected non-legacy stores. Treat this node as mid-migration and prefer explicit repair over assuming the vdisk is authoritative.".to_string(),
        );
    }

    if report.plans.iter().any(|p| p.source == "recovery") {
        actions.push(
            "One or more stores are on recovery backing. Run `rockbot storage repair` and re-check the plan before treating the node as healthy.".to_string(),
        );
    }

    if report
        .volumes
        .iter()
        .any(|v| v.exists && v.header_kind.as_deref() == Some("opaque_or_encrypted"))
        && report.legacy_files.iter().any(|f| f.exists)
    {
        actions.push(
            "Opaque/encrypted vdisk volumes coexist with legacy files. Keep using the runtime-selected source; do not open stores ad hoc.".to_string(),
        );
    }

    if actions.is_empty() {
        actions.push("No immediate storage migration actions detected.".to_string());
    }

    actions
}
