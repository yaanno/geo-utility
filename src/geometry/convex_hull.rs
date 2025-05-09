// Collects convex bounding boxes from a geojson FeatureCollection.

use geo::algorithm::convex_hull::ConvexHull;
use geo::geometry::{LineString as GeoLineString, MultiPoint};
use geo::{BoundingRect, Coord, Intersects, Point, Rect};
use geojson::{FeatureCollection, Value, Feature};
use ordered_float::OrderedFloat;
use std::collections::HashSet;
use crate::utils::error::Error;

use crate::utils::utils::{InBoundingBox, GERMANY_BBOX};


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
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    sorted_unique.sort();

    sorted_unique
}

/// Processes a single GeoJSON feature to extract either its convex hull or a
/// bounding box polygon if it's within the specified Germany boundaries.
///
/// Filters by feature bbox, extracts coordinates based on geometry type, checks
/// if all points are within Germany, and generates either a convex hull (>= 3
/// unique points) or a bounding box polygon (< 3 unique points).
///
/// # Arguments
/// * `feature` - The GeoJSON feature to process.
/// * `germany_rect` - The bounding rectangle for Germany for intersection checks.
///
/// # Returns
/// `Some(geo::Polygon)` containing the generated hull or bounding box polygon
/// if the feature meets the criteria, otherwise `None`.
fn process_single_feature(
    feature: &Feature,
    germany_rect: &Rect,
) -> Option<geo::Polygon> {
    // --- Early Filtering using Feature Bounding Box ---
    // Check feature bbox intersection with Germany bbox
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

            if !feature_rect.intersects(germany_rect) {
                return None; // Skip feature if its bbox doesn't intersect Germany
            }
        }
    }

    // --- Extract geometry and check location based on type ---
    let geometry_value = match feature.geometry.as_ref() {
        Some(geometry) => &geometry.value,
        None => {
            return None; // Skip features without geometry
        }
    };

    let mut coords: Vec<Coord> = Vec::new();
    let mut all_points_in_germany = true;
    let mut geometry_for_fallback_bbox: Option<geo::Geometry> = None; // Store geo::Geometry for fallback bbox calculation

    // Extract coordinates, check location, and prepare geo::Geometry for fallback
    match geometry_value {
        Value::Point(coord) => {
            let geo_coord = Coord {
                x: coord[0],
                y: coord[1],
            };
            if geo_coord.in_bounding_box(&GERMANY_BBOX) {
                coords.push(geo_coord); // Use for unique count check
                geometry_for_fallback_bbox = Some(geo::Geometry::Point(Point::from(geo_coord))); // Create geo::Point for bbox
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
                .any(|c| !c.in_bounding_box(&GERMANY_BBOX))
            {
                all_points_in_germany = false;
            } else {
                coords.extend(line_coords_geo.clone()); // Use for unique count check
                geometry_for_fallback_bbox = Some(geo::Geometry::LineString(GeoLineString::new(
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
                    .any(|c| !c.in_bounding_box(&GERMANY_BBOX))
                {
                    all_points_in_germany = false;
                } else {
                    coords.extend(&exterior_ring_geo_coords); // Use for unique count check
                    // Store the polygon geometry for potential fallback bbox if needed (though convex hull is primary for polygons)
                    // Note: Constructing a full geo::Polygon from geojson Vec<Vec<Vec<f64>>> is more complex,
                    // relying on extracted points for hull/bbox is simpler here.
                    // If needing the full polygon for fallback, you'd parse the interior rings too.
                    // Let's rely on extracted coords for unique count and hull/bbox.
                    if !exterior_ring_geo_coords.is_empty() {
                         // Note: BoundingRect on Polygon includes interior rings if they exist.
                         // For simplicity and focus on convex hull/outer bbox, we use extracted coords.
                         // If a precise Polygon BoundingRect was strictly needed for fallback, a full geo::Polygon parse would be required.
                         // Let's use the extracted coords for unique_count and rely on the logic below.
                         // If fallback is needed, and the geometry_for_fallback_bbox wasn't set (e.g., for Polygon/MultiPolygon),
                         // we might fallback to using MultiPoint::from(coords).bounding_rect()
                    } else {
                         // Polygon has an empty exterior ring, treat as out of bounds or invalid for processing
                         all_points_in_germany = false; // Effectively skips this feature
                    }
                }
            } else {
                // Polygon has no exterior ring, skip
                return None;
            }
        }
        // Add other geometry types here (MultiPoint, MultiLineString, MultiPolygon)
        // Extract their coordinates, check if in Germany, and if applicable,
        // create the corresponding geo::Geometry value for geometry_for_fallback_bbox.
        Value::MultiPoint(point_coords_vec) => {
            let multipoint_geo_coords: Vec<Coord> = point_coords_vec
                .iter()
                .map(|c| Coord { x: c[0], y: c[1] })
                .collect();
            if multipoint_geo_coords
                .iter()
                .any(|c| !c.in_bounding_box(&GERMANY_BBOX))
            {
                all_points_in_germany = false;
            } else {
                coords.extend(multipoint_geo_coords.clone()); // Use for unique count check
                if !multipoint_geo_coords.is_empty() {
                    geometry_for_fallback_bbox = Some(geo::Geometry::MultiPoint(MultiPoint::new(
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
                .any(|c| !c.in_bounding_box(&GERMANY_BBOX))
            {
                all_points_in_germany = false;
            } else {
                coords.extend(multiline_geo_coords.clone()); // Use for unique count check
                // Creating a geo::MultiLineString from flattened coords is tricky,
                // but we can use the flatten coords for convex hull/bbox.
                // Or process each LineString in MultiLineString individually if preferred.
                // For a simple bbox fallback, we can use the flattened coords.
                 if !multiline_geo_coords.is_empty() {
                     // Note: Using LineString for simplicity here, bbox is the same for flattened points
                     geometry_for_fallback_bbox = Some(geo::Geometry::LineString(GeoLineString::new(
                         multiline_geo_coords,
                     )));
                 }
            }
        }
        Value::MultiPolygon(multipolygon_coords_vec) => {
            let all_exterior_coords: Vec<Coord> = multipolygon_coords_vec
                .iter()
                .filter_map(|poly| poly.first()) // Get exterior rings
                .flatten() // Flatten points from all exterior rings
                .map(|c| Coord { x: c[0], y: c[1] })
                .collect();

            if all_exterior_coords
                .iter()
                .any(|c| !c.in_bounding_box(&GERMANY_BBOX))
            {
                all_points_in_germany = false;
            } else {
                coords.extend(all_exterior_coords.clone()); // Use for unique count check
                 if !all_exterior_coords.is_empty() {
                      // For MultiPolygon, rely on extracted coords for unique count and hull/bbox
                      // A full geo::MultiPolygon parse for geometry_for_fallback_bbox is complex.
                      // We can fall back to using MultiPoint::from(coords).bounding_rect() if needed.
                 } else {
                      // MultiPolygon has no non-empty exterior rings
                      all_points_in_germany = false; // Effectively skips this feature
                 }
            }
        }
        _ => {
            // Skip unsupported geometry types
            return None;
        }
    }

    // --- Final Location Check ---
    if !all_points_in_germany {
        return None; // Skip features that were not entirely within Germany bbox
    }

    // --- Check number of *unique* points derived from the geometry ---
    let unique_coords_count = coords
        .iter()
        .map(|c| (OrderedFloat(c.x), OrderedFloat(c.y)))
        .collect::<HashSet<_>>()
        .len();

    // --- Convex Hull vs. Fallback BBox Logic ---
    if unique_coords_count < 3 {
        // --- FALLBACK: Calculate bounding box and convert to polygon ---
        // Try to get bbox from geometry_for_fallback_bbox first
        let fallback_polygon = geometry_for_fallback_bbox
             .as_ref()
             .and_then(|geom| geom.bounding_rect())
             .or_else(|| { // If geometry_for_fallback_bbox didn't yield a rect (e.g., complex types or empty coords)
                  if !coords.is_empty() {
                       // Fallback to computing bbox from collected coords
                       let multi_point_from_coords = MultiPoint::new(coords.into_iter().map(Point::from).collect());
                       multi_point_from_coords.bounding_rect()
                  } else {
                       None // No coords, no bbox
                  }
             })
             .map(|rect| rect.to_polygon()); // Convert the Rect to a Polygon

        fallback_polygon // Return the generated polygon or None
    } else {
        // --- CONVEX HULL (Original Logic) ---
        // Compute the convex hull (use the original `coords` list which might have > unique_coords_count points)
        let multi_point = MultiPoint::from(coords);
        let hull = multi_point.convex_hull(); // This will be a geo::Polygon because unique_coords_count >= 3

        Some(hull) // Return the computed hull
    }
}

/// Filters a vector of polygons to remove duplicates based on their
/// canonical representation (unique sorted points).
///
/// # Arguments
/// * `hulls` - The vector of polygons potentially containing duplicates.
///
/// # Returns
/// A new vector containing only the unique polygons.
fn deduplicate_polygons(
    hulls: Vec<geo::Polygon>
) -> Vec<geo::Polygon> {
    let mut unique_hulls: Vec<geo::Polygon> = Vec::with_capacity(hulls.len());
    let mut seen_canonical_coords: HashSet<Vec<(OrderedFloat<f64>, OrderedFloat<f64>)>> =
        HashSet::with_capacity(hulls.len());

    for hull in hulls {
        let canonical_coords_hashable = canonical_hull_unique_sorted_points(&hull);

        if seen_canonical_coords.insert(canonical_coords_hashable) {
            unique_hulls.push(hull);
        }
    }
    unique_hulls
}


/// Collects convex bounding boxes and fallback bounding box polygons
/// from a geojson FeatureCollection.
///
/// Processes each feature, filters based on location within Germany, and
/// generates either a convex hull or a bounding box polygon. Duplicates
/// are removed at the end.
///
/// # Arguments
/// * `featurecollection` - The FeatureCollection from which to collect bounding box polygons.
///
/// # Returns
/// A vector of polygons, including convex hulls for features with >= 3 unique points
/// and bounding box polygons for features with < 3 unique points that can form a bounding box.
/// # Errors
/// Returns an error if a critical internal processing issue occurs (less likely with current logic).
pub fn collect_convex_boundingboxes(
    featurecollection: &FeatureCollection,
) -> Result<Vec<geo::Polygon>, Error> {
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

    let mut raw_hulls: Vec<geo::Polygon> = Vec::new();

    // Iterate through features and process each one individually
    for feature in &featurecollection.features {
        if let Some(polygon) = process_single_feature(feature, &germany_rect) {
            raw_hulls.push(polygon);
        }
        // Errors during processing a single feature are handled by returning None and skipping
    }

    // Deduplicate the collected polygons
    let unique_hulls = deduplicate_polygons(raw_hulls);

    // The current logic skips invalid features or geometries, so we always return Ok
    // unless a truly unexpected error occurred, which is not currently modeled.
    // Keeping Result<(), Error> signature for consistency if needed later.
    Ok(unique_hulls)
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
        assert!([10.0, 50.0].in_bounding_box(&GERMANY_BBOX));
        // Point outside Germany BBOX
        assert!(![0.0, 0.0].in_bounding_box(&GERMANY_BBOX));
        assert!(![20.0, 50.0].in_bounding_box(&GERMANY_BBOX)); // Outside max longitude
        assert!(![10.0, 40.0].in_bounding_box(&GERMANY_BBOX)); // Outside min latitude
        assert!(![10.0, 60.0].in_bounding_box(&GERMANY_BBOX)); // Outside max latitude
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
