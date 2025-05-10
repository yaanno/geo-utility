use std::fs::File;
use std::io::Read;

use criterion::{Criterion, black_box, criterion_group, criterion_main};

use geo_utility::processing::process_vertices_and_bends::process_vertices_and_bends;
use geojson::Feature;

fn load_features_from_file(file_path: &str) -> Vec<Feature> {
    let mut file = File::open(file_path).expect("Failed to open data file");
    let mut geojson_string = String::new();
    file.read_to_string(&mut geojson_string)
        .expect("Failed to read data file");
    serde_json::from_str(&geojson_string).expect("Failed to parse GeoJSON")
}


fn bench_100_features(c: &mut Criterion) {
    let bend_threshold_degrees = 10.0;
    let extension_distance = 0.5;

    let features_100 = load_features_from_file("synthetic_data_curve_test_0k_features.geojson");

    c.bench_function("process_vertices_and_bends_100_features", |b| {
        b.iter_with_setup(
            || {
                // This setup closure is run before each iteration to provide input
                features_100.clone() // Clone the data here
            },
            |input_data| {
                // This is the code being timed
                let results = process_vertices_and_bends(input_data.into_iter().map(|f| f.into()).collect(), bend_threshold_degrees, extension_distance);
                black_box(results);
            }
        )
    });
}

fn bench_1k_features_processing_only(c: &mut Criterion) {
    let bend_threshold_degrees = 10.0;
    let extension_distance = 0.5;

    let features_1k_loaded = load_features_from_file("synthetic_data_curve_test_1k_features.geojson");

    c.bench_function("process_vertices_and_bends_1k_features_processing_only", |b| {
        b.iter_with_setup(
            || {
                // This setup closure is run before each iteration to provide input
                features_1k_loaded.clone() // Clone the data here
            },
            |input_data| {
                // This is the code being timed
                let results = process_vertices_and_bends(input_data.into_iter().map(|f| f.into()).collect(), bend_threshold_degrees, extension_distance);
                black_box(results);
            }
        )
    });
}

fn bench_10k_features(c: &mut Criterion) {
    let bend_threshold_degrees = 10.0;
    let extension_distance = 0.5;

    let features_10k = load_features_from_file("synthetic_data_curve_test_10k_features.geojson");

    c.bench_function("process_vertices_and_bends_10k_features", |b| {
        b.iter_with_setup(
            || {
                // This setup closure is run before each iteration to provide input
                features_10k.clone() // Clone the data here
            },
            |input_data| {
                // This is the code being timed
                let results = process_vertices_and_bends(input_data.into_iter().map(|f| f.into()).collect(), bend_threshold_degrees, extension_distance);
                black_box(results);
            }
        )
    });
}

fn bench_100k_features(c: &mut Criterion) {
    let bend_threshold_degrees = 10.0;
    let extension_distance = 0.5;

    let features_100k = load_features_from_file("synthetic_data_curve_test_100k_features.geojson");

    c.bench_function("process_vertices_and_bends_100k_features", |b| {
        b.iter_with_setup(
            || {
                // This setup closure is run before each iteration to provide input
                features_100k.clone() // Clone the data here
            },
            |input_data| {
                // This is the code being timed
                let results = process_vertices_and_bends(input_data.into_iter().map(|f| f.into()).collect(), bend_threshold_degrees, extension_distance);
                black_box(results);
            }
        )
    });
}

fn bench_1000k_features(c: &mut Criterion) {
    let bend_threshold_degrees = 10.0;
    let extension_distance = 0.5;

    let features_1m = load_features_from_file("synthetic_data_curve_test_1000k_features.geojson");

    c.bench_function("process_vertices_and_bends_1000k_features", |b| {
        b.iter_with_setup(
            || {
                // This setup closure is run before each iteration to provide input
                features_1m.clone() // Clone the data here
            },
            |input_data| {
                // This is the code being timed
                let results = process_vertices_and_bends(input_data.into_iter().map(|f| f.into()).collect(), bend_threshold_degrees, extension_distance);
                black_box(results);
            }
        )
    });
}

criterion_group!(
    name = benches_100;
    config = Criterion::default().sample_size(100);
    targets = bench_100_features
);

criterion_group!(
    name = benches_1k;
    config = Criterion::default().sample_size(50);
    targets = bench_1k_features_processing_only
);

criterion_group!(
    name = benches_10k;
    config = Criterion::default().sample_size(10);
    targets = bench_10k_features
);

criterion_group!(
    name = benches_100k;
    config = Criterion::default().sample_size(10);
    targets = bench_100k_features
);

criterion_group!(
    name = benches_1000k;
    config = Criterion::default().sample_size(10)
    .measurement_time(std::time::Duration::from_secs(50));
    targets = bench_1000k_features
);

criterion_main!(benches_100, benches_1k, benches_10k, benches_100k, /*benches_1000k*/);
