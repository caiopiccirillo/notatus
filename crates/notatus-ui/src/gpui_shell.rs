use super::{AnnotationTool, UiState};
use gpui::prelude::*;
use gpui::{
    App, Application, Bounds, Context, FontWeight, IntoElement, ObjectFit, PathPromptOptions,
    Pixels, Render, SharedString, Subscription, Window, WindowBackgroundAppearance, WindowBounds,
    WindowDecorations, WindowOptions, div, img, px, rgb, size,
};
use gpui_component::{
    Icon, IconName, Root, Selectable as _, Sizable as _, TitleBar,
    button::Button,
    input::{Input, InputEvent, InputState},
    menu::{DropdownMenu, PopupMenuItem},
    resizable::{h_resizable, resizable_panel},
    sidebar::{SidebarMenu, SidebarMenuItem},
};
use notatus_core::{
    AnnotationGeometry, AnnotationRecord, AssetKind, AssetLocation, AssetRecord, Label, LabelId,
};
use std::path::{Path, PathBuf};

const DEFAULT_LABEL_COLOR: &str = "#2563eb";
const LABEL_COLORS: [&str; 8] = [
    "#2563eb", "#16a34a", "#dc2626", "#d97706", "#7c3aed", "#0891b2", "#db2777", "#4b5563",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LeftDock {
    Project,
    Media,
    Labels,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RightDock {
    Annotations,
    MediaInfo,
}

struct NotatusWindow {
    state: UiState,
    left_dock: LeftDock,
    right_dock: RightDock,
    status_message: Option<String>,
    label_name_input: gpui::Entity<InputState>,
    syncing_label_input: bool,
    _subscriptions: Vec<Subscription>,
}

impl NotatusWindow {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut state = UiState::new_project("Untitled dataset");
        state.set_tool(AnnotationTool::DrawBox);
        let label_name_input = cx.new(|cx| InputState::new(window, cx));
        let _subscriptions = vec![cx.subscribe_in(
            &label_name_input,
            window,
            |this, input, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change)
                    && !this.syncing_label_input
                    && let Some(label_id) = this.state.selected_label
                {
                    let value = input.read(cx).value().to_string();
                    match this.state.update_label_name(label_id, value) {
                        Ok(()) => this.status_message = None,
                        Err(error) => this.status_message = Some(error.to_string()),
                    }
                    cx.notify();
                }
            },
        )];

        Self {
            state,
            left_dock: LeftDock::Media,
            right_dock: RightDock::Annotations,
            status_message: None,
            label_name_input,
            syncing_label_input: false,
            _subscriptions,
        }
    }

    fn app_titlebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
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

    fn left_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .border_r_1()
            .border_color(rgb(0xd6d9de))
            .bg(rgb(0xffffff))
            .overflow_hidden()
            .child(match self.left_dock {
                LeftDock::Project => self.project_dock().into_any_element(),
                LeftDock::Media => self.media_dock(cx).into_any_element(),
                LeftDock::Labels => self.labels_dock(cx).into_any_element(),
            })
    }

    fn project_dock(&self) -> gpui::Div {
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
                    .child(section_title("Datasets"))
                    .child(sidebar_count("1")),
            )
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .p_2()
                    .child(SidebarMenu::new().children(self.dataset_items())),
            )
    }

    fn media_dock(&self, cx: &mut Context<Self>) -> gpui::Div {
        let view = cx.weak_entity();
        let asset_items = self.asset_items(view);

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
                    .child(sidebar_count(self.state.dataset.assets.len().to_string())),
            )
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .p_2()
                    .child(SidebarMenu::new().children(asset_items)),
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
                    .child(sidebar_count(self.state.dataset.labels.len().to_string())),
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
                        .items_center()
                        .justify_center()
                        .p_4()
                        .text_sm()
                        .text_color(rgb(0x6b7280))
                        .child("Select a label"),
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

    fn canvas_placeholder(&self) -> impl IntoElement {
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

    fn right_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .h_full()
            .flex()
            .flex_col()
            .border_l_1()
            .border_color(rgb(0xd6d9de))
            .bg(rgb(0xffffff))
            .child(self.right_panel_header(cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(match self.right_dock {
                        RightDock::Annotations => {
                            self.annotations_panel_content().into_any_element()
                        }
                        RightDock::MediaInfo => self.media_info_panel_content().into_any_element(),
                    }),
            )
    }

    fn right_panel_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(rgb(0xe5e7eb))
            .child(self.right_dock_button(
                "right-annotations",
                IconName::Frame,
                "Annotations",
                RightDock::Annotations,
                cx,
            ))
            .child(self.right_dock_button(
                "right-media-info",
                IconName::Info,
                "Info",
                RightDock::MediaInfo,
                cx,
            ))
    }

    fn right_dock_button(
        &self,
        id: &'static str,
        icon: IconName,
        label: &'static str,
        dock: RightDock,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let view = cx.weak_entity();
        Button::new(id)
            .small()
            .icon(Icon::new(icon))
            .label(label)
            .selected(self.right_dock == dock)
            .on_click(move |_, _, cx| {
                let _ = view.update(cx, |notatus, cx| {
                    notatus.right_dock = dock;
                    cx.notify();
                });
            })
    }

    fn annotations_panel_content(&self) -> gpui::Div {
        if let Some(asset) = self.selected_asset() {
            let annotations = self.annotations_for_asset(asset);

            div()
                .size_full()
                .flex()
                .flex_col()
                .gap_3()
                .p_4()
                .overflow_hidden()
                .child(section_title("Annotations"))
                .child(metric(
                    "Media",
                    compact_text(&asset_display_name(asset), 28),
                ))
                .child(metric("Total", annotation_count_label(annotations.len())))
                .child(div().flex_1().min_h_0().overflow_hidden().child(
                    SidebarMenu::new().children(annotation_items_for_asset(&self.state, asset)),
                ))
        } else {
            empty_panel("No media selected")
        }
    }

    fn media_info_panel_content(&self) -> gpui::Div {
        let selected_media = self
            .selected_asset()
            .map(compact_asset_name)
            .unwrap_or_else(|| "None".to_string());
        let active_label = self
            .selected_label()
            .map(|label| label.name.clone())
            .unwrap_or_else(|| "None".to_string());
        let review_queue = self
            .state
            .dataset
            .annotations
            .iter()
            .filter(|annotation| annotation.review_state == notatus_core::ReviewState::Draft)
            .count();

        let panel = div()
            .size_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .overflow_hidden()
            .child(section_title("Info"))
            .child(metric(
                "Dataset",
                compact_text(&self.state.dataset.manifest.project.name, 28),
            ))
            .child(metric(
                "Active tool",
                format!("{:?}", self.state.active_tool),
            ))
            .child(metric("Active label", active_label))
            .child(metric("Selected media", selected_media))
            .child(metric(
                "Media",
                media_count_label(self.state.dataset.assets.len()),
            ))
            .child(metric(
                "Annotations",
                annotation_count_label(self.state.dataset.annotations.len()),
            ))
            .child(metric(
                "Labels",
                label_count_label(self.state.dataset.labels.len()),
            ))
            .child(metric("Review queue", review_queue.to_string()))
            .child(metric(
                "State",
                if self.state.dirty { "Unsaved" } else { "Saved" }.to_string(),
            ))
            .when_some(self.status_message.clone(), |panel, status| {
                panel.child(metric("Status", status))
            });

        if let Some(asset) = self.selected_asset() {
            let annotation_count = self.annotations_for_asset(asset).len();

            panel
                .child(section_title("Media"))
                .child(metric("Name", compact_text(&asset_display_name(asset), 28)))
                .child(metric("Type", asset_kind_label(&asset.kind).to_string()))
                .child(metric("Dimensions", asset_dimensions_label(asset)))
                .child(metric(
                    "Annotations",
                    annotation_count_label(annotation_count),
                ))
                .child(metric(
                    "Split",
                    dataset_split_label(&asset.split).to_string(),
                ))
                .child(metric(
                    "Location",
                    compact_text(asset.location.display_path().as_ref(), 30),
                ))
        } else {
            panel
                .child(section_title("Media"))
                .child(metric("Name", "None".to_string()))
        }
    }

    fn bottom_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
                        "bottom-media",
                        IconName::GalleryVerticalEnd,
                        "Media",
                        LeftDock::Media,
                        cx,
                    ))
                    .child(self.bottom_left_dock_button(
                        "bottom-labels",
                        IconName::Palette,
                        "Labels",
                        LeftDock::Labels,
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

    fn selected_asset(&self) -> Option<&AssetRecord> {
        self.state
            .selected_asset
            .and_then(|asset_id| self.state.dataset.asset_by_id(asset_id))
    }

    fn selected_label(&self) -> Option<&notatus_core::Label> {
        self.state
            .selected_label
            .and_then(|label_id| self.state.dataset.label_by_id(label_id))
    }

    fn annotations_for_asset(&self, asset: &AssetRecord) -> Vec<&AnnotationRecord> {
        self.state
            .dataset
            .annotations
            .iter()
            .filter(|annotation| annotation.asset_id == asset.id)
            .collect()
    }

    fn project_summary(&self) -> String {
        format!(
            "{} · {} · {}",
            media_count_label(self.state.dataset.assets.len()),
            annotation_count_label(self.state.dataset.annotations.len()),
            label_count_label(self.state.dataset.labels.len())
        )
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

    fn create_label(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let label_number = self.state.dataset.labels.len() + 1;
        let label_id = self.state.add_label(format!("Label {label_number}"));
        let color = LABEL_COLORS[(label_number - 1) % LABEL_COLORS.len()].to_string();
        self.left_dock = LeftDock::Labels;
        if let Err(error) = self.state.update_label_color(label_id, Some(color)) {
            self.status_message = Some(error.to_string());
        } else {
            self.status_message = Some("Created label".to_string());
        }
        self.sync_label_name_input(window, cx);
        cx.notify();
    }

    fn select_label(&mut self, label_id: LabelId, window: &mut Window, cx: &mut Context<Self>) {
        match self.state.select_label(label_id) {
            Ok(()) => {
                self.left_dock = LeftDock::Labels;
                self.status_message = None;
                self.sync_label_name_input(window, cx);
            }
            Err(error) => self.status_message = Some(error.to_string()),
        }
        cx.notify();
    }

    fn update_selected_label_color(&mut self, color: &'static str, cx: &mut Context<Self>) {
        if let Some(label_id) = self.state.selected_label {
            match self
                .state
                .update_label_color(label_id, Some(color.to_string()))
            {
                Ok(()) => self.status_message = None,
                Err(error) => self.status_message = Some(error.to_string()),
            }
            cx.notify();
        }
    }

    fn sync_label_name_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self
            .selected_label()
            .map(|label| label.name.clone())
            .unwrap_or_default();
        self.syncing_label_input = true;
        self.label_name_input.update(cx, |input, cx| {
            input.set_value(name, window, cx);
        });
        self.syncing_label_input = false;
    }

    fn app_frame(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .text_color(rgb(0x111827))
            .bg(rgb(0xf3f4f6))
            .child(self.app_titlebar(cx))
            .child(
                div().flex_1().overflow_hidden().child(
                    h_resizable("notatus-annotation-panels")
                        .child(
                            resizable_panel()
                                .size(px(304.0))
                                .size_range(px(228.0)..px(420.0))
                                .child(self.left_panel(cx)),
                        )
                        .child(
                            resizable_panel()
                                .size_range(px(320.0)..Pixels::MAX)
                                .child(self.canvas_placeholder()),
                        )
                        .child(
                            resizable_panel()
                                .size(px(304.0))
                                .size_range(px(220.0)..px(420.0))
                                .child(self.right_panel(cx)),
                        ),
                ),
            )
            .child(self.bottom_bar(cx))
    }
}

impl Render for NotatusWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("notatus-window")
            .size_full()
            .bg(rgb(0xf3f4f6))
            .child(self.app_frame(cx))
    }
}

