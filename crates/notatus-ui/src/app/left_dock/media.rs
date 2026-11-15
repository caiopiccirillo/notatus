//! The media list: virtualized, searchable by name or path, and filtered by
//! status, so it stays quick with thousands of images.

use crate::app::actions::{self, ImportFolder, ImportMedia};
use crate::app::helpers::*;
use crate::app::media_import::{open_folder_picker, open_media_picker};
use crate::app::window::NotatusWindow;
use gpui::prelude::*;
use gpui::{App, Context, IntoElement, SharedString, Subscription, Task, Window, div, px};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::list::{List, ListDelegate, ListEvent, ListItem, ListState};
use gpui_component::menu::{ContextMenuExt as _, DropdownMenu as _, PopupMenuItem};
use gpui_component::{
    ActiveTheme as _, Icon, IconName, IndexPath, Selectable as _, Sizable as _, ThemeColor,
};
use notatus_core::{AssetId, AssetStatus, ReviewState};
use std::collections::HashMap;

/// Which media the list shows.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::app) enum MediaFilter {
    #[default]
    All,
    Unfinished,
    Done,
    /// Media with drafts nobody has reviewed yet.
    ToReview,
}

impl MediaFilter {
    const ALL: [Self; 4] = [Self::All, Self::Unfinished, Self::Done, Self::ToReview];

    fn title(self) -> &'static str {
        match self {
            Self::All => "All media",
            Self::Unfinished => "Not done",
            Self::Done => "Done",
            Self::ToReview => "With drafts to review",
        }
    }

    fn accepts(self, row: &MediaRow) -> bool {
        match self {
            Self::All => true,
            Self::Unfinished => !row.done,
            Self::Done => row.done,
            Self::ToReview => row.drafts > 0,
        }
    }
}

/// What a media row shows, taken from the dataset when it changes.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::app) struct MediaRow {
    id: AssetId,
    name: SharedString,
    /// Lowercased name and path, for search.
    search_text: String,
    done: bool,
    annotations: usize,
    drafts: usize,
}

pub(in crate::app) fn media_rows(dataset: &notatus_core::Dataset) -> Vec<MediaRow> {
    // Counted in one pass rather than once per media.
    let mut counts: HashMap<AssetId, (usize, usize)> = HashMap::new();
    for annotation in &dataset.annotations {
        let entry = counts.entry(annotation.asset_id).or_default();
        entry.0 += 1;
        if annotation.review_state == ReviewState::Draft {
            entry.1 += 1;
        }
    }
    dataset
        .assets
        .iter()
        .map(|asset| {
            let (annotations, drafts) = counts.get(&asset.id).copied().unwrap_or_default();
            let name = asset_display_name(asset);
            MediaRow {
                id: asset.id,
                search_text: format!("{name}\n{}", asset.location.display_path()).to_lowercase(),
                name: name.into(),
                done: asset.status == AssetStatus::Done,
                annotations,
                drafts,
            }
        })
        .collect()
}

#[derive(Default)]
pub(in crate::app) struct MediaListDelegate {
    rows: Vec<MediaRow>,
    /// Indexes into `rows` that match the query and filter, in order.
    visible: Vec<usize>,
    query: String,
    filter: MediaFilter,
}

impl MediaListDelegate {
    fn refilter(&mut self) {
        let query = self.query.to_lowercase();
        self.visible = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| self.filter.accepts(row) && row.search_text.contains(&query))
            .map(|(index, _)| index)
            .collect();
    }

    pub(in crate::app) fn set_rows(&mut self, rows: Vec<MediaRow>) {
        self.rows = rows;
        self.refilter();
    }

    pub(in crate::app) fn filter(&self) -> MediaFilter {
        self.filter
    }

    pub(in crate::app) fn set_filter(&mut self, filter: MediaFilter) {
        self.filter = filter;
        self.refilter();
    }

    pub(in crate::app) fn asset_at(&self, row: usize) -> Option<AssetId> {
        self.visible.get(row).map(|&index| self.rows[index].id)
    }

    pub(in crate::app) fn position_of(&self, asset_id: AssetId) -> Option<usize> {
        self.visible
            .iter()
            .position(|&index| self.rows[index].id == asset_id)
    }
}

impl ListDelegate for MediaListDelegate {
    type Item = ListItem;

    fn perform_search(
        &mut self,
        query: &str,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        self.query = query.to_string();
        self.refilter();
        Task::ready(())
    }

