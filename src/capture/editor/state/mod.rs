//! Editor session state and annotation mutation API.
//!
//! Behavior is split across child modules; the struct and shared helpers stay here
//! so private field access remains inside the `state` module tree.

mod arrow;
mod crop;
mod drag_draw;
mod export;
mod history;
mod text_input;

use super::color::{
    clamp_focus_intensity, clamp_obfuscate_amount, clamp_pixelate_amount, clamp_stroke_size,
    selection_handle_hit_radius_for_scale, selection_hit_padding_for_scale, DEFAULT_COLOR_INDEX,
    DEFAULT_FOCUS_INTENSITY, DEFAULT_OBFUSCATE_AMOUNT, DRAW_COLORS, STROKE_WIDTH, TEXT_SIZE,
};
use super::numbering_style::{NumberSize, NumberingStyle};
use super::pen_weight::{HighlighterMode, PenWeight};
use super::render::{
    apply_blackout_rect, apply_censor_rect, apply_focus_rect, apply_hybrid_blur,
    cairo_argb_to_rgba_image, rgba_image_to_surface,
};
use super::selection::{
    action_contains_point_with_padding, action_resize_handle_at_point_with_radius, resize_action,
    translate_action,
};
use super::text_detect::{BackgroundTextDetection, TextDetector};
use super::types::{
    AnnotationAction, ArrowStyle, BackgroundAlignment, BackgroundStyle, CropAspectRatio, DrawColor,
    EditorError, MoveHandle, ObfuscateMethod, Point, Rect, SizeControlMode, TextEditBounds, Tool,
};
use gtk4;
use image::RgbaImage;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

pub struct EditorState {
    pub base_image: Arc<RgbaImage>,
    pub working_image: Arc<RgbaImage>,
    pub working_image_revision: u64,
    pub crop_selection: Option<Rect>,
    pub crop_aspect_ratio: CropAspectRatio,
    pub crop_background_color: DrawColor,
    pub crop_background_color_explicit: bool,
    pub actions: Vec<AnnotationAction>,
    pub redo_actions: Vec<AnnotationAction>,
    pub selected_tool: Tool,
    pub selected_action_index: Option<usize>,
    pub selected_color: DrawColor,
    pub stroke_size: f64,
    pub smooth_drawing_enabled: bool,
    pub draw_object_shadow: bool,
    pub auto_expand_canvas: bool,
    pub inverse_arrow_direction: bool,
    pub text_size: f64,
    pub text_font_family: String,
    pub text_background_color: Option<DrawColor>,
    pub obfuscate_method: ObfuscateMethod,
    pub obfuscate_pixelate_amount: f64,
    pub obfuscate_blur_amount: f64,
    pub focus_intensity: f64,
    pub arrow_style: ArrowStyle,
    pub arrow_editing_controls: bool,
    pub arrow_control_dragging: Option<usize>,
    pub next_number: u32,
    pub select_drag_anchor: Option<Point>,
    pub select_resize_handle: Option<super::types::SelectHandle>,
    pub select_effect_rebuild_pending: bool,
    pub select_effect_rebuild_dirty: bool,
    pub select_drag_effect_dirty: bool,
    pub active_text_edit: Option<()>,
    pub active_text_entry: Option<gtk4::Entry>,
    pub active_text_bounds: Option<TextEditBounds>,
    pub active_text_is_dragging: bool,
    pub active_text_drag_handle: Option<MoveHandle>,
    pub active_text_drag_start: Option<Point>,
    pub pending_effect_revision: u64,
    pub last_applied_effect_revision: u64,
    pub last_effect_request_time_us: i64,
    pub drag_start: Option<Point>,
    pub drag_current: Option<Point>,
    pub drag_start_view: Option<Point>,
    pub drag_path: Vec<Point>,
    pub drag_shift_active: bool,
    pub background_style: BackgroundStyle,
    pub background_padding: f64,
    pub background_shadow: f64,
    pub background_insert: f64,
    pub auto_balance: bool,
    pub background_alignment: BackgroundAlignment,
    pub background_corner_radius: f64,
    pub background_aspect_ratio: CropAspectRatio,
    pub active_text_drag_start_bounds: Option<Rect>,
    pub active_text_is_resizing: bool,
    pub hovered_text_action_index: Option<usize>,
    pub active_text_input: Option<TextInputState>,

    // Text detection for highlighter
    pub text_detector: Arc<Mutex<TextDetector>>,
    pub text_detection_ready: Arc<AtomicBool>,
    pub text_detection_handle: Option<BackgroundTextDetection>,

    // Highlighter mode
    pub highlighter_mode: HighlighterMode,
    pub pen_weight: PenWeight,
    pub locked_highlighter_stroke_size: Option<f64>,

    // Number tool options
    pub numbering_style: NumberingStyle,
    pub numbering_start: u32,
    pub number_size: NumberSize,
}

#[derive(Debug, Clone)]
pub struct TextInputState {
    pub text: String,
    pub cursor_position: usize,
    pub cursor_visible: bool,
    pub cursor_blink_timer: u32,
    pub color: DrawColor,
    pub background_color: Option<DrawColor>,
    pub editing_action_index: Option<usize>,
}

pub(super) fn simplify_drag_path(points: &[Point], epsilon: f64) -> Vec<Point> {
    if points.len() <= 2 {
        return points.to_vec();
    }

    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    simplify_drag_path_range(points, 0, points.len() - 1, epsilon, &mut keep);

    points
        .iter()
        .zip(keep)
        .filter_map(|(point, keep)| keep.then_some(*point))
        .collect()
}

