mod dashboard;
mod services;
mod settings;

use icondata::{LuActivity, LuBox, LuCable, LuCpu, LuDatabase, LuShield, LuWifi};

use crate::models::ServiceCategory;

pub use dashboard::DashboardView;
pub use services::{ServiceDetailView, ServicesView};
pub use settings::SettingsView;

fn category_icon(category: ServiceCategory) -> icondata::Icon {
    match category {
        ServiceCategory::Core => LuBox,
        ServiceCategory::Compute => LuCpu,
        ServiceCategory::Data => LuDatabase,
        ServiceCategory::Messaging => LuCable,
        ServiceCategory::Networking => LuWifi,
        ServiceCategory::Observability => LuActivity,
        ServiceCategory::Security => LuShield,
    }
}

fn category_label(category: ServiceCategory) -> &'static str {
    match category {
        ServiceCategory::Core => "Core",
        ServiceCategory::Compute => "Compute",
        ServiceCategory::Data => "Data",
        ServiceCategory::Messaging => "Messaging",
        ServiceCategory::Networking => "Networking",
        ServiceCategory::Observability => "Observability",
        ServiceCategory::Security => "Security",
    }
}
