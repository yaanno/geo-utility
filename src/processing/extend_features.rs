use geo::algorithm::line_measures::metric_spaces::Euclidean;
use geo::{CoordsIter, Distance, LineString, LineStringSegmentize, Point};
use serde_json::Value as SerdeValue;

use crate::utils::geometry::{GeoFeature, GeoGeometry}; // To handle properties

// This function will generate new LineStrings based on segment endpoints
pub fn extend_features(
    features: Vec<GeoFeature>,
    segment_length: usize,   // The maximum length of the segments
    extension_distance: f64, // The distance to extend the lines
) -> Vec<(LineString<f64>, Option<SerdeValue>)> // Return generated geometry and original properties
{
    let mut generated_features: Vec<(LineString<f64>, Option<SerdeValue>)> =
        Vec::with_capacity(features.len());

    for feature in &features {
        if let Some(geometry) = &feature.geometry {
            match &geometry {
                GeoGeometry::LineString(line_coords) => {
                    // Filter out "Gebäudekante"
                    let is_gebaeudekante = feature
                        .properties
                        .as_ref()
                        .and_then(|props| props.get("properties"))
                        .and_then(|nested_props| nested_props.as_object())
                        .and_then(|obj| obj.get("objectId"))
                        .and_then(|id| id.as_str()) == Some("Gebäudekante");

                    if is_gebaeudekante {
                        println!("Skipping feature with objectId: Gebäudekante");
                        continue;
                    }

                    // Convert to geo::LineString
                    let original_line_string: LineString<f64> = line_coords
                        .into_iter()
                        .map(|coord| Point::new(coord.x, coord.y))
                        .collect();

                    // Segmentize the line string
                    if let Some(segmented_line_string) =
                        original_line_string.line_segmentize(segment_length)
                    {
                        // The segmentize method returns a new LineString made of the segments.
                        // We want the *end point* of each of these new segments as our points.
                        let segment_endpoints: Vec<Point<f64>> = segmented_line_string
                            .coords_iter()
                            .map(Point::from)
                            .collect();

                        // Iterate through the endpoints, starting from the second point
                        // (the first endpoint is the start of the original line)
                        if segment_endpoints.len() > 1 {
                            for i in 1..segment_endpoints.len() {
                                let current_point = segment_endpoints[i];
                                let previous_point = segment_endpoints[i - 1]; // Point from the previous segment endpoint or original start

                                // Determine the local direction vector using the current and previous point
                                // Note: This assumes direction from previous point to current point.
                                // For smoother curves, you might need a more sophisticated approach.
                                let direction_vector = Point::new(
                                    current_point.x() - previous_point.x(),
                                    current_point.y() - previous_point.y(),
                                );

                                // Check if the direction vector has a non-zero length to avoid division by zero
                                let direction_length =
                                    Euclidean.distance(&current_point, &previous_point);
                                if direction_length == 0.0 {
                                    println!(
                                        "Skipping extension at point {:?} due to zero-length direction vector.",
                                        current_point
                                    );
                                    continue; // Cannot determine direction
                                }

                                // Normalize the direction vector
                                let unit_direction_vector = Point::new(
                                    direction_vector.x() / direction_length,
                                    direction_vector.y() / direction_length,
                                );

                                // --- Generate the four extended/rotated points ---

                                // Point extended forward along direction
                                let extended_forward = Point::new(
                                    current_point.x()
                                        + unit_direction_vector.x() * extension_distance,
                                    current_point.y()
                                        + unit_direction_vector.y() * extension_distance,
                                );

                                // Point extended backward along direction (reverse the unit direction vector)
                                let extended_backward = Point::new(
                                    current_point.x()
                                        - unit_direction_vector.x() * extension_distance,
                                    current_point.y()
                                        - unit_direction_vector.y() * extension_distance,
                                );

                                // Rotate the unit direction vector +90 degrees for orthogonal direction
                                let orthogonal_direction_90 = Point::new(
                                    -unit_direction_vector.y(), // Rotated X
                                    unit_direction_vector.x(),  // Rotated Y
                                );

                                // Rotate the unit direction vector -90 degrees for orthogonal direction
                                let orthogonal_direction_minus_90 = Point::new(
                                    unit_direction_vector.y(),  // Rotated X
                                    -unit_direction_vector.x(), // Rotated Y
                                );

                                // Point extended in +90 degree orthogonal direction
                                let extended_orthogonal_90 = Point::new(
                                    current_point.x()
                                        + orthogonal_direction_90.x() * extension_distance,
                                    current_point.y()
                                        + orthogonal_direction_90.y() * extension_distance,
                                );

                                // Point extended in -90 degree orthogonal direction
                                let extended_orthogonal_minus_90 = Point::new(
                                    current_point.x()
                                        + orthogonal_direction_minus_90.x() * extension_distance,
                                    current_point.y()
                                        + orthogonal_direction_minus_90.y() * extension_distance,
                                );

                                // --- Build the generated LineStrings for this point ---
                                // Each pair of points forms a line segment
                                let mut generated_segments_coords: Vec<Point<f64>> =
                                    Vec::with_capacity(8);

                                // 1. Segment: current_point -> extended_forward
                                generated_segments_coords.push(current_point);
                                generated_segments_coords.push(extended_forward);

                                // 2. Segment: current_point -> extended_backward
                                generated_segments_coords.push(current_point);
                                generated_segments_coords.push(extended_backward);

                                // 3. Segment: current_point -> extended_orthogonal_90
                                generated_segments_coords.push(current_point);
                                generated_segments_coords.push(extended_orthogonal_90);

                                // 4. Segment: current_point -> extended_orthogonal_minus_90
                                generated_segments_coords.push(current_point);
                                generated_segments_coords.push(extended_orthogonal_minus_90);

                                // Create a single LineString containing all four segments from this point
                                let generated_line_string =
                                    LineString::from(generated_segments_coords);

                                // --- Store the generated geometry and original properties ---
                                // Store a clone of the original properties if they exist
                                generated_features.push((
                                    generated_line_string,
                                    feature.properties.clone().map(SerdeValue::Object),
                                ));
                            }
                        } else {
                            println!(
                                "Segmentized line has fewer than 2 points, cannot determine direction for extensions."
                            );
                        }
                    } else {
                        println!("Failed to segmentize the line string.");
                    }
                }
                _ => {
                    // Handle other geometry types if necessary
                }
            }
        }
    }

    generated_features
}

