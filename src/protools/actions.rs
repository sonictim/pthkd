//! ProTools actions (namespace: "pt")

use crate::actions_async;

// Define all ProTools actions using the async macro
// Actions are automatically registered with the "pt" namespace
actions_async!("pt", {
    solo_selected_tracks,
    solo_clear,
    crossfade,
    conform_delete,
    conform_insert,
    get_selection_samples,
    add_selected_to_solos,
    remove_selected_from_solos,
    toggle_edit_tool,
    go_to_marker,
    spot_to_protools_from_soundminer,
    reverse_selection,
    preview_audiosuite,
    open_plugin,
    open_plugin_type,
    send_receive_rx,
});
