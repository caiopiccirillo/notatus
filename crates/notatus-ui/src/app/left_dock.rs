use crate::app::dock::{DockTabs, dock_frame};
use crate::app::window::{LeftDock, NotatusWindow};
use gpui::prelude::*;
use gpui::{Context, IntoElement};
use gpui_component::ActiveTheme as _;

mod labels;
mod media;

pub(super) use media::MediaListState;

impl NotatusWindow {
    pub(super) fn left_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.weak_entity();
        let (selected, actions, content) = match self.left_dock {
            LeftDock::Media => (
                0,
                self.media_actions(view.clone(), cx).into_any_element(),
                self.media_content(view.clone(), cx).into_any_element(),
            ),
            LeftDock::Labels => (
                1,
                labels::labels_actions(view.clone()).into_any_element(),
                labels::labels_content(self, view.clone(), cx.theme().colors).into_any_element(),
            ),
        };
        dock_frame(
            DockTabs {
                id: "left-dock-tabs",
                titles: &["Media", "Labels"],
                selected,
                on_select: |index, notatus: &mut NotatusWindow| {
                    notatus.left_dock = if index == 0 {
                        LeftDock::Media
                    } else {
                        LeftDock::Labels
                    };
                },
            },
            view,
            actions,
            content,
            cx,
        )
        .border_r_1()
        .border_color(cx.theme().border)
    }
}