// Update the test case to use the new function and parameters
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_generate_extended_features_segmentize() {
        let segment_length = 1; // Generate points roughly every 1 unit
        let extension_distance = 0.5; // Extend by 0.5 units

        let prop_test = json!({"objectId": "test"});
        let mut properties_test = serde_json::Map::new();
        properties_test.insert("properties".to_string(), prop_test);

        let prop_gebaeudekante = json!({"objectId": "Gebäudekante"});
        let mut properties_gebaeudekante = serde_json::Map::new();
        properties_gebaeudekante.insert("properties".to_string(), prop_gebaeudekante);

        // Feature that should be processed - a line from (0,0) to (3,0)
        let feature_process_horizontal = GeoFeature {
            bbox: None,
            geometry: Some(GeoGeometry::LineString(vec![
                Point::new(0.0, 0.0),
                Point::new(1.0, 0.0),
                Point::new(2.0, 0.0),
                Point::new(3.0, 0.0),
            ].into())),
            id: None,
            properties: Some(properties_test.clone()), // Clone for use in multiple features
            foreign_members: None,
        };

        // Feature that should be processed - a line from (0,0) to (1,1)
        let feature_process_diagonal = GeoFeature {
            bbox: None,
            geometry: Some(GeoGeometry::LineString(vec![
                Point::new(0.0, 0.0),
                Point::new(1.0, 1.0),
            ].into())), // Length is sqrt(2)
            id: None,
            properties: Some(properties_test),
            foreign_members: None,
        };

        // Feature that should be skipped
        let feature_skip = GeoFeature {
            bbox: None,
            geometry: Some(GeoGeometry::LineString(vec![
                Point::new(10.0, 10.0),
                Point::new(11.0, 11.0),
            ].into())),
            id: None,
            properties: Some(properties_gebaeudekante),
            foreign_members: None,
        };

        let features = vec![
            feature_process_horizontal,
            feature_process_diagonal,
            feature_skip,
        ];
        let generated_features = extend_features(features, segment_length, extension_distance);

        println!("Generated {} feature sets.", generated_features.len());

        // Basic assertions
        // We expect generated features from the horizontal line (approx length 3, segment_length 1 -> ~3 segments, 3 endpoints)
        // and from the diagonal line (approx length 1.414, segment_length 1 -> ~1 segment, 1 endpoint)
        // So, expect features from ~4 points in total.
        // Each point generates one LineString with 4 segments (8 coordinates).
        assert!(
            !generated_features.is_empty(),
            "Should generate features from processed lines"
        );
        assert!(
            generated_features.len() <= 4,
            "Should not generate too many features"
        ); // Based on the test data structure

        // You can add more specific assertions here based on the expected coordinates
        for (i, (gen_geom, gen_props)) in generated_features.iter().enumerate() {
            println!("Generated Feature Set {}:", i);
            println!("  Geometry: {:?}", gen_geom);
            println!("  Properties: {:?}", gen_props);
            // Example: Check that each generated LineString has 8 coordinates (4 segments)
            assert_eq!(
                gen_geom.coords().count(),
                8,
                "Each generated LineString should have 8 coordinates (4 segments)"
            );
        }
    }
}
