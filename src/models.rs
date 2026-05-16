use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

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

impl DashboardSnapshot {
    pub fn endpoint_host(&self) -> String {
        Url::parse(&self.endpoint_url).map_or_else(
            |_| self.endpoint_url.clone(),
            |url| {
                let Some(host) = url.host_str() else {
                    return self.endpoint_url.clone();
                };

                match url.port() {
                    Some(port) if host.contains(':') => format!("[{host}]:{port}"),
                    Some(port) => format!("{host}:{port}"),
                    None => host.to_owned(),
                }
            },
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSnapshot {
    pub ok: bool,
    pub url: String,
    pub status: Option<u16>,
    pub body: Option<Value>,
    pub error: Option<String>,
    pub floci_version: Option<String>,
    pub health_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDescriptor {
    pub key: String,
    pub label: String,
    pub category: ServiceCategory,
    pub description: String,
    #[serde(default)]
    pub domain_epic: String,
    #[serde(default)]
    pub support_level: ServiceSupportLevel,
    #[serde(default)]
    pub primary_resource_kinds: Vec<String>,
    #[serde(default)]
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
