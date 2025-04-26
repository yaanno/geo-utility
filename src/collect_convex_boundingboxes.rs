// Collects convex bounding boxes from a geojson FeatureCollection.

use geo::algorithm::convex_hull::ConvexHull; // Import the ConvexHull trait
use geo::geometry::{LineString as GeoLineString, MultiPoint};
use geo::{BoundingRect, Coord, Intersects, Point, Rect}; // Import other necessary geo types
use geojson::{FeatureCollection, Value};
use ordered_float::OrderedFloat;
use std::collections::HashSet;
use thiserror::Error;

const GERMANY_BBOX: [f64; 4] = [
    5.866211,  // Min longitude
    47.270111, // Min latitude
    15.013611, // Max longitude
    55.058333, // Max latitude
];

// Define error type (kept as is, though less frequently returned now)
#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid geometry type")]
    UnsupportedGeometryType, // This specific error is no longer returned by the main function flow
    #[error("Missing geometry")]
    MissingGeometry, // This specific error is no longer returned by the main function flow
    #[error("Invalid coordinates")]
    InvalidCoordinates, // This specific error is no longer returned by the main function flow
}

/// Creates a canonical representation of polygon points for hashing purposes.
///
/// Uses unique sorted points from the exterior ring of the polygon to create
/// a consistent representation that can be used for comparison and hashing.
///
/// # Arguments
/// * `hull` - The polygon for which to create the canonical representation.
///
/// # Returns
/// A vector of tuples representing the unique sorted points of the polygon.
fn canonical_hull_unique_sorted_points(
    hull: &geo::Polygon,
) -> Vec<(OrderedFloat<f64>, OrderedFloat<f64>)> {
    let coords: Vec<geo::Coord> = hull.exterior().coords().cloned().collect();

    // Use a HashSet to get unique points represented as OrderedFloat tuples
    // Collect unique points into a vector and sort it
    let mut sorted_unique: Vec<(OrderedFloat<f64>, OrderedFloat<f64>)> = coords
        .into_iter()
        .map(|c| (OrderedFloat(c.x), OrderedFloat(c.y)))
        .collect::<HashSet<_>>() // Collect unique points into a HashSet first
        .into_iter() // Iterate the unique points
        .collect(); // Collect into a Vec

    sorted_unique.sort(); // Sorting tuples of OrderedFloat works naturally

    sorted_unique
}

