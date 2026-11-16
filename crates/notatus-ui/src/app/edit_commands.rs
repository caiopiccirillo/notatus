//! Editing annotations and image labels: undo and redo, selection,
//! relabeling, resizing, and deleting.

use crate::Direction;
use crate::app::window::NotatusWindow;
use gpui::Context;
use notatus_core::{AnnotationId, AssetId, BoundingBox, ClassificationId, LabelId};

impl NotatusWindow {
    pub(super) fn undo(&mut self, cx: &mut Context<Self>) {
        self.tools.cancel_drawing();
        self.status_message = Some(match self.state.undo() {
            Some(description) => format!("Undid {}", description.to_lowercase()),
            None => "Nothing to undo".to_string(),
        });
        self.after_history_jump(cx);
    }

    pub(super) fn redo(&mut self, cx: &mut Context<Self>) {
        self.tools.cancel_drawing();
        self.status_message = Some(match self.state.redo() {
            Some(description) => format!("Redid {}", description.to_lowercase()),
            None => "Nothing to redo".to_string(),
        });
        self.after_history_jump(cx);
    }

    fn after_history_jump(&mut self, cx: &mut Context<Self>) {
        self.hovered_annotation = None;
        cx.notify();
    }

    pub(super) fn step_annotation(&mut self, direction: Direction, cx: &mut Context<Self>) {
        if let Some(annotation_id) = self.state.adjacent_annotation(direction) {
            self.select_annotation(Some(annotation_id), cx);
        }
    }

    pub(super) fn delete_selection(&mut self, cx: &mut Context<Self>) {
        if self.tools.draw_polygon.is_some() {
            self.tools.remove_last_polygon_point();
            cx.notify();
            return;
        }
        let Some(annotation_id) = self.state.selected_annotation() else {
            return;
        };
        self.delete_annotation(annotation_id, cx);
    }

    pub(super) fn delete_annotation(
        &mut self,
        annotation_id: AnnotationId,
        cx: &mut Context<Self>,
    ) {
        match self.state.remove_annotation(annotation_id) {
            Ok(()) => {
                self.hovered_annotation = None;
                self.status_message = Some("Deleted annotation".to_string());
            }
            Err(error) => self.status_message = Some(error.to_string()),
        }
        cx.notify();
    }

    /// Escape: abandon an in-progress shape first, then clear the selection.
    pub(super) fn cancel(&mut self, cx: &mut Context<Self>) {
        if self.tools.draw_box.is_some() || self.tools.draw_polygon.is_some() {
            self.tools.cancel_drawing();
            self.status_message = Some("Cancelled drawing".to_string());
            cx.notify();
        } else if self.state.selected_annotation().is_some() {
            self.select_annotation(None, cx);
        }
    }

    pub(super) fn update_annotation_label(
        &mut self,
        annotation_id: AnnotationId,
        label_id: LabelId,
        cx: &mut Context<Self>,
    ) {
        match self.state.update_annotation_label(annotation_id, label_id) {
            Ok(()) => {
                self.status_message = Some("Updated annotation label".to_string());
            }
            Err(error) => self.status_message = Some(error.to_string()),
        }
        cx.notify();
    }

    pub(super) fn toggle_image_classification(
        &mut self,
        asset_id: AssetId,
        label_id: LabelId,
        cx: &mut Context<Self>,
    ) {
        match self.state.toggle_image_classification(asset_id, label_id) {
            Ok(Some(_)) => {
                self.status_message = Some("Added image classification".to_string());
            }
            Ok(None) => {
                self.status_message = Some("Removed image classification".to_string());
            }
            Err(error) => self.status_message = Some(error.to_string()),
        }
        cx.notify();
    }

    pub(super) fn remove_classification(
        &mut self,
        classification_id: ClassificationId,
        cx: &mut Context<Self>,
    ) {
        match self.state.remove_classification(classification_id) {
            Ok(()) => self.status_message = Some("Removed image classification".to_string()),
            Err(error) => self.status_message = Some(error.to_string()),
        }
        cx.notify();
    }

    pub(super) fn update_annotation_bbox(
        &mut self,
        annotation_id: AnnotationId,
        bbox: BoundingBox,
        cx: &mut Context<Self>,
    ) {
        match self.state.update_annotation_bbox(annotation_id, bbox) {
            Ok(()) => {
                self.status_message = Some("Updated annotation".to_string());
            }
            Err(error) => self.status_message = Some(error.to_string()),
        }
        cx.notify();
    }

    pub(super) fn select_annotation(
        &mut self,
        annotation_id: Option<AnnotationId>,
        cx: &mut Context<Self>,
    ) {
        match self.state.select_annotation(annotation_id) {
            Ok(()) => {
                self.status_message = annotation_id.map(|_| "Selected annotation".to_string());
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
}
