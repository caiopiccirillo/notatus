//! The Details tab: the project, the dataset's totals, and the current media.

use crate::app::helpers::*;
use crate::app::window::NotatusWindow;
use gpui::prelude::*;
use gpui::{App, IntoElement, div, px};
use gpui_component::description_list::DescriptionList;
use gpui_component::input::Input;
use gpui_component::scroll::ScrollableElement as _;
use gpui_component::{ActiveTheme as _, Sizable as _};

impl NotatusWindow {
    pub(super) fn details_panel_content(&self, cx: &App) -> impl IntoElement {
        let dataset = self.state.dataset();
        let done = dataset
            .assets
            .iter()
            .filter(|asset| asset.status == notatus_core::AssetStatus::Done)
            .count();
        let draft_count = dataset
            .annotations
            .iter()
            .filter(|annotation| annotation.review_state == notatus_core::ReviewState::Draft)
            .count();

        let project = details_list()
            .item("Location", self.project_location.display_name(), 1)
            .item(
                "Created",
                dataset.manifest.project.created_at.date().to_string(),
                1,
            );
        let totals = details_list()
            .item(
                "Media",
                format!("{} · {done} done", media_count_label(dataset.assets.len())),
                1,
            )
            .item("Labels", label_count_label(dataset.labels.len()), 1)
            .item(
                "Annotations",
                annotation_count_label(dataset.annotations.len()),
                1,
            )
            .item("Image labels", dataset.classifications.len().to_string(), 1)
            .item("Drafts", draft_count.to_string(), 1);

        div()
            .size_full()
            .overflow_y_scrollbar()
            .p_4()
            .flex()
            .flex_col()
            .gap_5()
            .child(
                details_section("Project")
                    .child(Input::new(&self.project_name_input).small().w_full())
                    .child(project),
            )
            .child(details_section("Dataset").child(totals))
            .when_some(self.selected_asset(), |panel, asset| {
                panel.child(
                    details_section("Media").child(
                        details_list()
                            .item("Name", asset_display_name(asset), 1)
                            .item("Type", asset_kind_label(&asset.kind), 1)
                            .item("Size", asset_dimensions_label(asset), 1)
                            .item("Split", dataset_split_label(&asset.split), 1)
                            .item("Location", asset.location.display_path().into_owned(), 1),
                    ),
                )
            })
            .text_color(cx.theme().foreground)
    }
}

/// One label and value per row, labels in a fixed column.
fn details_list() -> DescriptionList {
    DescriptionList::new()
        .columns(1)
        .bordered(false)
        .label_width(px(96.0))
        .small()
}

fn details_section(title: &'static str) -> gpui::Div {
    div().flex().flex_col().gap_2().child(section_title(title))
}