fn open_media_picker(view: gpui::WeakEntity<NotatusWindow>, cx: &mut App) {
    let _ = view.update(cx, |window, cx| {
        window.status_message = Some("Waiting for media selection".to_string());
        cx.notify();
    });

    let paths = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: true,
        prompt: Some(SharedString::from("Import media")),
    });

    cx.spawn(async move |cx| match paths.await {
        Ok(Ok(Some(paths))) => {
            let imported = inspect_media_paths(paths);
            let _ = view.update(cx, |window, cx| {
                window.apply_media_import(imported);
                cx.notify();
            });
        }
        Ok(Ok(None)) => {
            let _ = view.update(cx, |window, cx| {
                window.status_message = Some("Media import cancelled".to_string());
                cx.notify();
            });
        }
        Ok(Err(error)) => {
            let _ = view.update(cx, |window, cx| {
                window.status_message = Some(format!("Media picker failed: {error}"));
                cx.notify();
            });
        }
        Err(_) => {
            let _ = view.update(cx, |window, cx| {
                window.status_message = Some("Media picker closed unexpectedly".to_string());
                cx.notify();
            });
        }
    })
    .detach();
}

struct MediaImport {
    candidates: Vec<MediaCandidate>,
    failures: Vec<String>,
}

struct MediaCandidate {
    path: PathBuf,
    width: u32,
    height: u32,
}

