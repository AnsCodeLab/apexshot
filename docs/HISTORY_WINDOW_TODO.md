# History Window — Remaining Work

The `src/history/` module has its GUI-free foundations (scan, thumbnails, actions) committed. Everything below still needs to be built before the feature is complete.

---

## What's done

| Step | Description |
|------|-------------|
| 4 | `scan.rs` — local capture scanner (image/video, newest-first, unit-tested) |
| 5 (partial) | `thumbnails.rs` — off-main-thread pool, on-disk cache, ffmpeg poster, remote cache |
| 7 (partial) | `actions.rs` — open, editor, clipboard, reveal, upload, delete (all reuse existing plumbing) |

Supporting changes already landed:
- `src/recording/editor/ffmpeg.rs` — `extract_poster_frame()`
- `src/daemon/mod.rs` — `open_file()` is now `pub`, returns `Result<(), String>`
- `src/settings/mod.rs` — `cloud` submodule is `pub(crate)`
- `src/settings/cloud.rs` — `spawn_apexshot_login()` is `pub(crate)`

---

## What remains

### Step 1 — Window shell (`src/history/window.rs`)

Build the `ApplicationWindow` following `src/settings/mod.rs:142-500` exactly:

- `build_history_window(app: &Application)`
- `relm4_icons::initialize_icons(GRESOURCE_BYTES, RESOURCE_PREFIX)` via `Once`
- `install_settings_css()` (same CSS, so visual parity is automatic)
- `prefers_dark_glass_theme()` / `prefers_reduced_transparency()` → CSS class toggles
- Undecorated window, `editor-window` + `editor-root` classes
- `settings-window-controls` toolbar with `traffic_light_button` close + minimize, wired to `window.close()` / `window.minimize()`
- `GtkOverlay` carrying the root box + a toast label (same `settings-toast` setup)
- `install_window_drag(&drag_handle, &window)`, `install_edge_resize(&root_box, &window)`

### Step 2 — Sidebar + page stack (`src/history/window.rs`)

Mirror the Settings sidebar pattern:

- `settings-sidebar-wrapper` → `settings-sidebar` scroller → vertical nav strip
- Three items: **Screenshots**, **Recordings**, **Cloud** — with icons from `icon_names::custom::*`
  - Screenshots → `SCREENSHOOTER_SYMBOLIC`
  - Recordings → `RECORD_SCREEN_SYMBOLIC`
  - Cloud → `CLOUD_OUTLINE_THIN_SYMBOLIC`
- Hover/selected CSS classes on motion + click (same pattern as Settings nav items)
- `gtk4::Stack` with `Crossfade`, wired bidirectionally to the nav selection
- Each page wrapped in a `ScrolledWindow` with a `settings-page-title` header and `20/32/28/28` margins

### Step 3 — Adopt the existing gallery stylesheet

The `recent-captures-*` CSS vocabulary in `src/settings/ui_support.rs:1298-1713` defines everything needed. Use those classes for:

- Card grid: `recent-captures-grid`, `recent-captures-card`, `recent-captures-card-image`, `recent-captures-card-title`, `recent-captures-card-timestamp`
- Empty state: `recent-captures-empty-state`, `recent-captures-empty-title`, `recent-captures-empty-detail`
- Buttons: `recent-captures-primary-button`, `recent-captures-secondary-button`, `recent-captures-refresh-button`, `recent-captures-icon-btn`
- Media badge: `recent-captures-media-badge`
- Missing picture: `recent-captures-picture-missing`

Only extend the stylesheet (add new rules at the end of `SETTINGS_CSS`) for anything genuinely absent.

### Step 6 — Grid, search, and empty states (`src/history/local_page.rs`)

- `build_local_page(kind: MediaKind) -> gtk4::Widget`
- On first show: call `scan::scan_folder()` on a background thread, attach cards incrementally via `glib::idle_add_local`
- Each card: `Picture` or `Image` showing the thumbnail (placeholder while pending), filename label, relative-time label
- Search `Entry` filtering the visible cards by `entry.search_key()` on every keystroke
- Refresh `Button` that cancels the current thumbnail generation, rescans, and rebuilds
- Empty-state widget shown when the folder has no matching files

Thumbnail delivery loop (background → UI):
```
let (thumb_tx, thumb_rx) = mpsc::channel::<ThumbnailReady>();
// submit ThumbnailRequest for each entry, then drain thumb_rx with glib::idle_add_local
```

### Step 7 (remainder) — Per-item action popover

When a card is clicked, show a popover (or inline action bar) with:
- Open in default app → `actions::open_in_default_app`
- Open in editor → `actions::open_in_apexshot_editor`
- Copy to clipboard → `actions::copy_to_clipboard`
- Reveal in file manager → `actions::reveal_in_file_manager`
- Upload to cloud → `actions::upload_to_cloud` (spawned on a thread)
- **Delete** → confirm dialog first, then `actions::delete_capture`, then remove the card

