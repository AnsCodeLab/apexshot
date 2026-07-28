//! The History window shell: a sibling of Settings that shares its chrome,
//! sidebar, and stylesheet.
//!
//! The construction here mirrors `settings::build_settings_window` closely so
//! the two windows are visually indistinguishable: same undecorated glass
//! frame, same traffic-light controls, same drag/resize gestures, same toast.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Once;
use std::time::Duration;

use gtk4::{
    glib, prelude::*, Align, Application, ApplicationWindow, Box as GtkBox, Button, Entry, Image,
    Label, Orientation, Overlay as GtkOverlay, ScrolledWindow,
};

use crate::settings::ui_support::{install_settings_css, traffic_light_button};
use crate::settings::windowing::{
    install_edge_resize, install_window_drag, prefers_dark_glass_theme,
    prefers_reduced_transparency,
};

use super::cloud_page::build_cloud_page;
use super::local_page::build_local_page;
use super::scan::MediaKind;

/// Toast severity, matching the Settings toast palette.
#[derive(Clone, Copy)]
pub enum ToastKind {
    Neutral,
    Success,
    Error,
}

/// A cloneable handle to the window's shared toast label. Pages call
/// [`HistoryToast::show`] to report outcomes exactly as Settings does.
#[derive(Clone)]
pub struct HistoryToast {
    label: Label,
    generation: Rc<Cell<u32>>,
}

impl HistoryToast {
    /// Show `text` with a severity; `auto_hide` fades it out after the delay.
    ///
    /// Mirrors `settings::show_settings_toast`: the label stays mapped and is
    /// toggled via opacity so the first reveal always paints, and a fresh show
    /// cancels any in-flight hide from a previous message.
    pub fn show(&self, text: &str, kind: ToastKind, auto_hide: Option<Duration>) {
        self.label.remove_css_class("settings-toast-success");
        self.label.remove_css_class("settings-toast-error");
        match kind {
            ToastKind::Neutral => {}
            ToastKind::Success => self.label.add_css_class("settings-toast-success"),
            ToastKind::Error => self.label.add_css_class("settings-toast-error"),
        }
        self.label.set_text(text);
        self.label.set_opacity(1.0);
        self.label.queue_draw();

        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);

        if let Some(delay) = auto_hide {
            let label = self.label.clone();
            let gen_cell = Rc::clone(&self.generation);
            glib::timeout_add_local_once(delay, move || {
                if gen_cell.get() == generation {
                    label.set_opacity(0.0);
                    label.remove_css_class("settings-toast-success");
                    label.remove_css_class("settings-toast-error");
                }
            });
        }
    }
}

