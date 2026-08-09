use super::api::SelectionArea;
use super::background::BackgroundFrame;
use super::layout::{BORDER_HANDLE_THRESHOLD, MIN_SELECTION_HEIGHT, MIN_SELECTION_WIDTH};
use super::state::{DragMode, ResizeHandle, SelectorState};

#[derive(Debug, Clone, Copy)]
pub(crate) struct SelectionRectF {
    pub(crate) left: f64,
    pub(crate) top: f64,
    pub(crate) right: f64,
    pub(crate) bottom: f64,
}

impl SelectionRectF {
    pub(crate) fn from_points(x0: f64, y0: f64, x1: f64, y1: f64) -> Self {
        Self {
            left: x0.min(x1),
            top: y0.min(y1),
            right: x0.max(x1),
            bottom: y0.max(y1),
        }
    }

    pub(crate) fn width(&self) -> f64 {
        self.right - self.left
    }

    pub(crate) fn height(&self) -> f64 {
        self.bottom - self.top
    }
}

pub(crate) fn current_selection_rect(state: &SelectorState) -> SelectionRectF {
    SelectionRectF::from_points(
        state.start_x,
        state.start_y,
        state.current_x,
        state.current_y,
    )
}

pub(crate) fn set_selection_rect(state: &mut SelectorState, rect: SelectionRectF) {
    state.start_x = rect.left;
    state.start_y = rect.top;
    state.current_x = rect.right;
    state.current_y = rect.bottom;
}

pub(crate) fn clamp_point_to_bounds(
    x: f64,
    y: f64,
    bounds_width: f64,
    bounds_height: f64,
) -> (f64, f64) {
    (
        x.clamp(0.0, bounds_width.max(1.0)),
        y.clamp(0.0, bounds_height.max(1.0)),
    )
}

pub(crate) fn detect_resize_handle(x: f64, y: f64, rect: SelectionRectF) -> Option<ResizeHandle> {
    let left = rect.left;
    let right = rect.right;
    let top = rect.top;
    let bottom = rect.bottom;

    let near_left = (x - left).abs() <= BORDER_HANDLE_THRESHOLD
        && y >= top - BORDER_HANDLE_THRESHOLD
        && y <= bottom + BORDER_HANDLE_THRESHOLD;
    let near_right = (x - right).abs() <= BORDER_HANDLE_THRESHOLD
        && y >= top - BORDER_HANDLE_THRESHOLD
        && y <= bottom + BORDER_HANDLE_THRESHOLD;
    let near_top = (y - top).abs() <= BORDER_HANDLE_THRESHOLD
        && x >= left - BORDER_HANDLE_THRESHOLD
        && x <= right + BORDER_HANDLE_THRESHOLD;
    let near_bottom = (y - bottom).abs() <= BORDER_HANDLE_THRESHOLD
        && x >= left - BORDER_HANDLE_THRESHOLD
        && x <= right + BORDER_HANDLE_THRESHOLD;

    if near_left && near_top {
        return Some(ResizeHandle::NorthWest);
    }
    if near_right && near_top {
        return Some(ResizeHandle::NorthEast);
    }
    if near_left && near_bottom {
        return Some(ResizeHandle::SouthWest);
    }
    if near_right && near_bottom {
        return Some(ResizeHandle::SouthEast);
    }

    if near_top {
        return Some(ResizeHandle::North);
    }
    if near_bottom {
        return Some(ResizeHandle::South);
    }
    if near_left {
        return Some(ResizeHandle::West);
    }
    if near_right {
        return Some(ResizeHandle::East);
    }

    None
}

/// Returns `true` when `(x, y)` is strictly inside the selection rectangle,
/// far enough from every edge that it is not on a resize handle.
/// This is used to decide whether a drag should move the whole rect.
pub(crate) fn is_inside_selection(x: f64, y: f64, rect: SelectionRectF) -> bool {
    x > rect.left + BORDER_HANDLE_THRESHOLD
        && x < rect.right - BORDER_HANDLE_THRESHOLD
        && y > rect.top + BORDER_HANDLE_THRESHOLD
        && y < rect.bottom - BORDER_HANDLE_THRESHOLD
}

pub(crate) fn cursor_name_for_handle(handle: ResizeHandle) -> &'static str {
    match handle {
        ResizeHandle::North | ResizeHandle::South => "ns-resize",
        ResizeHandle::East | ResizeHandle::West => "ew-resize",
        ResizeHandle::NorthEast | ResizeHandle::SouthWest => "nesw-resize",
        ResizeHandle::NorthWest | ResizeHandle::SouthEast => "nwse-resize",
    }
}

