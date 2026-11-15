//! Keyboard actions, their default bindings, and the handlers that run them.

use super::helpers::plural;
use super::model_run::ModelScope;
use super::*;
use crate::Direction;
use gpui::{Action, KeyBinding, actions};
use notatus_core::{AssetStatus, ReviewState};

/// Key context set on the main window. Bare-key shortcuts additionally require
/// that no text input is focused.
pub(super) const CONTEXT: &str = "Notatus";
const CANVAS_CONTEXT: &str = "Notatus && !Input";

actions!(
    notatus,
    [
        /// Open the command palette.
        CommandPalette,
        Undo,
        Redo,
        NewProject,
        OpenProject,
        SaveProject,
        SaveProjectAs,
        ImportMedia,
        /// Import every image in a folder and its subfolders.
        ImportFolder,
        ExportAnnotations,
        NewLabel,
        /// Focus the label picker to choose or create the active label.
        PickLabel,
        SelectTool,
        BoxTool,
        PolygonTool,
        NextMedia,
        PreviousMedia,
        NextPendingMedia,
        ToggleMediaDone,
        DoneAndNext,
        NextAnnotation,
        PreviousAnnotation,
        DeleteSelection,
        Cancel,
        FitCanvas,
        ZoomIn,
        ZoomOut,
        ToggleTheme,
        ToggleHideAnnotations,
        ToggleLabelNames,
        ToggleCrosshair,
        ToggleFocusSelected,
        /// Accept the selected annotation and move to the next one.
        AcceptAnnotation,
        /// Reject the selected annotation and move to the next one.
        RejectAnnotation,
        /// Return the selected annotation to draft.
        ResetAnnotationReview,
        /// Show every keyboard shortcut.
        ShowShortcuts,
        /// Run the project's model on the current media.
        RunModel,
        /// Run the project's model on every media not marked done.
        RunModelOnUnfinished,
        /// Stop a model run after the media in progress.
        StopModelRun,
        /// Choose the program that runs the project's model.
        ChooseModelRunner,
    ]
);

/// Make the label at a 0-based position in the label list active.
#[derive(Clone, Debug, PartialEq, Action)]
#[action(namespace = notatus, no_json)]
pub(super) struct SelectLabelSlot(pub(super) usize);

/// Make a specific label active.
#[derive(Clone, Debug, PartialEq, Action)]
#[action(namespace = notatus, no_json)]
pub(super) struct SelectLabelById(pub(super) LabelId);

/// Jump to a specific media item.
#[derive(Clone, Debug, PartialEq, Action)]
#[action(namespace = notatus, no_json)]
pub(super) struct GoToMedia(pub(super) AssetId);

