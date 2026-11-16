//! The frame shared by the side docks: a tab bar naming each view, with the
//! current view's actions at its end, above the view itself.

use crate::app::window::NotatusWindow;
use gpui::prelude::*;
use gpui::{App, IntoElement, div};
use gpui_component::tab::{Tab, TabBar};
use gpui_component::{ActiveTheme as _, Sizable as _};

/// The views a dock switches between, one tab each.
pub(super) trait DockView: Copy + PartialEq + 'static {
    /// Every view, in tab order.
    const ALL: &'static [Self];
    fn title(self) -> &'static str;
}

/// A dock's tabs: the view shown and how a click switches views.
pub(super) struct DockTabs<V> {
    pub(super) id: &'static str,
    pub(super) current: V,
    pub(super) select: fn(&mut NotatusWindow, V),
}

/// A dock showing `tabs`, with `actions` at the right end of the tab bar.
pub(super) fn dock_frame<V: DockView>(
    tabs: DockTabs<V>,
    view: gpui::WeakEntity<NotatusWindow>,
    actions: impl IntoElement,
    content: impl IntoElement,
    cx: &App,
) -> gpui::Div {
    let colors = cx.theme().colors;
    let selected = V::ALL
        .iter()
        .position(|candidate| *candidate == tabs.current)
        .unwrap_or(0);
    let select = tabs.select;
    div()
        .size_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .bg(colors.sidebar)
        .child(
            TabBar::new(tabs.id)
                .underline()
                .small()
                .px_2()
                .selected_index(selected)
                .children(V::ALL.iter().map(|view| Tab::new().label(view.title())))
                .suffix(div().flex().items_center().gap_1().pr_2().child(actions))
                .on_click(move |index, _, cx| {
                    let Some(&target) = V::ALL.get(*index) else {
                        return;
                    };
                    let _ = view.update(cx, |notatus, cx| {
                        select(notatus, target);
                        cx.notify();
                    });
                }),
        )
        .child(div().flex_1().min_h_0().overflow_hidden().child(content))
}
