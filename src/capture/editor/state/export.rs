use super::super::composition::{BackgroundComposition, CompositionLayout};
use super::super::pen_weight::{HighlighterMode, PenWeight};
use super::super::render::apply_blur_rect;
use super::super::types::{AnnotationAction, BackgroundStyle, EditorError, Rect};
use super::crop::crop_image;
use super::{apply_corner_radius, render_shadow_layer, EditorState};
use image::RgbaImage;
use std::path::Path;

impl EditorState {
    pub fn to_rendered_image(&self) -> Result<RgbaImage, EditorError> {
        let (width, height) = self.working_image.dimensions();
        if width == 0 || height == 0 {
            return Err(EditorError::ImageSave(
                "image has invalid dimensions".into(),
            ));
        }

        let stride = gtk4::cairo::Format::ARgb32
            .stride_for_width(width)
            .map_err(|e| EditorError::ImageSave(e.to_string()))?;

        let data = super::super::render::rgba_to_cairo_argb_bytes(&self.working_image);
        let mut surface = gtk4::cairo::ImageSurface::create_for_data(
            data,
            gtk4::cairo::Format::ARgb32,
            width as i32,
            height as i32,
            stride,
        )
        .map_err(|e| EditorError::ImageSave(e.to_string()))?;

        {
            let context = gtk4::cairo::Context::new(&surface)
                .map_err(|e| EditorError::ImageSave(e.to_string()))?;

            for action in &self.actions {
                if matches!(
                    action,
                    AnnotationAction::Obfuscate { .. } | AnnotationAction::Focus { .. }
                ) {
                    continue;
                }
                super::super::render::draw_annotation_action(&context, action);
            }
        }

        surface.flush();
        let surface_data = surface
            .data()
            .map_err(|e| EditorError::ImageSave(e.to_string()))?;

        Ok(super::super::render::cairo_argb_to_rgba_image(
            width,
            height,
            stride as usize,
            surface_data.as_ref(),
        ))
    }

    pub fn to_final_image(&self) -> Result<RgbaImage, EditorError> {
        let mut rendered = self.to_rendered_image()?;

        if let Some(crop) = self.crop_selection {
            rendered = crop_image(&rendered, crop, self.crop_background_color);
        }

        if self.background_style != BackgroundStyle::None {
            return self.render_with_background(&rendered);
        }

        Ok(rendered)
    }

    fn background_layout_for(&self, screenshot: &RgbaImage) -> CompositionLayout {
        BackgroundComposition::new(screenshot.width() as f64, screenshot.height() as f64)
            .with_style(self.background_style.clone())
            .with_padding(self.background_padding)
            .with_shadow(self.background_shadow)
            .with_insert(self.background_insert)
            .with_alignment(self.background_alignment)
            .with_corner_radius(self.background_corner_radius)
            .with_aspect_ratio(self.background_aspect_ratio)
            .compute()
    }

