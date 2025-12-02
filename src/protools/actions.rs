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
    toggle_select_grab_tool => toggle_edit_tool,
    go_to_next_marker => go_to_next_marker,
    go_to_previous_marker => go_to_previous_marker,
    go_to_next_marker_lane_1 => go_to_next_marker_1,
    go_to_previous_marker_lane_1 => go_to_previous_marker_1,
    go_to_next_marker_lane_2 => go_to_next_marker_2,
    go_to_previous_marker_lane_2=> go_to_previous_marker_2,
    go_to_next_marker_lane_3 => go_to_next_marker_3,
    go_to_previous_marker_lane_3 => go_to_previous_marker_3,
    go_to_next_marker_lane_4 => go_to_next_marker_4,
    go_to_previous_marker_lane_4 => go_to_previous_marker_4,
    go_to_next_marker_lane_5 => go_to_next_marker_5,
    go_to_previous_marker_lane_5 => go_to_previous_marker_5,
    spot_to_protools_from_soundminer => spot_to_protools_from_soundminer,
    // Add more as needed when ready...
}
