use geojson::{GeoJson, Geometry};
use rayon::prelude::*;
use serde_json::Map;
use std::convert::TryFrom;
use crate::utils::error::Error;


// This function processes a single feature without converting to domain-specific structs
// (Kept the same as provided by the user)
fn process_raw_feature(mut feature: geojson::Feature) -> geojson::Feature {
    // let feature_id = feature.id.clone(); // Keep the original ID

    // Simplified properties: just create an empty map
    let properties = Map::with_capacity(1);

    // Get and convert geometry to geo::Geometry
    let geo_geometry: Option<geo::Geometry> = feature.geometry.clone().and_then(|geom| {
        match geo::Geometry::try_from(geom) {
            Ok(geo_geom) => Some(geo_geom),
            Err(_e) => {
                // Logging removed in this simplified version
                // let _geojson_type = feature.geometry.as_ref().map(|g| g.value.type_name()).unwrap_or("None");
                // eprintln!("Raw processing: Failed to convert geojson::{} to geo::Geometry for feature {:?}: {}", _geojson_type, feature_id, _e);
                None // Conversion failed
            }
        }
    });

    // --- Processing Logic based on Geometry Type and Properties ---
    match geo_geometry {
        Some(geo_geom) => {
            match geo_geom {
                // Note: This match moves the inner geometry out of geo_geom
                geo::Geometry::Point(point) => {
                    // Return the processed feature (with original geometry and modified properties)
                    feature.geometry = Some(Geometry::from(&point)); // Convert geo::Point back to geojson::Geometry
                    feature.properties = Some(properties); // Use the modified properties
                    feature.bbox = feature.bbox; // Keep original bbox or recalculate
                    feature.foreign_members = feature.foreign_members; // Keep original
                    feature // Return modified feature
                }
                geo::Geometry::Polygon(polygon) => {
                    feature.geometry = Some(Geometry::from(&polygon)); // Convert geo::Polygon back
                    feature.properties = Some(properties);
                    feature.bbox = feature.bbox;
                    feature.foreign_members = feature.foreign_members;
                    feature // Return modified feature
                }
                geo::Geometry::MultiPolygon(mp) => {
                    // This arm creates a *new* Feature, unlike the others.
                    // Should probably modify 'feature' in place here too for consistency with this version.
                    // Let's change this to match the other arms
                    feature.geometry = Some(Geometry::from(&mp));
                    feature.properties = Some(properties);
                    feature.bbox = feature.bbox;
                    feature.foreign_members = feature.foreign_members;
                    feature // Return modified feature
                }
                geo::Geometry::LineString(ls) => {
                    if ls.is_closed() {
                        feature.geometry = Some(Geometry::from(&ls));
                        feature.properties = Some(properties);
                        feature.bbox = feature.bbox;
                        feature.foreign_members = feature.foreign_members;
                        feature // Return modified feature
                    } else {
                        // It's a LineString with objectId "Gebaeude", but not closed - handle as Unknown or specific error
                        // Error logging removed in this version
                        // eprintln!("Raw processing: Gebaeude feature {:?} has an unclosed LineString geometry", feature_id);
                        feature.geometry = Some(Geometry::from(&ls)); // Keep the unclosed LS geometry
                        feature.properties = Some(properties);
                        feature.bbox = feature.bbox;
                        feature.foreign_members = feature.foreign_members;
                        feature // Return modified feature
                    }
                }
                _ => {
                    // Unidentified or other geometry types
                    // Logging removed in this version
                    // eprintln!("Raw processing: Unidentified feature {:?} (Geometry: {:?}, objectId: {:?})", ...);
                    feature.geometry = Some(Geometry::from(&geo_geom)); // Keep the geometry
                    feature.properties = Some(properties); // Use empty properties map
                    feature.bbox = feature.bbox;
                    feature.foreign_members = feature.foreign_members;
                    feature // Return the (potentially modified) feature
                }
            }
        }
        None => {
            // Handle features with no geometry or conversion failed
            // Error logging removed
            // eprintln!("Raw processing: Feature {:?} has no valid geometry", feature_id);
            feature.geometry = None; // Ensure geometry is None
            feature.properties = Some(properties); // Use empty properties map
            feature.bbox = feature.bbox;
            feature.foreign_members = feature.foreign_members;
            feature // Return the modified feature
        }
    }
}

// The main processing function uses Rayon for parallelism
#[allow(dead_code)]
pub fn process_raw_geojson_parallel(geojson: GeoJson) -> Result<(), Error> {
    let feature_collection = match geojson {
        GeoJson::FeatureCollection(fc) => fc,
        _ => return Err(Error::InvalidFeatureCollection), // Or handle other GeoJson types
    };

    // // // Use Rayon's parallel iterator on the features vector
    // let _processed_features: Vec<geojson::Feature> = feature_collection
    //     .features
    //     .into_par_iter() // Use par_iter() for parallelism
    //     .map(process_raw_feature) // Apply the raw processing function
    //     .collect(); // Collect results from multiple threads

    let _processed_features: Vec<geojson::Feature> = feature_collection
        .features
        .par_chunks(100)
        .map(|chunk| {
            chunk
                .iter()
                .map(|feature: &geojson::Feature| process_raw_feature(feature.clone()))
                .collect::<Vec<geojson::Feature>>()
        })
        .collect::<Vec<Vec<geojson::Feature>>>()
        .into_iter()
        .flatten()
        .collect();

    Ok(())
}
