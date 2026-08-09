use super::api::{SelectionError, SelectionResult};
use super::background::BackgroundFrame;
use super::layout::{
    DEFAULT_SELECTION_HEIGHT, DEFAULT_SELECTION_WIDTH, MIN_SELECTION_HEIGHT, MIN_SELECTION_WIDTH,
};
use super::monitor_picker::MonitorChoice;
use super::state::{OverlayMode, SelectorState};
use gtk4::{prelude::*, Application};
use std::sync::{Arc, Mutex};

mod audio;
mod countdown;
mod input;
mod platform;
mod result;
mod shell;

pub(crate) use platform::suppress_x11_compositor_animation;
pub(crate) use result::send_selection_result;

use audio::install_overlay_audio_meters;
use input::{wire_selection_drag, wire_selection_motion, wire_window_click, wire_window_keyboard};
use shell::{build_overlay_shell, ShellBuildError};

pub(crate) fn setup_window(
    app: &Application,
    state: Arc<Mutex<SelectorState>>,
    result_tx: std::sync::mpsc::Sender<SelectionResult>,
    background: Option<BackgroundFrame>,
    preselected_monitor: Option<MonitorChoice>,
) {
    let shell::OverlayWindowParts {
        window,
        drawing_area,
        screen_width,
        screen_height,
    } = match build_overlay_shell(app, &state, background.as_ref(), preselected_monitor) {
        Ok(parts) => parts,
        Err(ShellBuildError::NoDisplay) => {
            let _ = result_tx.send(Err(SelectionError::InitError("No display found".into())));
            return;
        }
        Err(ShellBuildError::Monitor(SelectionError::Cancelled)) => {
            let _ = result_tx.send(Err(SelectionError::Cancelled));
            app.quit();
            return;
        }
        Err(ShellBuildError::Monitor(error)) => {
            let _ = result_tx.send(Err(error));
            app.quit();
            return;
        }
    };

    {
        let mut st = state.lock().unwrap();
        let screen_width_f = screen_width.max(1) as f64;
        let screen_height_f = screen_height.max(1) as f64;
        if st.overlay_mode == OverlayMode::CrosshairCapture {
            st.start_x = screen_width_f / 2.0;
            st.start_y = screen_height_f / 2.0;
            st.current_x = st.start_x;
            st.current_y = st.start_y;
            st.completed = false;
        } else {
            let initial_width = DEFAULT_SELECTION_WIDTH
                .min(screen_width_f)
                .max(MIN_SELECTION_WIDTH.min(screen_width_f));
            let initial_height = DEFAULT_SELECTION_HEIGHT
                .min(screen_height_f)
                .max(MIN_SELECTION_HEIGHT.min(screen_height_f));
            let initial_left = ((screen_width_f - initial_width) / 2.0).max(0.0);
            let initial_top = ((screen_height_f - initial_height) / 2.0).max(0.0);

            st.start_x = initial_left;
            st.start_y = initial_top;
            st.current_x = initial_left + initial_width;
            st.current_y = initial_top + initial_height;
            st.completed = true;
        }
        st.cancelled = false;
        st.is_dragging = false;
    }

    install_overlay_audio_meters(&state, &drawing_area);

    wire_selection_motion(
        &window,
        state.clone(),
        &drawing_area,
        screen_width,
        screen_height,
    );
    wire_window_click(
        &window,
        state.clone(),
        result_tx.clone(),
        &drawing_area,
        background.clone(),
        screen_width,
        screen_height,
    );
    wire_selection_drag(
        &window,
        state.clone(),
        result_tx.clone(),
        &drawing_area,
        background.clone(),
        screen_width,
        screen_height,
    );
    wire_window_keyboard(
        &window,
        state,
        result_tx,
        &drawing_area,
        background,
        screen_width,
        screen_height,
    );

    // Install X11 compositor hints before the first map event.
    let window_bypass = window.downgrade();
    window.connect_realize(move |_| {
        if let Some(window) = window_bypass.upgrade() {
            suppress_x11_compositor_animation(&window);
        }
    });

    let _ = window.grab_focus();
    window.present();
}

#[cfg(test)]
mod tests {
    #[test]
    fn overlay_owners_are_wired_from_setup_in_controller_order() {
        let source = include_str!("mod.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production setup source");

        for call in [
            "build_overlay_shell(",
            "install_overlay_audio_meters(",
            "wire_selection_motion(",
            "wire_window_click(",
            "wire_selection_drag(",
            "wire_window_keyboard(",
        ] {
            assert!(production.contains(call), "setup must call {call}");
        }

        let motion = production.find("wire_selection_motion(").unwrap();
        let click = production.find("wire_window_click(").unwrap();
        let drag = production.find("wire_selection_drag(").unwrap();
        let keyboard = production.find("wire_window_keyboard(").unwrap();
        assert!(
            motion < click && click < drag && drag < keyboard,
            "capture controller order must stay motion, primary/secondary click, drag, keyboard"
        );

        assert!(
            production.contains("mod countdown;") && production.contains("mod input;"),
            "setup must declare countdown and input modules"
        );
        assert!(
            production.contains("pub(crate) use result::send_selection_result"),
            "setup facade must re-export send_selection_result"
        );
        assert!(
            production.contains("connect_realize") && production.contains("window.present()"),
            "setup must keep realize hook and present"
        );

        assert!(
            !production.contains("ApplicationWindow::builder")
                && !production.contains("init_layer_shell")
                && !production.contains("poll_daemon_audio_levels")
                && !production.contains("start_local_audio_monitoring"),
            "setup must not retain shell or audio internals"
        );
        assert!(
            !production.contains("GestureClick")
                && !production.contains("GestureDrag::builder")
                && !production.contains("EventControllerMotion::new")
                && !production.contains("EventControllerKey::builder"),
            "setup must wire input facades instead of owning controllers inline"
        );
        assert!(
            !production.contains("recording_request_from_state")
                && !production.contains("send_selection_result(")
                && !production.contains("open_url"),
            "setup must not retain click delivery or URL behavior"
        );
    }
}
