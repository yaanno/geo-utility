// Example separate program/test
use std::fs::File;
use std::io::Write;

use geo_utility::generate_synthetic_linestrings; // Assuming it's public

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
