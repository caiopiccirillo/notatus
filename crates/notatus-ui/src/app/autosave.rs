//! Saving projects that live on disk.
//!
//! Every write to the project folder goes through one pipeline with at most
//! one save in flight, so a manual save can never race an autosave writing
//! the same files. Edits schedule a save a short moment later, and Ctrl+S
//! saves right away.
//!
//! Saves run on a background thread. A save that finishes after newer edits
//! only marks the revision it wrote as saved.

use super::*;
use std::time::Duration;

pub(super) const AUTOSAVE_DELAY: Duration = Duration::from_millis(1500);

#[derive(Default)]
pub(super) struct AutosaveState {
    pub(super) enabled: bool,
    /// Pending debounce timer. Replacing it cancels the previous one.
    timer: Option<gpui::Task<()>>,
    saving: bool,
    /// The revision last handed to the scheduler, to ignore re-renders.
    scheduled_revision: Option<u64>,
    /// Bumped when another project is loaded, so late saves are ignored.
    generation: u64,
    /// Someone asked to save while a save was running; start another one
    /// as soon as it finishes, since it may predate their latest edit.
    save_requested: bool,
    /// Report "Saved project" when the current save finishes, because
    /// someone asked for it rather than the timer.
    announce: bool,
}

impl AutosaveState {
    pub(super) fn new() -> Self {
        Self {
            enabled: true,
            ..Self::default()
        }
    }

    /// Forgets pending work after the project was replaced or moved.
    pub(super) fn reset(&mut self) {
        let enabled = self.enabled;
        let generation = self.generation + 1;
        *self = Self {
            enabled,
            generation,
            ..Self::default()
        };
    }
}

impl NotatusWindow {
    /// Schedules a save when the dataset changed since the last save.
    /// Call after every update that may have mutated state.
    pub(super) fn schedule_autosave(&mut self, cx: &mut Context<Self>) {
        if !self.autosave.enabled
            || !self.state.dirty
            || self.project_location.path().is_none()
            || self.autosave.scheduled_revision == Some(self.state.revision())
        {
            return;
        }
        self.autosave.scheduled_revision = Some(self.state.revision());
        self.autosave.timer = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(AUTOSAVE_DELAY).await;
            let _ = this.update(cx, |this, cx| this.autosave_now(cx));
        }));
    }

    fn autosave_now(&mut self, cx: &mut Context<Self>) {
        self.autosave.timer = None;
        if self.autosave.saving {
            // The running save reschedules once it finishes.
            return;
        }
        if self.state.dirty {
            self.start_save(cx);
        }
    }

    /// Saves to the project folder right away, even when nothing changed,
    /// and reports the result. Waits for a save already running first.
    pub(super) fn save_now(&mut self, cx: &mut Context<Self>) {
        if self.project_location.path().is_none() {
            return;
        }
        self.autosave.timer = None;
        self.autosave.announce = true;
        if self.autosave.saving {
            self.autosave.save_requested = true;
        } else {
            self.start_save(cx);
        }
        cx.notify();
    }

    fn start_save(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.project_location.path().map(Path::to_path_buf) else {
            return;
        };

        let revision = self.state.revision();
        let generation = self.autosave.generation;
        let dataset = self.state.dataset.clone();
        self.autosave.saving = true;
        cx.notify();

        let save = cx
            .background_executor()
            .spawn(async move { LocalProjectStore::new(&path).save_dataset(&dataset) });
        cx.spawn(async move |this, cx| {
            let result = save.await;
            let _ = this.update(cx, |this, cx| {
                if this.autosave.generation == generation {
                    this.finish_save(revision, result.map_err(|error| error.to_string()), cx);
                }
            });
        })
        .detach();
    }

    fn finish_save(&mut self, revision: u64, result: Result<(), String>, cx: &mut Context<Self>) {
        self.autosave.saving = false;
        if let Err(error) = result {
            let announced = std::mem::take(&mut self.autosave.announce);
            self.autosave.save_requested = false;
            self.status_message = Some(if announced {
                format!("Save failed: {error}")
            } else {
                format!("Autosave failed: {error}")
            });
            // Retry only after the next edit, not in a loop.
            cx.notify();
            return;
        }

        self.state.mark_revision_saved(revision);
        self.autosave.scheduled_revision = None;
        if std::mem::take(&mut self.autosave.save_requested) {
            self.start_save(cx);
        } else {
            if std::mem::take(&mut self.autosave.announce) {
                self.status_message = Some("Saved project".to_string());
            }
            self.schedule_autosave(cx);
        }
        cx.notify();
    }

    /// Short save-state text for the status bar.
    pub(super) fn save_status(&self) -> &'static str {
        if self.autosave.saving {
            return "Saving…";
        }
        match (&self.project_location, self.state.dirty) {
            (ProjectLocation::Unsaved, _) => "Not saved to disk",
            (_, false) => "Saved",
            (ProjectLocation::Local(_), true) if self.autosave.enabled => "Edited",
            (ProjectLocation::Local(_), true) => "Unsaved",
        }
    }
}
