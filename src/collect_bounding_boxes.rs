use crate::geometry::Rectangle;
use crate::grouping::{group_rects_by_overlap, merge_components};
use crate::utils::{
    create_square_grid, expand_bounding_box, is_boundingbox_in_germany, is_coordinate_in_germany,
};
use geo::geometry::LineString as GeoLineString;
use geo::{BoundingRect, ConvexHull, Coord, MultiPoint, Point, Rect};
use ordered_float::OrderedFloat;
use proj::Proj;
use rstar::RTreeObject;
use std::collections::HashSet;

/**
 * Collects bounding boxes from a geojson FeatureCollection.
 *
 * # Arguments
 *  `featurecollection` - The FeatureCollection from which to collect bounding box polygons.
 *  `radius` - The radius for expanding the bounding boxes.
 *  `_combine` - Whether to combine overlapping bounding boxes.
 *
 * # Returns
 * A vector of bounding boxes.
 */
pub fn collect_bounding_boxes(
    featurecollection: &geojson::FeatureCollection,
    radius: f64,
    _combine: bool,
) -> Vec<Rectangle> {
    let from_crs = "EPSG:4326";
    let to_crs = "EPSG:3035";

    let proj_transformer =
        Proj::new_known_crs(&from_crs, &to_crs, None).expect("Failed to create PROJ transformer");
    let proj_transformer_reverse = Proj::new_known_crs(&to_crs, &from_crs, None)
        .expect("Failed to create reverse PROJ transformer");
    let initial_geo_rects =
        collect_initial_buffered_rects(featurecollection, radius, &proj_transformer);

    let rectangles: Vec<Rectangle> = initial_geo_rects.into_iter().map(Rectangle::from).collect();
    let overall_initial_extent = calculate_overall_extent(&rectangles);

    if rectangles.is_empty() {
        return vec![];
    }
    let initial_grid_cells: Vec<Rect>;
    if let Some(overall_initial_extent) = overall_initial_extent {
        let area = overall_initial_extent.height() * overall_initial_extent.width();
        let target_num_cells = 20.0;

        if area <= 0.0 {
            return vec![];
        }
        let area_per_cell = area / target_num_cells;

        if area_per_cell <= 0.0 {
            return vec![];
        }
        let calculated_cell_size_meters = area_per_cell.sqrt();

        initial_grid_cells = create_square_grid(
            overall_initial_extent,
            calculated_cell_size_meters,
            calculated_cell_size_meters,
        );
    } else {
        return vec![];
    }

    let uf = group_rects_by_overlap(&rectangles);
    let merged_rectangles: Vec<Rectangle> = merge_components(&rectangles, uf);

    let tree = crate::grouping::index_rectangles(&merged_rectangles);

    let grid_cells_intersecting_shapes: Vec<Rectangle> = initial_grid_cells
        .into_iter()
        .map(Rectangle::from)
        .filter(|grid_cell| {
            tree.locate_in_envelope_intersecting(&grid_cell.envelope())
                .next()
                .is_some()
        })
        .map(|grid_cell_projected| {
            let min_geographic = proj_transformer_reverse
                .convert(grid_cell_projected.min())
                .expect("Reverse projection of cell min failed");
            let max_geographic = proj_transformer_reverse
                .convert(grid_cell_projected.max())
                .expect("Reverse projection of cell max failed");

            Rectangle::from_corners(
                (min_geographic.x, min_geographic.y),
                (max_geographic.x, max_geographic.y),
            )
        })
        .collect();

    grid_cells_intersecting_shapes
}

/**
 * Collects initial buffered rectangles from a geojson FeatureCollection.
 *
 * # Arguments
 *  `featurecollection` - The FeatureCollection from which to collect bounding box polygons.
 *  `radius` - The radius for expanding the bounding boxes.
 *
 * # Returns
 * A vector of buffered rectangles.
 */
