//! The Annotations tab: the review queue, the current media's image labels,
//! and its objects.

use crate::app::DEFAULT_LABEL_COLOR;
use crate::app::helpers::{
    annotation_geometry_label, counted_section_title, empty_panel, hex_color, label_swatch,
    model_confidence_label, record_element_id, review_tag,
};
use crate::app::window::NotatusWindow;
use gpui::prelude::*;
use gpui::{IntoElement, div, px};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_component::scroll::ScrollableElement as _;
use gpui_component::tag::Tag;
use gpui_component::{Icon, IconName, Selectable as _, Sizable as _, ThemeColor};
use notatus_core::{
    AnnotationId, AnnotationRecord, AssetId, AssetRecord, ClassificationId, Label, LabelId,
    ReviewState,
};

impl NotatusWindow {
    pub(super) fn annotations_panel_content(
        &self,
        view: gpui::WeakEntity<NotatusWindow>,
        colors: ThemeColor,
    ) -> gpui::AnyElement {
        let Some(asset) = self.selected_asset() else {
            return empty_panel("No media selected").into_any_element();
        };
        let labels = self.state.dataset().labels.clone();
        let annotations = self.annotations_for_asset(asset);
        let annotation_count = annotations.len();
        let rows: Vec<_> = annotations
            .into_iter()
            .map(|annotation| self.annotation_row(annotation, &labels, view.clone(), colors))
            .collect();

        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_4()
            .p_3()
            .overflow_hidden()
            .child(self.review_queue_section(&labels, view.clone()))
            .child(self.image_labels_section(asset, &labels, view.clone(), colors))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(counted_section_title("Objects", annotation_count))
                    .child(if rows.is_empty() {
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_sm()
                            .text_color(colors.muted_foreground)
                            .child("Draw a box or polygon on the image")
                            .into_any_element()
                    } else {
                        div()
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scrollbar()
                            .flex()
                            .flex_col()
                            .children(rows)
                            .into_any_element()
                    }),
            )
            .into_any_element()
    }

    /// Whole-image labels as removable tags, with a menu to add more.
    fn image_labels_section(
        &self,
        asset: &AssetRecord,
        labels: &[Label],
        view: gpui::WeakEntity<NotatusWindow>,
        colors: ThemeColor,
    ) -> impl IntoElement {
        let classifications = self.classifications_for_asset(asset);
        let classified: Vec<_> = classifications.iter().map(|c| c.label_id).collect();
        let tags: Vec<_> = classifications
            .iter()
            .map(|classification| {
                let label = self.state.dataset().label_by_id(classification.label_id);
                let name = label.map_or("Unknown label", |label| label.name.as_str());
                let color = label
                    .and_then(|label| label.color.as_deref())
                    .unwrap_or(DEFAULT_LABEL_COLOR);
                image_label_tag(classification.id, name, color, view.clone(), colors)
            })
            .collect();

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(counted_section_title("Image labels", tags.len()))
                    .child(classification_label_menu(
                        asset.id,
                        labels.to_vec(),
                        classified,
                        view,
                    )),
            )
            .when(!tags.is_empty(), |section| {
                section.child(div().flex().flex_wrap().gap_1().children(tags))
            })
    }

    fn annotation_row(
        &self,
        annotation: &AnnotationRecord,
        labels: &[Label],
        view: gpui::WeakEntity<NotatusWindow>,
        colors: ThemeColor,
    ) -> gpui::AnyElement {
        let annotation_id = annotation.id;
        let label = self.state.dataset().label_by_id(annotation.label_id);
        let label_name = label.map_or("Unknown label", |label| label.name.as_str());
        let label_color = label
            .and_then(|label| label.color.as_deref())
            .unwrap_or(DEFAULT_LABEL_COLOR);
        let selected = self.state.selected_annotation() == Some(annotation_id);
        let hovered = self.hovered_annotation == Some(annotation_id);
        let uuid = annotation_id.as_uuid().as_u128();
        let row_view = view.clone();
        let hover_view = view.clone();

        div()
            .id(record_element_id("annotation-row", uuid))
            .flex()
            .items_center()
            .gap_2()
            .min_w_0()
            .h(px(44.0))
            .px_2()
            .rounded_md()
            .when(selected, |row| row.bg(colors.list_active))
            .when(hovered && !selected, |row| row.bg(colors.list_hover))
            .hover(move |row| {
                if selected {
                    row
                } else {
                    row.bg(colors.list_hover)
                }
            })
            .on_click(move |_, _, cx| {
                let _ = row_view.update(cx, |notatus, cx| {
                    notatus.select_annotation(Some(annotation_id), cx);
                });
            })
            .on_hover(move |hovered, _, cx| {
                let _ = hover_view.update(cx, |notatus, cx| {
                    notatus.hover_annotation(if *hovered { Some(annotation_id) } else { None }, cx);
                });
            })
            .child(label_swatch(label_color, false))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    // The menu button sizes to the name and lines up with
                    // the details under it.
                    .items_start()
                    .child(annotation_label_menu(
                        annotation_id,
                        annotation.label_id,
                        label_name,
                        labels.to_vec(),
                        view.clone(),
                    ))
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap_1()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_xs()
                            .text_color(colors.muted_foreground)
                            .child(annotation_geometry_label(&annotation.geometry))
                            .when_some(model_confidence_label(annotation), |row, confidence| {
                                row.child(format!("· Model {confidence}"))
                            })
                            .child(review_tag(annotation.review_state)),
                    ),
            )
            .child(review_buttons(annotation, view))
            .into_any_element()
    }
}

