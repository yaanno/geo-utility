use std::error::Error as StdError;

use thiserror::Error;

// Define error type
#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid geometry type")]
    UnsupportedGeometryType,
    #[error("Missing geometry")]
    MissingGeometry,
    #[error("Invalid coordinates")]
    InvalidCoordinates,
    #[error("Invalid feature")]
    InvalidFeature,
    #[error("Invalid feature collection")]
    InvalidFeatureCollection,
    #[error("Invalid feature properties")]
    InvalidFeatureProperties,
    #[error("Invalid feature geometry")]
    InvalidFeatureGeometry,
    #[error("Invalid objectId: {0}")]
    InvalidObjectId(String),
    #[error("Error converting geometry: {0}")]
    GeometryConversionError(#[from] Box<dyn StdError>),
}