pub(crate) fn resize_rect_from_handle(
    initial: SelectionRectF,
    handle: ResizeHandle,
    pointer_x: f64,
    pointer_y: f64,
    bounds_width: f64,
    bounds_height: f64,
) -> SelectionRectF {
    let mut left = initial.left;
    let mut top = initial.top;
    let mut right = initial.right;
    let mut bottom = initial.bottom;

    let move_left = matches!(
        handle,
        ResizeHandle::West | ResizeHandle::NorthWest | ResizeHandle::SouthWest
    );
    let move_right = matches!(
        handle,
        ResizeHandle::East | ResizeHandle::NorthEast | ResizeHandle::SouthEast
    );
    let move_top = matches!(
        handle,
        ResizeHandle::North | ResizeHandle::NorthWest | ResizeHandle::NorthEast
    );
    let move_bottom = matches!(
        handle,
        ResizeHandle::South | ResizeHandle::SouthWest | ResizeHandle::SouthEast
    );

    if move_left {
        left = pointer_x;
    }
    if move_right {
        right = pointer_x;
    }
    if move_top {
        top = pointer_y;
    }
    if move_bottom {
        bottom = pointer_y;
    }

    let min_width = MIN_SELECTION_WIDTH.min(bounds_width.max(1.0));
    let min_height = MIN_SELECTION_HEIGHT.min(bounds_height.max(1.0));

    if (right - left) < min_width {
        if move_left {
            left = right - min_width;
        } else {
            right = left + min_width;
        }
    }

    if (bottom - top) < min_height {
        if move_top {
            top = bottom - min_height;
        } else {
            bottom = top + min_height;
        }
    }

    left = left.clamp(0.0, (bounds_width - min_width).max(0.0));
    top = top.clamp(0.0, (bounds_height - min_height).max(0.0));
    right = right.clamp(min_width, bounds_width.max(min_width));
    bottom = bottom.clamp(min_height, bounds_height.max(min_height));

    if (right - left) < min_width {
        if move_left {
            left = (right - min_width).max(0.0);
        } else {
            right = (left + min_width).min(bounds_width.max(min_width));
        }
    }

    if (bottom - top) < min_height {
        if move_top {
            top = (bottom - min_height).max(0.0);
        } else {
            bottom = (top + min_height).min(bounds_height.max(min_height));
        }
    }

    SelectionRectF {
        left,
        top,
        right,
        bottom,
    }
}

pub(crate) fn update_selection_for_drag(
    state: &mut SelectorState,
    drag_offset_x: f64,
    drag_offset_y: f64,
    bounds_width: f64,
    bounds_height: f64,
) {
    match state.drag_mode {
        Some(DragMode::NewSelection) => {
            let (next_x, next_y) = clamp_point_to_bounds(
                state.drag_origin_x + drag_offset_x,
                state.drag_origin_y + drag_offset_y,
                bounds_width,
                bounds_height,
            );
            state.current_x = next_x;
            state.current_y = next_y;
        }
        Some(DragMode::Move) => {
            if let Some(initial_rect) = state.initial_rect {
                let w = initial_rect.width();
                let h = initial_rect.height();
                // Translate the whole rect by the drag delta, keeping it
                // fully within the screen bounds.
                let new_left =
                    (initial_rect.left + drag_offset_x).clamp(0.0, (bounds_width - w).max(0.0));
                let new_top =
                    (initial_rect.top + drag_offset_y).clamp(0.0, (bounds_height - h).max(0.0));
                set_selection_rect(
                    state,
                    SelectionRectF {
                        left: new_left,
                        top: new_top,
                        right: new_left + w,
                        bottom: new_top + h,
                    },
                );
                state.completed = true;
            }
        }
        Some(DragMode::Resize(handle)) => {
            if let Some(initial_rect) = state.initial_rect {
                let (pointer_x, pointer_y) = clamp_point_to_bounds(
                    state.drag_origin_x + drag_offset_x,
                    state.drag_origin_y + drag_offset_y,
                    bounds_width,
                    bounds_height,
                );
                let resized = resize_rect_from_handle(
                    initial_rect,
                    handle,
                    pointer_x,
                    pointer_y,
                    bounds_width,
                    bounds_height,
                );
                set_selection_rect(state, resized);
                state.completed = true;
            }
        }
        None => {}
    }
}

