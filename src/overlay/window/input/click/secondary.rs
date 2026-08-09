//! Secondary (right-click) gesture for recording panel volume popups.

use super::super::super::super::geometry::current_selection_rect;
use super::super::super::super::recording::hit_testing::recording_tile_at;
use super::super::super::super::recording::layout::RecordPanelTile;
use super::super::super::super::state::SelectorState;
use gtk4::prelude::*;
use gtk4::{DrawingArea, GestureClick};
use std::sync::{Arc, Mutex};

pub(super) fn wire_secondary_click(
    state: Arc<Mutex<SelectorState>>,
    drawing_area: &DrawingArea,
    screen_width: i32,
    screen_height: i32,
) {
    // Right-click gesture for recording panel tile menus
    let right_click_gesture = GestureClick::builder()
        .button(3)
        .propagation_phase(gtk4::PropagationPhase::Capture)
        .build();
    let state_rc = state.clone();
    let drawing_area_weak_rc = drawing_area.downgrade();
    right_click_gesture.connect_pressed(move |_, _n_press, x, y| {
        let mut st = state_rc.lock().unwrap();
        let rect = current_selection_rect(&st);
        let recording_panel_open = st.recording.panel_open;
        if recording_panel_open {
            if let Some(tile) = recording_tile_at(
                rect.left,
                rect.top,
                rect.width(),
                rect.height(),
                screen_width as f64,
                screen_height as f64,
                x,
                y,
            ) {
                match tile {
                    RecordPanelTile::Mic => {
                        st.recording.mic_volume_popup_open = !st.recording.mic_volume_popup_open;
                        st.recording.speaker_volume_popup_open = false;
                        st.recording.volume_slider_dragging = false;
                        st.recording.settings_menu_open = false;
                        st.recording.crop_menu_open = false;
                        st.recording.hover_record_tile = None;
                        st.hover_tool_index = None;
                    }
                    RecordPanelTile::Speaker => {
                        st.recording.speaker_volume_popup_open =
                            !st.recording.speaker_volume_popup_open;
                        st.recording.mic_volume_popup_open = false;
                        st.recording.volume_slider_dragging = false;
                        st.recording.settings_menu_open = false;
                        st.recording.crop_menu_open = false;
                        st.recording.hover_record_tile = None;
                        st.hover_tool_index = None;
                    }
                    _ => {}
                }
                drop(st);
                if let Some(da) = drawing_area_weak_rc.upgrade() {
                    da.queue_draw();
                }
                return;
            }
        }
        drop(st);
    });
    drawing_area.add_controller(right_click_gesture);
}

#[cfg(test)]
mod tests {
    #[test]
    fn secondary_owns_right_click_volume_toggles() {
        let source = include_str!("secondary.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production secondary");
        assert!(
            production.contains("fn wire_secondary_click") && production.contains(".button(3)"),
            "secondary must own right-click gesture"
        );
        assert!(
            production.contains("mic_volume_popup_open")
                && production.contains("speaker_volume_popup_open"),
            "secondary must toggle mic/speaker volume popups"
        );
        assert!(
            production.contains("PropagationPhase::Capture"),
            "secondary must stay capture-phase"
        );
    }
}
