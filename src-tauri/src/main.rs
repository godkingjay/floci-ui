#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(err) = floci_ui_tauri_lib::run() {
        eprintln!("failed to run Floci UI: {err}");
        std::process::exit(1);
    }
}
