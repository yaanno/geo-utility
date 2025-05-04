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
    InvalidCoordinates,
    #[error("Invalid feature")]
    InvalidFeature,
    #[error("Invalid feature collection")]
    InvalidFeatureCollection,
    #[error("Invalid feature properties")]
    InvalidFeatureProperties,
    #[error("Invalid feature geometry")]
    InvalidFeatureGeometry,
    #[error("Invalid objectId: {0}")]
    InvalidObjectId(String),
    #[error("Error converting geometry: {0}")]
    GeometryConversionError(#[from] Box<dyn StdError>),
}

// --- Macro for Into<Feature> ---
macro_rules! impl_into_feature_for_point_entity {
    ($struct_name:ident) => {
        impl Into<Feature> for &$struct_name {
            /// Helper macro to convert a DomainEntity variant to a GeoJSON feature.
            ///
            /// # Arguments
            ///
            /// * `self` - The DomainEntity variant to convert.
            ///
            /// # Returns
            ///
            /// * `Feature` - The converted GeoJSON feature.
            fn into(self) -> Feature {
                let geometry = Geometry::from(&self.geometry).clone();
                let mut properties = self.original_inner_properties.clone();

                // Example: Add objectId back to output properties if present in original inner
                if let Some(Value::String(obj_id)) = self.original_inner_properties.get("objectId")
                {
                    properties.insert("objectId".to_string(), Value::String(obj_id.clone()));
                }
                // // Example: Add CapturedMarker's specific field if it exists
                //  if let Some(marker) = self.as_captured_marker() { // Requires as_captured_marker helper on DomainEntity
                //      properties.insert("object_id_name".to_string(), Value::String(marker.object_id_name.clone()));
                //  }

                Feature {
                    geometry: Some(geometry),
                    properties: Some(properties),
                    bbox: None,
                    id: Some(Id::String(self.id.clone())),
                    foreign_members: None, // You might need to store and forward these
                }
            }
        }
    };
}

// --- Use the macro for each Point struct ---
impl_into_feature_for_point_entity!(CapturedMarker);
impl_into_feature_for_point_entity!(SupplyPoint);
impl_into_feature_for_point_entity!(OperationSite);
impl_into_feature_for_point_entity!(DrillingPoint);
impl_into_feature_for_point_entity!(CableTunnel);

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

#[derive(Debug, Clone)]
pub struct Building {
    pub id: String,
    pub geometry: GeoRustGeometry, // Assuming Kabelschachte are Points (if they appear in your data)
    pub original_inner_properties: Map<String, Value>,
}

