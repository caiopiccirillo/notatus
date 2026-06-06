use super::helpers::compact_text;
use super::*;

impl NotatusWindow {
    pub(super) fn app_titlebar(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        let project_name = compact_text(&self.state.dataset.manifest.project.name, 36);
        let state = if self.state.dirty { "Unsaved" } else { "Saved" };

        TitleBar::new().child(
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
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x6b7280))
                                .child("visual annotation"),
                        ),
                )
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap_2()
                        .text_xs()
                        .text_color(rgb(0x6b7280))
                        .child(project_name)
                        .child("·")
                        .child(state),
                ),
        )
    }
}