    fn items_count(&self, _: usize, _: &App) -> usize {
        self.visible.len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let row = self.visible.get(ix.row).map(|&index| &self.rows[index])?;
        let colors = cx.theme().colors;
        // Done or not leads the row, so the remaining work can be scanned
        // down the list.
        let status = if row.done {
            Icon::new(IconName::CircleCheck).text_color(colors.success)
        } else {
            Icon::new(gpui_kit_assets::IconName::Circle).text_color(colors.muted_foreground)
        };
        // Everything sits in the item's one child: its suffix slot comes
        // after a full-width wrapper and would be pushed out of view.
        Some(
            ListItem::new(ix.row).h(px(32.0)).px_2().rounded_md().child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .min_w_0()
                    .text_sm()
                    .child(status.small())
                    .child(div().flex_1().min_w_0().truncate().child(row.name.clone()))
                    .when(row.drafts > 0, |line| {
                        line.child(
                            gpui_component::tag::Tag::warning()
                                .xsmall()
                                .outline()
                                .flex_none()
                                .child(row.drafts.to_string()),
                        )
                    })
                    .child(
                        div()
                            .flex_none()
                            .min_w(px(20.0))
                            .text_xs()
                            .text_right()
                            .text_color(colors.muted_foreground)
                            .child(row.annotations.to_string()),
                    ),
            ),
        )
    }

    fn render_empty(
        &mut self,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .p_4()
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .child("No media matches")
    }

    fn set_selected_index(
        &mut self,
        _: Option<IndexPath>,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) {
    }
}

pub(in crate::app) type MediaListState = ListState<MediaListDelegate>;

