use crate::app::window::NotatusWindow;
use gpui::prelude::*;
use gpui::{App, FontWeight, IntoElement, Window, div, px, rgb};
use gpui_component::{ActiveTheme as _, Sizable as _};
use notatus_core::{
    AnnotationGeometry, AnnotationRecord, AnnotationSource, AssetKind, AssetRecord, Dataset,
    DatasetSplit, LabelId, ReviewState,
};
use std::path::Path;

/// Defers building an element until render time so free helper functions can
/// read the active [`gpui_component::Theme`] without threading `cx` through
/// every caller.
#[derive(IntoElement)]
pub(super) struct Themed<F, E>
where
    F: FnOnce(&gpui_component::Theme) -> E + 'static,
    E: IntoElement + 'static,
{
    build: F,
    _element: std::marker::PhantomData<fn() -> E>,
}

pub(super) fn themed<F, E>(build: F) -> Themed<F, E>
where
    F: FnOnce(&gpui_component::Theme) -> E + 'static,
    E: IntoElement + 'static,
{
    Themed {
        build,
        _element: std::marker::PhantomData,
    }
}

impl<F, E> RenderOnce for Themed<F, E>
where
    F: FnOnce(&gpui_component::Theme) -> E + 'static,
    E: IntoElement + 'static,
{
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        (self.build)(cx.theme())
    }
}

/// A stable element id for a row or button that belongs to one record,
/// from the record's UUID as a `u128`.
pub(super) fn record_element_id(prefix: &'static str, uuid: u128) -> gpui::ElementId {
    let high = (uuid >> 64) as u64;
    let low = uuid as u64;
    (gpui::ElementId::from((prefix, high)), low.to_string()).into()
}

pub(super) fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

pub(super) fn canvas_message(message: &'static str) -> impl IntoElement {
    themed(move |theme| {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_sm()
            .text_color(theme.muted_foreground)
            .child(message)
    })
}

pub(super) fn empty_panel(message: &'static str) -> impl IntoElement {
    themed(move |theme| {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .p_4()
            .text_sm()
            .text_color(theme.muted_foreground)
            .child(message)
    })
}

pub(super) fn sidebar_count(value: impl Into<String>) -> impl IntoElement {
    let value = value.into();
    themed(move |theme| {
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
            .text_color(theme.muted_foreground)
            .bg(theme.muted)
            .child(value)
    })
}

pub(super) fn label_color_button(
    color_ix: usize,
    color: &'static str,
    selected: bool,
    label_id: LabelId,
    view: gpui::WeakEntity<NotatusWindow>,
) -> impl IntoElement {
    themed(move |theme| {
        div()
            .id(("label-color", color_ix))
            .flex_none()
            .size(px(28.0))
            .rounded_sm()
            .border_1()
            .border_color(if selected {
                theme.foreground
            } else {
                theme.border
            })
            .p(px(3.0))
            .hover(|swatch| swatch.border_color(theme.muted_foreground))
            .on_click(move |_, _, cx| {
                let _ = view.update(cx, |notatus, cx| {
                    notatus.update_label_color(label_id, color, cx);
                });
            })
            .child(label_swatch(color, true))
    })
}

/// "87%" for a model's suggestion with a confidence; "?" without one. `None`
/// for annotations a person drew or imported, and for suggestions a person
/// has since edited.
pub(super) fn model_confidence_label(annotation: &AnnotationRecord) -> Option<String> {
    if annotation.human_edited || !matches!(annotation.source, AnnotationSource::Model(_)) {
        return None;
    }
    Some(match annotation.confidence {
        Some(confidence) => format!("{:.0}%", confidence * 100.0),
        None => "?".to_string(),
    })
}

/// A small colored tag naming an annotation's review state.
pub(super) fn review_tag(state: ReviewState) -> impl IntoElement {
    use gpui_component::tag::Tag;
    use notatus_core::ReviewState;

    let (tag, text) = match state {
        ReviewState::Draft => (Tag::secondary(), "Draft"),
        ReviewState::Reviewed => (Tag::info(), "Reviewed"),
        ReviewState::Accepted => (Tag::success(), "Accepted"),
        ReviewState::Rejected => (Tag::danger(), "Rejected"),
    };
    tag.xsmall().outline().child(text)
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

pub(super) fn exportable_annotation_count(dataset: &Dataset) -> usize {
    let filter = notatus_export::AnnotationFilter::ready_for_training();
    dataset
        .annotations
        .iter()
        .filter(|annotation| filter.accepts(annotation))
        .count()
}

pub(super) fn label_count_label(count: usize) -> String {
    format!("{count} label{}", plural(count))
}

pub(super) fn asset_kind_label(kind: &AssetKind) -> &'static str {
    match kind {
        AssetKind::Image => "Image",
        AssetKind::Video => "Video",
    }
}

pub(super) fn dataset_split_label(split: &DatasetSplit) -> &'static str {
    match split {
        DatasetSplit::Train => "Train",
        DatasetSplit::Validation => "Validation",
        DatasetSplit::Test => "Test",
        DatasetSplit::Unassigned => "Unassigned",
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

pub(super) fn section_title(title: &'static str) -> impl IntoElement {
    themed(move |theme| {
        div()
            .text_xs()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme.muted_foreground)
            .child(title)
    })
}

/// A section title followed by how many items the section holds.
pub(super) fn counted_section_title(title: &'static str, count: usize) -> impl IntoElement {
    themed(move |theme| {
        div()
            .text_xs()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme.muted_foreground)
            .child(format!("{title} · {count}"))
    })
}

#[cfg(test)]
mod confidence_tests {
    use super::*;
    use notatus_core::{AssetId, BoundingBox};

    #[test]
    fn only_untouched_model_suggestions_show_a_confidence() {
        let bbox = AnnotationGeometry::Bbox(BoundingBox::from_xywh(1.0, 1.0, 2.0, 2.0).unwrap());
        let model = notatus_core::ModelProvenance {
            name: "detector".to_string(),
            version: None,
            invocation_id: None,
            metadata: Default::default(),
        };
        let asset = AssetId::new();
        let label = LabelId::new();
        let mut suggestion =
            AnnotationRecord::new_model(asset, label, bbox.clone(), model.clone(), Some(0.874));
        let unsure = AnnotationRecord::new_model(asset, label, bbox.clone(), model, None);
        let drawn = AnnotationRecord::new_human(asset, label, bbox, None);

        assert_eq!(model_confidence_label(&suggestion).as_deref(), Some("87%"));
        assert_eq!(model_confidence_label(&unsure).as_deref(), Some("?"));
        assert_eq!(model_confidence_label(&drawn), None);
        suggestion.record_human_edit();
        assert_eq!(model_confidence_label(&suggestion), None);
    }
}
