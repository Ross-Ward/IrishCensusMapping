pub mod types;
pub mod config;
pub mod data;
pub mod processing;
pub mod render;
pub mod server;
pub mod masking;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate the dot density map tiles
    Generate {
        #[arg(short, long, value_name = "FILE", default_value = "config.toml")]
        config: PathBuf,
    },
    /// Serve the generated map
    Serve {
        #[arg(short, long, value_name = "FILE", default_value = "config.toml")]
        config: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();

    match &cli.command {
        Commands::Generate { config } => {
            println!("Generating map with config: {:?}", config);
            let app_config = config::AppConfig::load_from_file(config)?;
            
            // 1. Load Data
            let mut small_areas = data::load_data(&app_config)?;
            
            // 1b. Load and Apply Water Mask (if configured)
            if let Some(mask_path) = &app_config.input.water_mask {
                println!("Water masking enabled.");
                let water_tree = masking::load_water_mask(mask_path)?;
                masking::mask_small_areas(&mut small_areas, &water_tree);
            }
            
            // 2. Process Data
            let dots = processing::process_data(&app_config, small_areas);
            
            // 3. Render Tiles
            render::generate_tiles(&app_config, dots)?;
            
            println!("Generation complete!");
        }
        Commands::Serve { config } => {
            println!("Serving map with config: {:?}", config);
            let app_config = config::AppConfig::load_from_file(config)?;
            
            // Load data for API interactivity
            println!("Loading data for API...");
            // We don't strictly need water masking for the API lookup (or maybe we do?)
            // If we want hover to work only on land, we should mask. 
            // But usually loading raw areas is faster and fine for "what is this area".
            let small_areas = data::load_data(&app_config)?;
            
            server::start_server(app_config, small_areas).await?;
        }
    }

    Ok(())
}