/// Build the History `ApplicationWindow`, following the Settings shell exactly.
pub fn build_history_window(app: &Application) {
    static INIT_ICONS: Once = Once::new();
    INIT_ICONS.call_once(|| {
        relm4_icons::initialize_icons(
            crate::capture::editor::window::icon_names::GRESOURCE_BYTES,
            crate::capture::editor::window::icon_names::RESOURCE_PREFIX,
        );
    });

    install_settings_css();

    let prefers_dark = prefers_dark_glass_theme();
    let reduced_transparency = prefers_reduced_transparency();

    let window = ApplicationWindow::builder()
        .application(app)
        .title("ApexShot History")
        .icon_name(crate::app_identity::icon_name())
        .default_width(1020)
        .default_height(840)
        .build();

    window.set_decorated(false);
    window.add_css_class("editor-window");

    let root_box = GtkBox::new(Orientation::Vertical, 0);
    root_box.add_css_class("editor-root");
    root_box.add_css_class("history-root");
    if !prefers_dark {
        root_box.add_css_class("editor-theme-light");
    }
    if reduced_transparency {
        root_box.add_css_class("editor-reduced-transparency");
    }

    // --- TOOLBAR ---
    // GNOME-style borderless header bar: the shared search entry sits at the
    // far left and the refresh button immediately left of the window controls
    // (GNOME Settings / Files), with the drag handle filling the space between.
    let toolbar = GtkBox::new(Orientation::Horizontal, 0);
    toolbar.add_css_class("settings-window-controls");
    toolbar.set_size_request(-1, 46);

    let search = Entry::new();
    search.add_css_class("history-header-search");
    search.set_primary_icon_name(Some("system-search-symbolic"));
    search.set_valign(Align::Center);
    toolbar.append(&search);

    let drag_handle = GtkBox::new(Orientation::Horizontal, 0);
    drag_handle.set_hexpand(true);
    drag_handle.set_halign(Align::Fill);
    drag_handle.set_vexpand(false);
    toolbar.append(&drag_handle);

    let refresh_btn = Button::from_icon_name("view-refresh-symbolic");
    refresh_btn.add_css_class("history-header-refresh");
    refresh_btn.set_tooltip_text(Some("Refresh"));
    refresh_btn.set_valign(Align::Center);
    toolbar.append(&refresh_btn);

    let close_btn = traffic_light_button("traffic-light-red", "Close");
    close_btn.remove_css_class("recent-captures-wm-btn");
    close_btn.remove_css_class("recent-captures-wm-close");
    close_btn.add_css_class("recording-editor-traffic-btn");
    let win_clone = window.clone();
    close_btn.connect_clicked(move |_| win_clone.close());

    let min_btn = traffic_light_button("traffic-light-yellow", "Minimize");
    min_btn.remove_css_class("recent-captures-wm-btn");
    min_btn.add_css_class("recording-editor-traffic-btn");
    let win_clone = window.clone();
    min_btn.connect_clicked(move |_| win_clone.minimize());

    for button in [&close_btn, &min_btn] {
        button.set_size_request(24, 24);
        button.set_valign(Align::Center);
    }

    let right_box = GtkBox::new(Orientation::Horizontal, 6);
    right_box.set_halign(Align::End);
    right_box.append(&min_btn);
    right_box.append(&close_btn);
    toolbar.append(&right_box);

    // Toast: kept mapped at opacity 0 so the first reveal allocates correctly.
    let toast_label = Label::new(Some(""));
    toast_label.add_css_class("settings-toast");
    toast_label.set_halign(Align::Center);
    toast_label.set_valign(Align::Start);
    toast_label.set_margin_top(18);
    toast_label.set_opacity(0.0);
    toast_label.set_visible(true);
    toast_label.set_can_target(false);

    let toast = HistoryToast {
        label: toast_label.clone(),
        generation: Rc::new(Cell::new(0u32)),
    };

    let window_overlay = GtkOverlay::new();
    if !prefers_dark {
        window_overlay.add_css_class("editor-theme-light");
    }
    if reduced_transparency {
        window_overlay.add_css_class("editor-reduced-transparency");
    }
    window_overlay.set_child(Some(&root_box));
    window_overlay.add_overlay(&toast_label);

    root_box.append(&toolbar);

    // --- WINDOW GESTURES ---
    install_window_drag(&drag_handle, &window);
    install_edge_resize(&root_box, &window);

    // --- LAYOUT SPLIT ---
    let content_split = GtkBox::new(Orientation::Horizontal, 0);
    content_split.set_vexpand(true);
    content_split.set_hexpand(true);

    // --- SIDEBAR ---
    let sidebar_scroller = ScrolledWindow::new();
    sidebar_scroller.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

    let nav_strip = GtkBox::new(Orientation::Vertical, 4);
    nav_strip.add_css_class("settings-sidebar");
    nav_strip.set_halign(Align::Fill);
    nav_strip.set_valign(Align::Fill);
    nav_strip.set_hexpand(false);
    nav_strip.set_vexpand(true);

    sidebar_scroller.set_child(Some(&nav_strip));

    let sidebar_wrapper = GtkBox::new(Orientation::Vertical, 0);
    sidebar_wrapper.add_css_class("settings-sidebar-wrapper");
    sidebar_wrapper.set_vexpand(true);
    sidebar_wrapper.append(&sidebar_scroller);

    use crate::capture::editor::window::icon_names::custom;
    let labels = [
        ("Screenshots", custom::SCREENSHOOTER_SYMBOLIC),
        ("Recordings", custom::RECORD_SCREEN_SYMBOLIC),
        ("Cloud", custom::CLOUD_OUTLINE_THIN_SYMBOLIC),
    ];

    let stack = gtk4::Stack::new();
    stack.set_transition_type(gtk4::StackTransitionType::Crossfade);
    stack.set_vexpand(true);

    let mut nav_items = Vec::new();

    for (i, (label_text, icon_name)) in labels.iter().enumerate() {
        let item = GtkBox::new(Orientation::Horizontal, 8);
        item.add_css_class("settings-nav-item");
        item.set_halign(Align::Fill);
        item.set_valign(Align::Center);

        let icon = Image::from_icon_name(icon_name);
        icon.add_css_class("settings-nav-icon");
        icon.set_pixel_size(16);
        icon.set_halign(Align::Start);

        let label = Label::new(Some(label_text));
        label.add_css_class("settings-nav-label");
        label.set_halign(Align::Start);

        item.append(&icon);
        item.append(&label);

        if i == 0 {
            item.add_css_class("settings-nav-item-selected");
            icon.add_css_class("settings-nav-icon-selected");
            label.add_css_class("settings-nav-label-selected");
        }

        let motion = gtk4::EventControllerMotion::new();
        {
            let item = item.clone();
            let icon = icon.clone();
            let label = label.clone();
            motion.connect_enter(move |_, _, _| {
                item.add_css_class("settings-nav-item-hover");
                icon.add_css_class("settings-nav-icon-hover");
                label.add_css_class("settings-nav-label-hover");
            });
        }
        {
            let item = item.clone();
            let icon = icon.clone();
            let label = label.clone();
            motion.connect_leave(move |_| {
                item.remove_css_class("settings-nav-item-hover");
                icon.remove_css_class("settings-nav-icon-hover");
                label.remove_css_class("settings-nav-label-hover");
            });
        }
        item.add_controller(motion);

        let s_clone = stack.clone();
        let idx_str = i.to_string();
        let click = gtk4::GestureClick::new();
        click.connect_released(move |_, _, _, _| {
            s_clone.set_visible_child_name(&idx_str);
        });
        item.add_controller(click);

        nav_strip.append(&item);
        nav_items.push((item, icon, label));
    }

    content_split.append(&sidebar_wrapper);

    // --- BODY (STACK) ---
    let body_frame = GtkBox::new(Orientation::Vertical, 0);
    body_frame.set_vexpand(true);
    body_frame.set_hexpand(true);

    let pages = [
        build_local_page(MediaKind::Image, toast.clone(), &search),
        build_local_page(MediaKind::Video, toast.clone(), &search),
        build_cloud_page(toast.clone()),
    ];

    // Hooks the header bar drives: refresh reloads the visible page, and the
    // shared search entry takes each page's placeholder and sensitivity.
    let refresh_hooks: Rc<Vec<Rc<dyn Fn()>>> =
        Rc::new(pages.iter().map(|page| Rc::clone(&page.refresh)).collect());
    let search_meta: Vec<(bool, &'static str)> = pages
        .iter()
        .map(|page| (page.searchable, page.search_placeholder))
        .collect();

    stack.add_titled(&pages[0].widget, Some("0"), "Screenshots");
    stack.add_titled(&pages[1].widget, Some("1"), "Recordings");
    stack.add_titled(&pages[2].widget, Some("2"), "Cloud");

    {
        let stack = stack.clone();
        let refresh_hooks = Rc::clone(&refresh_hooks);
        refresh_btn.connect_clicked(move |_| {
            if let Some(name) = stack.visible_child_name() {
                if let Ok(idx) = name.parse::<usize>() {
                    if let Some(reload) = refresh_hooks.get(idx) {
                        reload();
                    }
                }
            }
        });
    }

    body_frame.append(&stack);

    // Keep the sidebar selection and header-bar search in sync when the
    // visible page changes.
    let nav_items_clone = nav_items.clone();
    let search_for_switch = search.clone();
    stack.connect_visible_child_name_notify(move |s| {
        if let Some(name) = s.visible_child_name() {
            if let Ok(idx) = name.parse::<usize>() {
                if let Some(&(searchable, placeholder)) = search_meta.get(idx) {
                    search_for_switch.set_sensitive(searchable);
                    search_for_switch.set_placeholder_text(Some(placeholder));
                }
                for (i, (item, icon, label)) in nav_items_clone.iter().enumerate() {
                    if i == idx {
                        item.add_css_class("settings-nav-item-selected");
                        icon.add_css_class("settings-nav-icon-selected");
                        label.add_css_class("settings-nav-label-selected");
                    } else {
                        item.remove_css_class("settings-nav-item-selected");
                        icon.remove_css_class("settings-nav-icon-selected");
                        label.remove_css_class("settings-nav-label-selected");
                    }
                }
            }
        }
    });
    stack.set_visible_child_name("0");

    content_split.append(&body_frame);
    root_box.append(&content_split);
    window.set_child(Some(&window_overlay));

    window.present();
}
