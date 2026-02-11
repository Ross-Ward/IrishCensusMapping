use geo::{Point, MultiPolygon};

#[derive(Debug, Clone)]
pub struct SmallArea {
    pub id: String,
    pub geometry: MultiPolygon<f64>,
    // Map<DatasetName, Map<Category/NotStated, Count>>
    pub population_data: std::collections::HashMap<String, std::collections::HashMap<String, u32>>,
}

#[derive(Debug, Clone)]
pub struct Dot {
    pub point: Point<f64>,
    pub dataset: String, // Added dataset field
    pub category: String,
}
