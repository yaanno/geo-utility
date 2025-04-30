use geo::{Coord, Point};
use geojson::FeatureCollection;
use rstar::RTree;

#[derive(Debug, Clone)]
pub enum Geometry {
    Point(Coord<f64>),
    LineString(Vec<Coord<f64>>),
    MultiPoint(Vec<Coord<f64>>),
}

impl From<geojson::Geometry> for Geometry {
    fn from(geometry: geojson::Geometry) -> Self {
        match geometry.value {
            geojson::Value::Point(coords) => Geometry::Point(Coord {
                x: coords[0],
                y: coords[1],
            }),
            geojson::Value::LineString(coords) => Geometry::LineString(
                coords
                    .into_iter()
                    .map(|coord| Coord {
                        x: coord[0],
                        y: coord[1],
                    })
                    .collect(),
            ),
            geojson::Value::MultiPoint(coords) => Geometry::MultiPoint(
                coords
                    .into_iter()
                    .map(|coord| Coord {
                        x: coord[0],
                        y: coord[1],
                    })
                    .collect(),
            ),
            geojson::Value::Polygon(_) => panic!("Unsupported geometry type"),
            geojson::Value::MultiLineString(_) => panic!("Unsupported geometry type"),
            geojson::Value::MultiPolygon(_) => panic!("Unsupported geometry type"),
            geojson::Value::GeometryCollection(_) => panic!("Unsupported geometry type"),
        }
    }
}

impl Into<geojson::Geometry> for Geometry {
    fn into(self) -> geojson::Geometry {
        let value = match self {
            Geometry::Point(coord) => geojson::Value::Point(vec![coord.x, coord.y]),
            Geometry::LineString(coords) => geojson::Value::LineString(
                coords
                    .into_iter()
                    .map(|coord| vec![coord.x, coord.y])
                    .collect(),
            ),
            Geometry::MultiPoint(coords) => geojson::Value::MultiPoint(
                coords
                    .into_iter()
                    .map(|coord| vec![coord.x, coord.y])
                    .collect(),
            ),
        };
        geojson::Geometry::new(value)
    }
}

