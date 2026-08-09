//! Overlay window shell: monitor resolution, layer-shell / fullscreen window,
//! drawing area, and screen geometry.
//!
//! Returns concrete `OverlayWindowParts` only — no event controllers, no
//! result channel ownership, no audio. Realization, focus, and present stay
//! with `setup_window` in `mod.rs`.

use super::super::api::SelectionError;
use super::super::background::BackgroundFrame;
use super::super::drawing::draw_overlay;
use super::super::monitor_picker::{find_monitor_at, select_target_monitor, MonitorChoice};
use super::super::state::{OverlayMode, SelectorState};
use super::platform::install_overlay_css;
use gtk4::prelude::*;
use gtk4::{gdk, Application, ApplicationWindow, DrawingArea};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::sync::{Arc, Mutex};

/// Concrete widgets and geometry produced by the shell builder.
pub(super) struct OverlayWindowParts {
    pub(super) window: ApplicationWindow,
    pub(super) drawing_area: DrawingArea,
    pub(super) screen_width: i32,
    pub(super) screen_height: i32,
}

/// Failures while building the shell (before input is wired).
#[derive(Debug)]
pub(super) enum ShellBuildError {
    NoDisplay,
    Monitor(SelectionError),
}

fn resolve_target_monitor(
    display: &gdk::Display,
    preselected: Option<MonitorChoice>,
) -> Result<gdk::Monitor, SelectionError> {
    if let Some(choice) = preselected {
        if let Some(monitor) = find_monitor_at(display, choice.x, choice.y) {
            return Ok(monitor);
        }
        eprintln!(
            "[overlay] preselected monitor at ({}, {}) not found; re-running picker",
            choice.x, choice.y
        );
    }
    select_target_monitor().map(|(monitor, _)| monitor)
}

/// Build the overlay window shell: CSS, monitor, window + layer-shell policy,
/// drawing area with draw func installed. Does **not** present the window.
pub(super) fn build_overlay_shell(
    app: &Application,
    state: &Arc<Mutex<SelectorState>>,
    background: Option<&BackgroundFrame>,
    preselected_monitor: Option<MonitorChoice>,
) -> Result<OverlayWindowParts, ShellBuildError> {
    // Suppress GTK-side animations so the overlay appears/disappears instantly.
    install_overlay_css();

    // Get the display and monitor for screen dimensions
    let display = match gdk::Display::default() {
        Some(d) => d,
        None => return Err(ShellBuildError::NoDisplay),
    };

    // Multi-monitor: use preselected choice (pick-then-freeze path) or show the
    // same floating display picker as the C++ overlay.
    let monitor = match resolve_target_monitor(&display, preselected_monitor) {
        Ok(m) => m,
        Err(e) => return Err(ShellBuildError::Monitor(e)),
    };

    let geometry = monitor.geometry();
    let screen_width = geometry.width();
    let screen_height = geometry.height();
    eprintln!(
        "[overlay] target monitor geom={}x{}+{}+{}",
        screen_width,
        screen_height,
        geometry.x(),
        geometry.y()
    );

    // Create the window
    let window = ApplicationWindow::builder()
        .application(app)
        .default_width(screen_width)
        .default_height(screen_height)
        .decorated(false)
        .resizable(false)
        .css_classes(["overlay", "transparent"])
        .build();

    let is_wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    // On Wayland, layer-shell gives a true transparent overlay surface.
    // Without this, some compositors show a black backing surface.
    let wayland_layer_shell = is_wayland && gtk4_layer_shell::is_supported();

    // NOTE: We no longer bail out when background.is_none() && Wayland-without-layer-shell.
    // Instead we fall through to window.set_fullscreened(true) which works on GNOME Wayland.
    // The drawing code already handles background=None by painting a dark semi-transparent
    // overlay — this is the "capture after selection" (Option B) path.

    if wayland_layer_shell {
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Bottom, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);
        let keyboard_mode = if state
            .lock()
            .map(|st| st.overlay_mode == OverlayMode::CrosshairCapture)
            .unwrap_or(false)
        {
            // Hyprland stops compositor global binds while a layer-shell surface
            // has exclusive keyboard focus. Crosshair mode is easy to leave open
            // accidentally, so avoid making all ApexShot shortcuts appear dead
            // until the app/overlay is restarted.
            KeyboardMode::OnDemand
        } else {
            KeyboardMode::Exclusive
        };
        window.set_keyboard_mode(keyboard_mode);
        window.set_monitor(Some(&monitor));
        window.set_namespace(Some("apexshot-area-selector"));
        window.set_exclusive_zone(-1);
    } else {
        // X11 or Wayland-without-layer-shell (e.g. GNOME Wayland):
        // Fullscreen on the chosen monitor so dual/multi-monitor setups
        // open the selector on the display the user picked.
        window.fullscreen_on_monitor(&monitor);
        window.set_decorated(false);
    }

    // Get the surface for cursor control
    let surface = window.surface();

    // Set cursor to crosshair when hovering over the window
    if let Some(ref surface) = surface {
        let cursor = gdk::Cursor::from_name("crosshair", None);
        surface.set_cursor(cursor.as_ref());
    }

    // Create a drawing area for rendering the selection
    let drawing_area = DrawingArea::builder().hexpand(true).vexpand(true).build();

    let state_draw = state.clone();
    let background_draw = background.cloned();
    drawing_area.set_draw_func(move |_, context, width, height| {
        draw_overlay(
            context,
            width,
            height,
            &state_draw,
            background_draw.as_ref(),
        );
    });

    // Set the drawing area as the child
    window.set_child(Some(&drawing_area));

    Ok(OverlayWindowParts {
        window,
        drawing_area,
        screen_width,
        screen_height,
    })
}

#[cfg(test)]
mod tests {
    /// Owner contract: shell owns monitor/window/layer-shell/drawing-area build.
    #[test]
    fn shell_owner_covers_monitor_window_and_drawing_area() {
        let source = include_str!("shell.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production shell source");
        assert!(
            production.contains("struct OverlayWindowParts"),
            "shell must define OverlayWindowParts"
        );
        assert!(
            production.contains("fn build_overlay_shell"),
            "shell must own build_overlay_shell"
        );
        assert!(
            production.contains("fn resolve_target_monitor"),
            "shell must own monitor resolution"
        );
        assert!(
            production.contains("init_layer_shell") && production.contains("fullscreen_on_monitor"),
            "shell must own layer-shell and fullscreen fallback"
        );
        assert!(
            production.contains("set_draw_func"),
            "shell must install the drawing-area draw func"
        );
        // Realization / present must stay with the setup coordinator.
        assert!(
            !production.contains("connect_realize") && !production.contains(".present()"),
            "shell must not present or install realize hooks"
        );
        assert!(
            !production.contains("send_selection_result")
                && !production.contains("recording_request_from_state"),
            "shell must not own result delivery"
        );
    }
}
