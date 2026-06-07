use super::overlay::AnnotationOverlay;
use super::*;

pub(super) fn hit_test_bbox_annotation(
    annotations: &[AnnotationOverlay],
    image_pos: (f64, f64),
) -> Option<AnnotationId> {
    annotations
        .iter()
        .rev()
        .find_map(|annotation| match &annotation.geometry {
            AnnotationGeometry::Bbox(bbox) => {
                bbox_contains_point(*bbox, image_pos).then_some(annotation.id)
            }
            AnnotationGeometry::Polygon(_) => None,
        })
}

fn bbox_contains_point(bbox: BoundingBox, (x, y): (f64, f64)) -> bool {
    x >= bbox.x && x <= bbox.max_x() && y >= bbox.y && y <= bbox.max_y()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_test_selects_topmost_bbox_annotation() {
        let bottom_id = AnnotationId::new();
        let top_id = AnnotationId::new();
        let annotations = vec![
            AnnotationOverlay {
                id: bottom_id,
                geometry: AnnotationGeometry::Bbox(
                    BoundingBox::from_xywh(0.0, 0.0, 100.0, 100.0).unwrap(),
                ),
                color: DEFAULT_LABEL_COLOR.to_string(),
                selected: false,
                hovered: false,
            },
            AnnotationOverlay {
                id: top_id,
                geometry: AnnotationGeometry::Bbox(
                    BoundingBox::from_xywh(25.0, 25.0, 100.0, 100.0).unwrap(),
                ),
                color: DEFAULT_LABEL_COLOR.to_string(),
                selected: false,
                hovered: false,
            },
        ];

        assert_eq!(
            hit_test_bbox_annotation(&annotations, (50.0, 50.0)),
            Some(top_id)
        );
        assert_eq!(hit_test_bbox_annotation(&annotations, (200.0, 200.0)), None);
    }
}
