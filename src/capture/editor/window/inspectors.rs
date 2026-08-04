use gtk4::{prelude::*, Box as GtkBox, Button, Label, Orientation, Stack, Widget};

use super::background_panel::BACKGROUND_SIDEBAR_WIDTH;

pub(super) struct InspectorParts {
    pub inspector_tabs: GtkBox,
    pub background_tab_btn: Button,
    pub colors_tab_btn: Button,
    pub inspector: GtkBox,
    pub inspector_stack: Stack,
}

pub(super) struct InspectorContentInputs<'a> {
    pub select_status_label: &'a Label,
    pub select_detail_label: &'a Label,
    pub select_geometry_label: &'a Label,
    pub select_hint_label: &'a Label,
    pub crop_dimensions_group: &'a GtkBox,
    pub crop_ratio_list: &'a GtkBox,
    pub crop_actions_group: &'a GtkBox,
    pub pen_inspector_list: &'a GtkBox,
    pub arrow_style_list: &'a GtkBox,
    pub arrow_thickness_list: &'a GtkBox,
    pub arrow_behavior_group: &'a GtkBox,
    pub line_inspector_list: &'a GtkBox,
    pub text_size_list: &'a GtkBox,
    pub font_family_list: &'a GtkBox,
    pub obfuscate_method_list: &'a GtkBox,
    pub number_options_list: &'a GtkBox,
    pub number_start_row: &'a GtkBox,
    pub number_size_list: &'a GtkBox,
    pub highlighter_inspector_list: &'a GtkBox,
    pub sidebar_utility_controls: &'a GtkBox,
    pub background_inspector: &'a GtkBox,
    pub colors_inspector: &'a GtkBox,
    pub placeholder_inspector: &'a GtkBox,
    pub copy_btn: &'a Button,
    pub upload_btn: &'a Button,
    pub save_btn: &'a Button,
}

fn build_tool_inspector() -> (GtkBox, GtkBox) {
    let root = GtkBox::new(Orientation::Vertical, 12);
    root.set_width_request(BACKGROUND_SIDEBAR_WIDTH);
    root.set_hexpand(false);
    root.set_halign(gtk4::Align::Fill);
    root.set_vexpand(true);

    let content = GtkBox::new(Orientation::Vertical, 10);
    content.set_margin_top(4);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);
    content.set_hexpand(false);
    content.set_halign(gtk4::Align::Fill);

    root.append(&content);
    (root, content)
}

fn append_inspector_section(content: &GtkBox, title: &str, widget: &Widget) {
    let section = GtkBox::new(Orientation::Vertical, 8);
    section.add_css_class("editor-inspector-section");
    section.set_hexpand(false);
    section.set_halign(gtk4::Align::Fill);

    let section_title = Label::new(Some(title));
    section_title.add_css_class("editor-background-section-title");
    section_title.set_xalign(0.0);
    section_title.set_hexpand(false);
    section_title.set_halign(gtk4::Align::Fill);

    let section_body = GtkBox::new(Orientation::Vertical, 0);
    section_body.set_hexpand(false);
    section_body.set_halign(gtk4::Align::Fill);
    section_body.append(widget);

    section.append(&section_title);
    section.append(&section_body);
    content.append(&section);
}

