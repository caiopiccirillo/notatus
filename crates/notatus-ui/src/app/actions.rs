//! Keyboard actions, their default bindings, and the handlers that run them.

use crate::app::helpers::plural;
use crate::app::model_run::ModelScope;
use crate::app::view_options::ViewToggle;
use crate::app::window::NotatusWindow;
use crate::app::{export_commands, media_import, project_commands};
use crate::{AnnotationTool, Direction};
use gpui::prelude::*;
use gpui::{Action, App, Context, KeyBinding, KeyBindingContextPredicate, Window, actions};
use gpui_component::{ActiveTheme as _, Theme};
use notatus_core::{AnnotationId, AssetId, AssetStatus, LabelId, ReviewState};
use std::rc::Rc;

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
        /// Go to the next draft to review, on any media.
        NextDraft,
        /// Go to the previous draft to review, on any media.
        PreviousDraft,
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
    let global = CONTEXT;
    let canvas = CANVAS_CONTEXT;
    let bindings = [
        // Ctrl+K (Cmd+K on macOS) is the one the app advertises, so it is bound last: hints
        // show the last binding.
        binding(cx, "secondary-shift-p", CommandPalette, global),
        binding(cx, "secondary-k", CommandPalette, global),
        binding(cx, "secondary-z", Undo, canvas),
        binding(cx, "secondary-shift-z", Redo, canvas),
        binding(cx, "secondary-y", Redo, canvas),
        binding(cx, "secondary-n", NewProject, global),
        binding(cx, "secondary-o", OpenProject, global),
        binding(cx, "secondary-s", SaveProject, global),
        binding(cx, "secondary-shift-s", SaveProjectAs, global),
        binding(cx, "secondary-i", ImportMedia, global),
        binding(cx, "secondary-shift-i", ImportFolder, global),
        binding(cx, "secondary-e", ExportAnnotations, global),
        binding(cx, "secondary-shift-l", NewLabel, global),
        binding(cx, "l", PickLabel, canvas),
        binding(cx, "v", SelectTool, canvas),
        binding(cx, "b", BoxTool, canvas),
        binding(cx, "p", PolygonTool, canvas),
        // The last binding for an action is the one shown in hints.
        binding(cx, "right", NextMedia, canvas),
        binding(cx, "d", NextMedia, canvas),
        binding(cx, "left", PreviousMedia, canvas),
        binding(cx, "a", PreviousMedia, canvas),
        binding(cx, "shift-d", NextPendingMedia, canvas),
        binding(cx, "m", ToggleMediaDone, canvas),
        binding(cx, "secondary-enter", DoneAndNext, global),
        binding(cx, "tab", NextAnnotation, canvas),
        binding(cx, "shift-tab", PreviousAnnotation, canvas),
        binding(cx, "backspace", DeleteSelection, canvas),
        binding(cx, "delete", DeleteSelection, canvas),
        binding(cx, "escape", Cancel, canvas),
        binding(cx, "f", FitCanvas, canvas),
        binding(cx, "secondary-0", FitCanvas, global),
        binding(cx, "=", ZoomIn, canvas),
        binding(cx, "secondary-=", ZoomIn, global),
        binding(cx, "-", ZoomOut, canvas),
        binding(cx, "secondary--", ZoomOut, global),
        binding(cx, "h", ToggleHideAnnotations, canvas),
        binding(cx, "shift-l", ToggleLabelNames, canvas),
        binding(cx, "c", ToggleCrosshair, canvas),
        binding(cx, "shift-f", ToggleFocusSelected, canvas),
        binding(cx, "?", ShowShortcuts, canvas),
        binding(cx, "secondary-r", RunModel, global),
        binding(cx, "secondary-shift-r", RunModelOnUnfinished, global),
        binding(cx, "y", AcceptAnnotation, canvas),
        binding(cx, "x", RejectAnnotation, canvas),
        binding(cx, "u", ResetAnnotationReview, canvas),
        binding(cx, "n", NextDraft, canvas),
        binding(cx, "shift-n", PreviousDraft, canvas),
    ];
    cx.bind_keys(bindings);
    let slots: Vec<_> = (0..9)
        .map(|slot| {
            const KEYS: [&str; 9] = ["1", "2", "3", "4", "5", "6", "7", "8", "9"];
            binding(cx, KEYS[slot], SelectLabelSlot(slot), canvas)
        })
        .collect();
    cx.bind_keys(slots);
}

/// Binds `keys` to `action` where `context` matches. Loading through the app
/// resolves `secondary-` to Cmd on macOS and Ctrl elsewhere.
fn binding(cx: &App, keys: &str, action: impl Action, context: &str) -> KeyBinding {
    let context = KeyBindingContextPredicate::parse(context).expect("a valid key context");
    KeyBinding::load(
        keys,
        Box::new(action),
        Some(Rc::new(context)),
        false,
        None,
        cx,
    )
    .expect("a valid keystroke")
}

