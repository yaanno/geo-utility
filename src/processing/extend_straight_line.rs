use geo::{
    Bearing, Destination, Distance, Geodesic, algorithm::line_measures::metric_spaces::Euclidean,
};

#[allow(dead_code)]
/// Extends a straight line from point A to point B by a specified distance.
///
/// # Arguments
///
/// * `point_a` - The starting point of the line.
/// * `point_b` - The ending point of the line.
/// * `extension` - The distance to extend the line.
/// * `is_geodesic` - If true, uses geodesic calculations; otherwise, uses Euclidean.
///
/// # Returns
///
/// The extended point.
///
/// # Examples
///
/// ```rust
/// use geo::Point;
/// use geo_utility::processing::extend_straight_line::extend_straight_line;
///
/// let point_a = Point::new(0.0, 0.0);
/// let point_b = Point::new(1.0, 1.0);
/// let extension = 1.0;
/// let is_geodesic = false;
///
/// let extended_point = extend_straight_line(point_a, point_b, extension, is_geodesic);
/// ```
pub fn extend_straight_line(
    point_a: geo::Point<f64>,
    point_b: geo::Point<f64>,
    extension: f64,
    is_geodesic: bool,
) -> geo::Point<f64> {
    if point_a.eq(&point_b) {
        return point_b;
    }

    if is_geodesic {
        // --- Correct Geodesic Extension Logic ---
        let bearing = Geodesic.bearing(point_a, point_b);
        let extended_point_geo = Geodesic.destination(point_b, bearing, extension);
        extended_point_geo
    } else {
        // --- Correct Euclidean Extension Logic ---
        let distance = Euclidean.distance(&point_a, &point_b);

        // The dx and dy represent the Euclidean vector components
        let dx = point_b.x() - point_a.x();
        let dy = point_b.y() - point_a.y();

        // Apply the linear extension formula using the Euclidean distance
        let new_x = point_b.x() + (dx / distance) * extension;
        let new_y = point_b.y() + (dy / distance) * extension;

        geo::Point::new(new_x, new_y) // Return the Euclidean extended point
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use geo::Point;
    use std::f64::EPSILON; // Standard epsilon for f64 comparisons

    // Helper function for approximate point equality
    fn points_approx_equal(p1: Point<f64>, p2: Point<f64>, epsilon: f64) -> bool {
        (p1.x() - p2.x()).abs() < epsilon && (p1.y() - p2.y()).abs() < epsilon
    }

    #[test]
    fn test_extend_straight_line_same_points() {
        let point_a = Point::new(0.0, 0.0);
        let point_b = Point::new(0.0, 0.0);
        let extension = 1.0;
        // is_geodesic value doesn't matter here due to early return
        let is_geodesic = false;

        let extended_point = extend_straight_line(point_a, point_b, extension, is_geodesic);
        assert_eq!(extended_point, Point::new(0.0, 0.0));
    }

    // --- Euclidean Tests ---

    #[test]
    fn test_extend_straight_line_euclidean_basic_x_axis() {
        let point_a = Point::new(0.0, 0.0);
        let point_b = Point::new(2.0, 0.0); // Distance 2
        let extension = 2.0; // Extend by 2
        let is_geodesic = false;

        let extended_point = extend_straight_line(point_a, point_b, extension, is_geodesic);
        let expected_point = Point::new(4.0, 0.0); // Start at (2,0), move 2 more units in x direction
        assert!(points_approx_equal(extended_point, expected_point, EPSILON));
    }

    #[test]
    fn test_extend_straight_line_euclidean_basic_diagonal() {
        let point_a = Point::new(0.0, 0.0);
        let point_b = Point::new(1.0, 1.0); // Distance sqrt(2)
        let extension = Euclidean.distance(&point_a, &point_b); // Extend by original distance
        let is_geodesic = false;

        let extended_point = extend_straight_line(point_a, point_b, extension, is_geodesic);
        let expected_point = Point::new(2.0, 2.0); // Start at (1,1), move (1,1) more
        assert!(points_approx_equal(extended_point, expected_point, EPSILON));
    }

    #[test]
    fn test_extend_straight_line_euclidean_diagonal_unit_extension() {
        let point_a = Point::new(0.0, 0.0);
        let point_b = Point::new(1.0, 1.0); // Distance sqrt(2)
        let extension = 1.0; // Extend by 1 unit length
        let is_geodesic = false;

        let extended_point = extend_straight_line(point_a, point_b, extension, is_geodesic);

        // Expected point: (1,1) + ( (1,1) / sqrt(2) ) * 1
        let expected_x = 1.0 + 1.0 / f64::sqrt(2.0);
        let expected_y = 1.0 + 1.0 / f64::sqrt(2.0);
        let expected_point = Point::new(expected_x, expected_y);

        assert!(points_approx_equal(extended_point, expected_point, EPSILON));
    }

    #[test]
    fn test_extend_straight_line_euclidean_extension_zero() {
        let point_a = Point::new(10.0, 20.0);
        let point_b = Point::new(15.0, 25.0);
        let extension = 0.0; // Extend by 0
        let is_geodesic = false;

        let extended_point = extend_straight_line(point_a, point_b, extension, is_geodesic);
        let expected_point = point_b; // Should return point_b
        assert!(points_approx_equal(extended_point, expected_point, EPSILON));
    }

    #[test]
    fn test_extend_straight_line_euclidean_negative_coords() {
        let point_a = Point::new(-5.0, -5.0);
        let point_b = Point::new(-10.0, -10.0); // Distance sqrt(50)
        let extension = Euclidean.distance(&point_a, &point_b); // Extend by original distance
        let is_geodesic = false;

        let extended_point = extend_straight_line(point_a, point_b, extension, is_geodesic);
        // Start at (-10,-10), move (-5,-5) - (-10,-10) = (5,5) more
        // let expected_point = Point::new(
        //     -10.0 + (point_b.x() - point_a.x()),
        //     -10.0 + (point_b.y() - point_a.y()),
        // );
        // let expected_point = Point::new(-10.0 + (-10.0 - (-5.0)), -10.0 + (-10.0 - (-5.0)));
        // let expected_point = Point::new(-10.0 - 5.0, -10.0 - 5.0);
        let expected_point = Point::new(-15.0, -15.0);
        assert!(points_approx_equal(extended_point, expected_point, EPSILON));
    }

    // --- Geodesic Tests ---
    // These require known-good values. Using a spherical model approximation for simplicity,
    // but ideally, these should be verified with a WGS84 calculator or another library.
    // Earth radius ~ 6371 km. 1 degree latitude ~ 111.32 km. 1 degree longitude ~ cos(lat) * 111.32 km.

    // Example: Extend North from equator (0,0) by 111.32 km (approx 1 degree lat)
    // Assuming point_a and point_b are lon/lat.
    #[test]
    fn test_extend_straight_line_geodesic_north_pole() {
        // Extend North from a point towards the North pole
        let point_a = Point::new(0.0, 89.0); // Lon 0, Lat 89 (near North pole)
        let point_b = Point::new(0.0, 89.5); // Lon 0, Lat 89.5 (closer to pole)
        let extension = 10000.0; // 10 km extension towards the pole
        let is_geodesic = true;

        let extended_point = extend_straight_line(point_a, point_b, extension, is_geodesic);

        // Calculate expected point using geo's Geodesic destination function
        let bearing_ab = Geodesic.bearing(point_a, point_b); // Bearing from A to B (should be 0 degrees North)
        let expected_point = Geodesic.destination(point_b, bearing_ab, extension); // Destination from B along that bearing

        // Use a reasonable epsilon for geodesic tests (1e-5 degrees ~ 1m at equator, smaller error near poles)
        let epsilon = 1e-5;
        assert!(points_approx_equal(extended_point, expected_point, epsilon));
    }

    // Example: Extend East from equator (0,0) by ~111.32 km (approx 1 degree lon at equator)
    #[test]
    fn test_extend_straight_line_geodesic_equator_east() {
        // Extend East along the equator
        let point_a = Point::new(0.0, 0.0); // Lon 0, Lat 0
        let point_b = Point::new(0.1, 0.0); // Lon 0.1, Lat 0.0
        let extension = 100_000.0; // 100 km extension
        let is_geodesic = true;

        let extended_point = extend_straight_line(point_a, point_b, extension, is_geodesic);

        // Calculate expected point using geo's Geodesic destination function
        let bearing_ab = Geodesic.bearing(point_a, point_b); // Bearing from A to B (should be 90 degrees East)
        let expected_point = Geodesic.destination(point_b, bearing_ab, extension); // Destination from B along that bearing

        // Use a reasonable epsilon for geodesic tests
        let epsilon = 1e-5;
        assert!(points_approx_equal(extended_point, expected_point, epsilon));
    }

    #[test]
    fn test_extend_straight_line_geodesic_diagonal_real_world() {
        // Extend from a point in a specific direction using real-world coordinates
        let point_a = Point::new(2.2944, 48.8583); // Lon, Lat (approx SW of B)
        let point_b = Point::new(2.2945, 48.8584); // Lon, Lat (Eiffel Tower area)
        let extension = 1000.0; // 1 km in meters
        let is_geodesic = true;

        let extended_point = extend_straight_line(point_a, point_b, extension, is_geodesic);

        // Calculate expected point using geo's Geodesic destination function
        let bearing_ab = Geodesic.bearing(point_a, point_b); // Bearing from A to B (approx 45 degrees)
        let expected_point = Geodesic.destination(point_b, bearing_ab, extension); // Destination from B along that bearing

        // Use a reasonable epsilon for geodesic tests
        let epsilon = 1e-5;
        assert!(points_approx_equal(extended_point, expected_point, epsilon));
    }
    // Add more tests for different quadrants, larger distances, points near poles/anti-meridian etc.
    // depending on how comprehensive you need the suite to be.
}
