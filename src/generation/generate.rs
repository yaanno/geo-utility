use crate::generation::generate_complex_sample::generate_synthetic_complex_featurecollection;
use crate::generation::generate_closeness_sample::generate_synthetic_data_concatenate_seeded;
use crate::generation::generate_curves_sample::generate_synthetic_linestrings;
use std::fs::File;
use std::io::Write;

pub fn generate_synthetic_data() -> Result<(), Box<dyn std::error::Error>> {
    generate_synthetic_data_curve_test()?;
    generate_synthetic_complex_collection_data()?;
    generate_deterministic_data_concatenate_seeded()?;
    Ok(())
}

fn generate_synthetic_data_curve_test() -> Result<(), Box<dyn std::error::Error>> {
    let features = vec![100, 1_000, 10_000, 100_000];
    let max_vertices = 50;
    let bend_freq = 0.1;
    let max_bend_angle = 45.0;

    for num_features in features {
        println!("Generating {} features...", num_features);
        let features =
            generate_synthetic_linestrings(num_features, max_vertices, bend_freq, max_bend_angle);
        println!("Generation complete. Serializing to JSON...");

        let geojson_string = serde_json::to_string(&features)?;
        let file_path = format!(
            "synthetic_data_curve_test_{}k_features.geojson",
            num_features / 1000
        );
        let mut file = File::create(&file_path)?;
        file.write_all(geojson_string.as_bytes())?;

        println!("Data saved to {}", file_path);
    }

    Ok(())
}

fn generate_synthetic_complex_collection_data() -> Result<(), Box<dyn std::error::Error>> {
    let features = vec![100, 1_000, 10_000, 100_000];
    let x_range = (10.2978, 10.2996);
    let y_range = (50.8924, 50.8931);

    for num_features in features {
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
    }

    Ok(())
}

fn generate_deterministic_data_concatenate_seeded() -> Result<(), Box<dyn std::error::Error>> {
    let features = vec![100, 1_000, 10_000, 100_000];
    let close_pairs_ratio = 0.3;
    let seed = 12345;

    for num_features in features {  
        println!("Generating {} features...", num_features);
        let feature_collection =
            generate_synthetic_data_concatenate_seeded(num_features, close_pairs_ratio, seed);
        println!("Generation complete. Serializing to JSON...");

        let geojson_string = serde_json::to_string(&feature_collection)?;

        let file_path = format!(
            "deterministic_data_concat_seeded_featurecollection_{}k_features.geojson",
            num_features / 1000
        );
        let mut file = File::create(&file_path)?;
        file.write_all(geojson_string.as_bytes())?;

        println!("Data saved to {}", file_path);
    }

    Ok(())
}