fn collect_initial_buffered_rects(
    featurecollection: &geojson::FeatureCollection,
    radius: f64,
    proj_transformer: &Proj,
) -> Vec<geo::Rect> {
    let mut bounding_boxes: Vec<geo::Rect> = Vec::with_capacity(featurecollection.features.len());

    for feature in &featurecollection.features {
        // 0. Early Filtering using Feature Bounding Box
        if let Some(feature_bbox_value) = &feature.bbox {
            if !is_boundingbox_in_germany(feature_bbox_value) {
                continue;
            }
        }

        let geometry_value = match feature.geometry.as_ref() {
            Some(geometry) => &geometry.value,
            None => {
                continue;
            }
        };
        let mut coords: Vec<Coord> = Vec::new();
        let mut all_points_in_germany = true;
        match geometry_value {
            geojson::Value::Point(coord) => {
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
            geojson::Value::LineString(line_coords) => {
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
                    coords.extend(&line_coords_geo);
                }
            }
            _ => {
                continue;
            }
        }
        if !all_points_in_germany {
            continue;
        }
        let projected_coords: Vec<Coord> = coords
            .into_iter()
            .map(|c| proj_transformer.convert(c).unwrap())
            .collect();

        let unique_coords_count = projected_coords
            .iter()
            .map(|c| (OrderedFloat(c.x), OrderedFloat(c.y)))
            .collect::<HashSet<_>>()
            .len();
        if unique_coords_count < 3 {
            let projected_geometry_to_process = match geometry_value.type_name() {
                "Point" => geo::Geometry::Point(Point::from(
                    *projected_coords
                        .first()
                        .expect("Point should have 1 projected coord"),
                )),
                "LineString" => {
                    geo::Geometry::LineString(GeoLineString::new(projected_coords.clone()))
                }
                _ => unreachable!("Should only be Point or LineString at this stage"),
            };

            let fallback_bbox_projected = projected_geometry_to_process.bounding_rect();

            if let Some(bounding_box) = fallback_bbox_projected {
                let expanded_bounding_box = expand_bounding_box(&bounding_box, radius);
                bounding_boxes.push(expanded_bounding_box);
            }
        } else {
            let multi_point_projected = MultiPoint::from(projected_coords);
            let bounding_box = multi_point_projected.convex_hull().bounding_rect();

            if let Some(bounding_box) = bounding_box {
                let expanded_bounding_box = expand_bounding_box(&bounding_box, radius);
                bounding_boxes.push(expanded_bounding_box);
            }
        }
    }
    bounding_boxes
}

/// Calculates the overall bounding box that encompasses all provided rectangles.
///
/// # Arguments
/// * `rectangles` - A slice of rectangles.
///
/// # Returns
/// A geo::Rect representing the overall bounding box, or None if the input slice is empty.
pub fn calculate_overall_extent(rectangles: &[Rectangle]) -> Option<Rect> {
    if rectangles.is_empty() {
        return None;
    }

    let mut overall_min_x = f64::INFINITY;
    let mut overall_min_y = f64::INFINITY;
    let mut overall_max_x = f64::NEG_INFINITY;
    let mut overall_max_y = f64::NEG_INFINITY;

    for rect in rectangles {
        overall_min_x = overall_min_x.min(rect.min().x);
        overall_min_y = overall_min_y.min(rect.min().y);
        overall_max_x = overall_max_x.max(rect.max().x);
        overall_max_y = overall_max_y.max(rect.max().y);
    }

    Some(Rect::new(
        Coord {
            x: overall_min_x,
            y: overall_min_y,
        },
        Coord {
            x: overall_max_x,
            y: overall_max_y,
        },
    ))
}

#[cfg(test)]
mod tests {
    use geojson::{Feature, FeatureCollection, GeoJson, Geometry};

    use super::*; // Import items from the parent module

    // Helper function to create a point feature
    fn point_feature(x: f64, y: f64) -> geojson::Feature {
        geojson::Feature {
            bbox: None, // Bbox is calculated later
            geometry: Some(geojson::Geometry {
                value: geojson::Value::Point(vec![x, y]),
                bbox: None,
                foreign_members: None,
            }),
            properties: None,
            foreign_members: None,
            id: None,
        }
    }

    //     // Helper to create a FeatureCollection
    fn feature_collection(features: Vec<Feature>) -> FeatureCollection {
        FeatureCollection {
            bbox: None,
            features,
            foreign_members: None,
        }
    }

