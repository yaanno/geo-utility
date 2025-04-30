use geo::Coord;
use geojson::{Feature, Geometry, Value};
use ordered_float::Float;
use rand::Rng;
use serde_json::json; // You'll need the 'rand' crate

pub fn generate_synthetic_linestrings(
    num_features: usize,
    max_vertices_per_feature: usize,
    bend_frequency: f64, // e.g., bends per unit length or per vertex
    max_bend_angle_degrees: f64,
) -> Vec<Feature> {
    let mut features = Vec::with_capacity(num_features);
    let mut rng = rand::thread_rng();

    for i in 0..num_features {
        let mut coords: Vec<Coord<f64>> = Vec::new();
        let mut current_coord = Coord {
            x: rng.gen_range(-1000.0..1000.0), // Random start position
            y: rng.gen_range(-1000.0..1000.0),
        };
        coords.push(current_coord);

        let num_vertices = rng.gen_range(2..=max_vertices_per_feature);
        let mut current_direction_angle = rng.gen_range(0.0..360.0); // Initial direction in degrees

        for j in 1..num_vertices {
            // Simple straight step
            let step_distance = rng.gen_range(1.0..100.0); // Random segment length
            let angle_rad = current_direction_angle.to_radians();
            let next_coord = Coord {
                x: current_coord.x + angle_rad.cos() * step_distance,
                y: current_coord.y + angle_rad.sin() * step_distance,
            };

            // Optionally introduce a bend
            // You'd need a more sophisticated logic here based on bend_frequency
            // For simplicity, let's sometimes add a bend after a few steps
            if j > 0 && j % 5 == 0 && rng.gen_bool(bend_frequency) {
                // Example: 50% chance every 5 points
                let bend_angle = rng.gen_range(-max_bend_angle_degrees..max_bend_angle_degrees);
                current_direction_angle += bend_angle;
                // Ensure angle stays within 0-360
                current_direction_angle =
                    (current_direction_angle.rem_euclid(360.0) + 360.0).rem_euclid(360.0);
            }

            coords.push(next_coord);
            current_coord = next_coord;
        }

        // Ensure at least 2 points for a valid LineString
        if coords.len() < 2 {
            continue; // Skip if generation resulted in < 2 points (unlikely with loop starting at 1)
        }

        let line_string_value = Value::LineString(coords.iter().map(|c| vec![c.x, c.y]).collect());

        // Add some dummy properties
        let mut properties = serde_json::Map::new();
        properties.insert(
            "properties".to_string(),
            json!({"objectId": format!("synthetic_{}", i)}),
        );
        // DROPPING THE AMOUNT OF PROPERTIES HELPED REDUCING THE DATA FILE SIZE
        // EVENTUALLY THIS MADE THE ALGORITHM BENCHMARK MORE ACCURATE
        // THIS ALSO MEANS PERFORMANCE REGRESSIONS ARE MORE LIKELY TO BE RELATED
        // TO THE DATA SIZE ITSELF COMBINED WITH THE PROPERTY CLONING IN THE PROCESSING ALGORITHM
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
