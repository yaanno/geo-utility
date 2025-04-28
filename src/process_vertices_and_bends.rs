use geo::{Coord, LineString, Point};
use geojson::{Feature, Value};
use serde_json::Value as SerdeValue;

pub fn is_gebaeudekante(feature: &Feature) -> bool {
    feature
        .properties
        .as_ref()
        .and_then(|props| props.get("properties"))
        .and_then(|nested_props| nested_props.as_object())
        .and_then(|obj| obj.get("objectId"))
        .and_then(|id| id.as_str())
        .map_or(false, |id_str| id_str == "Gebäudekante")
}

// This function processes original LineString vertices, detecting bends
// and handling simple 2-point lines, to generate extended features.
pub fn process_vertices_and_bends(
    features: Vec<Feature>,
    bend_threshold_degrees: f64, // Threshold angle in degrees to detect a bend at an inner vertex
    extension_distance: f64,     // The distance to extend the lines
) -> Vec<(LineString<f64>, Option<SerdeValue>)> {
    let mut generated_features: Vec<(LineString<f64>, Option<SerdeValue>)> =
        Vec::with_capacity(features.len());
    let bend_threshold_radians = bend_threshold_degrees.to_radians();

    for feature in &features {
        if let Some(geometry) = &feature.geometry {
            match &geometry.value {
                Value::LineString(line_coords) => {
                    // Filter out "Gebäudekante"
                    if is_gebaeudekante(feature) {
                        // println!("Skipping feature with objectId: Gebäudekante");
                        continue;
                    }

                    // Convert to geo::LineString coordinates
                    let coords: Vec<Coord<f64>> = line_coords
                        .into_iter()
                        .map(|coord| Coord {
                            x: coord[0],
                            y: coord[1],
                        })
                        .collect();

                    let num_points = coords.len();

                    if num_points < 2 {
                        // println!("LineString has fewer than 2 points, skipping.");
                        continue;
                    }

                    // --- Function to generate the four lines at a given point with a specific direction ---
                    let mut generate_lines_at_point = |
                    current_point: Point<f64>,
                    forward_direction: Point<f64>, // Vector for forward/orthogonal
                    backward_direction: Point<f64>, // Vector for backward extension
                    | {
                        // Check if the forward direction vector has a non-zero length
                        let forward_length = forward_direction.x().hypot(forward_direction.y());
                        if forward_length == 0.0 {
                            // println!(
                            //     "Skipping line generation at point {:?} due to zero-length forward direction vector.",
                            //     current_point
                            // );
                            return; // Cannot determine direction
                        }

                        // Normalize the forward direction vector
                        let unit_forward_direction = Point::new(
                            forward_direction.x() / forward_length,
                            forward_direction.y() / forward_length,
                        );

                        // Check if the backward direction vector has a non-zero length
                        let backward_length = backward_direction.x().hypot(backward_direction.y());
                        let unit_backward_direction = if backward_length > 0.0 {
                            Point::new(
                                backward_direction.x() / backward_length,
                                backward_direction.y() / backward_length,
                            )
                        } else {
                            // If backward direction is zero length, use the reverse of forward direction
                            Point::new(-unit_forward_direction.x(), -unit_forward_direction.y())
                        };

                        // --- Generate the four extended/rotated points ---

                        // Point extended forward along forward direction
                        let extended_forward = Point::new(
                            current_point.x() + unit_forward_direction.x() * extension_distance,
                            current_point.y() + unit_forward_direction.y() * extension_distance,
                        );

                        // Point extended backward along backward direction
                        let extended_backward = Point::new(
                            current_point.x() + unit_backward_direction.x() * extension_distance,
                            current_point.y() + unit_backward_direction.y() * extension_distance,
                        );

                        // Rotate the unit forward direction vector +90 degrees for orthogonal direction
                        let orthogonal_direction_90 = Point::new(
                            -unit_forward_direction.y(), // Rotated X
                            unit_forward_direction.x(),  // Rotated Y
                        );

                        // Rotate the unit forward direction vector -90 degrees for orthogonal direction
                        let orthogonal_direction_minus_90 = Point::new(
                            unit_forward_direction.y(),  // Rotated X
                            -unit_forward_direction.x(), // Rotated Y
                        );

                        // Point extended in +90 degree orthogonal direction
                        let extended_orthogonal_90 = Point::new(
                            current_point.x() + orthogonal_direction_90.x() * extension_distance,
                            current_point.y() + orthogonal_direction_90.y() * extension_distance,
                        );

                        // Point extended in -90 degree orthogonal direction
                        let extended_orthogonal_minus_90 = Point::new(
                            current_point.x()
                                + orthogonal_direction_minus_90.x() * extension_distance,
                            current_point.y()
                                + orthogonal_direction_minus_90.y() * extension_distance,
                        );

                        // --- Build the generated LineString for this point ---
                        let mut generated_segments_coords: Vec<Coord<f64>> = Vec::with_capacity(8);

                        // 1. Segment: current_point -> extended_forward
                        generated_segments_coords.push(current_point.into());
                        generated_segments_coords.push(extended_forward.into());

                        // 2. Segment: current_point -> extended_backward
                        generated_segments_coords.push(current_point.into());
                        generated_segments_coords.push(extended_backward.into());

                        // 3. Segment: current_point -> extended_orthogonal_90
                        generated_segments_coords.push(current_point.into());
                        generated_segments_coords.push(extended_orthogonal_90.into());

                        // 4. Segment: current_point -> extended_orthogonal_minus_90
                        generated_segments_coords.push(current_point.into());
                        generated_segments_coords.push(extended_orthogonal_minus_90.into());

                        let generated_line_string = LineString::new(generated_segments_coords);

                        // --- Store the generated geometry and original properties ---
                        // DROPPING THE AMOUNT OF PROPERTIES HELPED REDUCING THE DATA FILE SIZE
                        // EVENTUALLY THIS MADE THE ALGORITHM BENCHMARK MORE ACCURATE
                        // THIS ALSO MEANS PERFORMANCE REGRESSIONS ARE MORE LIKELY TO BE RELATED
                        // TO THE DATA SIZE ITSELF COMBINED WITH THE PROPERTY CLONING IN THE PROCESSING ALGORITHM
                        // THAT IS NOW DISABLED BELOW
                        generated_features.push((
                            generated_line_string,
                            // original_properties.clone().map(SerdeValue::Object),
                            None,
                        ));
                    }; // End of generate_lines_at_point closure

                    // --- Process Multi-Segment LineStrings (length > 2) ---
                    if num_points > 2 {
                        // println!("Processing multi-segment line ({} points).", num_points);

                        // Iterate through inner vertices (index 1 to num_points - 2)
                        for i in 1..(num_points - 1) {
                            let p_previous = Point::from(coords[i - 1]);
                            let p_current = Point::from(coords[i]);
                            let p_next = Point::from(coords[i + 1]);

                            let v_in = Point::new(
                                p_current.x() - p_previous.x(),
                                p_current.y() - p_previous.y(),
                            );
                            let v_out =
                                Point::new(p_next.x() - p_current.x(), p_next.y() - p_current.y());
                            let len_in = v_in.x().hypot(v_in.y());
                            let len_out = v_out.x().hypot(v_out.y());

                            // Skip bend check if either segment has zero length
                            if len_in == 0.0 || len_out == 0.0 {
                                // println!(
                                //     "Skipping bend check at point {:?} due to zero-length segment.",
                                //     p_current
                                // );
                                continue;
                            }

                            // Calculate dot product
                            let dot_product = v_in.x() * v_out.x() + v_in.y() * v_out.y();

                            // Calculate cosine of the angle
                            let cos_theta = dot_product / (len_in * len_out);

                            // Clamp cos_theta to handle potential floating point inaccuracies
                            let cos_theta_clamped = cos_theta.clamp(-1.0, 1.0);

                            // Calculate the angle between vectors (the turn angle) in radians
                            let angle_radians = cos_theta_clamped.acos();

                            // Check if the angle indicates a significant bend
                            // We detect a bend if the turn angle is GREATER than the threshold
                            if angle_radians.abs() > bend_threshold_radians {
                                // println!(
                                //     "Detected bend at point {:?} with angle {:.2} degrees. Generating lines.",
                                //     p_current,
                                //     angle_radians.to_degrees()
                                // );

                                // At bends, use the outgoing segment direction (v_out) for forward/orthogonal
                                // and the incoming segment direction (v_in) for backward.
                                generate_lines_at_point(
                                    p_current, v_out, v_in,
                                    // &feature.properties,
                                );
                            }
                            // Else: No significant bend at this inner vertex, no lines generated in this block
                        }

                        // For multi-segment lines, the start and end points (coords[0] and coords[num_points-1])
                        // might need separate processing if they are not covered by the bend logic.
                        // Based on the TS, simple lines process ends. Multi-segment only process inner bends.
                        // If you need to process ends of multi-segment lines too, add logic here
                        // similar to the 2-point case, using the first/last segment directions.
                        // println!(
                        //     "Inner vertices of multi-segment line processed. Start/End points not processed in this block."
                        // );
                    }
                    // --- Process Simple 2-Point LineStrings ---
                    else if num_points == 2 {
                        // println!("Processing simple 2-point line.");
                        let p_start = Point::from(coords[0]);
                        let p_end = Point::from(coords[1]);

                        let direction_forward_segment =
                            Point::new(p_end.x() - p_start.x(), p_end.y() - p_start.y());
                        let direction_backward_segment =
                            Point::new(p_start.x() - p_end.x(), p_start.y() - p_end.y());

                        // Process the Start Point (index 0)
                        // println!("Generating lines at start point {:?}.", p_start);
                        // For the start point, forward is 0->1, backward is 1->0
                        generate_lines_at_point(
                            p_start,
                            direction_forward_segment,
                            direction_backward_segment,
                            // &feature.properties,
                        );

                        // Process the End Point (index 1)
                        // println!("Generating lines at end point {:?}.", p_end);
                        // For the end point, forward is 1->0, backward is 0->1
                        generate_lines_at_point(
                            p_end,
                            direction_backward_segment,
                            direction_forward_segment,
                            // &feature.properties,
                        );
                    } // Else: num_points < 2 handled at the beginning
                }
                _ => {
                    // Handle other geometry types if necessary
                }
            }
        }
    }

    generated_features
}

