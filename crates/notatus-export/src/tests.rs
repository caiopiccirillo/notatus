use super::*;
use notatus_core::{
    AnnotationGeometry, AnnotationRecord, AssetLocation, AssetRecord, BoundingBox,
    ClassificationRecord, Dataset, Point, Polygon,
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

fn segmentation_dataset() -> Dataset {
    let mut dataset = Dataset::new("segmentation");
    let label_id = dataset.add_label("road");
    let asset = AssetRecord::new_image(AssetLocation::local("images/a.jpg"), 200, 100).unwrap();
    let asset_id = dataset.add_asset(asset);
    let polygon = Polygon::new(vec![
        Point::new(10.0, 20.0).unwrap(),
        Point::new(100.0, 20.0).unwrap(),
        Point::new(100.0, 80.0).unwrap(),
        Point::new(10.0, 80.0).unwrap(),
    ])
    .unwrap();
    let mut annotation = AnnotationRecord::new_human(
        asset_id,
        label_id,
        AnnotationGeometry::Polygon(polygon),
        None,
    );
    annotation.accept();
    dataset.add_annotation(annotation);
    dataset
}

fn classification_dataset() -> Dataset {
    let mut dataset = Dataset::new("classification");
    let label_id = dataset.add_label("outdoor");
    let asset = AssetRecord::new_image(AssetLocation::local("images/a.jpg"), 200, 100).unwrap();
    let asset_id = dataset.add_asset(asset);
    dataset.add_classification(ClassificationRecord::new_human(asset_id, label_id, None));
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

#[test]
fn exports_coco_polygon_segmentation_dataset() {
    let exported =
        coco::export_detection(&segmentation_dataset(), &AnnotationFilter::default()).unwrap();

    assert_eq!(exported.annotations.len(), 1);
    assert_eq!(exported.annotations[0].bbox, [10.0, 20.0, 90.0, 60.0]);
    assert_eq!(exported.annotations[0].area, 5400.0);
    assert_eq!(
        exported.annotations[0].segmentation,
        vec![vec![10.0, 20.0, 100.0, 20.0, 100.0, 80.0, 10.0, 80.0]]
    );
}

#[test]
fn exports_image_classifications() {
    let rows = classification::export_classifications(&classification_dataset()).unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].image_path, "images/a.jpg");
    assert_eq!(rows[0].label_name, "outdoor");
}

#[test]
fn writes_yolo_detection_export_layout() {
    let dataset = sample_dataset();
    let output = tempfile::tempdir().unwrap();

    let summary =
        yolo::write_detection_export(&dataset, &AnnotationFilter::default(), output.path())
            .unwrap();

    assert_eq!(summary.class_count, 1);
    assert_eq!(summary.annotation_file_count, 1);
    assert_eq!(summary.annotation_count, 1);
    assert_eq!(
        std::fs::read_to_string(output.path().join("classes.txt")).unwrap(),
        "car\n"
    );
    assert_eq!(
        std::fs::read_to_string(
            output
                .path()
                .join("labels")
                .join(format!("{}.txt", dataset.assets[0].id))
        )
        .unwrap(),
        "0 0.500000 0.500000 0.250000 0.400000"
    );
}

#[test]
fn writes_coco_detection_export_layout() {
    let output = tempfile::tempdir().unwrap();

    let summary = coco::write_detection_export(
        &sample_dataset(),
        &AnnotationFilter::default(),
        output.path(),
    )
    .unwrap();

    assert_eq!(summary.image_count, 1);
    assert_eq!(summary.category_count, 1);
    assert_eq!(summary.annotation_count, 1);
    let contents = std::fs::read_to_string(output.path().join("annotations.json")).unwrap();
    assert!(contents.contains("\"categories\""));
    assert!(contents.contains("\"name\": \"car\""));
}

#[test]
fn writes_classification_export_layout() {
    let output = tempfile::tempdir().unwrap();

    let summary =
        classification::write_classification_export(&classification_dataset(), output.path())
            .unwrap();

    assert_eq!(summary.classification_count, 1);
    let json = std::fs::read_to_string(output.path().join("classifications.json")).unwrap();
    assert!(json.contains("\"label_name\": \"outdoor\""));
    let csv = std::fs::read_to_string(output.path().join("classifications.csv")).unwrap();
    assert!(csv.contains("classification_id,asset_id,image_path,label_id,label_name"));
    assert!(csv.contains("images/a.jpg"));
}