pub(super) fn bind_keys(cx: &mut App) {
    let global = Some(CONTEXT);
    let canvas = Some(CANVAS_CONTEXT);
    cx.bind_keys([
        // Ctrl+K is the one the app advertises, so it is bound last: hints
        // show the last binding.
        KeyBinding::new("ctrl-shift-p", CommandPalette, global),
        KeyBinding::new("ctrl-k", CommandPalette, global),
        KeyBinding::new("ctrl-z", Undo, canvas),
        KeyBinding::new("ctrl-shift-z", Redo, canvas),
        KeyBinding::new("ctrl-y", Redo, canvas),
        KeyBinding::new("ctrl-n", NewProject, global),
        KeyBinding::new("ctrl-o", OpenProject, global),
        KeyBinding::new("ctrl-s", SaveProject, global),
        KeyBinding::new("ctrl-shift-s", SaveProjectAs, global),
        KeyBinding::new("ctrl-i", ImportMedia, global),
        KeyBinding::new("ctrl-shift-i", ImportFolder, global),
        KeyBinding::new("ctrl-e", ExportAnnotations, global),
        KeyBinding::new("ctrl-shift-l", NewLabel, global),
        KeyBinding::new("l", PickLabel, canvas),
        KeyBinding::new("v", SelectTool, canvas),
        KeyBinding::new("b", BoxTool, canvas),
        KeyBinding::new("p", PolygonTool, canvas),
        // The last binding for an action is the one shown in hints.
        KeyBinding::new("right", NextMedia, canvas),
        KeyBinding::new("d", NextMedia, canvas),
        KeyBinding::new("left", PreviousMedia, canvas),
        KeyBinding::new("a", PreviousMedia, canvas),
        KeyBinding::new("shift-d", NextPendingMedia, canvas),
        KeyBinding::new("m", ToggleMediaDone, canvas),
        KeyBinding::new("ctrl-enter", DoneAndNext, global),
        KeyBinding::new("tab", NextAnnotation, canvas),
        KeyBinding::new("shift-tab", PreviousAnnotation, canvas),
        KeyBinding::new("backspace", DeleteSelection, canvas),
        KeyBinding::new("delete", DeleteSelection, canvas),
        KeyBinding::new("escape", Cancel, canvas),
        KeyBinding::new("f", FitCanvas, canvas),
        KeyBinding::new("ctrl-0", FitCanvas, global),
        KeyBinding::new("=", ZoomIn, canvas),
        KeyBinding::new("ctrl-=", ZoomIn, global),
        KeyBinding::new("-", ZoomOut, canvas),
        KeyBinding::new("ctrl--", ZoomOut, global),
        KeyBinding::new("h", ToggleHideAnnotations, canvas),
        KeyBinding::new("shift-l", ToggleLabelNames, canvas),
        KeyBinding::new("c", ToggleCrosshair, canvas),
        KeyBinding::new("shift-f", ToggleFocusSelected, canvas),
        KeyBinding::new("?", ShowShortcuts, canvas),
        KeyBinding::new("ctrl-r", RunModel, global),
        KeyBinding::new("ctrl-shift-r", RunModelOnUnfinished, global),
        KeyBinding::new("y", AcceptAnnotation, canvas),
        KeyBinding::new("x", RejectAnnotation, canvas),
        KeyBinding::new("u", ResetAnnotationReview, canvas),
    ]);
    cx.bind_keys((0..9).map(|slot| {
        const KEYS: [&str; 9] = ["1", "2", "3", "4", "5", "6", "7", "8", "9"];
        KeyBinding::new(KEYS[slot], SelectLabelSlot(slot), canvas)
    }));
}

/// A command that reaches the window through a weak handle, like the dock
/// buttons use.
type WindowCommand = fn(gpui::WeakEntity<NotatusWindow>, &mut Window, &mut App);

/// Runs `command` right after the current update. Action listeners run while
/// the window is already being updated, and these commands update it again
/// through their handle, which gpui refuses with a panic.
fn run_outside_update(
    cx: &mut Context<NotatusWindow>,
    window: &mut Window,
    command: WindowCommand,
) {
    let view = cx.weak_entity();
    window.defer(cx, move |window, cx| command(view, window, cx));
}