/// The label's name, opening a menu to change it.
fn annotation_label_menu(
    annotation_id: AnnotationId,
    current_label_id: LabelId,
    label_name: &str,
    labels: Vec<Label>,
    view: gpui::WeakEntity<NotatusWindow>,
) -> impl IntoElement {
    Button::new(record_element_id(
        "annotation-label-menu",
        annotation_id.as_uuid().as_u128(),
    ))
    .xsmall()
    .ghost()
    .compact()
    .label(label_name.to_string())
    .dropdown_caret(true)
    .tooltip("Change label")
    .ml(px(-6.0))
    .dropdown_menu(move |menu, _, _| {
        labels.iter().fold(menu, |menu, label| {
            let label_id = label.id;
            let current = label_id == current_label_id;
            let view = view.clone();
            menu.item(
                PopupMenuItem::new(label.name.clone())
                    .checked(current)
                    .disabled(current)
                    .on_click(move |_, _, cx| {
                        let _ = view.update(cx, |notatus, cx| {
                            notatus.update_annotation_label(annotation_id, label_id, cx);
                        });
                    }),
            )
        })
    })
}

/// Accept and reject toggles. Pressing the active one returns the annotation
/// to draft.
fn review_buttons(
    annotation: &AnnotationRecord,
    view: gpui::WeakEntity<NotatusWindow>,
) -> impl IntoElement {
    let annotation_id = annotation.id;
    let current = annotation.review_state;
    let uuid = annotation_id.as_uuid().as_u128();
    let button = |kind: &'static str, state: ReviewState, icon: IconName, tooltip: &'static str| {
        let view = view.clone();
        let active = current == state;
        Button::new(record_element_id(kind, uuid))
            .xsmall()
            .ghost()
            .icon(Icon::new(icon))
            .selected(active)
            .tooltip(tooltip)
            .on_click(move |_, _, cx| {
                cx.stop_propagation();
                let target = if active { ReviewState::Draft } else { state };
                let _ = view.update(cx, |notatus, cx| {
                    notatus.select_annotation(Some(annotation_id), cx);
                    notatus.review_selection(target, cx);
                });
            })
    };
    div()
        .flex()
        .gap(px(2.0))
        .child(button(
            "annotation-accept",
            ReviewState::Accepted,
            IconName::Check,
            "Accept (Y)",
        ))
        .child(button(
            "annotation-reject",
            ReviewState::Rejected,
            IconName::Close,
            "Reject (X)",
        ))
}

/// An image label as a tag in its label's color, with a button to remove it.
fn image_label_tag(
    classification_id: ClassificationId,
    name: &str,
    color: &str,
    view: gpui::WeakEntity<NotatusWindow>,
    colors: ThemeColor,
) -> impl IntoElement {
    let color = hex_color(color);
    Tag::custom(color.opacity(0.15), colors.foreground, color.opacity(0.6))
        .small()
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(name.to_string())
                .child(
                    Button::new(record_element_id(
                        "remove-classification",
                        classification_id.as_uuid().as_u128(),
                    ))
                    .xsmall()
                    .ghost()
                    .compact()
                    .icon(Icon::new(IconName::Close))
                    .tooltip("Remove image label")
                    .on_click(move |_, _, cx| {
                        let _ = view.update(cx, |notatus, cx| {
                            notatus.remove_classification(classification_id, cx);
                        });
                    }),
                ),
        )
}

fn classification_label_menu(
    asset_id: AssetId,
    labels: Vec<Label>,
    classified: Vec<LabelId>,
    view: gpui::WeakEntity<NotatusWindow>,
) -> impl IntoElement {
    Button::new("classification-label-menu")
        .xsmall()
        .ghost()
        .icon(Icon::new(IconName::Plus))
        .tooltip("Add image label")
        .dropdown_menu(move |menu, _, _| {
            if labels.is_empty() {
                return menu.item(PopupMenuItem::new("Create labels first").disabled(true));
            }
            labels.iter().fold(menu, |menu, label| {
                let label_id = label.id;
                let view = view.clone();
                menu.item(
                    PopupMenuItem::new(label.name.clone())
                        .checked(classified.contains(&label_id))
                        .on_click(move |_, _, cx| {
                            let _ = view.update(cx, |notatus, cx| {
                                notatus.toggle_image_classification(asset_id, label_id, cx);
                            });
                        }),
                )
            })
        })
}
