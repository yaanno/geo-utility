use crate::geometry::{Rectangle, RectangleWithId};
use rstar::{RTree, RTreeObject};
use std::collections::HashMap;
use union_find::{QuickUnionUf, UnionBySize, UnionFind};

/**
* Merges rectangles in each component to compute the overall bounding box.

* # Arguments
* `rectangles` - The rectangles to merge.
* `uf` - The Union-Find structure representing the groups.
*
* # Returns
* A vector of merged rectangles
*/
pub fn merge_components(
    rectangles: &[Rectangle],
    mut uf: QuickUnionUf<UnionBySize>,
) -> Vec<Rectangle> {
    let capacity = rectangles.len();
    let mut components: HashMap<usize, Vec<&Rectangle>> = HashMap::with_capacity(capacity);

    for (i, rect) in rectangles.iter().enumerate() {
        // Iterating over &Rectangle here
        let root = uf.find(i);

        let group_for_root = components.entry(root).or_default();
        group_for_root.push(rect);
    }

    // Now merge rectangles in each component to compute the overall bounding box.
    let merged_rectangles: Vec<Rectangle> = components
        .into_iter()
        .map(|(_root, group)| {
            let (min_x, min_y, max_x, max_y) = group.iter().fold(
                // Iterating over &Rectangle here
                (
                    f64::INFINITY,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                    f64::NEG_INFINITY,
                ),
                |(min_x, min_y, max_x, max_y), r| {
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

/**
 * Groups rectangles by overlap using an R-tree and Union-Find.
 *
 * # Arguments
 * `rectangles` - The rectangles to group.
 *
 * # Returns
 * A Union-Find structure representing the groups.
 */
pub fn group_rects_by_overlap(rectangles: &[Rectangle]) -> QuickUnionUf<UnionBySize> {
    let tree = index_rectangles(rectangles);

    let mut uf = QuickUnionUf::<UnionBySize>::new(rectangles.len());
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
    uf
}

/**
 * Indexes rectangles using an R-tree.
 *
 * # Arguments
 * `rectangles` - The rectangles to index.
 *
 * # Returns
 * An R-tree containing the indexed rectangles.
 */
pub fn index_rectangles(rectangles: &[Rectangle]) -> RTree<RectangleWithId> {
    let rtree_data: Vec<RectangleWithId> = rectangles
        .into_iter()
        .enumerate()
        .map(|(i, rect)| RectangleWithId(rect.clone(), i))
        .collect();

    let tree = RTree::bulk_load(rtree_data);
    tree
}
