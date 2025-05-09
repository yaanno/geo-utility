use std::fs::File;
use std::io::Read;

use geojson::FeatureCollection;

use crate::{concatenate_features::concatenate_features, geometry::scaling::scale_buildings};

pub fn concat_and_scale(
    featurecollection: &FeatureCollection,
    scale_factor: f64,
) -> FeatureCollection {
    let concatenated_features = concatenate_features(featurecollection);
    let scaled_features = scale_buildings(&concatenated_features.into() , scale_factor);
    scaled_features.into()
}

fn load_features_from_file(file_path: &str) -> geojson::FeatureCollection {
    let mut file = File::open(file_path).expect("Failed to open data file");
    let mut geojson_string = String::new();
    file.read_to_string(&mut geojson_string)
        .expect("Failed to read data file");
    serde_json::from_str(&geojson_string).unwrap()
}

pub fn concat_test() {
    let features_100 = load_features_from_file("geometry.json");
    println!("features_100: {}", features_100.features.len());
    let scaled_features = concat_and_scale(&features_100, 1.0);
    println!("scaled_features: {}", scaled_features.features.len());
}