#[allow(dead_code)]
pub fn remove_near_points(collection: &FeatureCollection) -> FeatureCollection {
    let mut coll = FeatureCollection {
        bbox: collection.bbox.clone(),
        features: Vec::new(),
        foreign_members: collection.foreign_members.clone(),
    };

    let squared_distance_threshold = 0.3 * 0.3;

    for feature in &collection.features {
        // Pattern match on the geometry type
        match &feature.geometry {
            Some(geometry) => {
                // Convert the geometry to the custom Geometry enum
                match Geometry::from(geometry.clone()) {
                    Geometry::LineString(coordinates) => {
                        // Apply filtering logic to the LineString coordinates
                        let mut filtered_coordinates = Vec::with_capacity(coordinates.len());
                        // Create an R-tree to store the points
                        let mut rtree: RTree<Point<f64>> = RTree::new();
                        // Add first coordinate as reference point
                        if let Some(first) = coordinates.first() {
                            filtered_coordinates.push(first.clone());
                            rtree.insert(Point::new(first.x, first.y));
                        }
                        // Compare each point with all previously kept points
                        for coord in coordinates.iter().skip(1) {
                            let current_point = Point::new(coord.x, coord.y);
                            // Use the R-tree to find neighbors within the distance threshold
                            let neighbors = rtree
                                .locate_within_distance(current_point, squared_distance_threshold);

                            // If no neighbors are found, the point is far enough from existing points, so keep it.
                            if neighbors.count() == 0 {
                                filtered_coordinates.push(coord.clone());
                                rtree.insert(current_point); // Add the kept point to the R-tree
                            }
                        }
                        // Create a new feature with the filtered coordinates
                        coll.features.push(geojson::Feature {
                            bbox: feature.bbox.clone(),
                            geometry: Some(geojson::Geometry {
                                bbox: geometry.bbox.clone(),
                                value: geojson::Value::LineString(
                                    filtered_coordinates
                                        .into_iter()
                                        .map(|coord| vec![coord.x, coord.y])
                                        .collect(),
                                ),
                                foreign_members: geometry.foreign_members.clone(),
                            }),
                            id: feature.id.clone(),
                            properties: feature.properties.clone(),
                            foreign_members: feature.foreign_members.clone(),
                        })
                    }
                    Geometry::Point(point) => coll.features.push(geojson::Feature {
                        bbox: feature.bbox.clone(),
                        geometry: Some(geojson::Geometry {
                            bbox: geometry.bbox.clone(),
                            value: geojson::Value::Point(vec![point.x, point.y]),
                            foreign_members: geometry.foreign_members.clone(),
                        }),
                        id: feature.id.clone(),
                        properties: feature.properties.clone(),
                        foreign_members: feature.foreign_members.clone(),
                    }),
                    Geometry::MultiPoint(coordinates) => {
                        // Apply filtering logic to the MultiPoint coordinates
                        let mut filtered_coordinates = Vec::with_capacity(coordinates.len());
                        // Create an R-tree to store the points
                        let mut rtree: RTree<Point<f64>> = RTree::new();
                        // Add first coordinate as reference point
                        if let Some(first) = coordinates.first() {
                            filtered_coordinates.push(first.clone());
                            rtree.insert(Point::new(first.x, first.y));
                        }
                        // Compare each point with all previously kept points
                        for coord in coordinates.iter().skip(1) {
                            let current_point = Point::new(coord.x, coord.y);
                            // Use the R-tree to find neighbors within the distance threshold
                            let neighbors = rtree
                                .locate_within_distance(current_point, squared_distance_threshold);
                            // If no neighbors are found, the point is far enough from existing points, so keep it.
                            if neighbors.count() == 0 {
                                filtered_coordinates.push(coord.clone());
                                rtree.insert(current_point); // Add the kept point to the R-tree
                            }
                        }
                        // Create a new feature with the filtered coordinates
                        coll.features.push(geojson::Feature {
                            bbox: feature.bbox.clone(),
                            geometry: Some(geojson::Geometry {
                                bbox: geometry.bbox.clone(),
                                value: geojson::Value::MultiPoint(
                                    filtered_coordinates
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
                }
            }
            None => {
                println!("Feature has no geometry");
                coll.features.push(feature.clone());
            }
        }
    }
    coll
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a feature collection
    fn create_feature_collection(geometries: Vec<geojson::Value>) -> FeatureCollection {
        FeatureCollection {
            bbox: None,
            features: geometries
                .into_iter()
                .map(|value| geojson::Feature {
                    bbox: None,
                    geometry: Some(geojson::Geometry {
                        bbox: None,
                        value,
                        foreign_members: None,
                    }),
                    id: None,
                    properties: None,
                    foreign_members: None,
                })
                .collect(),
            foreign_members: None,
        }
    }

    #[test]
    fn test_empty_collection() {
        let empty_collection = FeatureCollection {
            bbox: None,
            features: vec![],
            foreign_members: None,
        };
        let cleaned = remove_near_points(&empty_collection);
        assert_eq!(cleaned.features.len(), 0);
    }

    #[test]
    fn test_single_point_collection() {
        let collection = create_feature_collection(vec![geojson::Value::Point(vec![0.0, 0.0])]);
        let cleaned = remove_near_points(&collection);
        assert_eq!(cleaned.features.len(), 1);
    }

    #[test]
    fn test_identical_points() {
        let collection = create_feature_collection(vec![geojson::Value::MultiPoint(vec![
            vec![1.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0, 1.0],
        ])]);
        let cleaned = remove_near_points(&collection);
        assert_eq!(
            cleaned.features[0].geometry.as_ref().unwrap().value,
            geojson::Value::MultiPoint(vec![vec![1.0, 1.0]])
        );
    }

    #[test]
    fn test_boundary_distance() {
        // Test points exactly at the threshold distance (0.3)
        let collection = create_feature_collection(vec![geojson::Value::MultiPoint(vec![
            vec![0.0, 0.0],
            vec![0.3, 0.0], // Exactly at threshold
            vec![0.0, 0.3], // Exactly at threshold
            vec![0.2, 0.2], // Just under threshold (≈0.28)
        ])]);
        let cleaned = remove_near_points(&collection);
        assert_eq!(
            cleaned.features[0].geometry.as_ref().unwrap().value,
            geojson::Value::MultiPoint(vec![vec![0.0, 0.0]])
        );
    }

    #[test]
    fn test_mixed_geometry_types() {
        let collection = create_feature_collection(vec![
            geojson::Value::Point(vec![0.0, 0.0]),
            geojson::Value::LineString(vec![vec![1.0, 1.0], vec![1.1, 1.1], vec![2.0, 2.0]]),
            geojson::Value::MultiPoint(vec![vec![3.0, 3.0], vec![3.1, 3.1], vec![4.0, 4.0]]),
        ]);
        let cleaned = remove_near_points(&collection);
        assert_eq!(cleaned.features.len(), 3);
        // Verify each geometry type is preserved
        assert!(matches!(
            cleaned.features[0].geometry.as_ref().unwrap().value,
            geojson::Value::Point(_)
        ));
        assert!(matches!(
            cleaned.features[1].geometry.as_ref().unwrap().value,
            geojson::Value::LineString(_)
        ));
        assert!(matches!(
            cleaned.features[2].geometry.as_ref().unwrap().value,
            geojson::Value::MultiPoint(_)
        ));
    }

    #[test]
    fn test_large_coordinate_values() {
        let collection = create_feature_collection(vec![geojson::Value::MultiPoint(vec![
            vec![1000000.0, 1000000.0],
            vec![1000000.1, 1000000.1],
            vec![1000001.0, 1000001.0],
        ])]);
        let cleaned = remove_near_points(&collection);
        assert_eq!(
            cleaned.features[0].geometry.as_ref().unwrap().value,
            geojson::Value::MultiPoint(vec![
                vec![1000000.0, 1000000.0],
                vec![1000001.0, 1000001.0],
            ])
        );
    }

    #[test]
    fn test_negative_coordinates() {
        let collection = create_feature_collection(vec![geojson::Value::MultiPoint(vec![
            vec![-1.0, -1.0],
            vec![-1.1, -1.1],
            vec![-2.0, -2.0],
        ])]);
        let cleaned = remove_near_points(&collection);
        assert_eq!(
            cleaned.features[0].geometry.as_ref().unwrap().value,
            geojson::Value::MultiPoint(vec![vec![-1.0, -1.0], vec![-2.0, -2.0],])
        );
    }

    #[test]
    fn test_zigzag_pattern() {
        let collection = create_feature_collection(vec![geojson::Value::LineString(vec![
            vec![0.0, 0.0],
            vec![0.2, 0.2], // Should be removed
            vec![0.4, 0.0],
            vec![0.6, 0.2], // Should be removed
            vec![0.8, 0.0],
        ])]);
        let cleaned = remove_near_points(&collection);
        assert_eq!(
            cleaned.features[0].geometry.as_ref().unwrap().value,
            geojson::Value::LineString(vec![vec![0.0, 0.0], vec![0.4, 0.0], vec![0.8, 0.0],])
        );
    }

    #[test]
    fn test_features_with_metadata() {
        let feature = geojson::Feature {
            bbox: Some(vec![-1.0, -1.0, 1.0, 1.0]),
            geometry: Some(geojson::Geometry {
                bbox: Some(vec![-1.0, -1.0, 1.0, 1.0]),
                value: geojson::Value::MultiPoint(vec![
                    vec![0.0, 0.0],
                    vec![0.1, 0.1],
                    vec![1.0, 1.0],
                ]),
                foreign_members: Some(serde_json::Map::from_iter(vec![(
                    "source".to_string(),
                    serde_json::Value::String("GPS".to_string()),
                )])),
            }),
            // id: Some(serde_json::Value::String("1".to_string())),
            id: None,
            properties: Some(serde_json::Map::from_iter(vec![(
                "name".to_string(),
                serde_json::Value::String("Test Feature".to_string()),
            )])),
            foreign_members: Some(serde_json::Map::from_iter(vec![(
                "timestamp".to_string(),
                serde_json::Value::String("2023-01-01".to_string()),
            )])),
        };
        let collection = FeatureCollection {
            bbox: Some(vec![-1.0, -1.0, 1.0, 1.0]),
            features: vec![feature],
            foreign_members: Some(serde_json::Map::from_iter(vec![(
                "created".to_string(),
                serde_json::Value::String("2023".to_string()),
            )])),
        };

        let cleaned = remove_near_points(&collection);

        // Verify metadata preservation
        assert!(cleaned.bbox.is_some());
        assert!(cleaned.foreign_members.is_some());
        let cleaned_feature = &cleaned.features[0];
        assert!(cleaned_feature.bbox.is_some());
        // assert!(cleaned_feature.id.is_some());
        assert!(cleaned_feature.properties.is_some());
        assert!(cleaned_feature.foreign_members.is_some());
        assert!(cleaned_feature.geometry.as_ref().unwrap().bbox.is_some());
        assert!(
            cleaned_feature
                .geometry
                .as_ref()
                .unwrap()
                .foreign_members
                .is_some()
        );
    }
}
