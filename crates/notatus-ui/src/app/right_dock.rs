use crate::app::dock::{DockTabs, DockView, dock_frame};
use crate::app::window::{NotatusWindow, RightDock};
use gpui::prelude::*;
use gpui::{Context, IntoElement, div};
use gpui_component::ActiveTheme as _;

mod annotations;
mod details;

pub(super) use annotations::Labels;

impl DockView for RightDock {
    const ALL: &'static [Self] = &[Self::Annotations, Self::Details];

    fn title(self) -> &'static str {
        match self {
            Self::Annotations => "Annotations",
            Self::Details => "Details",
        }
    }
}

impl NotatusWindow {
    pub(super) fn right_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.weak_entity();
        let colors = cx.theme().colors;
        let content = match self.right_dock {
            RightDock::Annotations => self.annotations_panel_content(view.clone(), colors),
            RightDock::Details => self.details_panel_content(cx).into_any_element(),
        };
        dock_frame(
            DockTabs {
                id: "right-dock-tabs",
                current: self.right_dock,
                select: |notatus, dock| notatus.right_dock = dock,
            },
            view,
            div(),
            content,
            cx,
        )
        .border_l_1()
        .border_color(colors.border)
    }
}
