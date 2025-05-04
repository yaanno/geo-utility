use geojson::{Feature, FeatureCollection, Geometry, Value};
use serde_json::Map;

#[allow(dead_code)]
pub fn generate_deterministic_feature_collection(num_features: usize) -> FeatureCollection {
    let mut features: Vec<Feature> = Vec::new();

    for _ in 0..num_features {
        let mut point = Feature::from(Geometry::new(Value::Point(vec![10.0, 10.0])));
        let mut props = Map::new();
        let mut inner_props = Map::new();   
        inner_props.insert("objectId".to_string(), serde_json::Value::String("Kugelmarker".to_string()));
        props.insert("properties".to_string(), serde_json::Value::Object(inner_props));
        point.properties = Some(props);
        features.push(point);

        let mut line = Feature::from(Geometry::new(Value::LineString(vec![vec![10.0, 10.0], vec![20.0, 20.0]])));
        let mut props = Map::new();
        let mut inner_props = Map::new();   
        inner_props.insert("objectId".to_string(), serde_json::Value::String("Linie".to_string()));
        props.insert("properties".to_string(), serde_json::Value::Object(inner_props));
        line.properties = Some(props);
        features.push(line);

        let mut polygon = Feature::from(Geometry::new(Value::LineString(vec![vec![10.0, 10.0], vec![20.0, 20.0], vec![30.0, 30.0], vec![10.0, 10.0]])));
        let mut props = Map::new();
        let mut inner_props = Map::new();   
        inner_props.insert("objectId".to_string(), serde_json::Value::String("Gebaeude".to_string()));
        props.insert("properties".to_string(), serde_json::Value::Object(inner_props));
        polygon.properties = Some(props);
        features.push(polygon);

        let mut polygon = Feature::from(Geometry::new(Value::Polygon(vec![vec![vec![10.0, 10.0], vec![20.0, 20.0], vec![30.0, 30.0], vec![10.0, 10.0]]])));
        let mut props = Map::new();
        let mut inner_props = Map::new();   
        inner_props.insert("objectId".to_string(), serde_json::Value::String("Gebaeude".to_string()));
        props.insert("properties".to_string(), serde_json::Value::Object(inner_props));
        polygon.properties = Some(props);
        features.push(polygon);
    }

    FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    }
}
