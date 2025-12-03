use std::sync::OnceLock;

// Generated protobuf module
#[allow(dead_code)]
pub mod ptsl {
    tonic::include_proto!("ptsl");
}

// Module declarations
pub mod actions;
pub mod client;
pub mod commands;

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
