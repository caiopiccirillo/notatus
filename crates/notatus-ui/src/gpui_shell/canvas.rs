use super::helpers::*;
use super::*;

impl NotatusWindow {
    pub(super) fn canvas_placeholder(&self) -> impl IntoElement {
        let selected_asset = self.selected_asset();

        div()
            .size_full()
            .p_6()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgb(0xf3f4f6))
            .child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .border_1()
                    .border_color(rgb(0xcbd5e1))
                    .bg(rgb(0xffffff))
                    .overflow_hidden()
                    .when_some(selected_asset, |canvas, asset| {
                        canvas.child(image_canvas_content(asset))
                    })
                    .when(selected_asset.is_none(), |canvas| {
                        canvas
                            .child(div().text_lg().child("Choose images to start"))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0x4b5563))
                                    .child("No media selected"),
                            )
                    }),
            )
    }
}
