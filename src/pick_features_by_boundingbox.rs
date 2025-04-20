// collect features that are within a bounding box or intersect with it

use geo::{Contains, Coord, Intersects, LineString, MultiPoint, Polygon};
use geojson::{Feature, FeatureCollection, Value};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid geometry type")]
    UnsupportedGeometryType,
    #[error("Missing geometry")]
    MissingGeometry,
    #[error("Invalid coordinates")]
    InvalidCoordinates,
}

pub fn pick_features_by_boundingbox(
    featurecollection: &FeatureCollection,
    bbox: Polygon,
) -> Result<Vec<&Feature>, Error> {
    let mut selected_features = Vec::new();

    for feature in &featurecollection.features {
        let geometry_value = match feature.geometry.as_ref() {
            Some(geometry) => &geometry.value,
            None => {
                // Skip features without geometry
                continue;
            }
        };

        match geometry_value {
            Value::Point(coord) => {
                let geo_coord = Coord {
                    x: coord[0],
                    y: coord[1],
                };
                let contains = bbox.contains(&geo_coord);
                let intersects = bbox.intersects(&geo_coord);
                if contains || intersects {
                    selected_features.push(feature);
                }
            }
            Value::LineString(line_coords) => {
                let line: LineString<f64> = line_coords
                    .into_iter()
                    .map(|coord| Coord {
                        x: coord[0],
                        y: coord[1],
                    })
                    .collect();
                if bbox.intersects(&line) || bbox.contains(&line) {
                    selected_features.push(feature);
                }
            }
            Value::Polygon(polygon_coords) => {
                if let Some(exterior_ring) = polygon_coords.first() {
                    let line = exterior_ring
                        .iter()
                        .map(|c| Coord { x: c[0], y: c[1] })
                        .collect();

                    let polygon = Polygon::new(line, vec![]);
                    if bbox.intersects(&polygon) || bbox.contains(&polygon) {
                        selected_features.push(feature);
                    }
                } else {
                    // Polygon has no exterior ring, skip
                    continue;
                }
            }
            Value::MultiPoint(point_coords_vec) => {
                let coords: Vec<Coord<f64>> = point_coords_vec
                    .into_iter()
                    .map(|coord| Coord {
                        x: coord[0],
                        y: coord[1],
                    })
                    .collect();
                let multi_point = MultiPoint::from(coords);
                if bbox.intersects(&multi_point) || bbox.contains(&multi_point) {
                    selected_features.push(feature);
                }
            }
            _ => {
                // Skip unsupported geometry types
                continue;
            }
        };
    }

    Ok(selected_features)
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::polygon;
    use geojson::{Feature, FeatureCollection, Geometry, Value};

    // Helper function to create a GeoJSON Feature with a given geometry
    fn create_feature(geometry: Value) -> Feature {
        Feature {
            bbox: None,
            geometry: Some(Geometry {
                bbox: None,
                foreign_members: None,
                value: geometry,
            }),
            id: None,
            properties: None,
            foreign_members: None,
        }
    }

    // Helper function to create a rectangular bounding box polygon
    fn create_bbox(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Polygon {
        polygon![
            (x: min_x, y: min_y),
            (x: max_x, y: min_y),
            (x: max_x, y: max_y),
            (x: min_x, y: max_y),
            (x: min_x, y: min_y) // Close the ring
        ]
    }

    #[test]
    fn test_empty_feature_collection() {
        let feature_collection = FeatureCollection {
            bbox: None,
            features: vec![],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert!(selected.is_empty());
    }

    #[test]
    fn test_point_inside_bbox() {
        let point_inside = create_feature(Value::Point(vec![5.0, 5.0]));
        let feature_collection = FeatureCollection {
            bbox: None,
            features: vec![point_inside.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert_eq!(selected.len(), 1);
        assert_eq!(
            selected[0].geometry.as_ref().unwrap().value,
            Value::Point(vec![5.0, 5.0])
        );
    }

    #[test]
    fn test_point_on_bbox_boundary() {
        let point_on_boundary = create_feature(Value::Point(vec![0.0, 5.0]));
        let feature_collection = FeatureCollection {
            bbox: None,
            features: vec![point_on_boundary.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert_eq!(selected.len(), 1);
        assert_eq!(
            selected[0].geometry.as_ref().unwrap().value,
            Value::Point(vec![0.0, 5.0])
        );
    }

    #[test]
    fn test_point_outside_bbox() {
        let point_outside = create_feature(Value::Point(vec![11.0, 11.0]));
        let feature_collection = FeatureCollection {
            bbox: None,
            features: vec![point_outside.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert!(selected.is_empty());
    }

    #[test]
    fn test_linestring_inside_bbox() {
        let linestring_inside =
            create_feature(Value::LineString(vec![vec![1.0, 1.0], vec![9.0, 9.0]]));
        let feature_collection = FeatureCollection {
            bbox: None,
            features: vec![linestring_inside.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert_eq!(selected.len(), 1);
        assert_eq!(
            selected[0].geometry.as_ref().unwrap().value,
            Value::LineString(vec![vec![1.0, 1.0], vec![9.0, 9.0]])
        );
    }

    #[test]
    fn test_linestring_intersecting_bbox() {
        let linestring_intersecting =
            create_feature(Value::LineString(vec![vec![-1.0, 5.0], vec![11.0, 5.0]]));
        let feature_collection = FeatureCollection {
            bbox: None,
            features: vec![linestring_intersecting.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert_eq!(selected.len(), 1);
        assert_eq!(
            selected[0].geometry.as_ref().unwrap().value,
            Value::LineString(vec![vec![-1.0, 5.0], vec![11.0, 5.0]])
        );
    }

    #[test]
    fn test_linestring_outside_bbox() {
        let linestring_outside =
            create_feature(Value::LineString(vec![vec![11.0, 1.0], vec![12.0, 2.0]]));
        let feature_collection = FeatureCollection {
            bbox: None,
            features: vec![linestring_outside.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert!(selected.is_empty());
    }

    #[test]
    fn test_polygon_inside_bbox() {
        let polygon_inside = create_feature(Value::Polygon(vec![vec![
            vec![1.0, 1.0],
            vec![3.0, 1.0],
            vec![3.0, 3.0],
            vec![1.0, 3.0],
            vec![1.0, 1.0],
        ]]));
        let feature_collection = FeatureCollection {
            bbox: None,
            features: vec![polygon_inside.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert_eq!(selected.len(), 1);
        assert_eq!(
            selected[0].geometry.as_ref().unwrap().value,
            Value::Polygon(vec![vec![
                vec![1.0, 1.0],
                vec![3.0, 1.0],
                vec![3.0, 3.0],
                vec![1.0, 3.0],
                vec![1.0, 1.0]
            ]])
        );
    }

    #[test]
    fn test_polygon_intersecting_bbox() {
        let polygon_intersecting = create_feature(Value::Polygon(vec![vec![
            vec![-1.0, -1.0],
            vec![1.0, -1.0],
            vec![1.0, 1.0],
            vec![-1.0, 1.0],
            vec![-1.0, -1.0],
        ]]));
        let feature_collection = FeatureCollection {
            bbox: None,
            features: vec![polygon_intersecting.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert_eq!(selected.len(), 1);
        assert_eq!(
            selected[0].geometry.as_ref().unwrap().value,
            Value::Polygon(vec![vec![
                vec![-1.0, -1.0],
                vec![1.0, -1.0],
                vec![1.0, 1.0],
                vec![-1.0, 1.0],
                vec![-1.0, -1.0]
            ]])
        );
    }

    // Note: The current implementation of Polygon handling in the function only considers
    // the exterior ring. This test reflects that limitation.
    #[test]
    fn test_polygon_with_hole_intersecting_bbox_by_exterior() {
        let polygon_with_hole = create_feature(Value::Polygon(vec![
            vec![
                vec![0.0, 0.0],
                vec![10.0, 0.0],
                vec![10.0, 10.0],
                vec![0.0, 10.0],
                vec![0.0, 0.0],
            ], // Exterior
            vec![
                vec![4.0, 4.0],
                vec![6.0, 4.0],
                vec![6.0, 6.0],
                vec![4.0, 6.0],
                vec![4.0, 4.0],
            ], // Interior (hole)
        ]));
        let feature_collection = FeatureCollection {
            bbox: None,
            features: vec![polygon_with_hole.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(1.0, 1.0, 9.0, 9.0); // Bbox is inside the exterior ring but outside the hole

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        // Based on the current function's logic (only checking exterior), this will be selected.
        // If the function were improved to handle holes, the result might differ depending on the
        // exact intersection/containment rules with holes.
        assert_eq!(selected.len(), 1);
        assert_eq!(
            selected[0].geometry.as_ref().unwrap().value,
            Value::Polygon(vec![
                vec![
                    vec![0.0, 0.0],
                    vec![10.0, 0.0],
                    vec![10.0, 10.0],
                    vec![0.0, 10.0],
                    vec![0.0, 0.0]
                ],
                vec![
                    vec![4.0, 4.0],
                    vec![6.0, 4.0],
                    vec![6.0, 6.0],
                    vec![4.0, 6.0],
                    vec![4.0, 4.0]
                ],
            ])
        );
    }

    #[test]
    fn test_polygon_outside_bbox() {
        let polygon_outside = create_feature(Value::Polygon(vec![vec![
            vec![11.0, 11.0],
            vec![12.0, 11.0],
            vec![12.0, 12.0],
            vec![11.0, 12.0],
            vec![11.0, 11.0],
        ]]));
        let feature_collection = FeatureCollection {
            bbox: None,
            features: vec![polygon_outside.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert!(selected.is_empty());
    }

    #[test]
    fn test_multipoint_inside_bbox() {
        let multipoint_inside = create_feature(Value::MultiPoint(vec![
            vec![1.0, 1.0],
            vec![2.0, 2.0],
            vec![3.0, 3.0],
        ]));
        let feature_collection = FeatureCollection {
            bbox: None,
            features: vec![multipoint_inside.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert_eq!(selected.len(), 1);
        assert_eq!(
            selected[0].geometry.as_ref().unwrap().value,
            Value::MultiPoint(vec![vec![1.0, 1.0], vec![2.0, 2.0], vec![3.0, 3.0]])
        );
    }

    #[test]
    fn test_multipoint_intersecting_bbox() {
        let multipoint_intersecting = create_feature(Value::MultiPoint(vec![
            vec![-1.0, -1.0],
            vec![5.0, 5.0],
            vec![11.0, 11.0],
        ]));
        let feature_collection = FeatureCollection {
            bbox: None,
            features: vec![multipoint_intersecting.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert_eq!(selected.len(), 1);
        assert_eq!(
            selected[0].geometry.as_ref().unwrap().value,
            Value::MultiPoint(vec![vec![-1.0, -1.0], vec![5.0, 5.0], vec![11.0, 11.0]])
        );
    }

    #[test]
    fn test_multipoint_outside_bbox() {
        let multipoint_outside =
            create_feature(Value::MultiPoint(vec![vec![11.0, 11.0], vec![12.0, 12.0]]));
        let feature_collection = FeatureCollection {
            bbox: None,
            features: vec![multipoint_outside.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert!(selected.is_empty());
    }

    #[test]
    fn test_feature_without_geometry() {
        let feature_no_geometry = Feature {
            bbox: None,
            geometry: None, // Missing geometry
            id: None,
            properties: None,
            foreign_members: None,
        };
        let feature_collection = FeatureCollection {
            bbox: None,
            features: vec![feature_no_geometry.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        // Features without geometry should be skipped
        assert!(selected.is_empty());
    }

    #[test]
    fn test_feature_with_unsupported_geometry_type() {
        // We'll use a MultiPolygon as an example of an currently unsupported type
        let unsupported_geometry = create_feature(Value::MultiPolygon(vec![vec![vec![
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
            vec![0.0, 0.0],
        ]]]));
        let feature_collection = FeatureCollection {
            bbox: None,
            features: vec![unsupported_geometry.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        // Features with unsupported geometry types should be skipped
        assert!(selected.is_empty());
    }

    #[test]
    fn test_collection_with_mixed_features() {
        let point_inside = create_feature(Value::Point(vec![5.0, 5.0]));
        let linestring_intersecting =
            create_feature(Value::LineString(vec![vec![-1.0, 5.0], vec![11.0, 5.0]]));
        let polygon_outside = create_feature(Value::Polygon(vec![vec![
            vec![11.0, 11.0],
            vec![12.0, 11.0],
            vec![12.0, 12.0],
            vec![11.0, 12.0],
            vec![11.0, 11.0],
        ]]));
        let feature_no_geometry = Feature {
            bbox: None,
            geometry: None,
            id: None,
            properties: None,
            foreign_members: None,
        };
        let unsupported_geometry = create_feature(Value::MultiPolygon(vec![vec![vec![
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
            vec![0.0, 0.0],
        ]]]));

        let feature_collection = FeatureCollection {
            bbox: None,
            features: vec![
                point_inside.clone(),
                linestring_intersecting.clone(),
                polygon_outside.clone(),
                feature_no_geometry.clone(),
                unsupported_geometry.clone(),
            ],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert_eq!(selected.len(), 2);
        // Verify that the correct features were selected
        let selected_values: Vec<Value> = selected
            .iter()
            .filter_map(|f| f.geometry.as_ref().map(|g| g.value.clone()))
            .collect();

        assert!(selected_values.contains(&Value::Point(vec![5.0, 5.0])));
        assert!(
            selected_values.contains(&Value::LineString(vec![vec![-1.0, 5.0], vec![11.0, 5.0]]))
        );
        assert!(!selected_values.contains(&Value::Polygon(vec![vec![
            vec![11.0, 11.0],
            vec![12.0, 11.0],
            vec![12.0, 12.0],
            vec![11.0, 12.0],
            vec![11.0, 11.0]
        ]])));
        assert!(
            !selected_values.contains(&Value::MultiPolygon(vec![vec![vec![
                vec![0.0, 0.0],
                vec![1.0, 0.0],
                vec![1.0, 1.0],
                vec![0.0, 0.0]
            ]],]))
        );
    }

    #[test]
    fn test_polygon_on_bbox_boundary() {
        let polygon_on_boundary = create_feature(Value::Polygon(vec![vec![
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
            vec![0.0, 1.0],
            vec![0.0, 0.0],
        ]]));
        let feature_collection = FeatureCollection {
            bbox: None,
            features: vec![polygon_on_boundary.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert_eq!(selected.len(), 1);
        assert_eq!(
            selected[0].geometry.as_ref().unwrap().value,
            Value::Polygon(vec![vec![
                vec![0.0, 0.0],
                vec![1.0, 0.0],
                vec![1.0, 1.0],
                vec![0.0, 1.0],
                vec![0.0, 0.0]
            ]])
        );
    }

    #[test]
    fn test_linestring_on_bbox_boundary() {
        let linestring_on_boundary =
            create_feature(Value::LineString(vec![vec![0.0, 5.0], vec![10.0, 5.0]]));
        let feature_collection = FeatureCollection {
            bbox: None,
            features: vec![linestring_on_boundary.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert_eq!(selected.len(), 1);
        assert_eq!(
            selected[0].geometry.as_ref().unwrap().value,
            Value::LineString(vec![vec![0.0, 5.0], vec![10.0, 5.0]])
        );
    }

    #[test]
    fn test_multipoint_on_bbox_boundary() {
        let multipoint_on_boundary =
            create_feature(Value::MultiPoint(vec![vec![0.0, 0.0], vec![10.0, 10.0]]));
        let feature_collection = FeatureCollection {
            bbox: None,
            features: vec![multipoint_on_boundary.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert_eq!(selected.len(), 1);
        assert_eq!(
            selected[0].geometry.as_ref().unwrap().value,
            Value::MultiPoint(vec![vec![0.0, 0.0], vec![10.0, 10.0]])
        );
    }

    // Test case for a polygon completely containing the bbox
    #[test]
    fn test_polygon_containing_bbox() {
        let large_polygon = create_feature(Value::Polygon(vec![vec![
            vec![-10.0, -10.0],
            vec![20.0, -10.0],
            vec![20.0, 20.0],
            vec![-10.0, 20.0],
            vec![-10.0, -10.0],
        ]]));
        let feature_collection = FeatureCollection {
            bbox: None,
            features: vec![large_polygon.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert_eq!(selected.len(), 1);
        assert_eq!(
            selected[0].geometry.as_ref().unwrap().value,
            Value::Polygon(vec![vec![
                vec![-10.0, -10.0],
                vec![20.0, -10.0],
                vec![20.0, 20.0],
                vec![-10.0, 20.0],
                vec![-10.0, -10.0]
            ]])
        );
    }
}
