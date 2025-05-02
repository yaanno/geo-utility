use geo::{Geometry as GeoRustGeometry, Point};
use geojson::{Feature, FeatureCollection, GeoJson, Geometry};
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
    #[error("Invalid objectId")]
    InvalidObjectId(String),
}

// --- Example Domain Structs ---
#[derive(Debug, Clone)]
pub struct CapturedMarker {
    pub id: String,                                    // From feature.id
    pub geometry: Point,                               // Assuming Markers are always Points
    pub original_inner_properties: Map<String, Value>, // Store original inner map
}

#[derive(Debug, Clone)]
pub struct SupplyPoint {
    pub id: String,
    pub geometry: Point, // Assuming Versorgungspunkte are Points (if they appear in your data)
    pub original_inner_properties: Map<String, Value>,
}

#[derive(Debug, Clone)]
pub struct OperationSite {
    pub id: String,
    pub geometry: Point, // Assuming Betriebsstellen are Points (if they appear in your data)
    pub original_inner_properties: Map<String, Value>,
}

#[derive(Debug, Clone)]
pub struct DrillingPoint {
    pub id: String,
    pub geometry: Point, // Assuming Bohrpunkte are Points (if they appear in your data)
    pub original_inner_properties: Map<String, Value>,
}

#[derive(Debug, Clone)]
pub struct CableTunnel {
    pub id: String,
    pub geometry: Point, // Assuming Kabelschachte are Points (if they appear in your data)
    pub original_inner_properties: Map<String, Value>,
}


// --- Domain Entity Enum ---
#[derive(Debug, Clone)]
pub enum DomainEntity {
    CapturedMarker(CapturedMarker),
    SupplyPoint(SupplyPoint),
    OperationSite(OperationSite),
    DrillingPoint(DrillingPoint),
    CableTunnel(CableTunnel),
    Unknown(Feature),
}

impl DomainEntity {
    pub fn id(&self) -> &String {
        match self {
            DomainEntity::CapturedMarker(marker) => &marker.id,
            DomainEntity::SupplyPoint(point) => &point.id,
            DomainEntity::OperationSite(site) => &site.id,
            DomainEntity::DrillingPoint(point) => &point.id,
            DomainEntity::CableTunnel(tunnel) => &tunnel.id,
            _ => unimplemented!(),
        }
    }
    pub fn geometry(&self) -> GeoRustGeometry {
        match self {
            DomainEntity::CapturedMarker(marker) => GeoRustGeometry::Point(marker.geometry),
            DomainEntity::SupplyPoint(point) => GeoRustGeometry::Point(point.geometry),
            DomainEntity::OperationSite(site) => GeoRustGeometry::Point(site.geometry),
            DomainEntity::DrillingPoint(point) => GeoRustGeometry::Point(point.geometry),
            DomainEntity::CableTunnel(tunnel) => GeoRustGeometry::Point(tunnel.geometry),
            _ => unimplemented!(),
        }
    }
    pub fn original_inner_properties(&self) -> &Map<String, Value> {
        match self {
            DomainEntity::CapturedMarker(marker) => &marker.original_inner_properties,
            DomainEntity::SupplyPoint(point) => &point.original_inner_properties,
            DomainEntity::OperationSite(site) => &site.original_inner_properties,
            DomainEntity::DrillingPoint(point) => &point.original_inner_properties,
            DomainEntity::CableTunnel(tunnel) => &tunnel.original_inner_properties,
            _ => unimplemented!(),
        }
    }
}

impl From<DomainEntity> for GeoRustGeometry {
    fn from(value: DomainEntity) -> Self {
        match value {
            DomainEntity::CapturedMarker(marker) => GeoRustGeometry::Point(marker.geometry),
            DomainEntity::SupplyPoint(point) => GeoRustGeometry::Point(point.geometry),
            DomainEntity::OperationSite(site) => GeoRustGeometry::Point(site.geometry),
            DomainEntity::DrillingPoint(point) => GeoRustGeometry::Point(point.geometry),
            DomainEntity::CableTunnel(tunnel) => GeoRustGeometry::Point(tunnel.geometry),
            _ => unimplemented!(),
        }
    }
}