pub(super) fn simplify_drag_path_range(
    points: &[Point],
    start: usize,
    end: usize,
    epsilon: f64,
    keep: &mut [bool],
) {
    if end <= start + 1 {
        return;
    }

    let first = points[start];
    let last = points[end];
    let mut max_distance = 0.0;
    let mut max_index = None;

    for (index, point) in points.iter().enumerate().take(end).skip(start + 1) {
        let distance = perpendicular_distance(*point, first, last);
        if distance > max_distance {
            max_distance = distance;
            max_index = Some(index);
        }
    }

    if max_distance > epsilon {
        if let Some(index) = max_index {
            keep[index] = true;
            simplify_drag_path_range(points, start, index, epsilon, keep);
            simplify_drag_path_range(points, index, end, epsilon, keep);
        }
    }
}

pub(super) fn perpendicular_distance(point: Point, start: Point, end: Point) -> f64 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    if dx.abs() <= f64::EPSILON && dy.abs() <= f64::EPSILON {
        return ((point.x - start.x).powi(2) + (point.y - start.y).powi(2)).sqrt();
    }

    let numerator = ((dy * point.x) - (dx * point.y) + (end.x * start.y) - (end.y * start.x)).abs();
    let denominator = (dx * dx + dy * dy).sqrt();
    numerator / denominator
}

pub(super) fn expand_rgba_image(
    image: &RgbaImage,
    new_width: u32,
    new_height: u32,
    offset_x: u32,
    offset_y: u32,
) -> RgbaImage {
    if new_width == image.width() && new_height == image.height() && offset_x == 0 && offset_y == 0
    {
        return image.clone();
    }

    let mut expanded = RgbaImage::from_pixel(new_width, new_height, image::Rgba([0, 0, 0, 0]));
    image::imageops::overlay(&mut expanded, image, offset_x as i64, offset_y as i64);
    expanded
}

impl EditorState {
    pub fn new(base_image: RgbaImage) -> Self {
        let base_image = Arc::new(base_image);
        Self {
            working_image: Arc::clone(&base_image),
            base_image,
            working_image_revision: 1,
            crop_selection: None,
            crop_aspect_ratio: CropAspectRatio::Freeform,
            crop_background_color: DrawColor::new(1.0, 1.0, 1.0, 1.0),
            crop_background_color_explicit: false,
            actions: Vec::new(),
            redo_actions: Vec::new(),
            selected_tool: Tool::Background,
            selected_action_index: None,
            selected_color: DRAW_COLORS[DEFAULT_COLOR_INDEX],
            stroke_size: STROKE_WIDTH,
            smooth_drawing_enabled: false,
            draw_object_shadow: false,
            auto_expand_canvas: false,
            inverse_arrow_direction: false,
            text_size: TEXT_SIZE,
            text_font_family: String::from("Sans"),
            text_background_color: None,
            obfuscate_method: ObfuscateMethod::Pixelate,
            obfuscate_pixelate_amount: DEFAULT_OBFUSCATE_AMOUNT,
            obfuscate_blur_amount: DEFAULT_OBFUSCATE_AMOUNT,
            focus_intensity: DEFAULT_FOCUS_INTENSITY,
            arrow_style: ArrowStyle::Standard,
            arrow_editing_controls: false,
            arrow_control_dragging: None,
            next_number: 1,
            select_drag_anchor: None,
            select_resize_handle: None,
            select_effect_rebuild_pending: false,
            select_effect_rebuild_dirty: false,
            select_drag_effect_dirty: false,
            active_text_edit: None,
            active_text_entry: None,
            active_text_bounds: None,
            active_text_is_dragging: false,
            active_text_drag_handle: None,
            active_text_drag_start: None,
            pending_effect_revision: 0,
            last_applied_effect_revision: 0,
            last_effect_request_time_us: 0,
            drag_start: None,
            drag_current: None,
            drag_start_view: None,
            drag_path: Vec::new(),
            drag_shift_active: false,
            background_style: BackgroundStyle::None,
            background_padding: 24.0,
            background_shadow: 15.0,
            background_insert: 0.0,
            auto_balance: false,
            background_alignment: BackgroundAlignment::Center,
            background_corner_radius: 18.0,
            background_aspect_ratio: CropAspectRatio::Original,
            active_text_drag_start_bounds: None,
            active_text_is_resizing: false,
            hovered_text_action_index: None,
            active_text_input: None,

            text_detector: Arc::new(Mutex::new(TextDetector::new_pending())),
            text_detection_ready: Arc::new(AtomicBool::new(false)),
            text_detection_handle: None,
            highlighter_mode: HighlighterMode::default(),
            pen_weight: PenWeight::default(),
            locked_highlighter_stroke_size: None,
            numbering_style: NumberingStyle::default(),
            numbering_start: 1,
            number_size: NumberSize::default(),
        }
    }

    pub fn set_tool(&mut self, tool: Tool) -> bool {
        let rebuild = self.set_tool_without_rebuild(tool);
        if rebuild {
            self.rebuild_effect_layer();
        }
        rebuild
    }

