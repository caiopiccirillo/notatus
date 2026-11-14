#!/usr/bin/env python3
"""Notatus model runner for Ultralytics YOLO detection models.

In Notatus, use "Choose model runner…" from the command palette and pick
this file; it runs through python3. Then Ctrl+R suggests boxes on the
current image and Ctrl+Shift+R on every unfinished one.

The model's class names are matched to the project's labels, ignoring case,
so create labels named like the model's classes ("person", "car", …).
Predictions for classes without a label are skipped.

Settings, read from environment variables when the runner starts:

    NOTATUS_YOLO_MODEL   weights file or model name (default: yolo11n.pt)
    NOTATUS_YOLO_CONF    minimum confidence, 0 to 1 (default: 0.25)
    NOTATUS_YOLO_DEVICE  "cpu", "0" for the first GPU, … (default: automatic)

Requires: pip install ultralytics

Protocol: one JSON request per line on stdin, one JSON response per line on
stdout. Anything else must go to stderr, so all other output is redirected
there.
"""

import json
import os
import sys


def main() -> None:
    # Ultralytics and its dependencies may print to stdout, which would
    # corrupt the protocol. Keep the real stdout for responses only.
    responses = sys.stdout
    sys.stdout = sys.stderr

    from ultralytics import YOLO

    weights = os.environ.get("NOTATUS_YOLO_MODEL", "yolo11n.pt")
    confidence = float(os.environ.get("NOTATUS_YOLO_CONF", "0.25"))
    device = os.environ.get("NOTATUS_YOLO_DEVICE") or None
    model = YOLO(weights)
    print(f"notatus runner: loaded {weights}", file=sys.stderr)

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        request = json.loads(line)
        response = {
            "request_id": request["request_id"],
            "model": {"name": "ultralytics", "version": weights},
            "predictions": [],
            "diagnostics": [],
        }
        try:
            response["predictions"] = predict(model, request, confidence, device)
        except Exception as error:  # Report and keep serving later images.
            response["diagnostics"].append(f"prediction failed: {error}")
        responses.write(json.dumps(response) + "\n")
        responses.flush()


def predict(model, request, confidence, device):
    path = request.get("resolved_local_path")
    if not path:
        raise ValueError("the media has no local file")
    predictions = []
    for result in model.predict(path, conf=confidence, device=device, verbose=False):
        names = result.names
        boxes = result.boxes
        for xyxy, cls, conf in zip(boxes.xyxy.tolist(), boxes.cls.tolist(), boxes.conf.tolist()):
            x1, y1, x2, y2 = xyxy
            predictions.append(
                {
                    "label_name": names[int(cls)],
                    "geometry": {
                        "kind": "bbox",
                        "x": x1,
                        "y": y1,
                        "width": x2 - x1,
                        "height": y2 - y1,
                    },
                    "confidence": float(conf),
                }
            )
    return predictions


if __name__ == "__main__":
    main()
