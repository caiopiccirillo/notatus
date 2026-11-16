//! Importing, selecting, removing, finishing, and stepping through media.

use super::{Direction, RemovalSummary, UiMutationError, UiState};
use notatus_core::{AssetId, AssetLocation, AssetRecord, AssetStatus};

impl UiState {
    #[tracing::instrument(level = "debug", skip_all, fields(width = width, height = height))]
    pub fn add_local_image_asset(
        &mut self,
        path: impl Into<String>,
        width: u32,
        height: u32,
    ) -> Result<AssetId, UiMutationError> {
        let path = path.into();
        let asset = AssetRecord::new_image(AssetLocation::local(path), width, height)?;
        self.begin_change("Import media", None);
        let asset_id = self.dataset.add_asset(asset);
        self.selected_asset = Some(asset_id);
        tracing::debug!(asset_id = %asset_id, "image asset added via UI");
        Ok(asset_id)
    }

    #[tracing::instrument(level = "debug", skip_all, fields(asset_id = %asset_id))]
    pub fn select_asset(&mut self, asset_id: AssetId) -> Result<(), UiMutationError> {
        if self.dataset.asset_by_id(asset_id).is_none() {
            tracing::warn!(%asset_id, "selected missing asset");
            return Err(UiMutationError::MissingAsset { asset_id });
        }

        self.selected_asset = Some(asset_id);
        self.selected_annotation = None;
        Ok(())
    }

    #[tracing::instrument(level = "debug", skip_all, fields(asset_id = %asset_id))]
    pub fn remove_asset(&mut self, asset_id: AssetId) -> Result<RemovalSummary, UiMutationError> {
        let asset_index = self
            .dataset
            .assets
            .iter()
            .position(|asset| asset.id == asset_id)
            .ok_or(UiMutationError::MissingAsset { asset_id })?;

        let selected_annotation_removed = self.selected_annotation.is_some_and(|annotation_id| {
            self.dataset
                .annotations
                .iter()
                .any(|annotation| annotation.id == annotation_id && annotation.asset_id == asset_id)
        });
        let original_annotation_count = self.dataset.annotations.len();
        let original_classification_count = self.dataset.classifications.len();

        self.begin_change("Remove media", None);
        self.dataset.assets.remove(asset_index);
        self.dataset
            .annotations
            .retain(|annotation| annotation.asset_id != asset_id);
        self.dataset
            .classifications
            .retain(|classification| classification.asset_id != asset_id);

        if self.selected_asset == Some(asset_id) {
            // Keep the user's place in the list: the next item, else the previous.
            let neighbor = asset_index.min(self.dataset.assets.len().saturating_sub(1));
            self.selected_asset = self.dataset.assets.get(neighbor).map(|asset| asset.id);
        }
        if selected_annotation_removed {
            self.selected_annotation = None;
        }

        let summary = RemovalSummary {
            annotations: original_annotation_count - self.dataset.annotations.len(),
            classifications: original_classification_count - self.dataset.classifications.len(),
        };
        tracing::info!(asset_id = %asset_id, removed_annotations = summary.annotations, removed_classifications = summary.classifications, "asset removed");
        Ok(summary)
    }

    #[tracing::instrument(level = "debug", skip_all, fields(asset_id = %asset_id))]
    pub fn set_asset_status(
        &mut self,
        asset_id: AssetId,
        status: AssetStatus,
    ) -> Result<(), UiMutationError> {
        let index = self
            .dataset
            .assets
            .iter()
            .position(|asset| asset.id == asset_id)
            .ok_or(UiMutationError::MissingAsset { asset_id })?;
        if self.dataset.assets[index].status == status {
            return Ok(());
        }
        let description = match status {
            AssetStatus::Done => "Mark media done",
            AssetStatus::Pending => "Mark media pending",
        };
        self.begin_change(description, None);
        self.dataset.assets[index].status = status;
        Ok(())
    }

    /// The asset before or after `from` in dataset order. With `pending_only`,
    /// skips media already marked done.
    pub fn adjacent_asset(
        &self,
        from: Option<AssetId>,
        direction: Direction,
        pending_only: bool,
    ) -> Option<AssetId> {
        let assets = &self.dataset.assets;
        let start = from.and_then(|id| assets.iter().position(|asset| asset.id == id));
        let eligible = |asset: &&AssetRecord| !pending_only || asset.status == AssetStatus::Pending;
        match (direction, start) {
            (Direction::Next, Some(index)) => assets[index + 1..].iter().find(eligible),
            (Direction::Previous, Some(index)) => assets[..index].iter().rev().find(eligible),
            (Direction::Next, None) => assets.iter().find(eligible),
            (Direction::Previous, None) => assets.iter().rev().find(eligible),
        }
        .map(|asset| asset.id)
    }
}
