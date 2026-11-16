//! Choosing which part of a dataset an export covers.

use notatus_core::{AssetStatus, Dataset};
use std::collections::BTreeSet;

/// A copy of `dataset` with only the media marked done and the records on
/// them.
///
/// Every exported image without annotations tells a model "nothing here",
/// so exporting media nobody has finished teaches it to miss objects. Labels
/// are kept whole, so class indices match an export of the full dataset.
pub fn finished_media(dataset: &Dataset) -> Dataset {
    let done: BTreeSet<_> = dataset
        .assets
        .iter()
        .filter(|asset| asset.status == AssetStatus::Done)
        .map(|asset| asset.id)
        .collect();

    Dataset {
        manifest: dataset.manifest.clone(),
        labels: dataset.labels.clone(),
        assets: dataset
            .assets
            .iter()
            .filter(|asset| done.contains(&asset.id))
            .cloned()
            .collect(),
        annotations: dataset
            .annotations
            .iter()
            .filter(|annotation| done.contains(&annotation.asset_id))
            .cloned()
            .collect(),
        classifications: dataset
            .classifications
            .iter()
            .filter(|classification| done.contains(&classification.asset_id))
            .cloned()
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use notatus_core::{
        AnnotationGeometry, AnnotationRecord, AssetLocation, AssetRecord, BoundingBox,
        ClassificationRecord,
    };

    #[test]
    fn keeps_only_finished_media_and_their_records() {
        let mut dataset = Dataset::new("demo");
        let car = dataset.add_label("car");
        let person = dataset.add_label("person");
        let mut finished = AssetRecord::new_image(AssetLocation::local("a.jpg"), 10, 10).unwrap();
        finished.status = AssetStatus::Done;
        let finished = dataset.add_asset(finished);
        let pending = dataset
            .add_asset(AssetRecord::new_image(AssetLocation::local("b.jpg"), 10, 10).unwrap());
        let bbox = AnnotationGeometry::Bbox(BoundingBox::from_xywh(1.0, 1.0, 2.0, 2.0).unwrap());
        for asset_id in [finished, pending] {
            dataset.add_annotation(AnnotationRecord::new_human(
                asset_id,
                car,
                bbox.clone(),
                None,
            ));
            dataset.add_classification(ClassificationRecord::new_human(asset_id, person, None));
        }

        let selected = finished_media(&dataset);

        assert_eq!(selected.labels, dataset.labels);
        assert_eq!(selected.assets.len(), 1);
        assert_eq!(selected.assets[0].id, finished);
        assert!(selected.annotations.iter().all(|a| a.asset_id == finished));
        assert_eq!(selected.annotations.len(), 1);
        assert_eq!(selected.classifications.len(), 1);
        assert!(selected.validate().is_ok());
    }
}
