# geo-utility ~~~ WIP

A Rust library providing utility functions for geometric operations and spatial data processing.

!!! This is just an exercise for now. No use for live projects !!!

## Features

- Convex bounding box collection
- Point filtering and simplification
- Integration with common GIS formats (GeoJSON)
- Coordinate system transformations (via PROJ)
- Spatial indexing capabilities (via RTree)

## Dependencies

This library relies on several well-established crates:

- `proj` (0.28.0) - Coordinate transformation and geodetic computations
- `geojson` (0.24.2) - GeoJSON format support
- `geo` (0.30.0) - Core geometric operations
- `rstar` (0.12.2) - RTree spatial indexing
- `ordered-float` (2.0) - Ordered floating-point operations
- `serde_json` (1.0) - JSON serialization/deserialization
- `thiserror` (1.0) - Error handling
- `log` and `env_logger` - Logging functionality

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
geo-utility = "0.1.0"
```

## Usage

```rust
use geo_utility::{collect_convex_boundingboxes, remove_near_points};

// Example usage will depend on your specific needs
```

## Features

### Convex Bounding Boxes

```rust
use geo_utility::collect_convex_boundingboxes;

// Collection of convex bounding boxes for geometric shapes
```

### Point Filtering

```rust
use geo_utility::remove_near_points;

// Remove points that are too close to each other
```

## Requirements

- Rust 2024 edition or later

## Development

### Building

```bash
cargo build
```

### Testing

```bash
cargo test
```

### Documentation

Generate documentation:

```bash
cargo doc --no-deps --open
```
