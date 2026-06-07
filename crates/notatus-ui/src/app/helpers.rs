use super::*;

pub(super) fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

pub(super) fn canvas_message(message: &'static str) -> gpui::Div {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(rgb(0x4b5563))
        .child(message)
}

pub(super) fn empty_panel(message: &'static str) -> gpui::Div {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .p_4()
        .text_sm()
        .text_color(rgb(0x6b7280))
        .child(message)
}

pub(super) fn sidebar_count(value: impl Into<String>) -> impl IntoElement {
    div()
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .min_w(px(24.0))
        .h(px(20.0))
        .px_1()
        .rounded_sm()
        .text_xs()
        .text_color(rgb(0x4b5563))
        .bg(rgb(0xf3f4f6))
        .child(value.into())
}

pub(super) fn media_asset_meta(kind: &AssetKind, annotation_count: usize) -> impl IntoElement {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap_1()
        .child(
            div()
                .text_xs()
                .text_color(rgb(0x6b7280))
                .child(asset_kind_label(kind)),
        )
        .child(sidebar_count(annotation_count.to_string()))
}

pub(super) fn label_asset_meta(color: Option<&str>, annotation_count: usize) -> impl IntoElement {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap_1()
        .child(label_swatch(color.unwrap_or(DEFAULT_LABEL_COLOR), false))
        .child(sidebar_count(annotation_count.to_string()))
}

pub(super) fn label_color_button(
    color_ix: usize,
    color: &'static str,
    selected: bool,
    view: gpui::WeakEntity<NotatusWindow>,
) -> impl IntoElement {
    div()
        .id(("label-color", color_ix))
        .flex_none()
        .size(px(28.0))
        .rounded_sm()
        .border_1()
        .border_color(if selected {
            rgb(0x111827)
        } else {
            rgb(0xd1d5db)
        })
        .p(px(3.0))
        .hover(|swatch| swatch.border_color(rgb(0x6b7280)))
        .on_click(move |_, _, cx| {
            let _ = view.update(cx, |notatus, cx| {
                notatus.update_selected_label_color(color, cx);
            });
        })
        .child(label_swatch(color, true))
}

pub(super) fn label_swatch(color: &str, fill_parent: bool) -> impl IntoElement {
    div()
        .flex_none()
        .when(fill_parent, |swatch| swatch.size_full())
        .when(!fill_parent, |swatch| swatch.size(px(12.0)))
        .rounded_sm()
        .bg(hex_color(color))
}

pub(super) fn annotation_geometry_label(geometry: &AnnotationGeometry) -> &'static str {
    match geometry {
        AnnotationGeometry::Bbox(_) => "Box",
        AnnotationGeometry::Polygon(_) => "Poly",
    }
}

pub(super) fn media_count_label(count: usize) -> String {
    format!("{count} media")
}

pub(super) fn annotation_count_label(count: usize) -> String {
    format!("{count} annotation{}", plural(count))
}

pub(super) fn label_count_label(count: usize) -> String {
    format!("{count} label{}", plural(count))
}

pub(super) fn dataset_created_label(dataset: &notatus_core::Dataset) -> String {
    format!("Created {}", dataset.manifest.project.created_at.date())
}

pub(super) fn asset_kind_label(kind: &AssetKind) -> &'static str {
    match kind {
        AssetKind::Image => "Image",
        AssetKind::Video => "Video",
    }
}

pub(super) fn dataset_split_label(split: &notatus_core::DatasetSplit) -> &'static str {
    match split {
        notatus_core::DatasetSplit::Train => "Train",
        notatus_core::DatasetSplit::Validation => "Validation",
        notatus_core::DatasetSplit::Test => "Test",
        notatus_core::DatasetSplit::Unassigned => "Unassigned",
    }
}

pub(super) fn asset_display_name(asset: &AssetRecord) -> String {
    let display_path = asset.location.display_path();
    Path::new(display_path.as_ref())
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(display_path.as_ref())
        .to_string()
}

pub(super) fn compact_asset_name(asset: &AssetRecord) -> String {
    compact_text(&asset_display_name(asset), 36)
}

pub(super) fn asset_dimensions_label(asset: &AssetRecord) -> String {
    format!("{}x{}", asset.dimensions.width, asset.dimensions.height)
}

pub(super) fn compact_text(value: &str, max_chars: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= max_chars || max_chars < 8 {
        return value.to_string();
    }

    let head_count = (max_chars - 3) * 2 / 3;
    let tail_count = max_chars - 3 - head_count;
    let head: String = value.chars().take(head_count).collect();
    let tail: String = value
        .chars()
        .rev()
        .take(tail_count)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{head}...{tail}")
}

pub(super) fn hex_color(value: &str) -> gpui::Hsla {
    let hex = value.strip_prefix('#').unwrap_or(value);
    u32::from_str_radix(hex, 16)
        .map(rgb)
        .unwrap_or_else(|_| rgb(0x2563eb))
        .into()
}

pub(super) fn dataset_has_local_path(state: &UiState, path: &str) -> bool {
    state.dataset.assets.iter().any(|asset| {
        matches!(
            &asset.location,
            AssetLocation::LocalPath { path: existing } if existing == path
        )
    })
}

pub(super) fn section_title(title: &'static str) -> impl IntoElement {
    div()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(0x374151))
        .child(title)
}

pub(super) fn metric(label: &'static str, value: String) -> impl IntoElement {
    div()
        .flex()
        .items_start()
        .gap_2()
        .min_w_0()
        .text_sm()
        .child(
            div()
                .flex_none()
                .w(px(84.0))
                .overflow_hidden()
                .whitespace_nowrap()
                .truncate()
                .text_color(rgb(0x4b5563))
                .child(label),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .truncate()
                .font_weight(FontWeight::SEMIBOLD)
                .child(value),
        )
}