    pub fn set_tool_without_rebuild(&mut self, tool: Tool) -> bool {
        if self.selected_tool == Tool::Crop && tool != Tool::Crop {
            self.crop_selection = None;
        }
        if tool != Tool::Select {
            self.selected_action_index = None;
            self.select_drag_anchor = None;
            self.select_resize_handle = None;
        }
        if tool != Tool::Text {
            self.cancel_text_input();
            self.hovered_text_action_index = None;
        }
        if tool != Tool::Arrow {
            self.finalize_arrow_control_editing();
        }
        self.selected_tool = tool;
        self.clear_drag_without_rebuild_and_check_effect()
    }

    pub fn set_color_index(&mut self, index: usize) {
        if let Some(color) = DRAW_COLORS.get(index).copied() {
            self.selected_color = color;
            if let Some(input) = self.active_text_input.as_mut() {
                input.color = color;
            }
        }
    }

    pub fn set_stroke_size(&mut self, size: f64) -> bool {
        let next = clamp_stroke_size(size);
        if (next - self.stroke_size).abs() <= f64::EPSILON {
            return false;
        }

        self.stroke_size = next;
        true
    }

    pub fn set_obfuscate_method(&mut self, method: ObfuscateMethod) {
        self.obfuscate_method = method;
    }

    pub fn obfuscate_method(&self) -> ObfuscateMethod {
        self.obfuscate_method
    }

    pub fn current_obfuscate_amount(&self) -> f64 {
        match self.obfuscate_method {
            ObfuscateMethod::Pixelate => self.obfuscate_pixelate_amount,
            ObfuscateMethod::Blur => self.obfuscate_blur_amount,
            ObfuscateMethod::Blackout => 0.0,
        }
    }

    pub fn set_current_obfuscate_amount(&mut self, amount: f64) {
        match self.obfuscate_method {
            ObfuscateMethod::Pixelate => {
                self.obfuscate_pixelate_amount = clamp_pixelate_amount(amount)
            }
            ObfuscateMethod::Blur => self.obfuscate_blur_amount = clamp_obfuscate_amount(amount),
            ObfuscateMethod::Blackout => {}
        }
    }

    /// Like set_current_obfuscate_amount but returns true if the value actually changed.
    pub fn set_current_obfuscate_amount_and_check(&mut self, amount: f64) -> bool {
        let before = self.current_obfuscate_amount();
        self.set_current_obfuscate_amount(amount);
        let after = self.current_obfuscate_amount();
        (after - before).abs() > f64::EPSILON
    }

    pub fn current_focus_intensity(&self) -> f64 {
        clamp_focus_intensity(self.focus_intensity)
    }

    pub fn set_current_focus_intensity_and_check(&mut self, intensity: f64) -> bool {
        let next = clamp_focus_intensity(intensity);
        if (self.focus_intensity - next).abs() <= f64::EPSILON {
            return false;
        }
        self.focus_intensity = next;
        true
    }

    pub fn selected_focus_action_intensity(&self) -> Option<f64> {
        let AnnotationAction::Focus { intensity, .. } = self.selected_action()? else {
            return None;
        };

        Some(*intensity)
    }

    pub fn set_selected_focus_action_intensity_without_rebuild(&mut self, intensity: f64) -> bool {
        let next = clamp_focus_intensity(intensity);

        let Some(index) = self.selected_action_index else {
            return false;
        };

        let Some(action) = self.actions.get_mut(index) else {
            self.selected_action_index = None;
            return false;
        };

        let AnnotationAction::Focus {
            intensity: act_intensity,
            ..
        } = action
        else {
            return false;
        };

        if (*act_intensity - next).abs() <= f64::EPSILON {
            return false;
        }

        *act_intensity = next;
        self.redo_actions.clear();
        true
    }

    pub fn selected_action_stroke_size(&self) -> Option<f64> {
        match self.selected_action()? {
            AnnotationAction::Pen { stroke_size, .. }
            | AnnotationAction::Highlighter { stroke_size, .. }
            | AnnotationAction::Circle { stroke_size, .. }
            | AnnotationAction::Line { stroke_size, .. }
            | AnnotationAction::Arrow { stroke_size, .. }
            | AnnotationAction::Box { stroke_size, .. } => Some(*stroke_size),
            AnnotationAction::Text { .. }
            | AnnotationAction::Number { .. }
            | AnnotationAction::Obfuscate { .. }
            | AnnotationAction::Focus { .. } => None,
        }
    }

    pub fn set_selected_action_stroke_size(&mut self, size: f64) -> bool {
        let next = clamp_stroke_size(size);

        let Some(index) = self.selected_action_index else {
            return false;
        };

        let Some(action) = self.actions.get_mut(index) else {
            self.selected_action_index = None;
            return false;
        };

        let target = match action {
            AnnotationAction::Pen { stroke_size, .. }
            | AnnotationAction::Highlighter { stroke_size, .. }
            | AnnotationAction::Circle { stroke_size, .. }
            | AnnotationAction::Line { stroke_size, .. }
            | AnnotationAction::Arrow { stroke_size, .. }
            | AnnotationAction::Box { stroke_size, .. } => stroke_size,
            AnnotationAction::Text { .. }
            | AnnotationAction::Number { .. }
            | AnnotationAction::Obfuscate { .. }
            | AnnotationAction::Focus { .. } => return false,
        };

        if (*target - next).abs() <= f64::EPSILON {
            return false;
        }

        *target = next;
        self.redo_actions.clear();
        true
    }

