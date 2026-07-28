//! ApexShot History — a browser for past captures and cloud uploads.
//!
//! Status: partially built. The GUI-free foundations are in place and tested:
//!
//! * [`scan`] lists captures from the configured screenshot and video folders.
//! * [`thumbnails`] renders grid thumbnails on background threads with an
//!   on-disk cache.
//! * [`actions`] performs the per-item actions a card offers, reusing the
//!   app's existing open / clipboard / editor / upload plumbing.
//!
//! Still to come: the GTK window shell itself (sidebar + page stack styled as a
//! sibling of Settings), the local grids, the Cloud page, and the `history`
//! command and tray entry that open the window. See
//! `docs/HISTORY_WINDOW_TODO.md` for the remaining work.

pub mod actions;
pub mod scan;
pub mod thumbnails;
