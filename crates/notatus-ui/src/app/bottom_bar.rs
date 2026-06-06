use super::*;

impl NotatusWindow {
    pub(super) fn bottom_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_none()
            .h(px(40.0))
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .px_3()
            .border_t_1()
            .border_color(rgb(0xd6d9de))
            .bg(rgb(0xf8fafc))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .min_w_0()
                    .child(self.bottom_left_dock_button(
                        "bottom-project",
                        IconName::LayoutDashboard,
                        "Project",
                        LeftDock::Project,
                        cx,
                    ))
                    .child(self.bottom_left_dock_button(
                        "bottom-labels",
                        IconName::Palette,
                        "Labels",
                        LeftDock::Labels,
                        cx,
                    ))
                    .child(self.bottom_left_dock_button(
                        "bottom-media",
                        IconName::GalleryVerticalEnd,
                        "Media",
                        LeftDock::Media,
                        cx,
                    )),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .min_w_0()
                    .child(self.bottom_right_dock_button(
                        "bottom-annotations",
                        IconName::Frame,
                        "Annotations",
                        RightDock::Annotations,
                        cx,
                    ))
                    .child(self.bottom_right_dock_button(
                        "bottom-info",
                        IconName::Info,
                        "Info",
                        RightDock::MediaInfo,
                        cx,
                    )),
            )
    }

    fn bottom_left_dock_button(
        &self,
        id: &'static str,
        icon: IconName,
        tooltip: &'static str,
        dock: LeftDock,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let view = cx.weak_entity();
        Button::new(id)
            .small()
            .icon(Icon::new(icon))
            .tooltip(tooltip)
            .selected(self.left_dock == dock)
            .on_click(move |_, _, cx| {
                let _ = view.update(cx, |notatus, cx| {
                    notatus.left_dock = dock;
                    cx.notify();
                });
            })
    }

    fn bottom_right_dock_button(
        &self,
        id: &'static str,
        icon: IconName,
        tooltip: &'static str,
        dock: RightDock,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let view = cx.weak_entity();
        Button::new(id)
            .small()
            .icon(Icon::new(icon))
            .tooltip(tooltip)
            .selected(self.right_dock == dock)
            .on_click(move |_, _, cx| {
                let _ = view.update(cx, |notatus, cx| {
                    notatus.right_dock = dock;
                    cx.notify();
                });
            })
    }
}