fn inspect_media_paths(paths: Vec<PathBuf>) -> MediaImport {
    let mut candidates = Vec::new();
    let mut failures = Vec::new();

    for path in paths {
        match image::image_dimensions(&path) {
            Ok((width, height)) => candidates.push(MediaCandidate {
                path,
                width,
                height,
            }),
            Err(error) => failures.push(format!("{}: {error}", path.display())),
        }
    }

    MediaImport {
        candidates,
        failures,
    }
}

impl NotatusWindow {
    fn apply_media_import(&mut self, imported: MediaImport) {
        let mut added = 0;
        let mut failed = imported.failures;

        for candidate in imported.candidates {
            let path = candidate.path.to_string_lossy().into_owned();
            if dataset_has_local_path(&self.state, &path) {
                failed.push(format!("{path}: already imported"));
                continue;
            }

            match self
                .state
                .add_local_image_asset(path, candidate.width, candidate.height)
            {
                Ok(_) => added += 1,
                Err(error) => failed.push(error.to_string()),
            }
        }

        if added > 0 {
            self.left_dock = LeftDock::Media;
        }
        self.status_message = Some(media_import_summary(added, failed.len()));
    }
}

fn media_import_summary(added: usize, failed: usize) -> String {
    match (added, failed) {
        (0, 0) => "No media selected".to_string(),
        (0, failed) => format!("Skipped {failed} unsupported file{}", plural(failed)),
        (added, 0) => format!("Imported {added} media item{}", plural(added)),
        (added, failed) => {
            format!(
                "Imported {added} media item{}; skipped {failed}",
                plural(added)
            )
        }
    }
}

fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

fn image_canvas_content(asset: &AssetRecord) -> impl IntoElement {
    match &asset.location {
        AssetLocation::LocalPath { path } => div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                img(PathBuf::from(path))
                    .size_full()
                    .object_fit(ObjectFit::Contain)
                    .with_loading(|| canvas_message("Loading image").into_any_element())
                    .with_fallback(|| {
                        canvas_message("Unable to load selected image").into_any_element()
                    }),
            ),
        AssetLocation::S3Object { .. } => {
            canvas_message("Remote image preview is not implemented yet")
        }
    }
}

fn canvas_message(message: &'static str) -> gpui::Div {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(rgb(0x4b5563))
        .child(message)
}

fn empty_panel(message: &'static str) -> gpui::Div {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .p_4()
        .text_sm()
        .text_color(rgb(0x6b7280))
        .child(message)
}

fn sidebar_count(value: impl Into<String>) -> impl IntoElement {
    div()
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .min_w(px(24.0))
        .h(px(20.0))
        .px_1()
        .rounded_sm()
        .text_xs()
        .text_color(rgb(0x4b5563))
        .bg(rgb(0xf3f4f6))
        .child(value.into())
}

fn media_asset_meta(kind: &AssetKind, annotation_count: usize) -> impl IntoElement {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap_1()
        .child(
            div()
                .text_xs()
                .text_color(rgb(0x6b7280))
                .child(asset_kind_label(kind)),
        )
        .child(sidebar_count(annotation_count.to_string()))
}

fn label_asset_meta(color: Option<&str>, annotation_count: usize) -> impl IntoElement {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap_1()
        .child(label_swatch(color.unwrap_or(DEFAULT_LABEL_COLOR), false))
        .child(sidebar_count(annotation_count.to_string()))
}

fn label_color_button(
    color_ix: usize,
    color: &'static str,
    selected: bool,
    view: gpui::WeakEntity<NotatusWindow>,
) -> impl IntoElement {
    div()
        .id(("label-color", color_ix))
        .flex_none()
        .size(px(28.0))
        .rounded_sm()
        .border_1()
        .border_color(if selected {
            rgb(0x111827)
        } else {
            rgb(0xd1d5db)
        })
        .p(px(3.0))
        .hover(|swatch| swatch.border_color(rgb(0x6b7280)))
        .on_click(move |_, _, cx| {
            let _ = view.update(cx, |notatus, cx| {
                notatus.update_selected_label_color(color, cx);
            });
        })
        .child(label_swatch(color, true))
}

fn label_swatch(color: &str, fill_parent: bool) -> impl IntoElement {
    div()
        .flex_none()
        .when(fill_parent, |swatch| swatch.size_full())
        .when(!fill_parent, |swatch| swatch.size(px(12.0)))
        .rounded_sm()
        .bg(hex_color(color))
}

fn annotation_items_for_asset(state: &UiState, asset: &AssetRecord) -> Vec<SidebarMenuItem> {
    let items: Vec<_> = state
        .dataset
        .annotations
        .iter()
        .filter(|annotation| annotation.asset_id == asset.id)
        .map(|annotation| {
            SidebarMenuItem::new(annotation_item_label(state, annotation))
                .suffix(sidebar_count(annotation_geometry_label(
                    &annotation.geometry,
                )))
                .active(state.selected_annotation == Some(annotation.id))
        })
        .collect();

    if items.is_empty() {
        vec![SidebarMenuItem::new("No annotations").disable(true)]
    } else {
        items
    }
}