/// Collects convex bounding boxes and fallback bounding box polygons
/// from a geojson FeatureCollection.
///
/// # Arguments
/// * `featurecollection` - The FeatureCollection from which to collect bounding box polygons.
///
/// # Returns
/// A vector of polygons, including convex hulls for features with >= 3 unique points
/// and bounding box polygons for features with < 3 unique points that can form a bounding box.
/// # Errors
/// Returns an error if the geometry type is unsupported or if there are invalid coordinates.
pub fn collect_convex_boundingboxes(
    // Renamed the function to reflect fallback
    featurecollection: &FeatureCollection,
) -> Result<Vec<geo::Polygon>, Error> {
    let mut hulls: Vec<geo::Polygon> = Vec::new();
    let germany_rect: Rect = Rect::new(
        Coord {
            x: 5.866211,
            y: 47.270111,
        },
        Coord {
            x: 15.013611,
            y: 55.058333,
        },
    );

    for feature in &featurecollection.features {
        // --- Early Filtering using Feature Bounding Box ---
        if let Some(feature_bbox_value) = &feature.bbox {
            if feature_bbox_value.len() >= 4 {
                let feature_rect = Rect::new(
                    Coord {
                        x: feature_bbox_value[0],
                        y: feature_bbox_value[1],
                    },
                    Coord {
                        x: feature_bbox_value[2],
                        y: feature_bbox_value[3],
                    },
                );

                if !feature_rect.intersects(&germany_rect) {
                    continue;
                }
            }
        }

        // --- Extract geometry and check location based on type ---
        let geometry_value = match feature.geometry.as_ref() {
            Some(geometry) => &geometry.value,
            None => {
                continue; // Skip features without geometry
            }
        };

        let mut coords: Vec<Coord> = Vec::new();
        let mut all_points_in_germany = true;
        let mut geometry_to_process: Option<geo::Geometry> = None; // Store geo::Geometry for fallback bbox calculation

        // Extract coordinates and check location based on geometry type
        // Also create geo::Geometry for fallback bounding_rect calculation
        match geometry_value {
            Value::Point(coord) => {
                let geo_coord = Coord {
                    x: coord[0],
                    y: coord[1],
                };
                if is_coordinate_in_germany(&coord) {
                    coords.push(geo_coord); // Use for unique count check
                    geometry_to_process = Some(geo::Geometry::Point(Point::from(geo_coord))); // Create geo::Point for bbox
                } else {
                    all_points_in_germany = false;
                }
            }
            Value::LineString(line_coords) => {
                let line_coords_geo: Vec<Coord> = line_coords
                    .iter()
                    .map(|c| Coord { x: c[0], y: c[1] })
                    .collect();
                if line_coords_geo
                    .iter()
                    .any(|c| !is_coordinate_in_germany(&[c.x, c.y]))
                {
                    all_points_in_germany = false;
                } else {
                    coords.extend(line_coords_geo.clone()); // Use for unique count check
                    geometry_to_process = Some(geo::Geometry::LineString(GeoLineString::new(
                        line_coords_geo,
                    ))); // Create geo::LineString for bbox
                }
            }
            Value::Polygon(polygon_coords) => {
                // Extract coords from exterior ring; interior rings don't affect convex hull
                if let Some(exterior_ring_coords) = polygon_coords.first() {
                    let exterior_ring_geo_coords: Vec<Coord> = exterior_ring_coords
                        .iter()
                        .map(|c| Coord { x: c[0], y: c[1] })
                        .collect();
                    if exterior_ring_geo_coords
                        .iter()
                        .any(|c| !is_coordinate_in_germany(&[c.x, c.y]))
                    {
                        all_points_in_germany = false;
                    } else {
                        coords.extend(exterior_ring_geo_coords); // Use for unique count check
                        // For Polygon, we'll likely calculate convex hull directly if >=3 points
                        // No need to store the full polygon geometry_to_process here unless needed elsewhere
                    }
                } else {
                    // Polygon has no exterior ring, skip
                    continue;
                }
            }
            // Add other geometry types here (MultiPoint, MultiLineString, MultiPolygon)
            // Extract their coordinates, check if in Germany, and if applicable,
            // create the corresponding geo::Geometry value for geometry_to_process.
            Value::MultiPoint(point_coords_vec) => {
                let multipoint_geo_coords: Vec<Coord> = point_coords_vec
                    .iter()
                    .map(|c| Coord { x: c[0], y: c[1] })
                    .collect();
                if multipoint_geo_coords
                    .iter()
                    .any(|c| !is_coordinate_in_germany(&[c.x, c.y]))
                {
                    all_points_in_germany = false;
                } else {
                    coords.extend(multipoint_geo_coords.clone()); // Use for unique count check
                    if !multipoint_geo_coords.is_empty() {
                        geometry_to_process = Some(geo::Geometry::MultiPoint(MultiPoint::new(
                            multipoint_geo_coords.into_iter().map(Point::from).collect(),
                        )));
                    }
                }
            }
            Value::MultiLineString(multiline_coords_vec) => {
                let multiline_geo_coords: Vec<Coord> = multiline_coords_vec
                    .iter()
                    .flatten()
                    .map(|c| Coord { x: c[0], y: c[1] })
                    .collect();
                if multiline_geo_coords
                    .iter()
                    .any(|c| !is_coordinate_in_germany(&[c.x, c.y]))
                {
                    all_points_in_germany = false;
                } else {
                    coords.extend(multiline_geo_coords.clone()); // Use for unique count check
                    // Creating a geo::MultiLineString from flatten coords is tricky,
                    // but we can use the flatten coords for convex hull/bbox.
                    // Or process each LineString in MultiLineString individually if preferred.
                    // For a simple bbox fallback, we can use the flattened coords.
                    if !multiline_geo_coords.is_empty() {
                        // Note: Creating a single LineString from flattened MultiLineString coords might not preserve structure
                        // but works for overall bbox. Consider iterating MultiLineString sections if precise per-line bbox needed.
                        geometry_to_process = Some(geo::Geometry::LineString(GeoLineString::new(
                            multiline_geo_coords,
                        ))); // Using LineString for simplicity here, bbox is the same
                    }
                }
            }
            Value::MultiPolygon(multipolygon_coords_vec) => {
                let all_exterior_coords: Vec<Coord> = multipolygon_coords_vec
                    .iter()
                    .filter_map(|poly| poly.first())
                    .flatten()
                    .map(|c| Coord { x: c[0], y: c[1] })
                    .collect();

                if all_exterior_coords
                    .iter()
                    .any(|c| !is_coordinate_in_germany(&[c.x, c.y]))
                {
                    all_points_in_germany = false;
                } else {
                    coords.extend(all_exterior_coords.clone()); // Use for unique count check
                    // Similar to MultiLineString, creating the full geo::MultiPolygon is more complex
                    // We'll rely on extracted coords for hull/bbox
                }
            }
            _ => {
                // Skip unsupported geometry types
                continue;
            }
        }

        // 2. Check if all relevant points were in Germany
        if !all_points_in_germany {
            continue; // Skip features that were not entirely within Germany bbox
        }

        // 3. Check number of *unique* points derived from the geometry
        let unique_coords_count = coords
            .iter()
            .map(|c| (OrderedFloat(c.x), OrderedFloat(c.y)))
            .collect::<HashSet<_>>()
            .len();

        // *** MODIFIED LOGIC FOR FALLBACK OR CONVEX HULL ***
        if unique_coords_count < 3 {
            // --- FALLBACK: Calculate bounding box and convert to polygon ---
            // Need to handle the case where `geometry_to_process` might not have been set
            // for some geometry types or empty collections resulted in all_points_in_germany = true but no coords
            let fallback_polygon = geometry_to_process
                .as_ref() // Get a reference to the geo::Geometry
                .and_then(|geom| geom.bounding_rect()) // Try to get the bounding Rect
                .map(|rect| rect.to_polygon()); // Convert the Rect to a Polygon

            if let Some(poly) = fallback_polygon {
                hulls.push(poly);
            }
            // else: If bounding_rect() failed (e.g. empty geometry or invalid), we just don't add anything for this feature.
        } else {
            // --- CONVEX HULL (Original Logic) ---
            // Compute the convex hull (use the original `coords` list which might have > unique_coords_count points)
            let multi_point = MultiPoint::from(coords);
            let hull = multi_point.convex_hull(); // This will be a geo::Polygon because unique_coords_count >= 3

            hulls.push(hull); // Collect the computed hull
        }
    }

    // --- Post-processing step: Filter out duplicate hulls (including fallback ones) ---
    let mut unique_hulls: Vec<geo::Polygon> = Vec::with_capacity(hulls.len());
    let mut seen_canonical_coords: HashSet<Vec<(OrderedFloat<f64>, OrderedFloat<f64>)>> =
        HashSet::with_capacity(hulls.len());

    for hull in hulls {
        let canonical_coords_hashable = canonical_hull_unique_sorted_points(&hull);

        if seen_canonical_coords.insert(canonical_coords_hashable) {
            unique_hulls.push(hull);
        }
    }
    // --- End post-processing step ---

    Ok(unique_hulls)
}

