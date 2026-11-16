use super::options::ExportFormat;
use crate::app::helpers::exportable_annotation_count;
use crate::app::window::{LeftDock, NotatusWindow};
use gpui::{App, Context, Window};
use gpui_component::WindowExt as _;
use gpui_component::notification::Notification;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) enum ExportWorkflowIssue {
    Labels,
    Media,
    Annotations,
    FinishedMedia,
    Format,
}

impl ExportWorkflowIssue {
    /// The dock that fixes the issue, when one does.
    fn target_dock(self) -> Option<LeftDock> {
        match self {
            Self::Labels => Some(LeftDock::Labels),
            Self::Media | Self::FinishedMedia | Self::Annotations => Some(LeftDock::Media),
            Self::Format => None,
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Labels => "Labels required",
            Self::Media => "Media required",
            Self::FinishedMedia => "No finished media",
            Self::Annotations => "Annotations required",
            Self::Format => "Format required",
        }
    }

    pub(super) fn message(self) -> &'static str {
        match self {
            Self::Labels => "Create a label before exporting annotations.",
            Self::Media => "Import media before exporting annotations.",
            Self::FinishedMedia => {
                "No media is marked done. Mark finished media with M or Done & next, \
                 or turn off \"Finished media only\" in the export dialog."
            }
            Self::Annotations => "Create exportable annotations before exporting.",
            Self::Format => "Select at least one export format.",
        }
    }

    fn status(self) -> &'static str {
        match self {
            Self::Labels => "Create a label before exporting",
            Self::Media => "Import media before exporting",
            Self::FinishedMedia => "Mark media done before exporting",
            Self::Annotations => "No exportable annotations",
            Self::Format => "Select an export format",
        }
    }
}

impl NotatusWindow {
    pub(in crate::app) fn export_workflow_issue(&self) -> Option<ExportWorkflowIssue> {
        if self.state.dataset().labels.is_empty() {
            Some(ExportWorkflowIssue::Labels)
        } else if self.state.dataset().assets.is_empty() {
            Some(ExportWorkflowIssue::Media)
        } else if exportable_annotation_count(self.state.dataset()) == 0
            && exportable_classification_count(self.state.dataset()) == 0
        {
            Some(ExportWorkflowIssue::Annotations)
        } else if self.export.finished_only
            && !self
                .state
                .dataset()
                .assets
                .iter()
                .any(|asset| asset.status == notatus_core::AssetStatus::Done)
        {
            Some(ExportWorkflowIssue::FinishedMedia)
        } else if !self.export.has_format() {
            Some(ExportWorkflowIssue::Format)
        } else {
            None
        }
    }

    pub(in crate::app) fn apply_export_workflow_issue(
        &mut self,
        issue: ExportWorkflowIssue,
        cx: &mut Context<Self>,
    ) {
        if let Some(dock) = issue.target_dock() {
            self.left_dock = dock;
        }
        self.status_message = Some(issue.status().to_string());
        cx.notify();
    }

    pub(in crate::app) fn toggle_export_format(
        &mut self,
        format: ExportFormat,
        cx: &mut Context<Self>,
    ) {
        self.status_message = if self.export.toggle(format) {
            None
        } else {
            Some(ExportWorkflowIssue::Format.status().to_string())
        };
        cx.notify();
    }

    pub(in crate::app) fn toggle_export_finished_only(&mut self, cx: &mut Context<Self>) {
        self.export.finished_only = !self.export.finished_only;
        self.status_message = None;
        cx.notify();
    }
}

fn exportable_classification_count(dataset: &notatus_core::Dataset) -> usize {
    let filter = notatus_export::AnnotationFilter::ready_for_training();
    dataset
        .classifications
        .iter()
        .filter(|classification| filter.accepts_classification(classification))
        .count()
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
