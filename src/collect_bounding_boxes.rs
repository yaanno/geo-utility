use std::collections::{HashMap, HashSet};

use crate::geometry::{Rectangle, RectangleWithId};
use crate::utils::{expand_bounding_box, is_coordinate_in_germany};
use geo::geometry::LineString as GeoLineString;
use geo::{BoundingRect, Coord, Intersects, MultiPoint, Point, Rect};
use ordered_float::OrderedFloat;
use rstar::{RTree, RTreeObject};
use union_find::{QuickUnionUf, UnionBySize, UnionFind};

pub fn collect_bounding_boxes(
    featurecollection: &geojson::FeatureCollection,
    radius: f64,
    _combine: bool,
) -> Vec<Rectangle> {
    let mut bounding_boxes: Vec<geo::Rect> = Vec::new();
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
        // 0. Early Filtering using Feature Bounding Box
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
    let rectangles: Vec<Rectangle> = bounding_boxes
        .clone()
        .into_iter()
        .map(Rectangle::from)
        .collect();

    let rtree_data: Vec<RectangleWithId> = rectangles
        .clone()
        .into_iter()
        .enumerate()
        .map(|(i, rect)| RectangleWithId(rect, i))
        .collect();

    let tree = RTree::bulk_load(rtree_data);

    let mut uf = QuickUnionUf::<UnionBySize>::new(bounding_boxes.len());
    for (i, rect) in rectangles.iter().enumerate() {
        // Query the R-tree to find rectangles overlapping the current 'rect'
        // locate_in_envelope_intersecting returns iter of &(Rectangle, usize)
        for RectangleWithId(_overlapping_rect, j) in
            tree.locate_in_envelope_intersecting(&rect.envelope())
        {
            // 'candidate_tuple_ref' is &(Rectangle, usize)
            // 'overlapping_rect' is &Rectangle
            // 'j' is usize (the original index of the overlapping rectangle)

            // We found an overlap between rectangle 'i' and rectangle 'j'.
            // Ensure we don't try to union an item with itself.
            if i != *j {
                // Perform the union operation in the Union-Find structure.
                // The union_find crate's union method merges the sets containing i and j.
                // It's efficient even if i and j are already in the same set.
                uf.union(i, *j);
            }
        }
    }

    // Step 3: Merge rectangles within each connected component.
    let mut components: HashMap<usize, Vec<&Rectangle>> = HashMap::new();

    for (i, rect) in rectangles.iter().enumerate() {
        let root = uf.find(i);
        components.entry(root).or_default().push(rect);
    }

    // Now merge rectangles in each component to compute the overall bounding box.
    let merged_rectangles: Vec<Rectangle> = components
        .into_iter()
        .map(|(_root, group)| {
            let (min_x, min_y, max_x, max_y) = group.iter().fold(
                (
                    f64::INFINITY,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                    f64::NEG_INFINITY,
                ),
                |(min_x, min_y, max_x, max_y), &r| {
                    (
                        min_x.min(r.min().x),
                        min_y.min(r.min().y),
                        max_x.max(r.max().x),
                        max_y.max(r.max().y),
                    )
                },
            );
            Rectangle::from_corners((min_x, min_y), (max_x, max_y))
        })
        .collect();

    merged_rectangles
}
