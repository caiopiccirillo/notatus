use super::*;

impl NotatusWindow {
    pub(super) fn create_label(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    pub(super) fn select_label(
        &mut self,
        label_id: LabelId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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

    pub(super) fn update_selected_label_color(
        &mut self,
        color: &'static str,
        cx: &mut Context<Self>,
    ) {
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

    pub(super) fn update_annotation_label(
        &mut self,
        annotation_id: AnnotationId,
        label_id: LabelId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.state.update_annotation_label(annotation_id, label_id) {
            Ok(()) => {
                self.status_message = Some("Updated annotation label".to_string());
                self.sync_label_name_input(window, cx);
            }
            Err(error) => self.status_message = Some(error.to_string()),
        }
        cx.notify();
    }

    pub(super) fn select_annotation(
        &mut self,
        annotation_id: Option<AnnotationId>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.state.select_annotation(annotation_id) {
            Ok(()) => {
                self.status_message = annotation_id.map(|_| "Selected annotation".to_string());
                self.sync_label_name_input(window, cx);
            }
            Err(error) => self.status_message = Some(error.to_string()),
        }
        cx.notify();
    }

    pub(super) fn hover_annotation(
        &mut self,
        annotation_id: Option<AnnotationId>,
        cx: &mut Context<Self>,
    ) {
        if self.hovered_annotation != annotation_id {
            self.hovered_annotation = annotation_id;
            cx.notify();
        }
    }

    pub(super) fn sync_label_name_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    pub(super) fn sync_project_name_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.state.dataset.manifest.project.name.clone();
        self.syncing_project_input = true;
        self.project_name_input.update(cx, |input, cx| {
            input.set_value(name, window, cx);
        });
        self.syncing_project_input = false;
    }

    pub(super) fn fit_canvas_to_view(&mut self, cx: &mut Context<Self>) {
        self.tools.fit_canvas_to_view();
        self.status_message = Some("Fit image to canvas".to_string());
        cx.notify();
    }

    pub(super) fn set_canvas_tool(&mut self, tool: AnnotationTool, cx: &mut Context<Self>) {
        self.tools.clear_for_tool(tool);
        self.state.set_tool(tool);
        self.status_message = None;
        cx.notify();
    }
}
