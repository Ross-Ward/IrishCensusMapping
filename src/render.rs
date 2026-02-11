use crate::config::AppConfig;
use crate::types::Dot;
use anyhow::{Context, Result};
use image::{ImageBuffer, Rgba, RgbaImage};
use rayon::prelude::*;
use std::collections::HashMap;
use std::f64::consts::PI;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

// Constants for Web Mercator
const TILE_SIZE: u32 = 256;

pub fn generate_tiles(config: &AppConfig, dots: Vec<Dot>) -> Result<()> {
    println!("Generating tiles from min_zoom {} to max_zoom {}...", config.output.min_zoom, config.output.max_zoom);

    // Group dots by Dataset
    // We want to render distinct tile sets for each dataset
    // output/tiles/{dataset_name}/z/x/y.png
    
    let mut dots_by_dataset: HashMap<String, Vec<Dot>> = HashMap::new();
    for dot in dots {
        dots_by_dataset.entry(dot.dataset.clone()).or_default().push(dot);
    }
    
    // Process each dataset
    for (dataset_name, dataset_dots) in dots_by_dataset {
        println!("Rendering dataset: {}", dataset_name);
        
        let colors: Arc<HashMap<String, Rgba<u8>>> = Arc::new(
            config.processing.datasets.get(&dataset_name)
                .expect("Dataset config missing")
                .categories.iter()
                .map(|c| (c.name.clone(), hex_to_rgba(&c.color)))
                .collect()
        );
        
        let dataset_dots = Arc::new(dataset_dots);
        let dataset_name = Arc::new(dataset_name);

        (config.output.min_zoom..=config.output.max_zoom).into_par_iter().for_each(|z| {
            let _ = render_zoom_level(config, &dataset_name, z, &dataset_dots, &colors);
        });
    }

    Ok(())
}

fn render_zoom_level(
    config: &AppConfig,
    dataset_name: &str,
    zoom: u8,
    dots: &Vec<Dot>,
    colors: &HashMap<String, Rgba<u8>>
) -> Result<()> {
    // println!("Rendering {} z{}", dataset_name, zoom);
    
    let mut local_tiles: HashMap<(u32, u32), RgbaImage> = HashMap::new();
    
    for dot in dots {
        let (tx, ty, px, py) = lat_lon_to_tile_pixel(dot.point.y(), dot.point.x(), zoom);
        
        let tile_img = local_tiles.entry((tx, ty))
            .or_insert_with(|| ImageBuffer::new(TILE_SIZE, TILE_SIZE));
            
        if let Some(color) = colors.get(&dot.category) {
             tile_img.put_pixel(px, py, *color);
        }
    }

    // Save tiles: output/tiles/{dataset_name}/{z}/{x}/{y}.png
    let z_dir = config.output.tile_dir.join(dataset_name).join(zoom.to_string());
    fs::create_dir_all(&z_dir).context("Failed to create zoom directory")?;

    local_tiles.par_iter().for_each(|((x, y), img)| {
         let x_dir = z_dir.join(x.to_string());
         if !x_dir.exists() {
             let _ = fs::create_dir_all(&x_dir);
         }
         let path = x_dir.join(format!("{}.png", y));
         
         if let Err(e) = img.save(&path) {
             eprintln!("Failed to save tile {:?}: {:?}", path, e);
         }
    });

    Ok(())
}


fn hex_to_rgba(hex: &str) -> Rgba<u8> {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    Rgba([r, g, b, 255])
}

// Coordinate conversions
fn lat_lon_to_tile_pixel(lat: f64, lon: f64, zoom: u8) -> (u32, u32, u32, u32) {
    let n = 2.0_f64.powi(zoom as i32); // Use powi for integer power
    let x_t = (lon + 180.0) / 360.0 * n;
    let lat_rad = lat.to_radians();
    let y_t = (1.0 - (lat_rad.tan() + (1.0 / lat_rad.cos())).ln() / PI) / 2.0 * n;
    
    let tx = x_t as u32;
    let ty = y_t as u32;
    
    let px = ((x_t - tx as f64) * TILE_SIZE as f64) as u32;
    let py = ((y_t - ty as f64) * TILE_SIZE as f64) as u32;
    
    (tx, ty, px, py)
}
