use geo::{Coord, LineString, Point};
use crate::utils::geometry::{GeoFeature, GeoGeometry};

/// Checks if a feature is a "Gebäudekante" (building edge).
///
/// # Arguments
/// * `feature`: The GeoJSON feature to check
///
/// # Returns
/// A boolean indicating whether the feature is a "Gebäudekante" (building edge)
pub fn is_gebaeudekante(feature: &GeoFeature) -> bool {
    feature
        .properties
        .as_ref()
        .and_then(|props| props.get("properties"))
        .and_then(|nested_props| nested_props.as_object())
        .and_then(|obj| obj.get("objectId"))
        .and_then(|id| id.as_str()) == Some("Gebäudekante")
}

/// Processes original LineString vertices, detecting bends and handling simple 2-point lines, to generate extended features.
///
/// # Arguments
/// * `features`: A vector of GeoJSON features to process
/// * `bend_threshold_degrees`: The angle in degrees to detect a bend at an inner vertex
/// * `extension_distance`: The distance to extend the lines
///
/// # Returns
/// A vector of (LineString) containing the generated geometry
pub fn process_vertices_and_bends(
    features: Vec<GeoFeature>,
    bend_threshold_degrees: f64, // Threshold angle in degrees to detect a bend at an inner vertex
    extension_distance: f64,     // The distance to extend the lines
) -> Vec<LineString<f64>> {
    let mut generated_features: Vec<LineString<f64>> = Vec::with_capacity(features.len());
    let bend_threshold_radians = bend_threshold_degrees.to_radians();

    for feature in &features {
        if let Some(geometry) = &feature.geometry {
            match geometry {
                GeoGeometry::LineString(line_coords) => {
                    // Filter out "Gebäudekante"
                    if is_gebaeudekante(feature) {
                        continue;
                    }

                    // Convert to geo::LineString coordinates
                    let coords: Vec<Coord<f64>> = line_coords
                        .into_iter()
                        .map(|coord| Coord {
                            x: coord.x,
                            y: coord.y,
                        })
                        .collect();

                    let num_points = coords.len();

                    if num_points < 2 {
                        continue;
                    }

                    // --- Process Multi-Segment LineStrings (length > 2) ---
                    if num_points > 2 {
                        let lines = process_multisegment_line(
                            &coords,
                            num_points,
                            bend_threshold_radians,
                            extension_distance,
                        );
                        generated_features.extend(lines);
                    }
                    // --- Process Simple 2-Point LineStrings ---
                    else if num_points == 2 {
                        let lines = process_simple_line(&coords, extension_distance);
                        generated_features.extend(lines);
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

/// Processes a simple 2-point LineString to generate lines at the start and end points.
///
/// # Arguments
/// * `coords`: A slice of coordinates representing the LineString
/// * `extension_distance`: The distance to extend the lines
///
/// # Returns
/// A vector of (LineString) containing the generated geometry
fn process_simple_line(coords: &[Coord<f64>], extension_distance: f64) -> Vec<LineString<f64>> {
    let p_start = Point::from(coords[0]);
    let p_end = Point::from(coords[1]);

    let mut generated_features = Vec::new();

    let direction_forward_segment = Point::new(p_end.x() - p_start.x(), p_end.y() - p_start.y());
    let direction_backward_segment = Point::new(p_start.x() - p_end.x(), p_start.y() - p_end.y());

    // Process the Start Point (index 0)
    if let Some(start_line_string) = generate_lines_at_point(
        p_start,
        direction_forward_segment,
        direction_backward_segment,
        extension_distance,
        // &feature.properties,
    ) {
        generated_features.push(start_line_string);
    }

    // Process the End Point (index 1)
    if let Some(end_line_string) = generate_lines_at_point(
        p_end,
        direction_backward_segment,
        direction_forward_segment,
        extension_distance,
        // &feature.properties,
    ) {
        generated_features.push(end_line_string);
    }

    generated_features
}

const EPSILON: f64 = 1e-10;
fn is_zero_length(v: &Point<f64>) -> bool {
    v.x().hypot(v.y()).abs() < EPSILON
}

/// Processes a multi-segment LineString to generate lines at each vertex.
///
/// # Arguments
/// * `coords`: A slice of coordinates representing the LineString
/// * `num_points`: The number of points in the LineString
/// * `bend_threshold_radians`: The angle in radians to detect a bend at an inner vertex
/// * `extension_distance`: The distance to extend the lines
///
/// # Returns
/// A vector of (LineString) containing the generated geometry
fn process_multisegment_line(
    coords: &[Coord<f64>],
    num_points: usize,
    bend_threshold_radians: f64,
    extension_distance: f64,
) -> Vec<LineString<f64>> {
    if num_points < 3 {
        return Vec::new();
    }

    let mut generated_features = Vec::with_capacity(num_points);

    // Iterate through inner vertices (index 1 to num_points - 2)
    for i in 1..(num_points - 1) {
        let p_previous = Point::from(coords[i - 1]);
        let p_current = Point::from(coords[i]);
        let p_next = Point::from(coords[i + 1]);

        let v_in = Point::new(
            p_current.x() - p_previous.x(),
            p_current.y() - p_previous.y(),
        );
        let v_out = Point::new(p_next.x() - p_current.x(), p_next.y() - p_current.y());
        let len_in = v_in.x().hypot(v_in.y());
        let len_out = v_out.x().hypot(v_out.y());

        // Skip bend check if either segment has zero length
        if is_zero_length(&v_in) || is_zero_length(&v_out) {
            continue;
        }

        // Calculate the dot product of the vectors, which is the cosine of the angle between them
        // a · b = a_x * b_x + a_y * b_y
        let dot_product = v_in.x() * v_out.x() + v_in.y() * v_out.y();

        // Calculate the angle between vectors using the dot product formula:
        // cos(θ) = (a · b) / (|a| * |b|)
        // where a and b are the direction vectors
        let cos_theta = dot_product / (len_in * len_out);

        // Clamp cos_theta to handle potential floating point inaccuracies
        let cos_theta_clamped = cos_theta.clamp(-1.0, 1.0);

        // Calculate the angle between vectors (the turn angle) in radians
        let angle_radians = cos_theta_clamped.acos();

        // Check if the angle between the incoming and outgoing vectors indicates a significant bend.
        // A bend is detected if the angle between vectors is GREATER than the threshold.
        if angle_radians.abs() > bend_threshold_radians {
            // At bends, use the outgoing segment direction (v_out) for forward/orthogonal
            // and the incoming segment direction (v_in) for backward.
            if let Some(line_string) = generate_lines_at_point(p_current, v_out, v_in, extension_distance) {
                generated_features.push(line_string);
            }
        }
    }

    generated_features
}

/// Generates lines at a given point based on forward and backward direction vectors.
///
/// # Arguments
/// * `current_point`: The point at which to generate lines
/// * `forward_direction`: The direction vector for forward extension
/// * `backward_direction`: The direction vector for backward extension
/// * `extension_distance`: The distance to extend the lines
///
/// # Returns
/// An Option containing the generated LineString, or None if the forward direction vector has zero length
fn generate_lines_at_point(
    current_point: Point<f64>,
    forward_direction: Point<f64>,
    backward_direction: Point<f64>,
    extension_distance: f64,
) -> Option<LineString<f64>> {
    // Check if the forward direction vector has a non-zero length
    let forward_length = forward_direction.x().hypot(forward_direction.y());
    if is_zero_length(&forward_direction) {
        return None;
    }

    // Normalize the forward direction vector
    let unit_forward_direction = Point::new(
        forward_direction.x() / forward_length,
        forward_direction.y() / forward_length,
    );

    // Check if the backward direction vector has a non-zero length
    let backward_length = backward_direction.x().hypot(backward_direction.y());
    let unit_backward_direction = if is_zero_length(&backward_direction) {
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
        current_point.x() + orthogonal_direction_minus_90.x() * extension_distance,
        current_point.y() + orthogonal_direction_minus_90.y() * extension_distance,
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

    Some(generated_line_string)
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
            process_vertices_and_bends(features.into_iter().map(|f| f.into()).collect(), bend_threshold_degrees, extension_distance);

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
        for (i, gen_geom) in generated_features.iter().enumerate() {
            println!("Generated Feature Set {}:", i);
            println!("  Geometry: {:?}", gen_geom);
            assert_eq!(
                gen_geom.coords().count(),
                8,
                "Each generated LineString should have 8 coordinates (4 segments)"
            );
        }
    }
}
