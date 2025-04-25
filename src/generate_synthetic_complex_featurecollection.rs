// Import other necessary geo types
use geojson::{Feature, FeatureCollection, Geometry, Value};
use rand::Rng;
use rand::prelude::*; // Import for thread_rng and Rng trait

/// Generates a single random coordinate within the specified ranges.
fn gen_coord(rng: &mut ThreadRng, x_range: (f64, f64), y_range: (f64, f64)) -> Vec<f64> {
    vec![
        rng.gen_range(x_range.0..x_range.1),
        rng.gen_range(y_range.0..y_range.1),
    ]
}

/// Generates a simple random polygon ring (exterior or interior).
/// Ensures at least 4 points (closed shape with minimum 3 unique vertices).
fn gen_polygon_ring(
    rng: &mut ThreadRng,
    x_range: (f64, f64),
    y_range: (f64, f64),
    num_points: usize,
) -> Vec<Vec<f64>> {
    let n = usize::max(num_points, 3); // Ensure at least 3 vertices before closing
    let mut coords: Vec<Vec<f64>> = (0..n).map(|_| gen_coord(rng, x_range, y_range)).collect();

    // Close the ring
    if let Some(first_coord) = coords.first().cloned() {
        coords.push(first_coord);
    } else {
        // This case should not happen if n >= 3, but handle defensively
        coords.push(gen_coord(rng, x_range, y_range)); // Add a dummy point
        coords.push(gen_coord(rng, x_range, y_range)); // Add another
        coords.push(coords[0].clone()); // Close with the first dummy
    }

    coords
}

/// Generates a synthetic GeoJSON FeatureCollection with various geometry types.
///
/// Geometries are generated randomly within the specified bounding box ranges.
/// Includes Point, LineString, Polygon, MultiPoint, MultiLineString, MultiPolygon.
/// Also includes some edge cases relevant to the benchmarked function,
/// like geometries with fewer than 3 unique points or duplicate points.
///
/// # Arguments
/// * `num_features` - The total number of features to generate.
/// * `x_range` - The (min_x, max_x) range for coordinates.
/// * `y_range` - The (min_y, max_y) range for coordinates.
///
/// # Returns
/// A `FeatureCollection` containing the generated features.
pub fn generate_synthetic_complex_featurecollection(
    num_features: usize,
    x_range: (f64, f64),
    y_range: (f64, f64),
) -> FeatureCollection {
    let mut rng = rand::thread_rng(); // Initialize a random number generator
    let mut features: Vec<Feature> = Vec::with_capacity(num_features); // Pre-allocate capacity

    // Define the types we want to generate (0 to 5)
    const NUM_GEOM_TYPES: usize = 6;

    for i in 0..num_features {
        // Randomly select a geometry type (0..NUM_GEOM_TYPES)
        let geom_type = rng.gen_range(0..NUM_GEOM_TYPES);

        let geometry = match geom_type {
            0 => {
                // Point
                let coord = gen_coord(&mut rng, x_range, y_range);
                Some(Geometry::new(Value::Point(coord)))
            }
            1 => {
                // LineString
                // Generate between 2 and 10 points. Sometimes generate only 2 points
                // to test the "less than 3 unique points" filter.
                let num_points = if rng.gen_bool(0.1) {
                    // 10% chance of 2 points
                    2
                } else {
                    rng.gen_range(3..=10)
                };
                let coords: Vec<Vec<f64>> = (0..num_points)
                    .map(|_| gen_coord(&mut rng, x_range, y_range))
                    .collect();
                Some(Geometry::new(Value::LineString(coords)))
            }
            2 => {
                // Polygon (exterior ring only for simplicity)
                // Generate a ring with 4 to 8 points (closed)
                let num_points = rng.gen_range(4..=8);
                let exterior_ring = gen_polygon_ring(&mut rng, x_range, y_range, num_points);
                Some(Geometry::new(Value::Polygon(vec![exterior_ring])))
            }
            3 => {
                // MultiPoint
                // Generate between 1 and 10 points. Sometimes generate duplicate points.
                let num_points = if rng.gen_bool(0.1) {
                    // 10% chance of 1 or 2 points total
                    rng.gen_range(1..=2)
                } else {
                    rng.gen_range(3..=10)
                };

                let mut coords: Vec<Vec<f64>> = (0..num_points)
                    .map(|_| gen_coord(&mut rng, x_range, y_range))
                    .collect();

                // Occasionally add duplicate points to test unique point handling
                if num_points > 0 && rng.gen_bool(0.2) {
                    // 20% chance of adding duplicates
                    let num_dupes = rng.gen_range(1..=usize::min(num_points, 3));
                    for _ in 0..num_dupes {
                        let idx = rng.gen_range(0..coords.len());
                        coords.push(coords[idx].clone());
                    }
                }

                Some(Geometry::new(Value::MultiPoint(coords)))
            }
            4 => {
                // MultiLineString
                // Generate between 1 and 5 LineStrings
                let num_linestrings = rng.gen_range(1..=5);
                let linestrings_coords: Vec<Vec<Vec<f64>>> = (0..num_linestrings)
                    .map(|_| {
                        // Each LineString has 2 to 5 points
                        let num_points = rng.gen_range(2..=5);
                        (0..num_points)
                            .map(|_| gen_coord(&mut rng, x_range, y_range))
                            .collect()
                    })
                    .collect();
                Some(Geometry::new(Value::MultiLineString(linestrings_coords)))
            }
            5 => {
                // MultiPolygon (exterior rings only for simplicity)
                // Generate between 1 and 3 Polygons
                let num_polygons = rng.gen_range(1..=3);
                let polygons_coords: Vec<Vec<Vec<Vec<f64>>>> = (0..num_polygons)
                    .map(|_| {
                        // Each Polygon has one exterior ring with 4 to 6 points
                        let num_points = rng.gen_range(4..=6);
                        let exterior_ring =
                            gen_polygon_ring(&mut rng, x_range, y_range, num_points);
                        vec![exterior_ring] // Polygon Value is a list of rings (exterior + interiors)
                    })
                    .collect();
                Some(Geometry::new(Value::MultiPolygon(polygons_coords)))
            }
            _ => None, // Should not happen with gen_range(0..NUM_GEOM_TYPES)
        };

        let feature = Feature {
            bbox: None,
            geometry,
            id: Some(geojson::feature::Id::Number((i as u64).into())), // Simple ID
            properties: None,
            foreign_members: None,
        };
        features.push(feature);
    }

    FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    }
}
