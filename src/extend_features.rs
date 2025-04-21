use geo::{Euclidean, Length, LineString, LineStringSegmentize};

pub fn extend_features(features: Vec<geojson::Feature>) {
    for feature in features {
        if let Some(geometry) = feature.geometry {
            match geometry.value {
                geojson::Value::LineString(line_coords) => {
                    // filter out gebaudekante
                    let is_gebaudekante = feature
                        .properties
                        .as_ref()
                        .and_then(|props| props.get("properties"))
                        .and_then(|nested_props| nested_props.as_object())
                        .and_then(|obj| obj.get("objectId"))
                        .and_then(|object_id| object_id.as_str())
                        .map_or(false, |s| s == "Gebäudekante");
                    if is_gebaudekante {
                        println!("Skipping feature with objectId: Gebäudekante");
                        continue;
                    }
                    // Convert the line coordinates to a LineString
                    let line_string: LineString<f64> = line_coords
                        .into_iter()
                        .map(|coord| geo::Coord {
                            x: coord[0],
                            y: coord[1],
                        })
                        .collect();

                    let length = Euclidean.length(&line_string);
                    println!("Length: {}", length);
                    let segmented_line_string = line_string.line_segmentize(5).unwrap();
                    let segment_endpoints = segmented_line_string
                        .into_iter()
                        .map(|segment| {
                            // get the direction of the segment

                            // Get the last coordinate of each segment
                            let coords = segment.coords().last().unwrap();
                            geo::Point::from(*coords)
                        })
                        .collect::<Vec<_>>();
                    println!("Segment endpoints: {:?}", segment_endpoints);
                    for point in segment_endpoints {
                        println!("Point: {:?}", point);
                    }
                }
                _ => {
                    // Handle other geometry types
                }
            }
        }
    }
}

// create simple test
#[cfg(test)]
mod tests {
    use super::*;
    use geojson::{Feature, Geometry, Value};

    #[test]
    fn test_extend_features() {
        let prop = serde_json::from_str(stringify!({"objectId": "test"})).unwrap();
        let mut properties = serde_json::Map::new();
        properties.insert("properties".to_string(), prop);
        let feature = Feature {
            bbox: None,
            geometry: Some(Geometry {
                bbox: None,
                foreign_members: None,
                value: Value::LineString(vec![vec![0.0, 0.0], vec![1.0, 1.0]]),
            }),
            id: None,
            properties: Some(properties),
            foreign_members: None,
        };
        let features = vec![feature];
        extend_features(features);
    }
}
