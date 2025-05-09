use std::fs::File;
use std::io::Write;

use geo_utility::concat_and_scale::concat_test;
use geo_utility::generation::complex::generate_synthetic_complex_featurecollection;
use geo_utility::generation::deterministic_data_concatenate::generate_synthetic_data_collection;
use geo_utility::generation::deterministic_feature_collection::generate_deterministic_feature_collection;
use geo_utility::generation::featurecollection::generate_synthetic_featurecollection;
use geo_utility::generation::linestrings::generate_synthetic_linestrings;
use geo_utility::generation::deterministic_data_concatenate_seeded::generate_synthetic_data_concatenate_seeded;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // generate_synthetic_collection_data()?;
    generate_synthetic_complex_collection_data()?;
    // calculate_bounding_box();
    // generate_deterministic_collection_data()?;
    // generate_deterministic_collection_data()?;
    // generate_synthetic_data()?;
    // generate_synthetic_collection_data()?;
    // generate_deterministic_data_concatenate()?;
    // concat_test();
    Ok(())
}

#[allow(dead_code)]
fn generate_synthetic_data() -> Result<(), Box<dyn std::error::Error>> {
    let num_features = 100_000; // Or 1_000_000
    let max_vertices = 50;
    let bend_freq = 0.1;
    let max_bend_angle = 45.0;

    println!("Generating {} features...", num_features);
    let features =
        generate_synthetic_linestrings(num_features, max_vertices, bend_freq, max_bend_angle);
    println!("Generation complete. Serializing to JSON...");

    let geojson_string = serde_json::to_string(&features)?;

    let file_path = format!("synthetic_data_{}k_features.geojson", num_features / 1000);
    let mut file = File::create(&file_path)?;
    file.write_all(geojson_string.as_bytes())?;

    println!("Data saved to {}", file_path);

    Ok(())
}

#[allow(dead_code)]
fn generate_synthetic_collection_data() -> Result<(), Box<dyn std::error::Error>> {
    let num_features = 100_000;
    let x_range = (-1000.0, 1000.0);
    let y_range = (-1000.0, 1000.0);

    println!("Generating {} features...", num_features);
    let feature_collection = generate_synthetic_featurecollection(num_features, x_range, y_range);
    println!("Generation complete. Serializing to JSON...");

    let geojson_string = serde_json::to_string(&feature_collection)?;

    let file_path = format!(
        "synthetic_data_featurecollection_{}k_features.geojson",
        num_features / 1000
    );
    let mut file = File::create(&file_path)?;
    file.write_all(geojson_string.as_bytes())?;

    println!("Data saved to {}", file_path);

    Ok(())
}

#[allow(dead_code)]
fn generate_synthetic_complex_collection_data() -> Result<(), Box<dyn std::error::Error>> {
    let num_features = 1000000;
    let x_range = (10.2978, 10.2996);
    let y_range = (50.8924, 50.8931);

    println!("Generating {} features...", num_features);
    let feature_collection =
        generate_synthetic_complex_featurecollection(num_features, x_range, y_range);

    println!("Generation complete. Serializing to JSON...");

    let geojson_string = serde_json::to_string(&feature_collection)?;

    let file_path = format!(
        "synthetic_data_complex_featurecollection_{}k_features.geojson",
        num_features / 1000
    );
    let mut file = File::create(&file_path)?;
    file.write_all(geojson_string.as_bytes())?;

    println!("Data saved to {}", file_path);

    Ok(())
}

#[allow(dead_code)]
fn generate_deterministic_collection_data() -> Result<(), Box<dyn std::error::Error>> {
    let num_features = 100000;

    println!("Generating {} features...", num_features);
    let feature_collection = generate_deterministic_feature_collection(num_features);
    println!("Generation complete. Serializing to JSON...");

    let geojson_string = serde_json::to_string(&feature_collection)?;

    let file_path = format!(
        "deterministic_data_featurecollection_{}k_features.geojson",
        num_features / 1000
    );
    let mut file = File::create(&file_path)?;
    file.write_all(geojson_string.as_bytes())?;

    println!("Data saved to {}", file_path);

    Ok(())
}

#[allow(dead_code)]
fn generate_deterministic_data_concatenate() -> Result<(), Box<dyn std::error::Error>> {
    let num_features = 100;
    let max_vertices_per_feature = 2;
    let bend_frequency = 0.1;
    let max_bend_angle = 10.0;

    println!("Generating {} features...", num_features);
    let feature_collection = generate_synthetic_data_collection(
        num_features,
        max_vertices_per_feature,
        bend_frequency,
        max_bend_angle,
    );
    println!("Generation complete. Serializing to JSON...");

    let geojson_string = serde_json::to_string(&feature_collection)?;

    let file_path = format!(
        "deterministic_data_concat_featurecollection_{}k_features.geojson",
        num_features / 1000
    );
    let mut file = File::create(&file_path)?;
    file.write_all(geojson_string.as_bytes())?;

    println!("Data saved to {}", file_path);

    Ok(())
}

#[allow(dead_code)]
fn generate_deterministic_data_concatenate_seeded() -> Result<(), Box<dyn std::error::Error>> {
    let num_features = 100;
    let close_pairs_ratio = 0.3;
    let seed = 12345;

    println!("Generating {} features...", num_features);
    let feature_collection = generate_synthetic_data_concatenate_seeded(
        num_features,
        close_pairs_ratio,
        seed,
    );
    println!("Generation complete. Serializing to JSON...");

    let geojson_string = serde_json::to_string(&feature_collection)?;

    let file_path = format!(
        "deterministic_data_concat_seeded_featurecollection_{}k_features.geojson",
        num_features / 1000
    );
    let mut file = File::create(&file_path)?;
    file.write_all(geojson_string.as_bytes())?;

    println!("Data saved to {}", file_path);

    Ok(())
}
