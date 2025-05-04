use geojson::{GeoJson, Geometry};
use serde_json::Map;
use std::convert::TryFrom;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid feature collection")]
    InvalidFeatureCollection,
}

// This function processes a single feature without converting to domain-specific structs
fn process_raw_feature(feature: geojson::Feature) -> geojson::Feature {
    let feature_id = feature.id.clone(); // Keep the original ID

    let properties = Map::with_capacity(1);

    // Get and convert geometry to geo::Geometry
    let geo_geometry: Option<geo::Geometry> = feature.geometry.clone().and_then(|geom| {
        match geo::Geometry::try_from(geom) {
            Ok(geo_geom) => Some(geo_geom),
            Err(_e) => {
                let _geojson_type = feature
                    .geometry
                    .as_ref()
                    .map(|g| g.value.type_name())
                    .unwrap_or("None");
                None // Conversion failed
            }
        }
    });

    // --- Processing Logic based on Geometry Type and Properties ---
    // This is where the complexity increases compared to the domain model
    match geo_geometry {
        Some(geo_geom) => {
            match geo_geom {
                geo::Geometry::Point(point) => {
                    // Return the processed feature (with original geometry and modified properties)
                    geojson::Feature {
                        id: feature_id,
                        geometry: Some(Geometry::from(&point)), // Pass reference to Point
                        properties: Some(properties),
                        bbox: feature.bbox,
                        foreign_members: feature.foreign_members,
                    }
                }
                geo::Geometry::Polygon(polygon) => {
                    geojson::Feature {
                        id: feature_id,
                        geometry: Some(Geometry::from(&polygon)), // Convert geo::Polygon back
                        properties: Some(properties),
                        bbox: feature.bbox,
                        foreign_members: feature.foreign_members,
                    }
                }
                geo::Geometry::MultiPolygon(mp) => {
                    geojson::Feature {
                        id: feature_id,
                        geometry: Some(Geometry::from(&mp)), // Convert geo::MultiPolygon back
                        properties: Some(properties),
                        bbox: feature.bbox,
                        foreign_members: feature.foreign_members,
                    }
                }
                geo::Geometry::LineString(ls) => {
                    if ls.is_closed() {
                        geojson::Feature {
                            id: feature_id,
                            geometry: Some(Geometry::from(&ls)),
                            properties: Some(properties),
                            bbox: feature.bbox,
                            foreign_members: feature.foreign_members,
                        }
                    } else {
                        // It's a LineString with objectId "Gebaeude", but not closed - handle as Unknown or specific error
                        geojson::Feature {
                            id: feature_id,
                            geometry: Some(Geometry::from(&ls)),
                            properties: Some(properties),
                            bbox: feature.bbox,
                            foreign_members: feature.foreign_members,
                        }
                    } // Or modify properties to indicate error
                }
                _ => geojson::Feature {
                    id: feature_id,
                    geometry: None,
                    properties: Some(properties),
                    bbox: feature.bbox,
                    foreign_members: feature.foreign_members,
                },
            }
        }
        None => {
            // Handle features with no geometry
            geojson::Feature {
                id: feature_id,
                geometry: None,               // No geometry
                properties: Some(properties), // Keep properties
                bbox: feature.bbox,
                foreign_members: feature.foreign_members,
            }
        }
    }
}

// The main processing function would iterate and call process_raw_feature
#[allow(dead_code)]
pub fn process_raw_geojson(geojson: GeoJson) -> Result<(), Error> {
    let feature_collection = match geojson {
        GeoJson::FeatureCollection(fc) => fc,
        _ => return Err(Error::InvalidFeatureCollection), // Or handle other GeoJson types
    };

    let _processed_features: Vec<geojson::Feature> = feature_collection
        .features
        .into_iter()
        .map(process_raw_feature)
        .collect();

    Ok(())
}
