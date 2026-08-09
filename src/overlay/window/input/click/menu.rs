//! Open menu and popup click state transitions.

use super::ClickEffect;
use crate::overlay::geometry::{
    apply_aspect_to_selection, aspect_ratio_for_index, current_selection_rect,
};
use crate::overlay::hit_testing::{capture_crop_menu_contains, capture_crop_menu_hit_item};
use crate::overlay::layout::{
    compute_scroll_popup_layout, compute_volume_popup_layout, compute_window_picker_layout, RectF,
};
use crate::overlay::recording::hit_testing::{
    recording_crop_menu_contains, recording_crop_menu_hit_item, settings_menu_contains,
    settings_menu_hit_item,
};
use crate::overlay::recording::layout::compute_dropdown_popup_y;
use crate::overlay::recording::state::SettingsTab;
use crate::overlay::state::SelectorState;

pub(super) fn handle_menu_click(
    st: &mut SelectorState,
    x: f64,
    y: f64,
    screen_width: i32,
    screen_height: i32,
) -> Option<ClickEffect> {
    let rect = current_selection_rect(st);

    if st.countdown_active
        && x >= st.countdown_bubble_x
        && x <= st.countdown_bubble_x + st.countdown_bubble_w
        && y >= st.countdown_bubble_y
        && y <= st.countdown_bubble_y + st.countdown_bubble_h
    {
        st.countdown_cancel_requested = true;
        return Some(ClickEffect::Redraw);
    }

    if st.capture_crop_menu_open {
        if let Some(item) = capture_crop_menu_hit_item(
            rect.left,
            rect.top,
            rect.width(),
            rect.height(),
            screen_width as f64,
            screen_height as f64,
            x,
            y,
        ) {
            st.capture_aspect_ratio_index = item;
            apply_aspect_to_selection(
                st,
                aspect_ratio_for_index(item),
                screen_width as f64,
                screen_height as f64,
            );
            st.capture_crop_menu_open = false;
            st.hovered_capture_crop_menu_item = -1;
            return Some(ClickEffect::Redraw);
        }
        if capture_crop_menu_contains(
            rect.left,
            rect.top,
            rect.width(),
            rect.height(),
            screen_width as f64,
            screen_height as f64,
            x,
            y,
        ) {
            return Some(ClickEffect::None);
        }
        st.capture_crop_menu_open = false;
        st.hovered_capture_crop_menu_item = -1;
        return Some(ClickEffect::Redraw);
    }

    if st.recording.crop_menu_open {
        if let Some(item) = recording_crop_menu_hit_item(
            rect.left,
            rect.top,
            rect.width(),
            rect.height(),
            screen_width as f64,
            screen_height as f64,
            x,
            y,
        ) {
            st.recording.record_aspect_ratio_index = item;
            apply_aspect_to_selection(
                st,
                aspect_ratio_for_index(item),
                screen_width as f64,
                screen_height as f64,
            );
            st.recording.crop_menu_open = false;
            st.recording.hovered_crop_menu_item = -1;
            return Some(ClickEffect::Redraw);
        }
        if recording_crop_menu_contains(
            rect.left,
            rect.top,
            rect.width(),
            rect.height(),
            screen_width as f64,
            screen_height as f64,
            x,
            y,
        ) {
            return Some(ClickEffect::None);
        }
        st.recording.crop_menu_open = false;
        st.recording.hovered_crop_menu_item = -1;
        return Some(ClickEffect::Redraw);
    }

    if st.recording.settings_menu_open {
        return Some(handle_settings_menu_click(
            st,
            rect,
            x,
            y,
            screen_width,
            screen_height,
        ));
    }

    if st.scroll_popup_open {
        let (cx, cy) = popup_center(st, screen_width, screen_height);
        let scroll = compute_scroll_popup_layout(cx, cy);
        if scroll.close.contains(x, y) {
            close_scroll_popup(st);
            return Some(ClickEffect::Redraw);
        }
        if scroll.download.contains(x, y) {
            close_scroll_popup(st);
            return Some(ClickEffect::OpenScrollExtension);
        }
        if !scroll.panel.contains(x, y) {
            close_scroll_popup(st);
            return Some(ClickEffect::Redraw);
        }
    }

    if st.window_picker_open {
        let n = st.windows.len();
        let (center_x, center_y) = popup_center(st, screen_width, screen_height);
        let picker = compute_window_picker_layout(
            center_x,
            center_y,
            screen_width as f64,
            screen_height as f64,
            n,
        );
        if !picker.panel.contains(x, y) {
            st.window_picker_open = false;
            st.hovered_window_picker_entry = -1;
            return Some(ClickEffect::Redraw);
        }
        let entry_index = ((y - picker.list_y) / picker.item_h) as i32;
        if entry_index >= 0 && (entry_index as usize) < st.windows.len() {
            let win = st.windows[entry_index as usize].clone();
            st.start_x = win.x as f64;
            st.start_y = win.y as f64;
            st.current_x = (win.x + win.width) as f64;
            st.current_y = (win.y + win.height) as f64;
            st.completed = true;
            st.window_picker_open = false;
            st.hovered_window_picker_entry = -1;
            return Some(ClickEffect::SendSelection);
        }
        return Some(ClickEffect::None);
    }

    if st.recording.mic_volume_popup_open || st.recording.speaker_volume_popup_open {
        let vol = compute_volume_popup_layout(
            rect.left,
            rect.top,
            rect.width(),
            screen_width as f64,
            screen_height as f64,
        );
        if vol.panel.contains(x, y) {
            if y >= vol.track_y - 13.0
                && y <= vol.track_y + vol.slider_track_h + 13.0
                && x >= vol.slider_x
                && x <= vol.slider_x + vol.slider_w
            {
                let fraction = ((x - vol.slider_x) / vol.slider_w).clamp(0.0, 1.0);
                st.recording.volume_slider_dragging = true;
                if st.recording.mic_volume_popup_open {
                    st.recording.mic_volume = fraction;
                    return Some(ClickEffect::SetMicVolume(fraction));
                }
                st.recording.speaker_volume = fraction;
                return Some(ClickEffect::SetSpeakerVolume(fraction));
            }
            return Some(ClickEffect::Redraw);
        }
        st.recording.mic_volume_popup_open = false;
        st.recording.speaker_volume_popup_open = false;
        st.recording.volume_slider_dragging = false;
        return Some(ClickEffect::Redraw);
    }

    None
}

