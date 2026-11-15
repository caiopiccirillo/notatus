use super::helpers::compact_text;
use super::*;

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
    pub(super) fn app_titlebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let project_name = compact_text(&self.state.dataset().manifest.project.name, 36);
        let state = self.save_status();
        let muted = cx.theme().muted_foreground;

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
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .min_w_0()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .min_w_0()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Notatus"),
                            )
                            .child(div().text_xs().text_color(muted).child("visual annotation")),
                    )
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap_2()
                            .text_xs()
                            .text_color(muted)
                            .child(project_name)
                            .child("·")
                            .child(state),
                    ),
            )
    }
}