Report every outcome with the same `show_settings_toast()` idiom used in Settings.

### Step 8 — Cloud page state machine (`src/history/cloud_page.rs`)

States driven by `cloud::listing::cached_is_subscribed()` + `config::is_cloud_logged_in()`:

| State | What to show |
|-------|-------------|
| Not signed in | Explanation + "Sign in" button → `settings::cloud::spawn_apexshot_login()`, then poll `load_config().cloud_user_email` every 2 s (same as Settings cloud page) |
| Signed in, free plan | Explanation showing the signed-in email; prompt to upgrade |
| XBackBone destination | Explanation that this page covers ApexShot Cloud only |
| Error | Readable message with a Retry button |
| Subscribed | Cloud grid (step 9) |

### Step 9 — Cloud grid with paging (`src/history/cloud_page.rs`)

For a subscribed account:
- Background thread: `cloud::listing::UploadsPager::new(DEFAULT_PAGE_SIZE)`, call `pager.next_page(&config)`, deliver results via `mpsc` + `glib::idle_add_local`
- Card: thumbnail via `thumbnails::ThumbnailSource::Remote(url)`, filename, timestamp (`upload.created_at_utc()`), size
- "Load more" button / scroll-triggered load for the next page; hide it when `pager.is_exhausted()`
- Never issue overlapping page requests (gate with a `Cell<bool> loading_in_flight`)
- Per-card actions: copy share link → `actions::copy_link_to_clipboard`, open in browser → `actions::open_in_browser`
- Network/server failures → readable message from `CloudReadError`, Retry button

### Step 10 — `history` terminal command (`src/main.rs`)

Same two-step pattern as `settings`:

```rust
// User-facing arm in async_main:
"history" => {
    let exe = std::env::current_exe()?;
    std::process::Command::new(&exe).arg("history-internal").spawn()?;
}

// GTK arm dispatched before the tokio runtime:
"history-internal" => {
    history::show_history_window()?;
}
```

Add `apexshot history` to `print_usage()` near the other window commands.

### Step 11 — Tray menu entry + daemon wiring

Four places to edit:

1. **`src/tray/mod.rs`** — add `TrayAction::OpenHistory` to the enum; add an `ltr("History")` item to the idle `menu()` near "Open Last Capture" and "Settings".
2. **`src/daemon/mod.rs`**:
   - Add `DaemonAction::OpenHistory` to the enum.
   - Add `TrayAction::OpenHistory => DaemonAction::OpenHistory` to `From<TrayAction>`.
   - Add an action-loop arm: `DaemonAction::OpenHistory => { tokio::task::spawn_blocking(spawn_history_subprocess); }`.
   - Add the D-Bus string mapping: `"history" => DaemonAction::OpenHistory`.
   - Add `fn spawn_history_subprocess()` (mirrors `show_settings_subprocess`, passes `"history"`).

### Step 12 — Single-instance behaviour (`src/history/mod.rs`)

`show_history_window()` must use its **own** application id (e.g. `format!("{}.history", app_identity::app_id())`), not the shared one Settings claims. This matches what the editors do with `ApplicationFlags::NON_UNIQUE` — but History wants single-instance, so use a distinct id with the default flags instead.

Verify that opening History while Settings is open gives two independent windows, not one hijacking the other.

### Step 13 — Docs

- **`README.md`**: add `apexshot history` to the CLI Commands table near `apexshot settings`.
- **`docs/MODULES.md`**: add a History section describing the module layout and the window's relationship to Settings.

---

## Key files to read before starting any step

| File | Why |
|------|-----|
| `src/settings/mod.rs:142-500` | Exact window-construction pattern to mirror |
| `src/settings/ui_support.rs:1298-1713` | `recent-captures-*` CSS vocabulary to build the gallery from |
| `src/settings/windowing.rs` | `install_window_drag`, `install_edge_resize`, `SETTINGS_WINDOW_MIN_*` |
| `src/settings/cloud.rs` | Worker-thread + `mpsc` + `glib::idle_add_local` delivery pattern; `spawn_apexshot_login` |
| `src/cloud/listing.rs` | `UploadsPager`, `CloudUpload`, `CloudReadError`, `cached_thumbnail` |
| `src/tray/mod.rs:10-35, 277-390` | Tray action enum and idle menu construction |
| `src/daemon/mod.rs:49-104, 737-930, 1164-1193` | Action enum, action loop, D-Bus string map |
| `src/main.rs:120-150, 363-375` | GTK dispatch block and async subcommand arms |
