use geo::{Geometry as GeoRustGeometry, LineString, Point, Polygon};
use geojson::{Feature, GeoJson, Geometry};
use serde_json::{Map, Value, from_str};
use thiserror::Error;

// Define error type
#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid geometry type")]
    UnsupportedGeometryType,
    #[error("Missing geometry")]
    MissingGeometry,
    #[error("Invalid coordinates")]
    InvalidCoordinates,
    #[error("Invalid feature")]
    InvalidFeature,
    #[error("Invalid feature collection")]
    InvalidFeatureCollection,
    #[error("Invalid feature properties")]
    InvalidFeatureProperties,
    #[error("Invalid feature geometry")]
    InvalidFeatureGeometry,
}

// --- Example Domain Structs ---
#[derive(Debug)]
pub struct CapturedMarker {
    pub id: String,                                    // From feature.id
    pub geometry: Point,                               // Assuming Markers are always Points
    pub object_id_name: String,                        // From inner "objectId"
    pub original_inner_properties: Map<String, Value>, // Store original inner map
}

#[derive(Debug)]
pub struct Building {
    pub id: String,
    pub geometry: Polygon, // Assuming Buildings are Polygons (if they appear in your data)
    pub original_inner_properties: Map<String, Value>,
}
#[derive(Debug)]
pub struct Street {
    pub id: String,
    pub geometry: LineString, // Assuming Streets are LineStrings (if they appear)
    pub original_inner_properties: Map<String, Value>,
}

// --- Domain Entity Enum ---
#[derive(Debug)]
pub enum DomainEntity {
    Marker(CapturedMarker),
    Building(Building),
    Street(Street),
    Unknown(Feature), // For features that couldn't be identified
}

impl DomainEntity {
    pub fn is_marker(&self) -> bool {
        matches!(self, DomainEntity::Marker(_))
    }
    pub fn is_building(&self) -> bool {
        matches!(self, DomainEntity::Building(_))
    }
    pub fn is_street(&self) -> bool {
        matches!(self, DomainEntity::Street(_))
    }
    pub fn is_unknown(&self) -> bool {
        matches!(self, DomainEntity::Unknown(_))
    }
}

#[allow(dead_code)]
fn indentify_domain_entities(geojson: GeoJson) -> Result<Vec<DomainEntity>, Error> {
    let feature_collection = match geojson {
        GeoJson::FeatureCollection(feature_collection) => feature_collection,
        _ => return Err(Error::InvalidFeatureCollection),
    };

    let mut domain_entities: Vec<DomainEntity> = Vec::new();

    for feature in feature_collection.features {
        let feature_id = match feature.id.clone() {
            Some(id) => match id {
                geojson::feature::Id::String(id) => id,
                geojson::feature::Id::Number(id) => id.to_string(),
            },
            None => "No ID".to_string(),
        };

        let outer_properties = match &feature.properties {
            Some(properties) => properties,
            None => {
                eprintln!("Skipping feature {:?} with no properties", feature_id);
                domain_entities.push(DomainEntity::Unknown(feature));
                continue;
            }
        };

        let inner_properties: Map<String, Value> = match &outer_properties.get("properties") {
            Some(Value::String(s)) => match from_str(s) {
                Ok(properties) => properties,
                Err(e) => {
                    eprintln!(
                        "Skipping feature {:?} because inner properties string failed to parse: {}",
                        feature_id, e
                    );
                    domain_entities.push(DomainEntity::Unknown(feature));
                    continue;
                }
            },
            Some(Value::Object(properties)) => properties.clone(),
            _ => {
                eprintln!(
                    "Skipping feature {:?} because outer properties is not an object or string",
                    feature_id
                );
                domain_entities.push(DomainEntity::Unknown(feature));
                continue;
            }
        };

        let identified_domain_entity = match inner_properties.get("objectId") {
            Some(Value::String(object_id_value)) => match object_id_value.as_str() {
                "Kugelmarker" => {
                    let point_geometry = if let Some(geometry) = &feature.geometry {
                        match GeoRustGeometry::try_from(geometry) {
                            Ok(GeoRustGeometry::Point(point)) => point,
                            Ok(other_geometry) => {
                                eprintln!(
                                    "Expected Point geometry for Marker type {}, found {:?}",
                                    object_id_value, other_geometry
                                );
                                domain_entities.push(DomainEntity::Unknown(feature));
                                continue;
                            }
                            Err(e) => {
                                eprintln!(
                                    "Expected Point geometry for Marker type {}, failed to parse: {}",
                                    object_id_value, e
                                );
                                domain_entities.push(DomainEntity::Unknown(feature));
                                continue;
                            }
                        }
                    } else {
                        eprintln!(
                            "Skipping feature {:?} because geometry is missing",
                            feature_id
                        );
                        domain_entities.push(DomainEntity::Unknown(feature));
                        continue;
                    };
                    DomainEntity::Marker(CapturedMarker {
                        id: feature_id,
                        geometry: point_geometry,
                        object_id_name: object_id_value.clone(),
                        original_inner_properties: inner_properties.clone(),
                    })
                }
                _ => {
                    eprintln!(
                        "Skipping feature {:?} because objectId is not a string or missing",
                        feature_id
                    );
                    domain_entities.push(DomainEntity::Unknown(feature));
                    continue;
                }
            },
            _ => {
                eprintln!(
                    "Skipping feature {:?} because outer properties['properties'] is not a string or missing",
                    feature_id
                );
                domain_entities.push(DomainEntity::Unknown(feature));
                continue;
            }
        };
        domain_entities.push(identified_domain_entity);
    }

    Ok(domain_entities)
}

