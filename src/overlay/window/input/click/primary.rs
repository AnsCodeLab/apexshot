//! Primary click gesture and post-state side effects.

use super::{menu, toolbar, ClickEffect};
use crate::overlay::api::SelectionResult;
use crate::overlay::background::BackgroundFrame;
use crate::overlay::state::SelectorState;
use crate::overlay::window::audio::{set_mic_volume, set_speaker_volume};
use crate::overlay::window::send_selection_result;
use gtk4::prelude::*;
use gtk4::{ApplicationWindow, DrawingArea, GestureClick};
use std::sync::{Arc, Mutex};

pub(super) fn wire_primary_click(
    window: &ApplicationWindow,
    state: Arc<Mutex<SelectorState>>,
    result_tx: std::sync::mpsc::Sender<SelectionResult>,
    drawing_area: &DrawingArea,
    background: Option<BackgroundFrame>,
    screen_width: i32,
    screen_height: i32,
) {
    let click_gesture = GestureClick::builder()
        .button(1)
        .propagation_phase(gtk4::PropagationPhase::Capture)
        .build();

    let state_click = state.clone();
    let drawing_area_weak = drawing_area.downgrade();
    let result_tx_click = result_tx;
    let window_weak = window.downgrade();
    let background_click = background;
    click_gesture.connect_pressed(move |_, n_press, x, y| {
        let effect = {
            let mut st = state_click.lock().unwrap();
            menu::handle_menu_click(&mut st, x, y, screen_width, screen_height).unwrap_or_else(
                || {
                    toolbar::handle_toolbar_click(
                        &mut st,
                        n_press,
                        x,
                        y,
                        screen_width,
                        screen_height,
                    )
                },
            )
        };

        match effect {
            ClickEffect::None => {}
            ClickEffect::Redraw => {
                if let Some(drawing_area) = drawing_area_weak.upgrade() {
                    drawing_area.queue_draw();
                }
            }
            ClickEffect::OpenScrollExtension => {
                let url = crate::onboarding::extensions::CHROME_EXTENSION_URL.to_string();
                std::thread::spawn(move || {
                    let _ = crate::utils::open::open_url(&url);
                });
                if let Some(drawing_area) = drawing_area_weak.upgrade() {
                    drawing_area.queue_draw();
                }
            }
            ClickEffect::SendSelection => {
                if let Some(window) = window_weak.upgrade() {
                    send_selection_result(
                        &state_click,
                        &result_tx_click,
                        &window,
                        screen_width,
                        screen_height,
                        background_click.as_ref(),
                    );
                }
            }
            ClickEffect::SendRecording(selection) => {
                let _ = result_tx_click.send(Ok(selection));
                if let Some(window) = window_weak.upgrade() {
                    window.close();
                }
            }
            ClickEffect::SetMicVolume(fraction) => {
                set_mic_volume(fraction);
                if let Some(drawing_area) = drawing_area_weak.upgrade() {
                    drawing_area.queue_draw();
                }
            }
            ClickEffect::SetSpeakerVolume(fraction) => {
                set_speaker_volume(fraction);
                if let Some(drawing_area) = drawing_area_weak.upgrade() {
                    drawing_area.queue_draw();
                }
            }
        }
    });
    drawing_area.add_controller(click_gesture);
}

#[cfg(test)]
mod tests {
    #[test]
    fn primary_owns_capture_gesture_and_runs_effects_after_state_dispatch() {
        let source = include_str!("primary.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production primary click");
        assert!(
            production.contains("fn wire_primary_click")
                && production.contains(".button(1)")
                && production.contains("PropagationPhase::Capture"),
            "primary must own the capture-phase button-one gesture"
        );
        assert!(
            production.contains("menu::handle_menu_click")
                && production.contains("toolbar::handle_toolbar_click"),
            "primary must dispatch to menu before toolbar behavior"
        );
        let dispatch = production.find("let effect = {").expect("state dispatch");
        let effects = production.find("match effect").expect("effect execution");
        assert!(
            dispatch < effects,
            "effects must run after the state scope ends"
        );
        assert!(
            production.contains("open_url")
                && production.contains("result_tx_click.send")
                && production.contains("send_selection_result")
                && production.contains("queue_draw")
                && production.contains("window.close()"),
            "primary must own external and GTK side effects"
        );
    }
}