/// A command that reaches the window through a weak handle, like the menus
/// and buttons use.
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
            .on_action(cx.listener(|this, _: &Undo, _, cx| this.undo(cx)))
            .on_action(cx.listener(|this, _: &Redo, _, cx| this.redo(cx)))
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
            .on_action(cx.listener(|this, _: &NewLabel, _, cx| {
                this.create_label(cx);
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
            .on_action(cx.listener(|this, _: &AcceptAnnotation, _, cx| {
                this.review_selection(ReviewState::Accepted, cx);
            }))
            .on_action(cx.listener(|this, _: &RejectAnnotation, _, cx| {
                this.review_selection(ReviewState::Rejected, cx);
            }))
            .on_action(cx.listener(|this, _: &ResetAnnotationReview, _, cx| {
                this.review_selection(ReviewState::Draft, cx);
            }))
            .on_action(cx.listener(|this, _: &NextDraft, _, cx| {
                this.step_review_queue(Direction::Next, cx);
            }))
            .on_action(cx.listener(|this, _: &PreviousDraft, _, cx| {
                this.step_review_queue(Direction::Previous, cx);
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
            .on_action(cx.listener(|this, _: &NextMedia, _, cx| {
                this.step_media(Direction::Next, false, cx);
            }))
            .on_action(cx.listener(|this, _: &PreviousMedia, _, cx| {
                this.step_media(Direction::Previous, false, cx);
            }))
            .on_action(cx.listener(|this, _: &NextPendingMedia, _, cx| {
                this.step_media(Direction::Next, true, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleMediaDone, _, cx| {
                this.toggle_media_done(cx);
            }))
            .on_action(cx.listener(|this, _: &DoneAndNext, _, cx| {
                this.done_and_next(cx);
            }))
            .on_action(cx.listener(|this, _: &NextAnnotation, _, cx| {
                this.step_annotation(Direction::Next, cx);
            }))
            .on_action(cx.listener(|this, _: &PreviousAnnotation, _, cx| {
                this.step_annotation(Direction::Previous, cx);
            }))
            .on_action(cx.listener(|this, _: &DeleteSelection, _, cx| {
                this.delete_selection(cx);
            }))
            .on_action(cx.listener(|this, _: &Cancel, _, cx| {
                this.cancel(cx);
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
            .on_action(cx.listener(|this, action: &SelectLabelSlot, _, cx| {
                if let Some(label_id) = this.state.dataset().labels.get(action.0).map(|l| l.id) {
                    this.choose_label(label_id, cx);
                }
            }))
            .on_action(cx.listener(|this, action: &SelectLabelById, _, cx| {
                this.choose_label(action.0, cx);
            }))
            .on_action(cx.listener(|this, action: &GoToMedia, _, cx| {
                this.go_to_media(action.0, cx);
            }))
    }

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

    /// Selects a media item and resets the canvas for it.
    pub(super) fn go_to_media(&mut self, asset_id: AssetId, cx: &mut Context<Self>) {
        if self.state.selected_asset() == Some(asset_id) {
            return;
        }
        match self.state.select_asset(asset_id) {
            Ok(()) => {
                self.tools.fit_canvas_to_view();
                self.canvas_image_layout.borrow_mut().take();
                self.hovered_annotation = None;
                self.status_message = None;
            }
            Err(error) => self.status_message = Some(error.to_string()),
        }
        cx.notify();
    }

    pub(super) fn step_media(
        &mut self,
        direction: Direction,
        pending_only: bool,
        cx: &mut Context<Self>,
    ) {
        match self
            .state
            .adjacent_asset(self.state.selected_asset(), direction, pending_only)
        {
            Some(asset_id) => self.go_to_media(asset_id, cx),
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
    pub(super) fn done_and_next(&mut self, cx: &mut Context<Self>) {
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
            Some(next) => self.go_to_media(next, cx),
            None => {
                self.status_message = Some("All media done".to_string());
                cx.notify();
            }
        }
    }

    fn step_annotation(&mut self, direction: Direction, cx: &mut Context<Self>) {
        if let Some(annotation_id) = self.state.adjacent_annotation(direction) {
            self.select_annotation(Some(annotation_id), cx);
        }
    }

    /// Reviews the selected annotation, or the first one on this media when
    /// none is selected. Accepting or rejecting then selects the next draft
    /// in the review queue on the same media, so a review pass is one key per
    /// shape. It stays on the media when none is left there, so moving on
    /// is a choice: N goes to the next draft on other media.
    pub(super) fn review_selection(&mut self, review_state: ReviewState, cx: &mut Context<Self>) {
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
        let done = match review_state {
            ReviewState::Accepted => "Accepted annotation",
            ReviewState::Rejected => "Rejected annotation",
            ReviewState::Reviewed => "Marked reviewed",
            ReviewState::Draft => "Reset to draft",
        };
        let next = (review_state != ReviewState::Draft)
            .then(|| self.state.next_draft_annotation(annotation_id))
            .flatten();
        if let Some(next) = next {
            self.select_annotation(Some(next), cx);
        }
        self.status_message = Some(match (review_state, next) {
            (ReviewState::Draft, _) | (_, Some(_)) => done.to_string(),
            (_, None) => match self.state.review_queue_len() {
                0 => format!("{done}. {}", self.review_queue_summary()),
                left => format!(
                    "{done}. No drafts left on this media; press N for the next one ({left} left)"
                ),
            },
        });
        cx.notify();
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
    fn cancel(&mut self, cx: &mut Context<Self>) {
        if self.tools.draw_box.is_some() || self.tools.draw_polygon.is_some() {
            self.tools.cancel_drawing();
            self.status_message = Some("Cancelled drawing".to_string());
            cx.notify();
        } else if self.state.selected_annotation().is_some() {
            self.select_annotation(None, cx);
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
    pub(super) fn choose_label(&mut self, label_id: LabelId, cx: &mut Context<Self>) {
        match self.state.selected_annotation() {
            Some(annotation_id) => self.update_annotation_label(annotation_id, label_id, cx),
            None => self.select_label(label_id, cx),
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
        let to_review = self.state.review_queue_len();
        Some(if to_review == 0 {
            format!("{position} · {done} done")
        } else {
            format!("{position} · {done} done · {to_review} to review")
        })
    }

    pub(super) fn current_annotation_summary(&self) -> Option<String> {
        let asset = self.selected_asset()?;
        let count = self.annotations_for_asset(asset).len();
        Some(format!("{count} annotation{}", plural(count)))
    }
}
