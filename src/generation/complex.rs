// Import other necessary geo types
use geojson::{Feature, FeatureCollection, Geometry, Value};


/// Generates a single deterministic coordinate within the specified ranges
/// based on a global point counter.
fn gen_coord_deterministic(counter: &mut usize, x_range: (f64, f64), y_range: (f64, f64)) -> Vec<f64> {
    let current_count = *counter;
    *counter += 1; // Increment counter for the next coordinate

    // Use a deterministic mapping based on the counter to distribute points
    // Using fractional part of multiplication with large primes for distribution
    const PRIME_X: f64 = 1_618_033.9887; // Related to Golden Ratio
    const PRIME_Y: f64 = 2_718_281.8284; // Related to e

    let x_progress = (current_count as f64 * PRIME_X).fract(); // Value between 0.0 and 1.0
    let y_progress = (current_count as f64 * PRIME_Y).fract(); // Value between 0.0 and 1.0

    vec![
        x_range.0 + x_progress * (x_range.1 - x_range.0),
        y_range.0 + y_progress * (y_range.1 - y_range.0),
    ]
}

/// Generates a simple deterministic polygon ring (exterior or interior).
/// Ensures at least 4 points (closed shape with minimum 3 unique vertices).
/// Uses a point counter for deterministic coordinate generation.
fn gen_polygon_ring_deterministic(
    counter: &mut usize,
    x_range: (f64, f64),
    y_range: (f64, f64),
    num_points: usize,
) -> Vec<Vec<f64>> {
    let n = usize::max(num_points, 3); // Ensure at least 3 vertices before closing
    let mut coords: Vec<Vec<f64>> = (0..n).map(|_| gen_coord_deterministic(counter, x_range, y_range)).collect();

    // Close the ring
    if let Some(first_coord) = coords.first().cloned() {
        coords.push(first_coord);
    } else {
         // Fallback, although gen_coord_deterministic should always produce a point
        let dummy1 = gen_coord_deterministic(counter, x_range, y_range);
        let dummy2 = gen_coord_deterministic(counter, x_range, y_range);
        let dummy3 = dummy1.clone(); // Close the ring deterministically
        coords.extend_from_slice(&[dummy1, dummy2, dummy3]);
    }
    coords
}

/// Calculates a deterministic bounding box from a slice of coordinates.
fn calculate_bbox_deterministic(coords: &[Vec<f64>]) -> Option<Vec<f64>> {
    if coords.is_empty() {
        None
    } else {
        let min_x = coords.iter().map(|c| c[0]).fold(f64::INFINITY, f64::min);
        let max_x = coords.iter().map(|c| c[0]).fold(f64::NEG_INFINITY, f64::max);
        let min_y = coords.iter().map(|c| c[1]).fold(f64::INFINITY, f64::min);
        let max_y = coords.iter().map(|c| c[1]).fold(f64::NEG_INFINITY, f64::max);
         // Check if min/max are still infinity/neg_infinity (could happen with NaN/Inf inputs, though gen_coord_deterministic avoids this)
        if min_x.is_infinite() || max_x.is_infinite() || min_y.is_infinite() || max_y.is_infinite() {
             None
        } else {
             Some(vec![min_x, min_y, max_x, max_y])
        }
    }
}


