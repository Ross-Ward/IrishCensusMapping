use crate::config::AppConfig;
use crate::types::{Dot, SmallArea};
use geo::algorithm::bounding_rect::BoundingRect;
use geo::algorithm::contains::Contains;
use geo::{MultiPolygon, Point, Rect};
use rand::Rng;
use rayon::prelude::*;
use std::collections::HashMap;
use rand::seq::SliceRandom;

pub fn process_data(config: &AppConfig, small_areas: Vec<SmallArea>) -> Vec<Dot> {
    println!("Processing {} areas to generate dots...", small_areas.len());

    let dots: Vec<Dot> = small_areas.par_iter().flat_map(|area| {
        generate_dots_for_area(config, area)
    }).collect();

    println!("Generated {} dots total.", dots.len());
    
    // Shuffle is implicit if we want random rendering order, but since we tile them later,
    // the order within the tile matters if they overlap.
    // Use rand::shuffle if needed, but for now simple collection is fine.
    // Actually, to avoid "z-ordering" bias where one race is always on top, we SHOULD shuffle.
    let mut rng = rand::thread_rng();
    let mut dots = dots;
    use rand::seq::SliceRandom;
    dots.shuffle(&mut rng);

    dots
}

fn generate_dots_for_area(config: &AppConfig, area: &SmallArea) -> Vec<Dot> {
    let mut area_dots = Vec::new();
    let bbox = area.geometry.bounding_rect().unwrap();

    for (dataset_name, dataset_config) in &config.processing.datasets {
        // Get population data for this dataset
        let pop_data = match area.population_data.get(dataset_name) {
            Some(d) => d,
            None => continue,
        };

        // 1. Calculate Distributable "Total" for Not Stated redistribution
        let not_stated_count = if let Some(_ns_config) = &dataset_config.not_stated {
             *pop_data.get("Not Stated").unwrap_or(&0) as f64
        } else {
            0.0
        };
        
        let mut total_known = 0.0;
        for cat in &dataset_config.categories {
            total_known += *pop_data.get(&cat.name).unwrap_or(&0) as f64;
        }

        // If total known is 0, we can't distribute not stated properly, so just define 0.
        // Or if we have no Not Stated config, we just ignore redistribution.
        
        for cat in &dataset_config.categories {
            let known_count = *pop_data.get(&cat.name).unwrap_or(&0) as f64;
            
            let final_count = if total_known > 0.0 {
                let proportion = known_count / total_known;
                let additional = proportion * not_stated_count;
                (known_count + additional).round() as u32
            } else {
                known_count as u32
            };

            for _ in 0..final_count {
                if let Some(pt) = generate_random_point_in_poly(&area.geometry, &bbox) {
                     area_dots.push(Dot {
                         point: pt,
                         dataset: dataset_name.clone(),
                         category: cat.name.clone(),
                     });
                }
            }
        }
    }

    area_dots
}


fn generate_random_point_in_poly(poly: &MultiPolygon<f64>, bbox: &Rect<f64>) -> Option<Point<f64>> {
    let mut rng = rand::thread_rng();
    // Simple rejection sampling
    // Try 100 times then give up (corner case: very thin polygons)
    for _ in 0..100 {
        let x = rng.gen_range(bbox.min().x..bbox.max().x);
        let y = rng.gen_range(bbox.min().y..bbox.max().y);
        let pt = Point::new(x, y);
        if poly.contains(&pt) {
            return Some(pt);
        }
    }
    None
}
