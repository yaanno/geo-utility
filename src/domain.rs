use geo::{Geometry as GeoRustGeometry, Point};
use geojson::{Feature, FeatureCollection, GeoJson, Geometry, feature::Id};
use serde_json::{Map, Value, from_str};
use std::error::Error as StdError;
use thiserror::Error;

// Define error type
#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid geometry type")]
    UnsupportedGeometryType,
    #[error("Missing geometry")]
    MissingGeometry,
    #[error("Invalid coordinates")]
    InvalidCoordinates, // Not used currently
    #[error("Invalid feature")]
    InvalidFeature, // Not used currently
    #[error("Invalid feature collection")]
    InvalidFeatureCollection,
    #[error("Invalid feature properties")]
    InvalidFeatureProperties, // Not used currently
    #[error("Invalid feature geometry")]
    InvalidFeatureGeometry, // Not used currently
    #[error("Invalid objectId: {0}")] // Include the bad string in the error message
    InvalidObjectId(String),
    #[error("Error converting geometry: {0}")] // More specific error for geo::Error
    GeometryConversionError(#[from] Box<dyn StdError>),
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

impl Into<Feature> for &CableTunnel {
    fn into(self) -> Feature {
        let geometry = Geometry::from(&self.geometry).clone();
        let properties = self.original_inner_properties.clone();
        Feature {
            geometry: Some(geometry),
            properties: Some(properties),
            bbox: None,
            id: Some(Id::String(self.id.clone())),
            foreign_members: None,
        }
    }
}

impl Into<Feature> for &DrillingPoint {
    fn into(self) -> Feature {
        let geometry = Geometry::from(&self.geometry).clone();
        let properties = self.original_inner_properties.clone();
        Feature {
            geometry: Some(geometry),
            properties: Some(properties),
            bbox: None,
            id: Some(Id::String(self.id.clone())),
            foreign_members: None,
        }
    }
}

impl Into<Feature> for &OperationSite {
    fn into(self) -> Feature {
        let geometry = Geometry::from(&self.geometry).clone();
        let properties = self.original_inner_properties.clone();
        Feature {
            geometry: Some(geometry),
            properties: Some(properties),
            bbox: None,
            id: Some(Id::String(self.id.clone())),
            foreign_members: None,
        }
    }
}

impl Into<Feature> for &SupplyPoint {
    fn into(self) -> Feature {
        let geometry = Geometry::from(&self.geometry).clone();
        let properties = self.original_inner_properties.clone();
        Feature {
            geometry: Some(geometry),
            properties: Some(properties),
            bbox: None,
            id: Some(Id::String(self.id.clone())),
            foreign_members: None,
        }
    }
}

impl Into<Feature> for &CapturedMarker {
    fn into(self) -> Feature {
        let geometry = Geometry::from(&self.geometry).clone();
        let properties = self.original_inner_properties.clone();
        Feature {
            geometry: Some(geometry),
            properties: Some(properties),
            bbox: None,
            id: Some(Id::String(self.id.clone())),
            foreign_members: None,
        }
    }
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
    pub fn id(&self) -> Option<&String> {
        match self {
            DomainEntity::CapturedMarker(marker) => Some(&marker.id),
            DomainEntity::SupplyPoint(point) => Some(&point.id),
            DomainEntity::OperationSite(site) => Some(&site.id),
            DomainEntity::DrillingPoint(point) => Some(&point.id),
            DomainEntity::CableTunnel(tunnel) => Some(&tunnel.id),
            DomainEntity::Unknown(feature) => feature.id.as_ref().and_then(|id| match id {
                Id::String(s) => Some(s),
                Id::Number(_) => None, // Or convert number ID to string reference if possible
            }), // Handle geojson::feature::Id
        }
    }
}

impl Into<Feature> for &DomainEntity {
    fn into(self) -> Feature {
        match self {
            DomainEntity::CapturedMarker(marker) => marker.into(),
            DomainEntity::SupplyPoint(point) => point.into(),
            DomainEntity::OperationSite(site) => site.into(),
            DomainEntity::DrillingPoint(point) => point.into(),
            DomainEntity::CableTunnel(tunnel) => tunnel.into(),
            DomainEntity::Unknown(feature) => feature.clone(),
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
            DomainEntity::Unknown(_) => GeoRustGeometry::Point(Point::new(0.0, 0.0)),
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

// Helper function to create the specific DomainEntity variant
// for a Point type, given the common data and the identified ObjectId.
fn create_point_domain_entity(
    id: String,
    geometry: Point,
    original_inner_properties: Map<String, Value>,
    object_id: ObjectId, // Takes the parsed enum variant
                         // Add other common fields needed for specific structs here if they weren't in properties
) -> DomainEntity {
    match object_id {
        ObjectId::Kugelmarker => DomainEntity::CapturedMarker(CapturedMarker {
            id,                        // Move id
            geometry,                  // Move geometry
            original_inner_properties, // Move properties
        }),
        ObjectId::Versorgungspunkt => DomainEntity::SupplyPoint(SupplyPoint {
            id,
            geometry,
            original_inner_properties,
        }),
        ObjectId::Betriebsstelle => DomainEntity::OperationSite(OperationSite {
            id,
            geometry,
            original_inner_properties,
        }),
        ObjectId::Bohrpunkt => DomainEntity::DrillingPoint(DrillingPoint {
            id,
            geometry,
            original_inner_properties,
        }),
        ObjectId::Kabelschacht => DomainEntity::CableTunnel(CableTunnel {
            id,
            geometry,
            original_inner_properties,
        }),
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
            match ObjectId::try_from(object_id_value.clone()) {
                Ok(object_id_enum) => {
                    // --- Common logic for ALL known Point types ---
                    let point_geometry = match get_point_geometry(
                        &feature, // Pass by reference
                        &original_feature,
                        &object_id_value, // Still need the string for logging context in helper
                    ) {
                        Ok(value) => value,
                        Err(unknown_entity) => return unknown_entity, // Return Unknown if geometry extraction fails
                    };

                    // --- Use the factory function to create the specific entity ---
                    create_point_domain_entity(
                        feature_id,       // Move id
                        point_geometry,   // Move geometry
                        inner_properties, // Move properties
                        object_id_enum,   // Pass the enum variant
                    )
                }
                Err(Error::InvalidObjectId(unrecognized_id_string)) => {
                    // This branch handles valid strings that are NOT known ObjectIds
                    eprintln!(
                        "Unrecognized objectId string: {} for feature {}",
                        unrecognized_id_string, feature_id
                    );
                    DomainEntity::Unknown(original_feature)
                }
                Err(e) => {
                    eprintln!(
                        "Failed to convert objectId for feature {}: {}",
                        feature_id, e
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
    feature: &Feature,          // Take by reference
    original_feature: &Feature, // Take by reference
    object_id_value: &str,      // Use &str, no need for &String
) -> Result<Point, DomainEntity> {
    let cloned_feature = original_feature.clone(); // Clone only when needed for Unknown

    let geometry_option = feature.geometry.clone(); // Clone geometry to consume

    let feature_id = match feature.id.clone() {
        Some(id) => match id {
            geojson::feature::Id::String(id) => id,
            geojson::feature::Id::Number(id) => id.to_string(),
        },
        None => "No ID".to_string(),
    };

    let geometry = match &geometry_option {
        Some(geom) => geom,
        None => {
            eprintln!(
                "Geometry is missing for objectId '{}' (feature {})",
                object_id_value, feature_id
            );
            return Err(DomainEntity::Unknown(cloned_feature));
        }
    };

    match geo::Point::try_from(geometry) {
        Ok(point) => Ok(point),
        Err(e) => {
            let geojson_type = geometry.value.type_name();
            // Use the new specific error variant
            let conversion_error = Error::GeometryConversionError(Box::new(e));
            eprintln!(
                "Failed to convert geojson::{} to geo::Point for objectId '{}' (feature {}): {}",
                geojson_type,
                object_id_value,
                feature_id,
                conversion_error // Use conversion_error here
            );
            Err(DomainEntity::Unknown(cloned_feature))
        }
    }
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

#[allow(dead_code)]
fn convert_domain_entity_to_geojson_feature(domain_entity: DomainEntity) -> Feature {
    match &domain_entity {
        // Match all Point variants and bind the enum value to `point_entity`
        entity_ref @ DomainEntity::CapturedMarker(_) |
         entity_ref @ DomainEntity::SupplyPoint(_) |
         entity_ref @ DomainEntity::OperationSite(_) |
         entity_ref @ DomainEntity::DrillingPoint(_) |
         entity_ref @ DomainEntity::CableTunnel(_) |
         entity_ref @ DomainEntity::Unknown(_) // Also include Unknown here
         => {
             entity_ref.into() // Calls the Into<Feature> for &DomainEntity impl
         }
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
