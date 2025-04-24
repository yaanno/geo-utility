use geo::{Coord, LineString};
use geojson::{Feature, FeatureCollection, Geometry, Value};
use rand::Rng; // Import the rand crate for random number generation

/// Generates a synthetic GeoJSON FeatureCollection for benchmarking.
///
/// The generated features include a mix of Point, LineString (open and closed),
/// and Polygon geometries with random coordinates within a specified range.
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
    let mut rng = rand::thread_rng(); // Initialize a random number generator
    let mut features: Vec<Feature> = Vec::with_capacity(num_features); // Pre-allocate capacity

    for i in 0..num_features {
        // Randomly select a geometry type (0-3 for 4 types)
        let geom_type = rng.gen_range(0..4);

        let geometry = match geom_type {
            0 => {
                // Point
                let coord = vec![
                    rng.gen_range(x_range.0..x_range.1),
                    rng.gen_range(y_range.0..y_range.1),
                ];
                Some(Geometry::new(Value::Point(coord)))
            }
            1 => {
                // Open LineString (3-5 points)
                let num_points = rng.gen_range(3..=5);
                let coords: Vec<Vec<f64>> = (0..num_points)
                    .map(|_| {
                        vec![
                            rng.gen_range(x_range.0..x_range.1),
                            rng.gen_range(y_range.0..y_range.1),
                        ]
                    })
                    .collect();
                Some(Geometry::new(Value::LineString(coords)))
            }
            2 => {
                // Closed LineString (simple square)
                // Generate a random bottom-left corner
                let x_min = rng.gen_range(x_range.0..x_range.1 - 10.0); // Ensure space for square
                let y_min = rng.gen_range(y_range.0..y_range.1 - 10.0);
                let size = rng.gen_range(5.0..10.0); // Random size for the square

                let coords = vec![
                    vec![x_min, y_min],
                    vec![x_min + size, y_min],
                    vec![x_min + size, y_min + size],
                    vec![x_min, y_min + size],
                    vec![x_min, y_min], // Close the loop
                ];
                // Ensure it's actually closed, though the logic should guarantee it
                let line_string: LineString<f64> =
                    coords.iter().map(|c| Coord { x: c[0], y: c[1] }).collect();
                if !line_string.is_closed() {
                    // This should ideally not happen with the square logic, but as a fallback
                    println!("Warning: Generated closed LineString was not closed!");
                }

                Some(Geometry::new(Value::LineString(coords)))
            }
            3 => {
                // Polygon (simple square, exterior ring only)
                // Generate a random bottom-left corner
                let x_min = rng.gen_range(x_range.0..x_range.1 - 10.0); // Ensure space for square
                let y_min = rng.gen_range(y_range.0..y_range.1 - 10.0);
                let size = rng.gen_range(5.0..10.0); // Random size for the square

                let exterior_coords = vec![
                    vec![x_min, y_min],
                    vec![x_min + size, y_min],
                    vec![x_min + size, y_min + size],
                    vec![x_min, y_min + size],
                    vec![x_min, y_min], // Close the loop
                ];
                // For simplicity, we won't add inner rings in this generator,
                // as your current scaling function ignores them anyway.
                Some(Geometry::new(Value::Polygon(vec![exterior_coords])))
            }
            _ => None, // Should not happen with gen_range(0..4)
        };

        let feature = Feature {
            bbox: None, // Can generate random bboxes if needed, but not essential for scaling benchmark
            geometry,
            id: Some(geojson::feature::Id::Number((i as u64).into())), // Simple ID
            properties: None,      // Can add random properties if needed
            foreign_members: None, // Can add random foreign members if needed
        };
        features.push(feature);
    }

    FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    }
}

/*
// Example usage for benchmarking:
#[cfg(test)] // Or in your main benchmarking code
mod bench_data_gen {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_generate_synthetic_data() {
        let num_features = 1000;
        let x_range = (-1000.0, 1000.0);
        let y_range = (-1000.0, 1000.0);

        let start_time = Instant::now();
        let feature_collection = generate_synthetic_data(num_features, x_range, y_range);
        let duration = start_time.elapsed();

        println!("Generated {} features in {:?}", num_features, duration);
        println!("First 5 features: {:?}", &feature_collection.features[0..5]); // Print a few to inspect structure

        assert_eq!(feature_collection.features.len(), num_features);
        // Add more assertions to check geometry types distribution if needed
    }

    // Example of how you might use it for benchmarking your scale_buildings function
    // This would typically be in a dedicated benchmarking file using `cargo bench`
    /*
    #[bench] // Requires #[feature(test)] at the crate root and `test = true` in Cargo.toml
    fn bench_scale_buildings_large_data(b: &mut test::Bencher) {
        let num_features = 100_000; // Adjust for desired scale
        let x_range = (-1000.0, 1000.0);
        let y_range = (-1000.0, 1000.0);
        let synthetic_data = generate_synthetic_data(num_features, x_range, y_range);
        let scale_factor = 0.9;

        b.iter(|| {
            // Benchmark the scale_buildings function
            let scaled_collection = super::scale_buildings(&synthetic_data, scale_factor);
            // Optionally assert something about the output to ensure correctness isn't sacrificed for speed
            // assert_eq!(scaled_collection.features.len(), num_features); // Or check geometry counts
        });
    }
    */
}
*/
