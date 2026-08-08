//! Canvas draw function and render caches (PR 10.19).
//!
//! Owns working-image / background / shadow surface caches and the
//! `DrawingArea::set_draw_func` body. Snapshot `EditorState` under the lock,
//! then release before Cairo work. Session/bootstrap entry points stay in
//! `window/mod.rs`.

use gtk4::cairo::ImageSurface;
use gtk4::{prelude::*, Button, DrawingArea};
use image::RgbaImage;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::capture::editor::{
    color::selection_hit_padding_for_scale,
    composition::BackgroundComposition,
    render::{
        draw_active_text_input, draw_annotation_action, draw_arrow_control_handles,
        draw_arrow_selection_outline, draw_canvas_checkerboard_background, draw_crop_overlay,
        draw_draft_action, draw_rgba_to_context, draw_selection_handles, draw_selection_outline,
        draw_text_edit_border, draw_text_edit_handles, rgba_image_to_surface, text_action_bounds,
    },
    selection::{action_bounds_with_padding, action_resize_handles},
    state::{render_shadow_layer, EditorState},
    types::{AnnotationAction, BackgroundStyle, Rect, Tool, ViewTransform},
    ui_support::EDITOR_TOP_CHROME_HEIGHT,
};

const MAX_PREVIEW_SHADOW_DIM: u32 = 1200;
const PREVIEW_SHADOW_BLUR_PASSES: usize = 2;

/// Surface caches shared by the canvas draw path (working image, background, shadow).
#[derive(Clone)]
pub(super) struct CanvasRenderCaches {
    pub working_surface: Rc<RefCell<Option<ImageSurface>>>,
    pub working_revision: Rc<Cell<u64>>,
    pub background_surface: Rc<RefCell<Option<ImageSurface>>>,
    pub background_signature: Rc<RefCell<Option<(BackgroundStyle, Option<u64>)>>>,
    pub shadow_surface: Rc<RefCell<Option<ImageSurface>>>,
    pub shadow_signature: Rc<Cell<Option<(u32, u32, u64, u64, u64)>>>,
}

impl CanvasRenderCaches {
    pub(super) fn new() -> Self {
        Self {
            working_surface: Rc::new(RefCell::new(None)),
            working_revision: Rc::new(Cell::new(0)),
            background_surface: Rc::new(RefCell::new(None)),
            background_signature: Rc::new(RefCell::new(None)),
            shadow_surface: Rc::new(RefCell::new(None)),
            shadow_signature: Rc::new(Cell::new(None)),
        }
    }
}

/// Inputs for installing the canvas `set_draw_func`.
pub(super) struct CanvasDrawInputs<'a> {
    pub state: &'a Arc<Mutex<EditorState>>,
    pub transform: &'a Arc<Mutex<ViewTransform>>,
    pub drawing_area: &'a DrawingArea,
    pub zoom_level: &'a Rc<Cell<f64>>,
    pub undo_btn: &'a Button,
    pub redo_btn: &'a Button,
    pub delete_selected_btn: &'a Button,
    pub canvas_padding: i32,
    pub prefers_dark: bool,
    pub caches: &'a CanvasRenderCaches,
    pub gradient_surfaces: &'a Rc<RefCell<Vec<Option<ImageSurface>>>>,
    pub wallpaper_cache: &'a Rc<RefCell<HashMap<PathBuf, ImageSurface>>>,
}

