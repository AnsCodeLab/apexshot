//! Primary and secondary overlay click wiring.

mod menu;
mod primary;
mod secondary;
mod toolbar;

use crate::overlay::api::{OverlaySelection, SelectionResult};
use crate::overlay::background::BackgroundFrame;
use crate::overlay::state::SelectorState;
use gtk4::{ApplicationWindow, DrawingArea};
use std::sync::{Arc, Mutex};

enum ClickEffect {
    None,
    Redraw,
    OpenScrollExtension,
    SendSelection,
    SendRecording(OverlaySelection),
    SetMicVolume(f64),
    SetSpeakerVolume(f64),
}

pub(in crate::overlay::window) fn wire_window_click(
    window: &ApplicationWindow,
    state: Arc<Mutex<SelectorState>>,
    result_tx: std::sync::mpsc::Sender<SelectionResult>,
    drawing_area: &DrawingArea,
    background: Option<BackgroundFrame>,
    screen_width: i32,
    screen_height: i32,
) {
    primary::wire_primary_click(
        window,
        state.clone(),
        result_tx,
        drawing_area,
        background,
        screen_width,
        screen_height,
    );
    secondary::wire_secondary_click(state, drawing_area, screen_width, screen_height);
}

#[cfg(test)]
mod tests {
    #[test]
    fn click_facade_installs_primary_before_secondary() {
        let source = include_str!("mod.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production click facade");
        let primary = production
            .find("primary::wire_primary_click(")
            .expect("primary click wiring");
        let secondary = production
            .find("secondary::wire_secondary_click(")
            .expect("secondary click wiring");
        assert!(
            primary < secondary,
            "primary must be attached before secondary"
        );
        assert!(
            production.contains("mod menu;") && production.contains("mod toolbar;"),
            "click facade must declare cohesive menu and toolbar owners"
        );
    }
}
