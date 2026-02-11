use crate::config::AppConfig;
use crate::types::SmallArea;
use anyhow::{Context, Result};
use axum::{
    extract::{Query, State},
    response::Json,
    routing::get,
    Router,
};
use geo::algorithm::contains::Contains;
use geo::{Point, Rect};
use rstar::{RTree, RTreeObject, AABB};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::services::ServeDir;
use tower_http::cors::CorsLayer;

// Wrapper for RTree indexing
struct AreaIndex {
    index: usize,
    aabb: AABB<[f64; 2]>,
}

impl RTreeObject for AreaIndex {
    type Envelope = AABB<[f64; 2]>;
    fn envelope(&self) -> Self::Envelope {
        self.aabb
    }
}

pub struct AppState {
    pub areas: Vec<SmallArea>,
    pub tree: RTree<AreaIndex>,
    pub config: AppConfig,
}

#[derive(Deserialize)]
pub struct QueryParams {
    lat: f64,
    lon: f64,
}

#[derive(Serialize)]
pub struct QueryResponse {
    id: String,
    population_data: HashMap<String, HashMap<String, u32>>,
}

pub async fn start_server(config: AppConfig, areas: Vec<SmallArea>) -> Result<()> {
    // Build Spatial Index
    println!("Building spatial index for API...");
    let tree_items: Vec<AreaIndex> = areas.iter().enumerate().map(|(i, area)| {
        use geo::bounding_rect::BoundingRect;
        let rect = area.geometry.bounding_rect().unwrap_or(
            Rect::new(
                geo::Coord { x: 0.0, y: 0.0 }, 
                geo::Coord { x: 0.0, y: 0.0 }
            )
        );
        AreaIndex {
            index: i,
            aabb: AABB::from_corners([rect.min().x, rect.min().y], [rect.max().x, rect.max().y]),
        }
    }).collect();

    let tree = RTree::bulk_load(tree_items);
    println!("Spatial index built.");

    let state = Arc::new(AppState {
        areas,
        tree,
        config: config.clone(),
    });

    let port = config.server.port;
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    
    println!("Starting server on http://{}", addr);

    let tile_service = ServeDir::new(&config.output.tile_dir);
    
    let app = Router::new()
        .route("/api/query", get(query_handler))
        .nest_service("/tiles", tile_service)
        .nest_service("/", ServeDir::new("."))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn query_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<QueryParams>,
) -> Json<Option<QueryResponse>> {
    let point = Point::new(params.lon, params.lat);
    let envelope = AABB::from_point([params.lon, params.lat]);

    // Query RTree
    let candidates = state.tree.locate_in_envelope_intersecting(&envelope);

    for candidate in candidates {
        if let Some(area) = state.areas.get(candidate.index) {
             if area.geometry.contains(&point) {
                 return Json(Some(QueryResponse {
                     id: area.id.clone(),
                     population_data: area.population_data.clone(),
                 }));
             }
        }
    }

    Json(None)
}