/// Install the canvas draw function. Releases `EditorState` before Cairo work.
pub(super) fn install_canvas_draw_func(input: CanvasDrawInputs<'_>) {
    let CanvasDrawInputs {
        state,
        transform,
        drawing_area,
        zoom_level,
        undo_btn,
        redo_btn,
        delete_selected_btn,
        canvas_padding,
        prefers_dark,
        caches,
        gradient_surfaces,
        wallpaper_cache,
    } = input;

    let state_draw = state.clone();
    let transform_draw = transform.clone();
    let zoom_level_draw = zoom_level.clone();
    let undo_btn_draw = undo_btn.clone();
    let redo_btn_draw = redo_btn.clone();
    let delete_selected_btn_draw = delete_selected_btn.clone();
    let working_surface = caches.working_surface.clone();
    let working_revision = caches.working_revision.clone();
    let background_surface = caches.background_surface.clone();
    let background_signature_cache = caches.background_signature.clone();
    let shadow_surface = caches.shadow_surface.clone();
    let shadow_signature_cache = caches.shadow_signature.clone();
    let canvas_padding_draw = canvas_padding as f64;
    let gradient_surfaces = gradient_surfaces.clone();
    let wallpaper_cache = wallpaper_cache.clone();
    drawing_area.set_draw_func(move |_, context, width, height| {
        // IMPORTANT: do not hold the state mutex while performing cairo drawing.
        // The async effects pipeline also locks this mutex on the GTK thread to apply results;
        // holding it here can cause UI stalls/deadlocks.
        let (
            can_undo,
            can_redo,
            can_delete,
            working_image,
            working_image_revision,
            actions,
            draft_action,
            crop_rect,
            crop_mode_active,
            crop_background_color_explicit,
            crop_background_color,
            background_style,
            background_padding,
            background_aspect_ratio,
            background_insert,
            background_alignment,
            background_shadow,
            background_corner_radius,
            selected_tool,
            selected_action,
            select_drag_anchor,
            select_resize_handle,
            active_text_bounds,
            active_text_input,
            active_text_drag_handle,
            text_font_family,
            text_size,
            hovered_text_action_index,
            arrow_editing_controls,
        ) = {
            let st = state_draw.lock().unwrap();
            let (can_undo, can_redo) = st.history_availability();
            (
                can_undo,
                can_redo,
                st.can_remove_selected_action(),
                Arc::clone(&st.working_image),
                st.working_image_revision,
                st.actions.clone(),
                st.draft_action(),
                if st.selected_tool == Tool::Crop {
                    st.draft_crop_rect().or(st.crop_selection)
                } else {
                    None
                },
                st.selected_tool == Tool::Crop,
                st.crop_background_color_explicit,
                st.crop_background_color,
                st.background_style.clone(),
                st.background_padding,
                st.background_aspect_ratio,
                st.background_insert,
                st.background_alignment,
                st.background_shadow,
                st.background_corner_radius,
                st.selected_tool,
                st.selected_action().cloned(),
                st.select_drag_anchor,
                st.select_resize_handle,
                st.active_text_bounds.clone(),
                st.active_text_input.clone(),
                st.active_text_drag_handle.clone(),
                st.text_font_family.clone(),
                st.text_size,
                st.hovered_text_action_index,
                st.arrow_editing_controls,
            )
        };

        undo_btn_draw.set_sensitive(can_undo);
        redo_btn_draw.set_sensitive(can_redo);
        delete_selected_btn_draw.set_sensitive(can_delete);

        let image_width = working_image.width() as f64;
        let image_height = working_image.height() as f64;
        let mut virtual_w = image_width;
        let mut virtual_h = image_height;
        let mut draw_scale_factor = 1.0;
        let mut background_scale_factor = 1.0;
        let mut background_layout = None;

        let has_background = background_style != BackgroundStyle::None;
        if has_background {
            let layout = BackgroundComposition::new(image_width, image_height)
                .with_style(background_style.clone())
                .with_padding(background_padding)
                .with_shadow(background_shadow)
                .with_insert(background_insert)
                .with_alignment(background_alignment)
                .with_corner_radius(background_corner_radius)
                .with_aspect_ratio(background_aspect_ratio)
                .compute();
            virtual_w = layout.canvas_width;
            virtual_h = layout.canvas_height;
            draw_scale_factor = layout.draw_scale;
            background_scale_factor = layout.scale_factor;
            background_layout = Some(layout);
        }

        let toolbar_clearance = f64::from(EDITOR_TOP_CHROME_HEIGHT);
        let top_pad = canvas_padding_draw + toolbar_clearance;
        let side_pad = canvas_padding_draw;
        let base_view_width = (width as f64 - side_pad * 2.0).max(1.0);
        let base_scale = (base_view_width / virtual_w).min(1.0);
        let (overflow_left, overflow_top, overflow_right, overflow_bottom) = if has_background {
            (0.0, 0.0, 0.0, 0.0)
        } else {
            super::canvas::crop_canvas_overflow(
                crop_rect,
                image_width,
                image_height,
                base_scale,
                crop_mode_active,
            )
        };

        let view_width = (width as f64 - side_pad * 2.0 - overflow_left - overflow_right).max(1.0);
        let view_height =
            (height as f64 - top_pad - side_pad - overflow_top - overflow_bottom).max(1.0);

        let scale = (view_width / virtual_w)
            .min(view_height / virtual_h)
            .min(1.0_f64)
            * zoom_level_draw.get().max(0.1_f64);
        let draw_width = virtual_w * scale;
        let draw_height = virtual_h * scale;
        // Center within the area below the toolbar strip; top_pad keeps image clear of tools.
        let placement = super::canvas::initial_viewport_offset(
            draw_width,
            draw_height,
            view_width,
            view_height,
            0.0,
        );
        let mut t = ViewTransform {
            scale,
            offset_x: side_pad + placement.offset_x + overflow_left,
            offset_y: top_pad + placement.offset_y + overflow_top,
            image_width: virtual_w,
            image_height: virtual_h,
        };

        let canvas_t = t;

        context.set_operator(gtk4::cairo::Operator::Source);
        draw_canvas_checkerboard_background(
            context,
            width,
            height,
            if crop_mode_active && crop_background_color_explicit {
                Some(crop_background_color)
            } else {
                None
            },
            !prefers_dark,
        );

        if has_background {
            context.set_operator(gtk4::cairo::Operator::Over);
            let current_style = background_style.clone();
            let current_background_signature = (
                current_style.clone(),
                if matches!(current_style, BackgroundStyle::Blurred(_)) {
                    Some(working_image_revision)
                } else {
                    None
                },
            );
            let needs_background_surface = !matches!(
                current_style,
                BackgroundStyle::None | BackgroundStyle::PlainColor(_)
            );
            let mut bg_cache = background_surface.borrow_mut();
            let mut bg_signature_cache = background_signature_cache.borrow_mut();

            if bg_signature_cache.as_ref() != Some(&current_background_signature)
                || (needs_background_surface && bg_cache.is_none())
            {
                if let BackgroundStyle::Gradient(idx) = &current_style {
                    let surfaces = gradient_surfaces.borrow();
                    if let Some(surface) = surfaces.get(*idx).and_then(|s| s.as_ref()) {
                        *bg_cache = Some(surface.clone());
                    } else {
                        let file_name =
                            super::background_panel::BACKGROUND_GRADIENT_PREVIEW_FILES[*idx];
                        let path =
                            super::background_panel::background_gradient_asset_path(file_name);
                        *bg_cache = rgba_image_to_surface(
                            &super::background_panel::load_background_image_optimized(&path)
                                .unwrap_or_else(|| RgbaImage::new(1, 1)),
                        );
                    }
                } else if let BackgroundStyle::Wallpaper(path) = &current_style {
                    let cache = wallpaper_cache.borrow();
                    if let Some(surface) = cache.get(path) {
                        *bg_cache = Some(surface.clone());
                    } else {
                        println!(
                            "[DEBUG] Cache miss for wallpaper: {:?}, loading synchronously",
                            path
                        );
                        if let Some(rgba) =
                            super::background_panel::load_background_image_optimized(path)
                        {
                            let surface = rgba_image_to_surface(&rgba);
                            *bg_cache = surface;
                        } else {
                            println!("[DEBUG] Failed to load wallpaper synchronously: {:?}", path);
                            *bg_cache = None;
                        }
                    }
                } else if let BackgroundStyle::PlainColor(_color) = &current_style {
                    *bg_cache = None;
                } else if let BackgroundStyle::Blurred(blur_idx) = &current_style {
                    let (bw, bh) = working_image.dimensions();

                    // Optimization: Downsample for background blur to save CPU.
                    // For very long webpage screenshots, resize directly from the
                    // source image to avoid cloning the full-size buffer first.
                    let max_dim = 800u32;
                    let mut blurred_bg = if bw > max_dim || bh > max_dim {
                        let scale = max_dim as f64 / (bw.max(bh) as f64);
                        image::imageops::resize(
                            &*working_image,
                            (bw as f64 * scale) as u32,
                            (bh as f64 * scale) as u32,
                            image::imageops::FilterType::Triangle,
                        )
                    } else {
                        (*working_image).clone()
                    };

                    let blur_radius = match blur_idx {
                        0 => 10.0,
                        1 => 35.0,
                        2 => 80.0,
                        _ => 20.0,
                    };

                    let (nbw, nbh) = blurred_bg.dimensions();
                    crate::capture::editor::render::apply_blur_rect(
                        &mut blurred_bg,
                        Rect {
                            x: 0,
                            y: 0,
                            width: nbw as i32,
                            height: nbh as i32,
                        },
                        blur_radius,
                        false,
                    );
                    *bg_cache = rgba_image_to_surface(&blurred_bg);
                }
                *bg_signature_cache = Some(current_background_signature);
            }

            if let Some(surface) = bg_cache.as_ref() {
                let _ = context.save();
                let sw = surface.width() as f64;
                let sh = surface.height() as f64;
                context.translate(canvas_t.offset_x, canvas_t.offset_y);
                context.scale(
                    (virtual_w * canvas_t.scale) / sw,
                    (virtual_h * canvas_t.scale) / sh,
                );
                context.set_source_surface(surface, 0.0, 0.0).unwrap();
                let _ = context.paint();
                let _ = context.restore();
            } else if let BackgroundStyle::PlainColor(color) = &current_style {
                context.set_source_rgba(color.r, color.g, color.b, color.a);
                context.rectangle(
                    canvas_t.offset_x,
                    canvas_t.offset_y,
                    virtual_w * canvas_t.scale,
                    virtual_h * canvas_t.scale,
                );
                let _ = context.fill();
            }

            if let Some(layout) = background_layout.as_ref() {
                t.offset_x = canvas_t.offset_x + layout.image_rect.x * canvas_t.scale;
                t.offset_y = canvas_t.offset_y + layout.image_rect.y * canvas_t.scale;
                t.scale = canvas_t.scale * layout.draw_scale;

                if let Some(shadow) = layout.shadow {
                    let mut shadow_surface_cache = shadow_surface.borrow_mut();
                    let base_blur = shadow.blur.max(1.0);
                    let base_corner = background_corner_radius * layout.scale_factor;
                    let preview_scale = (MAX_PREVIEW_SHADOW_DIM as f64
                        / image_width.max(image_height).max(1.0))
                    .min(1.0);
                    let preview_width = (image_width * preview_scale).round().max(1.0) as u32;
                    let preview_height = (image_height * preview_scale).round().max(1.0) as u32;
                    let preview_blur = (base_blur * preview_scale).max(1.0);
                    let preview_corner = base_corner * preview_scale;
                    let shadow_signature = (
                        preview_width,
                        preview_height,
                        preview_blur.to_bits(),
                        shadow.opacity.to_bits(),
                        preview_corner.to_bits(),
                    );
                    let needs_recompute = shadow_signature_cache.get() != Some(shadow_signature)
                        || shadow_surface_cache.is_none();

                    if needs_recompute {
                        if let Ok(mut shadow_image) = render_shadow_layer(
                            preview_width,
                            preview_height,
                            preview_blur,
                            shadow.opacity,
                            preview_corner,
                        ) {
                            let blur_rect = Rect {
                                x: 0,
                                y: 0,
                                width: shadow_image.width() as i32,
                                height: shadow_image.height() as i32,
                            };
                            let pass_radius = (preview_blur / 2.0).max(1.0);
                            for _ in 0..PREVIEW_SHADOW_BLUR_PASSES {
                                crate::capture::editor::render::apply_blur_rect(
                                    &mut shadow_image,
                                    blur_rect,
                                    pass_radius,
                                    true,
                                );
                            }
                            *shadow_surface_cache = rgba_image_to_surface(&shadow_image);
                            shadow_signature_cache.set(Some(shadow_signature));
                        }
                    }

                    if let Some(surface) = shadow_surface_cache.as_ref() {
                        let sw = surface.width() as f64;
                        let sh = surface.height() as f64;
                        let shadow_scale = t.scale;
                        let target_w = image_width * shadow_scale;
                        let target_h = image_height * shadow_scale;
                        let spread_px = (shadow.blur * 1.35).ceil().max(0.0);
                        let sx = (target_w + spread_px * 2.0) / sw;
                        let sy = (target_h + spread_px * 2.0) / sh;
                        let _ = context.save();
                        context.translate(
                            canvas_t.offset_x + shadow.rect.x * canvas_t.scale,
                            canvas_t.offset_y + shadow.rect.y * canvas_t.scale,
                        );
                        context.scale(sx, sy);
                        context.set_source_surface(surface, 0.0, 0.0).unwrap();
                        let _ = context.paint();
                        let _ = context.restore();
                    }
                }
            } else {
                t.scale = canvas_t.scale * draw_scale_factor;
            }

            let rect_w = image_width * t.scale;
            let rect_h = image_height * t.scale;
            let corner_r = background_corner_radius * background_scale_factor * t.scale;

            let _ = context.save();
            context.translate(t.offset_x, t.offset_y);
            draw_rounded_rect_path(context, rect_w, rect_h, corner_r, 0.0);
            context.clip();
            context.translate(-t.offset_x, -t.offset_y);
        }
        context.set_operator(gtk4::cairo::Operator::Over);
        *transform_draw.lock().unwrap() = t;

        let _ = context.save();
        context.translate(t.offset_x, t.offset_y);
        context.scale(t.scale, t.scale);

        if crop_mode_active && crop_background_color_explicit {
            if let Some(crop_rect) = crop_rect {
                context.set_source_rgba(
                    crop_background_color.r,
                    crop_background_color.g,
                    crop_background_color.b,
                    crop_background_color.a,
                );
                context.rectangle(
                    crop_rect.x as f64,
                    crop_rect.y as f64,
                    crop_rect.width as f64,
                    crop_rect.height as f64,
                );
                let _ = context.fill();
            }
        }

        if working_revision.get() != working_image_revision || working_surface.borrow().is_none() {
            *working_surface.borrow_mut() = rgba_image_to_surface(&working_image);
            working_revision.set(working_image_revision);
        }

        if let Some(surface) = working_surface.borrow().as_ref() {
            crate::capture::editor::render::paint_surface_with_filter(
                context,
                surface,
                0.0,
                0.0,
                crate::capture::editor::render::editor_image_filter_for_scale(t.scale),
            );
        } else {
            draw_rgba_to_context(context, &working_image);
        }

        let editing_action_index = active_text_input
            .as_ref()
            .and_then(|input| input.editing_action_index);
        for (index, action) in actions.iter().enumerate() {
            if Some(index) == editing_action_index {
                continue;
            }
            if matches!(
                action,
                AnnotationAction::Obfuscate { .. } | AnnotationAction::Focus { .. }
            ) {
                continue;
            }
            draw_annotation_action(context, action);
        }

        if let Some(draft) = draft_action {
            draw_draft_action(context, &draft);
        }

        if crop_mode_active {
            if let Some(crop_rect) = crop_rect {
                let canvas_left = -t.offset_x / t.scale;
                let canvas_top = -t.offset_y / t.scale;
                let canvas_width = width as f64 / t.scale;
                let canvas_height = height as f64 / t.scale;
                let _ = context.save();
                context.rectangle(canvas_left, canvas_top, canvas_width, canvas_height);
                context.rectangle(
                    crop_rect.x as f64,
                    crop_rect.y as f64,
                    crop_rect.width as f64,
                    crop_rect.height as f64,
                );
                context.set_fill_rule(gtk4::cairo::FillRule::EvenOdd);
                context.set_source_rgba(0.0, 0.0, 0.0, 140.0 / 255.0);
                let _ = context.fill();
                let _ = context.restore();
            }
        }

        if let Some(crop_rect) = crop_rect {
            draw_crop_overlay(
                context,
                working_image.width() as f64,
                working_image.height() as f64,
                crop_rect,
                selected_tool == Tool::Crop,
            );
        }

        // In Text tool mode: draw hover outline for the text action under the cursor.
        if selected_tool == Tool::Text && active_text_bounds.is_none() {
            if let Some(hover_idx) = hovered_text_action_index {
                if let Some(AnnotationAction::Text {
                    position,
                    text,
                    font,
                    max_width,
                    ..
                }) = actions.get(hover_idx)
                {
                    let available_width = max_width.unwrap_or_else(|| {
                        (working_image.width() as f64 - position.x).max(font.size * 1.8)
                    });
                    let mut text_bounds =
                        text_action_bounds(context, *position, text, font, Some(available_width));
                    text_bounds.rect.x = text_bounds.rect.x.clamp(
                        0,
                        (working_image.width() as i32 - text_bounds.rect.width).max(0),
                    );
                    text_bounds.rect.y = text_bounds.rect.y.clamp(
                        0,
                        (working_image.height() as i32 - text_bounds.rect.height).max(0),
                    );
                    text_bounds.sync_handles();
                    draw_text_edit_border(context, &text_bounds, t.scale);
                }
            }
        }

        if let Some(selected_action) = selected_action.as_ref() {
            if selected_tool == Tool::Select
                && select_drag_anchor.is_some()
                && matches!(
                    selected_action,
                    AnnotationAction::Obfuscate { .. } | AnnotationAction::Focus { .. }
                )
            {
                draw_draft_action(context, selected_action);
            }

            // Draw border + handles for a selected Text action in both
            // Select tool mode and Text tool mode (e.g. during drag-to-move).
            let show_text_handles = (selected_tool == Tool::Select || selected_tool == Tool::Text)
                && active_text_bounds.is_none();

            if show_text_handles {
                if let AnnotationAction::Text {
                    position,
                    text,
                    font,
                    max_width,
                    ..
                } = selected_action
                {
                    let available_width = max_width.unwrap_or_else(|| {
                        (working_image.width() as f64 - position.x).max(font.size * 1.8)
                    });
                    let mut text_bounds =
                        text_action_bounds(context, *position, text, font, Some(available_width));
                    text_bounds.rect.x = text_bounds.rect.x.clamp(
                        0,
                        (working_image.width() as i32 - text_bounds.rect.width).max(0),
                    );
                    text_bounds.rect.y = text_bounds.rect.y.clamp(
                        0,
                        (working_image.height() as i32 - text_bounds.rect.height).max(0),
                    );
                    text_bounds.sync_handles();
                    draw_text_edit_border(context, &text_bounds, t.scale);
                    draw_text_edit_handles(context, &text_bounds, None, t.scale);
                }
            }

            if selected_tool == Tool::Select
                || selected_tool == Tool::Arrow
                || (matches!(selected_tool, Tool::Box | Tool::Circle)
                    && matches!(
                        selected_action,
                        AnnotationAction::Box { .. } | AnnotationAction::Circle { .. }
                    ))
            {
                if let AnnotationAction::Text { .. } = selected_action {
                    // Already handled above.
                } else if let AnnotationAction::Arrow {
                    start,
                    end,
                    stroke_size,
                    style,
                    control_points,
                    ..
                } = selected_action
                {
                    draw_arrow_selection_outline(
                        context,
                        *start,
                        *end,
                        *stroke_size,
                        *style,
                        control_points.clone(),
                        t.scale,
                    );
                } else if matches!(selected_action, AnnotationAction::Line { .. }) {
                    // Intentionally show no crop-like selection outline or handles for lines.
                } else {
                    let selection_padding = selection_hit_padding_for_scale(t.scale);
                    if let Some(bounds) =
                        action_bounds_with_padding(selected_action, selection_padding)
                    {
                        if !matches!(
                            selected_action,
                            AnnotationAction::Box { .. } | AnnotationAction::Circle { .. }
                        ) {
                            draw_selection_outline(context, bounds, t.scale);
                        }
                    }

                    let handles = action_resize_handles(selected_action);
                    if !handles.is_empty() {
                        draw_selection_handles(context, &handles, select_resize_handle, t.scale);
                    }
                }
            }

            // The active text edit overlay (border + handles) is drawn by the
            // unconditional block below, which also handles clamping and cursor
            // rendering. Do NOT draw it here a second time.
        }

        // Draw arrow control handles when: (a) editing controls are active, OR
        // (b) Arrow or Select tool is selected and an existing arrow is selected.
        let show_handles = arrow_editing_controls
            || ((selected_tool == Tool::Arrow || selected_tool == Tool::Select)
                && selected_action
                    .as_ref()
                    .map(|a| matches!(a, AnnotationAction::Arrow { .. }))
                    .unwrap_or(false));

        if show_handles {
            if let Some(AnnotationAction::Arrow {
                control_points: Some(handles),
                color,
                ..
            }) = selected_action.as_ref()
            {
                draw_arrow_control_handles(context, handles.clone(), *color, t.scale);
            }
        }

        // Draw active text edit overlay (border + handles)
        if let Some(bounds) = active_text_bounds.as_ref() {
            let mut bounds = bounds.clone();
            bounds.rect.x = bounds
                .rect
                .x
                .clamp(0, (working_image.width() as i32 - bounds.rect.width).max(0));
            bounds.rect.y = bounds.rect.y.clamp(
                0,
                (working_image.height() as i32 - bounds.rect.height).max(0),
            );
            bounds.sync_handles();
            if let Some(input) = active_text_input.as_ref() {
                let font = crate::capture::editor::types::FontSettings {
                    family: text_font_family.clone(),
                    size: text_size,
                    style: crate::capture::editor::types::FontStyle::Normal,
                    decoration: crate::capture::editor::types::TextDecoration::None,
                    alignment: crate::capture::editor::types::TextAlignment::Left,
                };
                draw_active_text_input(
                    context,
                    &bounds,
                    &input.text,
                    input.cursor_position,
                    input.cursor_visible,
                    input.color,
                    &font,
                );
            }
            draw_text_edit_border(context, &bounds, t.scale);
            draw_text_edit_handles(context, &bounds, active_text_drag_handle.clone(), t.scale);
        }
        let _ = context.restore();
    });
}

