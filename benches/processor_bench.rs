use std::fs::File;
use std::io::Read;

use criterion::{Criterion, black_box, criterion_group, criterion_main};

use geo_utility::process_vertices_and_bends;
use geojson::Feature;

fn load_features_from_file(file_path: &str) -> Vec<Feature> {
    let mut file = File::open(file_path).expect("Failed to open data file");
    let mut geojson_string = String::new();
    file.read_to_string(&mut geojson_string)
        .expect("Failed to read data file");
    serde_json::from_str(&geojson_string).expect("Failed to parse GeoJSON")
}

fn bench_1k_features(c: &mut Criterion) {
    let bend_threshold_degrees = 10.0;
    let extension_distance = 0.5;

    let features_1k = load_features_from_file("synthetic_data_1k_features.geojson");

    c.bench_function("process_vertices_and_bends_1k_features", |b| {
        b.iter(|| {
            let input_data = features_1k.clone();
            let results =
                process_vertices_and_bends(input_data, bend_threshold_degrees, extension_distance);
            black_box(results);
        })
    });
}

fn bench_10k_features(c: &mut Criterion) {
    let bend_threshold_degrees = 10.0;
    let extension_distance = 0.5;

    let features_10k = load_features_from_file("synthetic_data_10k_features.geojson");

    c.bench_function("process_vertices_and_bends_10k_features", |b| {
        b.iter(|| {
            let input_data = features_10k.clone();
            let results =
                process_vertices_and_bends(input_data, bend_threshold_degrees, extension_distance);
            black_box(results);
        })
    });
}

fn bench_100k_features(c: &mut Criterion) {
    let bend_threshold_degrees = 10.0;
    let extension_distance = 0.5;

    let features_100k = load_features_from_file("synthetic_data_100k_features.geojson");

    c.bench_function("process_vertices_and_bends_100k_features", |b| {
        b.iter(|| {
            let input_data = features_100k.clone();
            let results =
                process_vertices_and_bends(input_data, bend_threshold_degrees, extension_distance);
            black_box(results);
        })
    });
}

fn bench_1000k_features(c: &mut Criterion) {
    let bend_threshold_degrees = 10.0;
    let extension_distance = 0.5;

    let features_1m = load_features_from_file("synthetic_data_1000k_features.geojson");

    c.bench_function("process_vertices_and_bends_1000k_features", |b| {
        b.iter(|| {
            let input_data = features_1m.clone();
            let results =
                process_vertices_and_bends(input_data, bend_threshold_degrees, extension_distance);
            black_box(results);
        })
    });
}

criterion_group!(
    name = benches_1k;
    config = Criterion::default().sample_size(100);
    targets = bench_1k_features
);

criterion_group!(
    name = benches_10k;
    config = Criterion::default().sample_size(100);
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

criterion_main!(benches_1k, benches_10k, benches_100k, benches_1000k);
