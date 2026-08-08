//! Shared interaction-state owners for editor canvas handlers (PR 10.15).
//!
//! Bundles the Space-pan and eyedropper `Rc` cells so drag/click/motion/keyboard
//! peels can clone one owner instead of threading loose handles. Zoom keyboard
//! shortcuts reuse `wire_zoom_controls`'s `apply_zoom_change` callback (including
//! Ctrl+2 → 1.5×).

use gtk4::DrawingArea;
use image::RgbaImage;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::capture::editor::types::Point;

/// Space-held pan mode: active while Space is down, dragging while primary is held.
#[derive(Clone)]
pub(super) struct SpacePanState {
    pub active: Rc<Cell<bool>>,
    pub dragging: Rc<Cell<bool>>,
    pub origin: Rc<Cell<(f64, f64)>>,
}

impl SpacePanState {
    pub(super) fn new() -> Self {
        Self {
            active: Rc::new(Cell::new(false)),
            dragging: Rc::new(Cell::new(false)),
            origin: Rc::new(Cell::new((0.0, 0.0))),
        }
    }
}

/// Eyedropper sampling mode shared by setup activation and canvas handlers.
///
/// Constructed in `window` setup and passed through `EventContext`, so the type
/// and constructor are visible to the parent `window` module (not only `events`).
#[derive(Clone)]
pub(in super::super) struct EyedropperBundle {
    pub mode: Rc<Cell<bool>>,
    pub from_sidebar: Rc<Cell<bool>>,
    pub point: Rc<RefCell<Option<Point>>>,
    pub rendered: Rc<RefCell<Option<RgbaImage>>>,
    pub ring: DrawingArea,
}

impl EyedropperBundle {
    pub(in super::super) fn new(ring: DrawingArea) -> Self {
        Self {
            mode: Rc::new(Cell::new(false)),
            from_sidebar: Rc::new(Cell::new(false)),
            point: Rc::new(RefCell::new(None)),
            rendered: Rc::new(RefCell::new(None)),
            ring,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn interaction_state_owns_space_pan_and_eyedropper_bundles() {
        let source = include_str!("interaction.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        assert!(
            production.contains("pub(super) struct SpacePanState")
                && production.contains("pub active: Rc<Cell<bool>>")
                && production.contains("pub dragging: Rc<Cell<bool>>")
                && production.contains("pub origin: Rc<Cell<(f64, f64)>>")
                && production.contains("pub(in super::super) struct EyedropperBundle")
                && production.contains("pub mode: Rc<Cell<bool>>")
                && production.contains("pub from_sidebar: Rc<Cell<bool>>")
                && production.contains("pub point: Rc<RefCell<Option<Point>>>")
                && production.contains("pub rendered: Rc<RefCell<Option<RgbaImage>>>")
                && production.contains("pub ring: DrawingArea")
                && production.contains("fn new(ring: DrawingArea)"),
            "interaction.rs must own SpacePanState and EyedropperBundle with their Rc cells"
        );
    }
}
