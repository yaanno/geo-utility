use geo::Rect;
use geo::{LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon};
use geojson::{Bbox, feature::Id};
use geojson::{Feature, FeatureCollection, Geometry, Value};
use rstar::{AABB, RTreeObject};
use serde_json::Map;
use serde_json::Value as JsonValue;
use std::ops::Deref;

/// Struct to hold a rectangle
#[derive(Debug, Clone, PartialEq)]
pub struct Rectangle(Rect<f64>);

/// Methods for Rectangle
impl Rectangle {
    /// Construct a new Rectangle from a geo::Rect.
    pub fn new(rect: Rect<f64>) -> Self {
        Self(rect)
    }

    /// Convenience constructor from corner coordinates.
    pub fn from_corners(min: (f64, f64), max: (f64, f64)) -> Self {
        Self(Rect::new(min, max))
    }

    /// Convert to geo::Rect<f64>
    pub fn to_geo_rect(&self) -> Rect<f64> {
        self.0
    }
}

/// Conversion from geo::Rect<f64> to Rectangle.
impl From<Rect<f64>> for Rectangle {
    fn from(rect: Rect<f64>) -> Self {
        Rectangle(rect)
    }
}

/// Conversion from Rectangle to geo::Rect<f64>.
impl From<Rectangle> for Rect<f64> {
    fn from(my_rect: Rectangle) -> Self {
        my_rect.0
    }
}

/// Allowing access to the inner Rect methods directly.
impl Deref for Rectangle {
    type Target = Rect<f64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Allow the Rectangle to be used as an RTreeObject
impl RTreeObject for Rectangle {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        let min = self.0.min();
        let max = self.0.max();
        AABB::from_corners([min.x, min.y], [max.x, max.y])
    }
}

/// Struct to hold a rectangle with an associated index
pub struct RectangleWithId(pub Rectangle, pub usize);

/// Allow the RectangleWithId to be used as an RTreeObject
impl RTreeObject for RectangleWithId {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        let min = self.0.min();
        let max = self.0.max();
        AABB::from_corners([min.x, min.y], [max.x, max.y])
    }
}

/// Enum to hold different types of geo geometry
#[derive(Debug, Clone)]
pub enum GeoGeometry {
    Point(Point<f64>),
    LineString(LineString<f64>),
    Polygon(Polygon<f64>),
    MultiPoint(MultiPoint<f64>),
    MultiLineString(MultiLineString<f64>),
    MultiPolygon(MultiPolygon<f64>),
    // GeometryCollection(GeometryCollection<f64>),
}

/// Struct to hold a feature's data with geo geometry
#[derive(Debug, Clone)]
pub struct GeoFeature {
    pub id: Option<Id>,
    pub properties: Option<Map<String, JsonValue>>,
    pub bbox: Option<Bbox>,
    pub foreign_members: Option<Map<String, JsonValue>>,
    pub geometry: Option<GeoGeometry>,
}

/// Struct to hold the collection of intermediate features
#[derive(Debug, Clone)]
pub struct GeoFeatureCollection {
    pub bbox: Option<Bbox>,
    pub foreign_members: Option<Map<String, JsonValue>>,
    pub features: Vec<GeoFeature>,
}

/// Convert a geojson Geometry to a GeoGeometry
impl From<Geometry> for GeoGeometry {
    fn from(value: Geometry) -> Self {
        match &value.value {
            Value::Point(coords) => {
                let point = Point::new(coords[0], coords[1]);
                GeoGeometry::Point(point)
            }
            Value::LineString(coords) => {
                let line_string = LineString::from(
                    coords
                        .iter()
                        .map(|p| Point::new(p[0], p[1]))
                        .collect::<Vec<Point<f64>>>(),
                );
                GeoGeometry::LineString(line_string)
            }
            Value::Polygon(coords) => {
                let exterior = coords.first();
                let exterior_ring = if let Some(exterior) = exterior {
                    LineString::from(
                        exterior
                            .iter()
                            .map(|p| Point::new(p[0], p[1]))
                            .collect::<Vec<Point<f64>>>(),
                    )
                } else {
                    LineString::new(vec![])
                };
                let interior_rings = if coords.len() > 1 {
                    let interiors = coords.iter().skip(1).collect::<Vec<_>>();
                    interiors
                        .iter()
                        .map(|ls| {
                            ls.iter()
                            .map(|p| Point::new(p[0], p[1]))
                            .collect::<LineString>()
                        })
                        .collect::<Vec<_>>()
                } else {
                    vec![]
                };
                let polygon = Polygon::new(exterior_ring, interior_rings);
                GeoGeometry::Polygon(polygon)
            }
            Value::MultiPoint(coords) => {
                let multi_point = MultiPoint::from(
                    coords
                        .iter()
                        .map(|p| Point::new(p[0], p[1]))
                        .collect::<Vec<Point<f64>>>(),
                );
                GeoGeometry::MultiPoint(multi_point)
            }
            Value::MultiLineString(coords) => {
                let multi_line_string = MultiLineString(
                    coords
                        .iter()
                        .map(|ls| {
                            LineString::from(
                                ls.iter()
                                    .map(|p| Point::new(p[0], p[1]))
                                    .collect::<Vec<Point<f64>>>(),
                            )
                        })
                        .collect::<Vec<LineString<f64>>>(),
                );
                GeoGeometry::MultiLineString(multi_line_string)
            }
            Value::MultiPolygon(coords) => {
                let mut polygons = vec![];
                for coord in coords {
                    let exterior = coord.first();
                    let exterior_ring = if let Some(exterior) = exterior {
                        LineString::from(
                            exterior
                                .iter()
                                .map(|p| Point::new(p[0], p[1]))
                                .collect::<Vec<Point<f64>>>(),
                        )
                    } else {
                        LineString::new(vec![])
                    };
                    let interior_rings = if coord.len() > 1 {
                        let interiors = coord.iter().skip(1).collect::<Vec<_>>();
                        interiors
                            .iter()
                            .map(|ls| {
                                ls.iter()
                                    .map(|p| Point::new(p[0], p[1]))
                                    .collect::<LineString>()
                            })
                            .collect::<Vec<_>>()
                    } else {
                        vec![]
                    };
                    let polygon = Polygon::new(exterior_ring, interior_rings);
                    polygons.push(polygon);
                }
                let multi_polygon = MultiPolygon(polygons);
                GeoGeometry::MultiPolygon(multi_polygon)
            }
            _ => unimplemented!(),
        }
    }
}

