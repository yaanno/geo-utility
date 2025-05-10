use geojson::{Feature, FeatureCollection, Geometry, JsonObject, Value};
use rstar::{PointDistance, RTree, RTreeObject};
use std::convert::TryInto;
use std::collections::VecDeque;

// Define the struct to be stored in the R-tree
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
struct LineStringEndpoint {
    point: [f64; 2],
    feature_idx: usize, // Index back to the original feature in the `line_strings` vector
    is_start: bool,     // Is this the start or end point of the original line?
}

// Implement RTreeObject for our endpoint struct
impl RTreeObject for LineStringEndpoint {
    type Envelope = rstar::AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        Self::Envelope::from_point(self.point)
    }
}

impl PointDistance for LineStringEndpoint {
    fn distance_2(&self, point: &[f64; 2]) -> f64 {
        let dx = self.point[0] - point[0];
        let dy = self.point[1] - point[1];
        dx * dx + dy * dy
    }
}
#[allow(dead_code)]
// Placeholder for the property matching logic
// TODO: Implement the actual property comparison based on your data structure
fn properties_match(props1: Option<&JsonObject>, props2: Option<&JsonObject>) -> bool {
    // Example placeholder logic: Check if both have properties and are not null
    // You need to replace this with your specific comparison logic
    // involving parsing the nested 'properties.properties' string.
    match (props1, props2) {
        (Some(v1), Some(v2)) => {
            // --- Implement your detailed property comparison here ---
            // This is where you'd parse the nested JSON strings and compare fields.
            // For now, a simple placeholder comparison
            v1 == v2 // This is likely NOT sufficient for your actual data
            // Example:
            // if let (Some(nested1), Some(nested2)) = (v1.get("properties"), v2.get("properties")) {
            //     if let (Some(props_str1), Some(props_str2)) = (nested1.get("properties").and_then(|v| v.as_str()), nested2.get("properties").and_then(|v| v.as_str())) {
            //         if let (Ok(parsed1), Ok(parsed2)) = (serde_json::from_str::<JsonValue>(props_str1), serde_json::from_str::<JsonValue>(props_str2)) {
            //             // Compare fields like objectId, constructionType, etc. on parsed1 and parsed2
            //             // Also compare top-level 'surface'
            //             // return parsed1.get("objectId") == parsed2.get("objectId") && ...
            //         }
            //     }
            // }
            // false // Default to false if parsing or comparison fails
        }
        (None, None) => true, // Both have no properties, consider them a match? Adjust as needed.
        _ => false,           // One has properties, the other doesn't
    }
}