/// Checks if a coordinate is within the bounding box of Germany.
///
/// # Arguments
/// * `coord` - The coordinate to check.
/// # Returns
/// `true` if the coordinate is within the bounding box, `false` otherwise.
fn is_coordinate_in_germany(coord: &[f64]) -> bool {
    // Ensure coord has at least 2 elements (longitude, latitude)
    if coord.len() < 2 {
        return false; // Cannot check if coords are invalid
    }

    coord[0] >= GERMANY_BBOX[0]
        && coord[0] <= GERMANY_BBOX[2]
        && coord[1] >= GERMANY_BBOX[1]
        && coord[1] <= GERMANY_BBOX[3]
}

#[allow(dead_code)]
fn convert_polygons_to_bounding_boxes(polygons: Vec<geo::Polygon>) -> Vec<geo::Rect> {
    polygons
        .into_iter()
        .map(|poly| poly.bounding_rect().unwrap())
        .collect()
}

#[allow(dead_code)]
/// Extends a bounding box to ensure it is a multiple of a given arealength.
///
/// # Arguments
/// * `bbox` - The bounding box to extend.
/// * `arealength` - The arealength to which the bounding box should be extended.
/// # Returns
/// The extended bounding box.
fn extend_bounding_box(bbox: geo::Rect, arealength: f64) -> geo::Rect {
    let width = bbox.width();
    let height = bbox.height();

    let length_width = if width < arealength {
        arealength + 1.0 - width
    } else {
        arealength + 1.0 - (width % arealength)
    };

    let length_height = if height < arealength {
        arealength + 1.0 - height
    } else {
        arealength + 1.0 - (height % arealength)
    };

    let new_min_x = bbox.min().x - length_width / 2.0;
    let new_min_y = bbox.min().y - length_height / 2.0;
    let new_max_x = bbox.max().x + length_width / 2.0;
    let new_max_y = bbox.max().y + length_height / 2.0;

    geo::Rect::new(
        geo::Coord {
            x: new_min_x,
            y: new_min_y,
        },
        geo::Coord {
            x: new_max_x,
            y: new_max_y,
        },
    )
}

// Assume your create_square_grid function is defined here
#[allow(dead_code)]
/// Creates a square grid from a bounding box with specified cell dimensions.
///
/// # Arguments
/// * `bbox` - The bounding box from which to create the grid.
/// * `cell_width` - The width of each cell in the grid.
/// * `cell_height` - The height of each cell in the grid.
/// # Returns
/// A vector of rectangles representing the grid cells.
fn create_square_grid(bbox: geo::Rect, cell_width: f64, cell_height: f64) -> Vec<geo::Rect> {
    let mut grid = Vec::new();

    // Ensure cell dimensions are positive to avoid infinite loops
    if cell_width <= 0.0 || cell_height <= 0.0 {
        return grid;
    }

    let bbox_min_x = bbox.min().x;
    let bbox_min_y = bbox.min().y;
    let bbox_max_x = bbox.max().x;
    let bbox_max_y = bbox.max().y;

    // Handle cases where bbox is degenerate or inverted (max <= min)
    if bbox_min_x >= bbox_max_x || bbox_min_y >= bbox_max_y {
        return grid;
    }

    let mut x = bbox_min_x;
    while x < bbox_max_x {
        let mut y = bbox_min_y;
        while y < bbox_max_y {
            // Calculate the top-right corner of the current cell
            // Clip to the bounding box's maximum x and y if necessary
            let current_cell_max_x = f64::min(x + cell_width, bbox_max_x);
            let current_cell_max_y = f64::min(y + cell_height, bbox_max_y);

            // Create the rectangle for the current cell
            let cell_rect = geo::Rect::new(
                geo::Coord { x, y },
                geo::Coord {
                    x: current_cell_max_x,
                    y: current_cell_max_y,
                },
            );

            // Add the cell to the grid
            grid.push(cell_rect);

            // Move the y-cursor to the start of the next row
            y += cell_height;
        }
        // Move the x-cursor to the start of the next column
        x += cell_width;
    }

    grid
}

#[allow(unused_imports)]
mod tests {
    use super::*;
    use geo::{LineString, MultiPoint, Point, Polygon};
    use geojson::{Feature, FeatureCollection, Value};
    use ordered_float::OrderedFloat;
    use std::collections::HashSet;

