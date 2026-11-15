//! The frame shared by the side docks: a tab bar naming each view, with the
//! current view's actions at its end, above the view itself.

use super::*;
use gpui_component::tab::{Tab, TabBar};

/// A dock's views: their titles, the one shown, and what a click on a tab
/// does with its index.
pub(super) struct DockTabs<F> {
    pub(super) id: &'static str,
    pub(super) titles: &'static [&'static str],
    pub(super) selected: usize,
    pub(super) on_select: F,
}

/// A dock showing `tabs`, with `actions` at the right end of the tab bar.
pub(super) fn dock_frame(
    tabs: DockTabs<impl Fn(usize, &mut NotatusWindow) + 'static>,
    view: gpui::WeakEntity<NotatusWindow>,
    actions: impl IntoElement,
    content: impl IntoElement,
    cx: &App,
) -> gpui::Div {
    let colors = cx.theme().colors;
    div()
        .size_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .bg(colors.sidebar)
        .child({
            let on_select = tabs.on_select;
            TabBar::new(tabs.id)
                .underline()
                .small()
                .px_2()
                .selected_index(tabs.selected)
                .children(tabs.titles.iter().map(|title| Tab::new().label(*title)))
                .suffix(div().flex().items_center().gap_1().pr_2().child(actions))
                .on_click(move |index, _, cx| {
                    let index = *index;
                    let _ = view.update(cx, |notatus, cx| {
                        on_select(index, notatus);
                        cx.notify();
                    });
                })
        })
        .child(div().flex_1().min_h_0().overflow_hidden().child(content))
}
