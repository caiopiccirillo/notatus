use super::*;
use crate::app::helpers::exportable_annotation_count;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ExportWorkflowIssue {
    Labels,
    Media,
    Annotations,
    Format,
}

impl ExportWorkflowIssue {
    fn target_dock(self) -> LeftDock {
        match self {
            Self::Labels => LeftDock::Dataset,
            Self::Media => LeftDock::Media,
            Self::Annotations | Self::Format => LeftDock::Dataset,
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Labels => "Labels required",
            Self::Media => "Media required",
            Self::Annotations => "Annotations required",
            Self::Format => "Format required",
        }
    }

    pub(super) fn message(self) -> &'static str {
        match self {
            Self::Labels => "Create a label before exporting annotations.",
            Self::Media => "Import media before exporting annotations.",
            Self::Annotations => "Create exportable annotations before exporting.",
            Self::Format => "Select at least one export format.",
        }
    }

    fn status(self) -> &'static str {
        match self {
            Self::Labels => "Create a label before exporting",
            Self::Media => "Import media before exporting",
            Self::Annotations => "No exportable annotations",
            Self::Format => "Select an export format",
        }
    }
}

impl NotatusWindow {
    pub(super) fn export_workflow_issue(&self) -> Option<ExportWorkflowIssue> {
        if self.state.dataset.labels.is_empty() {
            Some(ExportWorkflowIssue::Labels)
        } else if self.state.dataset.assets.is_empty() {
            Some(ExportWorkflowIssue::Media)
        } else if exportable_annotation_count(&self.state.dataset) == 0 {
            Some(ExportWorkflowIssue::Annotations)
        } else if !self.export_yolo && !self.export_coco {
            Some(ExportWorkflowIssue::Format)
        } else {
            None
        }
    }

    pub(super) fn apply_export_workflow_issue(
        &mut self,
        issue: ExportWorkflowIssue,
        cx: &mut Context<Self>,
    ) {
        self.left_dock = issue.target_dock();
        self.status_message = Some(issue.status().to_string());
        cx.notify();
    }

    pub(super) fn toggle_export_yolo(&mut self, cx: &mut Context<Self>) {
        self.export_yolo = !self.export_yolo;
        if !self.export_yolo && !self.export_coco {
            self.export_yolo = true;
            self.status_message = Some(ExportWorkflowIssue::Format.status().to_string());
        } else {
            self.status_message = None;
        }
        cx.notify();
    }

    pub(super) fn toggle_export_coco(&mut self, cx: &mut Context<Self>) {
        self.export_coco = !self.export_coco;
        if !self.export_yolo && !self.export_coco {
            self.export_coco = true;
            self.status_message = Some(ExportWorkflowIssue::Format.status().to_string());
        } else {
            self.status_message = None;
        }
        cx.notify();
    }
}

pub(super) fn push_export_workflow_notification(
    issue: ExportWorkflowIssue,
    window: &mut Window,
    cx: &mut App,
) {
    window.push_notification(
        Notification::warning(issue.message())
            .id1::<ExportWorkflowNotification>("workflow")
            .title(issue.title())
            .autohide(false),
        cx,
    );
}

struct ExportWorkflowNotification;