fn draw_rounded_rect_path(
    context: &gtk4::cairo::Context,
    width: f64,
    height: f64,
    corner_radius: f64,
    expansion: f64,
) {
    let r = corner_radius + expansion;
    let x = -expansion;
    let y = -expansion;
    let w = width + expansion * 2.0;
    let h = height + expansion * 2.0;

    context.new_sub_path();
    context.arc(x + w - r, y + r, r, -std::f64::consts::FRAC_PI_2, 0.0);
    context.arc(x + w - r, y + h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
    context.arc(
        x + r,
        y + h - r,
        r,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
    );
    context.arc(
        x + r,
        y + r,
        r,
        std::f64::consts::PI,
        std::f64::consts::PI * 1.5,
    );
    context.close_path();
}

#[cfg(test)]
mod tests {
    #[test]
    fn canvas_render_owns_draw_func_caches_and_lock_release() {
        let source = include_str!("canvas_render.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        assert!(
            production.contains("pub(super) struct CanvasRenderCaches")
                && production.contains("pub(super) fn install_canvas_draw_func(")
                && production.contains("drawing_area.set_draw_func(move |_, context, width, height| {")
                && production.contains("IMPORTANT: do not hold the state mutex while performing cairo drawing")
                && production.contains("draw_canvas_checkerboard_background")
                && production.contains("draw_annotation_action")
                && production.contains("draw_crop_overlay")
                && production.contains("draw_arrow_control_handles")
                && production.contains("MAX_PREVIEW_SHADOW_DIM")
                && production.contains("fn draw_rounded_rect_path"),
            "canvas_render.rs must own render caches, set_draw_func, lock-release snapshot, and rounded-rect helper"
        );
    }
}