pub(crate) fn selection_area_from_state(
    state: &SelectorState,
    screen_width: i32,
    screen_height: i32,
    background: Option<&BackgroundFrame>,
) -> SelectionArea {
    if state.fullscreen_mode {
        let mut full = SelectionArea {
            x: 0,
            y: 0,
            width: screen_width,
            height: screen_height,
        };
        if let Some(background) = background {
            full = map_selection_to_image(
                full,
                background.width,
                background.height,
                screen_width,
                screen_height,
            );
        }
        return full;
    }

    let rect = current_selection_rect(state);
    let area = SelectionArea {
        x: rect.left.floor() as i32,
        y: rect.top.floor() as i32,
        width: rect.width().round() as i32,
        height: rect.height().round() as i32,
    };
    if let Some(background) = background {
        map_selection_to_image(
            area,
            background.width,
            background.height,
            screen_width,
            screen_height,
        )
    } else {
        area
    }
}

pub(crate) fn map_selection_to_image(
    area: SelectionArea,
    image_width: i32,
    image_height: i32,
    view_width: i32,
    view_height: i32,
) -> SelectionArea {
    if image_width <= 0 || image_height <= 0 || view_width <= 0 || view_height <= 0 {
        return area;
    }

    let scale_x = image_width as f64 / view_width as f64;
    let scale_y = image_height as f64 / view_height as f64;

    let x0 = (area.x as f64 * scale_x).floor() as i32;
    let y0 = (area.y as f64 * scale_y).floor() as i32;
    let x1 = ((area.x + area.width) as f64 * scale_x).ceil() as i32;
    let y1 = ((area.y + area.height) as f64 * scale_y).ceil() as i32;

    let clamped_x0 = x0.clamp(0, image_width.saturating_sub(1));
    let clamped_y0 = y0.clamp(0, image_height.saturating_sub(1));
    let clamped_x1 = x1.clamp(clamped_x0 + 1, image_width);
    let clamped_y1 = y1.clamp(clamped_y0 + 1, image_height);

    SelectionArea {
        x: clamped_x0,
        y: clamped_y0,
        width: clamped_x1 - clamped_x0,
        height: clamped_y1 - clamped_y0,
    }
}

/// Aspect ratio values matching `layout::ASPECT_RATIO_OPTIONS` order.
/// Index 0 is freeform (`0.0`).
pub(crate) fn aspect_ratio_for_index(index: usize) -> f64 {
    const RATIOS: &[f64] = &[
        0.0,
        1.0,
        5.0 / 4.0,
        4.0 / 3.0,
        7.0 / 5.0,
        3.0 / 2.0,
        16.0 / 10.0,
        16.0 / 9.0,
        2.35,
        2.0 / 3.0,
        9.0 / 16.0,
    ];
    RATIOS.get(index).copied().unwrap_or(0.0)
}

pub(crate) fn active_aspect_ratio(st: &SelectorState) -> f64 {
    if st.recording.panel_open {
        aspect_ratio_for_index(st.recording.record_aspect_ratio_index)
    } else {
        aspect_ratio_for_index(st.capture_aspect_ratio_index)
    }
}

