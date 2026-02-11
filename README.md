# Irish Census Mapping

A high-performance Rust application that generates interactive dot density maps from Irish Census 2022 data.

## Features

- **Multi-Dataset Support** — Visualise Ethnicity, Religion, and Health data as separate layers
- **Water Masking** — Removes water bodies from land areas so dots only appear on land
- **Interactive API** — Hover over any area to see a full demographic breakdown
- **Fast** — Parallel tile rendering with Rayon (processes 15M+ dots)
- **Configurable** — Add new Census themes by editing `config.toml`

## Prerequisites

1. **Rust** — Install from [rustup.rs](https://rustup.rs)
2. **Data** (not included in repo, too large):
   - **GeoJSON boundaries** — CSO Small Area boundaries (ungeneralised)
   - **SAPS CSV** — Small Area Population Statistics from [CSO](https://www.cso.ie)
   - **Water mask GeoJSON** — High Water Mark boundaries (optional)

## Setup

1. Create a `data/` directory and place your data files there.
2. Edit `config.toml` to match your filenames, join columns, and desired categories.

## Usage

### Generate Tiles
```
cargo run --release -- generate
```
Processes data and renders PNG tiles into `output/tiles/{Dataset}/{z}/{x}/{y}.png`.

### Serve Map
```
cargo run --release -- serve
```
Opens an interactive map at [http://localhost:3000](http://localhost:3000) with:
- Layer switcher (Ethnicity / Religion / Health)
- Dynamic legend
- Hover info panel with per-area breakdowns

## Architecture

| Module | Role |
|---|---|
| `config.rs` | TOML configuration parsing |
| `data.rs` | CSV + GeoJSON loading and joining |
| `masking.rs` | Water body subtraction using R-tree spatial index |
| `processing.rs` | Dot generation with "Not Stated" proportional distribution |
| `render.rs` | Parallel Web Mercator tile rendering |
| `server.rs` | Axum web server with spatial query API |

## Configuration

Datasets are defined in `config.toml` under `[processing.datasets]`. Each dataset has:
- **categories** — name, color, and CSV column(s)
- **not_stated** — column for proportional redistribution

## License

MIT
