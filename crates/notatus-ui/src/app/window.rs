use super::helpers::hex_color;
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LeftDock {
    Dataset,
    Media,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RightDock {
    Annotations,
    Info,
}

pub(super) struct NotatusWindow {
    pub(super) state: UiState,
    pub(super) left_dock: LeftDock,
    pub(super) right_dock: RightDock,
    pub(super) status_message: Option<String>,
    pub(super) project_location: ProjectLocation,
    pub(super) project_name_input: gpui::Entity<InputState>,
    pub(super) syncing_project_input: bool,
    pub(super) label_name_input: gpui::Entity<InputState>,
    pub(super) syncing_label_input: bool,
    pub(super) label_color_picker: gpui::Entity<ColorPickerState>,
    pub(super) label_color_target: Option<LabelId>,
    pub(super) tools: ToolInteractionState,
    pub(super) hovered_annotation: Option<AnnotationId>,
    pub(super) canvas_cursor: Option<gpui::CursorStyle>,
    pub(super) export_yolo: bool,
    pub(super) export_coco: bool,
    pub(super) canvas_image_layout: SharedImageLayout,
    pub(super) _subscriptions: Vec<Subscription>,
}

impl NotatusWindow {
    pub(super) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut state = UiState::new_project("Untitled project");
        state.set_tool(AnnotationTool::DrawBox);
        let project_name_input = cx.new(|cx| InputState::new(window, cx));
        let label_name_input = cx.new(|cx| InputState::new(window, cx));
        let label_color_picker = cx.new(|cx| {
            ColorPickerState::new(window, cx).default_value(hex_color(DEFAULT_LABEL_COLOR))
        });
        project_name_input.update(cx, |input, cx| {
            input.set_value(state.dataset.manifest.project.name.clone(), window, cx);
        });
        let _subscriptions = vec![
            cx.subscribe_in(
                &project_name_input,
                window,
                |this, input, event: &InputEvent, _window, cx| {
                    if matches!(event, InputEvent::Change) && !this.syncing_project_input {
                        let value = input.read(cx).value().to_string();
                        match this.state.rename_project(value) {
                            Ok(()) => this.status_message = None,
                            Err(error) => this.status_message = Some(error.to_string()),
                        }
                        cx.notify();
                    }
                },
            ),
            cx.subscribe_in(
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
            ),
            cx.subscribe_in(
                &label_color_picker,
                window,
                |this, _, event: &ColorPickerEvent, _window, cx| {
                    let ColorPickerEvent::Change(Some(color)) = event else {
                        return;
                    };

                    let Some(label_id) = this.label_color_target else {
                        return;
                    };

                    match this
                        .state
                        .update_label_color(label_id, Some(color.to_hex()))
                    {
                        Ok(()) => this.status_message = None,
                        Err(error) => this.status_message = Some(error.to_string()),
                    }
                    cx.notify();
                },
            ),
        ];

        Self {
            state,
            left_dock: LeftDock::Dataset,
            right_dock: RightDock::Info,
            status_message: None,
            project_location: ProjectLocation::default(),
            project_name_input,
            syncing_project_input: false,
            label_name_input,
            syncing_label_input: false,
            label_color_picker,
            label_color_target: None,
            tools: ToolInteractionState::default(),
            hovered_annotation: None,
            canvas_cursor: None,
            export_yolo: true,
            export_coco: true,
            canvas_image_layout: Rc::new(RefCell::new(None)),
            _subscriptions,
        }
    }

    pub(super) fn selected_asset(&self) -> Option<&AssetRecord> {
        self.state
            .selected_asset
            .and_then(|asset_id| self.state.dataset.asset_by_id(asset_id))
    }

    pub(super) fn selected_label(&self) -> Option<&notatus_core::Label> {
        self.state
            .selected_label
            .and_then(|label_id| self.state.dataset.label_by_id(label_id))
    }

    pub(super) fn annotations_for_asset(&self, asset: &AssetRecord) -> Vec<&AnnotationRecord> {
        self.state
            .dataset
            .annotations
            .iter()
            .filter(|annotation| annotation.asset_id == asset.id)
            .collect()
    }

    pub(super) fn classifications_for_asset(
        &self,
        asset: &AssetRecord,
    ) -> Vec<&ClassificationRecord> {
        self.state
            .dataset
            .classifications
            .iter()
            .filter(|classification| classification.asset_id == asset.id)
            .collect()
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
                                .child(self.canvas_area(cx)),
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_layer = Root::render_dialog_layer(window, cx);
        let notification_layer = Root::render_notification_layer(window, cx);

        div()
            .id("notatus-window")
            .relative()
            .size_full()
            .bg(rgb(0xf3f4f6))
            .child(self.app_frame(cx))
            .children(dialog_layer)
            .children(notification_layer)
    }
}
