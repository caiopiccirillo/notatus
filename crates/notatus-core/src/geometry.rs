use serde::{Deserialize, Serialize};
use thiserror::Error;

const COORD_EPSILON: f64 = 1e-9;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct ImageDimensions {
    pub width: u32,
    pub height: u32,
}

impl ImageDimensions {
    pub fn new(width: u32, height: u32) -> Result<Self, GeometryError> {
        if width == 0 || height == 0 {
            return Err(GeometryError::InvalidImageDimensions { width, height });
        }

        Ok(Self { width, height })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Result<Self, GeometryError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(GeometryError::NonFiniteCoordinate);
        }

        Ok(Self { x, y })
    }

    pub fn validate_within_image(self, dimensions: ImageDimensions) -> Result<(), GeometryError> {
        if self.x < -COORD_EPSILON
            || self.y < -COORD_EPSILON
            || self.x - dimensions.width as f64 > COORD_EPSILON
            || self.y - dimensions.height as f64 > COORD_EPSILON
        {
            return Err(GeometryError::OutsideImage {
                width: dimensions.width,
                height: dimensions.height,
            });
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl BoundingBox {
    pub fn from_xywh(x: f64, y: f64, width: f64, height: f64) -> Result<Self, GeometryError> {
        let bbox = Self {
            x,
            y,
            width,
            height,
        };
        bbox.validate()?;
        Ok(bbox)
    }

    pub fn from_yolo_normalized(
        center_x: f64,
        center_y: f64,
        width: f64,
        height: f64,
        dimensions: ImageDimensions,
    ) -> Result<Self, GeometryError> {
        for value in [center_x, center_y, width, height] {
            if !value.is_finite() {
                return Err(GeometryError::NonFiniteCoordinate);
            }
            if !(0.0..=1.0).contains(&value) {
                return Err(GeometryError::InvalidNormalizedCoordinate { value });
            }
        }

        if width <= 0.0 || height <= 0.0 {
            return Err(GeometryError::NonPositiveExtent { width, height });
        }

        let pixel_width = width * dimensions.width as f64;
        let pixel_height = height * dimensions.height as f64;
        let x = center_x * dimensions.width as f64 - pixel_width / 2.0;
        let y = center_y * dimensions.height as f64 - pixel_height / 2.0;
        let bbox = Self::from_xywh(x, y, pixel_width, pixel_height)?;
        bbox.validate_within_image(dimensions)?;
        Ok(bbox)
    }

    pub fn max_x(self) -> f64 {
        self.x + self.width
    }

    pub fn max_y(self) -> f64 {
        self.y + self.height
    }

    pub fn area(self) -> f64 {
        self.width * self.height
    }

    pub fn to_yolo_normalized(self, dimensions: ImageDimensions) -> [f64; 4] {
        [
            (self.x + self.width / 2.0) / dimensions.width as f64,
            (self.y + self.height / 2.0) / dimensions.height as f64,
            self.width / dimensions.width as f64,
            self.height / dimensions.height as f64,
        ]
    }

    pub fn validate(self) -> Result<(), GeometryError> {
        for value in [self.x, self.y, self.width, self.height] {
            if !value.is_finite() {
                return Err(GeometryError::NonFiniteCoordinate);
            }
        }

        if self.x < 0.0 || self.y < 0.0 {
            return Err(GeometryError::NegativeOrigin {
                x: self.x,
                y: self.y,
            });
        }

        if self.width <= 0.0 || self.height <= 0.0 {
            return Err(GeometryError::NonPositiveExtent {
                width: self.width,
                height: self.height,
            });
        }

        Ok(())
    }

    pub fn validate_within_image(self, dimensions: ImageDimensions) -> Result<(), GeometryError> {
        self.validate()?;

        if self.max_x() - dimensions.width as f64 > COORD_EPSILON
            || self.max_y() - dimensions.height as f64 > COORD_EPSILON
        {
            return Err(GeometryError::OutsideImage {
                width: dimensions.width,
                height: dimensions.height,
            });
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Polygon {
    pub points: Vec<Point>,
}

impl Polygon {
    pub fn new(points: Vec<Point>) -> Result<Self, GeometryError> {
        if points.len() < 3 {
            return Err(GeometryError::PolygonTooSmall {
                points: points.len(),
            });
        }

        Ok(Self { points })
    }

    pub fn validate_within_image(&self, dimensions: ImageDimensions) -> Result<(), GeometryError> {
        if self.points.len() < 3 {
            return Err(GeometryError::PolygonTooSmall {
                points: self.points.len(),
            });
        }

        if self.area() <= COORD_EPSILON {
            return Err(GeometryError::DegeneratePolygon);
        }

        for point in &self.points {
            point.validate_within_image(dimensions)?;
        }

        Ok(())
    }

    pub fn area(&self) -> f64 {
        if self.points.len() < 3 {
            return 0.0;
        }

        let mut sum = 0.0;
        for index in 0..self.points.len() {
            let current = self.points[index];
            let next = self.points[(index + 1) % self.points.len()];
            sum += current.x * next.y - next.x * current.y;
        }
        sum.abs() / 2.0
    }

    pub fn bounding_box(&self) -> Result<BoundingBox, GeometryError> {
        if self.points.len() < 3 {
            return Err(GeometryError::PolygonTooSmall {
                points: self.points.len(),
            });
        }

        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for point in &self.points {
            if !point.x.is_finite() || !point.y.is_finite() {
                return Err(GeometryError::NonFiniteCoordinate);
            }
            min_x = min_x.min(point.x);
            min_y = min_y.min(point.y);
            max_x = max_x.max(point.x);
            max_y = max_y.max(point.y);
        }

        if max_x - min_x <= COORD_EPSILON || max_y - min_y <= COORD_EPSILON {
            return Err(GeometryError::DegeneratePolygon);
        }

        BoundingBox::from_xywh(min_x, min_y, max_x - min_x, max_y - min_y)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AnnotationGeometry {
    Bbox(BoundingBox),
    Polygon(Polygon),
}

impl AnnotationGeometry {
    pub fn validate_within_image(&self, dimensions: ImageDimensions) -> Result<(), GeometryError> {
        match self {
            Self::Bbox(bbox) => bbox.validate_within_image(dimensions),
            Self::Polygon(polygon) => polygon.validate_within_image(dimensions),
        }
    }

    pub fn as_bbox(&self) -> Option<BoundingBox> {
        match self {
            Self::Bbox(bbox) => Some(*bbox),
            Self::Polygon(_) => None,
        }
    }
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum GeometryError {
    #[error("image dimensions must be positive, got {width}x{height}")]
    InvalidImageDimensions { width: u32, height: u32 },
    #[error("coordinates must be finite")]
    NonFiniteCoordinate,
    #[error("coordinates must be normalized to 0..=1, got {value}")]
    InvalidNormalizedCoordinate { value: f64 },
    #[error("origin must be non-negative, got x={x}, y={y}")]
    NegativeOrigin { x: f64, y: f64 },
    #[error("extent must be positive, got width={width}, height={height}")]
    NonPositiveExtent { width: f64, height: f64 },
    #[error("geometry exceeds image bounds {width}x{height}")]
    OutsideImage { width: u32, height: u32 },
    #[error("polygon must have at least 3 points, got {points}")]
    PolygonTooSmall { points: usize },
    #[error("polygon area must be positive")]
    DegeneratePolygon,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_yolo_coordinates_to_pixels() {
        let dimensions = ImageDimensions::new(200, 100).unwrap();
        let bbox = BoundingBox::from_yolo_normalized(0.5, 0.5, 0.25, 0.4, dimensions).unwrap();

        assert_eq!(
            bbox,
            BoundingBox::from_xywh(75.0, 30.0, 50.0, 40.0).unwrap()
        );
        assert_eq!(bbox.to_yolo_normalized(dimensions), [0.5, 0.5, 0.25, 0.4]);
    }

    #[test]
    fn rejects_boxes_outside_image_bounds() {
        let dimensions = ImageDimensions::new(100, 100).unwrap();
        let bbox = BoundingBox::from_xywh(90.0, 10.0, 20.0, 20.0).unwrap();

        assert!(matches!(
            bbox.validate_within_image(dimensions),
            Err(GeometryError::OutsideImage { .. })
        ));
    }

    #[test]
    fn computes_polygon_area_and_bounds() {
        let polygon = Polygon::new(vec![
            Point::new(10.0, 20.0).unwrap(),
            Point::new(50.0, 20.0).unwrap(),
            Point::new(50.0, 60.0).unwrap(),
            Point::new(10.0, 60.0).unwrap(),
        ])
        .unwrap();

        assert_eq!(polygon.area(), 1600.0);
        assert_eq!(
            polygon.bounding_box().unwrap(),
            BoundingBox::from_xywh(10.0, 20.0, 40.0, 40.0).unwrap()
        );
    }

    #[test]
    fn rejects_degenerate_polygons() {
        let dimensions = ImageDimensions::new(100, 100).unwrap();
        let polygon = Polygon::new(vec![
            Point::new(10.0, 10.0).unwrap(),
            Point::new(20.0, 20.0).unwrap(),
            Point::new(30.0, 30.0).unwrap(),
        ])
        .unwrap();

        assert_eq!(
            polygon.validate_within_image(dimensions),
            Err(GeometryError::DegeneratePolygon)
        );
    }
}
