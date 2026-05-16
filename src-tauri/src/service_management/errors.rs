use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceManagementError {
    pub code: String,
    pub message: String,
    pub service_key: Option<String>,
    pub operation: Option<String>,
}

impl ServiceManagementError {
    pub fn unsupported_service(service_key: impl Into<String>) -> Self {
        let service_key = service_key.into();

        Self {
            code: "unsupported_service".to_owned(),
            message: format!("No service-management adapter is registered for `{service_key}`."),
            service_key: Some(service_key),
            operation: None,
        }
    }

    pub fn unsupported_operation(
        service_key: impl Into<String>,
        operation: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: "unsupported_operation".to_owned(),
            message: message.into(),
            service_key: Some(service_key.into()),
            operation: Some(operation.into()),
        }
    }

    pub fn client_error(
        service_key: impl Into<String>,
        operation: impl Into<String>,
        error: impl fmt::Display,
    ) -> Self {
        Self {
            code: "client_error".to_owned(),
            message: error.to_string(),
            service_key: Some(service_key.into()),
            operation: Some(operation.into()),
        }
    }

    pub fn invalid_input(
        service_key: impl Into<String>,
        operation: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: "invalid_input".to_owned(),
            message: message.into(),
            service_key: Some(service_key.into()),
            operation: Some(operation.into()),
        }
    }

    pub fn not_found(
        service_key: impl Into<String>,
        operation: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: "not_found".to_owned(),
            message: message.into(),
            service_key: Some(service_key.into()),
            operation: Some(operation.into()),
        }
    }

    pub fn serialization_error(
        service_key: impl Into<String>,
        operation: impl Into<String>,
        error: impl fmt::Display,
    ) -> Self {
        Self {
            code: "serialization_error".to_owned(),
            message: error.to_string(),
            service_key: Some(service_key.into()),
            operation: Some(operation.into()),
        }
    }
}

impl fmt::Display for ServiceManagementError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ServiceManagementError {}
