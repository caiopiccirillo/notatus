# Pre-Annotation Protocol

The pre-annotation protocol lives in `crates/notatus-infer`.

It is intentionally an external-process boundary. The desktop app does not need
to embed ONNX Runtime, TensorRT, Python, or model-specific dependencies to use
pre-annotation.

## Request

`InferenceRequest` contains:

- request ID
- task
- asset record
- optional resolved local path
- labels
- metadata

Implemented task:

```rust
InferenceTask::ObjectDetection
```

## Response

`InferenceResponse` contains:

- request ID
- model info
- predictions
- diagnostics

Each prediction contains:

- label ID
- geometry
- optional confidence
- metadata

## Model Info

`ModelInfo` stores:

- model name
- optional version
- metadata

When predictions become annotations, model info is converted into
`ModelProvenance`.

## JSON-Lines Encoding

The protocol encodes one request or response per line:

```rust
encode_request(&request)?;
decode_request(line)?;
encode_response(&response)?;
decode_response(line)?;
```

This makes the protocol easy to implement from Python, Rust, containers, and
remote shims.

## Prediction Conversion

`InferenceResponse::into_annotations()` converts predictions into draft model
annotations for the requested asset.

The resulting annotations:

- reference the original asset
- preserve label IDs
- preserve geometry
- preserve confidence
- store model provenance
- use `ReviewState::Draft`

This keeps model output separate from accepted ground truth until a human or
review process accepts it.

## External Runner Specification

`ExternalModelSpec` describes a future model runner process:

- program path
- arguments
- environment variables

The spec exists, but the desktop process manager that launches runners is not
implemented yet.