#[allow(dead_code)]
fn convert_domain_entity_to_geojson_feature(domain_entity: DomainEntity) -> Feature {
    match domain_entity {
        DomainEntity::Marker(marker) => {
            let geo = GeoRustGeometry::from(marker.geometry);
            let geometry = Geometry::from(&geo);
            let mut properties = Map::new();
            properties.insert(
                "original_inner_props".to_string(),
                Value::Object(marker.original_inner_properties),
            );
            Feature {
                geometry: Some(geometry),
                properties: Some(properties),
                bbox: None,
                id: Some(geojson::feature::Id::String(marker.id)),
                foreign_members: None,
            }
        }
        DomainEntity::Unknown(feature) => {
            // Pass through unknown features untouched
            feature
        }
        _ => unimplemented!(),
    }
}

#[allow(dead_code)]
fn convert_domain_entities_to_geojson_features(domain_entities: Vec<DomainEntity>) -> GeoJson {
    let features = domain_entities
        .into_iter()
        .map(convert_domain_entity_to_geojson_feature)
        .collect();
    GeoJson::FeatureCollection(features)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identify_domain_entities() {
        let geojson = serde_json::from_str(
            r#"
            {
                "type": "FeatureCollection",
                "features": [
                    {
                        "id": "1",
                        "type": "Feature",
                        "properties": {
                            "properties": {
                                "objectId": "Kugelmarker"
                            }
                        },
                        "geometry": {
                            "type": "Point",
                            "coordinates": [0.0, 0.0]
                        }
                    }
                ]
            }
            "#,
        )
        .unwrap();

        let domain_entities = indentify_domain_entities(geojson).unwrap();
        assert_eq!(domain_entities.len(), 1);
        assert!(domain_entities[0].is_marker());
    }
    #[test]
    fn test_identify_domain_entities_with_null_properties() {
        let geojson = serde_json::from_str(
            r#"
            {
                "type": "FeatureCollection",
                "features": [
                    {
                        "id": "1",
                        "type": "Feature",
                        "properties": null,
                        "geometry": {
                            "type": "Point",
                            "coordinates": [0.0, 0.0]
                        }
                    }
                ]
            }
            "#,
        )
        .unwrap();

        let domain_entities = indentify_domain_entities(geojson).unwrap();
        assert_eq!(domain_entities.len(), 1);
        assert!(domain_entities[0].is_unknown());
    }
    #[test]
    fn test_identify_domain_entities_with_null_geometry() {
        let geojson = serde_json::from_str(
            r#"
            {
                "type": "FeatureCollection",
                "features": [
                    {
                        "id": "1",
                        "type": "Feature",
                        "properties": {
                            "properties": {
                                "objectId": "Kugelmarker"
                            }
                        },
                        "geometry": null
                    }
                ]
            }
            "#,
        )
        .unwrap();

        let domain_entities = indentify_domain_entities(geojson).unwrap();
        assert_eq!(domain_entities.len(), 1);
        assert!(domain_entities[0].is_unknown());
    }
    #[test]
    fn test_identify_domain_entities_with_invalid_geometry_type() {
        let geojson = serde_json::from_str(
            r#"
            {
                "type": "FeatureCollection",
                "features": [
                    {
                        "id": "1",
                        "type": "Feature",
                        "properties": {
                            "properties": {
                                "objectId": "Kugelmarker"
                            }
                        },
                        "geometry": {
                            "type": "LineString",
                            "coordinates": [
                                [0.0, 0.0],
                                [1.0, 1.0]
                            ]
                        }
                    }
                ]
            }
            "#,
        )
        .unwrap();

        let domain_entities = indentify_domain_entities(geojson).unwrap();
        assert_eq!(domain_entities.len(), 1);
        assert!(domain_entities[0].is_unknown());
    }
    #[test]
    fn test_identify_domain_untities_with_empty_feature_collection() {
        let geojson = serde_json::from_str(
            r#"
            {
                "type": "FeatureCollection",
                "features": []
            }
            "#,
        )
        .unwrap();

        let domain_entities = indentify_domain_entities(geojson).unwrap();
        assert_eq!(domain_entities.len(), 0);
    }
}