    // Helper to sort rectangles for comparison (though we won't compare individual rects here)
    // use crate::geometry::sort_rectangles; // Assuming this helper exists

    // A simple test case for the new pipeline with dynamic grid sizing
    #[test]
    fn test_collect_bboxes_dynamic_grid_simple() {
        // Two points in Germany, close together, buffered boxes should overlap and merge
        let input_features = vec![
            point_feature(9.0, 50.0),     // Point 1
            point_feature(9.001, 50.001), // Point 2, very close,
            point_feature(8.9901, 49.99),
        ];
        let fc = feature_collection(input_features.clone()); // Clone input features for later visualization
        let radius = 10.0; // 10 meters buffer
        let combine = true; // Merging is enabled

        // --- Assumptions for this test ---
        // 1. Input CRS is WGS84 (EPSG:4326)
        // 2. Target CRS is ETRS89-LAEA (EPSG:3035) for metric operations
        // 3. Inside collect_bounding_boxes, the dynamic grid calculation uses
        //    a specific small target_num_cells, e.g., 10.0
        //    (You might need to set target_num_cells = 10.0 temporarily in your code for this test)
        // 4. The two points are close enough that their 10m projected buffers overlap and merge into 1 component (M=1).
        // 5. The overall extent of the merged shape is small.
        // 6. A target of 10 cells over that small extent results in G around 10.

        let result_rectangles = collect_bounding_boxes(&fc, radius, combine);

        // --- Convert result Rectangles to GeoJSON Features ---
        let result_features: Vec<Feature> = result_rectangles
            .iter()
            .map(|rect| {
                let geo_rect = rect.to_geo_rect(); // Convert your Rectangle to geo::Rect
                let polygon = geo::Polygon::from(geo_rect); // Convert geo::Rect to geo::Polygon
                let geometry = Geometry::from(&polygon); // Convert geo::Polygon to geojson::Geometry
                Feature {
                    bbox: None,
                    geometry: Some(geometry),
                    id: None,
                    properties: None,
                    foreign_members: None,
                }
            })
            .collect();

        // --- Combine input points and output grid cells into one FeatureCollection ---
        let mut all_features = input_features; // Start with the input points
        all_features.extend(result_features); // Add the resulting grid cells
        let output_fc = feature_collection(all_features);
        let geojson_output = GeoJson::from(output_fc);
        let geojson_string = geojson_output.to_string();

        // --- Print the GeoJSON string ---
        println!("--- GeoJSON Output for Visualization ---");
        println!("{}", geojson_string);
        println!("--- End GeoJSON Output ---");
        // Copy the string between the markers and paste into geojson.io or save as a .geojson file

        // Assertions
        // 1. The result vector is not empty
        assert!(
            !result_rectangles.is_empty(),
            "Resulting grid should not be empty"
        );

        // 2. The number of resulting grid cells (I) is close to the target (e.g., 10)
        //    Allow for some variability in the actual number of cells generated
        let expected_min_cells = 8; // Allow +/- a few cells around the target
        let expected_max_cells = 20; // Adjust this range based on your actual dynamic calc behavior
        assert!(
            result_rectangles.len() >= expected_min_cells
                && result_rectangles.len() <= expected_max_cells,
            "Number of resulting grid cells ({}) should be between {} and {}",
            result_rectangles.len(),
            expected_min_cells,
            expected_max_cells
        );

        // 3. The overall bounding box of the resulting grid cells is reasonable
        //    This is a tougher assertion due to projection/reverse projection noise.
        //    We'll calculate the overall bbox of the result rects (which are in WGS84)
        //    and check if its corners are within a small tolerance of the expected area.
        //    The expected area would be the overall bbox of the two input points
        //    plus a small geographic buffer that approximates the 10m metric buffer
        //    after reverse projection. This is complex to calculate precisely.
        //    A simpler check is to get the overall bbox and check its format/units.

        // Calculate the overall bounding box of the result rectangles (in WGS84)
        // Note: Need geo::Polygon for MultiPolygon::from
        let result_polygons: Vec<geo::Polygon> = result_rectangles
            .iter()
            .map(|r| geo::Polygon::from(r.to_geo_rect()))
            .collect();

        let result_overall_bbox_option = geo::MultiPolygon::from(result_polygons).bounding_rect(); // Use result_polygons

        assert!(
            result_overall_bbox_option.is_some(),
            "Overall bbox of result should not be None"
        );
        let result_overall_bbox = result_overall_bbox_option.unwrap();

        // --- Assert the units and general location ---
        // For WGS84 geographic, coordinates should be in the range [-180, 180] for x and [-90, 90] for y
        // And for points in Germany (around 9E, 50N), the bbox should be around that area.
        assert!(
            result_overall_bbox.min().x >= -180.0 && result_overall_bbox.max().x <= 180.0,
            "Result bbox X coordinates should be in WGS84 range (-180 to 180)"
        );
        assert!(
            result_overall_bbox.min().y >= -90.0 && result_overall_bbox.max().y <= 90.0,
            "Result bbox Y coordinates should be in WGS84 range (-90 to 90)"
        );

        // Assert the bbox is roughly centered around the input points (9E, 50N)
        let expected_center_x = (9.0 + 9.001) / 2.0;
        let expected_center_y = (50.0 + 50.001) / 2.0;
        let result_center_x = (result_overall_bbox.min().x + result_overall_bbox.max().x) / 2.0;
        let result_center_y = (result_overall_bbox.min().y + result_overall_bbox.max().y) / 2.0;

        let tolerance = 0.1; // A small tolerance in degrees
        assert!(
            (result_center_x - expected_center_x).abs() < tolerance,
            "Result bbox center X should be close to expected center X"
        );
        assert!(
            (result_center_y - expected_center_y).abs() < tolerance,
            "Result bbox center Y should be close to expected center Y"
        );

        // Note: Asserting the exact corners of result_overall_bbox is difficult due to dynamic sizing and projection noise.
        // These checks verify the count is reasonable and the overall output is in the correct CRS and general location.
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use geojson::{Feature, FeatureCollection, Geometry, Value};

//     // Helper to create a GeoJSON Feature for testing
//     fn create_geojson_feature(geometry: Value) -> Feature {
//         Feature {
//             bbox: None, // Rely on geometry processing, not feature bbox
//             geometry: Some(Geometry {
//                 value: geometry,
//                 bbox: None,
//                 foreign_members: None,
//             }),
//             id: None,
//             properties: None,
//             foreign_members: None,
//         }
//     }

//     // Helper to create a Point GeoJSON feature
//     fn point_feature(lon: f64, lat: f64) -> Feature {
//         create_geojson_feature(Value::Point(vec![lon, lat]))
//     }

//     // Helper to create a LineString GeoJSON feature
//     fn linestring_feature(coords: Vec<(f64, f64)>) -> Feature {
//         let geojson_coords: Vec<Vec<f64>> = coords.into_iter().map(|c| vec![c.0, c.1]).collect();
//         create_geojson_feature(Value::LineString(geojson_coords))
//     }

//     // Helper to create a Polygon GeoJSON feature (will be skipped by current logic)
//     fn polygon_feature(exterior_coords: Vec<(f64, f64)>) -> Feature {
//         let geojson_coords: Vec<Vec<f64>> = exterior_coords
//             .into_iter()
//             .map(|c| vec![c.0, c.1])
//             .collect();
//         create_geojson_feature(Value::Polygon(vec![geojson_coords]))
//     }

//     // Helper to create a FeatureCollection
//     fn feature_collection(features: Vec<Feature>) -> FeatureCollection {
//         FeatureCollection {
//             bbox: None,
//             features,
//             foreign_members: None,
//         }
//     }

//     // Helper to create expected Rectangle results concisely
//     fn r(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Rectangle {
//         Rectangle::from_corners((min_x, min_y), (max_x, max_y))
//     }

//     // Helper to sort Rectangles for consistent comparison
//     fn sort_rectangles(mut rects: Vec<Rectangle>) -> Vec<Rectangle> {
//         rects.sort_by(|a, b| {
//             a.min()
//                 .x
//                 .partial_cmp(&b.min().x)
//                 .unwrap_or(std::cmp::Ordering::Equal)
//                 .then(
//                     a.min()
//                         .y
//                         .partial_cmp(&b.min().y)
//                         .unwrap_or(std::cmp::Ordering::Equal),
//                 )
//         });
//         rects
//     }

//     #[test]
//     fn test_collect_bboxes_single_point_in_germany() {
//         // Point(9, 50) is in Germany's rough bbox (5.8-15.0, 47.2-55.0)
//         let fc = feature_collection(vec![point_feature(9.0, 50.0)]);
//         let radius = 10.0;
//         let combine = true; // Combine logic should merge a single item with itself (result is just the item)

//         // Expected: Buffered bbox of (9,50)
//         // (9-10, 50-10) to (9+10, 50+10) = (-1, 40) to (19, 60)
//         let expected = vec![r(-1.0, 40.0, 19.0, 60.0)];

//         let result = collect_bounding_boxes(&fc, radius, combine);
//         assert_eq!(sort_rectangles(result), sort_rectangles(expected));
//     }

//     #[test]
//     fn test_collect_bboxes_single_point_outside_germany() {
//         let fc = feature_collection(vec![point_feature(0.0, 0.0)]); // Outside Germany
//         let radius = 10.0;
//         let combine = true;

//         let expected: Vec<Rectangle> = vec![]; // Should be filtered out

//         let result = collect_bounding_boxes(&fc, radius, combine);
//         assert_eq!(sort_rectangles(result), sort_rectangles(expected));
//     }

//     #[test]
//     fn test_collect_bboxes_linestring_2_points_in_germany() {
//         // LineString (9,50)-(10,51) - 2 unique points, in Germany -> Fallback bbox + Buffer
//         let fc = feature_collection(vec![linestring_feature(vec![(9.0, 50.0), (10.0, 51.0)])]);
//         let radius = 5.0;
//         let combine = true;

//         // Bbox of (9,50)-(10,51) is Rect(9,50,10,51)
//         // Buffered by 5: (9-5, 50-5) to (10+5, 51+5) = (4, 45) to (15, 56)
//         let expected = vec![r(4.0, 45.0, 15.0, 56.0)];

//         let result = collect_bounding_boxes(&fc, radius, combine);
//         assert_eq!(sort_rectangles(result), sort_rectangles(expected));
//     }

//     #[test]
//     fn test_collect_bboxes_linestring_3_points_in_germany() {
//         // LineString (9,50)-(10,52)-(11,50) - 3 unique points, in Germany -> Convex Hull Bbox + Buffer
//         let fc = feature_collection(vec![linestring_feature(vec![
//             (9.0, 50.0),
//             (10.0, 52.0),
//             (11.0, 50.0),
//         ])]);
//         let radius = 5.0;
//         let combine = true;

//         // Convex hull of (9,50), (10,52), (11,50) is the triangle itself.
//         // Bbox of the triangle is Rect(9,50,11,52)
//         // Buffered by 5: (9-5, 50-5) to (11+5, 52+5) = (4, 45) to (16, 57)
//         let expected = vec![r(4.0, 45.0, 16.0, 57.0)];

//         let result = collect_bounding_boxes(&fc, radius, combine);
//         assert_eq!(sort_rectangles(result), sort_rectangles(expected));
//     }

//     #[test]
//     fn test_collect_bboxes_linestring_partly_outside_germany() {
//         // LineString (9,50)-(10,51)-(0,0) - (0,0) is outside Germany -> entire feature skipped
//         let fc = feature_collection(vec![linestring_feature(vec![
//             (9.0, 50.0),
//             (10.0, 51.0),
//             (0.0, 0.0),
//         ])]);
//         let radius = 5.0;
//         let combine = true;

//         let expected: Vec<Rectangle> = vec![]; // Should be filtered out

//         let result = collect_bounding_boxes(&fc, radius, combine);
//         assert_eq!(sort_rectangles(result), sort_rectangles(expected));
//     }

//     #[test]
//     fn test_collect_bboxes_unsupported_geometry_type() {
//         // Polygon feature - currently skipped
//         let fc = feature_collection(vec![polygon_feature(vec![
//             (9.0, 50.0),
//             (10.0, 50.0),
//             (10.0, 51.0),
//             (9.0, 51.0),
//             (9.0, 50.0),
//         ])]);
//         let radius = 10.0;
//         let combine = true;

//         let expected: Vec<Rectangle> = vec![]; // Should be skipped

//         let result = collect_bounding_boxes(&fc, radius, combine);
//         assert_eq!(sort_rectangles(result), sort_rectangles(expected));
//     }

//     #[test]
//     fn test_collect_bboxes_multiple_non_overlapping() {
//         // Two points in Germany, far apart, buffered boxes don't overlap
//         let fc = feature_collection(vec![
//             point_feature(9.0, 50.0),  // Bbox: (-1, 40) to (19, 60) with radius 10
//             point_feature(14.0, 54.0), // Bbox: (4, 44) to (24, 64) with radius 10
//         ]);
//         let radius = 10.0;
//         let combine = true; // Should result in two separate merged components (themselves)

//         let expected = vec![r(-1.0, 40.0, 24.0, 64.0)];

//         let result = collect_bounding_boxes(&fc, radius, combine);
//         // Sort both vectors for reliable comparison
//         assert_eq!(sort_rectangles(result), sort_rectangles(expected));
//     }

//     #[test]
//     fn test_collect_bboxes_two_overlapping_points() {
//         // Two points in Germany, buffered boxes overlap
//         let fc = feature_collection(vec![
//             point_feature(9.0, 50.0), // Bbox: (-1, 40) to (19, 60)
//             point_feature(9.5, 50.5), // Bbox: (-0.5, 40.5) to (19.5, 60.5)
//         ]);
//         let radius = 10.0;
//         let combine = true; // Should merge into a single box

//         // Overall bbox of the two buffered boxes
//         // min_x = min(-1.0, -0.5) = -1.0
//         // min_y = min(40.0, 40.5) = 40.0
//         // max_x = max(19.0, 19.5) = 19.5
//         // max_y = max(60.0, 60.5) = 60.5
//         let expected = vec![r(-1.0, 40.0, 19.5, 60.5)];

//         let result = collect_bounding_boxes(&fc, radius, combine);
//         assert_eq!(sort_rectangles(result), sort_rectangles(expected));
//     }

//     #[test]
//     fn test_collect_bboxes_point_overlapping_linestring_bbox() {
//         // Point and LineString in Germany, buffered bboxes overlap
//         let fc = feature_collection(vec![
//             point_feature(9.0, 50.0), // Bbox: (-1, 40) to (19, 60)
//             linestring_feature(vec![(9.5, 50.5), (10.5, 51.5)]), // Bbox (fallback): (9.5,50.5)-(10.5,51.5) -> buffered by 10: (-0.5, 40.5) to (20.5, 61.5)
//         ]);
//         let radius = 10.0;
//         let combine = true; // Should merge into a single box

//         // Overall bbox of the two buffered boxes
//         // min_x = min(-1.0, -0.5) = -1.0
//         // min_y = min(40.0, 40.5) = 40.0
//         // max_x = max(19.0, 20.5) = 20.5
//         // max_y = max(60.0, 61.5) = 61.5
//         let expected = vec![r(-1.0, 40.0, 20.5, 61.5)];

//         let result = collect_bounding_boxes(&fc, radius, combine);
//         assert_eq!(sort_rectangles(result), sort_rectangles(expected));
//     }

//     #[test]
//     fn test_collect_bboxes_multiple_overlapping_components() {
//         // Four points forming two separate overlapping pairs
//         let fc = feature_collection(vec![
//             point_feature(9.0, 50.0),  // Bbox: (-1, 40) to (19, 60)
//             point_feature(9.5, 50.5), // Bbox: (-0.5, 40.5) to (19.5, 60.5) -> overlaps first, merge
//             point_feature(15.0, 55.0), // Bbox: (5, 45) to (25, 65)
//             point_feature(15.5, 55.5), // Bbox: (5.5, 45.5) to (25.5, 65.5) -> overlaps third, merge
//         ]);
//         let radius = 10.0;
//         let combine = true; // Should result in two merged boxes

//         // Expected merged box 1 (from P1, P2): (-1.0, 40.0, 19.5, 60.5)
//         // Expected merged box 2 (from P3, P4): (5.0, 45.0, 25.5, 65.5)
//         let expected = vec![r(-1.0, 40.0, 25.0, 65.0)];

//         let result = collect_bounding_boxes(&fc, radius, combine);
//         assert_eq!(sort_rectangles(result), sort_rectangles(expected));
//     }

//     #[test]
//     fn test_collect_bboxes_contained_bbox_merges() {
//         // Larger LineString containing a point, buffered boxes overlap, point bbox contained in LS bbox
//         let fc = feature_collection(vec![
//             linestring_feature(vec![(9.0, 50.0), (15.0, 55.0)]), // Bbox (fallback): (9,50)-(15,55) -> buffered by 5: (4, 45)-(20, 60)
//             point_feature(10.0, 51.0), // Bbox: (10,51) -> buffered by 5: (5, 46)-(15, 56)
//         ]);
//         let radius = 5.0;
//         let combine = true; // Should merge into a single box

//         // Overall bbox of the two buffered boxes
//         // min_x = min(4.0, 5.0) = 4.0
//         // min_y = min(45.0, 46.0) = 45.0
//         // max_x = max(20.0, 15.0) = 20.0
//         // max_y = max(60.0, 56.0) = 60.0
//         // The contained box's bounds are within the container's bounds,
//         // so the merged box is just the container's buffered box.
//         let expected = vec![r(4.0, 45.0, 20.0, 60.0)];

//         let result = collect_bounding_boxes(&fc, radius, combine);
//         // Replace assert_eq! with fuzzy comparison
//         let sorted_result = sort_rectangles(result);
//         let sorted_expected = sort_rectangles(expected);

//         // Assert that the number of merged rectangles is the same
//         assert_eq!(
//             sorted_result.len(),
//             sorted_expected.len(),
//             "Mismatched number of merged rectangles"
//         );

//         // Use a small tolerance for float comparison
//         let epsilon = 1e-9; // A common tolerance for double-precision floats

//         // Iterate through the sorted results and expected values and compare coordinates fuzzily
//         for (res_rect, exp_rect) in sorted_result.iter().zip(sorted_expected.iter()) {
//             let min_x_diff = (res_rect.min().x - exp_rect.min().x).abs();
//             let min_y_diff = (res_rect.min().y - exp_rect.min().y).abs();
//             let max_x_diff = (res_rect.max().x - exp_rect.max().x).abs();
//             let max_y_diff = (res_rect.max().y - exp_rect.max().y).abs();

//             assert!(
//                 min_x_diff < epsilon
//                     && min_y_diff < epsilon
//                     && max_x_diff < epsilon
//                     && max_y_diff < epsilon,
//                 "Merged rectangle {:?} does not nearly match expected {:?} within tolerance {}",
//                 res_rect,
//                 exp_rect,
//                 epsilon
//             );
//         }
//     }

//     #[test]
//     fn test_collect_bboxes_zero_radius() {
//         // Single point in Germany, radius 0 -> buffer by 4
//         let fc = feature_collection(vec![point_feature(9.0, 50.0)]);
//         let radius = 0.0; // Test radius 0 case
//         let combine = true;

//         // Expected: Buffered bbox of (9,50) by 4
//         // (9-4, 50-4) to (9+4, 50+4) = (5, 46) to (13, 54)
//         let expected = vec![r(5.0, 46.0, 13.0, 54.0)];

//         // Need to adjust expand_bounding_box or pass radius=Some(0.0) if it expects Option
//         // Assuming expand_bounding_box is adjusted to handle f64 radius and 0.0->4.0 logic
//         let result = collect_bounding_boxes(&fc, radius, combine);
//         assert_eq!(sort_rectangles(result), sort_rectangles(expected));
//     }
// }
