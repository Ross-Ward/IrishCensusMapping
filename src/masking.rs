use crate::types::SmallArea;
use anyhow::{Context, Result, anyhow};
use geo::{MultiPolygon, Polygon};
use geo::BooleanOps; // Try top level first, if fails try algorithm::
use geo::intersects::Intersects;
use geo::bounding_rect::BoundingRect;
use rstar::{RTree, RTreeObject, AABB};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use geojson::{GeoJson, Value};
use std::convert::TryInto;
use rayon::prelude::*;

// Wrapper for Polygon to implement RTreeObject if needed, 
// but let's see if we can use geo types directly or if we need a wrapper.
// geo 0.27 might not implement RTreeObject for Polygon by default without feature.
// Let's implement a wrapper struct for the RTree.

pub struct WaterPolygon(Polygon<f64>);

impl RTreeObject for WaterPolygon {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        let bbox = self.0.bounding_rect().unwrap();
        AABB::from_corners([bbox.min().x, bbox.min().y], [bbox.max().x, bbox.max().y])
    }
}

pub fn load_water_mask(path: &Path) -> Result<RTree<WaterPolygon>> {
    println!("Loading water mask from {:?}...", path);
    let file = File::open(path).with_context(|| format!("Failed to open water mask: {:?}", path))?;
    let reader = BufReader::new(file);
    let geojson = GeoJson::from_reader(reader).context("Failed to parse Water Mask GeoJSON")?;

    let collection = match geojson {
        GeoJson::FeatureCollection(fc) => fc,
        _ => return Err(anyhow!("Water mask must be a FeatureCollection")),
    };

    let mut polygons = Vec::new();

    for feature in collection.features {
        if let Some(geom) = feature.geometry {
             match geom.value {
                 Value::Polygon(_) | Value::MultiPolygon(_) => {
                     let geo_geom: geo::Geometry<f64> = geom.value.try_into()
                        .map_err(|e| anyhow!("Failed to convert geometry: {:?}", e))?;
                     
                     match geo_geom {
                         geo::Geometry::Polygon(p) => polygons.push(WaterPolygon(p)),
                         geo::Geometry::MultiPolygon(mp) => {
                             for p in mp {
                                 polygons.push(WaterPolygon(p));
                             }
                         },
                         _ => {},
                     }
                 },
                 _ => {},
             }
        }
    }

    println!("Building spatial index for {} water polygons...", polygons.len());
    let tree = RTree::bulk_load(polygons);
    Ok(tree)
}

pub fn mask_small_areas(areas: &mut Vec<SmallArea>, water_tree: &RTree<WaterPolygon>) {
    println!("Masking water from {} areas...", areas.len());

    areas.par_iter_mut().for_each(|area| {
        let area_bbox = area.geometry.bounding_rect().unwrap();
        let area_aabb = AABB::from_corners(
            [area_bbox.min().x, area_bbox.min().y], 
            [area_bbox.max().x, area_bbox.max().y]
        );

        // Find intersecting water polygons
        let water_candidates: Vec<&WaterPolygon> = water_tree.locate_in_envelope_intersecting(&area_aabb).collect();

        if water_candidates.is_empty() {
            return;
        }

        // Subtract each water polygon
        for water_poly in water_candidates {
            if area.geometry.intersects(&water_poly.0) {
                let water_mp = MultiPolygon::new(vec![water_poly.0.clone()]);
                let new_geo = area.geometry.difference(&water_mp);
                area.geometry = new_geo;
            }
        }
    });
}
