use std::collections::{BTreeMap, BTreeSet};

use leptos::*;

use super::models::{ResourceSummary, ServiceInventory};

#[derive(Clone, Debug)]
pub struct CachedServiceInventory {
    pub inventory: ServiceInventory,
    pub cached_at: String,
}

#[derive(Clone, Copy)]
pub struct ServiceManagementStore {
    pub inventories: RwSignal<BTreeMap<String, CachedServiceInventory>>,
    pub selected_resources: RwSignal<BTreeMap<String, ResourceSummary>>,
    pub loading_services: RwSignal<BTreeSet<String>>,
    pub last_refresh_at: RwSignal<Option<String>>,
}

impl ServiceManagementStore {
    pub fn create() -> Self {
        Self {
            inventories: create_rw_signal(BTreeMap::new()),
            selected_resources: create_rw_signal(BTreeMap::new()),
            loading_services: create_rw_signal(BTreeSet::new()),
            last_refresh_at: create_rw_signal(None),
        }
    }

    pub fn cache_inventory(&self, service_key: String, inventory: ServiceInventory) {
        let cached_at = inventory.refreshed_at.clone();

        self.inventories.update(|inventories| {
            inventories.insert(
                service_key,
                CachedServiceInventory {
                    inventory,
                    cached_at: cached_at.clone(),
                },
            );
        });
        self.last_refresh_at.set(Some(cached_at));
    }

    pub fn set_loading(&self, service_key: String, loading: bool) {
        self.loading_services.update(|services| {
            if loading {
                services.insert(service_key);
            } else {
                services.remove(&service_key);
            }
        });
    }
}