impl NotatusWindow {
    /// Attaches every action handler to the window root.
    pub(super) fn action_handlers(
        &self,
        root: gpui::Stateful<gpui::Div>,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        root.key_context(CONTEXT)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|this, _: &ShowShortcuts, window, cx| {
                this.open_shortcuts(window, cx);
            }))
            .on_action(cx.listener(|this, _: &RunModel, _, cx| {
                this.run_model(ModelScope::CurrentMedia, cx);
            }))
            .on_action(cx.listener(|this, _: &RunModelOnUnfinished, _, cx| {
                this.run_model(ModelScope::UnfinishedMedia, cx);
            }))
            .on_action(cx.listener(|this, _: &StopModelRun, _, cx| {
                this.stop_model_run(cx);
            }))
            .on_action(cx.listener(|this, _: &ChooseModelRunner, _, cx| {
                this.choose_model_runner(None, cx);
            }))
            .on_action(cx.listener(|this, _: &CommandPalette, window, cx| {
                this.open_command_palette(window, cx);
            }))
            .on_action(cx.listener(|this, _: &Undo, window, cx| this.undo(window, cx)))
            .on_action(cx.listener(|this, _: &Redo, window, cx| this.redo(window, cx)))
            .on_action(cx.listener(|_, _: &NewProject, window, cx| {
                run_outside_update(cx, window, project_commands::new_project);
            }))
            .on_action(cx.listener(|_, _: &OpenProject, window, cx| {
                run_outside_update(cx, window, project_commands::open_project);
            }))
            .on_action(cx.listener(|_, _: &SaveProject, window, cx| {
                run_outside_update(cx, window, project_commands::save_project);
            }))
            .on_action(cx.listener(|_, _: &SaveProjectAs, window, cx| {
                run_outside_update(cx, window, project_commands::save_project_as);
            }))
            .on_action(cx.listener(|_, _: &ImportMedia, window, cx| {
                run_outside_update(cx, window, media_import::open_media_picker);
            }))
            .on_action(cx.listener(|_, _: &ImportFolder, window, cx| {
                run_outside_update(cx, window, media_import::open_folder_picker);
            }))
            .on_action(cx.listener(|_, _: &ExportAnnotations, window, cx| {
                run_outside_update(cx, window, export_commands::open_export_dialog);
            }))
            .on_action(cx.listener(|this, _: &NewLabel, window, cx| {
                this.create_label(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PickLabel, window, cx| {
                this.open_label_picker(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleHideAnnotations, _, cx| {
                this.toggle_view_option(ViewToggle::HideAnnotations, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleLabelNames, _, cx| {
                this.toggle_view_option(ViewToggle::Labels, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleCrosshair, _, cx| {
                this.toggle_view_option(ViewToggle::Crosshair, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleFocusSelected, _, cx| {
                this.toggle_view_option(ViewToggle::FocusSelected, cx);
            }))
            .on_action(cx.listener(|this, _: &AcceptAnnotation, window, cx| {
                this.review_selection(ReviewState::Accepted, window, cx);
            }))
            .on_action(cx.listener(|this, _: &RejectAnnotation, window, cx| {
                this.review_selection(ReviewState::Rejected, window, cx);
            }))
            .on_action(cx.listener(|this, _: &ResetAnnotationReview, window, cx| {
                this.review_selection(ReviewState::Draft, window, cx);
            }))
            .on_action(cx.listener(|this, _: &SelectTool, _, cx| {
                this.set_canvas_tool(AnnotationTool::Select, cx);
            }))
            .on_action(cx.listener(|this, _: &BoxTool, _, cx| {
                this.set_canvas_tool(AnnotationTool::DrawBox, cx);
            }))
            .on_action(cx.listener(|this, _: &PolygonTool, _, cx| {
                this.set_canvas_tool(AnnotationTool::DrawPolygon, cx);
            }))
            .on_action(cx.listener(|this, _: &NextMedia, window, cx| {
                this.step_media(Direction::Next, false, window, cx);
            }))
            .on_action(cx.listener(|this, _: &PreviousMedia, window, cx| {
                this.step_media(Direction::Previous, false, window, cx);
            }))
            .on_action(cx.listener(|this, _: &NextPendingMedia, window, cx| {
                this.step_media(Direction::Next, true, window, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleMediaDone, _, cx| {
                this.toggle_media_done(cx);
            }))
            .on_action(cx.listener(|this, _: &DoneAndNext, window, cx| {
                this.done_and_next(window, cx);
            }))
            .on_action(cx.listener(|this, _: &NextAnnotation, window, cx| {
                this.step_annotation(Direction::Next, window, cx);
            }))
            .on_action(cx.listener(|this, _: &PreviousAnnotation, window, cx| {
                this.step_annotation(Direction::Previous, window, cx);
            }))
            .on_action(cx.listener(|this, _: &DeleteSelection, window, cx| {
                this.delete_selection(window, cx);
            }))
            .on_action(cx.listener(|this, _: &Cancel, window, cx| {
                this.cancel(window, cx);
            }))
            .on_action(cx.listener(|this, _: &FitCanvas, _, cx| {
                this.fit_canvas_to_view(cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomIn, _, cx| this.zoom_canvas(1.25, cx)))
            .on_action(cx.listener(|this, _: &ZoomOut, _, cx| this.zoom_canvas(0.8, cx)))
            .on_action(cx.listener(|_, _: &ToggleTheme, window, cx| {
                let mode = if cx.theme().is_dark() {
                    gpui_component::ThemeMode::Light
                } else {
                    gpui_component::ThemeMode::Dark
                };
                Theme::change(mode, Some(window), cx);
                window.refresh();
            }))
            .on_action(cx.listener(|this, action: &SelectLabelSlot, window, cx| {
                if let Some(label_id) = this.state.dataset().labels.get(action.0).map(|l| l.id) {
                    this.choose_label(label_id, window, cx);
                }
            }))
            .on_action(cx.listener(|this, action: &SelectLabelById, window, cx| {
                this.choose_label(action.0, window, cx);
            }))
            .on_action(cx.listener(|this, action: &GoToMedia, window, cx| {
                this.go_to_media(action.0, window, cx);
            }))
    }

    pub(super) fn undo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.tools.cancel_drawing();
        self.status_message = Some(match self.state.undo() {
            Some(description) => format!("Undid {}", description.to_lowercase()),
            None => "Nothing to undo".to_string(),
        });
        self.after_history_jump(window, cx);
    }

    pub(super) fn redo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.tools.cancel_drawing();
        self.status_message = Some(match self.state.redo() {
            Some(description) => format!("Redid {}", description.to_lowercase()),
            None => "Nothing to redo".to_string(),
        });
        self.after_history_jump(window, cx);
    }

    fn after_history_jump(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.hovered_annotation = None;
        self.sync_project_name_input(window, cx);
        self.sync_label_name_input(window, cx);
        cx.notify();
    }

    /// Selects a media item and resets the canvas for it.
    pub(super) fn go_to_media(
        &mut self,
        asset_id: AssetId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.state.selected_asset() == Some(asset_id) {
            return;
        }
        match self.state.select_asset(asset_id) {
            Ok(()) => {
                self.tools.fit_canvas_to_view();
                self.canvas_image_layout.borrow_mut().take();
                self.hovered_annotation = None;
                self.status_message = None;
                self.sync_label_name_input(window, cx);
            }
            Err(error) => self.status_message = Some(error.to_string()),
        }
        cx.notify();
    }

    pub(super) fn step_media(
        &mut self,
        direction: Direction,
        pending_only: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self
            .state
            .adjacent_asset(self.state.selected_asset(), direction, pending_only)
        {
            Some(asset_id) => self.go_to_media(asset_id, window, cx),
            None => {
                self.status_message = Some(
                    match (direction, pending_only) {
                        (_, true) => "No more pending media",
                        (Direction::Next, false) => "Already at the last media",
                        (Direction::Previous, false) => "Already at the first media",
                    }
                    .to_string(),
                );
                cx.notify();
            }
        }
    }

    pub(super) fn toggle_media_done(&mut self, cx: &mut Context<Self>) {
        let Some(asset) = self.selected_asset() else {
            return;
        };
        let (asset_id, status) = match asset.status {
            AssetStatus::Done => (asset.id, AssetStatus::Pending),
            AssetStatus::Pending => (asset.id, AssetStatus::Done),
        };
        match self.state.set_asset_status(asset_id, status) {
            Ok(()) => {
                self.status_message = Some(match status {
                    AssetStatus::Done => "Marked media done".to_string(),
                    AssetStatus::Pending => "Marked media pending".to_string(),
                });
            }
            Err(error) => self.status_message = Some(error.to_string()),
        }
        cx.notify();
    }

    /// Marks the current media done and moves on to the next pending one.
    pub(super) fn done_and_next(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(asset_id) = self.state.selected_asset() else {
            return;
        };
        if let Err(error) = self.state.set_asset_status(asset_id, AssetStatus::Done) {
            self.status_message = Some(error.to_string());
            cx.notify();
            return;
        }
        let next = self
            .state
            .adjacent_asset(Some(asset_id), Direction::Next, true)
            .or_else(|| self.state.adjacent_asset(None, Direction::Next, true));
        match next {
            Some(next) => self.go_to_media(next, window, cx),
            None => {
                self.status_message = Some("All media done".to_string());
                cx.notify();
            }
        }
    }

    fn step_annotation(
        &mut self,
        direction: Direction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(annotation_id) = self.state.adjacent_annotation(direction) {
            self.select_annotation(Some(annotation_id), window, cx);
        }
    }

    /// Reviews the selected annotation, or the first one on this media when
    /// none is selected. Accepting or rejecting then selects the next
    /// annotation still in draft, so a review pass is one key per shape.
    pub(super) fn review_selection(
        &mut self,
        review_state: ReviewState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self
            .state
            .selected_annotation()
            .or_else(|| self.state.adjacent_annotation(Direction::Next));
        let Some(annotation_id) = target else {
            self.status_message = Some("No annotation to review".to_string());
            cx.notify();
            return;
        };
        if let Err(error) = self
            .state
            .set_annotation_review_state(annotation_id, review_state)
        {
            self.status_message = Some(error.to_string());
            cx.notify();
            return;
        }
        self.status_message = Some(
            match review_state {
                ReviewState::Accepted => "Accepted annotation",
                ReviewState::Rejected => "Rejected annotation",
                ReviewState::Reviewed => "Marked reviewed",
                ReviewState::Draft => "Reset to draft",
            }
            .to_string(),
        );
        if review_state != ReviewState::Draft
            && let Some(next) = self.state.next_draft_annotation(annotation_id)
        {
            self.select_annotation(Some(next), window, cx);
        }
        cx.notify();
    }

    pub(super) fn delete_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.tools.draw_polygon.is_some() {
            self.tools.remove_last_polygon_point();
            cx.notify();
            return;
        }
        let Some(annotation_id) = self.state.selected_annotation() else {
            return;
        };
        self.delete_annotation(annotation_id, window, cx);
    }

    pub(super) fn delete_annotation(
        &mut self,
        annotation_id: AnnotationId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.state.remove_annotation(annotation_id) {
            Ok(()) => {
                self.hovered_annotation = None;
                self.status_message = Some("Deleted annotation".to_string());
                self.sync_label_name_input(window, cx);
            }
            Err(error) => self.status_message = Some(error.to_string()),
        }
        cx.notify();
    }

    /// Escape: abandon an in-progress shape first, then clear the selection.
    fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.tools.draw_box.is_some() || self.tools.draw_polygon.is_some() {
            self.tools.cancel_drawing();
            self.status_message = Some("Cancelled drawing".to_string());
            cx.notify();
        } else if self.state.selected_annotation().is_some() {
            self.select_annotation(None, window, cx);
        }
    }

    fn zoom_canvas(&mut self, factor: f32, cx: &mut Context<Self>) {
        let layout = *self.canvas_image_layout.borrow();
        match layout {
            Some(layout) => {
                let center = layout.fit_bounds.center();
                self.tools
                    .viewport
                    .zoom_at(center, layout.fit_bounds, factor);
            }
            None => self.tools.viewport.zoom_by(factor),
        }
        cx.notify();
    }

    /// Makes a label active and applies it to the selected annotation, so
    /// number keys relabel boxes as quickly as they pick the drawing label.
    pub(super) fn choose_label(
        &mut self,
        label_id: LabelId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.state.selected_annotation() {
            Some(annotation_id) => {
                self.update_annotation_label(annotation_id, label_id, window, cx)
            }
            None => self.select_label(label_id, window, cx),
        }
    }

    /// One-line summary of review progress for the status bar.
    pub(super) fn progress_summary(&self) -> Option<String> {
        let assets = &self.state.dataset().assets;
        if assets.is_empty() {
            return None;
        }
        let done = assets
            .iter()
            .filter(|asset| asset.status == AssetStatus::Done)
            .count();
        let position = self
            .state
            .selected_asset()
            .and_then(|id| assets.iter().position(|asset| asset.id == id))
            .map(|index| format!("{} of {}", index + 1, assets.len()))
            .unwrap_or_else(|| format!("{} media", assets.len()));
        Some(format!("{position} · {done} done"))
    }

    pub(super) fn current_annotation_summary(&self) -> Option<String> {
        let asset = self.selected_asset()?;
        let count = self.annotations_for_asset(asset).len();
        Some(format!("{count} annotation{}", plural(count)))
    }
}
