use geo::{BoundingRect, Contains, Coord, Intersects, Point, Rect};

pub const GERMANY_BBOX: [f64; 4] = [
    5.866211,  // Min longitude
    47.270111, // Min latitude
    15.013611, // Max longitude
    55.058333, // Max latitude
];


/// Trait for checking if a geographic coordinate is within a specific bounding box.
pub trait InBoundingBox {
    /// Checks if the coordinate is within the specified bounding box.
    /// 
    /// # Arguments
    /// * `bbox`: The bounding box to check against
    /// 
    /// # Returns
    /// * `true` if the coordinate is within the bounding box, `false` otherwise
    fn in_bounding_box(&self, bbox: &[f64; 4]) -> bool;
}

/// Trait for expanding and extending bounding boxes.
pub trait BoundingBoxOps {
    /// Expands a bounding box by a given radius.
    /// 
    /// # Arguments
    /// * `radius`: The amount to expand the box by
    /// 
    /// # Returns
    /// A new expanded bounding box
    fn expand(&self, radius: f64) -> Rect;
    
    #[allow(dead_code)]
    /// Extends a bounding box to align with a grid of specified cell size.
    /// 
    /// # Arguments
    /// * `cell_size`: The size of the grid cells
    /// 
    /// # Returns
    /// A new extended bounding box
    fn extend(&self, cell_size: f64) -> Rect;
}

impl InBoundingBox for [f64; 2] {
    fn in_bounding_box(&self, bbox: &[f64; 4]) -> bool {
        let point = Point::new(self[0], self[1]);
        let rect = Rect::new(
            Coord { x: bbox[0], y: bbox[1] },
            Coord { x: bbox[2], y: bbox[3] },
        );
        rect.contains(&point)
    }
}

impl InBoundingBox for Vec<f64> {
    fn in_bounding_box(&self, bbox: &[f64; 4]) -> bool {
        if self.len() != 4 {
            return false;
        }
        let coords = Rect::new(
            Coord { x: self[0], y: self[1] },
            Coord { x: self[2], y: self[3] },
        );
        let rect = Rect::new(
            Coord { x: bbox[0], y: bbox[1] },
            Coord { x: bbox[2], y: bbox[3] },
        );
        rect.intersects(&coords)
    }
}

impl InBoundingBox for geo::Coord {
    fn in_bounding_box(&self, bbox: &[f64; 4]) -> bool {
        let point = Point::new(self.x, self.y);
        let rect = Rect::new(
            Coord { x: bbox[0], y: bbox[1] },
            Coord { x: bbox[2], y: bbox[3] },
        );
        rect.contains(&point)
    }
}

impl BoundingBoxOps for Rect {
    fn expand(&self, radius: f64) -> Rect {
        let expansion_amount = if radius == 0.0 { 4.0 } else { radius };
        let expanded_min_x = self.min().x - expansion_amount;
        let expanded_max_x = self.max().x + expansion_amount;
        let expanded_min_y = self.min().y - expansion_amount;
        let expanded_max_y = self.max().y + expansion_amount;
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
    
    fn extend(&self, cell_size: f64) -> Rect {
        if cell_size <= 0.0 {
            return *self;
        }
        
        let width = self.width();
        let height = self.height();
        
        let extra_width = cell_size * ((width / cell_size).ceil() - (width / cell_size));
        let extra_height = cell_size * ((height / cell_size).ceil() - (height / cell_size));
        
        let new_min_x = self.min().x - extra_width / 2.0;
        let new_min_y = self.min().y - extra_height / 2.0;
        let new_max_x = self.max().x + extra_width / 2.0;
        let new_max_y = self.max().y + extra_height / 2.0;
        
        Rect::new(
            Coord {
                x: new_min_x,
                y: new_min_y,
            },
            Coord {
                x: new_max_x,
                y: new_max_y,
            },
        )
    }
}


#[allow(dead_code)]
pub fn convert_polygons_to_bounding_boxes(polygons: Vec<geo::Polygon>) -> Vec<Rect> {
    polygons
        .into_iter()
        .map(|poly| poly.bounding_rect().unwrap())
        .collect()
}

#[allow(dead_code)]
pub struct Grid {
    pub cells: Vec<Rect>,
    num_cols: usize,
    num_rows: usize,
    cell_width: f64,
    cell_height: f64,
}

impl Grid {
    pub fn new(bbox: Rect, cell_width: f64, cell_height: f64) -> Self {
        if cell_width <= 0.0 || cell_height <= 0.0 {
            return Self::empty();
        }
    
        let bbox_min_x = bbox.min().x;
        let bbox_min_y = bbox.min().y;
        let bbox_max_x = bbox.max().x;
        let bbox_max_y = bbox.max().y;
    
        // Handle cases where bbox is degenerate or inverted (max <= min)
        if bbox_min_x >= bbox_max_x || bbox_min_y >= bbox_max_y {
            return Self::empty();
        }

        let num_cols = ((bbox_max_x - bbox_min_x) / cell_width).ceil() as usize;
        let num_rows = ((bbox_max_y - bbox_min_y) / cell_height).ceil() as usize;
        let mut cells = Vec::with_capacity(num_cols * num_rows);
    
        for i in 0..num_cols {
            let x = bbox_min_x + (i as f64 * cell_width);
            for j in 0..num_rows {
                let y = bbox_min_y + (j as f64 * cell_height);
                let cell_rect = Rect::new(
                    Coord { x, y },
                    Coord {
                    x: f64::min(x + cell_width, bbox_max_x),
                    y: f64::min(y + cell_height, bbox_max_y),
                },
            );
            cells.push(cell_rect);
        }
    }
    
        Self { cells, num_cols, num_rows, cell_width, cell_height }
    }

