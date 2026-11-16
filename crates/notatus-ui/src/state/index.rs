//! Lookups and totals derived from the dataset, built in one pass and kept
//! until the dataset or the review filter changes.
//!
//! The window draws on every pointer move while a shape is drawn, and its
//! panels need per-media annotations, per-label counts, and the review
//! queue's length. Reading them from the index keeps a frame from walking
//! every annotation several times.

use super::{ReviewFilter, UiState};
use notatus_core::{
    AnnotationRecord, AssetId, AssetStatus, ClassificationRecord, Dataset, LabelId, ReviewState,
};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Default)]
pub struct DatasetIndex {
    /// Positions in `dataset.annotations` per media, in dataset order.
    annotations_by_asset: HashMap<AssetId, Vec<usize>>,
    /// Positions in `dataset.classifications` per media, in dataset order.
    classifications_by_asset: HashMap<AssetId, Vec<usize>>,
    drafts_by_asset: HashMap<AssetId, usize>,
    annotations_by_label: HashMap<LabelId, usize>,
    /// Media marked done.
    pub done_media: usize,
    /// Annotations nobody has reviewed yet.
    pub drafts: usize,
    /// Drafts the review queue visits under the current filter.
    pub review_queue: usize,
}

impl DatasetIndex {
    fn build(dataset: &Dataset, filter: ReviewFilter) -> Self {
        let mut index = Self {
            done_media: dataset
                .assets
                .iter()
                .filter(|asset| asset.status == AssetStatus::Done)
                .count(),
            ..Self::default()
        };
        for (position, annotation) in dataset.annotations.iter().enumerate() {
            index
                .annotations_by_asset
                .entry(annotation.asset_id)
                .or_default()
                .push(position);
            *index
                .annotations_by_label
                .entry(annotation.label_id)
                .or_default() += 1;
            if annotation.review_state == ReviewState::Draft {
                index.drafts += 1;
                *index
                    .drafts_by_asset
                    .entry(annotation.asset_id)
                    .or_default() += 1;
            }
            if filter.accepts(annotation) {
                index.review_queue += 1;
            }
        }
        for (position, classification) in dataset.classifications.iter().enumerate() {
            index
                .classifications_by_asset
                .entry(classification.asset_id)
                .or_default()
                .push(position);
        }
        index
    }

    pub fn annotation_count(&self, asset_id: AssetId) -> usize {
        self.annotations_by_asset.get(&asset_id).map_or(0, Vec::len)
    }

    pub fn draft_count(&self, asset_id: AssetId) -> usize {
        self.drafts_by_asset.get(&asset_id).copied().unwrap_or(0)
    }

    pub fn label_annotation_count(&self, label_id: LabelId) -> usize {
        self.annotations_by_label
            .get(&label_id)
            .copied()
            .unwrap_or(0)
    }
}

/// The index and what it was built from.
#[derive(Clone, Debug)]
pub(super) struct CachedIndex {
    revision: u64,
    filter: ReviewFilter,
    index: Rc<DatasetIndex>,
}

impl UiState {
    /// The dataset's index, rebuilt when the dataset or the review filter
    /// changed since it was last read.
    ///
    /// Mutating methods change the revision before they change the dataset,
    /// so they must not read the index in between.
    pub fn index(&self) -> Rc<DatasetIndex> {
        let mut cache = self.index.borrow_mut();
        match cache.as_ref() {
            Some(cached)
                if cached.revision == self.revision && cached.filter == self.review_filter =>
            {
                cached.index.clone()
            }
            _ => {
                let index = Rc::new(DatasetIndex::build(&self.dataset, self.review_filter));
                *cache = Some(CachedIndex {
                    revision: self.revision,
                    filter: self.review_filter,
                    index: index.clone(),
                });
                index
            }
        }
    }

    /// The annotations on a media, in the order they were made.
    pub fn annotations_on(&self, asset_id: AssetId) -> Vec<&AnnotationRecord> {
        self.index()
            .annotations_by_asset
            .get(&asset_id)
            .map(|positions| {
                positions
                    .iter()
                    .map(|&position| &self.dataset.annotations[position])
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The whole-image labels on a media, in the order they were added.
    pub fn classifications_on(&self, asset_id: AssetId) -> Vec<&ClassificationRecord> {
        self.index()
            .classifications_by_asset
            .get(&asset_id)
            .map(|positions| {
                positions
                    .iter()
                    .map(|&position| &self.dataset.classifications[position])
                    .collect()
            })
            .unwrap_or_default()
    }
}
