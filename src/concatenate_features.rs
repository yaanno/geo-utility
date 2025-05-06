use geo::{Coord, LineString, Point};
use geojson::{Feature, FeatureCollection, Geometry, Value};
use rstar::{Point as RStarPoint, PointDistance, RTree, RTreeObject};
use serde_json::{Value as JsonValue, json}; // Import json! macro and JsonValue

// Define the struct to be stored in the R-tree
#[derive(Debug, Clone, Copy, PartialEq)] // Add PartialEq for assertions in tests
struct LineStringEndpoint {
    // Use a fixed-size array for the point, compatible with rstar::Point
    point: [f64; 2],
    // Index back to the original feature in the `line_strings` vector
    feature_idx: usize,
    // Is this the start or end point of the original line?
    is_start: bool,
}

// Implement RTreeObject for our endpoint struct
impl RTreeObject for LineStringEndpoint {
    type Envelope = rstar::AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        Self::Envelope::from_point(self.point)
    }
}

// Mock properties_match function for testing purposes
// In a real implementation, this would parse the nested properties.properties
fn properties_match(_props1: Option<&JsonValue>, _props2: Option<&JsonValue>) -> bool {
    // This is a placeholder. Implement your actual property comparison logic here.
    // For the current tests, this function is not called by the code being tested,
    // but it's included as a reminder for the next steps.
    true // Assume properties always match for this mock
}

pub fn concatenate_features(collection: &FeatureCollection) -> FeatureCollection {
    if collection.features.is_empty() {
        // Correctly return an empty FeatureCollection based on the input structure
        return FeatureCollection {
            bbox: collection.bbox.clone(), // Keep bbox if present
            features: Vec::new(),          // Empty features vector
            foreign_members: collection.foreign_members.clone(), // Keep foreign_members
        };
    }

    // Vector to hold only the LineString features
    let mut line_strings: Vec<Feature> = Vec::new();
    // Vector to hold non-LineString features
    let mut other_features: Vec<Feature> = Vec::new();

    // Build the list of LineString features and other features
    for feature in &collection.features {
        if let Some(geometry) = &feature.geometry {
            if let Value::LineString(coords) = &geometry.value {
                // Ensure line has at least 2 points
                if coords.len() >= 2 {
                    line_strings.push(feature.clone());
                } else {
                    // Handle degenerate LineStrings by treating them as other_features
                    other_features.push(feature.clone());
                }
            } else {
                other_features.push(feature.clone());
            }
        } else {
            // Features with no geometry also go to other_features
            other_features.push(feature.clone());
        }
    }

    // Now we have `line_strings` containing only valid LineStrings

    // Create the R-tree
    let mut tree: RTree<LineStringEndpoint> = RTree::new();

    // Create the status tracker for original LineString features
    // This vector's length should match the number of valid LineStrings found
    let mut is_merged: Vec<bool> = vec![false; line_strings.len()];

    // Populate the R-tree with endpoints of the LineString features
    for (index, feature) in line_strings.iter().enumerate() {
        if let Some(Value::LineString(coords)) = &feature.geometry.as_ref().map(|g| &g.value) {
            let start_coord = &coords[0]; // coords is Vec<Vec<f64>>, coords[0] is Vec<f64>
            let end_coord = &coords[coords.len() - 1];

            // Convert Vec<f64> to [f64; 2] for R-tree point
            // Added error handling in case the inner Vec<f64> is not length 2
            if let (Ok(start_point), Ok(end_point)) =
                (start_coord.clone().try_into(), end_coord.clone().try_into())
            {
                tree.insert(LineStringEndpoint {
                    point: start_point,
                    feature_idx: index,
                    is_start: true,
                });
                tree.insert(LineStringEndpoint {
                    point: end_point,
                    feature_idx: index,
                    is_start: false,
                });
            } else {
                // This case should ideally not happen with valid GeoJSON LineStrings,
                // but it's good practice to handle it.
                eprintln!(
                    "Warning: Coordinate in LineString not in [x, y] format for feature index {}",
                    index
                );
                // Decide how to handle this - maybe skip this feature or add to other_features
                // For now, we just print a warning and continue. The feature is already in line_strings.
            }
        }
    }

    // Now we have the R-tree built with endpoints of the LineString features
    // and the status tracker initialized.
    // --- The next steps would be the iterative merging loop using the R-tree ---
    
    // Placeholder for the rest of the function that performs the merging
    println!("R-tree built with {} points", tree.size());
    println!("Found {} valid LineStrings", line_strings.len());
    println!(
        "Found {} other features (including degenerate LineStrings)",
        other_features.len()
    );
    println!("is_merged vector initialized with size {}", is_merged.len());

    // For now, just return the other features, as the merging logic is not implemented yet
    // In the final version, this would include the merged LineStrings as well.
    // We will return a FeatureCollection containing the original LineStrings
    // and other features for testing purposes, as the merging isn't done yet.
    let mut result_features = line_strings; // Start with the filtered LineStrings
    result_features.extend(other_features); // Add the other features

    FeatureCollection {
        bbox: collection.bbox.clone(),
        features: result_features, // Return all features, separated by type
        foreign_members: collection.foreign_members.clone(),
    }
}

