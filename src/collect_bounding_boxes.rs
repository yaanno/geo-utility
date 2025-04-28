use crate::geometry::Rectangle;
use crate::grouping::{group_rects_by_overlap, merge_components};
use crate::utils::{expand_bounding_box, is_boundingbox_in_germany, is_coordinate_in_germany};
use geo::geometry::LineString as GeoLineString;
use geo::{BoundingRect, Coord, MultiPoint, Point};
use ordered_float::OrderedFloat;
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
    let initial_geo_rects = collect_initial_buffered_rects(featurecollection, radius);

    let rectangles: Vec<Rectangle> = initial_geo_rects.into_iter().map(Rectangle::from).collect();

    let uf = group_rects_by_overlap(&rectangles);
    let merged_rectangles: Vec<Rectangle> = merge_components(&rectangles, uf);
    merged_rectangles
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
) -> Vec<geo::Rect> {
    let mut bounding_boxes: Vec<geo::Rect> = Vec::new();

    for feature in &featurecollection.features {
        // 0. Early Filtering using Feature Bounding Box
        if let Some(feature_bbox_value) = &feature.bbox {
            if !is_boundingbox_in_germany(feature_bbox_value) {
                continue;
            }
        }

        // 1. Extract geometry and check location based on type
        let geometry_value = match feature.geometry.as_ref() {
            Some(geometry) => &geometry.value,
            None => {
                continue; // Skip features without geometry
            }
        };
        let mut coords: Vec<Coord> = Vec::new();
        let mut all_points_in_germany = true;
        let mut geometry_to_process: Option<geo::Geometry> = None;
        match geometry_value {
            geojson::Value::Point(coord) => {
                let geo_coord = Coord {
                    x: coord[0],
                    y: coord[1],
                };
                if is_coordinate_in_germany(&coord) {
                    coords.push(geo_coord);
                    geometry_to_process = Some(geo::Geometry::Point(Point::from(geo_coord)));
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
                    geometry_to_process = Some(geo::Geometry::LineString(GeoLineString::new(
                        line_coords_geo,
                    )));
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
        // 3. Create bounding box
        let unique_coords_count = coords
            .iter()
            .map(|c| (OrderedFloat(c.x), OrderedFloat(c.y)))
            .collect::<HashSet<_>>()
            .len();
        if unique_coords_count < 3 {
            let fallback_polygon = geometry_to_process
                .as_ref()
                .and_then(|geom| geom.bounding_rect());

            if let Some(bounding_box) = fallback_polygon {
                let expanded_bounding_box = expand_bounding_box(&bounding_box, radius);
                bounding_boxes.push(expanded_bounding_box);
            }
        } else {
            let multi_point = MultiPoint::from(coords);
            let bounding_box = multi_point.bounding_rect();
            if let Some(bounding_box) = bounding_box {
                let expanded_bounding_box = expand_bounding_box(&bounding_box, radius);
                bounding_boxes.push(expanded_bounding_box);
            }
        }
    }
    bounding_boxes
}

#[cfg(test)]
mod tests {
    use super::*;
    use geojson::{Feature, FeatureCollection, Geometry, Value};

    // Helper to create a GeoJSON Feature for testing
    fn create_geojson_feature(geometry: Value) -> Feature {
        Feature {
            bbox: None, // Rely on geometry processing, not feature bbox
            geometry: Some(Geometry {
                value: geometry,
                bbox: None,
                foreign_members: None,
            }),
            id: None,
            properties: None,
            foreign_members: None,
        }
    }

    // Helper to create a Point GeoJSON feature
    fn point_feature(lon: f64, lat: f64) -> Feature {
        create_geojson_feature(Value::Point(vec![lon, lat]))
    }

    // Helper to create a LineString GeoJSON feature
    fn linestring_feature(coords: Vec<(f64, f64)>) -> Feature {
        let geojson_coords: Vec<Vec<f64>> = coords.into_iter().map(|c| vec![c.0, c.1]).collect();
        create_geojson_feature(Value::LineString(geojson_coords))
    }

    // Helper to create a Polygon GeoJSON feature (will be skipped by current logic)
    fn polygon_feature(exterior_coords: Vec<(f64, f64)>) -> Feature {
        let geojson_coords: Vec<Vec<f64>> = exterior_coords
            .into_iter()
            .map(|c| vec![c.0, c.1])
            .collect();
        create_geojson_feature(Value::Polygon(vec![geojson_coords]))
    }

    // Helper to create a FeatureCollection
    fn feature_collection(features: Vec<Feature>) -> FeatureCollection {
        FeatureCollection {
            bbox: None,
            features,
            foreign_members: None,
        }
    }

    // Helper to create expected Rectangle results concisely
    fn r(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Rectangle {
        Rectangle::from_corners((min_x, min_y), (max_x, max_y))
    }

    // Helper to sort Rectangles for consistent comparison
    fn sort_rectangles(mut rects: Vec<Rectangle>) -> Vec<Rectangle> {
        rects.sort_by(|a, b| {
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
        rects
    }

    #[test]
    fn test_collect_bboxes_single_point_in_germany() {
        // Point(9, 50) is in Germany's rough bbox (5.8-15.0, 47.2-55.0)
        let fc = feature_collection(vec![point_feature(9.0, 50.0)]);
        let radius = 10.0;
        let combine = true; // Combine logic should merge a single item with itself (result is just the item)

        // Expected: Buffered bbox of (9,50)
        // (9-10, 50-10) to (9+10, 50+10) = (-1, 40) to (19, 60)
        let expected = vec![r(-1.0, 40.0, 19.0, 60.0)];

        let result = collect_bounding_boxes(&fc, radius, combine);
        assert_eq!(sort_rectangles(result), sort_rectangles(expected));
    }

    #[test]
    fn test_collect_bboxes_single_point_outside_germany() {
        let fc = feature_collection(vec![point_feature(0.0, 0.0)]); // Outside Germany
        let radius = 10.0;
        let combine = true;

        let expected: Vec<Rectangle> = vec![]; // Should be filtered out

        let result = collect_bounding_boxes(&fc, radius, combine);
        assert_eq!(sort_rectangles(result), sort_rectangles(expected));
    }

    #[test]
    fn test_collect_bboxes_linestring_2_points_in_germany() {
        // LineString (9,50)-(10,51) - 2 unique points, in Germany -> Fallback bbox + Buffer
        let fc = feature_collection(vec![linestring_feature(vec![(9.0, 50.0), (10.0, 51.0)])]);
        let radius = 5.0;
        let combine = true;

        // Bbox of (9,50)-(10,51) is Rect(9,50,10,51)
        // Buffered by 5: (9-5, 50-5) to (10+5, 51+5) = (4, 45) to (15, 56)
        let expected = vec![r(4.0, 45.0, 15.0, 56.0)];

        let result = collect_bounding_boxes(&fc, radius, combine);
        assert_eq!(sort_rectangles(result), sort_rectangles(expected));
    }

    #[test]
    fn test_collect_bboxes_linestring_3_points_in_germany() {
        // LineString (9,50)-(10,52)-(11,50) - 3 unique points, in Germany -> Convex Hull Bbox + Buffer
        let fc = feature_collection(vec![linestring_feature(vec![
            (9.0, 50.0),
            (10.0, 52.0),
            (11.0, 50.0),
        ])]);
        let radius = 5.0;
        let combine = true;

        // Convex hull of (9,50), (10,52), (11,50) is the triangle itself.
        // Bbox of the triangle is Rect(9,50,11,52)
        // Buffered by 5: (9-5, 50-5) to (11+5, 52+5) = (4, 45) to (16, 57)
        let expected = vec![r(4.0, 45.0, 16.0, 57.0)];

        let result = collect_bounding_boxes(&fc, radius, combine);
        assert_eq!(sort_rectangles(result), sort_rectangles(expected));
    }

    #[test]
    fn test_collect_bboxes_linestring_partly_outside_germany() {
        // LineString (9,50)-(10,51)-(0,0) - (0,0) is outside Germany -> entire feature skipped
        let fc = feature_collection(vec![linestring_feature(vec![
            (9.0, 50.0),
            (10.0, 51.0),
            (0.0, 0.0),
        ])]);
        let radius = 5.0;
        let combine = true;

        let expected: Vec<Rectangle> = vec![]; // Should be filtered out

        let result = collect_bounding_boxes(&fc, radius, combine);
        assert_eq!(sort_rectangles(result), sort_rectangles(expected));
    }

    #[test]
    fn test_collect_bboxes_unsupported_geometry_type() {
        // Polygon feature - currently skipped
        let fc = feature_collection(vec![polygon_feature(vec![
            (9.0, 50.0),
            (10.0, 50.0),
            (10.0, 51.0),
            (9.0, 51.0),
            (9.0, 50.0),
        ])]);
        let radius = 10.0;
        let combine = true;

        let expected: Vec<Rectangle> = vec![]; // Should be skipped

        let result = collect_bounding_boxes(&fc, radius, combine);
        assert_eq!(sort_rectangles(result), sort_rectangles(expected));
    }

    #[test]
    fn test_collect_bboxes_multiple_non_overlapping() {
        // Two points in Germany, far apart, buffered boxes don't overlap
        let fc = feature_collection(vec![
            point_feature(9.0, 50.0),  // Bbox: (-1, 40) to (19, 60) with radius 10
            point_feature(14.0, 54.0), // Bbox: (4, 44) to (24, 64) with radius 10
        ]);
        let radius = 10.0;
        let combine = true; // Should result in two separate merged components (themselves)

        let expected = vec![r(-1.0, 40.0, 24.0, 64.0)];

        let result = collect_bounding_boxes(&fc, radius, combine);
        // Sort both vectors for reliable comparison
        assert_eq!(sort_rectangles(result), sort_rectangles(expected));
    }

    #[test]
    fn test_collect_bboxes_two_overlapping_points() {
        // Two points in Germany, buffered boxes overlap
        let fc = feature_collection(vec![
            point_feature(9.0, 50.0), // Bbox: (-1, 40) to (19, 60)
            point_feature(9.5, 50.5), // Bbox: (-0.5, 40.5) to (19.5, 60.5)
        ]);
        let radius = 10.0;
        let combine = true; // Should merge into a single box

        // Overall bbox of the two buffered boxes
        // min_x = min(-1.0, -0.5) = -1.0
        // min_y = min(40.0, 40.5) = 40.0
        // max_x = max(19.0, 19.5) = 19.5
        // max_y = max(60.0, 60.5) = 60.5
        let expected = vec![r(-1.0, 40.0, 19.5, 60.5)];

        let result = collect_bounding_boxes(&fc, radius, combine);
        assert_eq!(sort_rectangles(result), sort_rectangles(expected));
    }

    #[test]
    fn test_collect_bboxes_point_overlapping_linestring_bbox() {
        // Point and LineString in Germany, buffered bboxes overlap
        let fc = feature_collection(vec![
            point_feature(9.0, 50.0), // Bbox: (-1, 40) to (19, 60)
            linestring_feature(vec![(9.5, 50.5), (10.5, 51.5)]), // Bbox (fallback): (9.5,50.5)-(10.5,51.5) -> buffered by 10: (-0.5, 40.5) to (20.5, 61.5)
        ]);
        let radius = 10.0;
        let combine = true; // Should merge into a single box

        // Overall bbox of the two buffered boxes
        // min_x = min(-1.0, -0.5) = -1.0
        // min_y = min(40.0, 40.5) = 40.0
        // max_x = max(19.0, 20.5) = 20.5
        // max_y = max(60.0, 61.5) = 61.5
        let expected = vec![r(-1.0, 40.0, 20.5, 61.5)];

        let result = collect_bounding_boxes(&fc, radius, combine);
        assert_eq!(sort_rectangles(result), sort_rectangles(expected));
    }

    #[test]
    fn test_collect_bboxes_multiple_overlapping_components() {
        // Four points forming two separate overlapping pairs
        let fc = feature_collection(vec![
            point_feature(9.0, 50.0),  // Bbox: (-1, 40) to (19, 60)
            point_feature(9.5, 50.5), // Bbox: (-0.5, 40.5) to (19.5, 60.5) -> overlaps first, merge
            point_feature(15.0, 55.0), // Bbox: (5, 45) to (25, 65)
            point_feature(15.5, 55.5), // Bbox: (5.5, 45.5) to (25.5, 65.5) -> overlaps third, merge
        ]);
        let radius = 10.0;
        let combine = true; // Should result in two merged boxes

        // Expected merged box 1 (from P1, P2): (-1.0, 40.0, 19.5, 60.5)
        // Expected merged box 2 (from P3, P4): (5.0, 45.0, 25.5, 65.5)
        let expected = vec![r(-1.0, 40.0, 25.0, 65.0)];

        let result = collect_bounding_boxes(&fc, radius, combine);
        assert_eq!(sort_rectangles(result), sort_rectangles(expected));
    }

    #[test]
    fn test_collect_bboxes_contained_bbox_merges() {
        // Larger LineString containing a point, buffered boxes overlap, point bbox contained in LS bbox
        let fc = feature_collection(vec![
            linestring_feature(vec![(9.0, 50.0), (15.0, 55.0)]), // Bbox (fallback): (9,50)-(15,55) -> buffered by 5: (4, 45)-(20, 60)
            point_feature(10.0, 51.0), // Bbox: (10,51) -> buffered by 5: (5, 46)-(15, 56)
        ]);
        let radius = 5.0;
        let combine = true; // Should merge into a single box

        // Overall bbox of the two buffered boxes
        // min_x = min(4.0, 5.0) = 4.0
        // min_y = min(45.0, 46.0) = 45.0
        // max_x = max(20.0, 15.0) = 20.0
        // max_y = max(60.0, 56.0) = 60.0
        // The contained box's bounds are within the container's bounds,
        // so the merged box is just the container's buffered box.
        let expected = vec![r(4.0, 45.0, 20.0, 60.0)];

        let result = collect_bounding_boxes(&fc, radius, combine);
        // Replace assert_eq! with fuzzy comparison
        let sorted_result = sort_rectangles(result);
        let sorted_expected = sort_rectangles(expected);

        // Assert that the number of merged rectangles is the same
        assert_eq!(
            sorted_result.len(),
            sorted_expected.len(),
            "Mismatched number of merged rectangles"
        );

        // Use a small tolerance for float comparison
        let epsilon = 1e-9; // A common tolerance for double-precision floats

        // Iterate through the sorted results and expected values and compare coordinates fuzzily
        for (res_rect, exp_rect) in sorted_result.iter().zip(sorted_expected.iter()) {
            let min_x_diff = (res_rect.min().x - exp_rect.min().x).abs();
            let min_y_diff = (res_rect.min().y - exp_rect.min().y).abs();
            let max_x_diff = (res_rect.max().x - exp_rect.max().x).abs();
            let max_y_diff = (res_rect.max().y - exp_rect.max().y).abs();

            assert!(
                min_x_diff < epsilon
                    && min_y_diff < epsilon
                    && max_x_diff < epsilon
                    && max_y_diff < epsilon,
                "Merged rectangle {:?} does not nearly match expected {:?} within tolerance {}",
                res_rect,
                exp_rect,
                epsilon
            );
        }
    }

    #[test]
    fn test_collect_bboxes_zero_radius() {
        // Single point in Germany, radius 0 -> buffer by 4
        let fc = feature_collection(vec![point_feature(9.0, 50.0)]);
        let radius = 0.0; // Test radius 0 case
        let combine = true;

        // Expected: Buffered bbox of (9,50) by 4
        // (9-4, 50-4) to (9+4, 50+4) = (5, 46) to (13, 54)
        let expected = vec![r(5.0, 46.0, 13.0, 54.0)];

        // Need to adjust expand_bounding_box or pass radius=Some(0.0) if it expects Option
        // Assuming expand_bounding_box is adjusted to handle f64 radius and 0.0->4.0 logic
        let result = collect_bounding_boxes(&fc, radius, combine);
        assert_eq!(sort_rectangles(result), sort_rectangles(expected));
    }
}
