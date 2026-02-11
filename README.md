# Dot Density Map (Rust)

A high-performance, extensible Rust application to generate dot density maps from Census data and Shapefiles.

## Project Structure
- `src/`: Rust source code.
- `config.toml`: Configuration for inputs, styling, and outputs.
- `index.html`: Frontend to view the generated tiles.

## Prerequisites
1.  **Rust**: Install from [rustup.rs](https://rustup.rs).
2.  **Data**:
    -   **Polygon Data**: Shapefile (`.shp`, `.shx`, `.dbf`) containing the regions (e.g., CSO Small Areas).
    -   **Attribute Data**: CSV file containing the population counts per region.
    -   **Join Column**: A common column in both files (e.g., `GUID`).

## Setup
1.  Place your `.shp` and `.csv` files in the `data/` directory.
2.  Update `config.toml` to match your filenames and column names.
    -   `[input]`: Set paths and join columns.
    -   `[processing]`: Define categories and their corresponding CSV columns.

## Usage

### 1. Generate Tiles
Run the generation command. This reads the data, processes points, and renders tiles.
```
cargo run --release -- generate --config config.toml
```

### 2. Serve Map
Start the local server to view your map.
```
cargo run --release -- serve --config config.toml
```
Open [http://localhost:3000](http://localhost:3000) in your browser.

## Logic
-   **Not Stated Distribution**: The "Not Stated" population is distributed proportionally among the known categories within each region.
-   **Point Generation**: Points are randomly generated within the polygon geometry.
-   **Rendering**: Tiles are generated using Parallel processing for maximum speed.
