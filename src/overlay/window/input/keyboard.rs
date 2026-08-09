//! Window-level keyboard controller for the area-selector overlay.

use super::super::super::api::{OverlaySelection, SelectionResult};
use super::super::super::background::BackgroundFrame;
use super::super::super::geometry::{current_selection_rect, set_selection_rect, SelectionRectF};
use super::super::super::icons::TOOLBAR_AREA_INDEX;
use super::super::super::state::SelectorState;
use super::super::countdown::try_start_capture_countdown;
use super::super::result::send_selection_result;
use gtk4::gdk::Key;
use gtk4::glib::{self, clone};
use gtk4::prelude::*;
use gtk4::{ApplicationWindow, DrawingArea, EventControllerKey};
use std::sync::{Arc, Mutex};

/// Install capture-phase keyboard handling on the overlay window.
///
/// Preserves Escape → `Area(None)` + close, Enter/KP_Enter/ISO_Enter/Space
/// confirm (with optional timer countdown), arrow-key nudge, and propagation
/// results (`Stop` / `Proceed`).
pub(in crate::overlay::window) fn wire_window_keyboard(
    window: &ApplicationWindow,
    state: Arc<Mutex<SelectorState>>,
    result_tx: std::sync::mpsc::Sender<SelectionResult>,
    drawing_area: &DrawingArea,
    background: Option<BackgroundFrame>,
    screen_width: i32,
    screen_height: i32,
) {
    // Setup keyboard controller for ESC key
    let key_controller = EventControllerKey::builder()
        .propagation_phase(gtk4::PropagationPhase::Capture)
        .build();

    let state_key = state.clone();
    let window_weak_esc = window.downgrade();
    let result_tx_esc = result_tx.clone();
    let background_key = background.clone();
    let drawing_area_weak_key = drawing_area.downgrade();
    let drawing_area_for_countdown = drawing_area.clone();
    let window_for_countdown = window.clone();

    key_controller.connect_key_pressed(clone!(
        #[strong]
        state_key,
        move |_, key, _, _| {
            if key == Key::Escape {
                let mut st = state_key.lock().unwrap();
                st.cancelled = true;
                st.fullscreen_mode = false;
                drop(st);

                let _ = result_tx_esc.send(Ok(OverlaySelection::Area(None)));

                if let Some(window) = window_weak_esc.upgrade() {
                    window.close();
                }

                return glib::Propagation::Stop;
            }

            if key == Key::Return
                || key == Key::KP_Enter
                || key == Key::ISO_Enter
                || key == Key::space
            {
                if try_start_capture_countdown(
                    &state_key,
                    result_tx_esc.clone(),
                    &window_for_countdown,
                    &drawing_area_for_countdown,
                    background_key.as_ref(),
                    screen_width,
                    screen_height,
                ) {
                    return glib::Propagation::Stop;
                }

                let st = state_key.lock().unwrap();
                if !st.countdown_active {
                    drop(st);
                    if let Some(window) = window_weak_esc.upgrade() {
                        send_selection_result(
                            &state_key,
                            &result_tx_esc,
                            &window,
                            screen_width,
                            screen_height,
                            background_key.as_ref(),
                        );
                    }
                }

                return glib::Propagation::Stop;
            }

            let delta = match key {
                Key::Left => Some((-1.0, 0.0)),
                Key::Right => Some((1.0, 0.0)),
                Key::Up => Some((0.0, -1.0)),
                Key::Down => Some((0.0, 1.0)),
                _ => None,
            };

            if let Some((dx, dy)) = delta {
                let mut st = state_key.lock().unwrap();
                if st.completed {
                    let rect = current_selection_rect(&st);
                    let next = SelectionRectF {
                        left: (rect.left + dx)
                            .clamp(0.0, (screen_width as f64 - rect.width()).max(0.0)),
                        top: (rect.top + dy)
                            .clamp(0.0, (screen_height as f64 - rect.height()).max(0.0)),
                        right: 0.0,
                        bottom: 0.0,
                    };
                    let moved = SelectionRectF {
                        right: next.left + rect.width(),
                        bottom: next.top + rect.height(),
                        ..next
                    };
                    set_selection_rect(&mut st, moved);
                    st.fullscreen_mode = false;
                    st.active_tool_index = TOOLBAR_AREA_INDEX;
                    drop(st);
                    if let Some(drawing_area) = drawing_area_weak_key.upgrade() {
                        drawing_area.queue_draw();
                    }
                    return glib::Propagation::Stop;
                }
            }

            glib::Propagation::Proceed
        }
    ));

    window.add_controller(key_controller);
}

#[cfg(test)]
mod tests {
    #[test]
    fn keyboard_owner_covers_escape_confirm_and_nudge() {
        let source = include_str!("keyboard.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production keyboard source");
        assert!(
            production.contains("fn wire_window_keyboard"),
            "keyboard must own wire_window_keyboard"
        );
        assert!(
            production.contains("Key::Escape")
                && production.contains("OverlaySelection::Area(None)"),
            "Escape must cancel with Area(None)"
        );
        assert!(
            production.contains("Key::Return")
                && production.contains("Key::KP_Enter")
                && production.contains("Key::ISO_Enter")
                && production.contains("Key::space"),
            "confirm must accept Enter variants and Space"
        );
        assert!(
            production.contains("try_start_capture_countdown"),
            "confirm path must use countdown owner"
        );
        assert!(
            production.contains("Key::Left")
                && production.contains("Key::Right")
                && production.contains("Key::Up")
                && production.contains("Key::Down"),
            "keyboard must own arrow-key nudge"
        );
        assert!(
            production.contains("PropagationPhase::Capture"),
            "keyboard controller must stay capture-phase"
        );
        assert!(
            production.contains("Propagation::Stop") && production.contains("Propagation::Proceed"),
            "keyboard must preserve stop/proceed results"
        );
    }
}
