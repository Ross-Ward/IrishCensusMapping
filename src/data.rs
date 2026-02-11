use crate::config::AppConfig;
use crate::types::SmallArea;
use anyhow::{Context, Result, anyhow};
use csv::ReaderBuilder;
use geo::MultiPolygon;
use shapefile::Reader;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

pub fn load_data(config: &AppConfig) -> Result<Vec<SmallArea>> {
    println!("Loading data...");

    // 1. Load CSV Data
    let csv_data = load_csv_data(config)?;
    println!("Loaded CSV data for {} areas", csv_data.len());

    // 2. Load Geometry (Shapefile or GeoJSON)
    let extension = config.input.shapefile.extension()
        .and_then(|e| e.to_str())
        .map(|s: &str| s.to_lowercase())
        .ok_or_else(|| anyhow!("Input geometry file has no extension"))?;

    let small_areas = match extension.as_str() {
        "shp" => load_shapefile_and_join(config, &csv_data)?,
        "json" | "geojson" => load_geojson_and_join(config, &csv_data)?,
        _ => return Err(anyhow!("Unsupported geometry format: {}", extension)),
    };

    println!("Loaded and joined geometry for {} areas", small_areas.len());

    Ok(small_areas)
}


fn load_csv_data(config: &AppConfig) -> Result<HashMap<String, HashMap<String, HashMap<String, u32>>>> {
    let file = File::open(&config.input.data_csv)
        .with_context(|| format!("Failed to open CSV file: {:?}", config.input.data_csv))?;
    let mut rdr = ReaderBuilder::new().from_reader(file);
    let headers = rdr.headers()?.clone();

    // Identify indices for join column and all category columns
    let join_col_idx = headers.iter().position(|h| h == config.input.join_column_csv)
        .ok_or_else(|| anyhow!("Join column '{}' not found in CSV", config.input.join_column_csv))?;

    // Map column names to indices for faster lookup
    let col_indices: HashMap<String, usize> = headers.iter().enumerate()
        .map(|(i, h)| (h.to_string(), i))
        .collect();

    let mut data_map = HashMap::new();

    for result in rdr.records() {
        let record = result?;
        let id = record.get(join_col_idx).unwrap_or("").to_string();
        
        if id.is_empty() { continue; }

        let mut area_datasets = HashMap::new();

        for (dataset_name, dataset_config) in &config.processing.datasets {
            let mut population_data = HashMap::new();

            // 1. Process Categories
            for category in &dataset_config.categories {
                let mut total = 0;
                for col_name in &category.columns {
                     if let Some(&idx) = col_indices.get(col_name) {
                         let val: u32 = record.get(idx).unwrap_or("0").parse().unwrap_or(0);
                         total += val;
                     }
                }
                population_data.insert(category.name.clone(), total);
            }

            // 2. Process Not Stated
            if let Some(ns_config) = &dataset_config.not_stated {
                if let Some(&idx) = col_indices.get(&ns_config.column) {
                    let val: u32 = record.get(idx).unwrap_or("0").parse().unwrap_or(0);
                    population_data.insert("Not Stated".to_string(), val);
                }
            }
            
            area_datasets.insert(dataset_name.clone(), population_data);
        }

        data_map.insert(id, area_datasets);
    }

    Ok(data_map)
}


fn load_shapefile_and_join(
    config: &AppConfig,
    csv_data: &HashMap<String, HashMap<String, HashMap<String, u32>>>
) -> Result<Vec<SmallArea>> {
    let mut reader = Reader::from_path(&config.input.shapefile)
        .with_context(|| format!("Failed to open Shapefile: {:?}", config.input.shapefile))?;

    let mut small_areas = Vec::new();

    for result in reader.iter_shapes_and_records() {
        let (shape, record) = result?;
        
        // Find the Join ID in the shapefile record (dbase)
        let id_value = record.get(&config.input.join_column_shape)
            .ok_or_else(|| anyhow!("Join column '{}' not found in Shapefile", config.input.join_column_shape))?;
        
        let id = match id_value {
            shapefile::dbase::FieldValue::Character(Some(s)) => s.clone(),
            shapefile::dbase::FieldValue::Character(None) => continue, // Skip if null
            _ => return Err(anyhow!("Shapefile join column must be a string")),
        };

        // If we have matching CSV data, create the SmallArea object
        if let Some(pop_data) = csv_data.get(&id) {
             let geometry = match shape {
                shapefile::Shape::Polygon(polygon) => {
                    let geo_polygon: MultiPolygon<f64> = polygon.try_into()
                        .map_err(|e| anyhow!("Failed to convert polygon: {:?}", e))?;
                    geo_polygon
                },
                 shapefile::Shape::PolygonM(polygon) => {
                     let geo_polygon: MultiPolygon<f64> = polygon.try_into()
                         .map_err(|e| anyhow!("Failed to convert polygonM: {:?}", e))?;
                     geo_polygon
                 },
                 shapefile::Shape::PolygonZ(polygon) => {
                     let geo_polygon: MultiPolygon<f64> = polygon.try_into()
                         .map_err(|e| anyhow!("Failed to convert polygonZ: {:?}", e))?;
                     geo_polygon
                 },
                _ => continue, // Skip non-polygon shapes
            };

            small_areas.push(SmallArea {
                id: id.clone(),
                geometry,
                population_data: pop_data.clone(),
            });
        }
    }

    Ok(small_areas)
}

fn load_geojson_and_join(
    config: &AppConfig,
    csv_data: &HashMap<String, HashMap<String, HashMap<String, u32>>>
) -> Result<Vec<SmallArea>> {
    use std::io::BufReader;
    use geojson::{GeoJson, Value};
    use std::convert::TryInto; // For TryInto<MultiPolygon>

    println!("Loading GeoJSON from {:?}...", config.input.shapefile);
    let file = File::open(&config.input.shapefile)
        .with_context(|| format!("Failed to open GeoJSON file: {:?}", config.input.shapefile))?;
    let reader = BufReader::new(file);
    
    // Parse the GeoJSON. warning: this loads the whole file into memory.
    let geojson = GeoJson::from_reader(reader).context("Failed to parse GeoJSON")?;
    
    let collection = match geojson {
        GeoJson::FeatureCollection(fc) => fc,
        _ => return Err(anyhow!("GeoJSON must be a FeatureCollection")),
    };

    let mut small_areas = Vec::new();

    for feature in collection.features {
        // 1. Get ID
        let id_val = feature.properties.as_ref()
            .and_then(|props| props.get(&config.input.join_column_shape));
        
        let id = match id_val {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(serde_json::Value::Number(n)) => n.to_string(),
            _ => continue, // Skip if no ID or not string/number
        };

        // 2. Check match
        if let Some(pop_data) = csv_data.get(&id) {
             // 3. Get Geometry
             // geojson crate Value -> geo types -> MultiPolygon
             let geometry = match feature.geometry {
                 Some(geo) => {
                     let valid_geo: geo::Geometry<f64> = geo.value.try_into()
                        .map_err(|e| anyhow!("Failed to convert geojson geometry: {:?}", e))?;
                     
                     match valid_geo {
                         geo::Geometry::MultiPolygon(mp) => mp,
                         geo::Geometry::Polygon(p) => MultiPolygon::new(vec![p]),
                         _ => continue, // Skip points/lines
                     }
                 },
                 None => continue,
             };

            small_areas.push(SmallArea {
                id,
                geometry,
                population_data: pop_data.clone(),
            });
        }
    }

    Ok(small_areas)
}