fn annotation_item_label(state: &UiState, annotation: &AnnotationRecord) -> String {
    let label = state
        .dataset
        .label_by_id(annotation.label_id)
        .map(|label| label.name.as_str())
        .unwrap_or("Unknown label");

    compact_text(&format!("{label} · {:?}", annotation.review_state), 34)
}

fn annotation_geometry_label(geometry: &AnnotationGeometry) -> &'static str {
    match geometry {
        AnnotationGeometry::Bbox(_) => "Box",
        AnnotationGeometry::Polygon(_) => "Poly",
    }
}

fn media_count_label(count: usize) -> String {
    format!("{count} media")
}

fn annotation_count_label(count: usize) -> String {
    format!("{count} annotation{}", plural(count))
}

fn label_count_label(count: usize) -> String {
    format!("{count} label{}", plural(count))
}

fn dataset_created_label(dataset: &notatus_core::Dataset) -> String {
    format!("Created {}", dataset.manifest.project.created_at.date())
}

fn asset_kind_label(kind: &AssetKind) -> &'static str {
    match kind {
        AssetKind::Image => "Image",
        AssetKind::Video => "Video",
    }
}

fn dataset_split_label(split: &notatus_core::DatasetSplit) -> &'static str {
    match split {
        notatus_core::DatasetSplit::Train => "Train",
        notatus_core::DatasetSplit::Validation => "Validation",
        notatus_core::DatasetSplit::Test => "Test",
        notatus_core::DatasetSplit::Unassigned => "Unassigned",
    }
}

fn asset_display_name(asset: &AssetRecord) -> String {
    let display_path = asset.location.display_path();
    Path::new(display_path.as_ref())
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(display_path.as_ref())
        .to_string()
}

fn compact_asset_name(asset: &AssetRecord) -> String {
    compact_text(&asset_display_name(asset), 36)
}

fn asset_dimensions_label(asset: &AssetRecord) -> String {
    format!("{}x{}", asset.dimensions.width, asset.dimensions.height)
}

fn compact_text(value: &str, max_chars: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= max_chars || max_chars < 8 {
        return value.to_string();
    }

    let head_count = (max_chars - 3) * 2 / 3;
    let tail_count = max_chars - 3 - head_count;
    let head: String = value.chars().take(head_count).collect();
    let tail: String = value
        .chars()
        .rev()
        .take(tail_count)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{head}...{tail}")
}

fn hex_color(value: &str) -> gpui::Hsla {
    let hex = value.strip_prefix('#').unwrap_or(value);
    u32::from_str_radix(hex, 16)
        .map(rgb)
        .unwrap_or_else(|_| rgb(0x2563eb))
        .into()
}

fn dataset_has_local_path(state: &UiState, path: &str) -> bool {
    state.dataset.assets.iter().any(|asset| {
        matches!(
            &asset.location,
            AssetLocation::LocalPath { path: existing } if existing == path
        )
    })
}

fn section_title(title: &'static str) -> impl IntoElement {
    div()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(0x374151))
        .child(title)
}

fn metric(label: &'static str, value: String) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .text_sm()
        .child(div().text_color(rgb(0x4b5563)).child(label))
        .child(div().font_weight(FontWeight::SEMIBOLD).child(value))
}

fn requested_window_decorations() -> Option<WindowDecorations> {
    if cfg!(target_os = "linux") {
        Some(WindowDecorations::Client)
    } else {
        None
    }
}

pub fn launch_gpui() {
    Application::new()
        .with_assets(gpui_component_assets::Assets)
        .run(|cx: &mut App| {
            gpui_component::init(cx);

            cx.on_window_closed(|cx| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();

            let bounds = Bounds::centered(None, size(px(1200.0), px(760.0)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitleBar::title_bar_options()),
                    window_background: WindowBackgroundAppearance::Opaque,
                    window_decorations: requested_window_decorations(),
                    window_min_size: Some(size(px(720.0), px(460.0))),
                    ..Default::default()
                },
                |window, cx| {
                    let view = cx.new(|cx| {
                        cx.observe_window_appearance(window, |_, window, _| {
                            window.refresh();
                        })
                        .detach();
                        NotatusWindow::new(window, cx)
                    });
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .unwrap();
            cx.activate(true);
        });
}
