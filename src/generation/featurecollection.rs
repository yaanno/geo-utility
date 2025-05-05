use geo::{Coord, LineString};
use geojson::{Feature, FeatureCollection, Geometry, Value};
// Remove rand imports
// use rand::Rng;
use serde_json::Map; // Import Map from serde_json

/// Generates a single deterministic coordinate within the specified ranges
/// based on a global point counter.
/// This is a helper, potentially kept private or inline.
fn gen_coord_deterministic(counter: &mut usize, x_range: (f64, f64), y_range: (f64, f64)) -> Vec<f64> {
    let current_count = *counter;
    *counter += 1; // Increment counter for the next coordinate

    // Use a deterministic mapping based on the counter to distribute points
    // Using fractional part of multiplication with large primes for distribution
    // These primes help ensure points don't just fall on a simple grid pattern.
    const PRIME_X: f64 = 1_618_033.9887; // Related to Golden Ratio
    const PRIME_Y: f64 = 2_718_281.8284; // Related to e

    let x_progress = (current_count as f64 * PRIME_X).fract(); // Value between 0.0 and 1.0
    let y_progress = (current_count as f64 * PRIME_Y).fract(); // Value between 0.0 and 1.0

    vec![
        x_range.0 + x_progress * (x_range.1 - x_range.0),
        y_range.0 + y_progress * (y_range.1 - y_range.0),
    ]
}

/// Generates a deterministic bottom-left corner and size for a square
/// based on the feature index.
fn deterministic_square_params(
    feature_index: usize,
    x_range: (f64, f64),
    y_range: (f64, f64),
    min_size: f64,
    max_size: f64,
    margin: f64, // Ensure square fits within range minus margin
) -> (f64, f64, f64) {
    // Use feature index and primes to deterministically distribute squares
    const PRIME_OFFSET_X: f64 = 3.1415926535; // Pi
    const PRIME_OFFSET_Y: f64 = 2.7182818284; // e
    const PRIME_SIZE: f64 = 1.4142135623; // Sqrt(2)

    // Map index to position within available range using fractional part
    let available_x_range = x_range.1 - margin - x_range.0;
    let available_y_range = y_range.1 - margin - y_range.0;

    let x_offset_progress = ((feature_index as f64 * PRIME_OFFSET_X) % 1000.0 / 1000.0).fract(); // Use modulo to keep numbers from growing too large before fract
    let y_offset_progress = ((feature_index as f64 * PRIME_OFFSET_Y) % 1000.0 / 1000.0).fract();
    let size_progress = ((feature_index as f64 * PRIME_SIZE) % 1000.0 / 1000.0).fract();

    let x_min = x_range.0 + x_offset_progress * available_x_range.max(0.0); // Ensure available range is not negative
    let y_min = y_range.0 + y_offset_progress * available_y_range.max(0.0);
    let size = min_size + size_progress * (max_size - min_size);

    (x_min, y_min, size)
}


