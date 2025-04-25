use std::fs::File;
use std::io::Read;

use criterion::{Criterion, black_box, criterion_group, criterion_main};

use geo_utility::collect_convex_boundingboxes;
use geojson::FeatureCollection;

fn load_features_from_file(file_path: &str) -> FeatureCollection {
    let mut file = File::open(file_path).expect("Failed to open data file");
    let mut geojson_string = String::new();
    file.read_to_string(&mut geojson_string)
        .expect("Failed to read data file");
    serde_json::from_str(&geojson_string).expect("Failed to parse GeoJSON")
}

fn bench_1k_features(c: &mut Criterion) {
    let features_1k =
        load_features_from_file("synthetic_data_complex_featurecollection_1k_features.geojson");

    c.bench_function("collect_convex_boundingboxes_1k_features", |b| {
        b.iter(|| {
            let input_data = &features_1k;
            let results = collect_convex_boundingboxes(input_data);
            black_box(results.unwrap());
        })
    });
}

fn bench_10k_features(c: &mut Criterion) {
    let features_10k =
        load_features_from_file("synthetic_data_complex_featurecollection_10k_features.geojson");

    c.bench_function("collect_convex_boundingboxes_10k_features", |b| {
        b.iter(|| {
            let input_data = &features_10k;
            let results = collect_convex_boundingboxes(input_data);
            black_box(results.unwrap());
        })
    });
}

fn bench_100k_features(c: &mut Criterion) {
    let features_100k =
        load_features_from_file("synthetic_data_complex_featurecollection_100k_features.geojson");

    c.bench_function("collect_convex_boundingboxes_100k_features", |b| {
        b.iter(|| {
            let input_data = &features_100k;
            let results = collect_convex_boundingboxes(input_data);
            black_box(results.unwrap());
        })
    });
}

fn bench_1000k_features(c: &mut Criterion) {
    let features_1m =
        load_features_from_file("synthetic_data_complex_featurecollection_1000k_features.geojson");

    c.bench_function("collect_convex_boundingboxes_1000k_features", |b| {
        b.iter(|| {
            let input_data = &features_1m;
            let results = collect_convex_boundingboxes(input_data);
            black_box(results.unwrap());
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

criterion_main!(benches_1k, benches_10k, benches_100k, benches_1000k);