    pub fn empty() -> Self {
        Self {
            cells: Vec::new(),
            num_cols: 0,
            num_rows: 0,
            cell_width: 0.0,
            cell_height: 0.0,
        }
    }

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

        let grid = Grid::new(bbox, cell_width, cell_height);
        assert_eq!(grid.cells.len(), expected_grid.len());
        assert_eq!(grid.cells, expected_grid);
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

        let grid = Grid::new(bbox, cell_width, cell_height);

        // Sort both vectors for reliable comparison, as the order might depend on loop implementation
        let mut grid_sorted = grid;
        grid_sorted.cells.sort_by(|a, b| {
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

        assert_eq!(grid_sorted.cells.len(), expected_grid_sorted.len());
        assert_eq!(grid_sorted.cells, expected_grid_sorted);
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

        let grid = Grid::new(bbox, cell_width, cell_height);

        // Sort both vectors for reliable comparison
        let mut grid_sorted = grid;
        grid_sorted.cells.sort_by(|a, b| {
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

        assert_eq!(grid_sorted.cells.len(), expected_grid_sorted.len());
        assert_eq!(grid_sorted.cells, expected_grid_sorted);
    }

    #[test]
    fn test_create_square_grid_bbox_smaller_than_cell() {
        let bbox = r(10.0, 20.0, 50.0, 70.0);
        let cell_width = 200.0;
        let cell_height = 200.0;

        // Should result in a single cell clipped to the bbox
        let expected_grid = vec![r(10.0, 20.0, 50.0, 70.0)];

        let grid = Grid::new(bbox, cell_width, cell_height);
        assert_eq!(grid.cells.len(), expected_grid.len());
        assert_eq!(grid.cells, expected_grid);
    }

    #[test]
    fn test_create_square_grid_zero_cell_width() {
        let bbox = r(0.0, 0.0, 200.0, 200.0);
        let cell_width = 0.0;
        let cell_height = 200.0;

        let expected_grid: Vec<Rect> = vec![]; // Should return empty grid

        let grid = Grid::new(bbox, cell_width, cell_height);
        assert_eq!(grid.cells.len(), expected_grid.len());
        assert_eq!(grid.cells, expected_grid);
    }

    #[test]
    fn test_create_square_grid_zero_cell_height() {
        let bbox = r(0.0, 0.0, 200.0, 200.0);
        let cell_width = 200.0;
        let cell_height = 0.0;

        let expected_grid: Vec<Rect> = vec![]; // Should return empty grid

        let grid = Grid::new(bbox, cell_width, cell_height);
        assert_eq!(grid.cells.len(), expected_grid.len());
        assert_eq!(grid.cells, expected_grid);
    }

    #[test]
    fn test_create_square_grid_zero_bbox_area() {
        let bbox = r(0.0, 0.0, 0.0, 0.0); // Point-like bbox
        let cell_width = 10.0;
        let cell_height = 10.0;

        let expected_grid: Vec<Rect> = vec![]; // Should return empty grid

        let grid = Grid::new(bbox, cell_width, cell_height);
        assert_eq!(grid.cells.len(), expected_grid.len());
        assert_eq!(grid.cells, expected_grid);
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

        let grid = Grid::new(bbox, cell_width, cell_height);

        // Sort both vectors for reliable comparison
        let mut grid_sorted = grid;
        grid_sorted.cells.sort_by(|a, b| {
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

        assert_eq!(grid_sorted.cells.len(), expected_grid_sorted.len());
        assert_eq!(grid_sorted.cells, expected_grid_sorted);
    }
}