impl NotatusWindow {
    pub(in crate::app) fn new_media_list(
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (gpui::Entity<MediaListState>, Subscription) {
        let list =
            cx.new(|cx| ListState::new(MediaListDelegate::default(), window, cx).searchable(true));
        let subscription = cx.subscribe_in(&list, window, |this, list, event, window, cx| {
            if let ListEvent::Confirm(ix) = event
                && let Some(asset_id) = list.read(cx).delegate().asset_at(ix.row)
            {
                this.go_to_media(asset_id, window, cx);
            }
        });
        (list, subscription)
    }

    /// Brings the list up to date with the dataset and the current media,
    /// scrolling to the media when it changed. Cheap when neither did.
    pub(in crate::app) fn sync_media_list(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let key = (self.state.revision(), self.state.selected_asset());
        if self.media_list_synced == Some(key) {
            return;
        }
        let asset_changed = self.media_list_synced.map(|(_, asset)| asset) != Some(key.1);
        self.media_list_synced = Some(key);
        let rows = media_rows(self.state.dataset());
        let selected = self.state.selected_asset();
        self.media_list.update(cx, |list, cx| {
            if list.delegate().rows != rows {
                list.delegate_mut().set_rows(rows);
            }
            let row = selected.and_then(|asset| list.delegate().position_of(asset));
            list.set_selected_index(row.map(IndexPath::new), window, cx);
            if asset_changed {
                list.scroll_to_selected_item(window, cx);
            }
            cx.notify();
        });
    }

    fn set_media_filter(&mut self, filter: MediaFilter, cx: &mut Context<Self>) {
        self.media_list.update(cx, |list, cx| {
            list.delegate_mut().set_filter(filter);
            cx.notify();
        });
        // Reselect the current media at its new position.
        self.media_list_synced = None;
        cx.notify();
    }

    /// Filter and import buttons, at the end of the dock's tab bar.
    pub(super) fn media_actions(
        &self,
        view: gpui::WeakEntity<NotatusWindow>,
        cx: &App,
    ) -> impl IntoElement {
        let folder_view = view.clone();
        let filter_view = view.clone();
        let current = self.media_list.read(cx).delegate().filter();
        let has_media = !self.state.dataset().assets.is_empty();
        div()
            .flex()
            .gap_1()
            .when(has_media, |actions| {
                actions.child(
                    Button::new("media-filter")
                        .xsmall()
                        .ghost()
                        .icon(Icon::new(gpui_kit_assets::IconName::ListFilter))
                        .selected(current != MediaFilter::All)
                        .tooltip(current.title())
                        .dropdown_menu(move |menu, _, _| {
                            MediaFilter::ALL.into_iter().fold(menu, |menu, filter| {
                                let view = filter_view.clone();
                                menu.item(
                                    PopupMenuItem::new(filter.title())
                                        .checked(filter == current)
                                        .on_click(move |_, _, cx| {
                                            let _ = view.update(cx, |notatus, cx| {
                                                notatus.set_media_filter(filter, cx);
                                            });
                                        }),
                                )
                            })
                        }),
                )
            })
            .child(
                Button::new("media-import")
                    .xsmall()
                    .ghost()
                    .icon(Icon::new(IconName::Plus))
                    .tooltip_with_action("Import images", &ImportMedia, Some(actions::CONTEXT))
                    .on_click(move |_, window, cx| {
                        open_media_picker(view.clone(), window, cx);
                    }),
            )
            .child(
                Button::new("media-import-folder")
                    .xsmall()
                    .ghost()
                    .icon(Icon::new(IconName::FolderOpen))
                    .tooltip_with_action("Import folder", &ImportFolder, Some(actions::CONTEXT))
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
            return media_empty_state(colors).into_any_element();
        }
        let list = self.media_list.clone();
        div()
            .id("media-list")
            .size_full()
            .child(
                List::new(&self.media_list)
                    .small()
                    .p_1()
                    .search_placeholder("Search media"),
            )
            .context_menu(move |menu, _, cx| {
                let list = list.read(cx);
                let Some(asset_id) = list
                    .right_clicked_index()
                    .and_then(|ix| list.delegate().asset_at(ix.row))
                else {
                    return menu;
                };
                let view = view.clone();
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
            .into_any_element()
    }
}

/// The canvas holds the import buttons while the project has no media, so
/// the dock only says why it is empty.
fn media_empty_state(colors: ThemeColor) -> gpui::Div {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .p_4()
        .text_sm()
        .text_color(colors.muted_foreground)
        .child("No media yet")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UiState;
    use notatus_core::BoundingBox;

    fn delegate() -> (MediaListDelegate, Vec<AssetId>) {
        let mut state = UiState::new_project("Media");
        let label = state.add_label("Thing");
        let ids: Vec<_> = ["/data/cat.png", "/data/dog.png", "/other/bird.png"]
            .into_iter()
            .map(|path| {
                state
                    .add_local_image_asset(path.to_string(), 100, 100)
                    .unwrap()
            })
            .collect();
        state.set_asset_status(ids[0], AssetStatus::Done).unwrap();
        let bbox = BoundingBox::from_xywh(1.0, 1.0, 5.0, 5.0).unwrap();
        state.add_human_bbox(ids[1], label, bbox, None).unwrap();
        let mut delegate = MediaListDelegate::default();
        delegate.set_rows(media_rows(state.dataset()));
        (delegate, ids)
    }

    fn shown(delegate: &MediaListDelegate) -> Vec<AssetId> {
        (0..delegate.visible.len())
            .filter_map(|row| delegate.asset_at(row))
            .collect()
    }

    #[test]
    fn search_matches_names_and_folders() {
        let (mut delegate, ids) = delegate();
        delegate.query = "DOG".into();
        delegate.refilter();
        assert_eq!(shown(&delegate), vec![ids[1]]);

        delegate.query = "/data/".into();
        delegate.refilter();
        assert_eq!(shown(&delegate), vec![ids[0], ids[1]]);
    }

    #[test]
    fn filters_narrow_by_status_and_drafts() {
        let (mut delegate, ids) = delegate();
        delegate.set_filter(MediaFilter::Done);
        assert_eq!(shown(&delegate), vec![ids[0]]);
        delegate.set_filter(MediaFilter::Unfinished);
        assert_eq!(shown(&delegate), vec![ids[1], ids[2]]);
        delegate.set_filter(MediaFilter::ToReview);
        assert_eq!(shown(&delegate), vec![ids[1]]);
        assert_eq!(delegate.position_of(ids[1]), Some(0));
        assert_eq!(delegate.position_of(ids[0]), None);
    }
}

#[cfg(test)]
mod window_tests {
    use crate::app::tests::support::open_test_window;
    use gpui::TestAppContext;
    use gpui_component::IndexPath;
    use gpui_component::list::ListEvent;

    #[gpui::test]
    fn the_list_follows_the_current_media_and_opens_what_is_clicked(cx: &mut TestAppContext) {
        let (window, view) = open_test_window(cx);
        let ids = view.update(cx, |notatus, cx| {
            let ids: Vec<_> = ["/tmp/a.png", "/tmp/b.png", "/tmp/c.png"]
                .into_iter()
                .map(|path| notatus.state.add_local_image_asset(path, 10, 10).unwrap())
                .collect();
            notatus.state.select_asset(ids[0]).unwrap();
            cx.notify();
            ids
        });
        cx.run_until_parked();

        cx.simulate_keystrokes(window.into(), "d");
        cx.run_until_parked();
        let selected_row = view.update(cx, |notatus, cx| {
            notatus
                .media_list
                .read(cx)
                .selected_index()
                .map(|ix| ix.row)
        });
        assert_eq!(selected_row, Some(1), "the list marks the media D went to");

        // A click on a row confirms it.
        let list = view.read_with(cx, |notatus, _| notatus.media_list.clone());
        list.update(cx, |_, cx| cx.emit(ListEvent::Confirm(IndexPath::new(2))));
        cx.run_until_parked();
        view.read_with(cx, |notatus, _| {
            assert_eq!(notatus.state.selected_asset(), Some(ids[2]));
        });
    }
}