impl DomainEntity {
    pub fn is_marker(&self) -> bool {
        matches!(self, DomainEntity::CapturedMarker(_))
    }
    pub fn is_supply_point(&self) -> bool {
        matches!(self, DomainEntity::SupplyPoint(_))
    }
    pub fn is_operation_site(&self) -> bool {
        matches!(self, DomainEntity::OperationSite(_))
    }
    pub fn is_drilling_point(&self) -> bool {
        matches!(self, DomainEntity::DrillingPoint(_))
    }
    pub fn is_cable_tunnel(&self) -> bool {
        matches!(self, DomainEntity::CableTunnel(_))
    }
    pub fn is_unknown(&self) -> bool {
        matches!(self, DomainEntity::Unknown(_))
    }
}

pub enum ObjectId {
    Kugelmarker,
    Versorgungspunkt,
    Betriebsstelle,
    Bohrpunkt,
    Kabelschacht,
}

impl TryFrom<String> for ObjectId {
    type Error = Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "Kugelmarker" => Ok(ObjectId::Kugelmarker),
            "Versorgungspunkt" => Ok(ObjectId::Versorgungspunkt),
            "Betriebsstelle" => Ok(ObjectId::Betriebsstelle),
            "Bohrpunkt" => Ok(ObjectId::Bohrpunkt),
            "Kabelschacht" => Ok(ObjectId::Kabelschacht),
            _ => Err(Error::InvalidObjectId(value)),
        }
    }
}

fn indentify_domain_entity(feature: Feature) -> DomainEntity {
    let feature_id = match feature.id.clone() {
        Some(id) => match id {
            geojson::feature::Id::String(id) => id,
            geojson::feature::Id::Number(id) => id.to_string(),
        },
        None => "No ID".to_string(),
    };

    let original_feature = feature.clone();

    let outer_properties = match &feature.properties {
        Some(properties) => properties,
        None => {
            eprintln!("Skipping feature {:?} with no properties", feature_id);
            return DomainEntity::Unknown(original_feature);
        }
    };

    let inner_properties: Map<String, Value> = match &outer_properties.get("properties") {
        Some(Value::String(s)) => match from_str(s) {
            Ok(properties) => properties,
            Err(e) => {
                eprintln!(
                    "Inner properties string failed to parse for feature {}: {}",
                    feature_id, e
                );
                return DomainEntity::Unknown(original_feature);
            }
        },
        Some(Value::Object(properties)) => properties.clone(),
        _ => {
            eprintln!(
                "Outer properties['properties'] is missing or not string/object for feature {}",
                feature_id
            );
            return DomainEntity::Unknown(original_feature);
        }
    };
    let identified_domain_entity = match inner_properties.get("objectId") {
        Some(Value::String(object_id_value)) => {
            match object_id_value.as_str() {
                "Kugelmarker" | "Versorgungspunkt" | "Betriebsstelle" | "Bohrpunkt"
                | "Kabelschacht" => {
                    // --- Common logic for ALL Point types ---
                    let point_geometry = match get_point_geometry(
                        &feature, // Pass by reference now!
                        &original_feature,
                        object_id_value,
                    ) {
                        Ok(value) => value,
                        Err(value) => return value, // Return Unknown if geometry extraction fails
                    };

                    // --- Specific struct creation based on objectId value ---
                    match ObjectId::try_from(object_id_value.clone()) {
                        Ok(ObjectId::Kugelmarker) => DomainEntity::CapturedMarker(CapturedMarker {
                            id: feature_id,
                            geometry: point_geometry,
                            original_inner_properties: inner_properties, /* other fields */
                        }),
                        Ok(ObjectId::Versorgungspunkt) => DomainEntity::SupplyPoint(SupplyPoint {
                            id: feature_id,
                            geometry: point_geometry,
                            original_inner_properties: inner_properties, /* other fields */
                        }),
                        Ok(ObjectId::Betriebsstelle) => {
                            DomainEntity::OperationSite(OperationSite {
                                id: feature_id,
                                geometry: point_geometry,
                                original_inner_properties: inner_properties, /* other fields */
                            })
                        }
                        Ok(ObjectId::Bohrpunkt) => DomainEntity::DrillingPoint(DrillingPoint {
                            id: feature_id,
                            geometry: point_geometry,
                            original_inner_properties: inner_properties, /* other fields */
                        }),
                        Ok(ObjectId::Kabelschacht) => DomainEntity::CableTunnel(CableTunnel {
                            id: feature_id,
                            geometry: point_geometry,
                            original_inner_properties: inner_properties, /* other fields */
                        }),
                        _ => {
                            eprintln!(
                                "Internal logic error: Unhandled point objectId: {}",
                                object_id_value
                            );
                            DomainEntity::Unknown(original_feature)
                        }
                    }
                }
                unrecognized_id => {
                    eprintln!(
                        "Unrecognized objectId string: {} for feature {}",
                        unrecognized_id, feature_id
                    );
                    DomainEntity::Unknown(original_feature)
                }
            }
        }
        // --- Fallback for missing or invalid objectId type ---
        _ => {
            eprintln!(
                "Missing or invalid 'objectId' in inner properties for feature {}",
                feature_id
            );
            DomainEntity::Unknown(original_feature)
        }
    };

    // Return the identified entity
    identified_domain_entity
}

