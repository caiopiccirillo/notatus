use super::helpers::*;
use super::*;

impl NotatusWindow {
    pub(super) fn right_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .h_full()
            .flex()
            .flex_col()
            .border_l_1()
            .border_color(rgb(0xd6d9de))
            .bg(rgb(0xffffff))
            .child(self.right_panel_header(cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(match self.right_dock {
                        RightDock::Annotations => {
                            self.annotations_panel_content().into_any_element()
                        }
                        RightDock::MediaInfo => self.media_info_panel_content().into_any_element(),
                    }),
            )
    }

    fn right_panel_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(rgb(0xe5e7eb))
            .child(self.right_dock_button(
                "right-annotations",
                IconName::Frame,
                "Annotations",
                RightDock::Annotations,
                cx,
            ))
            .child(self.right_dock_button(
                "right-media-info",
                IconName::Info,
                "Info",
                RightDock::MediaInfo,
                cx,
            ))
    }

    fn right_dock_button(
        &self,
        id: &'static str,
        icon: IconName,
        label: &'static str,
        dock: RightDock,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let view = cx.weak_entity();
        Button::new(id)
            .small()
            .icon(Icon::new(icon))
            .label(label)
            .selected(self.right_dock == dock)
            .on_click(move |_, _, cx| {
                let _ = view.update(cx, |notatus, cx| {
                    notatus.right_dock = dock;
                    cx.notify();
                });
            })
    }

    fn annotations_panel_content(&self) -> gpui::Div {
        if let Some(asset) = self.selected_asset() {
            let annotations = self.annotations_for_asset(asset);

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
                .child(metric("Total", annotation_count_label(annotations.len())))
                .child(div().flex_1().min_h_0().overflow_hidden().child(
                    SidebarMenu::new().children(annotation_items_for_asset(&self.state, asset)),
                ))
        } else {
            empty_panel("No media selected")
        }
    }

    fn media_info_panel_content(&self) -> gpui::Div {
        let selected_media = self
            .selected_asset()
            .map(compact_asset_name)
            .unwrap_or_else(|| "None".to_string());
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
            .gap_3()
            .p_4()
            .overflow_hidden()
            .child(section_title("Info"))
            .child(metric(
                "Dataset",
                compact_text(&self.state.dataset.manifest.project.name, 28),
            ))
            .child(metric(
                "Active tool",
                format!("{:?}", self.state.active_tool),
            ))
            .child(metric("Active label", active_label))
            .child(metric("Selected media", selected_media))
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
            .child(metric("Review queue", review_queue.to_string()))
            .child(metric(
                "State",
                if self.state.dirty { "Unsaved" } else { "Saved" }.to_string(),
            ))
            .when_some(self.status_message.clone(), |panel, status| {
                panel.child(metric("Status", status))
            });

        if let Some(asset) = self.selected_asset() {
            let annotation_count = self.annotations_for_asset(asset).len();

            panel
                .child(section_title("Media"))
                .child(metric("Name", compact_text(&asset_display_name(asset), 28)))
                .child(metric("Type", asset_kind_label(&asset.kind).to_string()))
                .child(metric("Dimensions", asset_dimensions_label(asset)))
                .child(metric(
                    "Annotations",
                    annotation_count_label(annotation_count),
                ))
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
                .child(section_title("Media"))
                .child(metric("Name", "None".to_string()))
        }
    }
}
