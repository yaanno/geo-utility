use std::fs::File;
use std::io::Read;

use geojson::{Feature, FeatureCollection, GeoJson, Geometry};

use crate::collect_bounding_boxes;

fn load_features_from_file(file_path: &str) -> FeatureCollection {
    let mut file = File::open(file_path).expect("Failed to open data file");
    let mut geojson_string = String::new();
    file.read_to_string(&mut geojson_string)
        .expect("Failed to read data file");
    serde_json::from_str(&geojson_string).expect("Failed to parse GeoJSON")
}

fn feature_collection(features: Vec<Feature>) -> FeatureCollection {
    FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    }
}

pub fn calculate_bounding_box() {
    let features_1k =
        load_features_from_file("synthetic_data_complex_featurecollection_0k_features.geojson");
        let result_rectangles = collect_bounding_boxes(&features_1k, 5.0, false);
        // --- Convert result Rectangles to GeoJSON Features ---
        let result_features: Vec<Feature> = result_rectangles
            .iter()
            .map(|rect| {
                let geo_rect = rect.to_geo_rect(); // Convert your Rectangle to geo::Rect
                let polygon = geo::Polygon::from(geo_rect); // Convert geo::Rect to geo::Polygon
                let geometry = Geometry::from(&polygon); // Convert geo::Polygon to geojson::Geometry
                Feature {
                    bbox: None,
                    geometry: Some(geometry),
                    id: None,
                    properties: None,
                    foreign_members: None,
                }
            })
            .collect();

        // --- Combine input points and output grid cells into one FeatureCollection ---
        let mut all_features = features_1k.features;
        all_features.extend(result_features);
        let output_fc = feature_collection(all_features);
        let geojson_output = GeoJson::from(output_fc);
        let geojson_string = geojson_output.to_string();

        // --- Print the GeoJSON string ---
        println!("--- GeoJSON Output for Visualization ---");
        println!("{}", geojson_string);
        println!("--- End GeoJSON Output ---");
}