// --- Test Suite ---
#[cfg(test)]
mod tests {
    use super::*; // Import items from the outer scope

    // Helper function to create a simple LineString feature
    fn create_line_string_feature(
        coords: Vec<Vec<f64>>,
        _properties: Option<JsonValue>,
    ) -> Feature {
        Feature {
            bbox: None,
            geometry: Some(Geometry::new(Value::LineString(coords))),
            id: None,
            properties: None,
            foreign_members: None,
        }
    }

    // Helper function to create a simple Point feature
    fn create_point_feature(coord: Vec<f64>, _properties: Option<JsonValue>) -> Feature {
        Feature {
            bbox: None,
            geometry: Some(Geometry::new(Value::Point(coord))),
            id: None,
            properties: None,
            foreign_members: None,
        }
    }

    // Helper function to create a feature with no geometry
    fn create_nogeometry_feature(_properties: Option<JsonValue>) -> Feature {
        Feature {
            bbox: None,
            geometry: None,
            id: None,
            properties: None,
            foreign_members: None,
        }
    }

    #[test]
    fn test_empty_feature_collection() {
        let collection = FeatureCollection {
            bbox: None,
            features: Vec::new(),
            foreign_members: None,
        };
        let result = concatenate_features(&collection);
        assert_eq!(result.features.len(), 0);
        assert_eq!(result.bbox, None);
        assert_eq!(result.foreign_members, None);
    }

    #[test]
    fn test_only_other_features() {
        let collection = FeatureCollection {
            bbox: None,
            features: vec![
                create_point_feature(vec![0.0, 0.0], None),
                create_point_feature(vec![1.0, 1.0], None),
                create_nogeometry_feature(None),
            ],
            foreign_members: None,
        };
        let result = concatenate_features(&collection);
        // All features should be in the output as 'other_features'
        assert_eq!(result.features.len(), 3);
        // Since no LineStrings were found, line_strings should be empty
        // The R-tree size check is not directly possible from the returned FC,
        // but we know no LineStrings means no R-tree points.
    }

    #[test]
    fn test_only_valid_linestrings() {
        let collection = FeatureCollection {
            bbox: None,
            features: vec![
                create_line_string_feature(vec![vec![0.0, 0.0], vec![1.0, 1.0]], None),
                create_line_string_feature(vec![vec![2.0, 2.0], vec![3.0, 3.0]], None),
            ],
            foreign_members: None,
        };
        let result = concatenate_features(&collection);
        // All features should be in the output as LineStrings
        assert_eq!(result.features.len(), 2);
        // In the actual function logic (not just the return value for this test),
        // line_strings.len() would be 2, is_merged.len() would be 2,
        // and the R-tree size would be 4 (2 endpoints per line).
    }

    #[test]
    fn test_mixed_features() {
        let collection = FeatureCollection {
            bbox: None,
            features: vec![
                create_line_string_feature(vec![vec![0.0, 0.0], vec![1.0, 1.0]], None), // LS 1
                create_point_feature(vec![10.0, 10.0], None),                           // Point 1
                create_line_string_feature(vec![vec![2.0, 2.0], vec![3.0, 3.0]], None), // LS 2
                create_nogeometry_feature(None), // NoGeometry 1
                create_line_string_feature(vec![vec![4.0, 4.0], vec![5.0, 5.0]], None), // LS 3
            ],
            foreign_members: None,
        };
        let result = concatenate_features(&collection);
        // Expected output features: 3 LineStrings + 2 other features = 5
        assert_eq!(result.features.len(), 5);

        // In the actual function logic (not just the return value for this test):
        // line_strings.len() would be 3
        // other_features.len() would be 2
        // is_merged.len() would be 3
        // R-tree size would be 6 (2 endpoints per valid LineString)
    }

    #[test]
    fn test_degenerate_linestring() {
        let collection = FeatureCollection {
            bbox: None,
            features: vec![
                create_line_string_feature(vec![vec![0.0, 0.0], vec![1.0, 1.0]], None), // Valid LS
                create_line_string_feature(vec![vec![2.0, 2.0]], None), // Degenerate LS (only 1 point)
                create_line_string_feature(vec![], None),               // Degenerate LS (0 points)
            ],
            foreign_members: None,
        };
        let result = concatenate_features(&collection);
        // Expected output features: 1 valid LS + 2 degenerate LS (as other) = 3
        assert_eq!(result.features.len(), 3);

        // In the actual function logic:
        // line_strings.len() would be 1 (only the first LS is valid)
        // other_features.len() would be 2 (the two degenerate LS)
        // is_merged.len() would be 1
        // R-tree size would be 2 (2 endpoints for the 1 valid LS)
    }

