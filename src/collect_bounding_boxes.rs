use crate::geometry::Rectangle;
use crate::grouping::{group_rects_by_overlap, merge_components};
use crate::utils::{
    create_square_grid, expand_bounding_box, is_boundingbox_in_germany, is_coordinate_in_germany,
};
use geo::geometry::LineString as GeoLineString;
use geo::{BoundingRect, ConvexHull, Coord, MultiPoint, Point, Rect};
use ordered_float::OrderedFloat;
use proj::{Proj, ProjCreateError};
use rstar::RTreeObject;
use std::collections::HashSet;

// Consider creating a new type for radius to ensure it's always positive
#[derive(Debug, Clone, Copy)]
pub struct Radius(f64);

impl Radius {
    pub fn new(radius: f64) -> Result<Self, CollectBoundingBoxError> {
        if radius < 0.0 {
            Err(CollectBoundingBoxError::InvalidRadius)
        } else {
            Ok(Self(radius))
        }
    }

    pub fn get(&self) -> f64 {
        self.0
    }
}

// Consider making TARGET_NUM_CELLS configurable
pub struct GridConfig {
    pub target_num_cells: f64,
    #[allow(dead_code)]
    pub min_cells: f64,
    #[allow(dead_code)]
    pub max_cells: f64,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            target_num_cells: 20.0,
            min_cells: 2.0,
            max_cells: 10.0,
        }
    }
}


#[derive(Debug)]
pub enum CollectBoundingBoxError {
    ProjCreateError(ProjCreateError),
    ProjTransformError,
    EmptyInput,
    InvalidArea,
    InvalidCellSize,
    InvalidRadius,
}

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
    radius: Radius,
    _combine: bool,
) -> Result<Vec<Rectangle>, CollectBoundingBoxError> {
    if featurecollection.features.is_empty() {
        return Err(CollectBoundingBoxError::EmptyInput);
    }
    let from_crs = "EPSG:4326";
    let to_crs = "EPSG:3035";

    let proj_transformer =
        Proj::new_known_crs(&from_crs, &to_crs, None).map_err(|e| CollectBoundingBoxError::ProjCreateError(e))?;
    let proj_transformer_reverse = Proj::new_known_crs(&to_crs, &from_crs, None)
        .map_err(|e| CollectBoundingBoxError::ProjCreateError(e))?;
    let initial_geo_rects =
        collect_initial_buffered_rects(featurecollection, radius.get(), &proj_transformer);

    let rectangles: Vec<Rectangle> = initial_geo_rects.into_iter().map(Rectangle::from).collect();
    let overall_initial_extent = calculate_overall_extent(&rectangles)?;

    if rectangles.is_empty() {
        return Err(CollectBoundingBoxError::EmptyInput);
    }

    let initial_grid_cells = calculate_initial_grid_cells(Some(overall_initial_extent))?;

    let uf = group_rects_by_overlap(&rectangles);
    let merged_rectangles: Vec<Rectangle> = merge_components(&rectangles, uf);

    let tree = crate::grouping::index_rectangles(&merged_rectangles);

    let grid_cells_intersecting_shapes =    
        create_transformed_grid_cells(proj_transformer_reverse, initial_grid_cells, tree);

    grid_cells_intersecting_shapes
}

/**
 * Creates grid cells intersecting shapes.
 *
 * # Arguments
 * `proj_transformer_reverse` - The reverse PROJ transformer.
 * `initial_grid_cells` - The initial grid cells.
 * `tree` - The R-tree containing the indexed rectangles.
 *
 * # Returns
 * A vector of grid cells intersecting shapes.
 */
fn create_transformed_grid_cells(
    proj_transformer_reverse: Proj,
    initial_grid_cells: Vec<Rect>,
    tree: rstar::RTree<crate::geometry::RectangleWithId>,
) -> Result<Vec<Rectangle>, CollectBoundingBoxError> {
    let grid_cells_intersecting_shapes: Vec<Rectangle> = initial_grid_cells
        .into_iter()
        .map(Rectangle::from)
        .filter(|grid_cell| {
            tree.locate_in_envelope_intersecting(&grid_cell.envelope())
                .next()
                .is_some()
        })
        .map(|grid_cell_projected| {
            let min_geographic = match proj_transformer_reverse
                            .convert(grid_cell_projected.min()) {
                Ok(it) => it,
                Err(_) => return Err(CollectBoundingBoxError::ProjTransformError),
            };
            let max_geographic = match proj_transformer_reverse
                            .convert(grid_cell_projected.max()) {
                Ok(it) => it,
                Err(_) => return Err(CollectBoundingBoxError::ProjTransformError),
            };
            Ok(Rectangle::from_corners(
                (min_geographic.x, min_geographic.y),
                (max_geographic.x, max_geographic.y),
            ))
        })
        .collect::<Result<Vec<Rectangle>, CollectBoundingBoxError>>()?;
    Ok(grid_cells_intersecting_shapes)
}

