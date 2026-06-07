use super::export_commands;
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
                LeftDock::Dataset => self.dataset_dock(cx).into_any_element(),
                LeftDock::Media => self.media_dock(cx).into_any_element(),
            })
    }

    fn dataset_dock(&self, cx: &mut Context<Self>) -> gpui::Div {
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
                    .child(section_title("Dataset"))
                    .child(self.project_actions(view.clone())),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_5()
                    .child(dataset_section("Project", self.project_editor()))
                    .child(self.labels_section(view)),
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

    fn project_actions(&self, view: gpui::WeakEntity<NotatusWindow>) -> impl IntoElement {
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap_1()
            .child(project_action_button(
                "project-new",
                IconName::Plus,
                "New project",
                project_commands::new_project,
                view.clone(),
            ))
            .child(project_action_button(
                "project-open",
                IconName::FolderOpen,
                "Open project",
                project_commands::open_project,
                view.clone(),
            ))
            .child(project_action_button(
                "project-save",
                IconName::FolderClosed,
                "Save project",
                project_commands::save_project,
                view.clone(),
            ))
            .child(project_action_button(
                "project-save-as",
                IconName::Folder,
                "Save project as",
                project_commands::save_project_as,
                view.clone(),
            ))
            .child(project_action_button(
                "project-export",
                IconName::ExternalLink,
                "Export annotations",
                export_commands::open_export_dialog,
                view,
            ))
    }

    fn project_editor(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(section_title("Name"))
                    .child(Input::new(&self.project_name_input).small().w_full()),
            )
            .child(metric(
                "Location",
                compact_text(&self.project_location.display_name(), 34),
            ))
            .child(dataset_created_label(&self.state.dataset))
    }

    fn labels_section(&self, view: gpui::WeakEntity<NotatusWindow>) -> impl IntoElement {
        let add_label_view = view.clone();
        let empty_label_view = view.clone();
        let label_items = self.label_items(view.clone());
        let selected_label = self.selected_label();

        div()
            .flex()
            .flex_col()
            .gap_2()
            .border_t_1()
            .border_color(rgb(0xe5e7eb))
            .pt_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
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
            .child(if self.state.dataset.labels.is_empty() {
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_3()
                    .py_6()
                    .text_sm()
                    .text_color(rgb(0x6b7280))
                    .child("Create labels before importing media")
                    .child(
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
                    .into_any_element()
            } else {
                SidebarMenu::new().children(label_items).into_any_element()
            })
            .when_some(selected_label, |panel, label| {
                panel.child(
                    div()
                        .border_t_1()
                        .border_color(rgb(0xe5e7eb))
                        .pt_3()
                        .child(self.label_editor(label, view)),
                )
            })
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
                    let select_view = view.clone();
                    let remove_view = view.clone();
                    SidebarMenuItem::new(compact_asset_name(asset))
                        .suffix(media_asset_actions(
                            &asset.kind,
                            annotation_count,
                            asset_id,
                            remove_view,
                        ))
                        .active(self.state.selected_asset == Some(asset_id))
                        .on_click(move |_, _, cx| {
                            let _ = select_view.update(cx, |window, cx| {
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
        let label_id = label.id;
        let remove_view = view.clone();
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
            .child(
                div().flex().justify_end().pt_1().child(
                    Button::new("labels-remove")
                        .small()
                        .danger()
                        .icon(Icon::new(IconName::Delete))
                        .label("Remove label")
                        .tooltip("Remove label and its annotations")
                        .on_click(move |_, window, cx| {
                            let _ = remove_view.update(cx, |notatus, cx| {
                                notatus.remove_label(label_id, window, cx);
                            });
                        }),
                ),
            )
    }
}

fn dataset_section(title: &'static str, content: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(section_title(title))
        .child(content)
}

fn media_asset_actions(
    kind: &AssetKind,
    annotation_count: usize,
    asset_id: AssetId,
    view: gpui::WeakEntity<NotatusWindow>,
) -> impl IntoElement {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap_1()
        .child(media_asset_meta(kind, annotation_count))
        .child(
            Button::new(asset_element_id("media-remove", asset_id))
                .xsmall()
                .ghost()
                .danger()
                .icon(Icon::new(IconName::Delete))
                .tooltip("Remove media and its annotations")
                .on_click(move |_, _, cx| {
                    cx.stop_propagation();
                    let _ = view.update(cx, |notatus, cx| {
                        notatus.remove_asset(asset_id, cx);
                    });
                }),
        )
}

fn asset_element_id(prefix: &'static str, asset_id: AssetId) -> gpui::ElementId {
    let value = asset_id.as_uuid().as_u128();
    let high = (value >> 64) as u64;
    let low = (value as u64).to_string();
    (gpui::ElementId::from((prefix, high)), low).into()
}

fn project_action_button(
    id: &'static str,
    icon: IconName,
    tooltip: &'static str,
    action: fn(gpui::WeakEntity<NotatusWindow>, &mut Window, &mut App),
    view: gpui::WeakEntity<NotatusWindow>,
) -> Button {
    Button::new(id)
        .small()
        .icon(Icon::new(icon))
        .tooltip(tooltip)
        .on_click(move |_, window, cx| {
            action(view.clone(), window, cx);
        })
}
