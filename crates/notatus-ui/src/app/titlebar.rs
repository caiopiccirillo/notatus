use super::media_import::open_media_picker;
use super::*;

impl NotatusWindow {
    pub(super) fn app_titlebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
                        .gap_1()
                        .child(self.title_project_menu(cx))
                        .child(self.title_media_menu(cx))
                        .child(self.title_labels_menu(cx))
                        .child(self.title_export_menu(cx)),
                ),
        )
    }

    fn title_project_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.weak_entity();
        Button::new("title-project-menu")
            .small()
            .label("Project")
            .dropdown_caret(true)
            .selected(self.left_dock == LeftDock::Project)
            .dropdown_menu(move |menu, _, _| {
                menu.item(PopupMenuItem::new("Show datasets").on_click({
                    let view = view.clone();
                    move |_, _, cx| {
                        let _ = view.update(cx, |notatus, cx| {
                            notatus.left_dock = LeftDock::Project;
                            cx.notify();
                        });
                    }
                }))
                .item(PopupMenuItem::separator())
                .item(PopupMenuItem::new("New dataset").disabled(true))
                .item(PopupMenuItem::new("Open dataset").disabled(true))
            })
    }

    fn title_media_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.weak_entity();
        let import_view = cx.weak_entity();
        Button::new("title-media-menu")
            .small()
            .label("Media")
            .dropdown_caret(true)
            .selected(self.left_dock == LeftDock::Media)
            .dropdown_menu(move |menu, _, _| {
                menu.item(PopupMenuItem::new("Show media").on_click({
                    let view = view.clone();
                    move |_, _, cx| {
                        let _ = view.update(cx, |notatus, cx| {
                            notatus.left_dock = LeftDock::Media;
                            cx.notify();
                        });
                    }
                }))
                .item(PopupMenuItem::new("Import media").on_click({
                    let import_view = import_view.clone();
                    move |_, _, cx| {
                        open_media_picker(import_view.clone(), cx);
                    }
                }))
            })
    }

    fn title_labels_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.weak_entity();
        let label_view = cx.weak_entity();
        Button::new("title-labels-menu")
            .small()
            .label("Labels")
            .dropdown_caret(true)
            .selected(self.left_dock == LeftDock::Labels)
            .dropdown_menu(move |menu, _, _| {
                menu.item(PopupMenuItem::new("Show labels").on_click({
                    let view = view.clone();
                    move |_, _, cx| {
                        let _ = view.update(cx, |notatus, cx| {
                            notatus.left_dock = LeftDock::Labels;
                            cx.notify();
                        });
                    }
                }))
                .item(PopupMenuItem::new("Add label").on_click({
                    let label_view = label_view.clone();
                    move |_, window, cx| {
                        let _ = label_view.update(cx, |notatus, cx| {
                            notatus.create_label(window, cx);
                        });
                    }
                }))
            })
    }

    fn title_export_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.weak_entity();
        Button::new("title-export-menu")
            .small()
            .label("Export")
            .dropdown_caret(true)
            .dropdown_menu(move |menu, _, _| {
                menu.item(PopupMenuItem::new("Export dataset").on_click({
                    let view = view.clone();
                    move |_, _, cx| {
                        let _ = view.update(cx, |notatus, cx| {
                            notatus.status_message =
                                Some("Export is not implemented yet".to_string());
                            notatus.right_dock = RightDock::MediaInfo;
                            cx.notify();
                        });
                    }
                }))
                .item(PopupMenuItem::new("Export annotations").disabled(true))
            })
    }
}
