use crate::pt_actions;

// Define all ProTools actions in one macro call
// Automatically generates functions AND registry
pt_actions! {
    solo_only_selected_tracks => solo_selected_tracks,
    solo_clear => solo_clear,
    crossfade => crossfade,
    conform_delete => conform_delete,
    conform_insert => conform_insert,
    get_selection => get_selection_samples,
    add_selected_tracks_to_solos => add_selected_to_solos,
    remove_selected_tracks_from_solos => remove_selected_from_solos,
    go_to_next_marker => go_to_next_marker,
    go_to_previous_marker => go_to_previous_marker,
    toggle_select_grab_tool => toggle_edit_tool,
    // Add more as needed when ready...
}
