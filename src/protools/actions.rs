use crate::pt_actions;

// Define all ProTools actions in one macro call
// Automatically generates functions AND registry
pt_actions! {
    solo_selected => solo_selected_tracks,
    solo_clear => solo_clear,
    crossfade => crossfade,
    conform_delete => conform_delete,
    conform_insert => conform_insert,
    get_selection => get_selection_samples,
    // Add more as needed when ready...
}