    fn render_with_background(&self, screenshot: &RgbaImage) -> Result<RgbaImage, EditorError> {
        let layout = self.background_layout_for(screenshot);

        let mut canvas = match &self.background_style {
            BackgroundStyle::PlainColor(color) => {
                let pixel = image::Rgba([
                    (color.r.clamp(0.0, 1.0) * 255.0) as u8,
                    (color.g.clamp(0.0, 1.0) * 255.0) as u8,
                    (color.b.clamp(0.0, 1.0) * 255.0) as u8,
                    (color.a.clamp(0.0, 1.0) * 255.0) as u8,
                ]);
                RgbaImage::from_pixel(
                    layout.canvas_width as u32,
                    layout.canvas_height as u32,
                    pixel,
                )
            }
            BackgroundStyle::Gradient(idx) => {
                let file_name = crate::capture::editor::window::background_panel::BACKGROUND_GRADIENT_PREVIEW_FILES[*idx];
                let path = crate::capture::editor::window::background_panel::background_gradient_asset_path(file_name);
                self.load_and_resize_background(
                    &path,
                    layout.canvas_width as u32,
                    layout.canvas_height as u32,
                )?
            }
            BackgroundStyle::Wallpaper(path) => self.load_and_resize_background(
                path,
                layout.canvas_width as u32,
                layout.canvas_height as u32,
            )?,
            BackgroundStyle::Blurred(blur_idx) => {
                let blur_radius = match blur_idx {
                    0 => 10.0,
                    1 => 35.0,
                    2 => 80.0,
                    _ => 20.0,
                };
                // Match the editor's on-screen preview: downsample the screenshot
                // to <=800px on its longest edge BEFORE blurring, then upscale to
                // canvas size. The on-screen draw path does this for the cached
                // preview surface, but the original save path re-blurred at full
                // resolution which dominated "Done" latency on large captures
                // (4K screenshots could spend 1-2 s just blurring before encode).
                // Blur is intrinsically smooth, so the visible result of
                // downsample -> blur -> upscale is indistinguishable from
                // full-resolution blur.
                const MAX_BLUR_DIM: u32 = 800;
                let (sw, sh) = screenshot.dimensions();
                let mut blurred = if sw > MAX_BLUR_DIM || sh > MAX_BLUR_DIM {
                    let scale = MAX_BLUR_DIM as f64 / (sw.max(sh) as f64);
                    image::imageops::resize(
                        screenshot,
                        ((sw as f64) * scale).round().max(1.0) as u32,
                        ((sh as f64) * scale).round().max(1.0) as u32,
                        image::imageops::FilterType::Triangle,
                    )
                } else {
                    screenshot.clone()
                };
                let (bw, bh) = blurred.dimensions();
                apply_blur_rect(
                    &mut blurred,
                    Rect {
                        x: 0,
                        y: 0,
                        width: bw as i32,
                        height: bh as i32,
                    },
                    blur_radius,
                    false,
                );
                image::imageops::resize(
                    &blurred,
                    layout.canvas_width as u32,
                    layout.canvas_height as u32,
                    image::imageops::FilterType::Triangle,
                )
            }
            BackgroundStyle::None => return Ok(screenshot.clone()),
        };

        let mut final_screenshot = if (layout.draw_scale - 1.0).abs() > 0.001 {
            image::imageops::resize(
                screenshot,
                layout.image_rect.width.round().max(1.0) as u32,
                layout.image_rect.height.round().max(1.0) as u32,
                image::imageops::FilterType::CatmullRom,
            )
        } else {
            screenshot.clone()
        };

        if self.background_corner_radius > 0.0 {
            let radius = self.background_corner_radius * layout.scale_factor * layout.draw_scale;
            apply_corner_radius(&mut final_screenshot, radius);
        }

        if let Some(shadow) = layout.shadow {
            let mut shadow_layer = render_shadow_layer(
                final_screenshot.width(),
                final_screenshot.height(),
                shadow.blur,
                shadow.opacity,
                self.background_corner_radius * layout.scale_factor * layout.draw_scale,
            )?;
            if shadow.blur > 0.0 {
                let shadow_width = shadow_layer.width() as i32;
                let shadow_height = shadow_layer.height() as i32;
                let blur_rect = Rect {
                    x: 0,
                    y: 0,
                    width: shadow_width,
                    height: shadow_height,
                };
                // Apply 3 passes of box blur to approximate Gaussian blur.
                // A single pass produces harsh edges; multiple passes create
                // the smooth falloff expected of a realistic shadow.
                let pass_radius = (shadow.blur / 2.0).max(1.0);
                for _ in 0..3 {
                    apply_blur_rect(&mut shadow_layer, blur_rect, pass_radius, true);
                }
            }
            image::imageops::overlay(
                &mut canvas,
                &shadow_layer,
                shadow.rect.x.round() as i64,
                shadow.rect.y.round() as i64,
            );
        }

        image::imageops::overlay(
            &mut canvas,
            &final_screenshot,
            layout.image_rect.x.round() as i64,
            layout.image_rect.y.round() as i64,
        );

        Ok(canvas)
    }

    fn load_and_resize_background(
        &self,
        path: &Path,
        width: u32,
        height: u32,
    ) -> Result<RgbaImage, EditorError> {
        let img = image::open(path).map_err(|e| EditorError::ImageLoad(e.to_string()))?;
        let rgba = img.into_rgba8();
        Ok(image::imageops::resize(
            &rgba,
            width,
            height,
            image::imageops::FilterType::Triangle,
        ))
    }

    pub fn set_highlighter_mode(&mut self, mode: HighlighterMode) {
        self.highlighter_mode = mode;
    }

    pub fn set_pen_weight(&mut self, weight: PenWeight) {
        self.pen_weight = weight;
    }
}