/// Generates a synthetic GeoJSON FeatureCollection for benchmarking.
/// Data generation is deterministic based on feature index and an internal counter.
///
/// The generated features include a predictable mix of Point, LineString (open and closed),
/// and Polygon geometries with deterministic coordinates within a specified range.
///
/// # Arguments
/// * `num_features` - The total number of features to generate.
/// * `x_range` - A tuple specifying the minimum and maximum x-coordinate value (e.g., (-100.0, 100.0)).
/// * `y_range` - A tuple specifying the minimum and maximum y-coordinate value (e.g., (-100.0, 100.0)).
///
/// # Returns
/// A `geojson::FeatureCollection` containing the generated synthetic data.
pub fn generate_synthetic_featurecollection(
    num_features: usize,
    x_range: (f64, f64),
    y_range: (f64, f64),
) -> FeatureCollection {
    // Remove random number generator
    // let mut rng = rand::thread_rng();
    let mut point_counter: usize = 0; // Deterministic counter for generating unique coordinates

    let mut features: Vec<Feature> = Vec::with_capacity(num_features); // Pre-allocate capacity

    for i in 0..num_features {
        // Deterministically select a geometry type based on feature index
        let geom_type = i % 4; // Cycles through Point (0), Open LineString (1), Closed LineString (2), Polygon (3)

        let geometry = match geom_type {
            0 => {
                // Point
                let coord = gen_coord_deterministic(&mut point_counter, x_range, y_range);
                Some(Geometry::new(Value::Point(coord)))
            }
            1 => {
                // Open LineString (3-5 points deterministically)
                let num_points = 3 + (i % 3); // i % 3 gives 0, 1, or 2; +3 gives 3, 4, or 5
                let coords: Vec<Vec<f64>> = (0..num_points)
                    .map(|_| gen_coord_deterministic(&mut point_counter, x_range, y_range))
                    .collect();
                Some(Geometry::new(Value::LineString(coords)))
            }
            2 => {
                // Closed LineString (simple square deterministically)
                // Ensure space for square (at least 10 units in x and y)
                let min_size = 5.0;
                let max_size = 10.0;
                let margin = max_size; // Need margin for max size square

                let (x_min, y_min, size) = deterministic_square_params(
                     i, x_range, y_range, min_size, max_size, margin
                );

                // Generate the 5 deterministic coordinates for the square
                let coords = vec![
                    vec![x_min, y_min],
                    vec![x_min + size, y_min],
                    vec![x_min + size, y_min + size],
                    vec![x_min, y_min + size],
                    vec![x_min, y_min], // Close the loop
                ];

                // You can still check if it's closed, but it will always be with deterministic math
                let line_string: LineString<f64> =
                    coords.iter().map(|c| Coord { x: c[0], y: c[1] }).collect();
                if !line_string.is_closed() {
                    // This print will now indicate an issue with the deterministic square logic
                    println!("Warning: Deterministic closed LineString was not closed!");
                }

                Some(Geometry::new(Value::LineString(coords)))
            }
            3 => {
                // Polygon (simple square, exterior ring only, deterministically)
                 // Ensure space for square (at least 10 units in x and y)
                let min_size = 5.0;
                let max_size = 10.0;
                let margin = max_size; // Need margin for max size square

                let (x_min, y_min, size) = deterministic_square_params(
                     i, x_range, y_range, min_size, max_size, margin
                );

                // Generate the 5 deterministic coordinates for the exterior ring
                let exterior_coords = vec![
                    vec![x_min, y_min],
                    vec![x_min + size, y_min],
                    vec![x_min + size, y_min + size],
                    vec![x_min, y_min + size],
                    vec![x_min, y_min], // Close the loop
                ];
                // For simplicity, we won't add inner rings in this generator.
                Some(Geometry::new(Value::Polygon(vec![exterior_coords])))
            }
            _ => None, // Should not happen with i % 4
        };

        // Properties remain deterministic based on geom_type
        let properties = match geom_type {
            0 => { // Point
                let mut props = Map::new();
                let mut inner_props = Map::new();
                inner_props.insert("objectId".to_string(), serde_json::Value::String("Kugelmarker".to_string()));
                props.insert("properties".to_string(), serde_json::Value::Object(inner_props));
                Some(props)
            }
            1 => { // Open LineString
                let mut props = Map::new();
                let mut inner_props = Map::new();
                inner_props.insert("objectId".to_string(), serde_json::Value::String("Linie".to_string()));
                props.insert("properties".to_string(), serde_json::Value::Object(inner_props));
                Some(props)
            },
            2 | 3 => { // Closed LineString (Gebaeude) or Polygon (Gebaeude)
                let mut props = Map::new();
                let mut inner_props = Map::new();
                inner_props.insert("objectId".to_string(), serde_json::Value::String("Gebaeude".to_string()));
                props.insert("properties".to_string(), serde_json::Value::Object(inner_props));
                Some(props)
            },
            _ => None, // Should not happen
        };

        let feature = Feature {
            bbox: None, // Keep None for now, could make deterministic bbox later if needed
            geometry,
            id: Some(geojson::feature::Id::Number((i as u64).into())), // Simple deterministic ID
            properties,
            foreign_members: None, // Keep None for now
        };
        features.push(feature);
    }

    FeatureCollection {
        bbox: None, // Keep None for now, could make deterministic bbox for the collection later
        features,
        foreign_members: None,
    }
}

// Example usage (can be in your main function or a test)
/*
fn main() {
    let num_features = 10; // Or 100, 1000, etc.
    let x_range = (0.0, 100.0);
    let y_range = (0.0, 100.0);

    let feature_collection = generate_synthetic_featurecollection(
        num_features,
        x_range,
        y_range,
    );

    let geojson_string = serde_json::to_string_pretty(&feature_collection).unwrap();
    println!("{}", geojson_string);
}
*/