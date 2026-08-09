//! Overlay pointer/keyboard input owners (drag, motion, click, keyboard).

mod click;
mod drag;
mod keyboard;
mod motion;

pub(in crate::overlay::window) use click::wire_window_click;
pub(in crate::overlay::window) use drag::wire_selection_drag;
pub(in crate::overlay::window) use keyboard::wire_window_keyboard;
pub(in crate::overlay::window) use motion::wire_selection_motion;
