//! ApexShot History — a browser for past captures and cloud uploads.
//!
//! The GUI-free foundations are in place and tested:
//!
//! * [`scan`] lists captures from the configured screenshot and video folders.
//! * [`thumbnails`] renders grid thumbnails on background threads with an
//!   on-disk cache.
//! * [`actions`] performs the per-item actions a card offers, reusing the
//!   app's existing open / clipboard / editor / upload plumbing.
//!
//! On top of those sit the GTK pieces: the [`window`] shell (sidebar + page
//! stack styled as a sibling of Settings), the local [`local_page`] grids, and
//! the Cloud page. The window is opened via [`show_history_window`].

pub mod actions;
pub mod cloud_page;
pub mod local_page;
pub mod scan;
pub mod thumbnails;
pub mod window;

use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::Application;

use crate::config::load_config;

/// One page of the History stack plus the hooks the window shell needs to
/// drive it from the shared header bar.
pub struct HistoryPage {
    /// The page widget placed into the window's stack.
    pub widget: gtk4::Widget,
    /// Reload the page's contents; wired to the header-bar refresh button.
    pub refresh: Rc<dyn Fn()>,
    /// Placeholder the shared search entry shows while this page is visible.
    pub search_placeholder: &'static str,
    /// Whether the shared search entry filters this page.
    pub searchable: bool,
}

/// Open the History window, presenting the existing one when it is already up.
///
/// History runs as its own single-instance application. It uses a dedicated
/// application id (`<app_id>.history`) with the default GApplication flags so it
/// never collides with the shared id that Settings claims: opening History while
/// Settings is open yields two independent windows, and a second `history`
/// invocation just presents the window already on screen.
pub fn show_history_window() -> anyhow::Result<()> {
    // Force-set GIO_LAUNCHED_DESKTOP_FILE to the main app's desktop entry
    // so GNOME Shell shows the correct icon and name (mirrors Settings).
    if let Some(desktop_path) = crate::app_identity::desktop_file_for_portal() {
        std::env::set_var("GIO_LAUNCHED_DESKTOP_FILE", desktop_path);
        std::env::set_var(
            "GIO_LAUNCHED_DESKTOP_FILE_PID",
            std::process::id().to_string(),
        );
    }

    // A dedicated application id with the default flags keeps History
    // single-instance and independent of the shared id Settings claims.
    let app = Application::builder()
        .application_id(format!("{}.history", crate::app_identity::app_id()))
        .build();

    app.connect_activate(|application| {
        // Single-instance behaviour: present the existing window rather than
        // building a second one.
        let windows = application.windows();
        if let Some(existing_window) = windows.first() {
            existing_window.present();
            return;
        }

        // Start the daemon if the tray should be visible, matching Settings.
        let config = load_config();
        if config.show_menu_bar_icon {
            let _ = crate::daemon::start_daemon_subprocess();
        }

        window::build_history_window(application);
    });

    let _ = app.run_with_args::<String>(&[]);
    Ok(())
}
