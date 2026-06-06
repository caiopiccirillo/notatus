use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LeftDock {
    Project,
    Media,
    Labels,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RightDock {
    Annotations,
    MediaInfo,
}

pub(super) struct NotatusWindow {
    pub(super) state: UiState,
    pub(super) left_dock: LeftDock,
    pub(super) right_dock: RightDock,
    pub(super) status_message: Option<String>,
    pub(super) label_name_input: gpui::Entity<InputState>,
    pub(super) syncing_label_input: bool,
    pub(super) tools: ToolInteractionState,
    pub(super) canvas_image_layout: SharedImageLayout,
    pub(super) _subscriptions: Vec<Subscription>,
}

impl NotatusWindow {
    pub(super) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
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
            tools: ToolInteractionState::default(),
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

    pub(super) fn project_summary(&self) -> String {
        format!(
            "{} · {} · {}",
            media_count_label(self.state.dataset.assets.len()),
            annotation_count_label(self.state.dataset.annotations.len()),
            label_count_label(self.state.dataset.labels.len())
        )
    }

    fn app_frame(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                                .child(self.canvas_area(window, cx)),
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
        div()
            .id("notatus-window")
            .size_full()
            .bg(rgb(0xf3f4f6))
            .child(self.app_frame(window, cx))
    }
}

pub(super) fn requested_window_decorations() -> Option<WindowDecorations> {
    if cfg!(target_os = "linux") {
        Some(WindowDecorations::Client)
    } else {
        None
    }
}
