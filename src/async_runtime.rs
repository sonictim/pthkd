//! Shared async runtime for both ProTools and macOS actions
//!
//! This module provides a single tokio runtime that is shared across all async operations.
//! Previously, ProTools had its own runtime and macOS used block_on(), which caused
//! threading issues. Now everything uses this shared runtime.

use tokio::runtime::Runtime;
use std::sync::OnceLock;

static TOKIO_RT: OnceLock<Runtime> = OnceLock::new();

/// Initialize the Tokio runtime (called from main)
pub fn init() {
    TOKIO_RT.get_or_init(|| {
        log::info!("Initializing shared tokio runtime");
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime")
    });
}

/// Spawn an async action on the runtime
///
/// Used by the actions! macro to run actions asynchronously
pub fn spawn_action<F>(future: F)
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    TOKIO_RT
        .get()
        .expect("Tokio runtime not initialized")
        .spawn(future);
}

/// Get the runtime (for blocking operations)
pub fn runtime() -> &'static Runtime {
    TOKIO_RT.get().expect("Tokio runtime not initialized")
}
