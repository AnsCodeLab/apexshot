use super::icons::TOOLBAR_ICONS;

pub(crate) const DEFAULT_SELECTION_WIDTH: f64 = 600.0;
pub(crate) const DEFAULT_SELECTION_HEIGHT: f64 = 744.0;
pub(crate) const MIN_SELECTION_WIDTH: f64 = 24.0;
pub(crate) const MIN_SELECTION_HEIGHT: f64 = 24.0;
pub(crate) const BORDER_HANDLE_THRESHOLD: f64 = 10.0;
pub(crate) const HANDLE_MARKER_LENGTH: f64 = 20.0;
pub(crate) const HANDLE_MARKER_THICKNESS: f64 = 2.5;
pub(crate) const BRAND_ORANGE_R: f64 = 1.0;
pub(crate) const BRAND_ORANGE_G: f64 = 0.4;
pub(crate) const BRAND_ORANGE_B: f64 = 0.0;
pub(crate) const FEATURE_PANEL_ITEM_WIDTH: f64 = 76.0;
pub(crate) const FEATURE_PANEL_HEIGHT: f64 = 62.0;
pub(crate) const FEATURE_PANEL_RADIUS: f64 = 13.0;
pub(crate) const FEATURE_PANEL_TOP_GAP: f64 = 12.0;
pub(crate) const FEATURE_PANEL_MARGIN: f64 = 16.0;
pub(crate) const TOOL_RAIL_GAP: f64 = 18.0;
pub(crate) const ACTION_CARD_GAP: f64 = 8.0;
pub(crate) const SIZE_CARD_WIDTH: f64 = 152.0;
pub(crate) const SIZE_CARD_HEIGHT: f64 = 56.0;
pub(crate) const CROP_CARD_WIDTH: f64 = 62.0;

/// Must stay equal to `TOOLBAR_ICONS.len()` — used for cell array sizing.
pub(crate) const TOOLBAR_TOOL_COUNT: usize = TOOLBAR_ICONS.len();

