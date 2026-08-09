use super::icons::{ToolbarIcon, TOOLBAR_ICONS};
use super::layout::*;
// Recording-specific hit-testing lives in recording/hit_testing.rs

pub(crate) fn capture_crop_menu_hit_item(
    selection_x: f64,
    selection_y: f64,
    selection_width: f64,
    selection_height: f64,
    screen_width: f64,
    screen_height: f64,
    x: f64,
    y: f64,
) -> Option<usize> {
    let layout = compute_toolbar_layout(
        selection_x,
        selection_y,
        selection_width,
        selection_height,
        screen_width,
        screen_height,
    );
    let anchor = layout.crop_panel;
    let (_panel, items) = compute_aspect_menu_rects(anchor, screen_width, screen_height);
    items.iter().position(|r| r.contains(x, y))
}

pub(crate) fn toolbar_item_at(
    selection_x: f64,
    selection_y: f64,
    selection_width: f64,
    selection_height: f64,
    screen_width: f64,
    screen_height: f64,
    x: f64,
    y: f64,
) -> Option<ToolbarIcon> {
    match toolbar_hit_at(
        selection_x,
        selection_y,
        selection_width,
        selection_height,
        screen_width,
        screen_height,
        x,
        y,
    ) {
        Some(ToolbarHit::Tool(index)) => TOOLBAR_ICONS.get(index).copied(),
        _ => None,
    }
}

pub(crate) fn toolbar_hit_at(
    selection_x: f64,
    selection_y: f64,
    selection_width: f64,
    selection_height: f64,
    screen_width: f64,
    screen_height: f64,
    x: f64,
    y: f64,
) -> Option<ToolbarHit> {
    let layout = compute_toolbar_layout(
        selection_x,
        selection_y,
        selection_width,
        selection_height,
        screen_width,
        screen_height,
    );

    for (index, cell) in layout.item_cells.iter().enumerate() {
        if cell.contains(x, y) {
            // Never return a tool index outside TOOLBAR_ICONS.
            if index < TOOLBAR_ICONS.len() {
                return Some(ToolbarHit::Tool(index));
            }
            return None;
        }
    }

    if layout.size_panel.contains(x, y) {
        return Some(ToolbarHit::SizePanel);
    }
    if layout.crop_panel.contains(x, y) {
        return Some(ToolbarHit::CropPanel);
    }

    None
}

pub(crate) fn capture_crop_menu_contains(
    selection_x: f64,
    selection_y: f64,
    selection_width: f64,
    selection_height: f64,
    screen_width: f64,
    screen_height: f64,
    x: f64,
    y: f64,
) -> bool {
    let layout = compute_toolbar_layout(
        selection_x,
        selection_y,
        selection_width,
        selection_height,
        screen_width,
        screen_height,
    );
    let anchor = layout.crop_panel;
    let (panel, _items) = compute_aspect_menu_rects(anchor, screen_width, screen_height);
    panel.contains(x, y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overlay::icons::{
        TOOLBAR_AREA_INDEX, TOOLBAR_ICONS, TOOLBAR_OCR_INDEX, TOOLBAR_TIMER_INDEX,
    };

    fn hit_on_cell(cell_index: usize) -> Option<ToolbarHit> {
        let layout = compute_toolbar_layout(200.0, 200.0, 400.0, 300.0, 1920.0, 1080.0);
        assert!(
            cell_index < layout.item_cells.len(),
            "test cell index must exist in layout"
        );
        let cell = layout.item_cells[cell_index];
        let x = cell.x + cell.width / 2.0;
        let y = cell.y + cell.height / 2.0;
        toolbar_hit_at(200.0, 200.0, 400.0, 300.0, 1920.0, 1080.0, x, y)
    }

    #[test]
    fn toolbar_hit_returns_only_valid_icon_indices() {
        for i in 0..TOOLBAR_ICONS.len() {
            match hit_on_cell(i) {
                Some(ToolbarHit::Tool(index)) => {
                    assert_eq!(index, i);
                    assert!(index < TOOLBAR_ICONS.len());
                    assert!(toolbar_item_at(
                        200.0,
                        200.0,
                        400.0,
                        300.0,
                        1920.0,
                        1080.0,
                        {
                            let layout =
                                compute_toolbar_layout(200.0, 200.0, 400.0, 300.0, 1920.0, 1080.0);
                            let cell = layout.item_cells[i];
                            cell.x + cell.width / 2.0
                        },
                        {
                            let layout =
                                compute_toolbar_layout(200.0, 200.0, 400.0, 300.0, 1920.0, 1080.0);
                            let cell = layout.item_cells[i];
                            cell.y + cell.height / 2.0
                        }
                    )
                    .is_some());
                }
                other => panic!("expected Tool({i}), got {other:?}"),
            }
        }
    }

    #[test]
    fn toolbar_hit_never_returns_index_outside_toolbar_icons() {
        // Sample a dense grid over the tools panel plus a band below it where a
        // phantom 7th cell used to live. No hit may index past TOOLBAR_ICONS.
        let layout = compute_toolbar_layout(200.0, 200.0, 400.0, 300.0, 1920.0, 1080.0);
        let panel = layout.tools_panel;
        let mut y = panel.y - 20.0;
        while y <= panel.y + panel.height + FEATURE_PANEL_HEIGHT + 20.0 {
            let mut x = panel.x - 10.0;
            while x <= panel.x + panel.width + 10.0 {
                if let Some(ToolbarHit::Tool(index)) =
                    toolbar_hit_at(200.0, 200.0, 400.0, 300.0, 1920.0, 1080.0, x, y)
                {
                    assert!(
                        index < TOOLBAR_ICONS.len(),
                        "toolbar hit returned out-of-range index {index} at ({x},{y})"
                    );
                    assert!(
                        TOOLBAR_ICONS.get(index).is_some(),
                        "toolbar_item_at must resolve icon for index {index}"
                    );
                }
                x += 4.0;
            }
            y += 4.0;
        }
    }

    #[test]
    fn toolbar_timer_and_ocr_indices_are_distinct() {
        assert_eq!(TOOLBAR_TIMER_INDEX, 3);
        assert_eq!(TOOLBAR_OCR_INDEX, 4);
        assert_ne!(TOOLBAR_TIMER_INDEX, TOOLBAR_OCR_INDEX);
        assert_eq!(hit_on_cell(TOOLBAR_TIMER_INDEX), Some(ToolbarHit::Tool(3)));
        assert_eq!(hit_on_cell(TOOLBAR_OCR_INDEX), Some(ToolbarHit::Tool(4)));
        assert_eq!(hit_on_cell(TOOLBAR_AREA_INDEX), Some(ToolbarHit::Tool(0)));
    }
}