/// Generates a synthetic GeoJSON FeatureCollection with various geometry types.
/// Data generation is deterministic based on feature index and an internal counter.
///
/// Geometries are generated predictably within the specified bounding box ranges.
/// Includes Point, LineString, Polygon, MultiPoint, MultiLineString, MultiPolygon.
/// Includes deterministic patterns that ensure edge cases like <3 unique points
/// or duplicate points occur predictably based on feature index.
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
    let mut point_counter: usize = 0; // Deterministic counter for generating unique coordinates

    let mut features: Vec<Feature> = Vec::with_capacity(num_features); // Pre-allocate capacity

    // Define the types we want to generate (0 to 5)
    const NUM_GEOM_TYPES: usize = 6; // Point, LineString, Polygon, MultiPoint, MultiLineString, MultiPolygon

    for i in 0..num_features {
        // Deterministically select a geometry type based on feature index
        let geom_type = i % NUM_GEOM_TYPES;

        let (geometry, bbox) = match geom_type {
            0 => {
                // Point
                let coord = gen_coord_deterministic(&mut point_counter, x_range, y_range);
                // Bbox for a single point is [x, y, x, y]
                let bbox = Some(vec![coord[0], coord[1], coord[0], coord[1]]);
                (Some(Geometry::new(Value::Point(coord))), bbox)
            }
            1 => {
                // LineString
                // Deterministically generate between 2 and 10 points.
                // Ensure 2 points occur predictably (e.g., every 10th feature of this type)
                let num_points = if (i / NUM_GEOM_TYPES) % 10 == 0 {
                    2 // Deterministically generate 2 points for testing <3 unique points
                } else {
                    // Deterministically vary point count between 3 and 10
                    3 + (i % 8) // i % 8 gives 0-7, +3 gives 3-10
                };

                let coords: Vec<Vec<f64>> = (0..num_points)
                    .map(|_| gen_coord_deterministic(&mut point_counter, x_range, y_range))
                    .collect();

                let bbox = calculate_bbox_deterministic(&coords);
                (Some(Geometry::new(Value::LineString(coords))), bbox)
            }
            2 => {
                // Polygon (exterior ring only for simplicity)
                // Deterministically generate a ring with 4 to 8 points (closed)
                 let num_points = 4 + (i % 5); // i % 5 gives 0-4, +4 gives 4-8
                let exterior_ring = gen_polygon_ring_deterministic(&mut point_counter, x_range, y_range, num_points);
                let bbox = calculate_bbox_deterministic(&exterior_ring);
                (
                    Some(Geometry::new(Value::Polygon(vec![exterior_ring]))),
                    bbox,
                )
            }
            3 => {
                // MultiPoint
                // Deterministically generate between 1 and 10 points.
                // Ensure 1 or 2 points occur predictably (e.g., every 10th feature of this type)
                let num_points = if (i / NUM_GEOM_TYPES) % 10 == 0 {
                     1 + (i % 2) // Deterministically 1 or 2 points
                } else {
                    // Deterministically vary point count between 3 and 10
                    3 + (i % 8) // i % 8 gives 0-7, +3 gives 3-10
                };

                let mut coords: Vec<Vec<f64>> = (0..num_points)
                    .map(|_| gen_coord_deterministic(&mut point_counter, x_range, y_range))
                    .collect();

                // Deterministically add duplicate points (e.g., every 5th feature of this type)
                if (i / NUM_GEOM_TYPES) % 5 == 0 && num_points > 0 {
                    let num_dupes = 1 + (i % 3); // Deterministically add 1 to 3 duplicates
                    for k in 0..num_dupes {
                        // Deterministically select index to duplicate
                        let idx_to_duplicate = (i + k) % num_points;
                        coords.push(coords[idx_to_duplicate].clone());
                    }
                }

                let bbox = calculate_bbox_deterministic(&coords);
                (Some(Geometry::new(Value::MultiPoint(coords))), bbox)
            }
            4 => {
                // MultiLineString
                // Deterministically generate between 1 and 5 LineStrings
                let num_linestrings = 1 + (i % 5); // i % 5 gives 0-4, +1 gives 1-5
                let mut linestrings_coords: Vec<Vec<Vec<f64>>> = Vec::with_capacity(num_linestrings);

                for ls_idx in 0..num_linestrings {
                     // Each LineString has 2 to 5 points deterministically
                    let num_points = 2 + ((i + ls_idx) % 4); // (i+ls_idx)%4 gives 0-3, +2 gives 2-5
                    let coords: Vec<Vec<f64>> = (0..num_points)
                        .map(|_| gen_coord_deterministic(&mut point_counter, x_range, y_range))
                        .collect();
                    if !coords.is_empty() {
                        linestrings_coords.push(coords);
                    }
                }

                // Calculate bbox from all flattened coordinates
                let all_coords_flat: Vec<Vec<f64>> = linestrings_coords.iter().flatten().cloned().collect();
                let bbox = calculate_bbox_deterministic(&all_coords_flat);

                (
                    Some(Geometry::new(Value::MultiLineString(linestrings_coords))),
                    bbox,
                )
            }
            5 => {
                // MultiPolygon (exterior rings only for simplicity)
                // Deterministically generate between 1 and 3 Polygons
                let num_polygons = 1 + (i % 3); // i % 3 gives 0-2, +1 gives 1-3
                let mut polygons_coords: Vec<Vec<Vec<Vec<f64>>>> = Vec::with_capacity(num_polygons);

                for poly_idx in 0..num_polygons {
                    // Each Polygon has one exterior ring with 4 to 6 points deterministically
                    let num_points = 4 + ((i + poly_idx) % 3); // (i+poly_idx)%3 gives 0-2, +4 gives 4-6
                    let exterior_ring =
                        gen_polygon_ring_deterministic(&mut point_counter, x_range, y_range, num_points);
                     if !exterior_ring.is_empty() {
                        polygons_coords.push(vec![exterior_ring]); // Polygon Value is a list of rings (exterior + interiors)
                     }
                }

                // Calculate bbox from all flattened exterior coordinates
                 let all_coords_flat: Vec<Vec<f64>> = polygons_coords.iter().flatten().flatten().cloned().collect();
                let bbox = calculate_bbox_deterministic(&all_coords_flat);

                (
                    Some(Geometry::new(Value::MultiPolygon(polygons_coords))),
                    bbox,
                )
            }
            _ => (None, None), // Should not happen with i % NUM_GEOM_TYPES
        };

        let feature = Feature {
            bbox,
            geometry,
            id: Some(geojson::feature::Id::Number((i as u64).into())), // Simple deterministic ID
            properties: None, // Or add deterministic properties based on `i`
            foreign_members: None,
        };
        features.push(feature);
    }

    FeatureCollection {
        bbox: None, // Or calculate a deterministic bbox for the entire collection
        features,
        foreign_members: None,
    }
}
