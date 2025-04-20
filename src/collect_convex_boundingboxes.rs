// Collects convex bounding boxes from a geojson FeatureCollection.

use geo::Coord; // Import other necessary geo types
use geo::algorithm::convex_hull::ConvexHull; // Import the ConvexHull trait
use geo::geometry::MultiPoint; // Import MultiPoint
use geojson::{FeatureCollection, Value};
use ordered_float::OrderedFloat;
use std::collections::HashSet;
use thiserror::Error;

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

/// Collects convex bounding boxes from a geojson FeatureCollection.
///
/// # Arguments
/// * `featurecollection` - The FeatureCollection from which to collect convex bounding boxes.
///
/// # Returns
/// A vector of convex bounding boxes as geo::Polygon.
/// # Errors
/// Returns an error if the geometry type is unsupported or if there are invalid coordinates.
pub fn collect_convex_boundingboxes(
    featurecollection: &FeatureCollection,
) -> Result<Vec<geo::Polygon>, Error> {
    let mut hulls: Vec<geo::Polygon> = Vec::new();

    for feature in &featurecollection.features {
        // 1. Check for geometry and extract coordinates based on type
        let geometry_value = match feature.geometry.as_ref() {
            Some(geometry) => &geometry.value,
            None => {
                // Skip features without geometry
                continue;
            }
        };

        let mut coords: Vec<Coord> = Vec::new();
        let mut all_points_in_germany = true;

        // Extract coordinates and check location based on geometry type
        match geometry_value {
            Value::Point(coord) => {
                let geo_coord = Coord {
                    x: coord[0],
                    y: coord[1],
                };
                if is_coordinate_in_germany(&coord) {
                    coords.push(geo_coord);
                } else {
                    all_points_in_germany = false;
                }
            }
            Value::LineString(line_coords) => {
                if line_coords.iter().all(|c| is_coordinate_in_germany(c)) {
                    coords.extend(line_coords.iter().map(|c| Coord { x: c[0], y: c[1] }));
                } else {
                    all_points_in_germany = false;
                }
            }
            Value::Polygon(polygon_coords) => {
                // Extract coords from exterior ring; interior rings don't affect convex hull
                if let Some(exterior_ring) = polygon_coords.first() {
                    if exterior_ring.iter().all(|c| is_coordinate_in_germany(c)) {
                        coords.extend(exterior_ring.iter().map(|c| Coord { x: c[0], y: c[1] }));
                    } else {
                        all_points_in_germany = false;
                    }
                } else {
                    // Polygon has no exterior ring, skip
                    continue;
                }
            }
            Value::MultiPoint(point_coords_vec) => {
                if point_coords_vec.iter().all(|c| is_coordinate_in_germany(c)) {
                    coords.extend(point_coords_vec.iter().map(|c| Coord { x: c[0], y: c[1] }));
                } else {
                    all_points_in_germany = false;
                }
            }
            Value::MultiLineString(multiline_coords_vec) => {
                // Check all points in all lines
                if multiline_coords_vec
                    .iter()
                    .flatten()
                    .all(|c| is_coordinate_in_germany(c))
                {
                    coords.extend(
                        multiline_coords_vec
                            .iter()
                            .flatten()
                            .map(|c| Coord { x: c[0], y: c[1] }),
                    );
                } else {
                    all_points_in_germany = false;
                }
            }
            Value::MultiPolygon(multipolygon_coords_vec) => {
                // Extract coords from all exterior rings
                let all_exterior_coords = multipolygon_coords_vec
                    .iter()
                    .filter_map(|poly| poly.first())
                    .flatten()
                    .collect::<Vec<_>>();

                if all_exterior_coords
                    .iter()
                    .all(|c| is_coordinate_in_germany(c))
                {
                    coords.extend(
                        all_exterior_coords
                            .iter()
                            .map(|c| Coord { x: c[0], y: c[1] }),
                    );
                } else {
                    all_points_in_germany = false;
                }
            }
            // Add other geometry types here if needed, otherwise they are skipped
            _ => {
                // Skip unsupported geometry types
                continue;
            }
        }

        // 2. Check if all relevant points were in Germany
        if !all_points_in_germany {
            // Skip features that were not entirely within the bounding box
            continue;
        }

        // *** MODIFIED CHECK FOR SUFFICIENT POINTS ***
        // Check number of *unique* points derived from the geometry
        let unique_coords_count = coords
            .iter()
            .map(|c| (OrderedFloat(c.x), OrderedFloat(c.y)))
            .collect::<HashSet<_>>()
            .len();

        if unique_coords_count < 3 {
            continue; // Skip features with less than 3 *unique* coordinates
        }

        // 3. Compute the convex hull (use the original `coords` list which might have > unique_coords_count points)
        let multi_point = MultiPoint::from(coords); // MultiPoint handles duplicate points internally
        let hull = multi_point.convex_hull();

        hulls.push(hull); // Collect all computed hulls first
    }

    // --- Post-processing step: Filter out duplicate hulls ---
    let mut unique_hulls: Vec<geo::Polygon> = Vec::with_capacity(hulls.len());
    // Use the unique sorted points representation as the key in the HashSet
    let mut seen_canonical_coords: HashSet<Vec<(OrderedFloat<f64>, OrderedFloat<f64>)>> =
        HashSet::with_capacity(hulls.len());

    for hull in hulls {
        // Use the unique sorted points canonicalization for hashing
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
    const GERMANY_BBOX: [f64; 4] = [
        5.866211,  // Min longitude
        47.270111, // Min latitude
        15.013611, // Max longitude
        55.058333, // Max latitude
    ];

    // Ensure coord has at least 2 elements (longitude, latitude)
    if coord.len() < 2 {
        return false; // Cannot check if coords are invalid
    }

    coord[0] >= GERMANY_BBOX[0]
        && coord[0] <= GERMANY_BBOX[2]
        && coord[1] >= GERMANY_BBOX[1]
        && coord[1] <= GERMANY_BBOX[3]
}

#[allow(unused_imports)]
mod tests {
    use super::*;
    use geo::{LineString, MultiPoint, Point, Polygon};
    use geojson::{Feature, FeatureCollection, Value};
    use ordered_float::OrderedFloat;
    use std::collections::HashSet;

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
    fn test_collect_convex_boundingboxes_features_with_insufficient_points_produce_empty() {
        let featurecollection = FeatureCollection {
            bbox: None,
            features: vec![
                // Point in Germany (1 point < 3)
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
                // LineString in Germany (2 points < 3)
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
                // LineString in Germany (2 identical points < 3)
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
                // MultiPoint in Germany (2 points < 3)
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
                // Polygon in Germany with only 2 points in exterior ring (invalid geojson, but test handling)
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
        assert_eq!(
            result.unwrap().len(),
            0,
            "Expected empty result when all features have insufficient points"
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
                // Valid LineString in Germany -> Hull A
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
                // Valid Polygon in Germany -> Hull B (different from A)
                Feature {
                    bbox: None,
                    geometry: Some(geojson::Geometry {
                        bbox: None,
                        value: Value::Polygon(vec![vec![
                            vec![12.0, 52.0],
                            vec![13.0, 52.0],
                            vec![13.0, 53.0],
                            vec![12.0, 53.0],
                            vec![12.0, 52.0],
                        ]]),
                        foreign_members: None,
                    }),
                    id: None,
                    properties: None,
                    foreign_members: None,
                },
                // Valid LineString (same as first) in Germany -> Duplicate of Hull A
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
                // Invalid Point (< 3 points) in Germany -> Skipped
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
                // Unsupported type -> Skipped
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
            ],
            foreign_members: None,
        };
        let result = collect_convex_boundingboxes(&featurecollection);
        assert!(result.is_ok());
        let hulls = result.unwrap();

        // Expecting 2 unique hulls: one from the LineString triangle, one from the Polygon square
        assert_eq!(hulls.len(), 2, "Expected two unique hulls from mixed input");

        // Optional: Verify the actual hulls returned match the two expected geometries
        let expected_hull_a_vertices = vec![
            Coord { x: 10.0, y: 50.0 },
            Coord { x: 11.0, y: 50.0 },
            Coord { x: 10.5, y: 51.0 },
            Coord { x: 10.0, y: 50.0 },
        ];
        let expected_hull_a = Polygon::new(LineString::new(expected_hull_a_vertices), vec![]);

        let expected_hull_b_vertices = vec![
            Coord { x: 12.0, y: 52.0 },
            Coord { x: 13.0, y: 52.0 },
            Coord { x: 13.0, y: 53.0 },
            Coord { x: 12.0, y: 53.0 },
            Coord { x: 12.0, y: 52.0 },
        ];
        let expected_hull_b = Polygon::new(LineString::new(expected_hull_b_vertices), vec![]);

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

        // Assert the sets of canonical representations are equal
        assert_eq!(
            returned_canonical, expected_canonical,
            "The set of returned hulls does not match the set of expected hulls"
        );
    }
}
