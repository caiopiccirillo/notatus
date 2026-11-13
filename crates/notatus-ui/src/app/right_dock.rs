use super::helpers::*;
use super::*;
use notatus_core::ReviewState;

impl NotatusWindow {
    pub(super) fn right_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors;
        div()
            .size_full()
            .h_full()
            .flex()
            .flex_col()
            .border_l_1()
            .border_color(colors.border)
            .bg(colors.sidebar)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(match self.right_dock {
                        RightDock::Annotations => self
                            .annotations_panel_content(cx.weak_entity(), colors)
                            .into_any_element(),
                        RightDock::Info => self.info_panel_content().into_any_element(),
                    }),
            )
    }

    fn annotations_panel_content(
        &self,
        view: gpui::WeakEntity<NotatusWindow>,
        colors: ThemeColor,
    ) -> gpui::AnyElement {
        if let Some(asset) = self.selected_asset() {
            let annotations = self.annotations_for_asset(asset);
            let classifications = self.classifications_for_asset(asset);
            let annotation_count = annotations.len();
            let classification_count = classifications.len();
            let labels = self.state.dataset.labels.clone();
            let classified_label_ids: Vec<_> = classifications
                .iter()
                .map(|classification| classification.label_id)
                .collect();
            let classification_rows: Vec<_> = classifications
                .into_iter()
                .map(|classification| self.classification_row(classification, view.clone(), colors))
                .map(IntoElement::into_any_element)
                .collect();
            let rows: Vec<_> = annotations
                .into_iter()
                .map(|annotation| {
                    self.annotation_row(annotation, labels.clone(), view.clone(), colors)
                })
                .map(IntoElement::into_any_element)
                .collect();

            div()
                .size_full()
                .flex()
                .flex_col()
                .gap_3()
                .p_4()
                .overflow_hidden()
                .child(section_title("Annotations"))
                .child(metric(
                    "Media",
                    compact_text(&asset_display_name(asset), 28),
                ))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(counted_section_title("Image labels", classification_count))
                        .child(self.classification_label_menu(
                            asset.id,
                            labels.clone(),
                            classified_label_ids,
                            view.clone(),
                        )),
                )
                .child(if classification_rows.is_empty() {
                    div()
                        .h(px(34.0))
                        .flex()
                        .items_center()
                        .text_sm()
                        .text_color(colors.muted_foreground)
                        .child("No image labels")
                        .into_any_element()
                } else {
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .children(classification_rows)
                        .into_any_element()
                })
                .child(counted_section_title("Objects", annotation_count))
                .child(if rows.is_empty() {
                    div()
                        .flex_1()
                        .min_h_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_sm()
                        .text_color(colors.muted_foreground)
                        .child("No annotations")
                        .into_any_element()
                } else {
                    div()
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scrollbar()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .children(rows)
                        .into_any_element()
                })
                .into_any_element()
        } else {
            empty_panel("No media selected").into_any_element()
        }
    }

    fn classification_row(
        &self,
        classification: &ClassificationRecord,
        view: gpui::WeakEntity<NotatusWindow>,
        colors: ThemeColor,
    ) -> impl IntoElement {
        let classification_id = classification.id;
        let label = self.state.dataset.label_by_id(classification.label_id);
        let label_name = label
            .map(|label| label.name.as_str())
            .unwrap_or("Unknown label")
            .to_string();
        let label_color = label
            .and_then(|label| label.color.as_deref())
            .unwrap_or(DEFAULT_LABEL_COLOR)
            .to_string();
        let (classification_key_high, classification_key_low) =
            classification_element_key(classification_id);
        let row_id = gpui::ElementId::from(("classification-row", classification_key_high));
        let remove_view = view;

        div()
            .id((row_id, classification_key_low.to_string()))
            .flex()
            .items_center()
            .gap_2()
            .min_w_0()
            .h(px(34.0))
            .px_2()
            .rounded_sm()
            .border_1()
            .border_color(colors.border)
            .bg(colors.background)
            .child(label_swatch(&label_color, false))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(label_name),
            )
            .child(
                Button::new((
                    gpui::ElementId::from(("remove-classification", classification_key_high)),
                    classification_key_low.to_string(),
                ))
                .small()
                .icon(Icon::new(IconName::Delete))
                .tooltip("Remove image label")
                .on_click(move |_, _, cx| {
                    let _ = remove_view.update(cx, |notatus, cx| {
                        notatus.remove_classification(classification_id, cx);
                    });
                }),
            )
    }

    fn classification_label_menu(
        &self,
        asset_id: AssetId,
        labels: Vec<Label>,
        classified_label_ids: Vec<LabelId>,
        view: gpui::WeakEntity<NotatusWindow>,
    ) -> impl IntoElement {
        Button::new("classification-label-menu")
            .small()
            .icon(Icon::new(IconName::Plus))
            .tooltip("Add image label")
            .dropdown_menu(move |menu, _, _| {
                if labels.is_empty() {
                    return menu
                        .item(PopupMenuItem::new("Create labels in Dataset").disabled(true));
                }

                let mut menu = menu;
                for label in labels.clone() {
                    let label_id = label.id;
                    let label_name = label.name.clone();
                    let selected = classified_label_ids.contains(&label_id);
                    let view = view.clone();
                    menu = menu.item(PopupMenuItem::new(label_name).checked(selected).on_click(
                        move |_, _, cx| {
                            let _ = view.update(cx, |notatus, cx| {
                                notatus.toggle_image_classification(asset_id, label_id, cx);
                            });
                        },
                    ));
                }
                menu
            })
    }

    fn annotation_row(
        &self,
        annotation: &AnnotationRecord,
        labels: Vec<Label>,
        view: gpui::WeakEntity<NotatusWindow>,
        colors: ThemeColor,
    ) -> impl IntoElement {
        let annotation_id = annotation.id;
        let label = self.state.dataset.label_by_id(annotation.label_id);
        let label_name = label
            .map(|label| label.name.as_str())
            .unwrap_or("Unknown label")
            .to_string();
        let label_color = label
            .and_then(|label| label.color.as_deref())
            .unwrap_or(DEFAULT_LABEL_COLOR)
            .to_string();
        let selected = self.state.selected_annotation == Some(annotation_id);
        let hovered = self.hovered_annotation == Some(annotation_id);
        let (annotation_key_high, annotation_key_low) = annotation_element_key(annotation_id);
        let row_id = gpui::ElementId::from(("annotation-row", annotation_key_high));
        let row_view = view.clone();
        let hover_view = view.clone();

        div()
            .id((row_id, annotation_key_low.to_string()))
            .flex()
            .items_center()
            .gap_2()
            .min_w_0()
            .h(px(44.0))
            .px_2()
            .rounded_sm()
            .border_1()
            .border_color(if selected {
                colors.list_active_border
            } else if hovered {
                colors.input
            } else {
                colors.border
            })
            .bg(if selected {
                colors.list_active
            } else if hovered {
                colors.list_hover
            } else {
                colors.background
            })
            .hover(move |row| row.bg(colors.list_hover))
            .on_click(move |_, window, cx| {
                let _ = row_view.update(cx, |notatus, cx| {
                    notatus.select_annotation(Some(annotation_id), window, cx);
                });
            })
            .on_hover(move |hovered, _, cx| {
                let _ = hover_view.update(cx, |notatus, cx| {
                    notatus.hover_annotation(if *hovered { Some(annotation_id) } else { None }, cx);
                });
            })
            .child(label_swatch(&label_color, false))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(1.0))
                    .child(
                        div()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(label_name),
                    )
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
                            .child(review_tag(annotation.review_state)),
                    ),
            )
            .child(self.annotation_review_buttons(annotation, view.clone()))
            .child(self.annotation_label_menu(
                annotation_id,
                annotation.label_id,
                labels,
                view.clone(),
            ))
    }

    /// Accept and reject toggles. Pressing the active one returns the
    /// annotation to draft.
    fn annotation_review_buttons(
        &self,
        annotation: &AnnotationRecord,
        view: gpui::WeakEntity<NotatusWindow>,
    ) -> impl IntoElement {
        let annotation_id = annotation.id;
        let current = annotation.review_state;
        let (key_high, key_low) = annotation_element_key(annotation_id);
        let button =
            |kind: &'static str, state: ReviewState, icon: IconName, tooltip: &'static str| {
                let view = view.clone();
                let active = current == state;
                Button::new((gpui::ElementId::from((kind, key_high)), key_low.to_string()))
                    .xsmall()
                    .ghost()
                    .icon(Icon::new(icon))
                    .selected(active)
                    .tooltip(tooltip)
                    .on_click(move |_, window, cx| {
                        cx.stop_propagation();
                        let target = if active { ReviewState::Draft } else { state };
                        let _ = view.update(cx, |notatus, cx| {
                            notatus.select_annotation(Some(annotation_id), window, cx);
                            notatus.review_selection(target, window, cx);
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

    fn annotation_label_menu(
        &self,
        annotation_id: AnnotationId,
        current_label_id: LabelId,
        labels: Vec<Label>,
        view: gpui::WeakEntity<NotatusWindow>,
    ) -> impl IntoElement {
        let (annotation_key_high, annotation_key_low) = annotation_element_key(annotation_id);
        let menu_id = gpui::ElementId::from(("annotation-label-menu", annotation_key_high));
        Button::new((menu_id, annotation_key_low.to_string()))
            .small()
            .icon(Icon::new(IconName::Palette))
            .tooltip("Change label")
            .dropdown_menu(move |menu, _, _| {
                let mut menu = menu;
                for label in labels.clone() {
                    let label_id = label.id;
                    let label_name = label.name.clone();
                    let selected = label_id == current_label_id;
                    let view = view.clone();
                    menu = menu.item(
                        PopupMenuItem::new(label_name)
                            .checked(selected)
                            .disabled(selected)
                            .on_click(move |_, window, cx| {
                                let _ = view.update(cx, |notatus, cx| {
                                    notatus.update_annotation_label(
                                        annotation_id,
                                        label_id,
                                        window,
                                        cx,
                                    );
                                });
                            }),
                    );
                }
                menu
            })
    }

    fn info_panel_content(&self) -> gpui::Div {
        let active_label = self
            .selected_label()
            .map(|label| label.name.clone())
            .unwrap_or_else(|| "None".to_string());
        let review_queue = self
            .state
            .dataset
            .annotations
            .iter()
            .filter(|annotation| annotation.review_state == notatus_core::ReviewState::Draft)
            .count();

        let panel = div()
            .size_full()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .overflow_hidden()
            .child(section_title("Info"))
            .child(metric(
                "Active tool",
                self.state.active_tool.display_name().to_string(),
            ))
            .child(metric(
                "Zoom",
                format!("{:.0}%", self.tools.viewport.zoom * 100.0),
            ))
            .child(metric("Active label", active_label))
            .child(metric(
                "Media",
                media_count_label(self.state.dataset.assets.len()),
            ))
            .child(metric(
                "Annotations",
                annotation_count_label(self.state.dataset.annotations.len()),
            ))
            .child(metric(
                "Labels",
                label_count_label(self.state.dataset.labels.len()),
            ))
            // Short enough for the metric label column.
            .child(metric("To review", review_queue.to_string()))
            .when_some(self.status_message.clone(), |panel, status| {
                panel.child(metric("Status", status))
            });

        if let Some(asset) = self.selected_asset() {
            panel
                .child(section_title("Media"))
                .child(metric("Name", compact_text(&asset_display_name(asset), 28)))
                .child(metric("Type", asset_kind_label(&asset.kind).to_string()))
                .child(metric("Dimensions", asset_dimensions_label(asset)))
                .child(metric(
                    "Split",
                    dataset_split_label(&asset.split).to_string(),
                ))
                .child(metric(
                    "Location",
                    compact_text(asset.location.display_path().as_ref(), 30),
                ))
        } else {
            panel
        }
    }
}

fn annotation_element_key(annotation_id: AnnotationId) -> (u64, u64) {
    let value = annotation_id.as_uuid().as_u128();
    ((value >> 64) as u64, value as u64)
}

fn classification_element_key(classification_id: ClassificationId) -> (u64, u64) {
    let value = classification_id.as_uuid().as_u128();
    ((value >> 64) as u64, value as u64)
}
