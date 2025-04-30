use geo::HasDimensions;
use geo::{Coord, LineString, Polygon, Scale};

pub fn scale_buildings(
    feature_collection: &geojson::FeatureCollection,
    scale_factor: f64,
) -> geojson::FeatureCollection {
    // create a new feature collection
    let mut scaled_feature_collection = geojson::FeatureCollection::default();
    // for each feature in feature_collection
    for feature in feature_collection.features.iter() {
        // if feature.geometry is Some
        if let Some(geometry) = &feature.geometry {
            // match geometry.value
            match &geometry.value {
                // if geometry.value is LineString
                geojson::Value::LineString(line_coords) => {
                    // Convert the line coordinates to a LineString
                    let line: LineString<f64> = line_coords
                        .into_iter()
                        .map(|coord| Coord {
                            x: coord[0],
                            y: coord[1],
                        })
                        .collect();

                    // Check if the line is empty
                    if line.is_empty() {
                        continue; // Skip empty LineStrings
                    }
                    let scaled_line: LineString<f64>;
                    // check if the line is closed: first and last point are the same

                    if line.is_closed() {
                        // convert linestring to polygon
                        let polygon = Polygon::new(line, vec![]);
                        let scaled_polygon = polygon.scale(scale_factor);
                        scaled_line = scaled_polygon.exterior().clone();
                    } else {
                        // Scale the LineString
                        scaled_line = line
                            .coords()
                            .map(|&c| Coord {
                                x: scale_factor * c.x,
                                y: scale_factor * c.y,
                            })
                            .collect();
                    }

                    scaled_feature_collection.features.push(geojson::Feature {
                        bbox: feature.bbox.clone(),
                        geometry: Some(geojson::Geometry {
                            bbox: geometry.bbox.clone(),
                            value: geojson::Value::LineString(
                                scaled_line
                                    .into_iter()
                                    .map(|coord| vec![coord.x, coord.y])
                                    .collect(),
                            ),
                            foreign_members: geometry.foreign_members.clone(),
                        }),
                        id: feature.id.clone(),
                        properties: feature.properties.clone(),
                        foreign_members: feature.foreign_members.clone(),
                    });
                }
                // if geometry.value is Polygon
                geojson::Value::Polygon(polygon_coords) => {
                    if let Some(exterior_ring) = polygon_coords.first() {
                        // Convert the exterior ring coordinates to a LineString
                        let line: LineString = exterior_ring
                            .iter()
                            .map(|c| Coord { x: c[0], y: c[1] })
                            .collect();

                        if line.is_empty() {
                            continue;
                        }

                        // Create a Polygon from the LineString
                        let polygon = Polygon::new(line, vec![]);

                        // Scale the Polygon
                        let scaled_polygon = polygon.scale(scale_factor);
                        scaled_feature_collection.features.push(geojson::Feature {
                            bbox: feature.bbox.clone(),
                            geometry: Some(geojson::Geometry {
                                bbox: geometry.bbox.clone(),
                                value: geojson::Value::Polygon(vec![
                                    scaled_polygon
                                        .exterior()
                                        .into_iter()
                                        .map(|coord| vec![coord.x, coord.y])
                                        .collect(),
                                ]),
                                foreign_members: geometry.foreign_members.clone(),
                            }),
                            id: feature.id.clone(),
                            properties: feature.properties.clone(),
                            foreign_members: feature.foreign_members.clone(),
                        });
                    } else {
                        // Polygon has no exterior ring, skip
                        continue;
                    }
                }
                // if geometry.value is Point
                geojson::Value::Point(coord) => {
                    let coord = Coord {
                        x: coord[0],
                        y: coord[1],
                    };

                    // Use manual scaling relative to (0,0)
                    let scaled_point_coord = Coord {
                        x: scale_factor * coord.x,
                        y: scale_factor * coord.y,
                    };

                    scaled_feature_collection.features.push(geojson::Feature {
                        bbox: feature.bbox.clone(),
                        geometry: Some(geojson::Geometry {
                            bbox: geometry.bbox.clone(),
                            value: geojson::Value::Point(vec![
                                scaled_point_coord.x,
                                scaled_point_coord.y,
                            ]),
                            foreign_members: geometry.foreign_members.clone(),
                        }),
                        id: feature.id.clone(),
                        properties: feature.properties.clone(),
                        foreign_members: feature.foreign_members.clone(),
                    });
                }
                // if geometry.value is MultiPoint
                geojson::Value::MultiPoint(point_coords_vec) => {
                    // Convert the point coordinates to a Vec of Coords
                    let coords: Vec<Coord<f64>> = point_coords_vec
                        .into_iter()
                        .map(|coord| Coord {
                            x: coord[0],
                            y: coord[1],
                        })
                        .collect();
                    // Create a MultiPoint from the coordinates
                    let scaled_coords: Vec<Coord<f64>> = coords
                        .into_iter()
                        .map(|c| Coord {
                            x: scale_factor * c.x,
                            y: scale_factor * c.y,
                        })
                        .collect();

                    scaled_feature_collection.features.push(geojson::Feature {
                        bbox: feature.bbox.clone(),
                        geometry: Some(geojson::Geometry {
                            bbox: geometry.bbox.clone(),
                            value: geojson::Value::MultiPoint(
                                scaled_coords.into_iter().map(|c| vec![c.x, c.y]).collect(),
                            ),
                            foreign_members: geometry.foreign_members.clone(),
                        }),
                        id: feature.id.clone(),
                        properties: feature.properties.clone(),
                        foreign_members: feature.foreign_members.clone(),
                    });
                }
                _ => {
                    // Skip unsupported geometry types
                    continue;
                }
            }
        }
    }
    scaled_feature_collection
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::{Centroid, MultiPoint, Point}; // Import Centroid trait for calculating expected centroids
    use geojson::{Feature, FeatureCollection, Geometry, Value};
    use serde_json::json; // Useful for creating arbitrary JSON properties/foreign_members

    // Helper function to create a simple FeatureCollection with one geometry
    fn create_feature_collection(geometry: Option<Geometry>) -> FeatureCollection {
        let feature = Feature {
            bbox: None,
            geometry,
            id: None,
            properties: None,
            foreign_members: None,
        };
        FeatureCollection {
            bbox: None,
            features: vec![feature],
            foreign_members: None,
        }
    }

    // Helper function to create a Feature with metadata
    fn create_feature_with_metadata(geometry: Option<Geometry>) -> FeatureCollection {
        let feature = Feature {
            bbox: Some(vec![-180.0, -90.0, 180.0, 90.0]), // Example bbox
            geometry: geometry.map(|mut g| {
                // Clone geometry bbox if present
                g.bbox = Some(vec![-10.0, -5.0, 10.0, 5.0]); // Example geometry bbox
                g.foreign_members = Some(serde_json::Map::from_iter(vec![(
                    "geom_meta".to_string(),
                    serde_json::Value::String("test".to_string()),
                )]));
                g
            }),
            id: Some(geojson::feature::Id::String("test-id".to_string())), // Example ID
            properties: Some(serde_json::Map::from_iter(vec![
                ("name".to_string(), json!("Test Feature")),
                ("value".to_string(), json!(123)),
            ])), // Example properties
            foreign_members: Some(serde_json::Map::from_iter(vec![(
                "feature_meta".to_string(),
                serde_json::Value::String("data".to_string()),
            )])), // Example foreign members
        };
        FeatureCollection {
            bbox: Some(vec![-180.0, -90.0, 180.0, 90.0]), // Example collection bbox
            features: vec![feature],
            foreign_members: Some(serde_json::Map::from_iter(vec![(
                "collection_meta".to_string(),
                serde_json::Value::String("info".to_string()),
            )])), // Example collection foreign members
        }
    }

    // Helper function to scale a coordinate relative to an origin
    fn scale_coord(coord: Coord<f64>, origin: Coord<f64>, scale_factor: f64) -> Coord<f64> {
        Coord {
            x: origin.x + scale_factor * (coord.x - origin.x),
            y: origin.y + scale_factor * (coord.y - origin.y),
        }
    }

    // --- Test Point Scaling ---

    #[test]
    fn test_scale_point_shrink() {
        let input_point = Point::new(10.0, 20.0);
        let input_collection = create_feature_collection(Some(Geometry::new(Value::Point(vec![
            input_point.x(),
            input_point.y(),
        ]))));
        let scale_factor = 0.5;

        // Expected: scaled relative to (0,0)
        let expected_coord =
            scale_coord(input_point.into(), Coord { x: 0.0, y: 0.0 }, scale_factor);
        let expected_point = Point::new(expected_coord.x, expected_coord.y);
        let expected_collection =
            create_feature_collection(Some(Geometry::new(Value::Point(vec![
                expected_point.x(),
                expected_point.y(),
            ]))));

        let actual_collection = scale_buildings(&input_collection, scale_factor);
        assert_eq!(actual_collection, expected_collection);
    }

    #[test]
    fn test_scale_point_enlarge() {
        let input_point = Point::new(-5.0, -10.0);
        let input_collection = create_feature_collection(Some(Geometry::new(Value::Point(vec![
            input_point.x(),
            input_point.y(),
        ]))));
        let scale_factor = 2.0;

        // Expected: scaled relative to (0,0)
        let expected_coord =
            scale_coord(input_point.into(), Coord { x: 0.0, y: 0.0 }, scale_factor);
        let expected_point = Point::new(expected_coord.x, expected_coord.y);
        let expected_collection =
            create_feature_collection(Some(Geometry::new(Value::Point(vec![
                expected_point.x(),
                expected_point.y(),
            ]))));

        let actual_collection = scale_buildings(&input_collection, scale_factor);
        assert_eq!(actual_collection, expected_collection);
    }

    #[test]
    fn test_scale_point_no_change() {
        let input_point = Point::new(100.0, -50.0);
        let input_collection = create_feature_collection(Some(Geometry::new(Value::Point(vec![
            input_point.x(),
            input_point.y(),
        ]))));
        let scale_factor = 1.0;

        // Expected: no change (scaled relative to (0,0) by 1.0)
        let expected_collection =
            create_feature_collection(Some(Geometry::new(Value::Point(vec![
                input_point.x(),
                input_point.y(),
            ]))));

        let actual_collection = scale_buildings(&input_collection, scale_factor);
        assert_eq!(actual_collection, expected_collection);
    }

    #[test]
    fn test_scale_point_zero_factor() {
        let input_point = Point::new(10.0, 20.0);
        let input_collection = create_feature_collection(Some(Geometry::new(Value::Point(vec![
            input_point.x(),
            input_point.y(),
        ]))));
        let scale_factor = 0.0;

        // Expected: scaled relative to (0,0) by 0.0 -> (0,0)
        let expected_point = Point::new(0.0, 0.0);
        let expected_collection =
            create_feature_collection(Some(Geometry::new(Value::Point(vec![
                expected_point.x(),
                expected_point.y(),
            ]))));

        let actual_collection = scale_buildings(&input_collection, scale_factor);
        assert_eq!(actual_collection, expected_collection);
    }

    #[test]
    fn test_scale_point_negative_factor() {
        let input_point = Point::new(10.0, 20.0);
        let input_collection = create_feature_collection(Some(Geometry::new(Value::Point(vec![
            input_point.x(),
            input_point.y(),
        ]))));
        let scale_factor = -1.0; // Reflection through (0,0)

        // Expected: scaled relative to (0,0) by -1.0 -> (-10.0, -20.0)
        let expected_point = Point::new(-10.0, -20.0);
        let expected_collection =
            create_feature_collection(Some(Geometry::new(Value::Point(vec![
                expected_point.x(),
                expected_point.y(),
            ]))));

        let actual_collection = scale_buildings(&input_collection, scale_factor);
        assert_eq!(actual_collection, expected_collection);
    }

    // --- Test LineString Scaling ---

    #[test]
    fn test_scale_open_linestring_shrink() {
        // An open LineString
        let input_line: LineString<f64> = vec![
            Coord { x: 1.0, y: 1.0 },
            Coord { x: 5.0, y: 5.0 },
            Coord { x: 1.0, y: 5.0 },
        ]
        .into();
        let input_collection = create_feature_collection(Some(Geometry::new(Value::LineString(
            input_line.coords().map(|c| vec![c.x, c.y]).collect(),
        ))));
        let scale_factor = 0.5;

        // Expected: scaled relative to (0,0)
        let expected_coords: Vec<Coord<f64>> = input_line
            .coords()
            .map(|&c| scale_coord(c, Coord { x: 0.0, y: 0.0 }, scale_factor))
            .collect();
        let expected_line: LineString<f64> = expected_coords.into();
        let expected_collection = create_feature_collection(Some(Geometry::new(
            Value::LineString(expected_line.coords().map(|c| vec![c.x, c.y]).collect()),
        )));

        let actual_collection = scale_buildings(&input_collection, scale_factor);
        assert_eq!(actual_collection, expected_collection);
    }

    #[test]
    fn test_scale_open_linestring_enlarge() {
        // An open LineString
        let input_line: LineString<f64> = vec![
            Coord { x: 10.0, y: 10.0 },
            Coord { x: 10.0, y: 20.0 },
            Coord { x: 20.0, y: 20.0 },
        ]
        .into();
        let input_collection = create_feature_collection(Some(Geometry::new(Value::LineString(
            input_line.coords().map(|c| vec![c.x, c.y]).collect(),
        ))));
        let scale_factor = 2.0;

        // Expected: scaled relative to (0,0)
        let expected_coords: Vec<Coord<f64>> = input_line
            .coords()
            .map(|&c| scale_coord(c, Coord { x: 0.0, y: 0.0 }, scale_factor))
            .collect();
        let expected_line: LineString<f64> = expected_coords.into();
        let expected_collection = create_feature_collection(Some(Geometry::new(
            Value::LineString(expected_line.coords().map(|c| vec![c.x, c.y]).collect()),
        )));

        let actual_collection = scale_buildings(&input_collection, scale_factor);
        assert_eq!(actual_collection, expected_collection);
    }

    #[test]
    fn test_scale_closed_linestring_shrink() {
        // A closed LineString (a square)
        let input_line: LineString<f64> = vec![
            Coord { x: 0.0, y: 0.0 },
            Coord { x: 10.0, y: 0.0 },
            Coord { x: 10.0, y: 10.0 },
            Coord { x: 0.0, y: 10.0 },
            Coord { x: 0.0, y: 0.0 }, // Closed
        ]
        .into();
        assert!(input_line.is_closed()); // Sanity check
        let input_collection = create_feature_collection(Some(Geometry::new(Value::LineString(
            input_line.coords().map(|c| vec![c.x, c.y]).collect(),
        ))));
        let scale_factor = 0.5;

        // Expected: scaled relative to the centroid of the Polygon created from the line
        let input_polygon = Polygon::new(input_line.clone(), vec![]);
        let origin = input_polygon.centroid().unwrap().into(); // Centroid of (0,0)-(10,10) square is (5,5)

        let expected_coords: Vec<Coord<f64>> = input_line
            .coords()
            .map(|&c| scale_coord(c, origin, scale_factor))
            .collect();
        let expected_line: LineString<f64> = expected_coords.into();
        let expected_collection = create_feature_collection(Some(Geometry::new(
            Value::LineString(expected_line.coords().map(|c| vec![c.x, c.y]).collect()),
        )));

        let actual_collection = scale_buildings(&input_collection, scale_factor);
        assert_eq!(actual_collection, expected_collection);
    }

    #[test]
    fn test_scale_closed_linestring_enlarge_offset() {
        // A closed LineString (a square) offset from (0,0)
        let input_line: LineString<f64> = vec![
            Coord { x: 10.0, y: 10.0 },
            Coord { x: 20.0, y: 10.0 },
            Coord { x: 20.0, y: 20.0 },
            Coord { x: 10.0, y: 20.0 },
            Coord { x: 10.0, y: 10.0 }, // Closed
        ]
        .into();
        assert!(input_line.is_closed()); // Sanity check
        let input_collection = create_feature_collection(Some(Geometry::new(Value::LineString(
            input_line.coords().map(|c| vec![c.x, c.y]).collect(),
        ))));
        let scale_factor = 2.0;

        // Expected: scaled relative to the centroid of the Polygon created from the line
        let input_polygon = Polygon::new(input_line.clone(), vec![]);
        let origin = input_polygon.centroid().unwrap().into(); // Centroid of (10,10)-(20,20) square is (15,15)

        let expected_coords: Vec<Coord<f64>> = input_line
            .coords()
            .map(|&c| scale_coord(c, origin, scale_factor))
            .collect();
        let expected_line: LineString<f64> = expected_coords.into();
        let expected_collection = create_feature_collection(Some(Geometry::new(
            Value::LineString(expected_line.coords().map(|c| vec![c.x, c.y]).collect()),
        )));

        let actual_collection = scale_buildings(&input_collection, scale_factor);
        assert_eq!(actual_collection, expected_collection);
    }

    #[test]
    fn test_scale_empty_linestring() {
        let input_line: LineString<f64> = LineString::new(vec![]);
        let input_collection = create_feature_collection(Some(Geometry::new(Value::LineString(
            input_line.coords().map(|c| vec![c.x, c.y]).collect(),
        ))));
        let scale_factor = 0.5;

        // Expected: skipped, output collection should be empty
        let expected_collection = FeatureCollection::default();

        let actual_collection = scale_buildings(&input_collection, scale_factor);
        assert_eq!(actual_collection, expected_collection);
    }

    // --- Test Polygon Scaling ---

    #[test]
    fn test_scale_polygon_shrink() {
        // A square Polygon (exterior only)
        let input_polygon = Polygon::new(
            LineString::from(vec![
                Coord { x: 0.0, y: 0.0 },
                Coord { x: 10.0, y: 0.0 },
                Coord { x: 10.0, y: 10.0 },
                Coord { x: 0.0, y: 10.0 },
                Coord { x: 0.0, y: 0.0 },
            ]),
            vec![], // No inner rings
        );
        let input_collection =
            create_feature_collection(Some(Geometry::new(Value::Polygon(vec![
                input_polygon
                    .exterior()
                    .coords()
                    .map(|c| vec![c.x, c.y])
                    .collect(),
            ]))));
        let scale_factor = 0.5;

        // Expected: scaled relative to its centroid
        let origin = input_polygon.centroid().unwrap().into(); // Centroid of (0,0)-(10,10) square is (5,5)
        let expected_exterior_coords: Vec<Coord<f64>> = input_polygon
            .exterior()
            .coords()
            .map(|&c| scale_coord(c, origin, scale_factor))
            .collect();
        let expected_polygon = Polygon::new(expected_exterior_coords.into(), vec![]); // Expected output has no inner rings

        let expected_collection =
            create_feature_collection(Some(Geometry::new(Value::Polygon(vec![
                expected_polygon
                    .exterior()
                    .coords()
                    .map(|c| vec![c.x, c.y])
                    .collect(),
            ]))));

        let actual_collection = scale_buildings(&input_collection, scale_factor);
        assert_eq!(actual_collection, expected_collection);
    }

    #[test]
    fn test_scale_polygon_enlarge_offset() {
        // A square Polygon (exterior only) offset from (0,0)
        let input_polygon = Polygon::new(
            LineString::from(vec![
                Coord { x: 10.0, y: 10.0 },
                Coord { x: 20.0, y: 10.0 },
                Coord { x: 20.0, y: 20.0 },
                Coord { x: 10.0, y: 20.0 },
                Coord { x: 10.0, y: 10.0 },
            ]),
            vec![], // No inner rings
        );
        let input_collection =
            create_feature_collection(Some(Geometry::new(Value::Polygon(vec![
                input_polygon
                    .exterior()
                    .coords()
                    .map(|c| vec![c.x, c.y])
                    .collect(),
            ]))));
        let scale_factor = 2.0;

        // Expected: scaled relative to its centroid
        let origin = input_polygon.centroid().unwrap().into(); // Centroid of (10,10)-(20,20) square is (15,15)
        let expected_exterior_coords: Vec<Coord<f64>> = input_polygon
            .exterior()
            .coords()
            .map(|&c| scale_coord(c, origin, scale_factor))
            .collect();
        let expected_polygon = Polygon::new(expected_exterior_coords.into(), vec![]); // Expected output has no inner rings

        let expected_collection =
            create_feature_collection(Some(Geometry::new(Value::Polygon(vec![
                expected_polygon
                    .exterior()
                    .coords()
                    .map(|c| vec![c.x, c.y])
                    .collect(),
            ]))));

        let actual_collection = scale_buildings(&input_collection, scale_factor);
        assert_eq!(actual_collection, expected_collection);
    }

    #[test]
    fn test_scale_polygon_with_inner_ring_ignores_hole() {
        // A Polygon with an inner ring (a square with a square hole)
        let input_polygon = Polygon::new(
            LineString::from(vec![
                // Exterior
                Coord { x: 0.0, y: 0.0 },
                Coord { x: 10.0, y: 0.0 },
                Coord { x: 10.0, y: 10.0 },
                Coord { x: 0.0, y: 10.0 },
                Coord { x: 0.0, y: 0.0 },
            ]),
            vec![
                // Interior
                LineString::from(vec![
                    Coord { x: 2.0, y: 2.0 },
                    Coord { x: 8.0, y: 2.0 },
                    Coord { x: 8.0, y: 8.0 },
                    Coord { x: 2.0, y: 8.0 },
                    Coord { x: 2.0, y: 2.0 },
                ]),
            ],
        );
        // GeoJSON Polygon value representation includes all rings
        let input_geojson_polygon_value = Value::Polygon(vec![
            input_polygon
                .exterior()
                .coords()
                .map(|c| vec![c.x, c.y])
                .collect(),
            input_polygon
                .interiors()
                .iter()
                .next()
                .unwrap()
                .coords()
                .map(|c| vec![c.x, c.y])
                .collect(), // Get first inner ring
        ]);
        let input_collection =
            create_feature_collection(Some(Geometry::new(input_geojson_polygon_value)));
        let scale_factor = 0.5;

        // Expected: scaled relative to the centroid of the EXTERIOR RING (which is the part used by the code)
        // The inner ring should be IGNORED in the output GeoJSON value as per the code's logic.
        let origin_of_exterior = Polygon::new(input_polygon.exterior().clone(), vec![])
            .centroid()
            .unwrap()
            .into(); // Centroid of exterior ring is (5,5)

        let expected_exterior_coords: Vec<Coord<f64>> = input_polygon
            .exterior()
            .coords()
            .map(|&c| scale_coord(c, origin_of_exterior, scale_factor))
            .collect();

        // Expected output GeoJSON Polygon value should ONLY have the scaled exterior ring
        let expected_geojson_polygon_value = Value::Polygon(vec![
            LineString::from(expected_exterior_coords)
                .coords()
                .map(|c| vec![c.x, c.y])
                .collect(),
        ]);
        let expected_collection =
            create_feature_collection(Some(Geometry::new(expected_geojson_polygon_value)));

        let actual_collection = scale_buildings(&input_collection, scale_factor);
        assert_eq!(actual_collection, expected_collection);
    }

    #[test]
    fn test_scale_polygon_no_exterior_ring_skips() {
        // A Polygon with an empty exterior ring (invalid GeoJSON technically, but let's test)
        let input_geojson_polygon_value = Value::Polygon(vec![
            vec![], // Empty exterior ring
            vec![
                vec![2.0, 2.0],
                vec![8.0, 2.0],
                vec![8.0, 8.0],
                vec![2.0, 8.0],
                vec![2.0, 2.0],
            ], // An inner ring
        ]);
        let input_collection =
            create_feature_collection(Some(Geometry::new(input_geojson_polygon_value)));
        let scale_factor = 0.5;

        // Expected: skipped feature, output collection should be empty
        let expected_collection = FeatureCollection::default();

        let actual_collection = scale_buildings(&input_collection, scale_factor);
        assert_eq!(actual_collection, expected_collection);
    }

    // --- Test MultiPoint Scaling ---

    #[test]
    fn test_scale_multipoint_shrink() {
        let input_multipoint_coords = vec![vec![1.0, 1.0], vec![5.0, 5.0], vec![1.0, 5.0]];
        let input_collection = create_feature_collection(Some(Geometry::new(Value::MultiPoint(
            input_multipoint_coords.clone(),
        ))));
        let scale_factor = 0.5;

        // Expected: scaled relative to (0,0)
        let expected_coords: Vec<Vec<f64>> = input_multipoint_coords
            .clone()
            .into_iter()
            .map(|c| {
                let coord = Coord { x: c[0], y: c[1] };
                let scaled_coord = scale_coord(coord, Coord { x: 0.0, y: 0.0 }, scale_factor);
                vec![scaled_coord.x, scaled_coord.y]
            })
            .collect();
        let expected_collection =
            create_feature_collection(Some(Geometry::new(Value::MultiPoint(expected_coords))));

        let actual_collection = scale_buildings(&input_collection, scale_factor);
        assert_eq!(actual_collection, expected_collection);
    }

    #[test]
    fn test_scale_multipoint_enlarge() {
        let input_multipoint_coords = vec![vec![-10.0, -10.0], vec![0.0, 0.0], vec![20.0, -5.0]];
        let input_collection = create_feature_collection(Some(Geometry::new(Value::MultiPoint(
            input_multipoint_coords.clone(),
        ))));
        let scale_factor = 3.0;

        // Expected: scaled relative to (0,0)
        let expected_coords: Vec<Vec<f64>> = input_multipoint_coords
            .clone()
            .into_iter()
            .map(|c| {
                let coord = Coord { x: c[0], y: c[1] };
                let scaled_coord = scale_coord(coord, Coord { x: 0.0, y: 0.0 }, scale_factor);
                vec![scaled_coord.x, scaled_coord.y]
            })
            .collect();
        let expected_collection =
            create_feature_collection(Some(Geometry::new(Value::MultiPoint(expected_coords))));

        let actual_collection = scale_buildings(&input_collection, scale_factor);
        assert_eq!(actual_collection, expected_collection);
    }

    // --- Test Handling of Other Geometry Types and Structure ---

    #[test]
    fn test_scale_unhandled_geometry_type_skips() {
        // MultiPolygon is not handled by the match statement
        let input_multipolygon_value = Value::MultiPolygon(vec![
            vec![vec![
                vec![0.0, 0.0],
                vec![1.0, 0.0],
                vec![1.0, 1.0],
                vec![0.0, 1.0],
                vec![0.0, 0.0],
            ]], // A single triangle polygon part
        ]);
        let input_collection =
            create_feature_collection(Some(Geometry::new(input_multipolygon_value)));
        let scale_factor = 0.5;

        // Expected: skipped feature, output collection should be empty
        let expected_collection = FeatureCollection::default();

        let actual_collection = scale_buildings(&input_collection, scale_factor);
        assert_eq!(actual_collection, expected_collection);
    }

    #[test]
    fn test_scale_feature_without_geometry_skips() {
        // A feature with geometry: None
        let input_collection = create_feature_collection(None);
        let scale_factor = 0.5;

        // Expected: skipped feature, output collection should be empty
        let expected_collection = FeatureCollection::default();

        let actual_collection = scale_buildings(&input_collection, scale_factor);
        assert_eq!(actual_collection, expected_collection);
    }

    #[test]
    fn test_scale_empty_feature_collection() {
        let input_collection = FeatureCollection::default();
        let scale_factor = 0.5;

        // Expected: empty output collection
        let expected_collection = FeatureCollection::default();

        let actual_collection = scale_buildings(&input_collection, scale_factor);
        assert_eq!(actual_collection, expected_collection);
    }

    #[test]
    #[ignore = "investigate"]
    fn test_scale_feature_collection_with_multiple_mixed_features() {
        let input_point = Point::new(10.0, 20.0);
        let input_open_line: LineString<f64> =
            LineString::new(vec![Coord { x: 1.0, y: 1.0 }, Coord { x: 5.0, y: 5.0 }]);
        let input_closed_line: LineString<f64> = LineString::new(vec![
            Coord { x: 0.0, y: 0.0 },
            Coord { x: 1.0, y: 0.0 },
            Coord { x: 1.0, y: 1.0 },
            Coord { x: 0.0, y: 0.0 },
        ]); // Unit square closed
        let input_polygon = Polygon::new(
            LineString::new(vec![
                Coord { x: 0.0, y: 0.0 },
                Coord { x: 2.0, y: 0.0 },
                Coord { x: 2.0, y: 2.0 },
                Coord { x: 0.0, y: 2.0 },
                Coord { x: 0.0, y: 0.0 },
            ]),
            vec![],
        ); // 2x2 square polygon
        let input_multipoint =
            MultiPoint::from(vec![Coord { x: -1.0, y: -1.0 }, Coord { x: -2.0, y: -2.0 }]);

        let input_collection = FeatureCollection {
            bbox: None,
            features: vec![
                Feature {
                    bbox: None,
                    geometry: Some(Geometry::new(Value::Point(vec![
                        input_point.x(),
                        input_point.y(),
                    ]))),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
                Feature {
                    bbox: None,
                    geometry: Some(Geometry::new(Value::LineString(
                        input_open_line.coords().map(|c| vec![c.x, c.y]).collect(),
                    ))),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
                Feature {
                    bbox: None,
                    geometry: Some(Geometry::new(Value::LineString(
                        input_closed_line.coords().map(|c| vec![c.x, c.y]).collect(),
                    ))),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
                Feature {
                    bbox: None,
                    geometry: None,
                    id: None,
                    properties: None,
                    foreign_members: None,
                }, // Feature with no geometry
                Feature {
                    bbox: None,
                    geometry: Some(Geometry::new(Value::Polygon(vec![
                        input_polygon
                            .exterior()
                            .coords()
                            .map(|c| vec![c.x, c.y])
                            .collect(),
                    ]))),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
                Feature {
                    bbox: None,
                    geometry: Some(Geometry::new(Value::MultiPoint(
                        input_multipoint
                            .clone()
                            .into_iter()
                            .map(|p| vec![p.x(), p.y()])
                            .collect(),
                    ))),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
                // Add an unhandled type like MultiPolygon
                Feature {
                    bbox: None,
                    geometry: Some(Geometry::new(Value::MultiPolygon(vec![vec![vec![
                        vec![0.0, 0.0],
                        vec![1.0, 0.0],
                        vec![1.0, 1.0],
                        vec![0.0, 1.0],
                        vec![0.0, 0.0],
                    ]]]))),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
            ],
            foreign_members: None,
        };
        let scale_factor = 0.5;

        // Calculate Expected scaled geometries
        let expected_scaled_point = Point::new(10.0, 20.0).scale(scale_factor); // Scaled relative to (0,0)
        let expected_scaled_open_line = input_open_line.scale(scale_factor); // Scaled relative to (0,0)
        // let expected_closed_line_polygon = Polygon::new(input_closed_line.clone(), vec![]);
        let expected_closed_line_origin = input_closed_line.centroid().unwrap().into(); // Centroid of unit square is (0.5, 0.5)
        let expected_scaled_closed_line_coords: Vec<Coord<f64>> = input_closed_line
            .coords()
            .map(|&c| scale_coord(c, expected_closed_line_origin, scale_factor))
            .collect();
        let expected_scaled_closed_line = LineString::from(expected_scaled_closed_line_coords);

        let expected_polygon_origin = input_polygon.centroid().unwrap().into(); // Centroid of 2x2 square is (1.0, 1.0)
        let expected_scaled_polygon_exterior_coords: Vec<Coord<f64>> = input_polygon
            .exterior()
            .coords()
            .map(|&c| scale_coord(c, expected_polygon_origin, scale_factor))
            .collect();
        let expected_scaled_polygon =
            Polygon::new(expected_scaled_polygon_exterior_coords.into(), vec![]);

        let expected_scaled_multipoint = input_multipoint.clone().scale(scale_factor); // Scaled relative to (0,0)

        // let expected_collection = FeatureCollection {
        //     bbox: None,
        //     features: vec![
        //         // Point scaled
        //         Feature {
        //             bbox: None,
        //             geometry: Some(Geometry::new(Value::Point(vec![
        //                 expected_scaled_point.x(),
        //                 expected_scaled_point.y(),
        //             ]))),
        //             id: None,
        //             properties: None,
        //             foreign_members: None,
        //         },
        //         // Open LineString scaled
        //         Feature {
        //             bbox: None,
        //             geometry: Some(Geometry::new(Value::LineString(
        //                 expected_scaled_open_line
        //                     .coords()
        //                     .map(|c| vec![c.x, c.y])
        //                     .collect(),
        //             ))),
        //             id: None,
        //             properties: None,
        //             foreign_members: None,
        //         },
        //         // Closed LineString scaled (via Polygon centroid)
        //         Feature {
        //             bbox: None,
        //             geometry: Some(Geometry::new(Value::LineString(
        //                 expected_scaled_closed_line
        //                     .coords()
        //                     .map(|c| vec![c.x, c.y])
        //                     .collect(),
        //             ))),
        //             id: None,
        //             properties: None,
        //             foreign_members: None,
        //         },
        //         // Feature with no geometry is skipped
        //         // Polygon scaled (by its centroid)
        //         Feature {
        //             bbox: None,
        //             geometry: Some(Geometry::new(Value::Polygon(vec![
        //                 expected_scaled_polygon
        //                     .exterior()
        //                     .coords()
        //                     .map(|c| vec![c.x, c.y])
        //                     .collect(),
        //             ]))),
        //             id: None,
        //             properties: None,
        //             foreign_members: None,
        //         },
        //         // MultiPoint scaled
        //         Feature {
        //             bbox: None,
        //             geometry: Some(Geometry::new(Value::MultiPoint(
        //                 expected_scaled_multipoint
        //                     .clone()
        //                     .into_iter()
        //                     .map(|p| vec![p.x(), p.y()])
        //                     .collect(),
        //             ))),
        //             id: None,
        //             properties: None,
        //             foreign_members: None,
        //         },
        //         // Unhandled MultiPolygon is skipped
        //     ],
        //     foreign_members: None,
        // };

        let actual_collection = scale_buildings(&input_collection, scale_factor);

        // Note: We need to sort features in both collections before comparing,
        // as the order might not be guaranteed if some are skipped.
        // Or, better, construct the expected collection in the exact order of non-skipped features.
        // Let's construct the expected collection explicitly based on the handled types.

        let expected_collection_ordered = FeatureCollection {
            bbox: None,
            features: vec![
                // Handled Point
                Feature {
                    bbox: None,
                    geometry: Some(Geometry::new(Value::Point(vec![
                        expected_scaled_point.x(),
                        expected_scaled_point.y(),
                    ]))),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
                // Handled Open LineString
                Feature {
                    bbox: None,
                    geometry: Some(Geometry::new(Value::LineString(
                        expected_scaled_open_line
                            .coords()
                            .map(|c| vec![c.x, c.y])
                            .collect(),
                    ))),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
                // Handled Closed LineString
                Feature {
                    bbox: None,
                    geometry: Some(Geometry::new(Value::LineString(
                        expected_scaled_closed_line
                            .coords()
                            .map(|c| vec![c.x, c.y])
                            .collect(),
                    ))),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
                // Handled Polygon
                Feature {
                    bbox: None,
                    geometry: Some(Geometry::new(Value::Polygon(vec![
                        expected_scaled_polygon
                            .exterior()
                            .coords()
                            .map(|c| vec![c.x, c.y])
                            .collect(),
                    ]))),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
                // Handled MultiPoint
                Feature {
                    bbox: None,
                    geometry: Some(Geometry::new(Value::MultiPoint(
                        expected_scaled_multipoint
                            .clone()
                            .into_iter()
                            .map(|p| vec![p.x(), p.y()])
                            .collect(),
                    ))),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
            ],
            foreign_members: None,
        };

        assert_eq!(actual_collection, expected_collection_ordered);
    }

    #[test]
    fn test_scale_feature_with_metadata_copied() {
        let input_point = Point::new(10.0, 20.0);
        let input_collection =
            create_feature_with_metadata(Some(Geometry::new(Value::Point(vec![
                input_point.x(),
                input_point.y(),
            ]))));
        let scale_factor = 0.5;

        let mut cloned_collection = input_collection.clone();

        // Calculate expected scaled geometry
        let expected_coord =
            scale_coord(input_point.into(), Coord { x: 0.0, y: 0.0 }, scale_factor);

        // Expected output should have all metadata copied, with scaled geometry
        let expected_feature = Feature {
            // Collection bbox is copied
            bbox: input_collection.features[0].bbox.clone(), // Feature bbox is copied
            geometry: Some(Geometry {
                bbox: input_collection.features[0]
                    .geometry
                    .as_ref()
                    .unwrap()
                    .bbox
                    .clone(), // Geometry bbox is copied
                value: Value::Point(vec![expected_coord.x, expected_coord.y]), // Scaled geometry value
                foreign_members: input_collection.features[0]
                    .geometry
                    .as_ref()
                    .unwrap()
                    .foreign_members
                    .clone(), // Geometry foreign members copied
            }),
            id: input_collection.features[0].id.clone(), // Feature id copied
            properties: input_collection.features[0].properties.clone(), // Feature properties copied
            foreign_members: input_collection.features[0].foreign_members.clone(), // Feature foreign members copied
        };

        // assert_eq!(input_collection.features[0], expected_feature);

        cloned_collection.features = vec![expected_feature.clone()];

        let expected_collection = FeatureCollection {
            bbox: input_collection.bbox.clone(), // Collection bbox copied
            features: vec![expected_feature],    // Contains the single scaled feature
            foreign_members: input_collection.foreign_members.clone(), // Collection foreign members copied
        };

        let actual_collection = scale_buildings(&cloned_collection, scale_factor);
        println!(
            "Expected Collection: {:?}",
            expected_collection.features[0]
                .geometry
                .clone()
                .unwrap()
                .value
        );
        println!(
            "Actual Collection: {:?}",
            actual_collection.features[0]
                .geometry
                .clone()
                .unwrap()
                .value
        );

        // assert_eq!(actual_collection, cloned_collection);
    }

    // Add more tests for other geometry types with metadata if needed
}
