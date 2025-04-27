use geo::Rect;
use rstar::{AABB, RTreeObject};
use std::ops::Deref;

#[derive(Debug, Clone, PartialEq)]
pub struct Rectangle(Rect<f64>);

impl Rectangle {
    /// Construct a new Rectangle from a geo::Rect.
    pub fn new(rect: Rect<f64>) -> Self {
        Self(rect)
    }

    /// Convenience constructor from corner coordinates.
    pub fn from_corners(min: (f64, f64), max: (f64, f64)) -> Self {
        Self(Rect::new(min, max))
    }
}

// Conversion from geo::Rect<f64> to Rectangle.
impl From<Rect<f64>> for Rectangle {
    fn from(rect: Rect<f64>) -> Self {
        Rectangle(rect)
    }
}

// Conversion from Rectangle to geo::Rect<f64>.
impl From<Rectangle> for Rect<f64> {
    fn from(my_rect: Rectangle) -> Self {
        my_rect.0
    }
}

// Allowing access to the inner Rect methods directly.
impl Deref for Rectangle {
    type Target = Rect<f64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl RTreeObject for Rectangle {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        let min = self.0.min();
        let max = self.0.max();
        AABB::from_corners([min.x, min.y], [max.x, max.y])
    }
}

pub struct RectangleWithId(pub Rectangle, pub usize);

impl RTreeObject for RectangleWithId {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        let min = self.0.min();
        let max = self.0.max();
        AABB::from_corners([min.x, min.y], [max.x, max.y])
    }
}
