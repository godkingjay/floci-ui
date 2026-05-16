use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::service_management::models::ServiceSupportLevel;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSnapshot {
    pub endpoint_url: String,
    pub region: String,
    pub access_key_id: String,
    pub credentials_status: String,
    pub last_refreshed_at: String,
    pub services: Vec<ServiceDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSnapshot {
    pub ok: bool,
    pub url: String,
    pub status: Option<u16>,
    pub floci_version: Option<String>,
    pub health_status: String,
    pub body: Option<Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDescriptor {
    pub key: String,
    pub label: String,
    pub category: ServiceCategory,
    pub description: String,
    pub domain_epic: String,
    pub support_level: ServiceSupportLevel,
    pub primary_resource_kinds: Vec<String>,
    pub safe_operations: Vec<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceCategory {
    Core,
    Compute,
    Data,
    Messaging,
    Networking,
    Observability,
    Security,
}
