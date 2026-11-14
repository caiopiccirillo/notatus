//! A stand-in model runner, for trying pre-annotation without a model and for
//! tests.
//!
//! For each request it predicts one box covering the middle of the image,
//! labelled with the request's first label. `NOTATUS_FAKE_RUNNER_MODE` makes
//! it misbehave on purpose: `crash` exits on the first request, `garbage`
//! answers with a line that is not JSON, `silent` never answers.

use notatus_core::{AnnotationGeometry, BoundingBox, Metadata};
use notatus_infer::{
    InferenceResponse, ModelInfo, PredictedAnnotation, decode_request, encode_response,
};
use std::io::{BufRead, Write};

fn main() {
    let mode = std::env::var("NOTATUS_FAKE_RUNNER_MODE").unwrap_or_default();
    eprintln!("fake runner ready");
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines().map_while(Result::ok) {
        match mode.as_str() {
            "crash" => {
                eprintln!("model weights not found");
                std::process::exit(3);
            }
            "garbage" => {
                writeln!(stdout, "loading model...").unwrap();
                stdout.flush().unwrap();
                continue;
            }
            "silent" => continue,
            _ => {}
        }

        let request = match decode_request(&line) {
            Ok(request) => request,
            Err(error) => {
                eprintln!("bad request: {error}");
                continue;
            }
        };
        let width = f64::from(request.asset.dimensions.width);
        let height = f64::from(request.asset.dimensions.height);
        let predictions = request
            .labels
            .first()
            .map(|label| PredictedAnnotation {
                label_id: None,
                label_name: Some(label.name.clone()),
                geometry: AnnotationGeometry::Bbox(
                    BoundingBox::from_xywh(width / 4.0, height / 4.0, width / 2.0, height / 2.0)
                        .expect("a box inside the image"),
                ),
                confidence: Some(0.5),
                metadata: Metadata::new(),
            })
            .into_iter()
            .collect();
        let response = InferenceResponse {
            request_id: request.request_id,
            model: ModelInfo {
                name: "notatus-fake-runner".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
                metadata: Metadata::new(),
            },
            predictions,
            diagnostics: Vec::new(),
        };
        write!(stdout, "{}", encode_response(&response).unwrap()).unwrap();
        stdout.flush().unwrap();
    }
}