    #[test]
    fn test_r_tree_population_and_is_merged_size() {
        let collection = FeatureCollection {
            bbox: None,
            features: vec![
                create_line_string_feature(vec![vec![0.0, 0.0], vec![1.0, 1.0]], None), // LS 0
                create_point_feature(vec![10.0, 10.0], None),                           // Point
                create_line_string_feature(vec![vec![2.0, 2.0], vec![3.0, 3.0]], None), // LS 1
                create_line_string_feature(vec![vec![4.0, 4.0]], None), // Degenerate LS
            ],
            foreign_members: None,
        };

        // We need to access the internal state (line_strings, is_merged, tree)
        // To do this directly in tests without changing the public function signature,
        // we can either refactor the function to return these intermediate values
        // or make parts of the function public or use a test helper function
        // that exposes these internals. For simplicity in this example, let's
        // manually replicate the initial filtering step within the test to check
        // the intermediate results.

        let mut line_strings: Vec<Feature> = Vec::new();
        let mut other_features: Vec<Feature> = Vec::new();

        for feature in &collection.features {
            if let Some(geometry) = &feature.geometry {
                if let Value::LineString(coords) = &geometry.value {
                    if coords.len() >= 2 {
                        line_strings.push(feature.clone());
                    } else {
                        other_features.push(feature.clone());
                    }
                } else {
                    other_features.push(feature.clone());
                }
            } else {
                other_features.push(feature.clone());
            }
        }

        // Check line_strings and other_features counts
        assert_eq!(line_strings.len(), 2, "Should have 2 valid LineStrings");
        assert_eq!(other_features.len(), 2, "Should have 2 other features"); // 1 Point + 1 degenerate LS

        // Check is_merged vector size
        let is_merged: Vec<bool> = vec![false; line_strings.len()];
        assert_eq!(
            is_merged.len(),
            2,
            "is_merged vector size should match valid LineStrings count"
        );
        assert!(
            is_merged.iter().all(|&m| m == false),
            "All is_merged flags should be initially false"
        );

        // Check R-tree population
        let mut tree: RTree<LineStringEndpoint> = RTree::new();
        for (index, feature) in line_strings.iter().enumerate() {
            if let Some(Value::LineString(coords)) = &feature.geometry.as_ref().map(|g| &g.value) {
                let start_coord = &coords[0];
                let end_coord = &coords[coords.len() - 1];
                if let (Ok(start_point), Ok(end_point)) =
                    (start_coord.clone().try_into(), end_coord.clone().try_into())
                {
                    tree.insert(LineStringEndpoint {
                        point: start_point,
                        feature_idx: index,
                        is_start: true,
                    });
                    tree.insert(LineStringEndpoint {
                        point: end_point,
                        feature_idx: index,
                        is_start: false,
                    });
                }
            }
        }

        assert_eq!(
            tree.size(),
            4,
            "R-tree should contain 4 points (2 for each of the 2 valid LineStrings)"
        );

        // Verify some points in the tree
        let expected_points = vec![
            LineStringEndpoint {
                point: [0.0, 0.0],
                feature_idx: 0,
                is_start: true,
            },
            LineStringEndpoint {
                point: [1.0, 1.0],
                feature_idx: 0,
                is_start: false,
            },
            LineStringEndpoint {
                point: [2.0, 2.0],
                feature_idx: 1,
                is_start: true,
            },
            LineStringEndpoint {
                point: [3.0, 3.0],
                feature_idx: 1,
                is_start: false,
            },
        ];

        // Collect all points from the tree (order is not guaranteed)
        let mut points_in_tree: Vec<LineStringEndpoint> = tree.iter().cloned().collect();
        // Sort both vectors to compare them
        points_in_tree.sort_by(|a, b| {
            a.feature_idx
                .cmp(&b.feature_idx)
                .then_with(|| a.is_start.cmp(&b.is_start))
                .then_with(|| a.point[0].partial_cmp(&b.point[0]).unwrap())
                .then_with(|| a.point[1].partial_cmp(&b.point[1]).unwrap())
        });
        let mut expected_points_sorted = expected_points;
        expected_points_sorted.sort_by(|a, b| {
            a.feature_idx
                .cmp(&b.feature_idx)
                .then_with(|| a.is_start.cmp(&b.is_start))
                .then_with(|| a.point[0].partial_cmp(&b.point[0]).unwrap())
                .then_with(|| a.point[1].partial_cmp(&b.point[1]).unwrap())
        });

        assert_eq!(
            points_in_tree, expected_points_sorted,
            "Points in R-tree should match expected endpoints"
        );
    }
}
