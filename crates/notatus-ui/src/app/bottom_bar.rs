use super::actions::{DoneAndNext, NextMedia, PreviousMedia, ToggleMediaDone};
use super::*;
use gpui::Action;

impl NotatusWindow {
    pub(super) fn bottom_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        div()
            .flex_none()
            .h(px(40.0))
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .px_3()
            .border_t_1()
            .border_color(cx.theme().status_bar_border)
            .bg(cx.theme().status_bar)
            .text_xs()
            .text_color(muted)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .flex_none()
                    .child(self.bottom_left_dock_button(
                        "bottom-dataset",
                        IconName::LayoutDashboard,
                        "Dataset",
                        LeftDock::Dataset,
                        cx,
                    ))
                    .child(self.bottom_left_dock_button(
                        "bottom-media",
                        IconName::GalleryVerticalEnd,
                        "Media",
                        LeftDock::Media,
                        cx,
                    ))
                    .when_some(self.status_message.clone(), |bar, message| {
                        bar.child(
                            div()
                                .id("status-message")
                                .ml_2()
                                .max_w(px(360.0))
                                .truncate()
                                .child(message),
                        )
                    }),
            )
            .child(self.review_controls())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .flex_none()
                    .child(div().mr_2().child(self.save_status()))
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
                        RightDock::Info,
                        cx,
                    )),
            )
    }

    /// Previous/next media, progress, and the done toggle, centered in the bar.
    fn review_controls(&self) -> impl IntoElement {
        let Some(progress) = self.progress_summary() else {
            return div().child("Press Ctrl+K for commands").into_any_element();
        };
        let done = self
            .selected_asset()
            .is_some_and(|asset| asset.status == notatus_core::AssetStatus::Done);
        let has_asset = self.state.selected_asset.is_some();

        div()
            .flex()
            .items_center()
            .gap_1()
            .min_w_0()
            .child(action_button(
                "status-previous",
                IconName::ChevronLeft,
                "Previous media",
                PreviousMedia,
            ))
            .child(div().px_1().child(progress))
            .child(action_button(
                "status-next",
                IconName::ChevronRight,
                "Next media",
                NextMedia,
            ))
            .when_some(self.current_annotation_summary(), |row, summary| {
                row.child(div().px_1().child(summary))
            })
            .when(has_asset, |row| {
                row.child(
                    Button::new("status-done")
                        .xsmall()
                        .icon(Icon::new(IconName::CircleCheck))
                        .label(if done { "Done" } else { "Mark done" })
                        .selected(done)
                        .tooltip_with_action(
                            "Toggle done",
                            &ToggleMediaDone,
                            Some(actions::CONTEXT),
                        )
                        .on_click(|_, window, cx| {
                            window.dispatch_action(ToggleMediaDone.boxed_clone(), cx)
                        }),
                )
                .child(
                    Button::new("status-done-next")
                        .xsmall()
                        .ghost()
                        .label("Done & next")
                        .tooltip_with_action(
                            "Mark done and go to next pending",
                            &DoneAndNext,
                            Some(actions::CONTEXT),
                        )
                        .on_click(|_, window, cx| {
                            window.dispatch_action(DoneAndNext.boxed_clone(), cx)
                        }),
                )
            })
            .into_any_element()
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

/// A compact icon button that dispatches `action`, with its shortcut in the tooltip.
fn action_button<A: Action>(
    id: &'static str,
    icon: IconName,
    tooltip: &'static str,
    action: A,
) -> Button {
    Button::new(id)
        .xsmall()
        .ghost()
        .icon(Icon::new(icon))
        .tooltip_with_action(tooltip, &action, Some(actions::CONTEXT))
        .on_click(move |_, window, cx| window.dispatch_action(action.boxed_clone(), cx))
}