#[derive(Debug, Clone, Copy)]
pub(crate) struct RectF {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

impl RectF {
    pub(crate) fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x && px <= self.x + self.width && py >= self.y && py <= self.y + self.height
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ToolbarLayout {
    pub(crate) tools_panel: RectF,
    pub(crate) size_panel: RectF,
    pub(crate) crop_panel: RectF,
    /// One cell per entry in `TOOLBAR_ICONS` (never more).
    pub(crate) item_cells: [RectF; TOOLBAR_TOOL_COUNT],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolbarHit {
    Tool(usize),
    SizePanel,
    CropPanel,
}

pub(crate) const ASPECT_RATIO_OPTIONS: &[&str] = &[
    "Freeform",
    "1 : 1 (Square)",
    "5 : 4 (10 : 8)",
    "4 : 3",
    "7 : 5",
    "3 : 2",
    "16 : 10",
    "16 : 9",
    "2.35 : 1",
    "2 : 3",
    "9 : 16",
];

pub(crate) fn compute_toolbar_layout(
    selection_x: f64,
    selection_y: f64,
    selection_width: f64,
    selection_height: f64,
    screen_width: f64,
    screen_height: f64,
) -> ToolbarLayout {
    let tool_panel_height = FEATURE_PANEL_HEIGHT * TOOLBAR_TOOL_COUNT as f64;
    let center_y = selection_y + selection_height / 2.0;
    let tool_x = (selection_x - TOOL_RAIL_GAP - FEATURE_PANEL_ITEM_WIDTH).max(FEATURE_PANEL_MARGIN);
    let tool_y = (center_y - tool_panel_height / 2.0).clamp(
        FEATURE_PANEL_MARGIN,
        (screen_height - tool_panel_height - FEATURE_PANEL_MARGIN).max(FEATURE_PANEL_MARGIN),
    );

    let tools_panel = RectF {
        x: tool_x,
        y: tool_y,
        width: FEATURE_PANEL_ITEM_WIDTH,
        height: tool_panel_height,
    };

    let top_width = SIZE_CARD_WIDTH + ACTION_CARD_GAP + CROP_CARD_WIDTH;
    let top_x = (selection_x + (selection_width - top_width) / 2.0).clamp(
        FEATURE_PANEL_MARGIN,
        (screen_width - top_width - FEATURE_PANEL_MARGIN).max(FEATURE_PANEL_MARGIN),
    );
    let top_y = (selection_y - FEATURE_PANEL_TOP_GAP - SIZE_CARD_HEIGHT).clamp(
        FEATURE_PANEL_MARGIN,
        (screen_height - SIZE_CARD_HEIGHT - FEATURE_PANEL_MARGIN).max(FEATURE_PANEL_MARGIN),
    );

    let size_panel = RectF {
        x: top_x,
        y: top_y,
        width: SIZE_CARD_WIDTH,
        height: SIZE_CARD_HEIGHT,
    };
    let crop_panel = RectF {
        x: top_x + SIZE_CARD_WIDTH + ACTION_CARD_GAP,
        y: top_y,
        width: CROP_CARD_WIDTH,
        height: SIZE_CARD_HEIGHT,
    };

    let mut item_cells = [RectF {
        x: 0.0,
        y: 0.0,
        width: FEATURE_PANEL_ITEM_WIDTH,
        height: FEATURE_PANEL_HEIGHT,
    }; TOOLBAR_TOOL_COUNT];

    for (index, cell) in item_cells.iter_mut().enumerate() {
        cell.x = tools_panel.x;
        cell.y = tools_panel.y + index as f64 * FEATURE_PANEL_HEIGHT;
    }

    ToolbarLayout {
        tools_panel,
        size_panel,
        crop_panel,
        item_cells,
    }
}

pub(crate) fn compute_aspect_menu_rects(
    anchor_rect: RectF,
    screen_width: f64,
    screen_height: f64,
) -> (RectF, Vec<RectF>) {
    let item_h = 34.0;
    let menu_w = 196.0;
    let menu_h = (ASPECT_RATIO_OPTIONS.len() as f64 * item_h) + 10.0;
    let menu_x = (anchor_rect.x + anchor_rect.width / 2.0 - menu_w / 2.0)
        .clamp(10.0, screen_width - menu_w - 10.0);
    let menu_y =
        (anchor_rect.y + anchor_rect.height + 8.0).clamp(10.0, screen_height - menu_h - 10.0);
    let panel_rect = RectF {
        x: menu_x,
        y: menu_y,
        width: menu_w,
        height: menu_h,
    };
    let mut item_rects = Vec::with_capacity(ASPECT_RATIO_OPTIONS.len());
    for i in 0..ASPECT_RATIO_OPTIONS.len() {
        item_rects.push(RectF {
            x: menu_x + 5.0,
            y: menu_y + 5.0 + i as f64 * item_h,
            width: menu_w - 10.0,
            height: item_h,
        });
    }
    (panel_rect, item_rects)
}

// ── Shared popup / menu layouts (drawing + hit-testing consume these) ─────────

pub(crate) const SCROLL_POPUP_WIDTH: f64 = 360.0;
pub(crate) const SCROLL_POPUP_HEIGHT: f64 = 170.0;
pub(crate) const SCROLL_POPUP_CLOSE_SIZE: f64 = 22.0;
pub(crate) const SCROLL_POPUP_DOWNLOAD_W: f64 = 182.0;
pub(crate) const SCROLL_POPUP_DOWNLOAD_H: f64 = 34.0;

#[derive(Debug, Clone, Copy)]
pub(crate) struct ScrollPopupLayout {
    pub(crate) panel: RectF,
    pub(crate) close: RectF,
    pub(crate) download: RectF,
}

/// Centered scroll-capture extension CTA. Drawing and hit-testing share this.
pub(crate) fn compute_scroll_popup_layout(center_x: f64, center_y: f64) -> ScrollPopupLayout {
    let panel = RectF {
        x: center_x - SCROLL_POPUP_WIDTH / 2.0,
        y: center_y - SCROLL_POPUP_HEIGHT / 2.0,
        width: SCROLL_POPUP_WIDTH,
        height: SCROLL_POPUP_HEIGHT,
    };
    let close = RectF {
        x: panel.x + panel.width - SCROLL_POPUP_CLOSE_SIZE - 10.0,
        y: panel.y + 10.0,
        width: SCROLL_POPUP_CLOSE_SIZE,
        height: SCROLL_POPUP_CLOSE_SIZE,
    };
    let download = RectF {
        x: panel.x + (panel.width - SCROLL_POPUP_DOWNLOAD_W) / 2.0,
        y: panel.y + 102.0,
        width: SCROLL_POPUP_DOWNLOAD_W,
        height: SCROLL_POPUP_DOWNLOAD_H,
    };
    ScrollPopupLayout {
        panel,
        close,
        download,
    }
}

pub(crate) const WINDOW_PICKER_MENU_W: f64 = 320.0;
pub(crate) const WINDOW_PICKER_ITEM_H: f64 = 28.0;
pub(crate) const WINDOW_PICKER_HEADER_H: f64 = 30.0;
pub(crate) const WINDOW_PICKER_PAD: f64 = 8.0;

#[derive(Debug, Clone, Copy)]
pub(crate) struct WindowPickerLayout {
    pub(crate) panel: RectF,
    pub(crate) list_y: f64,
    pub(crate) item_h: f64,
}

pub(crate) fn compute_window_picker_layout(
    center_x: f64,
    center_y: f64,
    screen_width: f64,
    screen_height: f64,
    window_count: usize,
) -> WindowPickerLayout {
    let total_h = WINDOW_PICKER_PAD * 2.0
        + WINDOW_PICKER_HEADER_H
        + window_count as f64 * WINDOW_PICKER_ITEM_H;
    let popup_x = (center_x - WINDOW_PICKER_MENU_W / 2.0)
        .clamp(10.0, (screen_width - WINDOW_PICKER_MENU_W - 10.0).max(10.0));
    let popup_y =
        (center_y - total_h / 2.0).clamp(10.0, (screen_height - total_h - 10.0).max(10.0));
    WindowPickerLayout {
        panel: RectF {
            x: popup_x,
            y: popup_y,
            width: WINDOW_PICKER_MENU_W,
            height: total_h,
        },
        list_y: popup_y + WINDOW_PICKER_PAD + WINDOW_PICKER_HEADER_H,
        item_h: WINDOW_PICKER_ITEM_H,
    }
}

pub(crate) const VOLUME_POPUP_WIDTH: f64 = 280.0;
pub(crate) const VOLUME_POPUP_HEIGHT: f64 = 130.0;
pub(crate) const VOLUME_SLIDER_OFFSET_X: f64 = 83.0;
pub(crate) const VOLUME_SLIDER_WIDTH: f64 = 140.0;
pub(crate) const VOLUME_SLIDER_TRACK_H: f64 = 6.0;
pub(crate) const VOLUME_SLIDER_ROW_Y: f64 = 78.0;
pub(crate) const VOLUME_SLIDER_ROW_H: f64 = 46.0;

#[derive(Debug, Clone, Copy)]
pub(crate) struct VolumePopupLayout {
    pub(crate) panel: RectF,
    pub(crate) slider_x: f64,
    pub(crate) slider_w: f64,
    pub(crate) track_y: f64,
    pub(crate) slider_track_h: f64,
}

/// Volume popup anchored like the settings menu (selection top-centre).
pub(crate) fn compute_volume_popup_layout(
    selection_x: f64,
    selection_y: f64,
    selection_width: f64,
    screen_width: f64,
    screen_height: f64,
) -> VolumePopupLayout {
    let menu_w = VOLUME_POPUP_WIDTH;
    let menu_h = VOLUME_POPUP_HEIGHT;
    let menu_x =
        (selection_x + (selection_width - menu_w) / 2.0).clamp(10.0, screen_width - menu_w - 10.0);
    let menu_y = (selection_y + 24.0).clamp(10.0, screen_height - menu_h - 10.0);
    let row_y = menu_y + VOLUME_SLIDER_ROW_Y;
    let track_y = row_y + (VOLUME_SLIDER_ROW_H - VOLUME_SLIDER_TRACK_H) / 2.0;
    VolumePopupLayout {
        panel: RectF {
            x: menu_x,
            y: menu_y,
            width: menu_w,
            height: menu_h,
        },
        slider_x: menu_x + VOLUME_SLIDER_OFFSET_X,
        slider_w: VOLUME_SLIDER_WIDTH,
        track_y,
        slider_track_h: VOLUME_SLIDER_TRACK_H,
    }
}

pub(crate) const SETTINGS_MENU_WIDTH: f64 = 440.0;
pub(crate) const SETTINGS_MENU_HEIGHT: f64 = 560.0;

#[derive(Debug, Clone, Copy)]
pub(crate) struct SettingsMenuLayout {
    pub(crate) panel: RectF,
}

/// Settings menu position (C++ contextual menu): selection top-centre, clamped.
pub(crate) fn compute_settings_menu_layout(
    selection_x: f64,
    selection_y: f64,
    selection_width: f64,
    screen_width: f64,
    screen_height: f64,
) -> SettingsMenuLayout {
    let menu_w = SETTINGS_MENU_WIDTH;
    let menu_h = SETTINGS_MENU_HEIGHT;
    // Keep the historical clamp constants (screen_w - 450 / screen_h - 570)
    // so drawing and hit-testing stay bit-identical to the prior inline math.
    let menu_x = (selection_x + (selection_width - menu_w) / 2.0).clamp(10.0, screen_width - 450.0);
    let menu_y = (selection_y + 24.0).clamp(10.0, screen_height - 570.0);
    SettingsMenuLayout {
        panel: RectF {
            x: menu_x,
            y: menu_y,
            width: menu_w,
            height: menu_h,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overlay::icons::TOOLBAR_ICONS;

    #[test]
    fn toolbar_tool_count_matches_icons() {
        assert_eq!(TOOLBAR_TOOL_COUNT, TOOLBAR_ICONS.len());
        assert_eq!(TOOLBAR_TOOL_COUNT, 6);
    }

    #[test]
    fn toolbar_layout_item_cells_match_icon_count() {
        let layout = compute_toolbar_layout(200.0, 200.0, 400.0, 300.0, 1920.0, 1080.0);
        assert_eq!(layout.item_cells.len(), TOOLBAR_ICONS.len());
        assert_eq!(
            layout.tools_panel.height,
            FEATURE_PANEL_HEIGHT * TOOLBAR_ICONS.len() as f64
        );
        // Every cell is stacked inside the tools panel.
        for (i, cell) in layout.item_cells.iter().enumerate() {
            assert!(
                (cell.y - (layout.tools_panel.y + i as f64 * FEATURE_PANEL_HEIGHT)).abs() < 1e-9
            );
            assert!((cell.x - layout.tools_panel.x).abs() < 1e-9);
            let cell_bottom = cell.y + cell.height;
            let panel_bottom = layout.tools_panel.y + layout.tools_panel.height;
            assert!(
                cell_bottom <= panel_bottom + 1e-9,
                "cell {i} extends past tools panel"
            );
        }
    }

    #[test]
    fn scroll_popup_layout_has_close_and_download_inside_panel() {
        let layout = compute_scroll_popup_layout(500.0, 400.0);
        assert!(layout
            .panel
            .contains(layout.close.x + 1.0, layout.close.y + 1.0));
        assert!(layout
            .panel
            .contains(layout.download.x + 1.0, layout.download.y + 1.0));
        assert_eq!(layout.panel.width, SCROLL_POPUP_WIDTH);
        assert_eq!(layout.panel.height, SCROLL_POPUP_HEIGHT);
    }

    #[test]
    fn window_picker_layout_scales_with_entry_count() {
        let one = compute_window_picker_layout(500.0, 400.0, 1920.0, 1080.0, 1);
        let three = compute_window_picker_layout(500.0, 400.0, 1920.0, 1080.0, 3);
        assert!((three.panel.height - one.panel.height - 2.0 * WINDOW_PICKER_ITEM_H).abs() < 1e-9);
        assert_eq!(one.item_h, WINDOW_PICKER_ITEM_H);
    }

    #[test]
    fn volume_and_settings_layouts_share_selection_anchor_style() {
        let vol = compute_volume_popup_layout(100.0, 200.0, 600.0, 1920.0, 1080.0);
        let settings = compute_settings_menu_layout(100.0, 200.0, 600.0, 1920.0, 1080.0);
        // Both are top-of-selection anchored; Y starts at selection_y + 24 when unclamped.
        assert!((vol.panel.y - 224.0).abs() < 1e-9);
        assert!((settings.panel.y - 224.0).abs() < 1e-9);
        assert_eq!(vol.panel.width, VOLUME_POPUP_WIDTH);
        assert_eq!(settings.panel.width, SETTINGS_MENU_WIDTH);
        assert!(vol.slider_w > 0.0);
    }
}
