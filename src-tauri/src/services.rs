use crate::models::ServiceDescriptor;

pub fn services() -> Vec<ServiceDescriptor> {
    crate::service_management::registry::services()
}