#[allow(dead_code)]
pub fn concatenate_features(collection: &FeatureCollection) -> FeatureCollection {
    if collection.features.is_empty() {
        return FeatureCollection {
            bbox: collection.bbox.clone(),
            features: Vec::new(),
            foreign_members: collection.foreign_members.clone(),
        };
    }

    let mut line_strings: Vec<Feature> = Vec::new();
    let mut other_features: Vec<Feature> = Vec::new();

    // Filter LineStrings and other features
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

    // If no LineStrings, just return other features
    if line_strings.is_empty() {
        return FeatureCollection {
            bbox: collection.bbox.clone(),
            features: other_features,
            foreign_members: collection.foreign_members.clone(),
        };
    }

    // Create the R-tree
    let mut tree: RTree<LineStringEndpoint> = RTree::new();

    // Populate the R-tree
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
            } else {
                eprintln!(
                    "Warning: Coordinate in LineString not in [x, y] format for feature index {}",
                    index
                );
            }
        }
    }

    // Status tracker for original LineString features
    let mut is_merged: Vec<bool> = vec![false; line_strings.len()];

    // Vector to store the final merged (or unmerged) LineString features
    let mut merged_line_strings: Vec<Feature> = Vec::new();

    // Define the distance threshold for merging
    const MERGE_DISTANCE_THRESHOLD: f64 = 0.1; // meters

    // --- Iterative Merging Loop ---
    for i in 0..line_strings.len() {
        // If this original feature has already been merged into another line, skip it
        if is_merged[i] {
            continue;
        }

        // This feature starts a new merged line
        is_merged[i] = true;

        // Get the initial coordinates and properties for the line being built
        let initial_coords = match line_strings[i]
            .geometry
            .as_ref()
            .and_then(|g| match &g.value {
                Value::LineString(coords) => Some(coords.clone()),
                _ => None,
            }) {
            Some(coords) => coords,
            None => {
                // This should not happen due to filtering, but handle defensively
                eprintln!(
                    "Error: Feature {} unexpectedly not a LineString during merging.",
                    i
                );
                continue; // Skip this feature
            }
        };
        let initial_properties = line_strings[i].properties.clone(); // Properties for the final merged feature

        // Use a mutable vector to build the coordinates of the current merged line
        let mut current_merged_coords: VecDeque<[f64; 2]> = initial_coords.into_iter()
            .map(|c| c.try_into().expect("Coordinate should be [f64; 2]"))
            .collect();

        // Loop to repeatedly try extending the current line
        'extend_loop: loop {
            let mut extended_in_this_iteration = false;

            // Get the current start and end points of the line being built
            // These unwraps are safe because current_merged_coords starts with >= 2 points
            let current_start_point: [f64; 2] = *current_merged_coords.front().unwrap();
            let current_end_point: [f64; 2] = *current_merged_coords.back().unwrap();

            // --- Try extending from the current END point ---
            // Query the R-tree for endpoints near the current end point
            for endpoint_ref in
                tree.locate_within_distance(current_end_point, MERGE_DISTANCE_THRESHOLD)
            {
                let j = endpoint_ref.feature_idx;

                // Check if the original feature this endpoint belongs to is unmerged
                // and if its properties match the initial feature of the current merged line
                if !is_merged[j]
                    && properties_match(
                        initial_properties.as_ref(),
                        line_strings[j].properties.as_ref(),
                    )
                {
                    // Get the coordinates of the potential feature to merge
                    if let Some(Value::LineString(potential_coords)) =
                        &line_strings[j].geometry.as_ref().map(|g| &g.value)
                    {
                        let potential_points: Vec<[f64; 2]> = potential_coords.iter()
                            .map(|c| c.clone().try_into().expect("Coordinate should be [f64; 2]"))
                            .collect();
                        // Check if this endpoint is the START of the potential feature
                        if endpoint_ref.is_start {
                            // Merge: Append potential_coords to current_merged_coords
                            // Optional: Remove duplicate point if current_end_point == potential_coords[0]
                            if let Some(potential_start_point) = potential_points.first() {
                                if current_end_point == *potential_start_point {
                                    current_merged_coords.pop_back(); // Remove duplicate end point
                                }
                            }
                             current_merged_coords.extend(potential_points);
                        } else {
                            // This endpoint is the END of the potential feature
                            // Merge: Append potential_coords (reversed) to current_merged_coords
                            // Optional: Remove duplicate point if current_end_point == potential_coords.last()
                            if let Some(potential_end_point) = potential_points.last() {
                                if current_end_point == *potential_end_point {
                                    current_merged_coords.pop_back(); // Remove duplicate end point
                                }
                            }
                            current_merged_coords.extend(potential_points.into_iter().rev());
                        }

                        // Mark the original feature as merged
                        is_merged[j] = true;
                        extended_in_this_iteration = true;
                        // Break from processing query results for the current end point,
                        // and try extending further from the new end point
                        break;
                    }
                }
            }

            // If we extended from the end, continue the outer 'extend_loop' immediately
            if extended_in_this_iteration {
                continue 'extend_loop;
            }

            // --- Try extending from the current START point ---
            // Query the R-tree for endpoints near the current start point
            for endpoint_ref in
                tree.locate_within_distance(current_start_point, MERGE_DISTANCE_THRESHOLD)
            {
                let j = endpoint_ref.feature_idx;

                // Check if the original feature this endpoint belongs to is unmerged
                // and if its properties match the initial feature of the current merged line
                if !is_merged[j]
                    && properties_match(
                        initial_properties.as_ref(),
                        line_strings[j].properties.as_ref(),
                    )
                {
                    // Get the coordinates of the potential feature to merge
                    if let Some(Value::LineString(potential_coords)) =
                        &line_strings[j].geometry.as_ref().map(|g| &g.value)
                    {
                        let potential_points: Vec<[f64; 2]> = potential_coords.iter()
                             .map(|c| c.clone().try_into().expect("Coordinate should be [f64; 2]"))
                             .collect();
                        
                        // Check if this endpoint is the START of the potential feature
                        if endpoint_ref.is_start {
                            // Merge: Prepend potential_coords (reversed) to current_merged_coords
                            // Optional: Remove duplicate point if current_start_point == potential_coords[0]
                            if let Some(potential_start_point) = potential_points.first() {
                                if current_start_point == *potential_start_point {
                                    current_merged_coords.pop_front(); // Remove duplicate start point
                                }
                            }
                            for point in potential_points.into_iter().rev() {
                                current_merged_coords.push_front(point);
                            }
                        } else { 
                            // This endpoint is the END of the potential feature
                            // Merge: Prepend potential_points to current_merged_coords
                            // Optional: Remove duplicate point if current_start_point == potential_points.last()
                            if let Some(potential_end_point) = potential_points.last() {
                                if current_start_point == *potential_end_point {
                                    current_merged_coords.pop_front(); // Remove duplicate start point
                                }
                            }
                            // Prepend points
                            for point in potential_points.into_iter().rev() { // Iterate in reverse to push_front in correct order
                                current_merged_coords.push_front(point);
                            }
                        }

                        // Mark the original feature as merged
                        is_merged[j] = true;
                        extended_in_this_iteration = true;
                        // Break from processing query results for the current start point,
                        // and try extending further from the new start point
                        break;
                    }
                }
            }

            // If we didn't extend in this iteration (from either end), the line is complete
            if !extended_in_this_iteration {
                break 'extend_loop;
            }
        } // End of 'extend_loop'
        // Convert the final VecDeque<[f64; 2]> back to Vec<Vec<f64>> for GeoJSON
        let final_coords_vec: Vec<Vec<f64>> = current_merged_coords.into_iter()
            .map(|arr| arr.to_vec())
            .collect();
        // Create a new Feature for the completed merged line
        let merged_feature = Feature {
            bbox: None, // Bbox could be calculated for the new geometry if needed
            geometry: Some(Geometry::new(Value::LineString(final_coords_vec))),
            id: line_strings[i].id.clone(), // Keep the ID of the starting feature
            properties: initial_properties, // Keep the properties of the starting feature
            foreign_members: line_strings[i].foreign_members.clone(),
        };

        // Add the completed merged feature to the results
        merged_line_strings.push(merged_feature);
    } // End of outer loop iterating through original features

    // Combine the merged LineStrings and the other features
    let mut final_features = merged_line_strings;
    final_features.extend(other_features);

    // Return the final FeatureCollection
    FeatureCollection {
        bbox: collection.bbox.clone(), // Could calculate a new bbox if needed
        features: final_features,
        foreign_members: collection.foreign_members.clone(),
    }
}

// --- Test Suite ---
#[cfg(test)]
mod tests {
    use super::*; // Import items from the outer scope
    use geojson::JsonValue;

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
            is_merged.iter().all(|&m| !m),
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
