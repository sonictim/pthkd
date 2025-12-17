use anyhow::Result;
use std::sync::OnceLock;

// Generated protobuf module
#[allow(dead_code)]
pub mod ptsl {
    tonic::include_proto!("ptsl");
}

// Module declarations
pub mod client;
pub mod edit;
pub mod markers;
pub mod plugins;
pub mod session;
pub mod timecode;
pub mod tracks;

// Re-exports
pub use client::ProtoolsSession;

// Tokio runtime for async operations
static TOKIO_RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

/// Initialize the tokio runtime (call once at startup)
pub fn init_runtime() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    TOKIO_RT.set(rt).unwrap();
}

/// Run an async ProTools command from a sync context
pub fn run_command<F, Fut>(f: F)
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    std::thread::spawn(move || {
        TOKIO_RT.get().unwrap().block_on(f());
    });
}

pub(crate) async fn keystroke(keys: &[&str]) -> Result<()> {
    crate::macos::keystroke::send_keystroke(keys)?;
    std::thread::sleep(std::time::Duration::from_millis(50)); // Wait 50ms
    Ok(())
}
pub(crate) async fn call_menu(menu: &[&str]) -> Result<()> {
    crate::macos::menu::menu_item_run("Pro Tools", menu)?;
    std::thread::sleep(std::time::Duration::from_millis(10)); // Wait 50ms
    Ok(())
}
pub(crate) async fn click_button(window: &str, button: &str) -> Result<()> {
    crate::macos::ui_elements::click_button("Pro Tools", window, button)?;
    std::thread::sleep(std::time::Duration::from_millis(20)); // Wait 50ms
    Ok(())
}

pub(crate) async fn click_checkbox(window: &str, checkbox: &str) -> Result<()> {
    crate::macos::ui_elements::click_checkbox("Pro Tools", window, checkbox)?;
    std::thread::sleep(std::time::Duration::from_millis(20)); // Wait 20ms
    Ok(())
}

/// Combine all module registries into one
pub fn get_action_registry()
-> std::collections::HashMap<&'static str, fn(&crate::params::Params) -> anyhow::Result<()>> {
    let mut registry = std::collections::HashMap::new();
    registry.extend(tracks::get_tracks_registry());
    registry.extend(markers::get_markers_registry());
    registry.extend(edit::get_edit_registry());
    registry.extend(plugins::get_plugins_registry());
    registry
}
