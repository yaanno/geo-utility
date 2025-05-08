// Assuming your concatenate_features function is in geo_utility::concatenate_features
use geojson::{Feature, FeatureCollection, Geometry, JsonObject, Value};
use serde_json::{json, Value as JsonValue};
use rand::{Rng, SeedableRng}; // Import SeedableRng
use rand::rngs::StdRng; // Use a standard RNG
use rand::distributions::Uniform;
use std::convert::TryInto;


// Helper function to generate a synthetic LineString feature
fn create_synthetic_line_string(id: usize, start_coord: [f64; 2], end_coord: [f64; 2], properties: Option<JsonObject>) -> Feature {
    let coords = vec![vec![start_coord[0], start_coord[1]], vec![end_coord[0], end_coord[1]]];
     Feature {
        bbox: None,
        geometry: Some(Geometry::new(Value::LineString(coords))),
        id: Some(geojson::feature::Id::Number(id.try_into().unwrap())), // Use usize as ID
        properties,
        foreign_members: None,
    }
}

// Helper function to generate synthetic data for benchmarking
// Uses a seeded RNG for deterministic output
pub fn generate_synthetic_data_concatenate_seeded(num_line_strings: usize, close_pairs_ratio: f64, seed: u64) -> FeatureCollection {
    // Use a seeded standard RNG
    let mut rng = StdRng::seed_from_u64(seed);
    let coord_range = Uniform::from(0.0..1000.0); // Coordinates within a 1000x1000 area
    let close_distance = 0.05; // Distance for 'close' endpoints

    let mut features: Vec<Feature> = Vec::with_capacity(num_line_strings);

    for i in 0..num_line_strings {
        let start_coord = [rng.sample(coord_range), rng.sample(coord_range)];
        let end_coord;

        // Generate some features with endpoints close to previous features' endpoints
        if i > 0 && rng.r#gen::<f64>() < close_pairs_ratio {
            // Pick a random previous feature's endpoint to be close to
            let prev_feature_idx = rng.r#gen_range(0..i);
            // Get the geometry of the previous feature (assuming it's a LineString)
            if let Some(geom) = features[prev_feature_idx].geometry.as_ref() {
                if let Value::LineString(coords) = &geom.value {
                    if coords.len() >= 2 {
                         let prev_endpoint = if rng.r#gen::<bool>() { // Randomly pick start or end of previous line
                             coords.first().unwrap()
                         } else {
                             coords.last().unwrap()
                         };

                         // Generate the new endpoint close to the previous endpoint
                         end_coord = [
                             prev_endpoint[0] + rng.sample(Uniform::from(-close_distance..close_distance)),
                             prev_endpoint[1] + rng.sample(Uniform::from(-close_distance..close_distance)),
                         ];
                    } else {
                        // Fallback if the previous feature was degenerate (shouldn't happen with this generator)
                         end_coord = [rng.sample(coord_range), rng.sample(coord_range)];
                    }
                } else {
                     // Fallback if previous feature was not a LineString (shouldn't happen with this generator)
                     end_coord = [rng.sample(coord_range), rng.sample(coord_range)];
                }
            } else {
                 // Fallback if previous feature had no geometry (shouldn't happen with this generator)
                 end_coord = [rng.sample(coord_range), rng.sample(coord_range)];
            }

        } else {
            // Generate a random endpoint
            end_coord = [rng.sample(coord_range), rng.sample(coord_range)];
        }

        // Simple properties (adjust based on your actual properties_match logic)
        let properties: Option<JsonValue> = Some(json!({
            "properties": json!({
                "objectId": i / 10, // Group features by objectId
                "constructionType": "pipe",
                "depth": 1.5,
                "width": 0.3,
                "comment": null,
                "label": format!("Line {}", i),
                "material": "PVC"
            }),
            "surface": "ground"
        }));

        let properties = properties.map(|v| v.as_object().unwrap().clone());


        features.push(create_synthetic_line_string(i, start_coord, end_coord, properties));
    }

    FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    }
}
