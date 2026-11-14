//! The runner process manager, driven against the fake runner binary.

use notatus_core::{AssetLocation, AssetRecord, Label};
use notatus_infer::{ExternalModelSpec, InferenceRequest, ModelRunner, RunnerError};
use std::collections::BTreeMap;
use std::time::Duration;

fn fake_runner(mode: &str) -> ExternalModelSpec {
    ExternalModelSpec {
        program: env!("CARGO_BIN_EXE_notatus-fake-runner").into(),
        args: Vec::new(),
        env: BTreeMap::from([("NOTATUS_FAKE_RUNNER_MODE".to_string(), mode.to_string())]),
    }
}

fn request() -> InferenceRequest {
    let asset = AssetRecord::new_image(AssetLocation::local("/tmp/a.jpg"), 200, 100).unwrap();
    InferenceRequest::object_detection(asset, vec![Label::new("person")], None)
}

#[test]
fn one_runner_answers_many_requests() {
    let mut runner = ModelRunner::spawn(&fake_runner("")).unwrap();

    for _ in 0..3 {
        let request = request();
        let response = runner.infer(&request).unwrap();
        assert_eq!(response.request_id, request.request_id);

        let resolved = response.into_annotations(&request);
        assert_eq!(resolved.annotations.len(), 1);
        let bbox = resolved.annotations[0].geometry.as_bbox().unwrap();
        assert_eq!(
            (bbox.x, bbox.y, bbox.width, bbox.height),
            (50.0, 25.0, 100.0, 50.0)
        );
    }
}

#[test]
fn a_crashing_runner_reports_its_exit_and_stderr() {
    let mut runner = ModelRunner::spawn(&fake_runner("crash")).unwrap();

    let error = runner.infer(&request()).unwrap_err();

    let message = error.to_string();
    assert!(
        matches!(
            error,
            RunnerError::Exited { .. } | RunnerError::Closed { .. }
        ),
        "{message}"
    );
    assert!(message.contains("model weights not found"), "{message}");
}

#[test]
fn a_line_that_is_not_a_response_is_an_error() {
    let mut runner = ModelRunner::spawn(&fake_runner("garbage")).unwrap();

    let error = runner.infer(&request()).unwrap_err();

    assert!(matches!(error, RunnerError::Response { .. }), "{error}");
    assert!(error.to_string().contains("loading model..."));
}

#[test]
fn a_silent_runner_times_out() {
    let mut runner = ModelRunner::spawn(&fake_runner("silent"))
        .unwrap()
        .with_timeout(Duration::from_millis(200));

    let error = runner.infer(&request()).unwrap_err();

    assert!(matches!(error, RunnerError::Timeout { .. }), "{error}");
}

#[test]
fn a_missing_program_cannot_start() {
    let spec = ExternalModelSpec {
        program: "/nonexistent/model-runner".into(),
        args: Vec::new(),
        env: BTreeMap::new(),
    };

    let error = ModelRunner::spawn(&spec).err().unwrap();

    assert!(error.to_string().contains("/nonexistent/model-runner"));
}