fn handle_settings_menu_click(
    st: &mut SelectorState,
    rect: crate::overlay::geometry::SelectionRectF,
    x: f64,
    y: f64,
    screen_width: i32,
    screen_height: i32,
) -> ClickEffect {
    if let Some(drop_idx) = st.recording.settings_dropdown_open {
        let tab = match st.recording.settings_tab {
            SettingsTab::Video => 1,
            SettingsTab::Gif => 2,
            _ => 0,
        };
        let (option_count, value_ptr): (usize, &mut usize) = if tab == 1 && drop_idx == 3 {
            (3, &mut st.recording.video_max_res)
        } else if tab == 1 && drop_idx == 4 {
            (4, &mut st.recording.video_fps)
        } else if tab == 2 && drop_idx == 6 {
            (4, &mut st.recording.gif_size_idx)
        } else {
            (0, &mut 0)
        };
        let menu_x =
            (rect.left + (rect.width() - 440.0) / 2.0).clamp(10.0, screen_width as f64 - 450.0);
        let menu_y = (rect.top + 24.0).clamp(10.0, screen_height as f64 - 570.0);
        let popup_y = compute_dropdown_popup_y(
            menu_y,
            drop_idx,
            match tab {
                1 => SettingsTab::Video,
                2 => SettingsTab::Gif,
                _ => SettingsTab::General,
            },
        );
        let popup_rect = RectF {
            x: menu_x + 130.0,
            y: popup_y,
            width: 140.0,
            height: option_count as f64 * 30.0,
        };
        if popup_rect.contains(x, y) {
            for option_index in 0..option_count {
                let item_rect = RectF {
                    x: popup_rect.x,
                    y: popup_rect.y + option_index as f64 * 30.0,
                    width: popup_rect.width,
                    height: 30.0,
                };
                if item_rect.contains(x, y) {
                    *value_ptr = option_index;
                    st.recording.hovered_settings_item = -1;
                    break;
                }
            }
        }
        st.recording.settings_dropdown_open = None;
        return ClickEffect::Redraw;
    }

    if let Some(item) = settings_menu_hit_item(
        rect.left,
        rect.top,
        rect.width(),
        rect.height(),
        screen_width as f64,
        screen_height as f64,
        x,
        y,
        st.recording.settings_tab,
    ) {
        if item < 3 {
            st.recording.settings_tab = match item {
                0 => SettingsTab::General,
                1 => SettingsTab::Video,
                _ => SettingsTab::Gif,
            };
            st.recording.settings_dropdown_open = None;
        } else if matches!(st.recording.settings_tab, SettingsTab::General) {
            match item - 3 {
                0 => st.recording.rec_controls = !st.recording.rec_controls,
                1 => st.recording.hidpi = !st.recording.hidpi,
                2 => st.recording.do_not_disturb = !st.recording.do_not_disturb,
                3 => st.recording.remember_selection = !st.recording.remember_selection,
                4 => st.recording.dim_screen = !st.recording.dim_screen,
                5 => st.recording.show_countdown = !st.recording.show_countdown,
                _ => {}
            }
            st.recording.settings_dropdown_open = None;
        } else if matches!(st.recording.settings_tab, SettingsTab::Video) {
            match item - 3 {
                0 => st.recording.settings_dropdown_open = Some(3),
                1 => st.recording.settings_dropdown_open = Some(4),
                2 => st.recording.record_mono = !st.recording.record_mono,
                3 => st.recording.open_editor = !st.recording.open_editor,
                _ => {}
            }
        } else if matches!(st.recording.settings_tab, SettingsTab::Gif) {
            let menu_x =
                (rect.left + (rect.width() - 440.0) / 2.0).clamp(10.0, screen_width as f64 - 450.0);
            let value_x = menu_x + 130.0;
            match item - 3 {
                0 => {
                    let slider_x = value_x + 55.0;
                    let slider_w = 220.0;
                    let click_x = x.clamp(slider_x, slider_x + slider_w);
                    st.recording.gif_fps = 5.0 + (click_x - slider_x) / slider_w * 55.0;
                    st.recording.gif_slider_dragging = Some(0);
                }
                1 => {
                    let slider_w = 160.0;
                    let click_x = x.clamp(value_x, value_x + slider_w);
                    st.recording.gif_quality = ((click_x - value_x) / slider_w).clamp(0.0, 1.0);
                    st.recording.gif_slider_dragging = Some(1);
                }
                2 => st.recording.optimize_gif = !st.recording.optimize_gif,
                3 => st.recording.settings_dropdown_open = Some(6),
                _ => {}
            }
        }
        st.recording.hovered_settings_item = -1;
        return ClickEffect::Redraw;
    }

    if settings_menu_contains(
        rect.left,
        rect.top,
        rect.width(),
        screen_width as f64,
        screen_height as f64,
        x,
        y,
    ) {
        return ClickEffect::None;
    }
    st.recording.settings_menu_open = false;
    st.recording.hovered_settings_item = -1;
    st.recording.settings_dropdown_open = None;
    ClickEffect::Redraw
}

