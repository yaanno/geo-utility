use geojson::{Feature, FeatureCollection, Geometry, JsonObject, Value};
use geo::Coord;
use serde_json::json;

// Helper function to generate a synthetic LineString feature
fn create_synthetic_line_string(id: usize, coords: Vec<Vec<f64>>, properties: Option<JsonObject>) -> Feature {
    Feature {
       bbox: None,
       geometry: Some(Geometry::new(Value::LineString(coords))),
       id: Some(geojson::feature::Id::Number(id.into())), // Use usize as ID
       properties,
       foreign_members: None,
   }
}

// Helper function to generate synthetic data for benchmarking using deterministic patterns
pub fn generate_synthetic_data_collection(
   num_features: usize,
   max_vertices_per_feature: usize,
   bend_frequency: f64, // Used deterministically
   max_bend_angle_degrees: f64,
) -> FeatureCollection {
   let mut features: Vec<Feature> = Vec::with_capacity(num_features);

   for i in 0..num_features {
       let mut coords: Vec<Coord<f64>> = Vec::new();

       // Deterministic start position based on feature index
       let mut current_coord = Coord {
           x: (i as f64 % 2000.0) - 1000.0, // Simple pattern based on index
           y: (i as f64 / 2000.0 * 2000.0) - 1000.0, // Another simple pattern
       };
       coords.push(current_coord);

       // Deterministic number of vertices based on feature index
       // Ensure at least 2 vertices for a valid LineString
       let num_vertices = 2 + (i % (max_vertices_per_feature.max(2) - 1));


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

           // Apply bend if condition met and not the very first segment
           if j > 0 && bend_condition {
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

       // Convert geo::Coord to Vec<f64> for geojson Value::LineString
       let line_string_value_coords: Vec<Vec<f64>> = coords.iter().map(|c| vec![c.x, c.y]).collect();

       // Add deterministic dummy properties based on index
       let mut properties = serde_json::Map::new();
       properties.insert(
           "properties".to_string(),
           // Group objectId deterministically, e.g., every 10 features share an objectId
           json!({"objectId": format!("synthetic_{}", i / 10)}),
       );
       // Other properties can also be deterministic based on index or fixed
       properties.insert(
           "constructionType".to_string(),
           json!("pipe"),
       );
       properties.insert(
           "depth".to_string(),
           json!(1.5 + (i as f64 % 10.0) * 0.1), // Vary depth slightly
       );
        properties.insert(
           "width".to_string(),
           json!(0.3),
       );
        properties.insert(
           "comment".to_string(),
           json!(null),
       );
        properties.insert(
           "label".to_string(),
           json!(format!("Line {}", i)),
       );
        properties.insert(
           "material".to_string(),
           json!("PVC"),
       );
       // Add a large data property if needed to simulate larger properties
       properties.insert(
           "large_data".to_string(),
           json!({"filler": "a".repeat(100)}), // 100B string example
       );


       let feature = create_synthetic_line_string(i, line_string_value_coords, Some(properties));

       features.push(feature);
   }

   FeatureCollection {
       bbox: None,
       features,
       foreign_members: None,
   }
}