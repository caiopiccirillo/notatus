use super::*;
use notatus_core::{
    AnnotationGeometry, AnnotationRecord, AssetLocation, AssetRecord, BoundingBox,
    ClassificationRecord, Dataset, DatasetSplit, Point, Polygon,
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
    let rows = classification::export_classifications(
        &classification_dataset(),
        &AnnotationFilter::ready_for_training(),
    )
    .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].image_path, "images/a.jpg");
    assert_eq!(rows[0].label_name, "outdoor");
}

/// A PNG signature followed by `name`, enough to pass the export's check.
fn png_bytes(name: &str) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    bytes.extend_from_slice(name.as_bytes());
    bytes
}

/// A dataset whose media are real files in `dir`, as a YOLO export needs.
fn dataset_with_image_files(dir: &std::path::Path, names: &[(&str, DatasetSplit)]) -> Dataset {
    let mut dataset = Dataset::new("demo");
    let label_id = dataset.add_label("car");
    dataset.add_label("traffic \"light\"");
    for (name, split) in names {
        let path = dir.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, png_bytes(name)).unwrap();
        let mut asset = AssetRecord::new_image(
            AssetLocation::local(path.to_string_lossy().into_owned()),
            200,
            100,
        )
        .unwrap();
        asset.split = split.clone();
        let asset_id = dataset.add_asset(asset);
        let mut annotation = AnnotationRecord::new_human(
            asset_id,
            label_id,
            AnnotationGeometry::Bbox(BoundingBox::from_xywh(75.0, 30.0, 50.0, 40.0).unwrap()),
            None,
        );
        annotation.accept();
        dataset.add_annotation(annotation);
    }
    dataset
}

