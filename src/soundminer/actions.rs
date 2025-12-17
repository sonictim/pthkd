//! Soundminer actions (namespace: "sm")

use crate::actions_sync;

// Define all Soundminer actions using the sync macro
// Actions are automatically registered with the "sm" namespace
actions_sync!("sm", {
    spot_to_protools,
    spot_to_daw,
});
