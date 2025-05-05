use geo::Coord;
use geojson::{Feature, Geometry, Value};
use serde_json::json;

pub fn generate_synthetic_linestrings(
    num_features: usize,
    max_vertices_per_feature: usize,
    bend_frequency: f64, // This will now influence bend *likelihood* deterministically
    max_bend_angle_degrees: f64,
) -> Vec<Feature> {
    let mut features = Vec::with_capacity(num_features);


    // Use a fixed starting seed or a deterministic generator if needed for more complex patterns,
    // but for simple replacements, counters are often sufficient.

    for i in 0..num_features {
        let mut coords: Vec<Coord<f64>> = Vec::new();

        // Deterministic start position based on feature index
        let mut current_coord = Coord {
            x: (i as f64 % 2000.0) - 1000.0, // Simple pattern based on index
            y: (i as f64 / 2000.0 * 2000.0) - 1000.0, // Another simple pattern
        };
        coords.push(current_coord);

        // Deterministic number of vertices based on feature index
        let num_vertices = 2 + (i % (max_vertices_per_feature - 1)); // Ensures at least 2 vertices

        // Deterministic initial direction based on feature index
        let mut current_direction_angle = (i as f64 * 123.45) % 360.0; // Use a constant multiplier for variation

        for j in 1..num_vertices {
            // Deterministic step distance based on indices
            let step_distance = 1.0 + ((i * num_vertices + j) as f64 % 99.0); // Based on feature and vertex indices
            let angle_rad = current_direction_angle.to_radians();
            let next_coord = Coord {
                x: current_coord.x + angle_rad.cos() * step_distance,
                y: current_coord.y + angle_rad.sin() * step_distance,
            };

            // Introduce a bend deterministically based on bend_frequency and indices
            // We use a condition based on indices and bend_frequency
            let bend_condition = ((i * 1000 + j) % 1000) as f64 / 1000.0 < bend_frequency;

            if j > 0 && j % 5 == 0 && bend_condition {
                // Deterministic bend angle based on indices
                let bend_angle = ((i * 500 + j) as f64 % (max_bend_angle_degrees * 2.0)) - max_bend_angle_degrees;
                current_direction_angle += bend_angle;
                // Ensure angle stays within 0-360
                current_direction_angle =
                    (current_direction_angle.rem_euclid(360.0) + 360.0).rem_euclid(360.0);
            }

            coords.push(next_coord);
            current_coord = next_coord;
        }

        let line_string_value = Value::LineString(coords.iter().map(|c| vec![c.x, c.y]).collect());

        // Add deterministic dummy properties
        let mut properties = serde_json::Map::new();
        properties.insert(
            "properties".to_string(),
            json!({"objectId": format!("synthetic_{}", i)}),
        );
        // The large data property remains the same deterministic string
        properties.insert(
            "large_data".to_string(),
            json!({"filler": "a".repeat(100)}), // 100B string example
        );

        let feature = Feature {
            bbox: None,
            geometry: Some(Geometry {
                bbox: None,
                foreign_members: None,
                value: line_string_value,
            }),
            id: Some(geojson::feature::Id::String(format!("feature_{}", i))),
            properties: Some(properties),
            foreign_members: None,
        };

        features.push(feature);
    }

    features
}
