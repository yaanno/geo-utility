use geo::{Contains, Intersects, LineString, MultiPoint, Point, Polygon};

use crate::utils::geometry::{GeoFeature, GeoFeatureCollection, GeoGeometry};
use crate::utils::error::Error;

pub fn pick_features_by_boundingbox(
    featurecollection: &GeoFeatureCollection,
    bbox: Polygon,
) -> Result<Vec<&GeoFeature>, Error> {
    let mut selected_features = Vec::with_capacity(featurecollection.features.len());

    for feature in &featurecollection.features {
        let geometry_value = match feature.geometry.as_ref() {
            Some(geometry) => geometry,
            None => {
                // Skip features without geometry
                continue;
            }
        };

        match geometry_value {
            GeoGeometry::Point(coord) => {
                // Convert the coordinate to a GeoCoord
                let geo_coord = Point::new(coord.x(), coord.y());

                // Check if the point is contained or intersects the bbox
                let contains = bbox.contains(&geo_coord);
                let intersects = bbox.intersects(&geo_coord);
                if contains || intersects {
                    selected_features.push(feature);
                }
            }
            GeoGeometry::LineString(line_coords) => {
                // Convert the line coordinates to a LineString
                let line: LineString<f64> = line_coords
                    .into_iter()
                    .map(|coord| Point::new(coord.x, coord.y))
                    .collect();

                // Check if the line intersects or contains the bbox
                if bbox.intersects(&line) || bbox.contains(&line) {
                    selected_features.push(feature);
                }
            }
            GeoGeometry::Polygon(polygon_coords) => {
                // Extract coords from exterior ring; interior rings don't affect intersection
                let exterior_ring = polygon_coords.exterior();

                // Convert the exterior ring coordinates to a LineString
                let line = exterior_ring
                    .into_iter()
                    .map(|c| Point::new(c.x, c.y))
                    .collect();

                // Create a Polygon from the LineString
                let polygon = Polygon::new(line, vec![]);

                // Check if the Polygon intersects or contains the bbox
                if bbox.intersects(&polygon) || bbox.contains(&polygon) {
                    selected_features.push(feature);
                }
            }
            GeoGeometry::MultiPoint(point_coords_vec) => {
                // Convert the point coordinates to a Vec of Coords
                let coords: Vec<Point<f64>> = point_coords_vec
                    .into_iter()
                    .map(|coord| Point::new(coord.x(), coord.y()))
                    .collect();
                // Create a MultiPoint from the coordinates
                let multi_point = MultiPoint::from(coords);

                // Check if the MultiPoint intersects or contains the bbox
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
    use geo::{polygon, MultiPolygon};

    // Helper function to create a GeoJSON Feature with a given geometry
    fn create_feature(geometry: GeoGeometry) -> GeoFeature {
        GeoFeature {
            bbox: None,
            geometry: Some(geometry),
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
        let feature_collection = GeoFeatureCollection {
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
        let point_inside = create_feature(GeoGeometry::Point(Point::new(5.0, 5.0)));
        let feature_collection = GeoFeatureCollection {
            bbox: None,
            features: vec![point_inside.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert_eq!(selected.len(), 1);
        assert_eq!(
            selected[0].geometry,
            Some(GeoGeometry::Point(Point::new(5.0, 5.0)))
        );
    }

    #[test]
    fn test_point_on_bbox_boundary() {
        let point_on_boundary = create_feature(GeoGeometry::Point(Point::new(0.0, 5.0)));
        let feature_collection = GeoFeatureCollection {
            bbox: None,
            features: vec![point_on_boundary.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert_eq!(selected.len(), 1);
        assert_eq!(
            selected[0].geometry,
            Some(GeoGeometry::Point(Point::new(0.0, 5.0)))
        );
    }

    #[test]
    fn test_point_outside_bbox() {
        let point_outside = create_feature(GeoGeometry::Point(Point::new(11.0, 11.0)));
        let feature_collection = GeoFeatureCollection {
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
        let linestring_inside = create_feature(GeoGeometry::LineString(LineString::from(vec![
            Point::new(1.0, 1.0),
            Point::new(9.0, 9.0),
        ])));
        let feature_collection = GeoFeatureCollection {
            bbox: None,
            features: vec![linestring_inside.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert_eq!(selected.len(), 1);
        assert_eq!(
            selected[0].geometry,
            Some(GeoGeometry::LineString(LineString::from(vec![
                Point::new(1.0, 1.0),
                Point::new(9.0, 9.0)
            ])))
        );
    }

    #[test]
    fn test_linestring_intersecting_bbox() {
        let linestring_intersecting =
            create_feature(GeoGeometry::LineString(LineString::from(vec![
                Point::new(-1.0, 5.0),
                Point::new(11.0, 5.0),
            ])));
        let feature_collection = GeoFeatureCollection {
            bbox: None,
            features: vec![linestring_intersecting.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert_eq!(selected.len(), 1);
        assert_eq!(
            selected[0].geometry,
            Some(GeoGeometry::LineString(LineString::from(vec![
                Point::new(-1.0, 5.0),
                Point::new(11.0, 5.0)
            ])))
        );
    }

    #[test]
    fn test_linestring_outside_bbox() {
        let linestring_outside = create_feature(GeoGeometry::LineString(LineString::from(vec![
            Point::new(11.0, 1.0),
            Point::new(12.0, 2.0),
        ])));
        let feature_collection = GeoFeatureCollection {
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
        let polygon_inside = create_feature(GeoGeometry::Polygon(Polygon::new(
            LineString::from(vec![
                Point::new(1.0, 1.0),
                Point::new(3.0, 1.0),
                Point::new(3.0, 3.0),
                Point::new(1.0, 3.0),
                Point::new(1.0, 1.0),
            ]),
            vec![],
        )));
        let feature_collection = GeoFeatureCollection {
            bbox: None,
            features: vec![polygon_inside.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert_eq!(selected.len(), 1);
        assert_eq!(
            selected[0].geometry,
            Some(GeoGeometry::Polygon(Polygon::new(
                LineString::from(vec![
                    Point::new(1.0, 1.0),
                    Point::new(3.0, 1.0),
                    Point::new(3.0, 3.0),
                    Point::new(1.0, 3.0),
                    Point::new(1.0, 1.0)
                ]),
                vec![],
            )))
        );
        assert_eq!(
            selected[0].geometry,
            Some(GeoGeometry::Polygon(Polygon::new(
                LineString::from(vec![
                    Point::new(1.0, 1.0),
                    Point::new(3.0, 1.0),
                    Point::new(3.0, 3.0),
                    Point::new(1.0, 3.0),
                    Point::new(1.0, 1.0),
                ]),
                vec![],
            )))
        );
    }

    #[test]
    fn test_polygon_intersecting_bbox() {
        let polygon_intersecting = create_feature(GeoGeometry::Polygon(Polygon::new(
            LineString::from(vec![
                Point::new(-1.0, -1.0),
                Point::new(1.0, -1.0),
                Point::new(1.0, 1.0),
                Point::new(-1.0, 1.0),
                Point::new(-1.0, -1.0),
            ]),
            vec![],
        )));
        let feature_collection = GeoFeatureCollection {
            bbox: None,
            features: vec![polygon_intersecting.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert_eq!(selected.len(), 1);
        let poly = GeoGeometry::Polygon(Polygon::new(
            LineString::from(vec![
                Point::new(-1.0, -1.0),
                Point::new(1.0, -1.0),
                Point::new(1.0, 1.0),
                Point::new(-1.0, 1.0),
                Point::new(-1.0, -1.0),
            ]),
            vec![],
        ));
        assert_eq!(selected[0].geometry, Some(poly));
    }

    // Note: The current implementation of Polygon handling in the function only considers
    // the exterior ring. This test reflects that limitation.
    #[test]
    fn test_polygon_with_hole_intersecting_bbox_by_exterior() {
        let polygon_with_hole = create_feature(GeoGeometry::Polygon(Polygon::new(
            LineString::from(vec![
                Point::new(0.0, 0.0),
                Point::new(10.0, 0.0),
                Point::new(10.0, 10.0),
                Point::new(0.0, 10.0),
                Point::new(0.0, 0.0),
            ]), // Exterior
            vec![
                LineString::from(vec![
                    Point::new(4.0, 4.0),
                    Point::new(6.0, 4.0),
                    Point::new(6.0, 6.0),
                    Point::new(4.0, 6.0),
                    Point::new(4.0, 4.0),
                ]),
            ], // Interior (hole)
        )));
        let feature_collection = GeoFeatureCollection {
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
        let poly = GeoGeometry::Polygon(Polygon::new(
            LineString::from(vec![
                Point::new(0.0, 0.0),
                Point::new(10.0, 0.0),
                Point::new(10.0, 10.0),
                Point::new(0.0, 10.0),
                Point::new(0.0, 0.0),
            ]),
            vec![
                LineString::from(vec![
                    Point::new(4.0, 4.0),
                    Point::new(6.0, 4.0),
                    Point::new(6.0, 6.0),
                    Point::new(4.0, 6.0),
                    Point::new(4.0, 4.0),
                ]),
            ],
        ));
        assert_eq!(selected[0].geometry, Some(poly));
    }

    #[test]
    fn test_polygon_outside_bbox() {
        let polygon_outside = create_feature(GeoGeometry::Polygon(Polygon::new(
            LineString::from(vec![
                Point::new(11.0, 11.0),
                Point::new(12.0, 11.0),
                Point::new(12.0, 12.0),
                Point::new(11.0, 12.0),
                Point::new(11.0, 11.0),
            ]),
            vec![],
        )));
        let feature_collection = GeoFeatureCollection {
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
        let multipoint_inside = create_feature(GeoGeometry::MultiPoint(MultiPoint::from(vec![
            Point::new(1.0, 1.0),
            Point::new(2.0, 2.0),
            Point::new(3.0, 3.0),
        ])));
        let feature_collection = GeoFeatureCollection {
            bbox: None,
            features: vec![multipoint_inside.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert_eq!(selected.len(), 1);
        assert_eq!(
            selected[0].geometry,
            Some(GeoGeometry::MultiPoint(MultiPoint::from(vec![
                Point::new(1.0, 1.0),
                Point::new(2.0, 2.0),
                Point::new(3.0, 3.0),
            ])))
        );
    }

    #[test]
    fn test_multipoint_intersecting_bbox() {
        let multipoint_intersecting = create_feature(GeoGeometry::MultiPoint(MultiPoint::from(vec![
            Point::new(-1.0, -1.0),
            Point::new(5.0, 5.0),
            Point::new(11.0, 11.0),
        ])));
        let feature_collection = GeoFeatureCollection {
            bbox: None,
            features: vec![multipoint_intersecting.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert_eq!(selected.len(), 1);
        assert_eq!(
            selected[0].geometry,
            Some(GeoGeometry::MultiPoint(MultiPoint::from(vec![
                Point::new(-1.0, -1.0),
                Point::new(5.0, 5.0),
                Point::new(11.0, 11.0),
            ])))
        );
    }

    #[test]
    fn test_multipoint_outside_bbox() {
        let multipoint_outside =
            create_feature(GeoGeometry::MultiPoint(MultiPoint::from(vec![
                Point::new(11.0, 11.0),
                Point::new(12.0, 12.0),
            ])));
        let feature_collection = GeoFeatureCollection {
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
        let feature_no_geometry = GeoFeature {
            bbox: None,
            geometry: None, // Missing geometry
            id: None,
            properties: None,
            foreign_members: None,
        };
        let feature_collection = GeoFeatureCollection {
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
        let unsupported_geometry = create_feature(GeoGeometry::MultiPolygon(MultiPolygon::from(vec![Polygon::new(
            LineString::from(vec![
                Point::new(0.0, 0.0),
                Point::new(1.0, 0.0),
                Point::new(1.0, 1.0),
                Point::new(0.0, 0.0),
            ]),
            vec![],
        )])));
        let feature_collection = GeoFeatureCollection {
            bbox: None,
            features: vec![unsupported_geometry.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        // Features with unsupported geometry types should be skipped
        assert!(selected.is_empty());
    }

    // #[test]
    // fn test_collection_with_mixed_features() {
    //     let point_inside = create_feature(GeoGeometry::Point(Point::new(5.0, 5.0)));
    //     let linestring_intersecting =
    //         create_feature(GeoGeometry::LineString(LineString::from(vec![
    //             Point::new(-1.0, 5.0),
    //             Point::new(11.0, 5.0),
    //         ])));
    //     let polygon_outside = create_feature(GeoGeometry::Polygon(Polygon::new(
    //         LineString::from(vec![
    //             Point::new(11.0, 11.0),
    //             Point::new(12.0, 11.0),
    //             Point::new(12.0, 12.0),
    //             Point::new(11.0, 12.0),
    //             Point::new(11.0, 11.0),
    //         ]),
    //         vec![],
    //     )));
    //     let feature_no_geometry = GeoFeature {
    //         bbox: None,
    //         geometry: None,
    //         id: None,
    //         properties: None,
    //         foreign_members: None,
    //     };
    //     let unsupported_geometry = create_feature(GeoGeometry::MultiPolygon(MultiPolygon::from(vec![Polygon::new(
    //         LineString::from(vec![
    //             Point::new(0.0, 0.0),
    //             Point::new(1.0, 0.0),
    //             Point::new(1.0, 1.0),
    //             Point::new(0.0, 0.0),
    //         ]),
    //         vec![],
    //     )])));

    //     let feature_collection = GeoFeatureCollection {
    //         bbox: None,
    //         features: vec![
    //             point_inside.clone(),
    //             linestring_intersecting.clone(),
    //             polygon_outside.clone(),
    //             feature_no_geometry.clone(),
    //             unsupported_geometry.clone(),
    //         ],
    //         foreign_members: None,
    //     };
    //     let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

    //     let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

    //     assert_eq!(selected.len(), 2);
    //     // Verify that the correct features were selected
    //     let selected_values: Vec<GeoGeometry> = selected
    //         .iter()
    //         .filter_map(|f| f.geometry.as_ref().map(|g| g.clone()))
    //         .collect();

    //     assert!(selected_values.contains(&point_inside));
    //     assert!(selected_values.contains(&linestring_intersecting));
    //     assert!(!selected_values.contains(&polygon_outside));
    //     assert!(!selected_values.contains(&feature_no_geometry));
    //     assert!(!selected_values.contains(&unsupported_geometry));
    //     assert!(!selected_values.contains(&GeoGeometry::Polygon(Polygon::new(
    //         LineString::from(vec![
    //             Point::new(11.0, 11.0),
    //             Point::new(12.0, 11.0),
    //             Point::new(12.0, 12.0),
    //             Point::new(11.0, 12.0),
    //             Point::new(11.0, 11.0),
    //         ]),
    //         vec![],
    //     ))));
    //     let polygon_on_boundary = create_feature(GeoGeometry::Polygon(Polygon::new(
    //         LineString::from(vec![
    //             Point::new(0.0, 0.0),
    //             Point::new(1.0, 0.0),
    //             Point::new(1.0, 1.0),
    //             Point::new(0.0, 1.0),
    //             Point::new(0.0, 0.0),
    //         ]),
    //         vec![],
    //     )));
    //     assert!(!selected_values.contains(&polygon_on_boundary));

    //     let multi_polygon_on_boundary = create_feature(GeoGeometry::MultiPolygon(MultiPolygon::from(vec![Polygon::new(
    //         LineString::from(vec![
    //             Point::new(0.0, 0.0),
    //             Point::new(1.0, 0.0),
    //             Point::new(1.0, 1.0),
    //             Point::new(0.0, 1.0),
    //             Point::new(0.0, 0.0),
    //         ]),
    //         vec![],
    //     )])));
    //     assert!(!selected_values.contains(&multi_polygon_on_boundary));
    // }

    #[test]
    fn test_polygon_on_bbox_boundary() {
        let polygon_on_boundary = create_feature(GeoGeometry::Polygon(Polygon::new(
            LineString::from(vec![
                Point::new(0.0, 0.0),
                Point::new(1.0, 0.0),
                Point::new(1.0, 1.0),
                Point::new(0.0, 1.0),
                Point::new(0.0, 0.0),
            ]),
            vec![],
        )));
        let feature_collection = GeoFeatureCollection {
            bbox: None,
            features: vec![polygon_on_boundary.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert_eq!(selected.len(), 1);
        let poly = GeoGeometry::Polygon(Polygon::new(
            LineString::from(vec![
                Point::new(0.0, 0.0),
                    Point::new(1.0, 0.0),
                    Point::new(1.0, 1.0),
                    Point::new(0.0, 1.0),
                    Point::new(0.0, 0.0),
                ]),
                vec![],
            ));
        assert_eq!(selected[0].geometry.as_ref().unwrap(), &poly);
    }

    #[test]
    fn test_linestring_on_bbox_boundary() {
        let linestring_on_boundary =
            create_feature(GeoGeometry::LineString(LineString::from(vec![
                Point::new(0.0, 5.0),
                Point::new(10.0, 5.0),
            ])));
        let feature_collection = GeoFeatureCollection {
            bbox: None,
            features: vec![linestring_on_boundary.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert_eq!(selected.len(), 1);
        let ls = GeoGeometry::LineString(LineString::from(vec![
            Point::new(0.0, 5.0),
            Point::new(10.0, 5.0),
        ]));
        assert_eq!(selected[0].geometry.as_ref().unwrap(), &ls);
    }

    #[test]
    fn test_linestring_on_bbox_boundary_2() {
        let linestring_on_boundary =
            create_feature(GeoGeometry::LineString(LineString::from(vec![
                Point::new(0.0, 5.0),
                Point::new(10.0, 5.0),
            ])));
        let feature_collection = GeoFeatureCollection {
            bbox: None,
            features: vec![linestring_on_boundary.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert_eq!(selected.len(), 1);
        assert_eq!(
            selected[0].geometry,
            Some(GeoGeometry::LineString(LineString::from(vec![
                Point::new(0.0, 5.0),
                Point::new(10.0, 5.0),
            ])))
        );
    }

    #[test]
    fn test_multipoint_on_bbox_boundary() {
        let multipoint_on_boundary =
            create_feature(GeoGeometry::MultiPoint(MultiPoint::from(vec![Point::new(0.0, 0.0), Point::new(10.0, 10.0)])));
        let feature_collection = GeoFeatureCollection {
            bbox: None,
            features: vec![multipoint_on_boundary.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert_eq!(selected.len(), 1);
        assert_eq!(
            selected[0].geometry,
            Some(GeoGeometry::MultiPoint(MultiPoint::from(vec![Point::new(0.0, 0.0), Point::new(10.0, 10.0)])))
        );
    }

    // Test case for a polygon completely containing the bbox
    #[test]
    fn test_polygon_containing_bbox() {
        let large_polygon = create_feature(GeoGeometry::Polygon(Polygon::new(
            LineString::from(vec![
                Point::new(-10.0, -10.0),
                Point::new(20.0, -10.0),
                Point::new(20.0, 20.0),
                Point::new(-10.0, 20.0),
                Point::new(-10.0, -10.0),
            ]),
            vec![],
        )));
        let feature_collection = GeoFeatureCollection {
            bbox: None,
            features: vec![large_polygon.clone()],
            foreign_members: None,
        };
        let bbox = create_bbox(0.0, 0.0, 10.0, 10.0);

        let selected = pick_features_by_boundingbox(&feature_collection, bbox).unwrap();

        assert_eq!(selected.len(), 1);
        assert_eq!(
            selected[0].geometry,
            Some(GeoGeometry::Polygon(Polygon::new(
                LineString::from(vec![
                    Point::new(-10.0, -10.0),
                    Point::new(20.0, -10.0),
                    Point::new(20.0, 20.0),
                    Point::new(-10.0, 20.0),
                ]),
                vec![],
            )))
        );
    }
}
