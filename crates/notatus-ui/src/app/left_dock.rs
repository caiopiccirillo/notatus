use super::*;

mod labels;
mod media;
mod project;

impl NotatusWindow {
    pub(super) fn left_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .border_r_1()
            .border_color(rgb(0xd6d9de))
            .bg(rgb(0xffffff))
            .overflow_hidden()
            .child(match self.left_dock {
                LeftDock::Dataset => self.dataset_dock(cx).into_any_element(),
                LeftDock::Media => self.media_dock(cx).into_any_element(),
            })
    }
}
