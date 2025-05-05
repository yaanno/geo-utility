use geojson::{Feature, FeatureCollection, Geometry, Value};

#[allow(dead_code)]
pub fn generate_deterministic_feature_collection(num_features: usize) -> FeatureCollection {
    let mut features: Vec<Feature> = Vec::new();

    for _ in 0..num_features {
        let point = Feature::from(Geometry::new(Value::Point(vec![10.0, 10.0])));
        features.push(point);

        let line = Feature::from(Geometry::new(Value::LineString(vec![vec![10.0, 10.0], vec![20.0, 20.0]])));
        features.push(line);

        let polygon = Feature::from(Geometry::new(Value::LineString(vec![vec![10.0, 10.0], vec![20.0, 20.0], vec![30.0, 30.0], vec![10.0, 10.0]])));
        features.push(polygon);

        let polygon = Feature::from(Geometry::new(Value::Polygon(vec![vec![vec![10.0, 10.0], vec![20.0, 20.0], vec![30.0, 30.0], vec![10.0, 10.0]]])));
        features.push(polygon);

        let polygon = Feature::from(Geometry::new(Value::Polygon(vec![vec![vec![10.0, 10.0], vec![20.0, 20.0], vec![30.0, 30.0], vec![10.0, 10.0]]])));
        features.push(polygon);
    }

    FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    }
}
