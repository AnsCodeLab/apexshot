//! Cloud page for the History window.
//!
//! This page renders the signed-in account's ApexShot Cloud uploads. It is a
//! small state machine driven entirely by config and the cached entitlement:
//!
//! | State            | What the user sees                                    |
//! |------------------|-------------------------------------------------------|
//! | Not signed in    | Explanation + a "Sign in" button, then login polling  |
//! | Signed in, free  | The signed-in email + a prompt to upgrade             |
//! | XBackBone chosen | A note that this page covers ApexShot Cloud only      |
//! | Error            | A readable message with a Retry button                |
//! | Subscribed       | A paged grid of the account's uploads (Step 9)        |
//!
//! Networking (listing, thumbnails) always happens on background threads and
//! results are delivered to the main loop through `mpsc` channels drained by
//! `glib::idle_add_local`, mirroring `settings::cloud` and the local grids.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

use gtk4::prelude::*;
use gtk4::{
    glib, Align, Box as GtkBox, Button, FlowBox, Image, Label, Orientation, Picture,
    ScrolledWindow, SelectionMode, Widget,
};

use crate::cloud::listing::{
    cached_is_subscribed, CloudReadError, CloudUpload, UploadsPage, UploadsPager, DEFAULT_PAGE_SIZE,
};
use crate::config::{is_cloud_logged_in, load_config, AppConfig};
use crate::history::thumbnails::{
    self, ThumbnailReady, ThumbnailRequest, ThumbnailSource, THUMB_HEIGHT, THUMB_WIDTH,
};

use super::actions;
use super::window::{HistoryToast, ToastKind};

/// How long an action-outcome toast stays up before fading.
const TOAST_SUCCESS: Duration = Duration::from_secs(2);
const TOAST_ERROR: Duration = Duration::from_secs(4);

/// How often to re-check config for a login that landed in a terminal.
const LOGIN_POLL_SECONDS: u32 = 2;

/// Build the History window's Cloud page.
///
/// Returns the page widget (a self-contained scroller with the same title
/// header and margins the local pages use) plus a refresh hook that re-renders
/// from current config. The page evaluates its state immediately and rebuilds
/// itself in place as the session or entitlement changes, so the caller never
/// has to rebuild it. Cloud uploads page in from the server, so the shared
/// header-bar search does not filter this page.
///
/// `toast` is the shared window toast (the same `HistoryToast` handed to
/// `build_local_page`), used to report per-card action outcomes.
pub fn build_cloud_page(toast: HistoryToast) -> super::HistoryPage {
    // Same page chrome the local pages build: an outer vertical scroller and a
    // margined column with a settings-style title header, so the three stack
    // pages line up pixel-for-pixel.
    let scroller = ScrolledWindow::new();
    scroller.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    scroller.set_vexpand(true);
    scroller.set_hexpand(true);

    let column = GtkBox::new(Orientation::Vertical, 0);
    column.add_css_class("recent-captures-root");
    column.set_margin_top(20);
    column.set_margin_bottom(32);
    column.set_margin_start(28);
    column.set_margin_end(28);

    let title = Label::new(Some("Cloud"));
    title.add_css_class("recent-captures-title");
    title.set_halign(Align::Start);

    let subtitle = Label::new(Some("Everything you have uploaded to ApexShot Cloud"));
    subtitle.add_css_class("history-page-subtitle");
    subtitle.set_halign(Align::Start);
    subtitle.set_margin_bottom(18);

    column.append(&title);
    column.append(&subtitle);

    // Body holds whichever state is currently rendered; a rebuild only ever
    // clears and refills this box, leaving the title header untouched.
    let body = GtkBox::new(Orientation::Vertical, 0);
    body.set_hexpand(true);
    body.set_vexpand(true);
    column.append(&body);

    scroller.set_child(Some(&column));

    let page = Rc::new(CloudPage {
        body,
        scroller: scroller.clone(),
        toast,
        polling: Cell::new(false),
    });

    page.render_current_state();

    // The header-bar refresh button re-renders from the current config.
    let refresh = {
        let page = Rc::clone(&page);
        Rc::new(move || page.render_current_state()) as Rc<dyn Fn()>
    };

    super::HistoryPage {
        widget: scroller.upcast(),
        refresh,
        search_placeholder: "Search isn't available for Cloud",
        searchable: false,
    }
}

