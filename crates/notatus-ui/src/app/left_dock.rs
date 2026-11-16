use crate::app::dock::{DockTabs, DockView, dock_frame};
use crate::app::window::{LeftDock, NotatusWindow};
use gpui::prelude::*;
use gpui::{Context, IntoElement};
use gpui_component::ActiveTheme as _;

mod labels;
mod media;

pub(super) use media::MediaListState;

impl DockView for LeftDock {
    const ALL: &'static [Self] = &[Self::Media, Self::Labels];

    fn title(self) -> &'static str {
        match self {
            Self::Media => "Media",
            Self::Labels => "Labels",
        }
    }
}

impl NotatusWindow {
    pub(super) fn left_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.weak_entity();
        let (actions, content) = match self.left_dock {
            LeftDock::Media => (
                self.media_actions(view.clone(), cx).into_any_element(),
                self.media_content(view.clone(), cx).into_any_element(),
            ),
            LeftDock::Labels => (
                labels::labels_actions(view.clone()).into_any_element(),
                labels::labels_content(self, view.clone(), cx.theme().colors).into_any_element(),
            ),
        };
        dock_frame(
            DockTabs {
                id: "left-dock-tabs",
                current: self.left_dock,
                select: |notatus, dock| notatus.left_dock = dock,
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
