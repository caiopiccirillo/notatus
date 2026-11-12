//! Debounced autosave for projects that already live on disk.
//!
//! Every change to the dataset schedules a save a short moment later. Saves run
//! on a background thread with at most one in flight, and a save that finishes
//! after newer edits only marks the revision it wrote as saved.

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
}

impl AutosaveState {
    pub(super) fn new() -> Self {
        Self {
            enabled: true,
            ..Self::default()
        }
    }

    /// Forgets pending work after the project was replaced.
    pub(super) fn reset(&mut self) {
        self.timer = None;
        self.saving = false;
        self.scheduled_revision = None;
        self.generation += 1;
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
        let Some(path) = self.project_location.path().map(Path::to_path_buf) else {
            return;
        };
        if !self.state.dirty {
            return;
        }

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
                if this.autosave.generation != generation {
                    return;
                }
                this.autosave.saving = false;
                match result {
                    Ok(()) => this.state.mark_revision_saved(revision),
                    Err(error) => {
                        this.status_message = Some(format!("Autosave failed: {error}"));
                        // Retry only after the next edit, not in a loop.
                        return cx.notify();
                    }
                }
                this.autosave.scheduled_revision = None;
                this.schedule_autosave(cx);
                cx.notify();
            });
        })
        .detach();
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