struct CloudPage {
    /// The refillable body of the page (everything below the title header).
    body: GtkBox,
    /// The page's outer scroller, whose vadjustment drives grid paging.
    scroller: ScrolledWindow,
    toast: HistoryToast,
    /// True while a login poll timer is running, so we never start a second.
    polling: Cell<bool>,
}

impl CloudPage {
    /// Clear the page and lay out whichever state config currently describes.
    fn render_current_state(self: &Rc<Self>) {
        clear_box(&self.body);
        let config = load_config();

        // XBackBone users have no ApexShot Cloud listing to show, whatever their
        // session state — check the destination before anything else.
        if config.cloud_destination == "xbackbone" {
            self.show_xbackbone_notice();
            return;
        }

        if !is_cloud_logged_in(&config) {
            self.show_signed_out();
            return;
        }

        if cached_is_subscribed(&config) {
            self.show_subscribed_grid(config);
        } else {
            self.show_free_plan(&config);
        }
    }

    // --- signed-out state ---

    fn show_signed_out(self: &Rc<Self>) {
        let state = empty_state(
            "Sign in to ApexShot Cloud",
            "Connect your ApexShot Cloud account to browse everything you have uploaded, \
             right here in History.",
        );

        let sign_in = Button::with_label("Sign in");
        sign_in.add_css_class("recent-captures-primary-button");
        sign_in.set_halign(Align::Center);
        sign_in.set_margin_top(20);

        {
            let page = Rc::clone(self);
            sign_in.connect_clicked(move |btn| {
                crate::settings::cloud::spawn_apexshot_login();
                btn.set_label("Waiting for sign-in\u{2026}");
                btn.set_sensitive(false);
                page.start_login_polling();
            });
        }

        state.append(&sign_in);
        self.body.append(&state);

        // A terminal login may already be in flight from a previous click; keep
        // watching so the page flips to the grid the moment it lands.
        self.start_login_polling();
    }

    /// Watch config for a session to appear after `spawn_apexshot_login`, then
    /// re-render. Runs at most one timer at a time.
    fn start_login_polling(self: &Rc<Self>) {
        if self.polling.get() {
            return;
        }
        self.polling.set(true);

        let page = Rc::clone(self);
        glib::timeout_add_seconds_local(LOGIN_POLL_SECONDS, move || {
            // Stop if the page state moved on (e.g. the user is no longer on the
            // signed-out screen because a rebuild already happened).
            if is_cloud_logged_in(&load_config()) {
                page.polling.set(false);
                page.render_current_state();
                return glib::ControlFlow::Break;
            }
            glib::ControlFlow::Continue
        });
    }

    // --- free-plan state ---

    fn show_free_plan(self: &Rc<Self>, config: &AppConfig) {
        let email = config.cloud_user_email.trim();
        let detail = if email.is_empty() {
            "You are signed in to ApexShot Cloud on the free plan. Upgrade to a paid plan to \
             browse and manage your uploads here."
                .to_string()
        } else {
            format!(
                "You are signed in as {email} on the free plan. Upgrade to a paid plan to \
                 browse and manage your uploads here."
            )
        };

        let state = empty_state("Upgrade to browse your cloud uploads", &detail);

        let upgrade = Button::with_label("See plans");
        upgrade.add_css_class("recent-captures-primary-button");
        upgrade.set_halign(Align::Center);
        upgrade.set_margin_top(20);
        {
            let toast = self.toast.clone();
            upgrade.connect_clicked(move |_| {
                report(
                    &toast,
                    actions::open_in_browser("https://apexshot.org/pricing"),
                );
            });
        }
        state.append(&upgrade);

        self.body.append(&state);
    }

