//! Tagging whole images with labels.

use super::{UiMutationError, UiState};
use notatus_core::{AssetId, ClassificationId, ClassificationRecord, LabelId};

impl UiState {
    #[tracing::instrument(level = "debug", skip_all, fields(asset_id = %asset_id, label_id = %label_id))]
    pub fn toggle_image_classification(
        &mut self,
        asset_id: AssetId,
        label_id: LabelId,
    ) -> Result<Option<ClassificationId>, UiMutationError> {
        if self.dataset.asset_by_id(asset_id).is_none() {
            return Err(UiMutationError::MissingAsset { asset_id });
        }
        if self.dataset.label_by_id(label_id).is_none() {
            return Err(UiMutationError::MissingLabel { label_id });
        }

        if let Some(index) = self
            .dataset
            .classifications
            .iter()
            .position(|classification| {
                classification.asset_id == asset_id && classification.label_id == label_id
            })
        {
            self.begin_change("Remove classification", None);
            self.dataset.classifications.remove(index);
            self.selected_label = Some(label_id);
            tracing::debug!(asset_id = %asset_id, label_id = %label_id, "classification toggled off");
            return Ok(None);
        }

        let classification = ClassificationRecord::new_human(asset_id, label_id, None);
        let classification_id = classification.id;
        self.begin_change("Add classification", None);
        self.dataset.add_classification(classification);
        self.selected_asset = Some(asset_id);
        self.selected_label = Some(label_id);
        tracing::debug!(%classification_id, asset_id = %asset_id, label_id = %label_id, "classification toggled on");
        Ok(Some(classification_id))
    }

    #[tracing::instrument(level = "debug", skip_all, fields(classification_id = %classification_id))]
    pub fn remove_classification(
        &mut self,
        classification_id: ClassificationId,
    ) -> Result<(), UiMutationError> {
        let classification_index = self
            .dataset
            .classifications
            .iter()
            .position(|classification| classification.id == classification_id)
            .ok_or(UiMutationError::MissingClassification { classification_id })?;

        self.begin_change("Remove classification", None);
        let classification = self.dataset.classifications.remove(classification_index);
        self.selected_asset = Some(classification.asset_id);
        self.selected_label = Some(classification.label_id);
        tracing::debug!(%classification_id, "classification removed");
        Ok(())
    }
}
