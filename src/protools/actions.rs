//! ProTools actions (namespace: "pt")

use crate::actions_async;

// Define all ProTools actions using the async macro
// Actions are automatically registered with the "pt" namespace
actions_async!("pt", {
    solo_selected_tracks,
    solo_clear,
    crossfade,
    adjust_clip_to_match_selection,
    conform_delete,
    conform_insert,
    add_selected_to_solos,
    remove_selected_from_solos,
    toggle_edit_tool,
    go_to_next_marker,
    go_to_quick_marker,
    update_quick_marker,
    audiosuite,
    multitap_plugin_selector,
    send_receive_rx,
    reset_clip,
    click_a_button,
});
