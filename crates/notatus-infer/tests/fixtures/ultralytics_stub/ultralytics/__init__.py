"""A stand-in for the parts of Ultralytics the example runner uses, so the
runner can be tested without installing it or downloading a model."""

print("Ultralytics stub banner on stdout")


class _Values(list):
    def tolist(self):
        return list(self)


class _Boxes:
    def __init__(self, rows):
        self.xyxy = _Values(row[0] for row in rows)
        self.cls = _Values(row[1] for row in rows)
        self.conf = _Values(row[2] for row in rows)


class _Result:
    names = {0: "person", 1: "bicycle"}

    def __init__(self, rows):
        self.boxes = _Boxes(rows)


class YOLO:
    def __init__(self, weights):
        self.weights = weights

    def predict(self, path, conf, device, verbose):
        print("predicting on stdout", path)
        if path.endswith("broken.jpg"):
            raise OSError("cannot read image")
        return [_Result([([10.0, 20.0, 50.0, 80.0], 0, 0.9), ([0.0, 0.0, 5.0, 5.0], 1, 0.4)])]