    // --- XBackBone state ---

    fn show_xbackbone_notice(self: &Rc<Self>) {
        let state = empty_state(
            "This page covers ApexShot Cloud",
            "Your uploads currently go to a self-hosted XBackBone instance. Switch your upload \
             destination to ApexShot Cloud in Settings to browse those uploads here.",
        );
        self.body.append(&state);
    }

    // --- error state ---

    fn show_error(self: &Rc<Self>, message: &str) {
        // Replace whatever partial grid chrome was already in the body (the
        // "Loading…" status line, an empty grid) with a clean error state.
        clear_box(&self.body);
        let state = empty_state("Could not load your cloud uploads", message);

        let retry = Button::with_label("Retry");
        retry.add_css_class("recent-captures-secondary-button");
        retry.set_halign(Align::Center);
        retry.set_margin_top(20);
        {
            let page = Rc::clone(self);
            retry.connect_clicked(move |_| {
                page.render_current_state();
            });
        }
        state.append(&retry);

        self.body.append(&state);
    }

    // --- subscribed grid (Step 9) ---

    fn show_subscribed_grid(self: &Rc<Self>, config: AppConfig) {
        let grid = FlowBox::new();
        grid.add_css_class("recent-captures-grid");
        grid.set_selection_mode(SelectionMode::None);
        grid.set_homogeneous(true);
        grid.set_max_children_per_line(4);
        grid.set_min_children_per_line(1);
        grid.set_row_spacing(8);
        grid.set_column_spacing(8);
        grid.set_halign(Align::Fill);
        grid.set_valign(Align::Start);
        grid.set_hexpand(true);

        // A status line doubles as the empty-state message once the first page
        // has come back with nothing.
        let status = Label::new(Some("Loading your cloud uploads\u{2026}"));
        status.add_css_class("recent-captures-empty-detail");
        status.set_halign(Align::Center);
        status.set_margin_top(16);
        status.set_margin_bottom(8);

        let load_more = Button::with_label("Load more");
        load_more.add_css_class("recent-captures-secondary-button");
        load_more.add_css_class("history-load-more");
        load_more.set_halign(Align::Center);
        load_more.set_margin_top(16);
        load_more.set_visible(false);

        // The grid, status line, and load-more button sit directly in the page
        // body — the page already lives inside the window's outer scroller, so
        // nesting a second scroller here would trap the wheel and split paging.
        self.body.append(&grid);
        self.body.append(&status);
        self.body.append(&load_more);

        // A fresh thumbnail batch id, so a page rebuild ignores late results
        // from a previous session's in-flight decodes.
        let generation = thumbnails::next_generation();

        // Own the pager behind an Rc<RefCell> so both the button and the scroll
        // handler drive the same cursor, and a `Cell<bool>` gate so overlapping
        // page requests can never be issued.
        let ctx = Rc::new(GridContext {
            page: Rc::clone(self),
            config,
            pager: RefCell::new(UploadsPager::new(DEFAULT_PAGE_SIZE)),
            loading_in_flight: Cell::new(false),
            first_page_loaded: Cell::new(false),
            next_card_id: Cell::new(0),
            generation,
            grid,
            status,
            load_more: load_more.clone(),
        });

        {
            let ctx = Rc::clone(&ctx);
            load_more.connect_clicked(move |_| {
                ctx.request_next_page();
            });
        }

        // Trigger the next page as the user nears the bottom of the page's own
        // outer scroller. The non-overlap gate in `request_next_page` keeps a
        // burst of scroll events from firing more than one fetch.
        {
            let ctx = Rc::clone(&ctx);
            let adjustment = self.scroller.vadjustment();
            adjustment.connect_value_changed(move |adj| {
                let remaining = adj.upper() - (adj.value() + adj.page_size());
                if remaining < adj.page_size() {
                    ctx.request_next_page();
                }
            });
        }

        ctx.request_next_page();
    }
}

