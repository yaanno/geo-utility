use std::fs::File;
use std::io::Read;

use criterion::{Criterion, black_box, criterion_group, criterion_main};

use geo_utility::scale_buildings;
use geojson::FeatureCollection;

fn load_features_from_file(file_path: &str) -> FeatureCollection {
    let mut file = File::open(file_path).expect("Failed to open data file");
    let mut geojson_string = String::new();
    file.read_to_string(&mut geojson_string)
        .expect("Failed to read data file");
    serde_json::from_str(&geojson_string).expect("Failed to parse GeoJSON")
}

fn bench_1k_features(c: &mut Criterion) {
    let scale_factor = 0.5;

    let features_1k =
        load_features_from_file("synthetic_data_featurecollection_1k_features.geojson");

    c.bench_function("scale_buildings_1k_features", |b| {
        b.iter(|| {
            let input_data = features_1k.clone();
            let results = scale_buildings(&input_data, scale_factor);
            black_box(results);
        })
    });
}

fn bench_10k_features(c: &mut Criterion) {
    let scale_factor = 0.5;

    let features_10k =
        load_features_from_file("synthetic_data_featurecollection_10k_features.geojson");

    c.bench_function("scale_buildings_10k_features", |b| {
        b.iter(|| {
            let input_data = features_10k.clone();
            let results = scale_buildings(&input_data, scale_factor);
            black_box(results);
        })
    });
}

fn bench_100k_features(c: &mut Criterion) {
    let scale_factor = 0.5;

    let features_100k =
        load_features_from_file("synthetic_data_featurecollection_100k_features.geojson");

    c.bench_function("scale_buildings_100k_features", |b| {
        b.iter(|| {
            let input_data = features_100k.clone();
            let results = scale_buildings(&input_data, scale_factor);
            black_box(results);
        })
    });
}

fn bench_1000k_features(c: &mut Criterion) {
    let scale_factor = 0.5;

    let features_1m =
        load_features_from_file("synthetic_data_featurecollection_1000k_features.geojson");

    c.bench_function("scale_buildings_1000k_features", |b| {
        b.iter(|| {
            let input_data = features_1m.clone();
            let results = scale_buildings(&input_data, scale_factor);
            black_box(results);
        })
    });
}

fn bench_10_000k_features(c: &mut Criterion) {
    let scale_factor = 0.5;

    let features_1m =
        load_features_from_file("synthetic_data_featurecollection_10000k_features.geojson");

    c.bench_function("scale_buildings_10_000k_features", |b| {
        b.iter(|| {
            let input_data = features_1m.clone();
            let results = scale_buildings(&input_data, scale_factor);
            black_box(results);
        })
    });
}

criterion_group!(
    name = benches_1k;
    config = Criterion::default().sample_size(50); // Reduce samples
    targets = bench_1k_features                           // List the benchmark function(s) for this group
);

// For 10k features, the warning was less specific about reduction, but let's keep default or slightly reduced if needed
// Warning was "increase target time to 24.3s" or reduce sample count. Let's stick with default 100 samples here unless it's too long.
criterion_group!(
    name = benches_10k;
    // Use default settings (100 samples, 5s target time)
    config = Criterion::default().sample_size(100); // Explicitly 100 samples
    targets = bench_10k_features
);

// For 100k features, warning suggested sample count 10 (or increase time to 27.7s)
criterion_group!(
    name = benches_100k;
    config = Criterion::default().sample_size(10); // Reduce samples significantly
    targets = bench_100k_features
);

// For 100k features, warning suggested sample count 10 (or increase time to 27.7s)
criterion_group!(
    name = benches_1000k;
    config = Criterion::default().sample_size(10); // Reduce samples significantly
    targets = bench_1000k_features
);

// For 100k features, warning suggested sample count 10 (or increase time to 27.7s)
criterion_group!(
    name = benches_10_000k;
    config = Criterion::default().sample_size(10); // Reduce samples significantly
    targets = bench_10_000k_features
);

criterion_main!(
    benches_1k,
    benches_10k,
    benches_100k,
    benches_1000k,
    benches_10_000k
);
