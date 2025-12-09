//! Soundminer actions (namespace: "sm")

use crate::actions;

// Define all Soundminer actions using the unified macro (sync variant)
// Actions are automatically registered with the "sm" namespace
actions!("sync", {
    spot_to_protools,
});
