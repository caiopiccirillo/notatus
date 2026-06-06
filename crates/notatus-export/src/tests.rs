use super::*;
use notatus_core::{
    AnnotationGeometry, AnnotationRecord, AssetLocation, AssetRecord, BoundingBox, Dataset,
};

fn sample_dataset() -> Dataset {
    let mut dataset = Dataset::new("demo");
    let label_id = dataset.add_label("car");
    let asset = AssetRecord::new_image(AssetLocation::local("images/a.jpg"), 200, 100).unwrap();
    let asset_id = dataset.add_asset(asset);
    let mut annotation = AnnotationRecord::new_human(
        asset_id,
        label_id,
        AnnotationGeometry::Bbox(BoundingBox::from_xywh(75.0, 30.0, 50.0, 40.0).unwrap()),
        None,
    );
    annotation.accept();
    dataset.add_annotation(annotation);
    dataset
}

#[test]
fn exports_yolo_detection_files() {
    let files = yolo::export_detection(&sample_dataset(), &AnnotationFilter::default()).unwrap();

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].contents, "0 0.500000 0.500000 0.250000 0.400000");
}

#[test]
fn imports_yolo_detection_files() {
    let dataset = sample_dataset();
    let imported = yolo::import_detection(
        &dataset,
        &[yolo::YoloImportFile {
            asset_id: dataset.assets[0].id,
            contents: "0 0.500000 0.500000 0.250000 0.400000".to_string(),
        }],
    )
    .unwrap();

    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0].label_id, dataset.labels[0].id);
}

#[test]
fn exports_coco_detection_dataset() {
    let exported = coco::export_detection(&sample_dataset(), &AnnotationFilter::default()).unwrap();

    assert_eq!(exported.images.len(), 1);
    assert_eq!(exported.categories[0].name, "car");
    assert_eq!(exported.annotations[0].bbox, [75.0, 30.0, 50.0, 40.0]);
}
