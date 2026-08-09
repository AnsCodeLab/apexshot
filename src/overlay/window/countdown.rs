//! Capture-delay countdown tick (1s) used when Timer tool is armed.
//!
//! Owns timer arming, cancel handling, redraw, and final result delivery.
//! Click-path bubble cancel remains in the click handler (sets
//! `countdown_cancel_requested`); this module reacts to that flag.

use super::super::api::SelectionResult;
use super::super::background::BackgroundFrame;
use super::super::state::SelectorState;
use super::result::send_selection_result;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{ApplicationWindow, DrawingArea};
use std::sync::{Arc, Mutex};

/// If timer delay is armed and a countdown is not already running, start it
/// and install the 1-second UI tick. Returns `true` when countdown started.
///
/// When `false`, the caller should decide whether to confirm immediately
/// (no active countdown) or ignore (countdown already in progress).
pub(in crate::overlay::window) fn try_start_capture_countdown(
    state: &Arc<Mutex<SelectorState>>,
    result_tx: std::sync::mpsc::Sender<SelectionResult>,
    window: &ApplicationWindow,
    drawing_area: &DrawingArea,
    background: Option<&BackgroundFrame>,
    screen_width: i32,
    screen_height: i32,
) -> bool {
    {
        let mut st = state.lock().unwrap();
        if !(st.timer_delay_active && st.capture_delay_seconds > 0 && !st.countdown_active) {
            return false;
        }
        st.countdown_active = true;
        st.countdown_cancel_requested = false;
        st.countdown_value = st.capture_delay_seconds;
        st.hovered_countdown_cancel = false;
    }

    drawing_area.queue_draw();

    let state_countdown = state.clone();
    let result_tx_countdown = result_tx;
    let window_weak_countdown = window.downgrade();
    let drawing_area_weak_countdown = drawing_area.downgrade();
    let background_countdown = background.cloned();
    glib::timeout_add_seconds_local(1, move || {
        let mut st = state_countdown.lock().unwrap();
        if st.countdown_cancel_requested || st.cancelled {
            st.countdown_active = false;
            st.countdown_cancel_requested = false;
            drop(st);
            if let Some(da) = drawing_area_weak_countdown.upgrade() {
                da.queue_draw();
            }
            return glib::ControlFlow::Break;
        }

        st.countdown_value -= 1;
        if st.countdown_value <= 0 {
            st.countdown_active = false;
            drop(st);
            if let Some(window) = window_weak_countdown.upgrade() {
                send_selection_result(
                    &state_countdown,
                    &result_tx_countdown,
                    &window,
                    screen_width,
                    screen_height,
                    background_countdown.as_ref(),
                );
            }
            glib::ControlFlow::Break
        } else {
            drop(st);
            if let Some(da) = drawing_area_weak_countdown.upgrade() {
                da.queue_draw();
            }
            glib::ControlFlow::Continue
        }
    });

    true
}

#[cfg(test)]
mod tests {
    #[test]
    fn countdown_owner_covers_timer_cancel_redraw_and_delivery() {
        let source = include_str!("countdown.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production countdown source");
        assert!(
            production.contains("fn try_start_capture_countdown"),
            "countdown must own try_start_capture_countdown"
        );
        assert!(
            production.contains("timeout_add_seconds_local(1"),
            "countdown must keep the 1s tick"
        );
        assert!(
            production.contains("countdown_cancel_requested")
                && production.contains("countdown_value"),
            "countdown must honor cancel and tick value"
        );
        assert!(
            production.contains("send_selection_result"),
            "countdown finish must deliver selection result"
        );
        assert!(
            production.contains("queue_draw"),
            "countdown must schedule redraws"
        );
    }
}
