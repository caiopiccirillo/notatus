use super::helpers::*;
use super::media_import::open_media_picker;
use super::project_commands;
use super::*;

impl NotatusWindow {
    pub(super) fn left_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .border_r_1()
            .border_color(rgb(0xd6d9de))
            .bg(rgb(0xffffff))
            .overflow_hidden()
            .child(match self.left_dock {
                LeftDock::Project => self.project_dock(cx).into_any_element(),
                LeftDock::Media => self.media_dock(cx).into_any_element(),
                LeftDock::Labels => self.labels_dock(cx).into_any_element(),
            })
    }

    fn project_dock(&self, cx: &mut Context<Self>) -> gpui::Div {
        let view = cx.weak_entity();

        div()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(0xe5e7eb))
                    .child(section_title("Project"))
                    .child(sidebar_count("1")),
            )
            .child(
                div()
                    .flex_1()
                    .overflow_y_scrollbar()
                    .p_2()
                    .child(SidebarMenu::new().children(self.dataset_items()))
                    .child(self.project_editor(view)),
            )
    }

    fn media_dock(&self, cx: &mut Context<Self>) -> gpui::Div {
        let view = cx.weak_entity();
        let import_view = cx.weak_entity();

        div()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(0xe5e7eb))
                    .child(section_title("Media"))
                    .child(
                        Button::new("media-import")
                            .small()
                            .icon(Icon::new(IconName::Plus))
                            .tooltip("Import media")
                            .on_click(move |_, window, cx| {
                                open_media_picker(import_view.clone(), window, cx);
                            }),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .p_2()
                    .child(self.media_content(view)),
            )
    }

    fn dataset_items(&self) -> Vec<SidebarMenuItem> {
        let dataset_name = self.state.dataset.manifest.project.name.clone();
        let summary = self.project_summary();

        vec![
            SidebarMenuItem::new(dataset_name)
                .suffix(sidebar_count(if self.state.dirty {
                    "Unsaved"
                } else {
                    "Saved"
                }))
                .default_open(true)
                .active(true)
                .children(vec![
                    SidebarMenuItem::new(summary).disable(true),
                    SidebarMenuItem::new(dataset_created_label(&self.state.dataset)).disable(true),
                ]),
        ]
    }

    fn project_editor(&self, view: gpui::WeakEntity<NotatusWindow>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .px_2()
            .pt_3()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(section_title("Name"))
                    .child(Input::new(&self.project_name_input).small().w_full()),
            )
            .child(metric("Location", self.project_location.display_name()))
            .child(metric(
                "Status",
                if self.state.dirty { "Unsaved" } else { "Saved" }.to_string(),
            ))
            .child(metric("Summary", self.project_summary()))
            .child(dataset_created_label(&self.state.dataset))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        Button::new("project-save")
                            .small()
                            .icon(Icon::new(IconName::Folder))
                            .label("Save")
                            .on_click({
                                let view = view.clone();
                                move |_, window, cx| {
                                    project_commands::save_project(view.clone(), window, cx);
                                }
                            }),
                    )
                    .child(
                        Button::new("project-save-as")
                            .small()
                            .icon(Icon::new(IconName::Folder))
                            .label("Save as")
                            .on_click({
                                let view = view.clone();
                                move |_, window, cx| {
                                    project_commands::save_project_as(view.clone(), window, cx);
                                }
                            }),
                    )
                    .child(
                        Button::new("project-new")
                            .small()
                            .icon(Icon::new(IconName::Plus))
                            .label("New")
                            .on_click({
                                let view = view.clone();
                                move |_, window, cx| {
                                    project_commands::new_project(view.clone(), window, cx);
                                }
                            }),
                    )
                    .child(
                        Button::new("project-open")
                            .small()
                            .icon(Icon::new(IconName::FolderOpen))
                            .label("Open")
                            .on_click(move |_, window, cx| {
                                project_commands::open_project(view.clone(), window, cx);
                            }),
                    ),
            )
    }

    fn media_content(&self, view: gpui::WeakEntity<NotatusWindow>) -> impl IntoElement {
        if self.state.dataset.assets.is_empty() {
            let import_view = view.clone();
            div()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_3()
                .p_4()
                .text_sm()
                .text_color(rgb(0x6b7280))
                .child("Import media after setting up project labels")
                .child(
                    Button::new("media-empty-import")
                        .small()
                        .icon(Icon::new(IconName::Plus))
                        .label("Import media")
                        .on_click(move |_, window, cx| {
                            open_media_picker(import_view.clone(), window, cx);
                        }),
                )
                .into_any_element()
        } else {
            SidebarMenu::new()
                .children(self.asset_items(view))
                .into_any_element()
        }
    }

    fn asset_items(&self, view: gpui::WeakEntity<NotatusWindow>) -> Vec<SidebarMenuItem> {
        if self.state.dataset.assets.is_empty() {
            vec![SidebarMenuItem::new("No media yet").disable(true)]
        } else {
            self.state
                .dataset
                .assets
                .iter()
                .map(|asset| {
                    let asset_id = asset.id;
                    let annotation_count = self
                        .state
                        .dataset
                        .annotations
                        .iter()
                        .filter(|annotation| annotation.asset_id == asset_id)
                        .count();
                    let annotation_items = annotation_items_for_asset(&self.state, asset);
                    let view = view.clone();
                    SidebarMenuItem::new(compact_asset_name(asset))
                        .suffix(media_asset_meta(&asset.kind, annotation_count))
                        .default_open(self.state.selected_asset == Some(asset_id))
                        .children(annotation_items)
                        .active(self.state.selected_asset == Some(asset_id))
                        .on_click(move |_, _, cx| {
                            let _ = view.update(cx, |window, cx| {
                                if let Err(error) = window.state.select_asset(asset_id) {
                                    window.status_message = Some(error.to_string());
                                } else {
                                    window.tools.fit_canvas_to_view();
                                }
                                cx.notify();
                            });
                        })
                })
                .collect()
        }
    }

    fn labels_dock(&self, cx: &mut Context<Self>) -> gpui::Div {
        let view = cx.weak_entity();
        let add_label_view = cx.weak_entity();
        let empty_label_view = cx.weak_entity();
        let label_items = self.label_items(view.clone());
        let selected_label = self.selected_label();

        div()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(0xe5e7eb))
                    .child(section_title("Labels"))
                    .child(
                        Button::new("labels-add")
                            .small()
                            .icon(Icon::new(IconName::Plus))
                            .tooltip("Add label")
                            .on_click(move |_, window, cx| {
                                let _ = add_label_view.update(cx, |notatus, cx| {
                                    notatus.create_label(window, cx);
                                });
                            }),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .p_2()
                    .child(SidebarMenu::new().children(label_items)),
            )
            .when_some(selected_label, |panel, label| {
                panel.child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .overflow_hidden()
                        .border_t_1()
                        .border_color(rgb(0xe5e7eb))
                        .p_4()
                        .child(self.label_editor(label, view)),
                )
            })
            .when(selected_label.is_none(), |panel| {
                panel.child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap_3()
                        .p_4()
                        .text_sm()
                        .text_color(rgb(0x6b7280))
                        .child(if self.state.dataset.labels.is_empty() {
                            "Create labels before importing media"
                        } else {
                            "Select a label"
                        })
                        .when(self.state.dataset.labels.is_empty(), |empty| {
                            empty.child(
                                Button::new("labels-empty-add")
                                    .small()
                                    .icon(Icon::new(IconName::Plus))
                                    .label("Add label")
                                    .on_click(move |_, window, cx| {
                                        let _ = empty_label_view.update(cx, |notatus, cx| {
                                            notatus.create_label(window, cx);
                                        });
                                    }),
                            )
                        }),
                )
            })
    }

    fn label_items(&self, view: gpui::WeakEntity<NotatusWindow>) -> Vec<SidebarMenuItem> {
        if self.state.dataset.labels.is_empty() {
            vec![SidebarMenuItem::new("No labels yet").disable(true)]
        } else {
            self.state
                .dataset
                .labels
                .iter()
                .map(|label| {
                    let label_id = label.id;
                    let annotation_count = self
                        .state
                        .dataset
                        .annotations
                        .iter()
                        .filter(|annotation| annotation.label_id == label_id)
                        .count();
                    let label_name = label.name.clone();
                    let label_color = label.color.clone();
                    let view = view.clone();
                    SidebarMenuItem::new(label_name)
                        .suffix(label_asset_meta(label_color.as_deref(), annotation_count))
                        .active(self.state.selected_label == Some(label_id))
                        .on_click(move |_, window, cx| {
                            let _ = view.update(cx, |notatus, cx| {
                                notatus.select_label(label_id, window, cx);
                            });
                        })
                })
                .collect()
        }
    }

    fn label_editor(
        &self,
        label: &Label,
        view: gpui::WeakEntity<NotatusWindow>,
    ) -> impl IntoElement {
        let selected_color = label.color.as_deref().unwrap_or(DEFAULT_LABEL_COLOR);
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(metric("Editing", "Label".to_string()))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(section_title("Name"))
                    .child(Input::new(&self.label_name_input).small().w_full()),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(section_title("Color"))
                    .child(div().flex().flex_wrap().gap_2().children(
                        LABEL_COLORS.iter().enumerate().map(|(color_ix, color)| {
                            label_color_button(
                                color_ix,
                                *color,
                                *color == selected_color,
                                view.clone(),
                            )
                        }),
                    )),
            )
            .child(metric(
                "Annotations",
                self.state
                    .dataset
                    .annotations
                    .iter()
                    .filter(|annotation| annotation.label_id == label.id)
                    .count()
                    .to_string(),
            ))
    }
}