/**
 * Calculates the initial grid cells based on the overall initial extent.
 *
 * # Arguments
 * `overall_initial_extent` - The overall initial extent.
 *
 * # Returns
 * A Result containing a vector of grid cells or an empty vector if the calculation fails.
 */
fn calculate_initial_grid_cells(
    overall_initial_extent: Option<Rect>,
) -> Result<Vec<Rect>, CollectBoundingBoxError> {
    let initial_grid_cells: Vec<Rect>;
    if let Some(overall_initial_extent) = overall_initial_extent {
        let area = overall_initial_extent.height() * overall_initial_extent.width();
        let target_num_cells = GridConfig::default().target_num_cells;

        if area <= 0.0 {
            return Err(CollectBoundingBoxError::InvalidArea);
        }
        let area_per_cell = area / target_num_cells;

        if area_per_cell <= 0.0 {
            return Err(CollectBoundingBoxError::InvalidArea);
        }
        let calculated_cell_size_meters = area_per_cell.sqrt();

        initial_grid_cells = create_square_grid(
            overall_initial_extent,
            calculated_cell_size_meters,
            calculated_cell_size_meters,
        );
    } else {
        return Err(CollectBoundingBoxError::InvalidCellSize);
    }
    Ok(initial_grid_cells)
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
pub fn calculate_overall_extent(rectangles: &[Rectangle]) -> Result<Rect, CollectBoundingBoxError> {
    if rectangles.is_empty() {
        return Err(CollectBoundingBoxError::EmptyInput);
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

    Ok(Rect::new(
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
    use geojson::{Feature, FeatureCollection};

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

    // A simple test case for the new pipeline with dynamic grid sizing
    #[test]
    fn test_collect_bboxes_dynamic_grid_simple() {
        // Two points in Germany, close together, buffered boxes should overlap and merge
        let input_features = vec![
            point_feature(9.0, 50.0),     // Point 1
            point_feature(9.1, 50.1),     // Point 2, very close
            point_feature(8.9, 49.9),     // Point 3, also close
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

        let result_rectangles = collect_bounding_boxes(&fc, Radius::new(radius).unwrap(), combine).unwrap();

        // --- Convert result Rectangles to GeoJSON Features ---
        // let result_rectangles = result_rectangles.unwrap();
        // let result_features: Vec<Feature> = result_rectangles
        //     .iter()
        //     .map(|rect| {
        //         let geo_rect = rect.to_geo_rect(); // Convert your Rectangle to geo::Rect
        //         let polygon = geo::Polygon::from(geo_rect); // Convert geo::Rect to geo::Polygon
        //         let geometry = Geometry::from(&polygon); // Convert geo::Polygon to geojson::Geometry
        //         Feature {
        //             bbox: None,
        //             geometry: Some(geometry),
        //             id: None,
        //             properties: None,
        //             foreign_members: None,
        //         }
        //     })
        //     .collect();

        // // --- Combine input points and output grid cells into one FeatureCollection ---
        // let mut all_features = input_features; // Start with the input points
        // all_features.extend(result_features); // Add the resulting grid cells
        // let output_fc = feature_collection(all_features);
        // let geojson_output = GeoJson::from(output_fc);
        // let geojson_string = geojson_output.to_string();

        // // --- Print the GeoJSON string ---
        // println!("--- GeoJSON Output for Visualization ---");
        // println!("{}", geojson_string);
        // println!("--- End GeoJSON Output ---");
        // // Copy the string between the markers and paste into geojson.io or save as a .geojson file

        // // Assertions
        // // 1. The result vector is not empty
        // assert!(
        //     !result_rectangles.is_empty(),
        //     "Resulting grid should not be empty"
        // );

        // 2. The number of resulting grid cells (I) is close to the target (e.g., 10)
        //    Allow for some variability in the actual number of cells generated
        let grid_config = GridConfig::default();
        let expected_min_cells = grid_config.min_cells as usize; // Allow +/- a few cells around the target
        let expected_max_cells = grid_config.max_cells as usize; // Adjust this range based on your actual dynamic calc behavior
        assert!(
            result_rectangles.len() >= expected_min_cells
                && result_rectangles.len() <= expected_max_cells,
            "Number of resulting grid cells ({}) should be between {} and {}",
            result_rectangles.len(),
            expected_min_cells,
            expected_max_cells
        );

        // 3. The overall bounding box of the resulting grid cells is reasonable
        //    This is a tougher assertion due to dynamic sizing and projection noise.
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
        // Calculate the center using all three points
        let expected_center_x = (9.0 + 9.1 + 8.9) / 3.0;
        let expected_center_y = (50.0 + 50.1 + 49.9) / 3.0;
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