    // Helper to create Rects more concisely in tests
    #[allow(unused)]
    fn r(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Rect {
        Rect::new(Coord { x: min_x, y: min_y }, Coord { x: max_x, y: max_y })
    }
    #[test]
    fn test_create_square_grid_exact_fit_single_cell() {
        let bbox = r(0.0, 0.0, 200.0, 200.0);
        let cell_width = 200.0;
        let cell_height = 200.0;

        let expected_grid = vec![r(0.0, 0.0, 200.0, 200.0)];

        let grid = create_square_grid(bbox, cell_width, cell_height);
        assert_eq!(grid.len(), expected_grid.len());
        assert_eq!(grid, expected_grid);
    }

    #[test]
    fn test_create_square_grid_exact_fit_multiple_cells() {
        let bbox = r(0.0, 0.0, 400.0, 400.0);
        let cell_width = 200.0;
        let cell_height = 200.0;

        let expected_grid = vec![
            r(0.0, 0.0, 200.0, 200.0),
            r(0.0, 200.0, 200.0, 400.0),
            r(200.0, 0.0, 400.0, 200.0),
            r(200.0, 200.0, 400.0, 400.0),
        ];

        let grid = create_square_grid(bbox, cell_width, cell_height);

        // Sort both vectors for reliable comparison, as the order might depend on loop implementation
        let mut grid_sorted = grid;
        grid_sorted.sort_by(|a, b| {
            a.min()
                .x
                .partial_cmp(&b.min().x)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(
                    a.min()
                        .y
                        .partial_cmp(&b.min().y)
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
        });
        let mut expected_grid_sorted = expected_grid;
        expected_grid_sorted.sort_by(|a, b| {
            a.min()
                .x
                .partial_cmp(&b.min().x)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(
                    a.min()
                        .y
                        .partial_cmp(&b.min().y)
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
        });

        assert_eq!(grid_sorted.len(), expected_grid_sorted.len());
        assert_eq!(grid_sorted, expected_grid_sorted);
    }

    #[test]
    fn test_create_square_grid_clipping_required() {
        let bbox = r(0.0, 0.0, 450.0, 350.0);
        let cell_width = 200.0;
        let cell_height = 200.0;

        let expected_grid = vec![
            // Row 1 (y=0)
            r(0.0, 0.0, 200.0, 200.0),
            r(200.0, 0.0, 400.0, 200.0),
            r(400.0, 0.0, 450.0, 200.0), // Clipped x
            // Row 2 (y=200)
            r(0.0, 200.0, 200.0, 350.0),   // Clipped y
            r(200.0, 200.0, 400.0, 350.0), // Clipped y
            r(400.0, 200.0, 450.0, 350.0), // Clipped x and y
        ];

        let grid = create_square_grid(bbox, cell_width, cell_height);

        // Sort both vectors for reliable comparison
        let mut grid_sorted = grid;
        grid_sorted.sort_by(|a, b| {
            a.min()
                .x
                .partial_cmp(&b.min().x)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(
                    a.min()
                        .y
                        .partial_cmp(&b.min().y)
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
        });
        let mut expected_grid_sorted = expected_grid;
        expected_grid_sorted.sort_by(|a, b| {
            a.min()
                .x
                .partial_cmp(&b.min().x)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(
                    a.min()
                        .y
                        .partial_cmp(&b.min().y)
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
        });

        assert_eq!(grid_sorted.len(), expected_grid_sorted.len());
        assert_eq!(grid_sorted, expected_grid_sorted);
    }

    #[test]
    fn test_create_square_grid_bbox_smaller_than_cell() {
        let bbox = r(10.0, 20.0, 50.0, 70.0);
        let cell_width = 200.0;
        let cell_height = 200.0;

        // Should result in a single cell clipped to the bbox
        let expected_grid = vec![r(10.0, 20.0, 50.0, 70.0)];

        let grid = create_square_grid(bbox, cell_width, cell_height);
        assert_eq!(grid.len(), expected_grid.len());
        assert_eq!(grid, expected_grid);
    }

    #[test]
    fn test_create_square_grid_zero_cell_width() {
        let bbox = r(0.0, 0.0, 200.0, 200.0);
        let cell_width = 0.0;
        let cell_height = 200.0;

        let expected_grid: Vec<Rect> = vec![]; // Should return empty grid

        let grid = create_square_grid(bbox, cell_width, cell_height);
        assert_eq!(grid.len(), expected_grid.len());
        assert_eq!(grid, expected_grid);
    }

    #[test]
    fn test_create_square_grid_zero_cell_height() {
        let bbox = r(0.0, 0.0, 200.0, 200.0);
        let cell_width = 200.0;
        let cell_height = 0.0;

        let expected_grid: Vec<Rect> = vec![]; // Should return empty grid

        let grid = create_square_grid(bbox, cell_width, cell_height);
        assert_eq!(grid.len(), expected_grid.len());
        assert_eq!(grid, expected_grid);
    }

    #[test]
    fn test_create_square_grid_zero_bbox_area() {
        let bbox = r(0.0, 0.0, 0.0, 0.0); // Point-like bbox
        let cell_width = 10.0;
        let cell_height = 10.0;

        let expected_grid: Vec<Rect> = vec![]; // Should return empty grid

        let grid = create_square_grid(bbox, cell_width, cell_height);
        assert_eq!(grid.len(), expected_grid.len());
        assert_eq!(grid, expected_grid);
    }

    #[test]
    fn test_create_square_grid_negative_coords() {
        let bbox = r(-100.0, -100.0, 100.0, 100.0);
        let cell_width = 50.0;
        let cell_height = 50.0;

        let expected_grid = vec![
            // Row 1 (y=-100)
            r(-100.0, -100.0, -50.0, -50.0),
            r(-50.0, -100.0, 0.0, -50.0),
            r(0.0, -100.0, 50.0, -50.0),
            r(50.0, -100.0, 100.0, -50.0),
            // Row 2 (y=-50)
            r(-100.0, -50.0, -50.0, 0.0),
            r(-50.0, -50.0, 0.0, 0.0),
            r(0.0, -50.0, 50.0, 0.0),
            r(50.0, -50.0, 100.0, 0.0),
            // Row 3 (y=0)
            r(-100.0, 0.0, -50.0, 50.0),
            r(-50.0, 0.0, 0.0, 50.0),
            r(0.0, 0.0, 50.0, 50.0),
            r(50.0, 0.0, 100.0, 50.0),
            // Row 4 (y=50)
            r(-100.0, 50.0, -50.0, 100.0),
            r(-50.0, 50.0, 0.0, 100.0),
            r(0.0, 50.0, 50.0, 100.0),
            r(50.0, 50.0, 100.0, 100.0),
        ];

        let grid = create_square_grid(bbox, cell_width, cell_height);

        // Sort both vectors for reliable comparison
        let mut grid_sorted = grid;
        grid_sorted.sort_by(|a, b| {
            a.min()
                .x
                .partial_cmp(&b.min().x)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(
                    a.min()
                        .y
                        .partial_cmp(&b.min().y)
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
        });
        let mut expected_grid_sorted = expected_grid;
        expected_grid_sorted.sort_by(|a, b| {
            a.min()
                .x
                .partial_cmp(&b.min().x)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(
                    a.min()
                        .y
                        .partial_cmp(&b.min().y)
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
        });

        assert_eq!(grid_sorted.len(), expected_grid_sorted.len());
        assert_eq!(grid_sorted, expected_grid_sorted);
    }

    // --- Helper for comparing polygon equality in tests ---
    // Reuse the function defined in the main module for canonical representation
    // Add debug prints here to see what vectors are being compared on failure

    #[allow(dead_code)]
    fn are_polygons_geometrically_equal(poly1: &Polygon, poly2: &Polygon) -> bool {
        let canon1 = canonical_hull_unique_sorted_points(poly1);
        let canon2 = canonical_hull_unique_sorted_points(poly2);

        // Add debug prints here
        if canon1 != canon2 {
            println!("--- Canonical Comparison Failed ---");
            println!("Expected Canonical: {:?}", canon2);
            println!("Returned Canonical: {:?}", canon1); // Note: canon1 is from the returned hull, canon2 is from the expected hull
            println!("-----------------------------------");
        }

        canon1 == canon2
    }

    // --- Basic Helper Function Test ---
    #[test]
    fn test_is_coordinate_in_germany() {
        // Point inside Germany BBOX
        assert!(is_coordinate_in_germany(&[10.0, 50.0]));
        // Point outside Germany BBOX
        assert!(!is_coordinate_in_germany(&[0.0, 0.0]));
        assert!(!is_coordinate_in_germany(&[20.0, 50.0])); // Outside max longitude
        assert!(!is_coordinate_in_germany(&[10.0, 40.0])); // Outside min latitude
        assert!(!is_coordinate_in_germany(&[10.0, 60.0])); // Outside max latitude
        assert!(!is_coordinate_in_germany(&[0.0])); // Malformed coord
    }

    // --- Tests for Empty or Skipped Inputs ---

    #[test]
    fn test_collect_convex_boundingboxes_empty_collection_produces_empty() {
        let featurecollection = FeatureCollection {
            bbox: None,
            features: vec![], // Empty features list
            foreign_members: None,
        };
        let result = collect_convex_boundingboxes(&featurecollection);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().len(),
            0,
            "Expected empty result for empty collection"
        );
    }

    #[test]
    fn test_collect_convex_boundingboxes_features_outside_germany_produce_empty() {
        let featurecollection = FeatureCollection {
            bbox: None,
            features: vec![
                // Point outside Germany
                Feature {
                    bbox: None,
                    geometry: Some(geojson::Geometry {
                        bbox: None,
                        value: Value::Point(vec![0.0, 0.0]),
                        foreign_members: None,
                    }),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
                // LineString outside Germany
                Feature {
                    bbox: None,
                    geometry: Some(geojson::Geometry {
                        bbox: None,
                        value: Value::LineString(vec![
                            vec![0.0, 0.0],
                            vec![1.0, 1.0],
                            vec![0.0, 1.0],
                        ]),
                        foreign_members: None,
                    }),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
                // Polygon outside Germany
                Feature {
                    bbox: None,
                    geometry: Some(geojson::Geometry {
                        bbox: None,
                        value: Value::Polygon(vec![vec![
                            vec![0.0, 0.0],
                            vec![1.0, 0.0],
                            vec![1.0, 1.0],
                            vec![0.0, 0.0],
                        ]]),
                        foreign_members: None,
                    }),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
                // MultiPoint outside Germany
                Feature {
                    bbox: None,
                    geometry: Some(geojson::Geometry {
                        bbox: None,
                        value: Value::MultiPoint(vec![vec![0.0, 0.0], vec![1.0, 1.0]]),
                        foreign_members: None,
                    }),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
            ],
            foreign_members: None,
        };
        let result = collect_convex_boundingboxes(&featurecollection);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().len(),
            0,
            "Expected empty result when all features are outside Germany"
        );
    }

    #[test]
    fn test_collect_convex_boundingboxes_features_with_insufficient_points_produce_bboxes() {
        let featurecollection = FeatureCollection {
            bbox: None,
            features: vec![
                // Point in Germany (1 point -> bounding box)
                Feature {
                    bbox: None,
                    geometry: Some(geojson::Geometry {
                        bbox: None,
                        value: Value::Point(vec![10.0, 50.0]),
                        foreign_members: None,
                    }),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
                // LineString in Germany (2 points -> bounding box)
                Feature {
                    bbox: None,
                    geometry: Some(geojson::Geometry {
                        bbox: None,
                        value: Value::LineString(vec![vec![10.0, 50.0], vec![11.0, 51.0]]),
                        foreign_members: None,
                    }),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
                // LineString in Germany (2 identical points -> should be skipped)
                Feature {
                    bbox: None,
                    geometry: Some(geojson::Geometry {
                        bbox: None,
                        value: Value::LineString(vec![vec![10.0, 50.0], vec![10.0, 50.0]]),
                        foreign_members: None,
                    }),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
                // MultiPoint in Germany (2 points -> bounding box)
                Feature {
                    bbox: None,
                    geometry: Some(geojson::Geometry {
                        bbox: None,
                        value: Value::MultiPoint(vec![vec![10.0, 50.0], vec![11.0, 51.0]]),
                        foreign_members: None,
                    }),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
                // Polygon in Germany with only 2 unique points (invalid geojson, should produce bounding box)
                Feature {
                    bbox: None,
                    geometry: Some(geojson::Geometry {
                        bbox: None,
                        value: Value::Polygon(vec![vec![
                            vec![10.0, 50.0],
                            vec![11.0, 51.0],
                            vec![10.0, 50.0],
                        ]]),
                        foreign_members: None,
                    }),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
            ],
            foreign_members: None,
        };
        let result = collect_convex_boundingboxes(&featurecollection);
        assert!(result.is_ok());
        let hulls = result.unwrap();

        // We expect 2 unique bounding boxes:
        // 1. Point at [10.0, 50.0]
        // 2. Box from [10.0, 50.0] to [11.0, 51.0] (shared by LineString, MultiPoint, and Polygon)
        // The LineString with identical points should be skipped
        assert_eq!(
            hulls.len(),
            2,
            "Expected two unique bounding boxes from features with insufficient points"
        );
    }

    #[test]
    fn test_collect_convex_boundingboxes_unsupported_geometry_types_produce_empty_or_correct_subset()
     {
        let featurecollection = FeatureCollection {
            bbox: None,
            features: vec![
                // Unsupported type (e.g., GeometryCollection)
                Feature {
                    bbox: None,
                    geometry: Some(geojson::Geometry {
                        bbox: None,
                        value: Value::GeometryCollection(vec![]),
                        foreign_members: None,
                    }),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
                // A valid feature that should be processed
                Feature {
                    bbox: None,
                    geometry: Some(geojson::Geometry {
                        bbox: None,
                        value: Value::LineString(vec![
                            vec![10.0, 50.0],
                            vec![11.0, 50.0],
                            vec![10.5, 51.0],
                        ]),
                        foreign_members: None,
                    }),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
            ],
            foreign_members: None,
        };
        let result = collect_convex_boundingboxes(&featurecollection);
        assert!(result.is_ok());
        // Should only get the hull from the valid LineString
        assert_eq!(
            result.unwrap().len(),
            1,
            "Expected only hull from valid geometry, skipping unsupported"
        );
    }

    #[test]
    fn test_collect_convex_boundingboxes_features_partly_outside_germany_produce_empty() {
        let featurecollection = FeatureCollection {
            bbox: None,
            features: vec![
                // LineString crossing border
                Feature {
                    bbox: None,
                    geometry: Some(geojson::Geometry {
                        bbox: None,
                        value: Value::LineString(vec![vec![10.0, 50.0], vec![16.0, 50.0]]),
                        foreign_members: None,
                    }),
                    id: None,
                    properties: None,
                    foreign_members: None,
                }, // 10,50 inside, 16,50 outside
                // Polygon crossing border
                Feature {
                    bbox: None,
                    geometry: Some(geojson::Geometry {
                        bbox: None,
                        value: Value::Polygon(vec![vec![
                            vec![10.0, 50.0],
                            vec![16.0, 50.0],
                            vec![16.0, 51.0],
                            vec![10.0, 50.0],
                        ]]),
                        foreign_members: None,
                    }),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
            ],
            foreign_members: None,
        };
        let result = collect_convex_boundingboxes(&featurecollection);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().len(),
            0,
            "Expected empty result when features are only partly inside Germany"
        );
    }

    // --- Tests for Successful Hull Generation ---

    #[test]
    fn test_collect_convex_boundingboxes_single_linestring_in_germany_produces_one_hull() {
        let featurecollection = FeatureCollection {
            bbox: None,
            features: vec![Feature {
                bbox: None,
                geometry: Some(geojson::Geometry {
                    bbox: None,
                    value: Value::LineString(vec![
                        vec![10.0, 50.0],
                        vec![11.0, 50.0],
                        vec![10.5, 51.0],
                    ]), // 3 points forming a triangle in Germany
                    foreign_members: None,
                }),
                id: None,
                properties: None,
                foreign_members: None,
            }],
            foreign_members: None,
        };
        let result = collect_convex_boundingboxes(&featurecollection);
        assert!(result.is_ok());
        let hulls = result.unwrap();
        assert_eq!(
            hulls.len(),
            1,
            "Expected one hull from a valid LineString in Germany"
        );

        // Optional: Check the vertices of the resulting hull
        let expected_vertices = vec![
            Coord { x: 10.0, y: 50.0 },
            Coord { x: 11.0, y: 50.0 },
            Coord { x: 10.5, y: 51.0 },
            Coord { x: 10.0, y: 50.0 }, // Closed ring
        ];
        // Convert expected vertices to canonical hashable form for comparison
        let expected_hull = Polygon::new(LineString::new(expected_vertices), vec![]);
        assert!(
            are_polygons_geometrically_equal(&hulls[0], &expected_hull),
            "Resulting hull does not match expected geometry"
        );
    }

    #[test]
    fn test_collect_convex_boundingboxes_single_polygon_in_germany_produces_one_hull() {
        let featurecollection = FeatureCollection {
            bbox: None,
            features: vec![Feature {
                bbox: None,
                geometry: Some(geojson::Geometry {
                    bbox: None,
                    value: Value::Polygon(vec![vec![
                        vec![10.0, 50.0],
                        vec![11.0, 50.0],
                        vec![11.0, 51.0],
                        vec![10.0, 51.0],
                        vec![10.0, 50.0],
                    ]]), // Square in Germany
                    foreign_members: None,
                }),
                id: None,
                properties: None,
                foreign_members: None,
            }],
            foreign_members: None,
        };
        let result = collect_convex_boundingboxes(&featurecollection);
        assert!(result.is_ok());
        let hulls = result.unwrap();
        assert_eq!(
            hulls.len(),
            1,
            "Expected one hull from a valid Polygon in Germany"
        );

        // Check the vertices of the resulting hull (should be the square itself as it's convex)
        let expected_vertices = vec![
            Coord { x: 10.0, y: 50.0 },
            Coord { x: 11.0, y: 50.0 },
            Coord { x: 11.0, y: 51.0 },
            Coord { x: 10.0, y: 51.0 },
            Coord { x: 10.0, y: 50.0 }, // Closed ring
        ];
        let expected_hull = Polygon::new(LineString::new(expected_vertices), vec![]);
        assert!(
            are_polygons_geometrically_equal(&hulls[0], &expected_hull),
            "Resulting hull does not match expected geometry for polygon"
        );
    }

    #[test]
    fn test_collect_convex_boundingboxes_single_multipoint_in_germany_produces_one_hull() {
        let featurecollection = FeatureCollection {
            bbox: None,
            features: vec![Feature {
                bbox: None,
                geometry: Some(geojson::Geometry {
                    bbox: None,
                    value: Value::MultiPoint(vec![
                        vec![10.0, 50.0],
                        vec![11.0, 50.0],
                        vec![10.5, 51.0],
                        vec![10.5, 50.5],
                    ]), // 4 points in Germany
                    foreign_members: None,
                }),
                id: None,
                properties: None,
                foreign_members: None,
            }],
            foreign_members: None,
        };
        let result = collect_convex_boundingboxes(&featurecollection);
        assert!(result.is_ok());
        let hulls = result.unwrap();
        assert_eq!(
            hulls.len(),
            1,
            "Expected one hull from a valid MultiPoint in Germany"
        );

        // Check the vertices of the resulting hull (should be the hull of the 4 points)
        let expected_hull_points = vec![
            Coord { x: 10.0, y: 50.0 },
            Coord { x: 11.0, y: 50.0 },
            Coord { x: 10.5, y: 51.0 },
            Coord { x: 10.0, y: 50.0 }, // Hull of these points is the triangle
        ];
        let expected_hull = Polygon::new(LineString::new(expected_hull_points), vec![]);

        assert!(
            are_polygons_geometrically_equal(&hulls[0], &expected_hull),
            "Resulting hull does not match expected geometry for multipoint"
        );
    }

    // --- Tests for Duplicate Handling ---

    #[test]
    fn test_collect_convex_boundingboxes_duplicate_linestrings_in_germany_produce_one_hull() {
        let featurecollection = FeatureCollection {
            bbox: None,
            features: vec![
                // Feature 1
                Feature {
                    bbox: None,
                    geometry: Some(geojson::Geometry {
                        bbox: None,
                        value: Value::LineString(vec![
                            vec![10.0, 50.0],
                            vec![11.0, 50.0],
                            vec![10.5, 51.0],
                            vec![10.0, 50.0], // 3 points for hull (closed triangle)
                        ]),
                        foreign_members: None,
                    }),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
                // Feature 2 (identical geometry value)
                Feature {
                    bbox: None,
                    geometry: Some(geojson::Geometry {
                        bbox: None,
                        value: Value::LineString(vec![
                            vec![10.0, 50.0],
                            vec![11.0, 50.0],
                            vec![10.5, 51.0],
                            vec![10.0, 50.0],
                        ]),
                        foreign_members: None,
                    }),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
                // Feature 3 (identical geometry value)
                Feature {
                    bbox: None,
                    geometry: Some(geojson::Geometry {
                        bbox: None,
                        value: Value::LineString(vec![
                            vec![10.0, 50.0],
                            vec![11.0, 50.0],
                            vec![10.5, 51.0],
                            vec![10.0, 50.0],
                        ]),
                        foreign_members: None,
                    }),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
            ],
            foreign_members: None,
        };
        let result = collect_convex_boundingboxes(&featurecollection);
        assert!(result.is_ok());
        let hulls = result.unwrap();
        assert_eq!(
            hulls.len(),
            1,
            "Expected one unique hull from duplicate LineStrings"
        );

        // Check the vertex coordinates of the single resulting hull
        let expected_vertices = vec![
            Coord { x: 10.0, y: 50.0 },
            Coord { x: 11.0, y: 50.0 },
            Coord { x: 10.5, y: 51.0 },
            Coord { x: 10.0, y: 50.0 }, // Hull of these points is the triangle
        ];
        let expected_hull = Polygon::new(LineString::new(expected_vertices), vec![]);
        assert!(
            are_polygons_geometrically_equal(&hulls[0], &expected_hull),
            "Resulting hull does not match expected geometry for duplicate linestrings"
        );
    }

    #[test]
    fn test_collect_convex_boundingboxes_duplicate_polygons_in_germany_produce_one_hull() {
        let featurecollection = FeatureCollection {
            bbox: None,
            features: vec![
                // Feature 1
                Feature {
                    bbox: None,
                    geometry: Some(geojson::Geometry {
                        bbox: None,
                        value: Value::Polygon(vec![vec![
                            vec![10.0, 50.0],
                            vec![11.0, 50.0],
                            vec![11.0, 51.0],
                            vec![10.0, 51.0],
                            vec![10.0, 50.0],
                        ]]), // Square
                        foreign_members: None,
                    }),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
                // Feature 2 (identical geometry value)
                Feature {
                    bbox: None,
                    geometry: Some(geojson::Geometry {
                        bbox: None,
                        value: Value::Polygon(vec![vec![
                            vec![10.0, 50.0],
                            vec![11.0, 50.0],
                            vec![11.0, 51.0],
                            vec![10.0, 51.0],
                            vec![10.0, 50.0],
                        ]]), // Square
                        foreign_members: None,
                    }),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
            ],
            foreign_members: None,
        };
        let result = collect_convex_boundingboxes(&featurecollection);
        assert!(result.is_ok());
        let hulls = result.unwrap();
        assert_eq!(
            hulls.len(),
            1,
            "Expected one unique hull from duplicate Polygons"
        );

        // Check the vertices of the single resulting hull
        let expected_vertices = vec![
            Coord { x: 10.0, y: 50.0 },
            Coord { x: 11.0, y: 50.0 },
            Coord { x: 11.0, y: 51.0 },
            Coord { x: 10.0, y: 51.0 },
            Coord { x: 10.0, y: 50.0 }, // Closed ring
        ];
        let expected_hull = Polygon::new(LineString::new(expected_vertices), vec![]);
        assert!(
            are_polygons_geometrically_equal(&hulls[0], &expected_hull),
            "Resulting hull does not match expected geometry for duplicate polygons"
        );
    }

    // --- Tests for Mixed Inputs ---

    #[test]
    fn test_collect_convex_boundingboxes_mixed_valid_and_invalid_features() {
        let featurecollection = FeatureCollection {
            bbox: None,
            features: vec![
                // Valid LineString in Germany -> Hull A (convex hull)
                Feature {
                    bbox: None,
                    geometry: Some(geojson::Geometry {
                        bbox: None,
                        value: Value::LineString(vec![
                            vec![10.0, 50.0],
                            vec![11.0, 50.0],
                            vec![10.5, 51.0],
                        ]),
                        foreign_members: None,
                    }),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
                // Invalid LineString (outside Germany) -> Skipped
                Feature {
                    bbox: None,
                    geometry: Some(geojson::Geometry {
                        bbox: None,
                        value: Value::LineString(vec![
                            vec![0.0, 0.0],
                            vec![1.0, 1.0],
                            vec![0.5, 1.0],
                        ]),
                        foreign_members: None,
                    }),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
                // Valid Point in Germany -> Hull B (bounding box)
                Feature {
                    bbox: None,
                    geometry: Some(geojson::Geometry {
                        bbox: None,
                        value: Value::Point(vec![10.0, 50.0]),
                        foreign_members: None,
                    }),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
                // Valid LineString in Germany (2 points) -> Hull C (bounding box)
                Feature {
                    bbox: None,
                    geometry: Some(geojson::Geometry {
                        bbox: None,
                        value: Value::LineString(vec![vec![10.0, 50.0], vec![11.0, 51.0]]),
                        foreign_members: None,
                    }),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
            ],
            foreign_members: None,
        };
        let result = collect_convex_boundingboxes(&featurecollection);
        assert!(result.is_ok());
        let hulls = result.unwrap();

        assert_eq!(
            hulls.len(),
            3,
            "Expected three unique hulls from mixed input: one convex hull and two bounding boxes"
        );

        // Create expected hulls
        // Hull A: Convex hull from 3-point LineString
        let expected_hull_a = Polygon::new(
            LineString::new(vec![
                Coord { x: 10.0, y: 50.0 },
                Coord { x: 11.0, y: 50.0 },
                Coord { x: 10.5, y: 51.0 },
                Coord { x: 10.0, y: 50.0 },
            ]),
            vec![],
        );

        // Hull B: Bounding box from single point
        let expected_hull_b =
            Rect::new(Coord { x: 10.0, y: 50.0 }, Coord { x: 10.0, y: 50.0 }).to_polygon();

        // Hull C: Bounding box from 2-point LineString
        let expected_hull_c =
            Rect::new(Coord { x: 10.0, y: 50.0 }, Coord { x: 11.0, y: 51.0 }).to_polygon();

        // Collect canonical representations of returned hulls
        let returned_canonical: HashSet<Vec<(OrderedFloat<f64>, OrderedFloat<f64>)>> = hulls
            .iter()
            .map(|h| canonical_hull_unique_sorted_points(h))
            .collect();

        // Collect canonical representations of expected hulls
        let mut expected_canonical: HashSet<Vec<(OrderedFloat<f64>, OrderedFloat<f64>)>> =
            HashSet::new();
        expected_canonical.insert(canonical_hull_unique_sorted_points(&expected_hull_a));
        expected_canonical.insert(canonical_hull_unique_sorted_points(&expected_hull_b));
        expected_canonical.insert(canonical_hull_unique_sorted_points(&expected_hull_c));

        assert_eq!(
            returned_canonical, expected_canonical,
            "The set of returned hulls does not match the set of expected hulls"
        );
    }
}
