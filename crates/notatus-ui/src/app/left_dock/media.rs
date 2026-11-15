use super::*;
use crate::app::helpers::*;
use crate::app::media_import::{open_folder_picker, open_media_picker};
use gpui::StyleRefinement;
use std::collections::HashMap;

impl NotatusWindow {
    /// Import buttons, at the end of the dock's tab bar.
    pub(super) fn media_actions(&self, view: gpui::WeakEntity<NotatusWindow>) -> impl IntoElement {
        let folder_view = view.clone();
        div()
            .flex()
            .gap_1()
            .child(
                Button::new("media-import")
                    .xsmall()
                    .ghost()
                    .icon(Icon::new(IconName::Plus))
                    .tooltip_with_action(
                        "Import images",
                        &crate::app::actions::ImportMedia,
                        Some(crate::app::actions::CONTEXT),
                    )
                    .on_click(move |_, window, cx| {
                        open_media_picker(view.clone(), window, cx);
                    }),
            )
            .child(
                Button::new("media-import-folder")
                    .xsmall()
                    .ghost()
                    .icon(Icon::new(IconName::FolderOpen))
                    .tooltip_with_action(
                        "Import folder",
                        &crate::app::actions::ImportFolder,
                        Some(crate::app::actions::CONTEXT),
                    )
                    .on_click(move |_, window, cx| {
                        open_folder_picker(folder_view.clone(), window, cx);
                    }),
            )
    }

    pub(super) fn media_content(
        &self,
        view: gpui::WeakEntity<NotatusWindow>,
        cx: &App,
    ) -> impl IntoElement {
        let colors = cx.theme().colors;
        if self.state.dataset().assets.is_empty() {
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
                .child("Import images or a whole folder to start")
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            Button::new("media-empty-import")
                                .small()
                                .icon(Icon::new(IconName::Plus))
                                .label("Images")
                                .on_click({
                                    let view = import_view.clone();
                                    move |_, window, cx| {
                                        open_media_picker(view.clone(), window, cx);
                                    }
                                }),
                        )
                        .child(
                            Button::new("media-empty-import-folder")
                                .small()
                                .icon(Icon::new(IconName::FolderOpen))
                                .label("Folder")
                                .on_click(move |_, window, cx| {
                                    open_folder_picker(import_view.clone(), window, cx);
                                }),
                        ),
                )
                .into_any_element()
        } else {
            div()
                .size_full()
                .p_2()
                .child(SidebarMenuElement(
                    SidebarMenu::new().children(self.asset_items(view, colors)),
                ))
                .into_any_element()
        }
    }

    fn asset_items(
        &self,
        view: gpui::WeakEntity<NotatusWindow>,
        colors: ThemeColor,
    ) -> Vec<SidebarMenuItem> {
        if self.state.dataset().assets.is_empty() {
            return vec![SidebarMenuItem::new("No media yet").disable(true)];
        }

        // Counted in one pass, so rendering the list does not rescan every
        // annotation once per media item.
        let mut annotation_counts: HashMap<AssetId, usize> = HashMap::new();
        for annotation in &self.state.dataset().annotations {
            *annotation_counts.entry(annotation.asset_id).or_default() += 1;
        }

        self.state
            .dataset()
            .assets
            .iter()
            .map(|asset| {
                let asset_id = asset.id;
                let annotation_count = annotation_counts.get(&asset_id).copied().unwrap_or(0);
                let done = asset.status == notatus_core::AssetStatus::Done;
                let select_view = view.clone();
                let menu_view = view.clone();
                // Done or not leads the row, so the remaining work can be
                // scanned down the list.
                let status_icon = if done {
                    Icon::new(IconName::CircleCheck).text_color(colors.success)
                } else {
                    Icon::new(gpui_kit_assets::IconName::Circle).text_color(colors.muted_foreground)
                };
                SidebarMenuItem::new(asset_display_name(asset))
                    .icon(status_icon)
                    .label_style(StyleRefinement::default().truncate())
                    .suffix(move |_, _| sidebar_count(annotation_count.to_string()))
                    .active(self.state.selected_asset() == Some(asset_id))
                    .on_click(move |_, window, cx| {
                        let _ = select_view.update(cx, |notatus, cx| {
                            notatus.go_to_media(asset_id, window, cx);
                        });
                    })
                    .context_menu(move |menu, _, _| {
                        let view = menu_view.clone();
                        menu.item(
                            PopupMenuItem::new("Remove media and its annotations")
                                .icon(Icon::new(IconName::Delete))
                                .on_click(move |_, _, cx| {
                                    let _ = view.update(cx, |notatus, cx| {
                                        notatus.remove_asset(asset_id, cx);
                                    });
                                }),
                        )
                    })
            })
            .collect()
    }
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