fn get_point_geometry(
    feature: &Feature,
    original_feature: &Feature,
    object_id_value: &String,
) -> Result<Point, DomainEntity> {
    let cloned_feature = original_feature.clone();
    let point_geometry = match feature.geometry.clone() {
        Some(geometry) => match GeoRustGeometry::try_from(geometry) {
            Ok(GeoRustGeometry::Point(point)) => point,
            Ok(other_geometry) => {
                eprintln!(
                    "Expected Point geometry for objectId '{}', found {:?}",
                    object_id_value, other_geometry
                );
                return Err(DomainEntity::Unknown(cloned_feature));
            }
            Err(e) => {
                eprintln!(
                    "Failed to convert geometry for objectId '{}': {}",
                    object_id_value, e
                );
                return Err(DomainEntity::Unknown(cloned_feature));
            }
        },
        None => {
            eprintln!("Geometry is missing for objectId '{}'", object_id_value);
            return Err(DomainEntity::Unknown(cloned_feature));
        }
    };
    Ok(point_geometry)
}

#[allow(dead_code)]
fn indentify_domain_entities(geojson: GeoJson) -> Result<Vec<DomainEntity>, Error> {
    let feature_collection = match geojson {
        GeoJson::FeatureCollection(feature_collection) => feature_collection,
        _ => return Err(Error::InvalidFeatureCollection),
    };

    // Process each feature using the helper function
    let domain_entities: Vec<DomainEntity> = feature_collection
        .features
        .into_iter()
        .map(indentify_domain_entity) // Apply helper to each feature
        .collect(); // Collect results

    Ok(domain_entities)
}

fn convert_point_entity_to_geojson_feature(point_entity: &DomainEntity) -> Feature {
    let geo = point_entity.geometry();
    let geometry = Geometry::from(&geo);
    let mut properties = Map::new();
    let id = point_entity.id();
    let original_inner_properties = point_entity.original_inner_properties().clone();
    properties.insert("properties".to_string(), Value::Object(original_inner_properties));

    // Optionally add specific fields here if needed in the output properties,
    // accessing them via pattern matching or helper methods if they exist on the enum
    if let DomainEntity::CapturedMarker(_marker) = point_entity {
        properties.insert("objectId".to_string(), Value::String("Kugelmarker".to_string()));
    }

    Feature {
        geometry: Some(geometry),
        properties: Some(properties),
        bbox: None,
        id: Some(geojson::feature::Id::String(id.to_string())),
        foreign_members: None,
    }
}

