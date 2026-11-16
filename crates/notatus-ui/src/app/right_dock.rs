use crate::app::dock::{DockTabs, dock_frame};
use crate::app::window::{NotatusWindow, RightDock};
use gpui::prelude::*;
use gpui::{Context, IntoElement, div};
use gpui_component::ActiveTheme as _;

mod annotations;
mod details;

pub(super) use annotations::Labels;

impl NotatusWindow {
    pub(super) fn right_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.weak_entity();
        let colors = cx.theme().colors;
        let (selected, content) = match self.right_dock {
            RightDock::Annotations => (0, self.annotations_panel_content(view.clone(), colors)),
            RightDock::Details => (1, self.details_panel_content(cx).into_any_element()),
        };
        dock_frame(
            DockTabs {
                id: "right-dock-tabs",
                titles: &["Annotations", "Details"],
                selected,
                on_select: |index, notatus: &mut NotatusWindow| {
                    notatus.right_dock = if index == 0 {
                        RightDock::Annotations
                    } else {
                        RightDock::Details
                    };
                },
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
