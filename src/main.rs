use std::fs::File;
use std::io::Write;

use geo_utility::{
    calculate_bounding_box, generate_synthetic_complex_featurecollection,
    generate_synthetic_featurecollection, generate_synthetic_linestrings,
}; // Assuming it's public

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // generate_synthetic_collection_data()?;
    generate_synthetic_complex_collection_data()?;
    calculate_bounding_box();

    Ok(())
}

#[allow(dead_code)]
fn generate_synthetic_data() -> Result<(), Box<dyn std::error::Error>> {
    let num_features = 1_000; // Or 1_000_000
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
    let num_features = 10_000_000;
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
    let num_features = 100;
    let x_range = (10.2978, 10.2996);
    let y_range = (50.8924, 50.8931);

    println!("Generating {} features...", num_features);
    let mut feature_collection =
        generate_synthetic_complex_featurecollection(num_features, x_range, y_range);

    let num_features = 100;
    let x_range = (10.2989, 10.3012);
    let y_range = (50.8916, 50.8918);

    println!("Generating {} features...", num_features);
    let feature_collection2 =
        generate_synthetic_complex_featurecollection(num_features, x_range, y_range);
    feature_collection.features.extend(feature_collection2.features); // Extend the existing feature collection with the new one
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