/// Fit the current selection to `ratio` while keeping its centre, clamping to
/// bounds and enforcing minimum size. Freeform (`ratio <= 0`) is a no-op.
pub(crate) fn apply_aspect_to_selection(
    st: &mut SelectorState,
    ratio: f64,
    bounds_width: f64,
    bounds_height: f64,
) {
    if ratio <= 0.0 || !st.completed {
        return;
    }

    let sel = current_selection_rect(st);
    let mut new_w = sel.width();
    let mut new_h = new_w / ratio;
    if new_h > sel.height() {
        new_h = sel.height();
        new_w = new_h * ratio;
    }

    new_w = new_w.clamp(MIN_SELECTION_WIDTH, bounds_width.max(MIN_SELECTION_WIDTH));
    new_h = new_h.clamp(
        MIN_SELECTION_HEIGHT,
        bounds_height.max(MIN_SELECTION_HEIGHT),
    );
    if new_w / ratio > bounds_height {
        new_h = bounds_height;
        new_w = new_h * ratio;
    }
    if new_h * ratio > bounds_width {
        new_w = bounds_width;
        new_h = new_w / ratio;
    }

    let center_x = (sel.left + sel.right) / 2.0;
    let center_y = (sel.top + sel.bottom) / 2.0;
    let width = new_w.max(MIN_SELECTION_WIDTH).round();
    let height = new_h.max(MIN_SELECTION_HEIGHT).round();
    let left = (center_x - width / 2.0).clamp(0.0, (bounds_width - width).max(0.0));
    let top = (center_y - height / 2.0).clamp(0.0, (bounds_height - height).max(0.0));

    set_selection_rect(
        st,
        SelectionRectF {
            left,
            top,
            right: left + width,
            bottom: top + height,
        },
    );
    st.completed = true;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overlay::layout::ASPECT_RATIO_OPTIONS;
    use crate::overlay::state::SelectorState;

    fn completed_selection(left: f64, top: f64, right: f64, bottom: f64) -> SelectorState {
        SelectorState {
            start_x: left,
            start_y: top,
            current_x: right,
            current_y: bottom,
            completed: true,
            ..Default::default()
        }
    }

    #[test]
    fn aspect_ratio_index_zero_is_freeform() {
        assert_eq!(aspect_ratio_for_index(0), 0.0);
        assert_eq!(ASPECT_RATIO_OPTIONS[0], "Freeform");
    }

    #[test]
    fn freeform_aspect_is_noop() {
        let mut st = completed_selection(100.0, 100.0, 300.0, 250.0);
        apply_aspect_to_selection(&mut st, 0.0, 1920.0, 1080.0);
        let rect = current_selection_rect(&st);
        assert!((rect.width() - 200.0).abs() < 1e-9);
        assert!((rect.height() - 150.0).abs() < 1e-9);
    }

    #[test]
    fn fixed_ratio_square_fits_inside_selection_and_keeps_center() {
        let mut st = completed_selection(100.0, 100.0, 300.0, 220.0); // 200×120
        let before = current_selection_rect(&st);
        let cx = (before.left + before.right) / 2.0;
        let cy = (before.top + before.bottom) / 2.0;
        apply_aspect_to_selection(&mut st, 1.0, 1920.0, 1080.0);
        let rect = current_selection_rect(&st);
        assert!((rect.width() - rect.height()).abs() < 1.0);
        // Fits inside original extents.
        assert!(rect.width() <= 200.0 + 1e-6);
        assert!(rect.height() <= 120.0 + 1e-6);
        let ncx = (rect.left + rect.right) / 2.0;
        let ncy = (rect.top + rect.bottom) / 2.0;
        assert!((ncx - cx).abs() < 1.0, "center x drifted: {ncx} vs {cx}");
        assert!((ncy - cy).abs() < 1.0, "center y drifted: {ncy} vs {cy}");
    }

    #[test]
    fn fixed_ratio_clamps_to_screen_edges() {
        // Selection near the right edge; applying a wide ratio must stay on-screen.
        let mut st = completed_selection(1800.0, 100.0, 1910.0, 400.0);
        apply_aspect_to_selection(&mut st, 16.0 / 9.0, 1920.0, 1080.0);
        let rect = current_selection_rect(&st);
        assert!(rect.left >= 0.0);
        assert!(rect.top >= 0.0);
        assert!(rect.right <= 1920.0 + 1e-6);
        assert!(rect.bottom <= 1080.0 + 1e-6);
        assert!(rect.width() >= MIN_SELECTION_WIDTH - 1e-6);
        assert!(rect.height() >= MIN_SELECTION_HEIGHT - 1e-6);
    }

    #[test]
    fn fixed_ratio_enforces_minimum_size() {
        // Smaller than mins; apply still enforces minimum width/height.
        let mut st = completed_selection(10.0, 10.0, 20.0, 18.0);
        apply_aspect_to_selection(&mut st, 1.0, 1920.0, 1080.0);
        let rect = current_selection_rect(&st);
        assert!(rect.width() + 1e-6 >= MIN_SELECTION_WIDTH);
        assert!(rect.height() + 1e-6 >= MIN_SELECTION_HEIGHT);
    }

    #[test]
    fn active_aspect_ratio_prefers_recording_index_when_panel_open() {
        let mut st = SelectorState {
            capture_aspect_ratio_index: 1, // 1:1
            ..Default::default()
        };
        st.recording.record_aspect_ratio_index = 7; // 16:9
        st.recording.panel_open = false;
        assert!((active_aspect_ratio(&st) - 1.0).abs() < 1e-9);
        st.recording.panel_open = true;
        assert!((active_aspect_ratio(&st) - 16.0 / 9.0).abs() < 1e-9);
    }
}
