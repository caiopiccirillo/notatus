use super::helpers::*;
use super::*;

impl NotatusWindow {
    pub(super) fn right_panel(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .h_full()
            .flex()
            .flex_col()
            .border_l_1()
            .border_color(rgb(0xd6d9de))
            .bg(rgb(0xffffff))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(match self.right_dock {
                        RightDock::Annotations => {
                            self.annotations_panel_content().into_any_element()
                        }
                        RightDock::Info => self.info_panel_content().into_any_element(),
                    }),
            )
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
            .child(metric("Review queue", review_queue.to_string()))
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