#[cfg(test)]
mod tests {
    use super::*;
    use geojson::{Feature, Geometry, Value};
    use serde_json::json;

    // Helper to create a basic feature
    fn create_test_feature(coords: Vec<Vec<f64>>, object_id: &str) -> Feature {
        let prop = json!({"objectId": object_id});
        let mut properties = serde_json::Map::new();
        properties.insert("properties".to_string(), prop);

        Feature {
            bbox: None,
            geometry: Some(Geometry {
                bbox: None,
                foreign_members: None,
                value: Value::LineString(coords),
            }),
            id: None,
            properties: Some(properties),
            foreign_members: None,
        }
    }

    #[test]
    fn test_process_vertices_and_bends() {
        let bend_threshold_degrees = 10.0; // Detect bends greater than 10 degrees turn
        let extension_distance = 0.5;

        // 1. Simple 2-point line (should process both ends)
        let feature_simple = create_test_feature(vec![vec![0.0, 0.0], vec![1.0, 1.0]], "test");

        // 2. Multi-segment line with no significant bends (should process ends if logic added, inner vertices not processed by bend)
        // Points (0,0) -> (2,0) -> (4,0) is a straight line, angle = 180 degrees.
        // Turn angle = 0 degrees. Should NOT trigger bend detection.
        let feature_straight_multi =
            create_test_feature(vec![vec![0.0, 0.0], vec![2.0, 0.0], vec![4.0, 0.0]], "test");

        // 3. Multi-segment line with a sharp bend at the middle vertex (should process middle vertex)
        // Points (0,0) -> (1,0) -> (1,1) is a 90-degree turn at (1,0). Angle = 90 degrees.
        // 90 > 10, should trigger bend detection.
        let feature_bend_90 =
            create_test_feature(vec![vec![0.0, 0.0], vec![1.0, 0.0], vec![1.0, 1.0]], "test");

        // 4. Multi-segment line with a slight bend (should NOT trigger bend)
        // Points (0,0) -> (2,0) -> (4, 0.1) - small angle change
        let feature_slight_bend =
            create_test_feature(vec![vec![0.0, 0.0], vec![2.0, 0.0], vec![4.0, 0.1]], "test");

        // 5. Line to be skipped
        let feature_skip =
            create_test_feature(vec![vec![10.0, 10.0], vec![11.0, 11.0]], "Gebäudekante");

        // 6. Multi-segment line with two bends
        let feature_double_bend = create_test_feature(
            vec![
                vec![0.0, 0.0],
                vec![1.0, 0.0],
                vec![1.0, 1.0],
                vec![2.0, 1.0],
            ],
            "test",
        );
        // Bend at (1,0) (90 deg turn)
        // Bend at (1,1) (90 deg turn)

        let features = vec![
            feature_simple,
            feature_straight_multi,
            feature_bend_90,
            feature_slight_bend,
            feature_skip,
            feature_double_bend,
        ];

        let generated_features =
            process_vertices_and_bends(features, bend_threshold_degrees, extension_distance);

        println!(
            "Generated {} feature sets in total.",
            generated_features.len()
        );

        // Assertions:
        // feature_simple (2 points): Processes 2 points (start and end) -> 2 * 1 = 2 feature sets (each set is one LineString with 4 segments)
        // feature_straight_multi (3 points): Inner vertex at (2,0). Angle is 180 (0 turn). 0 < 10. No bend processed. Ends not processed in this logic. -> 0 feature sets from bend logic.
        // feature_bend_90 (3 points): Inner vertex at (1,0). Angle is 90. 90 > 10. Bend processed. -> 1 feature set.
        // feature_slight_bend (3 points): Inner vertex at (2,0). Angle will be close to 180. Turn angle small. Should be < 10 degrees. No bend processed. -> 0 feature sets.
        // feature_skip: Skipped -> 0 feature sets.
        // feature_double_bend (4 points): Inner vertices at (1,0) and (1,1). Both are 90-degree turns. Both > 10. Both processed. -> 2 feature sets.

        // Expected total generated feature sets = 2 (simple line ends) + 0 (straight multi inner) + 1 (90 bend inner) + 0 (slight bend inner) + 0 (skip) + 2 (double bend inner) = 5
        assert_eq!(
            generated_features.len(),
            5,
            "Incorrect number of generated feature sets"
        );

        // You can add more specific assertions here to check the properties or coordinates
        // of the generated LineStrings if needed.
        for (i, (gen_geom, gen_props)) in generated_features.iter().enumerate() {
            println!("Generated Feature Set {}:", i);
            println!("  Geometry: {:?}", gen_geom);
            if let Some(props) = gen_props {
                println!("  Properties: {:?}", props);
            } else {
                println!("  Properties: None");
            }
            assert_eq!(
                gen_geom.coords().count(),
                8,
                "Each generated LineString should have 8 coordinates (4 segments)"
            );
        }
    }
}
