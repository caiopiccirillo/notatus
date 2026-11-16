use crate::app::helpers::compact_text;
use crate::app::window::NotatusWindow;
use gpui::prelude::*;
use gpui::{App, Context, FontWeight, IntoElement, Window, div, px};
use gpui_component::{ActiveTheme as _, TitleBar};

/// The title bar's close button: closes the window only when
/// [`NotatusWindow::should_close`] allows it, which may first save or ask.
pub(super) fn close_from_title_bar(
    view: &gpui::WeakEntity<NotatusWindow>,
    window: &mut Window,
    cx: &mut App,
) {
    let may_close = view
        .update(cx, |notatus, cx| notatus.should_close(window, cx))
        .unwrap_or(true);
    if may_close {
        window.remove_window();
    }
}

impl NotatusWindow {
    /// The menus on the left; the project and whether it is saved on the
    /// right, the one place the save state is shown.
    pub(super) fn app_titlebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let project_name = compact_text(&self.state.dataset().manifest.project.name, 48);
        let muted = cx.theme().muted_foreground;
        let unsaved = self.has_unsaved_changes();

        // On Linux the window may draw its own title bar, and its close
        // button would otherwise remove the window directly, skipping the
        // save-before-close check that compositor close requests get.
        let view = cx.weak_entity();
        TitleBar::new()
            .on_close_window(move |_, window, cx| close_from_title_bar(&view, window, cx))
            .child(
                div()
                    .flex()
                    .w_full()
                    .h_full()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .min_w_0()
                    .pr_3()
                    .child(match &self.app_menu_bar {
                        Some(menu_bar) => div()
                            .h_full()
                            .flex()
                            .items_center()
                            .child(menu_bar.clone())
                            .into_any_element(),
                        None => div().into_any_element(),
                    })
                    .child(
                        div()
                            .id("title-project")
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap_2()
                            .text_xs()
                            .child(div().font_weight(FontWeight::MEDIUM).child(project_name))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .text_color(muted)
                                    .when(unsaved, |state| {
                                        state.child(
                                            div()
                                                .size(px(6.0))
                                                .rounded_full()
                                                .bg(cx.theme().warning),
                                        )
                                    })
                                    .child(self.save_status()),
                            ),
                    ),
            )
    }

    /// Whether closing now would lose edits: never saved, or edited since
    /// the last save and not about to be autosaved.
    fn has_unsaved_changes(&self) -> bool {
        self.state.is_dirty() && !(self.autosave.enabled && self.project_location.path().is_some())
    }
}
