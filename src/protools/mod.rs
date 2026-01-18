use crate::prelude::*;
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

/// Combine all module registries into one
pub fn get_action_registry()
-> std::collections::HashMap<&'static str, fn(&crate::config::Params) -> R<()>> {
    let mut registry = std::collections::HashMap::new();
    registry.extend(tracks::get_tracks_registry());
    registry.extend(markers::get_markers_registry());
    registry.extend(edit::get_edit_registry());
    registry.extend(session::get_session_registry());
    registry.extend(plugins::get_plugins_registry());
    registry
}
