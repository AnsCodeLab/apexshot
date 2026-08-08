//! Canvas motion / leave: cursor, eyedropper loupe, hover, text-handle resize (PR 10.18).
//!
//! Owns `EventControllerMotion` motion + leave on the drawing area. Click and
//! keyboard live in sibling modules.

use gtk4::{prelude::*, ApplicationWindow, DrawingArea, EventControllerMotion};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::capture::editor::{
    state::EditorState,
    types::{MoveHandle, Point, Tool, ViewTransform},
};

use super::super::{
    canvas::eyedropper_loupe_position,
    cursor::{cursor_name_for_view_point, set_window_cursor_name},
};
use super::{MOVE_HANDLE_DRAG_RADIUS, RESIZE_HANDLE_DRAG_SIZE};

/// Wire canvas motion and leave controllers (cursor, loupe, hover, text-handle drag).
pub(super) fn wire_canvas_motion(
    window: &ApplicationWindow,
    state: &Arc<Mutex<EditorState>>,
    transform: &Arc<Mutex<ViewTransform>>,
    drawing_area: &DrawingArea,
    space_pan_active: &Rc<Cell<bool>>,
    space_pan_dragging: &Rc<Cell<bool>>,
    eyedropper_mode: &Rc<Cell<bool>>,
    eyedropper_point: &Rc<RefCell<Option<Point>>>,
    canvas_eyedropper_ring: &DrawingArea,
) {
    let motion = EventControllerMotion::new();
    let eyedropper_mode_motion = eyedropper_mode.clone();
    let eyedropper_point_motion = eyedropper_point.clone();
    let canvas_eyedropper_ring_motion = canvas_eyedropper_ring.clone();
    let state_motion = state.clone();
    let transform_motion = transform.clone();
    let window_motion = window.downgrade();
    let drawing_area_motion = drawing_area.downgrade();
    let space_pan_active_motion = space_pan_active.clone();
    let space_pan_dragging_motion = space_pan_dragging.clone();
    motion.connect_motion(move |_, x, y| {
        let t = *transform_motion.lock().unwrap();
        let view_point = Point { x, y };

        if space_pan_active_motion.get() {
            if let Some(window) = window_motion.upgrade() {
                set_window_cursor_name(
                    &window,
                    Some(if space_pan_dragging_motion.get() {
                        "grabbing"
                    } else {
                        "grab"
                    }),
                );
            }
            return;
        }

        if eyedropper_mode_motion.get() {
            if !t.contains_view(view_point) {
                *eyedropper_point_motion.borrow_mut() = None;
                canvas_eyedropper_ring_motion.set_visible(false);
                if let Some(window) = window_motion.upgrade() {
                    set_window_cursor_name(&window, Some("crosshair"));
                }
                return;
            }

            *eyedropper_point_motion.borrow_mut() = Some(t.view_to_image_clamped(view_point));
            canvas_eyedropper_ring_motion.set_visible(true);
            let (left, top) = eyedropper_loupe_position(x, y);
            canvas_eyedropper_ring_motion.set_margin_start(left);
            canvas_eyedropper_ring_motion.set_margin_top(top);
            canvas_eyedropper_ring_motion.queue_draw();

            if let Some(window) = window_motion.upgrade() {
                set_window_cursor_name(&window, Some("none"));
            }
            return;
        }

        let is_highlighter = {
            let st = state_motion.lock().unwrap();
            st.selected_tool == Tool::Highlighter
        };

        let is_pen = {
            let st = state_motion.lock().unwrap();
            st.selected_tool == Tool::Pen
        };

        if is_highlighter {
            if let Some(window) = window_motion.upgrade() {
                if !t.contains_view(view_point) {
                    set_window_cursor_name(&window, Some("pointer"));
                } else {
                    let st = state_motion.lock().unwrap();
                    let image_point = t.view_to_image_clamped(view_point);
                    super::super::cursor::update_cursor_for_position(&window, &st, image_point, t.scale);
                }
            }
        } else if is_pen {
            if let Some(window) = window_motion.upgrade() {
                if !t.contains_view(view_point) {
                    set_window_cursor_name(&window, Some("pointer"));
                } else {
                    let st = state_motion.lock().unwrap();
                    super::super::cursor::update_pen_cursor(&window, &st);
                }
            }
        } else {
            let cursor_name = {
                let st = state_motion.lock().unwrap();
                cursor_name_for_view_point(&st, t, view_point)
            };

            if let Some(window) = window_motion.upgrade() {
                set_window_cursor_name(&window, Some(cursor_name));
            }
        }

        // In Text tool mode: detect hover over existing text actions.
        // Show outline border on hover and change cursor to "grab".
        {
            let mut st = state_motion.lock().unwrap();
            if st.selected_tool == Tool::Text && st.active_text_input.is_none() {
                let image_point = t.view_to_image_clamped(view_point);
                let hit = st
                    .actions
                    .iter()
                    .enumerate()
                    .rev()
                    .find_map(|(index, action)| {
                        if matches!(action, crate::capture::editor::types::AnnotationAction::Text { .. })
                            && crate::capture::editor::selection::action_contains_point_with_padding(
                                action,
                                image_point,
                                0.0,
                            )
                        {
                            Some(index)
                        } else {
                            None
                        }
                    });
                if st.hovered_text_action_index != hit {
                    st.hovered_text_action_index = hit;
                    if let Some(area) = drawing_area_motion.upgrade() {
                        area.queue_draw();
                    }
                }
                if hit.is_some() {
                    if let Some(window) = window_motion.upgrade() {
                        set_window_cursor_name(&window, Some("grab"));
                    }
                }
            } else if st.selected_tool != Tool::Text && st.hovered_text_action_index.is_some() {
                st.hovered_text_action_index = None;
                if let Some(area) = drawing_area_motion.upgrade() {
                    area.queue_draw();
                }
            }
        }

        // Check for text edit handle hover
        let text_bounds = state_motion.lock().unwrap().active_text_bounds.clone();
        if let Some(bounds) = &text_bounds {
            let t = *transform_motion.lock().unwrap();
            let view_point = Point { x, y };
            let _image_point = t.view_to_image(view_point);

            // Check move handles (convert to view coordinates)
            for (_handle, center) in &bounds.move_handles {
                let center_view = Point {
                    x: center.x * t.scale + t.offset_x,
                    y: center.y * t.scale + t.offset_y,
                };
                let dx = x - center_view.x;
                let dy = y - center_view.y;
                if (dx * dx + dy * dy).sqrt() < MOVE_HANDLE_DRAG_RADIUS {
                    if let Some(window) = window_motion.upgrade() {
                        set_window_cursor_name(&window, Some("grab"));
                    }
                    return;
                }
            }

            // Check resize handle
            if let Some((_, resize_pos)) = &bounds.resize_handle {
                let resize_view = Point {
                    x: resize_pos.x * t.scale + t.offset_x,
                    y: resize_pos.y * t.scale + t.offset_y,
                };
                let dx = x - resize_view.x;
                let dy = y - resize_view.y;
                if dx.abs() < RESIZE_HANDLE_DRAG_SIZE && dy.abs() < RESIZE_HANDLE_DRAG_SIZE {
                    if let Some(window) = window_motion.upgrade() {
                        set_window_cursor_name(&window, Some("nwse-resize"));
                    }
                    return;
                }
            }
        }

        let drag_state = {
            let st = state_motion.lock().unwrap();
            if st.active_text_is_dragging {
                st.active_text_drag_start.map(|start| {
                    (
                        start,
                        st.active_text_drag_handle.clone(),
                        st.active_text_drag_start_bounds,
                        st.active_text_is_resizing,
                        st.base_image.width() as i32,
                        st.base_image.height() as i32,
                    )
                })
            } else {
                None
            }
        };
        if let Some((start_point, handle, start_bounds, is_resizing, image_width, image_height)) =
            drag_state
        {
            let view_point = Point { x, y };
            let current_point = t.view_to_image(view_point);
            let dx = current_point.x - start_point.x;
            let dy = current_point.y - start_point.y;

            {
                let mut st = state_motion.lock().unwrap();
                // Compute min_width before the mutable borrow of active_text_bounds.
                let min_width = if st.active_text_input.is_none() && !is_resizing {
                    st.committed_text_min_width()
                } else {
                    50.0
                };
                if let (Some(bounds), Some(start_bounds)) =
                    (st.active_text_bounds.as_mut(), start_bounds)
                {
                    let min_height = 44.0;
                    if is_resizing {
                        let max_width = (image_width - start_bounds.x).max(min_width as i32) as f64;
                        let max_height =
                            (image_height - start_bounds.y).max(min_height as i32) as f64;
                        bounds.rect.x = start_bounds.x;
                        bounds.rect.y = start_bounds.y;
                        bounds.rect.width = ((start_bounds.width as f64 + dx)
                            .clamp(min_width, max_width))
                        .round() as i32;
                        bounds.rect.height = ((start_bounds.height as f64 + dy)
                            .clamp(min_height, max_height))
                        .round() as i32;
                    } else {
                        match handle {
                            Some(MoveHandle::Left) => {
                                // Mirror the Right handle exactly:
                                // right edge is fixed, x moves with dx, width = right - x.
                                let right = start_bounds.x + start_bounds.width;
                                let proposed_x = start_bounds.x + dx.round() as i32;
                                // x can't go below 0 or past (right - min_width)
                                let new_x = proposed_x.clamp(0, (right - min_width as i32).max(0));
                                bounds.rect.x = new_x;
                                bounds.rect.width = (right - new_x).max(min_width as i32);
                                bounds.rect.y = start_bounds.y;
                                bounds.rect.height = start_bounds.height;
                            }
                            Some(MoveHandle::Right) => {
                                let max_width =
                                    (image_width - start_bounds.x).max(min_width as i32) as f64;
                                bounds.rect.x = start_bounds.x;
                                bounds.rect.y = start_bounds.y;
                                bounds.rect.height = start_bounds.height;
                                bounds.rect.width = ((start_bounds.width as f64 + dx)
                                    .clamp(min_width, max_width))
                                .round() as i32;
                            }
                            None => {}
                        }
                    }
                    bounds.rect.x = bounds
                        .rect
                        .x
                        .clamp(0, (image_width - bounds.rect.width).max(0));
                    bounds.rect.y = bounds
                        .rect
                        .y
                        .clamp(0, (image_height - bounds.rect.height).max(0));
                    bounds.sync_handles();
                }
                if st.active_text_input.is_some() {
                    if is_resizing {
                        st.fit_active_text_to_layout_preserving_box();
                    } else {
                        st.fit_active_text_height_only();
                    }
                } else if !is_resizing {
                    // Committed action circle-handle resize: reflow height so
                    // text never overflows the bottom of the box.
                    st.fit_committed_text_height_only();
                }
                // Keep the original drag anchor fixed while using drag-start bounds.
            }

            if let Some(area) = drawing_area_motion.upgrade() {
                area.queue_draw();
            }
        }
    });

    let eyedropper_mode_motion_leave = eyedropper_mode.clone();
    let eyedropper_point_motion_leave = eyedropper_point.clone();
    let canvas_eyedropper_ring_motion_leave = canvas_eyedropper_ring.clone();
    let window_motion_leave = window.downgrade();
    motion.connect_leave(move |_| {
        *eyedropper_point_motion_leave.borrow_mut() = None;
        canvas_eyedropper_ring_motion_leave.set_visible(false);

        if let Some(window) = window_motion_leave.upgrade() {
            if eyedropper_mode_motion_leave.get() {
                set_window_cursor_name(&window, Some("crosshair"));
            } else {
                set_window_cursor_name(&window, None);
            }
        }
    });

    drawing_area.add_controller(motion);
}

#[cfg(test)]
mod tests {
    #[test]
    fn canvas_motion_owns_cursor_loupe_hover_and_text_handles() {
        let source = include_str!("motion.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        assert!(
            production.contains("pub(super) fn wire_canvas_motion(")
                && production.contains("let motion = EventControllerMotion::new();")
                && production.contains("motion.connect_motion(move |_, x, y| {")
                && production.contains("motion.connect_leave(move |_| {")
                && production.contains("drawing_area.add_controller(motion);")
                && production.contains("eyedropper_loupe_position")
                && production.contains("cursor_name_for_view_point")
                && production.contains("MOVE_HANDLE_DRAG_RADIUS")
                && production.contains("active_text_is_dragging"),
            "motion.rs must own motion/leave, loupe, cursor hover, and text-handle resize"
        );
    }
}
