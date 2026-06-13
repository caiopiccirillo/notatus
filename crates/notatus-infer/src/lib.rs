//! External pre-annotation protocol.
//!
//! V1 keeps inference out of the desktop binary. A model runner can be Python,
//! ONNX Runtime, TensorRT, a container, or a remote shim as long as it speaks
//! this JSON-lines protocol.

use notatus_core::{
    AnnotationGeometry, AnnotationRecord, AssetRecord, Label, Metadata, ModelProvenance,
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

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PredictedAnnotation {
    pub label_id: notatus_core::LabelId,
    pub geometry: AnnotationGeometry,
    pub confidence: Option<f32>,
    #[serde(default)]
    pub metadata: Metadata,
}

impl PredictedAnnotation {
    pub fn into_annotation(
        self,
        request: &InferenceRequest,
        model: &ModelInfo,
    ) -> AnnotationRecord {
        let mut annotation = AnnotationRecord::new_model(
            request.asset.id,
            self.label_id,
            self.geometry,
            model.provenance(Some(request.request_id.as_str().to_string())),
            self.confidence,
        );
        annotation.metadata = self.metadata;
        annotation
    }
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

impl InferenceResponse {
    pub fn into_annotations(self, request: &InferenceRequest) -> Vec<AnnotationRecord> {
        self.predictions
            .into_iter()
            .map(|prediction| prediction.into_annotation(request, &self.model))
            .collect()
    }
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

    #[test]
    fn predictions_become_draft_model_annotations() {
        let asset = AssetRecord::new_image(AssetLocation::local("images/a.jpg"), 100, 100).unwrap();
        let label = Label::new("person");
        let request = InferenceRequest::object_detection(asset.clone(), vec![label.clone()], None);
        let response = InferenceResponse {
            request_id: request.request_id.clone(),
            model: ModelInfo {
                name: "demo-detector".to_string(),
                version: Some("1".to_string()),
                metadata: Metadata::new(),
            },
            predictions: vec![PredictedAnnotation {
                label_id: label.id,
                geometry: AnnotationGeometry::Bbox(
                    BoundingBox::from_xywh(1.0, 2.0, 10.0, 12.0).unwrap(),
                ),
                confidence: Some(0.91),
                metadata: Metadata::new(),
            }],
            diagnostics: Vec::new(),
        };

        let annotations = response.into_annotations(&request);

        assert_eq!(annotations.len(), 1);
        assert_eq!(annotations[0].asset_id, asset.id);
        assert_eq!(annotations[0].confidence, Some(0.91));
    }
}
