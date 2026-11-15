//! Snapshot-based undo history for [`crate::UiState`].
//!
//! Every recorded entry holds the dataset and selection as they were before a
//! change. Continuous edits (dragging a box, typing a name) share a coalescing
//! key so they collapse into a single undo step.

use notatus_core::{AnnotationId, AssetId, Dataset, LabelId};

const HISTORY_LIMIT: usize = 200;

/// Groups consecutive changes of the same kind into one undo step.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoalesceKey {
    ProjectName,
    LabelName(LabelId),
    LabelColor(LabelId),
    AnnotationGeometry(AnnotationId),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Selection {
    pub(crate) asset: Option<AssetId>,
    pub(crate) annotation: Option<AnnotationId>,
    pub(crate) label: Option<LabelId>,
}

#[derive(Clone, Debug)]
pub(crate) struct Snapshot {
    pub(crate) dataset: Dataset,
    pub(crate) selection: Selection,
    pub(crate) revision: u64,
}

#[derive(Clone, Debug)]
struct Entry {
    snapshot: Snapshot,
    description: &'static str,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct History {
    undo: Vec<Entry>,
    redo: Vec<Entry>,
    coalesce: Option<CoalesceKey>,
}

impl History {
    /// Whether a change with `key` folds into the previous entry, so no new
    /// snapshot is needed.
    pub(crate) fn coalesces(&self, key: Option<CoalesceKey>) -> bool {
        key.is_some() && key == self.coalesce && !self.undo.is_empty()
    }

    /// Records the state before a change.
    pub(crate) fn record(
        &mut self,
        snapshot: Snapshot,
        description: &'static str,
        key: Option<CoalesceKey>,
    ) {
        self.coalesce = key;
        self.redo.clear();
        self.undo.push(Entry {
            snapshot,
            description,
        });
        if self.undo.len() > HISTORY_LIMIT {
            self.undo.remove(0);
        }
    }

    /// Ends the current coalescing run, so the next change starts a new step.
    pub(crate) fn break_coalescing(&mut self) {
        self.coalesce = None;
    }

    pub(crate) fn undo(&mut self, current: Snapshot) -> Option<(Snapshot, &'static str)> {
        let entry = self.undo.pop()?;
        self.coalesce = None;
        self.redo.push(Entry {
            snapshot: current,
            description: entry.description,
        });
        Some((entry.snapshot, entry.description))
    }

    pub(crate) fn redo(&mut self, current: Snapshot) -> Option<(Snapshot, &'static str)> {
        let entry = self.redo.pop()?;
        self.coalesce = None;
        self.undo.push(Entry {
            snapshot: current,
            description: entry.description,
        });
        Some((entry.snapshot, entry.description))
    }

    pub(crate) fn undo_description(&self) -> Option<&'static str> {
        self.undo.last().map(|entry| entry.description)
    }

    pub(crate) fn redo_description(&self) -> Option<&'static str> {
        self.redo.last().map(|entry| entry.description)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(revision: u64) -> Snapshot {
        Snapshot {
            dataset: Dataset::new("demo"),
            selection: Selection::default(),
            revision,
        }
    }

    #[test]
    fn coalesces_only_matching_keys() {
        let mut history = History::default();
        let key = Some(CoalesceKey::ProjectName);
        assert!(!history.coalesces(key));

        history.record(snapshot(0), "Rename project", key);
        assert!(history.coalesces(key));
        assert!(!history.coalesces(None));

        history.break_coalescing();
        assert!(!history.coalesces(key));
    }

    #[test]
    fn new_change_clears_redo() {
        let mut history = History::default();
        history.record(snapshot(0), "Draw box", None);
        history.undo(snapshot(1)).unwrap();
        assert_eq!(history.redo_description(), Some("Draw box"));

        history.record(snapshot(0), "Create label", None);
        assert_eq!(history.redo_description(), None);
    }

    #[test]
    fn caps_history_length() {
        let mut history = History::default();
        for revision in 0..(HISTORY_LIMIT as u64 + 10) {
            history.record(snapshot(revision), "Draw box", None);
        }
        assert_eq!(history.undo.len(), HISTORY_LIMIT);
        assert_eq!(history.undo[0].snapshot.revision, 10);
    }
}
