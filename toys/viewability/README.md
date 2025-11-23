# Viewability Simulation - Rust Implementation

This is a Rust rewrite of the Python viewability simulation application. It provides an interactive GUI for visualizing sigmoid-based probability calculations and CPM (Cost Per Mille) relationships.

## Features

- Interactive GTK4 GUI with parameter sliders
- Real-time chart updates
- Three visualization types:
  - Win probability curves (2D line chart)
  - CPM C surface (3D surface plot)
  - Weighted sum surface with isohypsis contours (3D surface plot)

## Requirements

- Rust (latest stable version)
- GTK4 development libraries
- pkg-config

### Installing GTK4 on Linux

```bash
# Ubuntu/Debian
sudo apt-get install libgtk-4-dev pkg-config

# Fedora
sudo dnf install gtk4-devel pkg-config

# Arch Linux
sudo pacman -S gtk4 pkg-config
```

## Building

```bash
cargo build --release
```

## Running

```bash
cargo run
```

Or run the release binary:

```bash
./target/release/viewability
```

## Project Structure

- `src/main.rs` - Main application code
- `Cargo.toml` - Rust project configuration and dependencies

## Dependencies

- `gtk4` - GTK4 GUI framework
- `plotters` - Plotting library for creating charts
- `plotters-bitmap` - Bitmap backend for plotters

## Notes

This implementation uses `plotters` for charting instead of matplotlib, as there is no direct Rust equivalent to Python's matplotlib. The 3D plotting capabilities are implemented using plotters' 3D support, which may have limitations compared to matplotlib's 3D plotting.

## Original Python Implementation

The original Python implementation used:
- GTK4 with Adwaita
- Matplotlib with GTK4 backend
- NumPy for numerical computations

The Rust version maintains the same functionality while leveraging Rust's performance and safety features.

