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

/// Macro to generate ProTools actions and registry
#[macro_export]
macro_rules! pt_actions {
    ($($action_name:ident => $command:ident),* $(,)?) => {
        $(
            pub fn $action_name() {
                $crate::protools::run_command(|| async {
                    let mut pt = $crate::protools::ProtoolsSession::new().await.unwrap();
                    $crate::protools::commands::$command(&mut pt).await.ok();
                });
            }
        )*

        pub fn get_action_registry() -> std::collections::HashMap<&'static str, fn()> {
            let mut registry = std::collections::HashMap::new();
            $(
                registry.insert(stringify!($action_name), $action_name as fn());
            )*
            registry
        }
    };
}