#[allow(dead_code)]
fn convert_domain_entity_to_geojson_feature(domain_entity: DomainEntity) -> Feature {
    match domain_entity {
        // Match all Point variants and bind the enum value to `point_entity`
        point_entity @ DomainEntity::CapturedMarker(_) |
        point_entity @ DomainEntity::SupplyPoint(_) |
        point_entity @ DomainEntity::OperationSite(_) |
        point_entity @ DomainEntity::DrillingPoint(_) |
        point_entity @ DomainEntity::CableTunnel(_) => {
            // `point_entity` is the owned DomainEntity value (e.g., DomainEntity::CapturedMarker(...))
            // Pass a reference to this value to the helper
            convert_point_entity_to_geojson_feature(&point_entity)
        }
        DomainEntity::Unknown(feature) => feature,
    }
}

#[allow(dead_code)]
fn convert_domain_entities_to_geojson_features(domain_entities: Vec<DomainEntity>) -> GeoJson {
    let features = domain_entities
        .into_iter()
        .map(convert_domain_entity_to_geojson_feature)
        .collect::<Vec<Feature>>();
    GeoJson::FeatureCollection(FeatureCollection {
        features,
        bbox: None,
        foreign_members: None,
    })
}

#[cfg(test)]
mod tests {
    use geojson::{FeatureCollection, feature::Id};

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
    #[test]
    fn test_convert_domain_entity_to_geojson_feature() {
        let domain_entity = DomainEntity::CapturedMarker(CapturedMarker {
            id: "1".to_string(),
            geometry: Point::new(0.0, 0.0),
            original_inner_properties: Map::new(),
        });
        let geojson_feature = convert_domain_entity_to_geojson_feature(domain_entity);
        println!("geojson_feature: {:#?}", geojson_feature);
        assert_eq!(geojson_feature.id.unwrap(), Id::String("1".to_string()));
        // assert_eq!(geojson_feature.geometry.unwrap().to_owned().geometry_type(), "Point");
    }
    #[test]
    fn test_convert_domain_entity_to_geojson_feature_with_unknown() {
        let domain_entity = DomainEntity::Unknown(Feature {
            geometry: None,
            properties: None,
            bbox: None,
            id: Some(Id::String("No ID".to_string())),
            foreign_members: None,
        });
        let geojson_feature = convert_domain_entity_to_geojson_feature(domain_entity);
        assert_eq!(geojson_feature.id.unwrap(), Id::String("No ID".to_string()));
        assert_eq!(geojson_feature.geometry.is_none(), true);
        assert_eq!(geojson_feature.properties.is_none(), true);
    }
    #[test]
    fn test_convert_domain_entities_to_geojson_features() {
        let domain_entities = vec![
            DomainEntity::CapturedMarker(CapturedMarker {
                id: "1".to_string(),
                geometry: Point::new(0.0, 0.0),
                original_inner_properties: Map::new(),
            }),
            DomainEntity::Unknown(Feature {
                geometry: None,
                properties: None,
                bbox: None,
                id: Some(Id::String("No ID".to_string())),
                foreign_members: None,
            }),
        ];
        let geojson_features = convert_domain_entities_to_geojson_features(domain_entities);
        println!("geojson_features: {:#?}", geojson_features);
    }
    #[test]
    fn test_convert_domain_entities_to_geojson_features_with_empty_vector() {
        let domain_entities = vec![];
        let geojson_features = convert_domain_entities_to_geojson_features(domain_entities);
        assert_eq!(
            geojson_features,
            GeoJson::FeatureCollection(FeatureCollection {
                features: vec![],
                bbox: None,
                foreign_members: None
            })
        );
    }
    #[test]
    fn test_indentify_domain_entity() {
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
    fn test_indentify_domain_entity_with_supply_point() {
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
                                "objectId": "Versorgungspunkt"
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
        assert!(domain_entities[0].is_supply_point());
    }
    #[test]
    fn test_indentify_domain_entity_with_operation_site() {
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
                                "objectId": "Betriebsstelle"
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
        assert!(domain_entities[0].is_operation_site());
    }
    #[test]
    fn test_indentify_domain_entity_with_drilling_point() {
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
                                "objectId": "Bohrpunkt"
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
        assert!(domain_entities[0].is_drilling_point());
    }
    #[test]
    fn test_indentify_domain_entity_with_cable_tunnel() {
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
                                "objectId": "Kabelschacht"
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
        assert!(domain_entities[0].is_cable_tunnel());
    }
}