    pub fn selected_obfuscate_action_amount(&self) -> Option<f64> {
        let AnnotationAction::Obfuscate { amount, .. } = self.selected_action()? else {
            return None;
        };

        Some(*amount)
    }

    pub fn set_selected_obfuscate_action_amount_without_rebuild(&mut self, amount: f64) -> bool {
        let next = clamp_obfuscate_amount(amount);

        let Some(index) = self.selected_action_index else {
            return false;
        };

        let Some(action) = self.actions.get_mut(index) else {
            self.selected_action_index = None;
            return false;
        };

        let AnnotationAction::Obfuscate {
            amount: act_amount, ..
        } = action
        else {
            return false;
        };

        if (*act_amount - next).abs() <= f64::EPSILON {
            return false;
        }

        *act_amount = next;
        self.redo_actions.clear();
        true
    }

    pub fn active_size_control_mode(&self) -> Option<SizeControlMode> {
        if self.selected_tool == Tool::Select {
            if self.selected_action_stroke_size().is_some() {
                return Some(SizeControlMode::Stroke);
            }
            if self.selected_obfuscate_action_amount().is_some() {
                return Some(SizeControlMode::Obfuscate);
            }
            if self.selected_focus_action_intensity().is_some() {
                return Some(SizeControlMode::Focus);
            }
            return None;
        }

        if self.selected_tool == Tool::Text {
            return None;
        }

        if self.selected_tool == Tool::Obfuscate {
            return Some(SizeControlMode::Obfuscate);
        }

        if self.selected_tool == Tool::Focus {
            return Some(SizeControlMode::Focus);
        }

        if super::types::tool_uses_stroke_size(self.selected_tool) {
            return Some(SizeControlMode::Stroke);
        }

        None
    }

    pub fn active_size_value(&self) -> Option<f64> {
        match self.active_size_control_mode()? {
            SizeControlMode::Stroke => {
                if self.selected_tool == Tool::Select {
                    Some(
                        self.selected_action_stroke_size()
                            .unwrap_or(self.stroke_size),
                    )
                } else {
                    Some(self.stroke_size)
                }
            }
            SizeControlMode::Obfuscate => {
                if self.selected_tool == Tool::Select {
                    Some(
                        self.selected_obfuscate_action_amount()
                            .unwrap_or_else(|| self.current_obfuscate_amount()),
                    )
                } else {
                    Some(self.current_obfuscate_amount())
                }
            }
            SizeControlMode::Focus => {
                if self.selected_tool == Tool::Select {
                    Some(
                        self.selected_focus_action_intensity()
                            .unwrap_or_else(|| self.current_focus_intensity()),
                    )
                } else {
                    Some(self.current_focus_intensity())
                }
            }
        }
    }

    pub fn set_active_size_without_rebuild(&mut self, size: f64) -> bool {
        match self.active_size_control_mode() {
            Some(SizeControlMode::Stroke) => {
                let changed = self.set_stroke_size(size);
                let is_highlighter = self
                    .selected_action()
                    .is_some_and(|a| matches!(a, AnnotationAction::Highlighter { .. }));
                if !is_highlighter {
                    let _ = self.set_selected_action_stroke_size(self.stroke_size);
                }
                changed
            }
            Some(SizeControlMode::Obfuscate) => {
                // Update the per-method amount for the current method only.
                // This ensures Pixelate and Blur each keep their own intensity values.
                let changed = self.set_current_obfuscate_amount_and_check(size);
                // Also update any currently selected obfuscate action in-place.
                let current_amount = self.current_obfuscate_amount();
                let _ = self.set_selected_obfuscate_action_amount_without_rebuild(current_amount);
                changed
            }
            Some(SizeControlMode::Focus) => {
                let changed = self.set_current_focus_intensity_and_check(size);
                let current_intensity = self.current_focus_intensity();
                let _ = self.set_selected_focus_action_intensity_without_rebuild(current_intensity);
                changed
            }
            None => false,
        }
    }

    pub fn selected_action_color(&self) -> Option<DrawColor> {
        match self.selected_action()? {
            AnnotationAction::Pen { color, .. }
            | AnnotationAction::Highlighter { color, .. }
            | AnnotationAction::Circle { color, .. }
            | AnnotationAction::Line { color, .. }
            | AnnotationAction::Arrow { color, .. }
            | AnnotationAction::Box { color, .. }
            | AnnotationAction::Text { color, .. }
            | AnnotationAction::Number { color, .. } => Some(*color),
            AnnotationAction::Obfuscate { .. } | AnnotationAction::Focus { .. } => None,
        }
    }

    pub fn set_selected_action_color(&mut self, color: DrawColor) -> bool {
        if let Some(input) = self.active_text_input.as_mut() {
            input.color = color;
            return true;
        }

        let Some(index) = self.selected_action_index else {
            return false;
        };

        let Some(action) = self.actions.get_mut(index) else {
            self.selected_action_index = None;
            return false;
        };

        let target = match action {
            AnnotationAction::Pen { color, .. }
            | AnnotationAction::Highlighter { color, .. }
            | AnnotationAction::Circle { color, .. }
            | AnnotationAction::Line { color, .. }
            | AnnotationAction::Arrow { color, .. }
            | AnnotationAction::Box { color, .. }
            | AnnotationAction::Text { color, .. }
            | AnnotationAction::Number { color, .. } => color,
            AnnotationAction::Obfuscate { .. } | AnnotationAction::Focus { .. } => return false,
        };

        if *target == color {
            return false;
        }

        *target = color;
        self.redo_actions.clear();
        true
    }

