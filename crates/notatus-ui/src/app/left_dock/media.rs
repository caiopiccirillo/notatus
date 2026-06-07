use super::*;
use crate::app::helpers::*;
use crate::app::media_import::open_media_picker;

impl NotatusWindow {
    pub(super) fn media_dock(&self, cx: &mut Context<Self>) -> gpui::Div {
        let view = cx.weak_entity();
        let import_view = cx.weak_entity();

        div()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(0xe5e7eb))
                    .child(section_title("Media"))
                    .child(
                        Button::new("media-import")
                            .small()
                            .icon(Icon::new(IconName::Plus))
                            .tooltip("Import media")
                            .on_click(move |_, window, cx| {
                                open_media_picker(import_view.clone(), window, cx);
                            }),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .p_2()
                    .child(self.media_content(view)),
            )
    }

    fn media_content(&self, view: gpui::WeakEntity<NotatusWindow>) -> impl IntoElement {
        if self.state.dataset.assets.is_empty() {
            let import_view = view.clone();
            div()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_3()
                .p_4()
                .text_sm()
                .text_color(rgb(0x6b7280))
                .child("Import media after setting up project labels")
                .child(
                    Button::new("media-empty-import")
                        .small()
                        .icon(Icon::new(IconName::Plus))
                        .label("Import media")
                        .on_click(move |_, window, cx| {
                            open_media_picker(import_view.clone(), window, cx);
                        }),
                )
                .into_any_element()
        } else {
            SidebarMenu::new()
                .children(self.asset_items(view))
                .into_any_element()
        }
    }

    fn asset_items(&self, view: gpui::WeakEntity<NotatusWindow>) -> Vec<SidebarMenuItem> {
        if self.state.dataset.assets.is_empty() {
            vec![SidebarMenuItem::new("No media yet").disable(true)]
        } else {
            self.state
                .dataset
                .assets
                .iter()
                .map(|asset| {
                    let asset_id = asset.id;
                    let annotation_count = self
                        .state
                        .dataset
                        .annotations
                        .iter()
                        .filter(|annotation| annotation.asset_id == asset_id)
                        .count();
                    let select_view = view.clone();
                    let remove_view = view.clone();
                    SidebarMenuItem::new(compact_asset_name(asset))
                        .suffix(media_asset_actions(
                            &asset.kind,
                            annotation_count,
                            asset_id,
                            remove_view,
                        ))
                        .active(self.state.selected_asset == Some(asset_id))
                        .on_click(move |_, _, cx| {
                            let _ = select_view.update(cx, |window, cx| {
                                if let Err(error) = window.state.select_asset(asset_id) {
                                    window.status_message = Some(error.to_string());
                                } else {
                                    window.tools.fit_canvas_to_view();
                                }
                                cx.notify();
                            });
                        })
                })
                .collect()
        }
    }
}

fn media_asset_actions(
    kind: &AssetKind,
    annotation_count: usize,
    asset_id: AssetId,
    view: gpui::WeakEntity<NotatusWindow>,
) -> impl IntoElement {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap_1()
        .child(media_asset_meta(kind, annotation_count))
        .child(
            Button::new(asset_element_id("media-remove", asset_id))
                .xsmall()
                .ghost()
                .danger()
                .icon(Icon::new(IconName::Delete))
                .tooltip("Remove media and its annotations")
                .on_click(move |_, _, cx| {
                    cx.stop_propagation();
                    let _ = view.update(cx, |notatus, cx| {
                        notatus.remove_asset(asset_id, cx);
                    });
                }),
        )
}

fn asset_element_id(prefix: &'static str, asset_id: AssetId) -> gpui::ElementId {
    let value = asset_id.as_uuid().as_u128();
    let high = (value >> 64) as u64;
    let low = (value as u64).to_string();
    (gpui::ElementId::from((prefix, high)), low).into()
}
