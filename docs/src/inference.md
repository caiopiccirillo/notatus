# Pre-Annotation

A model suggests annotations; a person reviews them. The model runs in its own
process, so the app needs no Python, ONNX Runtime, TensorRT, or model code:
any program that speaks the JSON-lines protocol in `crates/notatus-infer` can
be a runner.

## Using It in the App

1. Choose the runner with **Model → Choose model runner…** or the command
   palette. A Python script (`.py`) runs through `python3`; anything else runs
   directly.
2. Press **Ctrl+R** (or the sparkle button in the canvas toolbar) to run the
   model on the current image, or **Ctrl+Shift+R** to run it on every image not
   marked done. The status bar shows a progress bar and how many suggestions
   have arrived; its stop button, or **Model → Stop**, stops after the current
   image.
3. Suggestions appear as drafts whose tags show the model's confidence
   ("Car 87%"). Review them with **Y** to accept, **X** to reject, or edit them,
   and press **N** to go to the next image with suggestions; the review filter
   can narrow that to suggestions the model was unsure of. Exports leave out
   suggestions nobody has reviewed.

The runner process stays alive between runs, so the model loads once. Running
the model again on an image replaces its earlier suggestions that nobody has
touched; suggestions a person edited or reviewed stay. Each image's
suggestions are one undo step.

The model's labels are matched to the project's labels by name, ignoring case,
so create labels named like the model's classes. Predictions for classes
without a label are skipped and counted in the status message.

### Where the Runner Choice Is Kept

A runner is a program the app executes, so the choice is never stored in the
project: a project downloaded or shared by someone else could otherwise name
any command. It is kept per project ID in the user's own settings,
`notatus/model-runners.json` in the config folder (`$XDG_CONFIG_HOME` or
`~/.config` on Linux, `~/Library/Application Support` on macOS, `%APPDATA%` on
Windows). A project starts with no runner until its user chooses one.

## Example Runners

- `examples/runners/ultralytics_runner.py` serves Ultralytics YOLO detection
  models (`pip install ultralytics`). `NOTATUS_YOLO_MODEL` selects the weights
  (default `yolo11n.pt`), `NOTATUS_YOLO_CONF` the minimum confidence (default
  0.25), and `NOTATUS_YOLO_DEVICE` the device.
- `notatus-fake-runner` (`cargo build -p notatus-infer --bin
  notatus-fake-runner`, then choose `target/debug/notatus-fake-runner`)
  suggests one centered box per image with the first label, for trying the
  workflow without a model.

## Protocol

The app writes one request per line to the runner's stdin and reads one
response per line from its stdout. Anything else the runner prints must go to
stderr; the app shows the last stderr lines when a runner fails.

A request (`InferenceRequest`) carries:

- `request_id`, which the response must repeat
- `task`: `object_detection`
- `asset`: the media record, including its size after EXIF orientation
- `resolved_local_path`: the image file to read
- `labels`: the project's labels, with IDs and names
- `metadata`

A response (`InferenceResponse`) carries:

- `request_id`
- `model`: `name`, optional `version`, `metadata`
- `predictions`, each with a label, `geometry` (a `bbox` in pixels, or a
  `polygon`), an optional `confidence` from 0 to 1, and `metadata`
- `diagnostics`: messages for the user

A prediction names its label with `label_id`, one of the request's label IDs,
or with `label_name`, matched against the request's label names ignoring case.

```json
{"request_id":"…","model":{"name":"yolo","version":"yolo11n.pt"},"predictions":[{"label_name":"person","geometry":{"kind":"bbox","x":10,"y":20,"width":40,"height":60},"confidence":0.9}],"diagnostics":[]}
```

## Turning Predictions into Annotations

`InferenceResponse::into_annotations()` makes draft annotations with model
provenance and returns them with notes on what was skipped. Models often place
boxes a little past the image edge, so boxes and polygon points are clipped to
the image; predictions with an unknown label or nothing inside the image are
skipped, and confidences outside 0 to 1 are dropped. An invalid prediction
therefore never reaches the dataset, where it would make every save fail.

## Runner Process

`ModelRunner` starts the program an `ExternalModelSpec` names (program,
arguments, environment) and keeps it running. Each `infer` call writes a
request and waits for the matching response, by default for up to two minutes
since the first request usually includes loading the model. It reports a
runner that exits, prints something that is not a response, answers a
different request, or does not answer in time, and kills the process when
dropped.