// This impl should NOT use the macro defined for Point entities
impl Into<Feature> for &Building {
    fn into(self) -> Feature {
        // Convert the geo::Geometry to geojson::Geometry
        let geometry = Geometry::from(&self.geometry).clone(); // Convert GeoRustGeometry

        // Clone the original inner properties
        let mut properties = self.original_inner_properties.clone();

        // Add objectId back to output properties if present in original inner (optional)
        if let Some(Value::String(obj_id)) = self.original_inner_properties.get("objectId") {
            properties.insert("objectId".to_string(), Value::String(obj_id.clone()));
        }
        // Add Building-specific fields to output properties if needed

        Feature {
            geometry: Some(geometry),
            properties: Some(properties),
            bbox: None, // Calculate this if needed
            id: Some(Id::String(self.id.clone())),
            foreign_members: None, // Forward foreign members if your structs held them
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
    Building(Building),
    Unknown(Feature),
}

impl DomainEntity {
    /// Helper function to get the ID of the feature.
    ///
    /// # Arguments
    ///
    /// * `self` - The DomainEntity variant to get the ID from.
    ///
    /// # Returns
    ///
    /// * `Option<&String>` - The ID of the feature.
    pub fn id(&self) -> Option<&String> {
        match self {
            DomainEntity::CapturedMarker(marker) => Some(&marker.id),
            DomainEntity::SupplyPoint(point) => Some(&point.id),
            DomainEntity::OperationSite(site) => Some(&site.id),
            DomainEntity::DrillingPoint(point) => Some(&point.id),
            DomainEntity::CableTunnel(tunnel) => Some(&tunnel.id),
            DomainEntity::Building(building) => Some(&building.id),
            DomainEntity::Unknown(feature) => feature.id.as_ref().and_then(|id| match id {
                Id::String(s) => Some(s),
                Id::Number(_) => None, // Or convert number ID to string reference if possible
            }), // Handle geojson::feature::Id
        }
    }
}

impl Into<Feature> for &DomainEntity {
    /// Helper function to convert a DomainEntity variant to a GeoJSON feature.
    ///
    /// # Arguments
    ///
    /// * `domain_entity` - The DomainEntity variant to convert.
    ///
    /// # Returns
    ///
    /// * `Feature` - The converted GeoJSON feature.
    fn into(self) -> Feature {
        match self {
            DomainEntity::CapturedMarker(marker) => marker.into(),
            DomainEntity::SupplyPoint(point) => point.into(),
            DomainEntity::OperationSite(site) => site.into(),
            DomainEntity::DrillingPoint(point) => point.into(),
            DomainEntity::CableTunnel(tunnel) => tunnel.into(),
            DomainEntity::Building(building) => building.into(),
            DomainEntity::Unknown(feature) => feature.clone(),
        }
    }
}

impl From<DomainEntity> for GeoRustGeometry {
    /// Helper function to convert a DomainEntity variant to a GeoRustGeometry variant.
    ///
    /// # Arguments
    ///
    /// * `value` - The DomainEntity variant to convert.
    ///
    /// # Returns
    ///
    /// * `GeoRustGeometry` - The converted GeoRustGeometry variant.
    fn from(value: DomainEntity) -> Self {
        match value {
            DomainEntity::CapturedMarker(marker) => GeoRustGeometry::Point(marker.geometry),
            DomainEntity::SupplyPoint(point) => GeoRustGeometry::Point(point.geometry),
            DomainEntity::OperationSite(site) => GeoRustGeometry::Point(site.geometry),
            DomainEntity::DrillingPoint(point) => GeoRustGeometry::Point(point.geometry),
            DomainEntity::CableTunnel(tunnel) => GeoRustGeometry::Point(tunnel.geometry),
            DomainEntity::Building(building) => building.geometry,
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
    pub fn is_building(&self) -> bool {
        matches!(self, DomainEntity::Building(_))
    }
}

pub enum ObjectId {
    Kugelmarker,
    Versorgungspunkt,
    Betriebsstelle,
    Bohrpunkt,
    Kabelschacht,
    Building,
}

impl TryFrom<String> for ObjectId {
    type Error = Error;
    /// Helper function to convert a string to an ObjectId variant.
    ///
    /// # Arguments
    ///
    /// * `value` - The string to convert.
    ///
    /// # Returns
    ///
    /// * `Result<ObjectId, Error>` - The converted ObjectId variant.
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "Kugelmarker" => Ok(ObjectId::Kugelmarker),
            "Versorgungspunkt" => Ok(ObjectId::Versorgungspunkt),
            "Betriebsstelle" => Ok(ObjectId::Betriebsstelle),
            "Bohrpunkt" => Ok(ObjectId::Bohrpunkt),
            "Kabelschacht" => Ok(ObjectId::Kabelschacht),
            "Gebaeude" => Ok(ObjectId::Building),
            _ => Err(Error::InvalidObjectId(value)),
        }
    }
}

/// Helper function to create the specific DomainEntity variant
/// for a Point type, given the common data and the identified ObjectId.
///
/// # Arguments
///
/// * `id` - The ID of the feature.
/// * `geometry` - The geometry of the feature.
/// * `original_inner_properties` - The original inner properties of the feature.
/// * `object_id` - The identified ObjectId.
///
/// # Returns
///
/// * `DomainEntity` - The created DomainEntity variant.
fn create_point_domain_entity(
    id: String,
    geometry: Point,
    original_inner_properties: Map<String, Value>,
    object_id: ObjectId,
) -> DomainEntity {
    match object_id {
        ObjectId::Kugelmarker => DomainEntity::CapturedMarker(CapturedMarker {
            id,
            geometry,
            original_inner_properties,
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
        _ => unimplemented!(),
    }
}

/// Helper function to identify the specific DomainEntity variant
/// for a Point type, given the feature.
///
/// # Arguments
///
/// * `feature` - The feature to identify.
///
/// # Returns
///
/// * `DomainEntity` - The identified DomainEntity variant.
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
            #[cfg(debug_assertions)]
            eprintln!("Skipping feature {:?} with no properties", feature_id);
            return DomainEntity::Unknown(original_feature);
        }
    };

    let inner_properties: Map<String, Value> = match &outer_properties.get("properties") {
        Some(Value::String(s)) => match from_str(s) {
            Ok(properties) => properties,
            Err(e) => {
                #[cfg(debug_assertions)]
                eprintln!(
                    "Inner properties string failed to parse for feature {}: {}",
                    feature_id, e
                );
                return DomainEntity::Unknown(original_feature);
            }
        },
        Some(Value::Object(properties)) => properties.clone(),
        _ => {
            #[cfg(debug_assertions)]
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
                    match object_id_enum {
                        ObjectId::Kugelmarker
                        | ObjectId::Versorgungspunkt
                        | ObjectId::Betriebsstelle
                        | ObjectId::Bohrpunkt
                        | ObjectId::Kabelschacht => {
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
                        ObjectId::Building => {
                            let building_geometry_option = feature.geometry.clone(); // Clone geometry

                            let building_geometry = match &building_geometry_option {
                                Some(geom) => {
                                    // Try converting the geojson::Geometry into the general geo::Geometry enum
                                    match GeoRustGeometry::try_from(geom) {
                                        Ok(geo_geom) => geo_geom, // Successfully converted to GeoRustGeometry
                                        Err(e) => {
                                            let geojson_type = geom.value.type_name();
                                            let conversion_error =
                                                Error::GeometryConversionError(Box::new(e));
                                            eprintln!(
                                                "Failed to convert geojson::{} to geo::Geometry for Building (feature {}): {}",
                                                geojson_type, feature_id, conversion_error
                                            );
                                            return DomainEntity::Unknown(original_feature); // Conversion error
                                        }
                                    }
                                }
                                None => {
                                    #[cfg(debug_assertions)]
                                    eprintln!(
                                        "Geometry is missing for Building (feature {})",
                                        feature_id
                                    );
                                    return DomainEntity::Unknown(original_feature); // Missing geometry
                                }
                            };
                            match building_geometry {
                                GeoRustGeometry::Polygon(polygon) => {
                                    DomainEntity::Building(Building {
                                        id: feature_id,
                                        geometry: GeoRustGeometry::Polygon(polygon),
                                        original_inner_properties: inner_properties,
                                    })
                                }
                                GeoRustGeometry::MultiPolygon(multi_polygon) => {
                                    DomainEntity::Building(Building {
                                        id: feature_id,
                                        geometry: GeoRustGeometry::MultiPolygon(multi_polygon),
                                        original_inner_properties: inner_properties,
                                    })
                                }
                                GeoRustGeometry::LineString(ls) => {
                                    // Allowed type: Closed LineString
                                    if ls.is_closed() {
                                        DomainEntity::Building(Building {
                                            id: feature_id,
                                            geometry: GeoRustGeometry::LineString(ls), // Store the LineString
                                            original_inner_properties: inner_properties,
                                            // Add other specific fields
                                        })
                                    } else {
                                        #[cfg(debug_assertions)]
                                        eprintln!(
                                            "LineString geometry for Building (feature {}) is not closed",
                                            feature_id
                                        );
                                        DomainEntity::Unknown(original_feature) // LineString but not closed
                                    }
                                }
                                // Disallowed geometry type for Building
                                other_geometry => {
                                    #[cfg(debug_assertions)]
                                    eprintln!(
                                        "Disallowed geometry type for Building (feature {}): {:?}",
                                        feature_id,
                                        other_geometry // Use geom_type() for better logging
                                    );
                                    DomainEntity::Unknown(original_feature)
                                }
                            }
                        }
                    }
                }
                Err(Error::InvalidObjectId(unrecognized_id_string)) => {
                    // This branch handles valid strings that are NOT known ObjectIds
                    #[cfg(debug_assertions)]
                    eprintln!(
                        "Unrecognized objectId string: {} for feature {}",
                        unrecognized_id_string, feature_id
                    );
                    DomainEntity::Unknown(original_feature)
                }
                Err(e) => {
                    #[cfg(debug_assertions)]
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
            #[cfg(debug_assertions)]
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

/// Helper function to extract the geometry from a feature.
///
/// # Arguments
///
/// * `feature` - The feature to extract the geometry from.
///
/// # Returns
///
/// * `Result<Point, DomainEntity>` - The extracted geometry.
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
            #[cfg(debug_assertions)]
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
            #[cfg(debug_assertions)]
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

/// Helper function to identify the specific DomainEntity variants
/// for a Point type, given the feature collection.
///
/// # Arguments
///
/// * `geojson` - The feature collection to identify the DomainEntity variants from.
///
/// # Returns
///
/// * `Result<Vec<DomainEntity>, Error>` - The identified DomainEntity variant.
pub fn indentify_domain_entities(geojson: GeoJson) -> Result<Vec<DomainEntity>, Error> {
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

/// Helper function to convert a DomainEntity variant to a GeoJSON feature.
///
/// # Arguments
///
/// * `domain_entity` - The DomainEntity variant to convert.
///
/// # Returns
///
/// * `Feature` - The converted GeoJSON feature.
#[allow(dead_code)]
fn convert_domain_entity_to_geojson_feature(domain_entity: DomainEntity) -> Feature {
    match &domain_entity {
        // Match all Point variants and bind the enum value to `point_entity`
        entity_ref @ DomainEntity::CapturedMarker(_) |
         entity_ref @ DomainEntity::SupplyPoint(_) |
         entity_ref @ DomainEntity::OperationSite(_) |
         entity_ref @ DomainEntity::DrillingPoint(_) |
         entity_ref @ DomainEntity::CableTunnel(_) |
         entity_ref @ DomainEntity::Building(_) |
         entity_ref @ DomainEntity::Unknown(_) // Also include Unknown here
         => {
             entity_ref.into() // Calls the Into<Feature> for &DomainEntity impl
         }
    }
}

/// Helper function to convert a vector of DomainEntity variants to a GeoJSON feature collection.
///
/// # Arguments
///
/// * `domain_entities` - The vector of DomainEntity variants to convert.
///
/// # Returns
///
/// * `GeoJson` - The converted GeoJSON feature collection.
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
    #[test]
    fn test_indentify_domain_entity_with_unknown() {
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
                                "objectId": "Unknown"
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
        assert!(domain_entities[0].is_unknown());
    }

    #[test]
    fn test_identify_domain_entity_with_line_string_geomtery() {
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
    fn test_indentify_domain_entity_with_polygon_geomtery() {
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
                                "objectId": "Gebaeude"
                            }
                        },
                        "geometry": {
                            "type": "Polygon",
                            "coordinates": [
                                [
                                    [0.0, 0.0],
                                    [1.0, 1.0],
                                    [1.0, 0.0],
                                    [0.0, 1.0],
                                    [0.0, 0.0]
                                ]
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
        assert!(domain_entities[0].is_building());
    }
    #[test]
    fn test_identify_domain_entity_with_multi_polygon_geomtery() {
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
                                "objectId": "Gebaeude"
                            }
                        },
                        "geometry": {
                            "type": "MultiPolygon",
                            "coordinates": [
                                [
                                    [
                                        [0.0, 0.0],
                                        [1.0, 1.0],
                                        [1.0, 0.0],
                                        [0.0, 1.0],
                                        [0.0, 0.0]
                                    ]
                                ]
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
        assert!(domain_entities[0].is_building());
    }
}
