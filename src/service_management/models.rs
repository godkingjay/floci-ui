use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type ServiceKey = String;
pub type ResourceKind = String;
pub type PaginationCursor = String;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceSupportLevel {
    Managed,
    ReadOnly,
    Unsupported,
}

impl Default for ServiceSupportLevel {
    fn default() -> Self {
        Self::Unsupported
    }
}

impl ServiceSupportLevel {
    pub fn label(self) -> &'static str {
        match self {
            Self::Managed => "Managed",
            Self::ReadOnly => "Read-only",
            Self::Unsupported => "Unsupported",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceTab {
    pub key: String,
    pub label: String,
    pub kinds: Vec<ResourceKind>,
    pub empty_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSummary {
    pub id: String,
    pub name: String,
    pub kind: ResourceKind,
    pub status: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    #[serde(default)]
    pub tags: BTreeMap<String, String>,
    #[serde(default)]
    pub attributes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDetail {
    pub summary: ResourceSummary,
    pub metadata: Value,
    #[serde(default)]
    pub relationships: Vec<ResourceRelationship>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRelationship {
    pub label: String,
    pub target: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInventory {
    pub service_key: ServiceKey,
    pub service_label: String,
    pub support_level: ServiceSupportLevel,
    pub refreshed_at: String,
    pub tabs: Vec<ResourceTab>,
    pub resources: Vec<ResourceSummary>,
    pub unsupported_operations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInventoryRequest {
    pub service_key: ServiceKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDetailRequest {
    pub service_key: ServiceKey,
    pub resource_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceAction {
    pub key: String,
    pub label: String,
    pub description: String,
    pub destructive: bool,
    pub requires_selection: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceActionRequest {
    pub service_key: ServiceKey,
    pub action: String,
    pub resource_id: Option<String>,
    pub resource_name: Option<String>,
    pub confirmation: Option<String>,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub changed: bool,
    pub message: String,
    pub resource_id: Option<String>,
}