/// Convert a GeoGeometry to a geojson Geometry
impl From<GeoGeometry> for Geometry {
    fn from(value: GeoGeometry) -> Self {
        match value {
            GeoGeometry::Point(point) => {
                let coords: Vec<f64> = vec![point.x(), point.y()];
                Geometry::new(Value::Point(coords))
            }
            GeoGeometry::LineString(line_string) => {
                let coords: Vec<Vec<f64>> = line_string.coords().map(|p| vec![p.x, p.y]).collect();
                Geometry::new(Value::LineString(coords))
            }
            GeoGeometry::Polygon(polygon) => {
                let mut rings = vec![
                    polygon
                        .exterior()
                        .coords()
                        .map(|p| vec![p.x, p.y])
                        .collect(),
                ];
                let interiors_coords: Vec<Vec<Vec<f64>>> = polygon
                    .interiors()
                    .iter()
                    .map(|ring| ring.coords().map(|p| vec![p.x, p.y]).collect())
                    .collect();
                rings.extend_from_slice(&interiors_coords);
                Geometry::new(Value::Polygon(rings))
            }
            GeoGeometry::MultiPoint(multi_point) => {
                let coords: Vec<Vec<f64>> =
                    multi_point.iter().map(|p| vec![p.x(), p.y()]).collect();
                Geometry::new(Value::MultiPoint(coords))
            }
            GeoGeometry::MultiLineString(multi_line_string) => {
                let coords: Vec<Vec<Vec<f64>>> = multi_line_string
                    .iter()
                    .map(|ls| ls.coords().map(|p| vec![p.x, p.y]).collect())
                    .collect();
                Geometry::new(Value::MultiLineString(coords))
            }
            GeoGeometry::MultiPolygon(multi_polygon) => {
                let coords: Vec<Vec<Vec<Vec<f64>>>> = multi_polygon
                    .iter()
                    .map(|p| {
                        let mut rings =
                            vec![p.exterior().coords().map(|p| vec![p.x, p.y]).collect()];
                        let interiors_coords: Vec<Vec<Vec<f64>>> = p
                            .interiors()
                            .iter()
                            .map(|ring| ring.coords().map(|p| vec![p.x, p.y]).collect())
                            .collect();
                        rings.extend_from_slice(&interiors_coords);
                        rings
                    })
                    .collect();
                Geometry::new(Value::MultiPolygon(coords))
            }
        }
    }
}

/// Convert a GeoFeature to a geojson Feature
impl From<GeoFeature> for Feature {
    fn from(geo_feature: GeoFeature) -> Feature {
        Feature {
            id: geo_feature.id,
            properties: geo_feature.properties,
            bbox: geo_feature.bbox,
            foreign_members: geo_feature.foreign_members,
            geometry: geo_feature.geometry.map(|g| g.into()),
        }
    }
}

/// Convert a geojson Feature to a GeoFeature
impl From<Feature> for GeoFeature {
    fn from(feature: Feature) -> GeoFeature {
        GeoFeature {
            id: feature.id,
            properties: feature.properties,
            bbox: feature.bbox,
            foreign_members: feature.foreign_members,
            geometry: feature.geometry.map(|g| g.into()),
        }
    }
}

/// Convert a geojson FeatureCollection to a GeoFeatureCollection
impl From<FeatureCollection> for GeoFeatureCollection {
    fn from(feature_collection: FeatureCollection) -> GeoFeatureCollection {
        GeoFeatureCollection {
            bbox: feature_collection.bbox,
            foreign_members: feature_collection.foreign_members,
            features: feature_collection.features.into_iter().map(Into::into).collect(),
        }
    }
}

/// Convert a GeoFeatureCollection to a geojson FeatureCollection
impl From<GeoFeatureCollection> for FeatureCollection {
    fn from(geo_feature_collection: GeoFeatureCollection) -> FeatureCollection {
        FeatureCollection {
            bbox: geo_feature_collection.bbox,
            foreign_members: geo_feature_collection.foreign_members,
            features: geo_feature_collection.features.into_iter().map(Into::into).collect(),
        }
    }
}
