use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use crate::utils::error::Error;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureCollection {
    #[serde(rename = "type")]
    pub type_field: String,
    pub features: Vec<Feature>,
}

impl FeatureCollection {
    pub fn from_geojson(geojson: geojson::FeatureCollection) -> Self {
        FeatureCollection {
            type_field: "FeatureCollection".to_string(),
            features: geojson.features.into_iter().map(Feature::from_geojson).collect(),
        }
    }
}

impl Feature {
    pub fn from_geojson(feature: geojson::Feature) -> Self {
        Feature {
            type_field: "Feature".to_string(),
            properties: feature.properties,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Feature {
    #[serde(rename = "type")]
    pub type_field: String,
    pub properties: Option<Map<String, Value>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Geometry {
    #[serde(rename = "type")]
    pub type_field: String,
    pub coordinates: Option<Vec<f64>>,
}

impl Geometry {
    pub fn from_geojson(geometry: Vec<f64>) -> Self {
        Geometry {
            type_field: "Geometry".to_string(),
            coordinates: Some(geometry),
        }
    }
}


#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Properties {
    pub properties: Map<String, Value>,
}

fn process_feature(feature: Feature) -> Feature {
    match feature.type_field.as_str() {
        "Point" => feature,
        "LineString" => feature,
        "Polygon" => feature,
        "MultiPolygon" => feature,
        _ => feature,
    }
}

#[allow(dead_code)]
pub fn process_raw_serde(geojson: geojson::FeatureCollection) -> Result<Vec<Feature>, Error> {
    let collection = FeatureCollection::from_geojson(geojson);

    let processed_features: Vec<Feature> = collection
        .features
        .into_iter()
        .map(process_feature)
        .collect();

    Ok(processed_features)
}