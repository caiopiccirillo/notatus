//! Creating, choosing, recoloring, renaming, and removing labels.

use crate::UiState;
use crate::app::helpers::{hex_color, plural};
use crate::app::window::{LeftDock, NotatusWindow};
use crate::app::{DEFAULT_LABEL_COLOR, LABEL_COLORS};
use gpui::{Context, Window};
use notatus_core::LabelId;

/// Name given to the label created when someone draws before making any.
pub(super) const FALLBACK_LABEL_NAME: &str = "Object";

/// Adds a label with the next palette color, as one undo step.
pub(super) fn add_colored_label(state: &mut UiState, name: Option<String>) -> LabelId {
    let label_number = state.dataset().labels.len() + 1;
    let name = name.unwrap_or_else(|| format!("Label {label_number}"));
    let color = LABEL_COLORS[(label_number - 1) % LABEL_COLORS.len()].to_string();
    state.batch("Create label", |state| {
        let label_id = state.add_label(name);
        let _ = state.update_label_color(label_id, Some(color));
        label_id
    })
}

/// The label new shapes get, creating one when the project has none so
/// drawing never stops to ask for setup first. Returns whether it was created.
pub(super) fn drawing_label(state: &mut UiState) -> (LabelId, bool) {
    if let Some(label_id) = state.selected_label() {
        return (label_id, false);
    }
    if let Some(label_id) = state.dataset().labels.first().map(|label| label.id) {
        let _ = state.select_label(label_id);
        return (label_id, false);
    }
    (
        add_colored_label(state, Some(FALLBACK_LABEL_NAME.to_string())),
        true,
    )
}

impl NotatusWindow {
    pub(super) fn create_label(&mut self, cx: &mut Context<Self>) {
        add_colored_label(&mut self.state, None);
        self.left_dock = LeftDock::Labels;
        self.status_message = Some("Created label".to_string());
        cx.notify();
    }

    pub(super) fn select_label(&mut self, label_id: LabelId, cx: &mut Context<Self>) {
        let result = self.state.select_label(label_id);
        self.show_result(result, cx);
    }

    pub(super) fn update_label_color(
        &mut self,
        label_id: LabelId,
        color: impl Into<String>,
        cx: &mut Context<Self>,
    ) {
        let result = self.state.update_label_color(label_id, Some(color.into()));
        self.show_result(result, cx);
    }

    pub(super) fn prepare_label_color_picker(
        &mut self,
        label_id: LabelId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> String {
        self.label_color_target = Some(label_id);
        let (label_name, color) = self
            .state
            .dataset()
            .label_by_id(label_id)
            .map(|label| {
                (
                    label.name.clone(),
                    label
                        .color
                        .as_deref()
                        .unwrap_or(DEFAULT_LABEL_COLOR)
                        .to_string(),
                )
            })
            .unwrap_or_else(|| ("Label".to_string(), DEFAULT_LABEL_COLOR.to_string()));

        self.label_color_picker.update(cx, |picker, cx| {
            picker.set_value(hex_color(&color), window, cx);
        });
        cx.notify();
        label_name
    }

    pub(super) fn prepare_label_rename(
        &mut self,
        label_id: LabelId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.state.select_label(label_id).is_ok() {
            // The rename dialog opens on this frame, so fill its field now.
            self.sync_name_inputs(window, cx);
            cx.notify();
        }
    }

    pub(super) fn remove_label(&mut self, label_id: LabelId, cx: &mut Context<Self>) {
        match self.state.remove_label(label_id) {
            Ok(removed) => {
                self.hovered_annotation = None;
                self.status_message = Some(format!(
                    "Removed label, {} annotation{}, and {} classification{}",
                    removed.annotations,
                    plural(removed.annotations),
                    removed.classifications,
                    plural(removed.classifications),
                ));
            }
            Err(error) => self.status_message = Some(error.to_string()),
        }
        cx.notify();
    }

    /// Makes a label active and applies it to the selected annotation, so
    /// number keys relabel boxes as quickly as they pick the drawing label.
    pub(super) fn choose_label(&mut self, label_id: LabelId, cx: &mut Context<Self>) {
        match self.state.selected_annotation() {
            Some(annotation_id) => self.update_annotation_label(annotation_id, label_id, cx),
            None => self.select_label(label_id, cx),
        }
    }
}