/// Everything a live cloud grid needs to page and render.
struct GridContext {
    page: Rc<CloudPage>,
    config: AppConfig,
    pager: RefCell<UploadsPager>,
    /// Non-overlap gate: at most one page request may be outstanding.
    loading_in_flight: Cell<bool>,
    first_page_loaded: Cell<bool>,
    next_card_id: Cell<u64>,
    generation: u64,
    grid: FlowBox,
    status: Label,
    load_more: Button,
}

impl GridContext {
    /// Fetch the next page on a worker thread, unless the pager is exhausted or
    /// a request is already in flight.
    fn request_next_page(self: &Rc<Self>) {
        if self.loading_in_flight.get() {
            return;
        }
        if self.pager.borrow().is_exhausted() {
            self.load_more.set_visible(false);
            return;
        }
        self.loading_in_flight.set(true);
        self.load_more.set_sensitive(false);

        // The pager is stateful and !Send-friendly to keep on the UI thread, so
        // clone it into the worker and copy back the advanced cursor on return.
        let mut pager = self.pager.borrow().clone();
        let config = self.config.clone();
        let (tx, rx) = mpsc::channel::<PageOutcome>();
        std::thread::spawn(move || {
            let result = pager.next_page(&config);
            let _ = tx.send(PageOutcome { pager, result });
        });

        let ctx = Rc::clone(self);
        glib::source::idle_add_local(move || match rx.try_recv() {
            Ok(outcome) => {
                ctx.on_page_result(outcome);
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            // The worker vanished without sending: release the gate so a later
            // scroll or click can retry rather than wedging forever.
            Err(mpsc::TryRecvError::Disconnected) => {
                ctx.loading_in_flight.set(false);
                ctx.load_more.set_sensitive(true);
                glib::ControlFlow::Break
            }
        });
    }

    fn on_page_result(self: &Rc<Self>, outcome: PageOutcome) {
        self.loading_in_flight.set(false);
        self.load_more.set_sensitive(true);
        // Adopt the worker's advanced cursor so the next request continues.
        *self.pager.borrow_mut() = outcome.pager;

        match outcome.result {
            Ok(Some(page)) => self.append_page(page),
            // The pager reports completion as Ok(None).
            Ok(None) => {
                self.load_more.set_visible(false);
                self.finish_if_empty();
            }
            Err(error) => self.on_page_error(error),
        }
    }

    fn append_page(self: &Rc<Self>, page: UploadsPage) {
        let is_first = !self.first_page_loaded.get();
        self.first_page_loaded.set(true);

        for upload in &page.items {
            let card = self.build_card(upload);
            self.grid.insert(&card, -1);
        }

        let exhausted = self.pager.borrow().is_exhausted() || !page.has_more;
        self.load_more.set_visible(!exhausted);

        if is_first {
            self.finish_if_empty();
        }
    }

    /// Update or hide the status line once we know whether anything loaded.
    fn finish_if_empty(&self) {
        if self.grid.first_child().is_none() {
            self.status.set_visible(true);
            self.status
                .set_text("You have not uploaded anything to ApexShot Cloud yet.");
        } else {
            self.status.set_visible(false);
        }
    }

    fn on_page_error(self: &Rc<Self>, error: CloudReadError) {
        // If the very first page failed there is nothing to show, so replace the
        // whole page with the error state (which offers a clean retry). A later
        // page failing keeps the cards already on screen and reports via toast.
        if !self.first_page_loaded.get() {
            self.page.show_error(&error.to_string());
        } else {
            self.load_more.set_visible(true);
            self.page
                .toast
                .show(&error.to_string(), ToastKind::Error, Some(TOAST_ERROR));
        }
    }

    fn build_card(self: &Rc<Self>, upload: &CloudUpload) -> Widget {
        let card = GtkBox::new(Orientation::Vertical, 0);
        card.add_css_class("recent-captures-card");
        card.set_hexpand(true);

        // Thumbnail area, sized to the card so the grid stays even while images
        // stream in. A remote thumbnail decodes on the shared pool; uploads with
        // no thumbnail URL keep the placeholder frame.
        let frame = GtkBox::new(Orientation::Vertical, 0);
        frame.add_css_class("recent-captures-card-image");
        frame.set_size_request(THUMB_WIDTH as i32, THUMB_HEIGHT as i32);
        frame.set_overflow(gtk4::Overflow::Hidden);

        let picture = Picture::new();
        picture.set_hexpand(true);
        picture.set_vexpand(true);
        picture.set_can_shrink(true);
        picture.set_valign(Align::Fill);
        picture.set_halign(Align::Fill);
        frame.append(&picture);
        card.append(&frame);

        if let Some(url) = upload
            .thumbnail_url
            .as_deref()
            .map(str::trim)
            .filter(|url| !url.is_empty())
        {
            self.load_thumbnail(url, &picture, &frame);
        } else {
            add_missing_badge(&frame);
        }

        // Filename.
        let title = Label::new(Some(&upload.display_name()));
        title.add_css_class("recent-captures-card-title");
        title.set_xalign(0.0);
        title.set_halign(Align::Start);
        title.set_max_width_chars(24);
        title.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        card.append(&title);

        // Timestamp.
        let when = format_upload_time(upload);
        if !when.is_empty() {
            let ts = Label::new(Some(&when));
            ts.add_css_class("recent-captures-card-timestamp");
            ts.set_xalign(0.0);
            ts.set_halign(Align::Start);
            card.append(&ts);
        }

        // Size.
        if let Some(size) = upload.size_bytes.filter(|bytes| *bytes > 0) {
            let meta = Label::new(Some(&super::scan::format_size(size as u64)));
            meta.add_css_class("recent-captures-card-meta");
            meta.set_xalign(0.0);
            meta.set_halign(Align::Start);
            card.append(&meta);
        }

        // Per-card actions: copy the share link, open it in the browser.
        card.append(&self.build_card_actions(upload));

        card.upcast()
    }

    fn build_card_actions(self: &Rc<Self>, upload: &CloudUpload) -> GtkBox {
        let actions_row = GtkBox::new(Orientation::Horizontal, 4);
        actions_row.set_halign(Align::Start);
        actions_row.set_margin_top(6);

        let share_url = upload
            .share_url
            .as_deref()
            .map(str::trim)
            .filter(|url| !url.is_empty())
            .map(str::to_string);

        let copy = Button::with_label("Copy link");
        copy.add_css_class("recent-captures-icon-btn");
        let open = Button::with_label("Open");
        open.add_css_class("recent-captures-icon-btn");

        match share_url {
            Some(url) => {
                {
                    let toast = self.page.toast.clone();
                    let url = url.clone();
                    copy.connect_clicked(move |_| {
                        report(&toast, actions::copy_link_to_clipboard(&url));
                    });
                }
                {
                    let toast = self.page.toast.clone();
                    open.connect_clicked(move |_| {
                        report(&toast, actions::open_in_browser(&url));
                    });
                }
            }
            None => {
                // No share link on this upload: keep the buttons visible but
                // inert so the card layout stays uniform.
                copy.set_sensitive(false);
                open.set_sensitive(false);
            }
        }

        actions_row.append(&copy);
        actions_row.append(&open);
        actions_row
    }

    /// Decode a remote thumbnail on the shared pool and drop it into `picture`
    /// when it arrives, discarding results from a superseded page generation.
    fn load_thumbnail(self: &Rc<Self>, url: &str, picture: &Picture, frame: &GtkBox) {
        let id = self.next_card_id.get();
        self.next_card_id.set(id + 1);

        let (tx, rx) = mpsc::channel::<ThumbnailReady>();
        thumbnails::submit(ThumbnailRequest {
            id,
            generation: self.generation,
            source: ThumbnailSource::Remote(url.to_string()),
            reply: tx,
        });

        let generation = self.generation;
        let picture = picture.clone();
        let frame = frame.clone();
        glib::source::idle_add_local(move || match rx.try_recv() {
            Ok(ready) => {
                if ready.generation == generation {
                    match ready.result {
                        Ok(path) => picture.set_filename(Some(&path)),
                        Err(_) => add_missing_badge(&frame),
                    }
                }
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
        });
    }
}

/// Result of a background page fetch, carrying the advanced pager back so the
/// UI thread can continue from where the worker left off.
struct PageOutcome {
    pager: UploadsPager,
    result: Result<Option<UploadsPage>, CloudReadError>,
}

// --- shared helpers ---

/// A centred empty-state card matching the gallery vocabulary. Callers append
/// their own action button.
fn empty_state(title: &str, detail: &str) -> GtkBox {
    let state = GtkBox::new(Orientation::Vertical, 0);
    state.add_css_class("recent-captures-empty-state");
    // History-only hook: flat, borderless panel sitting higher on the page.
    state.add_css_class("history-cloud-state");
    state.set_halign(Align::Center);
    state.set_valign(Align::Center);
    state.set_hexpand(true);
    state.set_vexpand(true);
    state.set_margin_top(12);

    let title_lbl = Label::new(Some(title));
    title_lbl.add_css_class("recent-captures-empty-title");
    title_lbl.set_halign(Align::Center);
    title_lbl.set_justify(gtk4::Justification::Center);
    state.append(&title_lbl);

    let detail_lbl = Label::new(Some(detail));
    detail_lbl.add_css_class("recent-captures-empty-detail");
    detail_lbl.set_halign(Align::Center);
    detail_lbl.set_justify(gtk4::Justification::Center);
    detail_lbl.set_wrap(true);
    detail_lbl.set_max_width_chars(52);
    state.append(&detail_lbl);

    state
}

/// Drop a "no preview" badge into a thumbnail frame that has no image.
///
/// Idempotent: adding the marker class more than once is harmless, and a frame
/// that already shows a badge simply gets a second identical one only if called
/// twice, which never happens (build-time xor thumbnail-failure, not both).
fn add_missing_badge(frame: &GtkBox) {
    frame.add_css_class("recent-captures-picture-missing");

    let badge = Image::from_icon_name("image-missing-symbolic");
    badge.add_css_class("history-media-badge");
    badge.set_pixel_size(16);
    badge.set_halign(Align::Center);
    badge.set_valign(Align::Center);
    badge.set_hexpand(true);
    badge.set_vexpand(true);
    frame.append(&badge);
}

/// Human-readable timestamp for an upload, empty when the server sent nothing
/// usable. Uses the parsed UTC time formatted in the local zone.
fn format_upload_time(upload: &CloudUpload) -> String {
    match upload.created_at_utc() {
        Some(utc) => utc
            .with_timezone(&chrono::Local)
            .format("%b %-d, %Y")
            .to_string(),
        None => String::new(),
    }
}

/// Send an action outcome to the window's toast: the `Ok` message as success,
/// the `Err` message as an error.
fn report<T: AsRef<str>>(toast: &HistoryToast, outcome: Result<T, String>) {
    match outcome {
        Ok(message) => toast.show(message.as_ref(), ToastKind::Success, Some(TOAST_SUCCESS)),
        Err(message) => toast.show(&message, ToastKind::Error, Some(TOAST_ERROR)),
    }
}

/// Remove every child of a box, so a state transition starts from a clean slate.
fn clear_box(container: &GtkBox) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}
