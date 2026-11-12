use super::*;
use crate::app::helpers::*;
use crate::app::media_import::open_media_picker;

impl NotatusWindow {
    pub(super) fn media_dock(&self, cx: &mut Context<Self>) -> gpui::Div {
        let view = cx.weak_entity();
        let import_view = cx.weak_entity();
        let colors = cx.theme().colors;

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
                    .border_color(colors.border)
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
                    .child(self.media_content(view, colors)),
            )
    }

    fn media_content(
        &self,
        view: gpui::WeakEntity<NotatusWindow>,
        colors: ThemeColor,
    ) -> impl IntoElement {
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
                .text_color(colors.muted_foreground)
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
            SidebarMenuElement(SidebarMenu::new().children(self.asset_items(view)))
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
                    let asset_kind = asset.kind.clone();
                    let done = asset.status == notatus_core::AssetStatus::Done;
                    SidebarMenuItem::new(compact_asset_name(asset))
                        .suffix(move |_, _| {
                            media_asset_actions(
                                &asset_kind,
                                annotation_count,
                                done,
                                asset_id,
                                remove_view.clone(),
                            )
                        })
                        .active(self.state.selected_asset == Some(asset_id))
                        .on_click(move |_, window, cx| {
                            let _ = select_view.update(cx, |notatus, cx| {
                                notatus.go_to_media(asset_id, window, cx);
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
    done: bool,
    asset_id: AssetId,
    view: gpui::WeakEntity<NotatusWindow>,
) -> impl IntoElement + use<> {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap_1()
        .when(done, |row| {
            row.child(themed(|theme| {
                Icon::new(IconName::CircleCheck)
                    .xsmall()
                    .text_color(theme.success)
            }))
        })
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

/// Renders a standalone [`SidebarMenu`] outside of a `Sidebar` container.
///
/// Since gpui-component 0.9 a `SidebarMenu` is a `SidebarItem` rather than an
/// element, so it needs a stable id and the window context to render.
#[derive(IntoElement)]
struct SidebarMenuElement(SidebarMenu);

impl RenderOnce for SidebarMenuElement {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.0.render("media-menu", window, cx)
    }
}

fn asset_element_id(prefix: &'static str, asset_id: AssetId) -> gpui::ElementId {
    let value = asset_id.as_uuid().as_u128();
    let high = (value >> 64) as u64;
    let low = (value as u64).to_string();
    (gpui::ElementId::from((prefix, high)), low).into()
}
