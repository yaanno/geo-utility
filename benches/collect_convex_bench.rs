use std::fs::File;
use std::io::Read;

use criterion::{Criterion, black_box, criterion_group, criterion_main};

use geo_utility::geometry::convex_hull::collect_convex_boundingboxes;
use geojson::FeatureCollection;

fn load_features_from_file(file_path: &str) -> FeatureCollection {
    let mut file = File::open(file_path).expect("Failed to open data file");
    let mut geojson_string = String::new();
    file.read_to_string(&mut geojson_string)
        .expect("Failed to read data file");
    serde_json::from_str(&geojson_string).expect("Failed to parse GeoJSON")
}

fn bench_100_features(c: &mut Criterion) {
    let features_100 =
        load_features_from_file("synthetic_data_complex_featurecollection_0k_features.geojson");

    c.bench_function("collect_convex_boundingboxes_100_features", |b| {
        b.iter_with_setup(
            || {
                features_100.clone().into() // Clone the data here
            },
            |input_data| {
                let results = collect_convex_boundingboxes(&input_data);
                black_box(results.unwrap());
            }
        )
    });
}

fn bench_1k_features(c: &mut Criterion) {
    let features_1k =
        load_features_from_file("synthetic_data_complex_featurecollection_1k_features.geojson");

    c.bench_function("collect_convex_boundingboxes_1k_features", |b| {
        b.iter_with_setup(
            || {
                features_1k.clone().into() // Clone the data here
            },
            |input_data| {
                let results = collect_convex_boundingboxes(&input_data);
                black_box(results.unwrap());
            }
        )
    });
}

fn bench_10k_features(c: &mut Criterion) {
    let features_10k =
        load_features_from_file("synthetic_data_complex_featurecollection_10k_features.geojson");

    c.bench_function("collect_convex_boundingboxes_10k_features", |b| {
        b.iter_with_setup(
            || {
                features_10k.clone().into() // Clone the data here
            },
            |input_data| {
                let results = collect_convex_boundingboxes(&input_data);
                black_box(results.unwrap());
            }
        )
    });
}

fn bench_100k_features(c: &mut Criterion) {
    let features_100k =
        load_features_from_file("synthetic_data_complex_featurecollection_100k_features.geojson");

    c.bench_function("collect_convex_boundingboxes_100k_features", |b| {
        b.iter_with_setup(
            || {
                features_100k.clone().into() // Clone the data here
            },
            |input_data| {
                let results = collect_convex_boundingboxes(&input_data);
                black_box(results.unwrap());
            }
        )
    });
}

fn bench_1000k_features(c: &mut Criterion) {
    let features_1m =
        load_features_from_file("synthetic_data_complex_featurecollection_1000k_features.geojson");

    c.bench_function("collect_convex_boundingboxes_1000k_features", |b| {
        b.iter_with_setup(
            || {
                features_1m.clone().into() // Clone the data here
            },
            |input_data| {
                let results = collect_convex_boundingboxes(&input_data);
                black_box(results.unwrap());
            }
        )
    });
}

criterion_group!(
    name = benches_100;
    config = Criterion::default().sample_size(50);
    targets = bench_100_features
);


criterion_group!(
    name = benches_1k;
    config = Criterion::default().sample_size(50);
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
    config = Criterion::default().sample_size(10);
    targets = bench_1000k_features
);

criterion_main!(benches_100, benches_1k, benches_10k, benches_100k, /*benches_1000k*/);
