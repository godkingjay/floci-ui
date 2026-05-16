fn main() {
    console_error_panic_hook::set_once();
    floci_ui::theme::initialize_theme();
    leptos::mount_to_body(floci_ui::app::App);
}
