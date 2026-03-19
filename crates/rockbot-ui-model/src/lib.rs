//! Shared UI-facing models that can be rendered by both terminal and web
//! frontends without coupling those surfaces to each other's widget systems.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatusTone {
    Idle,
    Ok,
    Warn,
    Danger,
}

impl StatusTone {
    pub fn css_class(self) -> &'static str {
        match self {
            Self::Idle => "pill-idle",
            Self::Ok => "pill-ok",
            Self::Warn => "pill-warn",
            Self::Danger => "pill-danger",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PillModel {
    pub label: String,
    pub tone: StatusTone,
}

impl PillModel {
    pub fn idle(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            tone: StatusTone::Idle,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroModel {
    pub eyebrow: String,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactModel {
    pub label: String,
    pub value: String,
    pub href: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelModel {
    pub title: String,
    pub pill: PillModel,
    pub description: Option<String>,
    pub facts: Vec<FactModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapStep {
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapShellModel {
    pub hero: HeroModel,
    pub gateway_panel: PanelModel,
    pub steps: Vec<BootstrapStep>,
    pub nav_items: Vec<String>,
}

impl Default for BootstrapShellModel {
    fn default() -> Self {
        Self {
            hero: HeroModel {
                eyebrow: "RockBot Web".to_string(),
                title: "Import your client identity, then move onto the authenticated control plane."
                    .to_string(),
                body: "The public HTTPS listener is only a bootstrap surface. The real application surface belongs on the authenticated WebSocket with imported client identity material."
                    .to_string(),
            },
            gateway_panel: PanelModel {
                title: "Gateway".to_string(),
                pill: PillModel::idle("Checking"),
                description: Some(
                    "Bootstrap-only health and trust information from the public listener."
                        .to_string(),
                ),
                facts: vec![
                    FactModel {
                        label: "Health".to_string(),
                        value: "Loading...".to_string(),
                        href: None,
                    },
                    FactModel {
                        label: "CA Bundle".to_string(),
                        value: "Download public CA".to_string(),
                        href: Some("/api/cert/ca".to_string()),
                    },
                ],
            },
            steps: vec![
                BootstrapStep {
                    title: "1. Verify the gateway".to_string(),
                    body: "Use the public health endpoint and CA bundle to confirm you are connecting to the intended cluster."
                        .to_string(),
                },
                BootstrapStep {
                    title: "2. Import client identity".to_string(),
                    body: "Drop or select PEM certificate and private key files, then persist them locally in the browser."
                        .to_string(),
                },
                BootstrapStep {
                    title: "3. Authenticate over WS".to_string(),
                    body: "The browser authenticates over the bootstrap WebSocket and then moves onto the authenticated app control plane."
                        .to_string(),
                },
            ],
            nav_items: vec![
                "Agents".to_string(),
                "Markdown State".to_string(),
                "Replication".to_string(),
                "Topology".to_string(),
                "Zones".to_string(),
            ],
        }
    }
}