pub(super) fn build_tool_inspectors(input: InspectorContentInputs<'_>) -> InspectorParts {
    let (select_inspector, select_inspector_content) = build_tool_inspector();
    append_inspector_section(
        &select_inspector_content,
        "Selection",
        input.select_status_label.upcast_ref(),
    );
    append_inspector_section(
        &select_inspector_content,
        "Details",
        input.select_detail_label.upcast_ref(),
    );
    append_inspector_section(
        &select_inspector_content,
        "Geometry",
        input.select_geometry_label.upcast_ref(),
    );
    append_inspector_section(
        &select_inspector_content,
        "Actions",
        input.select_hint_label.upcast_ref(),
    );

    let (crop_inspector, crop_inspector_content) = build_tool_inspector();
    input
        .crop_ratio_list
        .add_css_class("editor-inspector-option-list");
    append_inspector_section(
        &crop_inspector_content,
        "Dimensions",
        input.crop_dimensions_group.upcast_ref(),
    );
    append_inspector_section(
        &crop_inspector_content,
        "Aspect Ratio",
        input.crop_ratio_list.upcast_ref(),
    );
    append_inspector_section(
        &crop_inspector_content,
        "Actions",
        input.crop_actions_group.upcast_ref(),
    );

    let (pen_inspector, pen_inspector_content) = build_tool_inspector();
    input
        .pen_inspector_list
        .add_css_class("editor-inspector-option-list");
    append_inspector_section(
        &pen_inspector_content,
        "Thickness",
        input.pen_inspector_list.upcast_ref(),
    );

    let (arrow_inspector, arrow_inspector_content) = build_tool_inspector();
    input
        .arrow_style_list
        .add_css_class("editor-inspector-option-list");
    input
        .arrow_thickness_list
        .add_css_class("editor-inspector-option-list");
    append_inspector_section(
        &arrow_inspector_content,
        "Style",
        input.arrow_style_list.upcast_ref(),
    );
    append_inspector_section(
        &arrow_inspector_content,
        "Thickness",
        input.arrow_thickness_list.upcast_ref(),
    );
    append_inspector_section(
        &arrow_inspector_content,
        "Behavior",
        input.arrow_behavior_group.upcast_ref(),
    );

    let (line_inspector, line_inspector_content) = build_tool_inspector();
    input
        .line_inspector_list
        .add_css_class("editor-inspector-option-list");
    append_inspector_section(
        &line_inspector_content,
        "Thickness",
        input.line_inspector_list.upcast_ref(),
    );

    let (text_inspector, text_inspector_content) = build_tool_inspector();
    input
        .text_size_list
        .add_css_class("editor-inspector-option-list");
    input
        .font_family_list
        .add_css_class("editor-inspector-option-list");
    append_inspector_section(
        &text_inspector_content,
        "Size",
        input.text_size_list.upcast_ref(),
    );
    append_inspector_section(
        &text_inspector_content,
        "Font",
        input.font_family_list.upcast_ref(),
    );

    let (obfuscate_inspector, obfuscate_inspector_content) = build_tool_inspector();
    input
        .obfuscate_method_list
        .add_css_class("editor-inspector-option-list");
    append_inspector_section(
        &obfuscate_inspector_content,
        "Method",
        input.obfuscate_method_list.upcast_ref(),
    );

    let (number_inspector, number_inspector_content) = build_tool_inspector();
    input
        .number_options_list
        .add_css_class("editor-inspector-option-list");
    input
        .number_size_list
        .add_css_class("editor-inspector-option-list");
    append_inspector_section(
        &number_inspector_content,
        "Style",
        input.number_options_list.upcast_ref(),
    );
    append_inspector_section(
        &number_inspector_content,
        "Start",
        input.number_start_row.upcast_ref(),
    );
    append_inspector_section(
        &number_inspector_content,
        "Size",
        input.number_size_list.upcast_ref(),
    );

    let (highlighter_inspector, highlighter_inspector_content) = build_tool_inspector();
    input
        .highlighter_inspector_list
        .add_css_class("editor-inspector-option-list");
    append_inspector_section(
        &highlighter_inspector_content,
        "Thickness",
        input.highlighter_inspector_list.upcast_ref(),
    );

    let inspector_tabs = GtkBox::new(Orientation::Horizontal, 8);
    inspector_tabs.add_css_class("editor-inspector-tabs");
    inspector_tabs.set_width_request(BACKGROUND_SIDEBAR_WIDTH);
    inspector_tabs.set_hexpand(false);
    inspector_tabs.set_halign(gtk4::Align::Fill);

    let background_tab_btn = Button::with_label("Background");
    background_tab_btn.set_has_frame(false);
    background_tab_btn.add_css_class("editor-inspector-tab-button");

    let colors_tab_btn = Button::with_label("Colors");
    colors_tab_btn.set_has_frame(false);
    colors_tab_btn.add_css_class("editor-inspector-tab-button");

    inspector_tabs.append(&background_tab_btn);
    inspector_tabs.append(&colors_tab_btn);

    let inspector = GtkBox::new(Orientation::Vertical, 0);
    inspector.add_css_class("editor-right-inspector");
    inspector.set_width_request(BACKGROUND_SIDEBAR_WIDTH);
    inspector.set_hexpand(false);
    inspector.set_vexpand(true);
    inspector.append(input.sidebar_utility_controls);
    inspector.append(&inspector_tabs);

    let inspector_stack = Stack::new();
    inspector_stack.set_hhomogeneous(true);
    inspector_stack.set_vhomogeneous(false);
    inspector_stack.set_width_request(BACKGROUND_SIDEBAR_WIDTH);
    inspector_stack.set_hexpand(false);
    inspector_stack.set_vexpand(true);
    input.background_inspector.set_visible(true);
    crop_inspector.set_visible(true);
    pen_inspector.set_visible(true);
    arrow_inspector.set_visible(true);
    line_inspector.set_visible(true);
    text_inspector.set_visible(true);
    highlighter_inspector.set_visible(true);
    obfuscate_inspector.set_visible(true);
    number_inspector.set_visible(true);
    input.colors_inspector.set_visible(true);
    input.placeholder_inspector.set_visible(true);
    select_inspector.set_visible(true);
    inspector_stack.add_named(input.background_inspector, Some("background"));
    inspector_stack.add_named(&select_inspector, Some("select"));
    inspector_stack.add_named(&crop_inspector, Some("crop"));
    inspector_stack.add_named(&pen_inspector, Some("pen"));
    inspector_stack.add_named(&arrow_inspector, Some("arrow"));
    inspector_stack.add_named(&line_inspector, Some("line"));
    inspector_stack.add_named(&text_inspector, Some("text"));
    inspector_stack.add_named(&highlighter_inspector, Some("highlighter"));
    inspector_stack.add_named(&obfuscate_inspector, Some("obfuscate"));
    inspector_stack.add_named(&number_inspector, Some("number"));
    inspector_stack.add_named(input.colors_inspector, Some("colors"));
    inspector_stack.add_named(input.placeholder_inspector, Some("placeholder"));
    inspector_stack.set_visible_child_name("placeholder");
    inspector.append(&inspector_stack);

    let sidebar_actions = GtkBox::new(Orientation::Horizontal, 8);
    sidebar_actions.add_css_class("editor-sidebar-actions");
    let sidebar_action_spacer = GtkBox::new(Orientation::Horizontal, 0);
    sidebar_action_spacer.set_hexpand(true);
    sidebar_actions.append(input.copy_btn);
    sidebar_actions.append(input.upload_btn);
    sidebar_actions.append(&sidebar_action_spacer);
    sidebar_actions.append(input.save_btn);
    inspector.append(&sidebar_actions);

    InspectorParts {
        inspector_tabs,
        background_tab_btn,
        colors_tab_btn,
        inspector,
        inspector_stack,
    }
}
