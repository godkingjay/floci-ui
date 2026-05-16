mod commands;
mod config;
mod floci;
mod models;
mod service_management;
mod services;

use std::sync::Arc;

use reqwest::Client;
use tauri::Manager;

use crate::{config::AppConfig, floci::FlociClient};

#[derive(Clone)]
pub struct AppState {
    config: Arc<AppConfig>,
    floci: FlociClient,
}

impl AppState {
    fn new(config: AppConfig) -> Result<Self, reqwest::Error> {
        let http = Client::builder()
            .user_agent(concat!("floci-ui/", env!("CARGO_PKG_VERSION")))
            .build()?;
        let config = Arc::new(config);
        let floci = FlociClient::new(config.endpoint_url.clone(), http);

        Ok(Self { config, floci })
    }

    fn config(&self) -> &AppConfig {
        &self.config
    }

    fn floci(&self) -> &FlociClient {
        &self.floci
    }
}

/// Run the Tauri desktop application.
///
/// # Errors
///
/// Returns an error if Tauri setup fails, the local Floci configuration is invalid,
/// or the desktop runtime cannot start.
pub fn run() -> tauri::Result<()> {
    tauri::Builder::default()
        .setup(|app| {
            let _ = dotenvy::dotenv();
            let config = AppConfig::from_env()?;
            let state = AppState::new(config)?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::floci_health,
            commands::service_catalog,
            commands::service_inventory,
            commands::service_resource_detail,
            commands::service_execute_action,
        ])
        .run(tauri::generate_context!())
}
