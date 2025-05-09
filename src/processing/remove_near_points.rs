use geo::{CoordsIter, Point};
use rstar::RTree;

use crate::utils::geometry::{GeoFeature, GeoFeatureCollection, GeoGeometry};

#[allow(dead_code)]
pub fn remove_near_points(collection: &GeoFeatureCollection) -> GeoFeatureCollection {
    let mut coll = GeoFeatureCollection {
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
                match geometry {
                    GeoGeometry::LineString(coordinates) => {
                        // Apply filtering logic to the LineString coordinates
                        let mut filtered_coordinates =
                            Vec::with_capacity(coordinates.coords_count());
                        // Create an R-tree to store the points
                        let mut rtree: RTree<Point<f64>> = RTree::new();
                        // Add first coordinate as reference point
                        if let Some(first) = coordinates.coords().next() {
                            filtered_coordinates.push(first.clone());
                            rtree.insert(Point::new(first.x, first.y));
                        }
                        // Compare each point with all previously kept points
                        for coord in coordinates.coords().skip(1) {
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
                        coll.features.push(GeoFeature {
                            bbox: feature.bbox.clone(),
                            foreign_members: feature.foreign_members.clone(),
                            id: feature.id.clone(),
                            properties: feature.properties.clone(),
                            geometry: Some(GeoGeometry::LineString(
                                filtered_coordinates
                                    .into_iter()
                                    .map(|coord| Point::new(coord.x, coord.y))
                                    .collect(),
                            )),
                        });
                    }
                    GeoGeometry::Point(point) => coll.features.push(GeoFeature {
                        bbox: feature.bbox.clone(),
                        geometry: Some(GeoGeometry::Point(point.clone())),
                        id: feature.id.clone(),
                        properties: feature.properties.clone(),
                        foreign_members: feature.foreign_members.clone(),
                    }),
                    GeoGeometry::MultiPoint(coordinates) => {
                        // Apply filtering logic to the MultiPoint coordinates
                        let mut filtered_coordinates =
                            Vec::with_capacity(coordinates.coords_count());
                        // Create an R-tree to store the points
                        let mut rtree: RTree<Point<f64>> = RTree::new();
                        // Add first coordinate as reference point
                        if let Some(first) = coordinates.coords_iter().next() {
                            filtered_coordinates.push(first.clone());
                            rtree.insert(Point::new(first.x, first.y));
                        }
                        // Compare each point with all previously kept points
                        for coord in coordinates.coords_iter() {
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
                        coll.features.push(GeoFeature {
                            id: feature.id.clone(),
                            properties: feature.properties.clone(),
                            foreign_members: feature.foreign_members.clone(),
                            bbox: feature.bbox.clone(),
                            geometry: Some(GeoGeometry::MultiPoint(
                                filtered_coordinates
                                    .into_iter()
                                    .map(|coord| Point::new(coord.x, coord.y))
                                    .collect(),
                            )),
                        });
                    }
                    _ => {
                        coll.features.push(feature.clone());
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
    fn create_feature_collection(geometries: Vec<geojson::Geometry>) -> GeoFeatureCollection {
        GeoFeatureCollection {
            bbox: None,
            features: geometries
                .into_iter()
                .map(|value| GeoFeature {
                    bbox: None,
                    geometry: Some(GeoGeometry::from(value)),
                    foreign_members: None,
                    id: None,
                    properties: None,
                })
                .collect(),
            foreign_members: None,
        }
    }

    #[test]
    fn test_empty_collection() {
        let empty_collection = GeoFeatureCollection {
            bbox: None,
            features: vec![],
            foreign_members: None,
        };
        let cleaned = remove_near_points(&empty_collection);
        assert_eq!(cleaned.features.len(), 0);
    }

    #[test]
    fn test_single_point_collection() {
        let collection = create_feature_collection(vec![geojson::Geometry {
            bbox: None,
            value: geojson::Value::Point(vec![0.0, 0.0]),
            foreign_members: None,
        }]);
        let cleaned = remove_near_points(&collection);
        assert_eq!(cleaned.features.len(), 1);
    }

    #[test]
    fn test_identical_points() {
        let collection = create_feature_collection(vec![geojson::Geometry {
            bbox: None,
            value: geojson::Value::MultiPoint(vec![vec![1.0, 1.0], vec![1.0, 1.0], vec![1.0, 1.0]]),
            foreign_members: None,
        }]);
        let cleaned = remove_near_points(&collection);
        assert_eq!(cleaned.features.len(), 1);
    }

    #[test]
    fn test_mixed_geometry_types() {
        let collection = create_feature_collection(vec![
            geojson::Geometry {
                bbox: None,
                value: geojson::Value::Point(vec![0.0, 0.0]),
                foreign_members: None,
            },
            geojson::Geometry {
                bbox: None,
                value: geojson::Value::LineString(vec![
                    vec![1.0, 1.0],
                    vec![1.1, 1.1],
                    vec![2.0, 2.0],
                ]),
                foreign_members: None,
            },
            geojson::Geometry {
                bbox: None,
                value: geojson::Value::MultiPoint(vec![
                    vec![3.0, 3.0],
                    vec![3.1, 3.1],
                    vec![4.0, 4.0],
                ]),
                foreign_members: None,
            },
        ]);
        let cleaned = remove_near_points(&collection);
        assert_eq!(cleaned.features.len(), 3);
        // Verify each geometry type is preserved
        assert!(matches!(
            cleaned.features[0].geometry.as_ref().unwrap(),
            GeoGeometry::Point(_)
        ));
        assert!(matches!(
            cleaned.features[1].geometry.as_ref().unwrap(),
            GeoGeometry::LineString(_)
        ));
        assert!(matches!(
            cleaned.features[2].geometry.as_ref().unwrap(),
            GeoGeometry::MultiPoint(_)
        ));
    }

    #[test]
    fn test_large_coordinate_values() {
        let collection = create_feature_collection(vec![geojson::Geometry {
            bbox: None,
            value: geojson::Value::MultiPoint(vec![
                vec![1000000.0, 1000000.0],
                vec![1000000.1, 1000000.1],
                vec![1000001.0, 1000001.0],
            ]),
            foreign_members: None,
        }]);
        let cleaned = remove_near_points(&collection);
        assert_eq!(cleaned.features.len(), 1);
    }

    #[test]
    fn test_negative_coordinates() {
        let collection = create_feature_collection(vec![geojson::Geometry {
            bbox: None,
            value: geojson::Value::MultiPoint(vec![
                vec![-1.0, -1.0],
                vec![-1.1, -1.1],
                vec![-2.0, -2.0],
            ]),
            foreign_members: None,
        }]);
        let cleaned = remove_near_points(&collection);
        assert_eq!(cleaned.features.len(), 1);
    }

    #[test]
    fn test_zigzag_pattern() {
        let collection = create_feature_collection(vec![geojson::Geometry {
            bbox: None,
            value: geojson::Value::LineString(vec![
                vec![0.0, 0.0],
                vec![0.2, 0.2], // Should be removed
                vec![0.4, 0.0],
                vec![0.6, 0.2], // Should be removed
                vec![0.8, 0.0],
            ]),
            foreign_members: None,
        }]);
        let cleaned = remove_near_points(&collection);
        assert_eq!(cleaned.features.len(), 1);
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
        let collection = GeoFeatureCollection {
            bbox: Some(vec![-1.0, -1.0, 1.0, 1.0]),
            features: vec![feature.into()],
            foreign_members: Some(serde_json::Map::from_iter(vec![(
                "created".to_string(),
                serde_json::Value::String("2023".to_string()),
            )])),
        };

        let cleaned = remove_near_points(&collection);

        assert!(cleaned.bbox.is_some());
        assert!(cleaned.foreign_members.is_some());
        let cleaned_feature = &cleaned.features[0];
        assert!(cleaned_feature.bbox.is_some());
        assert!(cleaned_feature.properties.is_some());
        assert!(cleaned_feature.foreign_members.is_some());
        assert!(cleaned_feature.geometry.is_some());
    }
}