fn popup_center(st: &SelectorState, screen_width: i32, screen_height: i32) -> (f64, f64) {
    if st.completed || st.is_dragging {
        let rect = current_selection_rect(st);
        (
            rect.left + rect.width() / 2.0,
            rect.top + rect.height() / 2.0,
        )
    } else {
        (screen_width as f64 / 2.0, screen_height as f64 / 2.0)
    }
}

fn close_scroll_popup(st: &mut SelectorState) {
    st.scroll_popup_open = false;
    st.hovered_scroll_popup_close = false;
    st.hovered_scroll_download = false;
}

#[cfg(test)]
mod tests {
    #[test]
    fn menu_owner_covers_all_open_click_surfaces() {
        let source = include_str!("menu.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production menu owner");
        for surface in [
            "capture_crop_menu_open",
            "crop_menu_open",
            "settings_menu_open",
            "scroll_popup_open",
            "window_picker_open",
            "mic_volume_popup_open",
            "speaker_volume_popup_open",
        ] {
            assert!(
                production.contains(surface),
                "missing menu surface {surface}"
            );
        }
        assert!(
            production.contains("ClickEffect::OpenScrollExtension")
                && production.contains("ClickEffect::SendSelection")
                && production.contains("ClickEffect::SetMicVolume")
                && production.contains("ClickEffect::SetSpeakerVolume"),
            "menu state owner must return external effects instead of performing them"
        );
        assert!(
            !production.contains("open_url")
                && !production.contains("queue_draw")
                && !production.contains("send_selection_result"),
            "menu owner must remain free of URL/result/GTK effects"
        );
    }
}