    pub fn sync_next_number(&mut self) {
        let max_number = self
            .actions
            .iter()
            .filter_map(|action| match action {
                AnnotationAction::Number { number, style, .. } => {
                    // Only consider numbers with the same style
                    if *style == self.numbering_style {
                        Some(*number)
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .max()
            .unwrap_or(0);
        // Use the user-specified starting number if no numbers exist yet
        self.next_number = if max_number == 0 {
            self.numbering_start
        } else {
            max_number.saturating_add(1)
        };
    }

    pub fn add_number_marker(&mut self, position: Point) {
        let number = self.next_number;
        let radius = self.number_size.radius();
        let image_width = self.working_image.width() as f64;
        let image_height = self.working_image.height() as f64;

        let clamped_x = if image_width <= radius * 2.0 {
            image_width / 2.0
        } else {
            position.x.clamp(radius, image_width - radius)
        };
        let clamped_y = if image_height <= radius * 2.0 {
            image_height / 2.0
        } else {
            position.y.clamp(radius, image_height - radius)
        };

        self.push_action(AnnotationAction::Number {
            position: Point {
                x: clamped_x,
                y: clamped_y,
            },
            number,
            color: self.selected_color,
            style: self.numbering_style,
            size: self.number_size,
            shadow: self.draw_object_shadow,
        });
    }

    pub fn selected_action(&self) -> Option<&AnnotationAction> {
        self.selected_action_index
            .and_then(|index| self.actions.get(index))
    }

    pub fn select_action_at_point_with_scale(&mut self, point: Point, view_scale: f64) -> bool {
        let hit_padding = selection_hit_padding_for_scale(view_scale);

        self.selected_action_index = self
            .actions
            .iter()
            .enumerate()
            .rev()
            .find(|(_, action)| action_contains_point_with_padding(action, point, hit_padding))
            .map(|(index, _)| index);
        self.select_drag_anchor = None;
        self.select_resize_handle = None;
        self.selected_action_index.is_some()
    }

    pub fn begin_select_drag_with_scale(&mut self, point: Point, view_scale: f64) -> bool {
        let handle_hit_radius = selection_handle_hit_radius_for_scale(view_scale);

        if let Some(selected) = self.selected_action() {
            if let Some(handle) =
                action_resize_handle_at_point_with_radius(selected, point, handle_hit_radius)
            {
                self.select_resize_handle = Some(handle);
                self.select_drag_anchor = Some(point);
                return true;
            }
        }

        self.select_resize_handle = None;
        let selected = self.select_action_at_point_with_scale(point, view_scale);
        if selected {
            self.select_drag_anchor = Some(point);
        }
        selected
    }

    pub fn update_select_drag(&mut self, point: Point) -> bool {
        let Some(anchor) = self.select_drag_anchor else {
            return false;
        };
        let Some(index) = self.selected_action_index else {
            return false;
        };

        let dx = point.x - anchor.x;
        let dy = point.y - anchor.y;

        let img_w = self.base_image.width() as i32;
        let img_h = self.base_image.height() as i32;

        let resize_handle = self.select_resize_handle;
        let (moved, effect_action) = if let Some(action) = self.actions.get_mut(index) {
            let moved = if let Some(handle) = resize_handle {
                resize_action(action, handle, dx, dy)
            } else {
                translate_action(action, dx, dy)
            };

            // Clamp the action so it cannot be moved/resized outside the image bounds.
            if moved {
                clamp_action_to_image(action, img_w, img_h);
            }

            let effect_action = matches!(
                action,
                AnnotationAction::Obfuscate { .. } | AnnotationAction::Focus { .. }
            );
            (moved, effect_action)
        } else {
            self.selected_action_index = None;
            self.select_drag_anchor = None;
            return false;
        };

        if !moved {
            return false;
        }

        self.select_drag_anchor = Some(point);
        self.redo_actions.clear();
        if effect_action {
            self.select_drag_effect_dirty = true;
        }
        true
    }

    #[cfg(test)]
    pub fn end_select_drag(&mut self) -> bool {
        let rebuild = self.select_drag_effect_dirty;
        if rebuild {
            self.rebuild_effect_layer();
            self.select_drag_effect_dirty = false;
        }
        self.end_select_drag_without_rebuild();
        rebuild
    }

    pub fn end_select_drag_without_rebuild(&mut self) {
        self.select_drag_anchor = None;
        self.select_resize_handle = None;
        self.drag_start = None;
        self.drag_current = None;
        self.drag_start_view = None;
        self.drag_path.clear();
    }

    pub fn end_select_drag_without_rebuild_and_check_effect(&mut self) -> bool {
        let rebuild = self.select_drag_effect_dirty;
        self.select_drag_effect_dirty = false;
        self.end_select_drag_without_rebuild();
        rebuild
    }

    pub fn remove_selected_action(&mut self) -> bool {
        if self.remove_selected_action_without_rebuild() {
            self.rebuild_effect_layer();
            true
        } else {
            false
        }
    }

    pub fn remove_selected_action_without_rebuild(&mut self) -> bool {
        let Some(index) = self.selected_action_index.take() else {
            return false;
        };

        if index >= self.actions.len() {
            return false;
        }

        let removed = self.actions.remove(index);
        let next_number_after_remove = match &removed {
            AnnotationAction::Number { number, style, .. } if *style == self.numbering_style => {
                Some(*number)
            }
            _ => None,
        };
        self.select_drag_anchor = None;
        self.select_resize_handle = None;
        self.redo_actions.clear();
        if let Some(next_number) = next_number_after_remove {
            self.next_number = next_number;
        } else {
            self.sync_next_number();
        }
        true
    }

    pub fn rebuild_effect_layer(&mut self) {
        let mut working = (*self.base_image).clone();
        apply_effect_actions(&mut working, &self.actions);
        self.working_image = Arc::new(working);
        self.select_effect_rebuild_pending = false;
        self.mark_working_image_dirty();
    }
}

/// Clamp an annotation action so it stays within the image bounds.
/// For rect-based actions (Obfuscate, Focus, Box, Circle) the rect is clamped.
/// For point-based actions (Text, Number, Pen, Arrow, Line) each point is clamped.
fn clamp_action_to_image(action: &mut AnnotationAction, img_w: i32, img_h: i32) {
    match action {
        AnnotationAction::Obfuscate { rect, .. }
        | AnnotationAction::Focus { rect, .. }
        | AnnotationAction::Box { rect, .. }
        | AnnotationAction::Circle { rect, .. } => {
            // Keep the rect fully inside the image.
            let w = rect.width.min(img_w);
            let h = rect.height.min(img_h);
            rect.width = w;
            rect.height = h;
            rect.x = rect.x.max(0).min(img_w - w);
            rect.y = rect.y.max(0).min(img_h - h);
        }
        AnnotationAction::Text {
            position,
            text,
            font,
            max_width,
            ..
        } => {
            // Compute the real rendered bounds so we clamp correctly for
            // any number of lines at any font size.
            let surface = match gtk4::cairo::ImageSurface::create(gtk4::cairo::Format::ARgb32, 1, 1)
            {
                Ok(s) => s,
                Err(_) => return,
            };
            let context = match gtk4::cairo::Context::new(&surface) {
                Ok(c) => c,
                Err(_) => return,
            };
            let available_width = max_width.unwrap_or(font.size * 1.8).max(font.size * 1.8);
            let bounds = super::render::text_action_bounds(
                &context,
                *position,
                text,
                font,
                Some(available_width),
            );
            let box_w = bounds.rect.width as f64;
            let box_h = bounds.rect.height as f64;

            // Clamp box_left to [0, img_w - box_w]
            let new_box_left = (bounds.rect.x as f64)
                .max(0.0)
                .min((img_w as f64 - box_w).max(0.0));
            position.x = new_box_left; // position.x == box_left for Text

            // Clamp box_top to [0, img_h - box_h], then recompute baseline
            // position.y = box_top + font.size + padding_y
            let padding_y = 8.0;
            let new_box_top = (bounds.rect.y as f64)
                .max(0.0)
                .min((img_h as f64 - box_h).max(0.0));
            position.y = new_box_top + font.size + padding_y;
        }
        AnnotationAction::Number { position, .. } => {
            position.x = position.x.max(0.0).min(img_w as f64);
            position.y = position.y.max(0.0).min(img_h as f64);
        }
        AnnotationAction::Pen { points, .. } | AnnotationAction::Highlighter { points, .. } => {
            for p in points {
                p.x = p.x.max(0.0).min(img_w as f64);
                p.y = p.y.max(0.0).min(img_h as f64);
            }
        }
        AnnotationAction::Line { start, end, .. } => {
            start.x = start.x.max(0.0).min(img_w as f64);
            start.y = start.y.max(0.0).min(img_h as f64);
            end.x = end.x.max(0.0).min(img_w as f64);
            end.y = end.y.max(0.0).min(img_h as f64);
        }
        AnnotationAction::Arrow {
            start,
            end,
            control_points,
            stroke_size,
            ..
        } => {
            let iw = img_w as f64;
            let ih = img_h as f64;
            // Account for stroke width — the arrow's visual bounds extend
            // beyond the curve centerline by roughly half the stroke size.
            let margin = *stroke_size * 0.5;
            // Compute the actual visual bounds of the arrow including Bezier
            // curve extrema, not just the endpoints. A quadratic Bezier can
            // bulge well beyond its start/end points.
            let mut min_x = start.x.min(end.x);
            let mut max_x = start.x.max(end.x);
            let mut min_y = start.y.min(end.y);
            let mut max_y = start.y.max(end.y);

            if let Some(cps) = control_points.as_ref() {
                if cps.len() >= 3 {
                    let p0 = *start;
                    let p1 = cps[1]; // middle control point
                    let p2 = *end;
                    // Quadratic Bezier extrema: t = (P0 - P1) / (P0 - 2*P1 + P2)
                    // Check x-axis extremum
                    let denom_x = p0.x - 2.0 * p1.x + p2.x;
                    if denom_x.abs() > 1e-10 {
                        let t = (p0.x - p1.x) / denom_x;
                        if t > 0.0 && t < 1.0 {
                            let bx = (1.0 - t).powi(2) * p0.x
                                + 2.0 * (1.0 - t) * t * p1.x
                                + t.powi(2) * p2.x;
                            min_x = min_x.min(bx);
                            max_x = max_x.max(bx);
                        }
                    }
                    // Check y-axis extremum
                    let denom_y = p0.y - 2.0 * p1.y + p2.y;
                    if denom_y.abs() > 1e-10 {
                        let t = (p0.y - p1.y) / denom_y;
                        if t > 0.0 && t < 1.0 {
                            let by = (1.0 - t).powi(2) * p0.y
                                + 2.0 * (1.0 - t) * t * p1.y
                                + t.powi(2) * p2.y;
                            min_y = min_y.min(by);
                            max_y = max_y.max(by);
                        }
                    }
                }
            }

            let shift_x = if min_x < margin {
                margin - min_x
            } else if max_x > iw - margin {
                (iw - margin) - max_x
            } else {
                0.0
            };
            let shift_y = if min_y < margin {
                margin - min_y
            } else if max_y > ih - margin {
                (ih - margin) - max_y
            } else {
                0.0
            };
            if shift_x != 0.0 || shift_y != 0.0 {
                start.x += shift_x;
                start.y += shift_y;
                end.x += shift_x;
                end.y += shift_y;
                if let Some(cps) = control_points.as_mut() {
                    for cp in cps.iter_mut() {
                        cp.x += shift_x;
                        cp.y += shift_y;
                    }
                }
            }
        }
    }
}

pub fn apply_effect_actions(image: &mut RgbaImage, actions: &[AnnotationAction]) {
    for action in actions {
        match action {
            AnnotationAction::Obfuscate {
                rect,
                method,
                amount,
            } => match method {
                ObfuscateMethod::Pixelate => {
                    apply_censor_rect(image, *rect, *amount);
                }
                ObfuscateMethod::Blur => {
                    apply_hybrid_blur(image, *rect, *amount);
                }
                ObfuscateMethod::Blackout => {
                    apply_blackout_rect(image, rect);
                }
            },
            AnnotationAction::Focus { rect, intensity } => {
                apply_focus_rect(image, *rect, *intensity);
            }
            _ => {}
        }
    }
}

pub(crate) fn render_shadow_layer(
    width: u32,
    height: u32,
    blur: f64,
    opacity: f64,
    corner_radius: f64,
) -> Result<RgbaImage, EditorError> {
    let spread_px = (blur * 1.35).ceil().max(0.0) as i32;
    let shadow_width = width as i32 + spread_px * 2;
    let shadow_height = height as i32 + spread_px * 2;
    let stride = gtk4::cairo::Format::ARgb32
        .stride_for_width(shadow_width as u32)
        .map_err(|e| EditorError::ImageSave(e.to_string()))?;
    let mut surface =
        gtk4::cairo::ImageSurface::create(gtk4::cairo::Format::ARgb32, shadow_width, shadow_height)
            .map_err(|e| EditorError::ImageSave(e.to_string()))?;
    {
        let context = gtk4::cairo::Context::new(&surface)
            .map_err(|e| EditorError::ImageSave(e.to_string()))?;
        context.set_source_rgba(0.0, 0.0, 0.0, opacity.clamp(0.0, 1.0));
        draw_rounded_rect_path(
            &context,
            spread_px as f64,
            spread_px as f64,
            width as f64,
            height as f64,
            corner_radius,
        );
        let _ = context.fill();
    }
    surface.flush();
    let data = surface
        .data()
        .map_err(|e| EditorError::ImageSave(e.to_string()))?;
    Ok(super::render::cairo_argb_to_rgba_image(
        shadow_width as u32,
        shadow_height as u32,
        stride as usize,
        data.as_ref(),
    ))
}

pub(super) fn draw_rounded_rect_path(
    context: &gtk4::cairo::Context,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    radius: f64,
) {
    let radius = radius.min(width / 2.0).min(height / 2.0).max(0.0);
    if radius <= 0.0 {
        context.rectangle(x, y, width, height);
        return;
    }

    let right = x + width;
    let bottom = y + height;
    context.new_sub_path();
    context.arc(
        right - radius,
        y + radius,
        radius,
        -std::f64::consts::FRAC_PI_2,
        0.0,
    );
    context.arc(
        right - radius,
        bottom - radius,
        radius,
        0.0,
        std::f64::consts::FRAC_PI_2,
    );
    context.arc(
        x + radius,
        bottom - radius,
        radius,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
    );
    context.arc(
        x + radius,
        y + radius,
        radius,
        std::f64::consts::PI,
        std::f64::consts::PI * 1.5,
    );
    context.close_path();
}

pub(super) fn apply_corner_radius(image: &mut RgbaImage, radius: f64) {
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 || radius <= 0.0 {
        return;
    }

    let radius = radius.min(width as f64 / 2.0).min(height as f64 / 2.0);
    if radius <= 0.0 {
        return;
    }

    let Some(source_surface) = rgba_image_to_surface(image) else {
        return;
    };

    let stride = match gtk4::cairo::Format::ARgb32.stride_for_width(width) {
        Ok(stride) => stride,
        Err(_) => return,
    };
    let mut clipped_surface = match gtk4::cairo::ImageSurface::create(
        gtk4::cairo::Format::ARgb32,
        width as i32,
        height as i32,
    ) {
        Ok(surface) => surface,
        Err(_) => return,
    };

    {
        let context = match gtk4::cairo::Context::new(&clipped_surface) {
            Ok(context) => context,
            Err(_) => return,
        };
        context.set_antialias(gtk4::cairo::Antialias::Best);
        draw_rounded_rect_path(&context, 0.0, 0.0, width as f64, height as f64, radius);
        context.clip();
        if context
            .set_source_surface(&source_surface, 0.0, 0.0)
            .is_err()
        {
            return;
        }
        let _ = context.paint();
    }

    clipped_surface.flush();
    let surface_data = match clipped_surface.data() {
        Ok(data) => data,
        Err(_) => return,
    };
    *image = cairo_argb_to_rgba_image(width, height, stride as usize, surface_data.as_ref());
}

#[cfg(test)]
mod tests {
    use image::RgbaImage;

    use crate::capture::editor::color::DEFAULT_OBFUSCATE_AMOUNT;
    use crate::capture::editor::types::{AnnotationAction, ObfuscateMethod, Point, Rect};

    use super::{apply_corner_radius, EditorState};

    #[test]
    fn editor_state_defaults_to_background_tool() {
        let source = include_str!("mod.rs");
        let production_source = source.split("#[cfg(test)]").next().unwrap_or(source);
        assert!(
            production_source.contains("selected_tool: Tool::Background,"),
            "Editor state should default to the Background tool so startup inspector width matches the initial tool surface",
        );
    }

    #[test]
    fn corner_radius_antialiases_top_right_edge() {
        let mut image = RgbaImage::from_pixel(40, 40, image::Rgba([255, 255, 255, 255]));

        apply_corner_radius(&mut image, 12.0);

        let top_right_band_has_partial_alpha = (28..40).any(|x| {
            (0..12).any(|y| {
                let alpha = image.get_pixel(x, y)[3];
                alpha > 0 && alpha < 255
            })
        });

        assert!(
            top_right_band_has_partial_alpha,
            "expected antialiased pixels along the top-right rounded edge"
        );
        assert_eq!(image.get_pixel(39, 0)[3], 0);
        assert_eq!(image.get_pixel(20, 20)[3], 255);
    }

    #[test]
    fn focus_tool_uses_dedicated_slider_state_and_persists_intensity_per_action() {
        let mut state = EditorState::new(RgbaImage::from_pixel(
            16,
            16,
            image::Rgba([200, 180, 160, 255]),
        ));
        state.selected_tool = super::Tool::Focus;

        assert_eq!(
            state.active_size_control_mode(),
            Some(super::SizeControlMode::Focus)
        );
        assert_eq!(state.active_size_value(), Some(58.0));

        assert!(state.set_active_size_without_rebuild(72.0));
        assert_eq!(state.current_focus_intensity(), 72.0);
        assert_eq!(state.active_size_value(), Some(72.0));

        state.drag_start = Some(Point { x: 2.0, y: 2.0 });
        state.drag_current = Some(Point { x: 10.0, y: 10.0 });
        let draft = state.draft_action().expect("focus draft");
        match draft {
            AnnotationAction::Focus { rect, intensity } => {
                assert_eq!(rect.x, 2);
                assert_eq!(rect.y, 2);
                assert_eq!(rect.width, 8);
                assert_eq!(rect.height, 8);
                assert_eq!(intensity, 72.0);
            }
            other => panic!("expected focus draft, got {other:?}"),
        }

        state.actions.push(AnnotationAction::Focus {
            rect: Rect {
                x: 3,
                y: 3,
                width: 6,
                height: 6,
            },
            intensity: 44.0,
        });
        state.selected_tool = super::Tool::Select;
        state.selected_action_index = Some(0);

        assert_eq!(
            state.active_size_control_mode(),
            Some(super::SizeControlMode::Focus)
        );
        assert_eq!(state.active_size_value(), Some(44.0));
        assert!(state.set_active_size_without_rebuild(66.0));
        assert_eq!(state.selected_focus_action_intensity(), Some(66.0));
        state.rebuild_effect_layer();

        let final_image = state.to_rendered_image().expect("rendered image");
        assert_eq!(
            *final_image.get_pixel(4, 4),
            image::Rgba([200, 180, 160, 255])
        );
        let outside = *final_image.get_pixel(1, 1);
        assert!(outside[0] < 200 && outside[1] < 180 && outside[2] < 160);
    }

    #[test]
    fn obfuscate_blur_uses_single_shared_blur_method_and_slider_state() {
        let mut state = EditorState::new(RgbaImage::new(32, 32));
        state.set_obfuscate_method(ObfuscateMethod::Blur);

        assert_eq!(state.current_obfuscate_amount(), DEFAULT_OBFUSCATE_AMOUNT);
        assert_eq!(state.active_size_control_mode(), None);

        state.selected_tool = super::Tool::Obfuscate;
        assert_eq!(
            state.active_size_control_mode(),
            Some(super::SizeControlMode::Obfuscate)
        );
        assert_eq!(state.active_size_value(), Some(DEFAULT_OBFUSCATE_AMOUNT));

        assert!(state.set_active_size_without_rebuild(21.0));
        assert_eq!(state.current_obfuscate_amount(), 21.0);

        state.drag_start = Some(Point { x: 4.0, y: 5.0 });
        state.drag_current = Some(Point { x: 15.0, y: 18.0 });
        match state.draft_action().expect("obfuscate draft") {
            AnnotationAction::Obfuscate {
                rect,
                method,
                amount,
            } => {
                assert_eq!(rect.x, 4);
                assert_eq!(rect.y, 5);
                assert_eq!(rect.width, 11);
                assert_eq!(rect.height, 13);
                assert_eq!(method, ObfuscateMethod::Blur);
                assert_eq!(amount, 21.0);
            }
            other => panic!("expected obfuscate draft, got {other:?}"),
        }
    }
}
