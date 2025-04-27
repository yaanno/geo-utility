use geo::{BoundingRect, Coord, Intersects, Rect};

const GERMANY_BBOX: [f64; 4] = [
    5.866211,  // Min longitude
    47.270111, // Min latitude
    15.013611, // Max longitude
    55.058333, // Max latitude
];
/// Checks if a coordinate is within the bounding box of Germany.
///
/// # Arguments
/// * `coord` - The coordinate to check.
/// # Returns
/// `true` if the coordinate is within the bounding box, `false` otherwise.
pub fn is_coordinate_in_germany(coord: &[f64]) -> bool {
    // Ensure coord has at least 2 elements (longitude, latitude)
    if coord.len() < 2 {
        return false; // Cannot check if coords are invalid
    }

    coord[0] >= GERMANY_BBOX[0]
        && coord[0] <= GERMANY_BBOX[2]
        && coord[1] >= GERMANY_BBOX[1]
        && coord[1] <= GERMANY_BBOX[3]
}

pub fn is_boundingbox_in_germany(coords: &Vec<f64>) -> bool {
    if coords.len() < 4 {
        return false;
    }
    let coords = Rect::new(
        Coord {
            x: coords[0],
            y: coords[1],
        },
        Coord {
            x: coords[2],
            y: coords[3],
        },
    );
    let germany_rect = Rect::new(
        Coord {
            x: GERMANY_BBOX[0],
            y: GERMANY_BBOX[1],
        },
        Coord {
            x: GERMANY_BBOX[2],
            y: GERMANY_BBOX[3],
        },
    );

    coords.intersects(&germany_rect)
}

#[allow(dead_code)]
/// Expands a bounding box by a given radius.
///
/// # Arguments
/// * `bbox` - The bounding box to expand.
/// * `radius` - The radius by which to expand the bounding box.
/// # Returns
/// The expanded bounding box.
pub fn expand_bounding_box(bbox: &Rect, radius: f64) -> Rect {
    let expansion_amount = if radius == 0.0 { 4.0 } else { radius };
    let expanded_min_x = bbox.min().x - expansion_amount;
    let expanded_max_x = bbox.max().x + expansion_amount;
    let expanded_min_y = bbox.min().y - expansion_amount;
    let expanded_max_y = bbox.max().y + expansion_amount;
    Rect::new(
        Coord {
            x: expanded_min_x,
            y: expanded_min_y,
        },
        Coord {
            x: expanded_max_x,
            y: expanded_max_y,
        },
    )
}

#[allow(dead_code)]
/// Extends a bounding box to ensure it is a multiple of a given area length.
///
/// # Arguments
/// * `bbox` - The bounding box to extend.
/// * `area_length` - The length of the area to which the bounding box should be extended.
/// # Returns
/// The extended bounding box.
pub fn extend_bounding_box(bbox: &geo::Rect, area_length: f64) -> geo::Rect {
    let width = bbox.width();
    let height = bbox.height();

    let length_width = if width < area_length {
        area_length + 1.0 - width
    } else {
        area_length + 1.0 - (width % area_length)
    };

    let length_height = if height < area_length {
        area_length + 1.0 - height
    } else {
        area_length + 1.0 - (height % area_length)
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

#[allow(dead_code)]
pub fn convert_polygons_to_bounding_boxes(polygons: Vec<geo::Polygon>) -> Vec<geo::Rect> {
    polygons
        .into_iter()
        .map(|poly| poly.bounding_rect().unwrap())
        .collect()
}

#[allow(dead_code)]
/// Creates a square grid from a bounding box with specified cell dimensions.
///
/// # Arguments
/// * `bbox` - The bounding box from which to create the grid.
/// * `cell_width` - The width of each cell in the grid.
/// * `cell_height` - The height of each cell in the grid.
/// # Returns
/// A vector of rectangles representing the grid cells.
pub fn create_square_grid(bbox: geo::Rect, cell_width: f64, cell_height: f64) -> Vec<geo::Rect> {
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
    use geo::{Coord, LineString, MultiPoint, Point, Polygon, Rect};
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
}