#[test]
fn writes_a_trainable_yolo_dataset() {
    let media = tempfile::tempdir().unwrap();
    let dataset = dataset_with_image_files(
        media.path(),
        &[
            ("a/frame.jpg", DatasetSplit::Unassigned),
            ("b/frame.jpg", DatasetSplit::Train),
            ("c/val.png", DatasetSplit::Validation),
        ],
    );
    let output = tempfile::tempdir().unwrap();
    let yolo_dir = output.path().join("yolo");

    let summary =
        yolo::write_detection_export(&dataset, &AnnotationFilter::default(), &yolo_dir).unwrap();

    assert_eq!(summary.class_count, 2);
    assert_eq!(summary.annotation_file_count, 3);
    assert_eq!(summary.annotation_count, 3);
    let read = |path: &str| std::fs::read_to_string(yolo_dir.join(path)).unwrap();
    // Both "frame.jpg" land in train, so the second gets a unique name, and
    // every label file matches its image's stem.
    let read_bytes = |path: &str| std::fs::read(yolo_dir.join(path)).unwrap();
    assert_eq!(
        read_bytes("images/train/frame.jpg"),
        png_bytes("a/frame.jpg")
    );
    assert_eq!(
        read("labels/train/frame.txt"),
        "0 0.500000 0.500000 0.250000 0.400000\n"
    );
    let mut train_images: Vec<_> = std::fs::read_dir(yolo_dir.join("images/train"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    train_images.sort();
    assert_eq!(train_images.len(), 2);
    let renamed = train_images
        .iter()
        .find(|name| *name != "frame.jpg")
        .unwrap();
    assert!(renamed.starts_with("frame-") && renamed.ends_with(".jpg"));
    let renamed_stem = renamed.trim_end_matches(".jpg");
    assert!(
        yolo_dir
            .join(format!("labels/train/{renamed_stem}.txt"))
            .is_file()
    );
    assert_eq!(read_bytes("images/val/val.png"), png_bytes("c/val.png"));
    assert!(yolo_dir.join("labels/val/val.txt").is_file());

    let yaml = read("data.yaml");
    assert!(yaml.contains(&format!("path: \"{}\"", yolo_dir.display())));
    assert!(yaml.contains("train: images/train\n"));
    assert!(yaml.contains("val: images/val\n"));
    assert!(!yaml.contains("test:"));
    assert!(yaml.contains("  0: \"car\"\n"));
    assert!(yaml.contains("  1: \"traffic \\\"light\\\"\"\n"));
    assert_eq!(read("classes.txt"), "car\ntraffic \"light\"\n");
}

#[test]
fn yolo_export_without_validation_media_validates_on_train() {
    let media = tempfile::tempdir().unwrap();
    let dataset = dataset_with_image_files(media.path(), &[("a.jpg", DatasetSplit::Unassigned)]);
    let output = tempfile::tempdir().unwrap();

    yolo::write_detection_export(
        &dataset,
        &AnnotationFilter::default(),
        output.path().join("y"),
    )
    .unwrap();

    let yaml = std::fs::read_to_string(output.path().join("y/data.yaml")).unwrap();
    assert!(yaml.contains("val: images/train\n"));
}

#[test]
fn yolo_export_replaces_its_own_earlier_export() {
    let media = tempfile::tempdir().unwrap();
    let mut dataset = dataset_with_image_files(
        media.path(),
        &[
            ("a.jpg", DatasetSplit::Train),
            ("b.jpg", DatasetSplit::Train),
        ],
    );
    let output = tempfile::tempdir().unwrap();
    let yolo_dir = output.path().join("yolo");
    yolo::write_detection_export(&dataset, &AnnotationFilter::default(), &yolo_dir).unwrap();

    let removed = dataset.assets.pop().unwrap();
    dataset
        .annotations
        .retain(|annotation| annotation.asset_id != removed.id);
    yolo::write_detection_export(&dataset, &AnnotationFilter::default(), &yolo_dir).unwrap();

    assert!(yolo_dir.join("labels/train/a.txt").is_file());
    assert!(
        !yolo_dir.join("labels/train/b.txt").exists(),
        "no stale labels"
    );
    let leftovers: Vec<_> = std::fs::read_dir(output.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(leftovers, vec![std::ffi::OsString::from("yolo")]);
}

#[test]
fn yolo_export_never_replaces_someone_elses_folder() {
    let media = tempfile::tempdir().unwrap();
    let dataset = dataset_with_image_files(media.path(), &[("a.jpg", DatasetSplit::Train)]);
    let output = tempfile::tempdir().unwrap();
    let yolo_dir = output.path().join("yolo");
    std::fs::create_dir_all(&yolo_dir).unwrap();
    std::fs::write(yolo_dir.join("notes.txt"), "mine").unwrap();

    let error = yolo::write_detection_export(&dataset, &AnnotationFilter::default(), &yolo_dir)
        .unwrap_err();

    assert!(matches!(error, ExportError::OutputExists { .. }));
    assert_eq!(
        std::fs::read_to_string(yolo_dir.join("notes.txt")).unwrap(),
        "mine"
    );
    assert_eq!(std::fs::read_dir(output.path()).unwrap().count(), 1);
}

#[test]
fn yolo_export_fails_cleanly_when_an_image_is_missing() {
    let media = tempfile::tempdir().unwrap();
    let dataset = dataset_with_image_files(media.path(), &[("a.jpg", DatasetSplit::Train)]);
    std::fs::remove_file(media.path().join("a.jpg")).unwrap();
    let output = tempfile::tempdir().unwrap();

    let error = yolo::write_detection_export(
        &dataset,
        &AnnotationFilter::default(),
        output.path().join("yolo"),
    )
    .unwrap_err();

    assert!(matches!(error, ExportError::Image { .. }));
    assert_eq!(std::fs::read_dir(output.path()).unwrap().count(), 0);
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

    let summary = classification::write_classification_export(
        &classification_dataset(),
        &AnnotationFilter::ready_for_training(),
        output.path(),
    )
    .unwrap();

    assert_eq!(summary.classification_count, 1);
    let json = std::fs::read_to_string(output.path().join("classifications.json")).unwrap();
    assert!(json.contains("\"label_name\": \"outdoor\""));
    let csv = std::fs::read_to_string(output.path().join("classifications.csv")).unwrap();
    assert!(csv.contains("classification_id,asset_id,image_path,label_id,label_name"));
    assert!(csv.contains("images/a.jpg"));
}

#[test]
fn yolo_roundtrip_keeps_boxes_touching_every_edge() {
    let mut dataset = Dataset::new("edges");
    let label_id = dataset.add_label("car");
    let asset = AssetRecord::new_image(AssetLocation::local("a.jpg"), 1920, 1080).unwrap();
    let asset_id = dataset.add_asset(asset);
    // Boxes flush with each edge, sized so 6-decimal rounding pushes them
    // past the edge before clipping.
    let boxes = [
        BoundingBox::from_xywh(0.0, 0.0, 333.3, 77.7).unwrap(),
        BoundingBox::from_xywh(1920.0 - 457.1, 1080.0 - 91.3, 457.1, 91.3).unwrap(),
        BoundingBox::from_xywh(0.0, 1080.0 - 0.7, 1920.0, 0.7).unwrap(),
    ];
    for bbox in boxes {
        let mut annotation =
            AnnotationRecord::new_human(asset_id, label_id, AnnotationGeometry::Bbox(bbox), None);
        annotation.accept();
        dataset.add_annotation(annotation);
    }

    let files = yolo::export_detection(&dataset, &AnnotationFilter::default()).unwrap();
    let imported = yolo::import_detection(
        &dataset,
        &[yolo::YoloImportFile {
            asset_id,
            contents: files[0].contents.clone(),
        }],
    )
    .unwrap();

    assert_eq!(imported.len(), boxes.len());
    for (bbox, annotation) in boxes.iter().zip(&imported) {
        let got = annotation.geometry.as_bbox().unwrap();
        for (expected, actual) in [
            (bbox.x, got.x),
            (bbox.y, got.y),
            (bbox.width, got.width),
            (bbox.height, got.height),
        ] {
            assert!((expected - actual).abs() < 0.01, "{bbox:?} became {got:?}");
        }
        assert!(got.max_x() <= 1920.0 && got.max_y() <= 1080.0);
    }
}

#[test]
fn yolo_import_rejects_values_far_outside_the_image() {
    let dataset = sample_dataset();
    let import = |contents: &str| {
        yolo::import_detection(
            &dataset,
            &[yolo::YoloImportFile {
                asset_id: dataset.assets[0].id,
                contents: contents.to_string(),
            }],
        )
    };

    assert!(import("0 1.0000004 0.5 0.2 0.2").is_ok());
    assert!(import("0 1.5 0.5 0.2 0.2").is_err());
    assert!(import("0 0.5 0.5 0 0.2").is_err());
    assert!(import("0 0.5 0.5 NaN 0.2").is_err());
}

#[test]
fn training_filter_leaves_out_unreviewed_model_predictions() {
    let mut dataset = sample_dataset();
    let asset_id = dataset.assets[0].id;
    let label_id = dataset.labels[0].id;
    let model = notatus_core::ModelProvenance {
        name: "detector".to_string(),
        version: None,
        invocation_id: None,
        metadata: Default::default(),
    };
    let bbox = AnnotationGeometry::Bbox(BoundingBox::from_xywh(1.0, 1.0, 5.0, 5.0).unwrap());
    let human_draft = AnnotationRecord::new_human(asset_id, label_id, bbox.clone(), None);
    let model_draft =
        AnnotationRecord::new_model(asset_id, label_id, bbox.clone(), model.clone(), Some(0.9));
    let mut model_accepted =
        AnnotationRecord::new_model(asset_id, label_id, bbox, model.clone(), Some(0.8));
    model_accepted.accept();
    let filter = AnnotationFilter::ready_for_training();

    assert!(filter.accepts(&human_draft));
    assert!(!filter.accepts(&model_draft));
    assert!(filter.accepts(&model_accepted));
    assert!(AnnotationFilter::all_non_rejected().accepts(&model_draft));

    dataset.add_classification(ClassificationRecord::new_model(
        asset_id,
        label_id,
        model,
        Some(0.7),
    ));
    let rows = classification::export_classifications(&dataset, &filter).unwrap();
    assert!(rows.is_empty());
}

#[test]
fn yolo_export_refuses_media_that_is_not_an_image() {
    let media = tempfile::tempdir().unwrap();
    let dataset = dataset_with_image_files(media.path(), &[("a.jpg", DatasetSplit::Train)]);
    // A project from someone else pointing a media path at a private file.
    std::fs::write(
        media.path().join("a.jpg"),
        "-----BEGIN OPENSSH PRIVATE KEY-----",
    )
    .unwrap();
    let output = tempfile::tempdir().unwrap();

    let error = yolo::write_detection_export(
        &dataset,
        &AnnotationFilter::default(),
        output.path().join("yolo"),
    )
    .unwrap_err();

    assert!(
        error.to_string().contains("not a recognized image file"),
        "{error}"
    );
    assert_eq!(std::fs::read_dir(output.path()).unwrap().count(), 0);
}
