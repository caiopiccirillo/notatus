//! External pre-annotation protocol.
//!
//! V1 keeps inference out of the desktop binary. A model runner can be Python,
//! ONNX Runtime, TensorRT, a container, or a remote shim as long as it speaks
//! this JSON-lines protocol.

use notatus_core::{
    AnnotationGeometry, AnnotationRecord, AssetRecord, BoundingBox, ImageDimensions, Label,
    LabelId, Metadata, ModelProvenance, Point, Polygon,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct InferenceRequestId(String);

impl InferenceRequestId {
    pub fn new() -> Self {
        Self(Uuid::now_v7().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for InferenceRequestId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InferenceTask {
    ObjectDetection,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct InferenceRequest {
    pub request_id: InferenceRequestId,
    pub task: InferenceTask,
    pub asset: AssetRecord,
    pub resolved_local_path: Option<String>,
    pub labels: Vec<Label>,
    #[serde(default)]
    pub metadata: Metadata,
}

impl InferenceRequest {
    #[tracing::instrument(level = "debug", skip_all, fields(
        request_id = tracing::field::Empty,
        asset_id = %asset.id,
        label_count = labels.len(),
    ))]
    pub fn object_detection(
        asset: AssetRecord,
        labels: Vec<Label>,
        resolved_local_path: Option<String>,
    ) -> Self {
        let request = Self {
            request_id: InferenceRequestId::new(),
            task: InferenceTask::ObjectDetection,
            asset,
            resolved_local_path,
            labels,
            metadata: Metadata::new(),
        };
        tracing::debug!(request_id = %request.request_id.0, "created inference request");
        request
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ModelInfo {
    pub name: String,
    pub version: Option<String>,
    #[serde(default)]
    pub metadata: Metadata,
}

impl ModelInfo {
    pub fn provenance(&self, invocation_id: Option<String>) -> ModelProvenance {
        ModelProvenance {
            name: self.name.clone(),
            version: self.version.clone(),
            invocation_id,
            metadata: self.metadata.clone(),
        }
    }
}

/// One object a model found.
///
/// The label is named by `label_id`, one of the IDs sent in the request, or by
/// `label_name`, matched against the request's label names ignoring case, so a
/// runner can pass its model's class names through unchanged.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PredictedAnnotation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_id: Option<LabelId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_name: Option<String>,
    pub geometry: AnnotationGeometry,
    #[serde(default)]
    pub confidence: Option<f32>,
    #[serde(default)]
    pub metadata: Metadata,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct InferenceResponse {
    pub request_id: InferenceRequestId,
    pub model: ModelInfo,
    #[serde(default)]
    pub predictions: Vec<PredictedAnnotation>,
    #[serde(default)]
    pub diagnostics: Vec<String>,
}

/// Predictions turned into draft annotations, and the ones that could not be.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResolvedPredictions {
    pub annotations: Vec<AnnotationRecord>,
    /// Why each skipped prediction was skipped, plus the runner's own
    /// diagnostics.
    pub notes: Vec<String>,
}

impl InferenceResponse {
    /// Turns predictions into draft model annotations on the requested media.
    ///
    /// Models often place boxes a little past the image edge, so boxes and
    /// polygon points are clipped to the image. Predictions whose label is not
    /// in the request, or with nothing left inside the image, are skipped and
    /// noted, so one bad prediction never reaches the dataset, where it would
    /// make every save fail. Confidences outside 0..=1 are dropped.
    pub fn into_annotations(self, request: &InferenceRequest) -> ResolvedPredictions {
        let provenance = self
            .model
            .provenance(Some(request.request_id.as_str().to_string()));
        let mut resolved = ResolvedPredictions {
            notes: self.diagnostics,
            ..ResolvedPredictions::default()
        };
        let dimensions = request.asset.dimensions;

        for (index, prediction) in self.predictions.into_iter().enumerate() {
            let number = index + 1;
            let Some(label_id) = resolve_label(&prediction, &request.labels) else {
                let name = prediction
                    .label_name
                    .clone()
                    .or_else(|| prediction.label_id.map(|id| id.to_string()))
                    .unwrap_or_else(|| "no label".to_string());
                resolved
                    .notes
                    .push(format!("prediction {number}: unknown label \"{name}\""));
                continue;
            };
            let Some(geometry) = clip_to_image(prediction.geometry, dimensions) else {
                resolved
                    .notes
                    .push(format!("prediction {number}: nothing inside the image"));
                continue;
            };
            let confidence = prediction
                .confidence
                .filter(|value| value.is_finite() && (0.0..=1.0).contains(value));
            let mut annotation = AnnotationRecord::new_model(
                request.asset.id,
                label_id,
                geometry,
                provenance.clone(),
                confidence,
            );
            annotation.metadata = prediction.metadata;
            resolved.annotations.push(annotation);
        }
        resolved
    }
}

fn resolve_label(prediction: &PredictedAnnotation, labels: &[Label]) -> Option<LabelId> {
    if let Some(id) = prediction.label_id {
        return labels.iter().any(|label| label.id == id).then_some(id);
    }
    let name = prediction.label_name.as_deref()?.trim();
    labels
        .iter()
        .find(|label| label.name.trim().eq_ignore_ascii_case(name))
        .map(|label| label.id)
}

/// `geometry` clipped to the image, or `None` when nothing valid remains.
fn clip_to_image(
    geometry: AnnotationGeometry,
    dimensions: ImageDimensions,
) -> Option<AnnotationGeometry> {
    let width = f64::from(dimensions.width);
    let height = f64::from(dimensions.height);
    let clipped = match geometry {
        AnnotationGeometry::Bbox(bbox) => {
            let min_x = bbox.x.max(0.0);
            let min_y = bbox.y.max(0.0);
            let max_x = (bbox.x + bbox.width).min(width);
            let max_y = (bbox.y + bbox.height).min(height);
            AnnotationGeometry::Bbox(
                BoundingBox::from_xywh(min_x, min_y, max_x - min_x, max_y - min_y).ok()?,
            )
        }
        AnnotationGeometry::Polygon(polygon) => {
            let points = polygon
                .points
                .into_iter()
                .map(|point| Point::new(point.x.clamp(0.0, width), point.y.clamp(0.0, height)))
                .collect::<Result<Vec<_>, _>>()
                .ok()?;
            AnnotationGeometry::Polygon(Polygon::new(points).ok()?)
        }
    };
    clipped.validate_within_image(dimensions).ok()?;
    Some(clipped)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalModelSpec {
    pub program: PathBuf,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

#[tracing::instrument(level = "debug", skip_all, fields(request_id = %request.request_id.0))]
pub fn encode_request(request: &InferenceRequest) -> Result<String, InferenceProtocolError> {
    tracing::debug!(request_id = %request.request_id.0, "encoding inference request");
    encode_line(request)
}

#[tracing::instrument(level = "debug", skip_all)]
pub fn decode_request(line: &str) -> Result<InferenceRequest, InferenceProtocolError> {
    let request: InferenceRequest = decode_line(line)?;
    tracing::debug!(request_id = %request.request_id.0, "decoded inference request");
    Ok(request)
}

#[tracing::instrument(level = "debug", skip_all, fields(request_id = %response.request_id.0))]
pub fn encode_response(response: &InferenceResponse) -> Result<String, InferenceProtocolError> {
    tracing::debug!(request_id = %response.request_id.0, predictions = response.predictions.len(), "encoding inference response");
    encode_line(response)
}

#[tracing::instrument(level = "debug", skip_all)]
pub fn decode_response(line: &str) -> Result<InferenceResponse, InferenceProtocolError> {
    let response: InferenceResponse = decode_line(line)?;
    tracing::debug!(request_id = %response.request_id.0, predictions = response.predictions.len(), "decoded inference response");
    Ok(response)
}

fn encode_line<T>(value: &T) -> Result<String, InferenceProtocolError>
where
    T: Serialize,
{
    let mut line = serde_json::to_string(value)?;
    line.push('\n');
    Ok(line)
}

fn decode_line<T>(line: &str) -> Result<T, InferenceProtocolError>
where
    T: serde::de::DeserializeOwned,
{
    Ok(serde_json::from_str(line.trim_end())?)
}

#[derive(Debug, Error)]
pub enum InferenceProtocolError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use notatus_core::{AnnotationGeometry, AssetLocation, AssetRecord, BoundingBox};

    #[test]
    fn request_roundtrips_as_json_line() {
        let asset = AssetRecord::new_image(AssetLocation::local("images/a.jpg"), 100, 100).unwrap();
        let request = InferenceRequest::object_detection(asset, vec![Label::new("person")], None);

        let encoded = encode_request(&request).unwrap();
        assert!(encoded.ends_with('\n'));

        let decoded = decode_request(&encoded).unwrap();
        assert_eq!(decoded.task, InferenceTask::ObjectDetection);
        assert_eq!(decoded.labels[0].name, "person");
    }

    fn request_with_labels(names: &[&str]) -> InferenceRequest {
        let asset = AssetRecord::new_image(AssetLocation::local("a.jpg"), 100, 50).unwrap();
        let labels = names.iter().map(|name| Label::new(*name)).collect();
        InferenceRequest::object_detection(asset, labels, None)
    }

    fn response(
        request: &InferenceRequest,
        predictions: Vec<PredictedAnnotation>,
    ) -> InferenceResponse {
        InferenceResponse {
            request_id: request.request_id.clone(),
            model: ModelInfo {
                name: "demo-detector".to_string(),
                version: Some("1".to_string()),
                metadata: Metadata::new(),
            },
            predictions,
            diagnostics: vec!["loaded on cpu".to_string()],
        }
    }

    fn boxed(x: f64, y: f64, w: f64, h: f64) -> AnnotationGeometry {
        AnnotationGeometry::Bbox(BoundingBox {
            x,
            y,
            width: w,
            height: h,
        })
    }

    fn prediction(
        label_id: Option<LabelId>,
        label_name: Option<&str>,
        geometry: AnnotationGeometry,
        confidence: Option<f32>,
    ) -> PredictedAnnotation {
        PredictedAnnotation {
            label_id,
            label_name: label_name.map(str::to_string),
            geometry,
            confidence,
            metadata: Metadata::new(),
        }
    }

    #[test]
    fn predictions_become_draft_model_annotations() {
        let request = request_with_labels(&["person", "Car"]);
        let person = request.labels[0].id;
        let car = request.labels[1].id;
        let predictions = vec![
            prediction(Some(person), None, boxed(1.0, 2.0, 10.0, 12.0), Some(0.91)),
            // Class names match labels ignoring case.
            prediction(None, Some("car"), boxed(20.0, 5.0, 10.0, 10.0), Some(0.5)),
        ];

        let resolved = response(&request, predictions).into_annotations(&request);

        assert_eq!(resolved.annotations.len(), 2);
        assert_eq!(resolved.annotations[0].label_id, person);
        assert_eq!(resolved.annotations[1].label_id, car);
        assert!(resolved.annotations.iter().all(|annotation| {
            annotation.asset_id == request.asset.id
                && annotation.review_state == notatus_core::ReviewState::Draft
                && matches!(annotation.source, notatus_core::AnnotationSource::Model(_))
        }));
        assert_eq!(resolved.notes, ["loaded on cpu"]);
    }

    #[test]
    fn predictions_are_clipped_or_skipped_never_invalid() {
        let request = request_with_labels(&["person"]);
        let predictions = vec![
            // Spills past the right and bottom edges: clipped.
            prediction(
                None,
                Some("person"),
                boxed(90.0, 40.0, 20.0, 20.0),
                Some(1.7),
            ),
            // Entirely outside: skipped.
            prediction(None, Some("person"), boxed(200.0, 0.0, 5.0, 5.0), None),
            // A class the project has no label for: skipped.
            prediction(None, Some("bicycle"), boxed(1.0, 1.0, 5.0, 5.0), None),
            // An ID that was not in the request: skipped.
            prediction(Some(LabelId::new()), None, boxed(1.0, 1.0, 5.0, 5.0), None),
        ];

        let resolved = response(&request, predictions).into_annotations(&request);

        assert_eq!(resolved.annotations.len(), 1);
        let clipped = resolved.annotations[0].geometry.as_bbox().unwrap();
        assert_eq!((clipped.max_x(), clipped.max_y()), (100.0, 50.0));
        assert_eq!(
            resolved.annotations[0].confidence, None,
            "out-of-range dropped"
        );
        assert_eq!(resolved.notes.len(), 4);
        assert!(resolved.notes[1].contains("nothing inside"));
        assert!(resolved.notes[2].contains("bicycle"));
    }

    #[test]
    fn responses_may_name_labels_without_ids() {
        let line = r#"{"request_id":"r1","model":{"name":"yolo"},"predictions":[{"label_name":"person","geometry":{"kind":"bbox","x":1,"y":2,"width":3,"height":4},"confidence":0.8}]}"#;

        let decoded = decode_response(line).unwrap();

        assert_eq!(decoded.predictions[0].label_id, None);
        assert_eq!(decoded.predictions[0].label_name.as_deref(), Some("person"));
    }
}
