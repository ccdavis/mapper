# Mapper - Procedural Map Generator

A Rust application for generating and displaying procedural maps with both CLI and GUI interfaces using the Slint framework.

This is my first true vibe-coded app. It's mainly meant to show how to set up a Rust Slint UI application that can build for multiple platforms at once. I've tested on Linux and Windows. The map generation was just the only interesting thing I could think to put into a GUI application that wouldn't get too complicated.

The maps have labels (testing the cross platform fonts), roads, rivers, lakes, oceans and cities with populations. The terrain is generated with domain-warped fractal noise shaped by per-seed continent plans, plus a proper hydrology pass so rivers always reach the sea.

## Features

- **Realistic Terrain Generation**:
  - Domain-warped fractal (fBm + ridged) elevation biased by per-seed continent plans
  - Histogram-equalized elevations with a quantile sea level, so the land percentage setting is exact
  - Moisture from noise + distance-to-ocean; temperature from latitude + elevation
  - Biome classification (ocean, mountains, forest, swamp, desert, ...) based on environmental factors
  - Priority-flood hydrology: rivers always reach the sea, depressions become lakes, flow accumulation makes rivers join and widen downstream
  - City placement with A* road pathfinding and bridges
  - Procedural place names and region labels
- **Configurable Generation Settings**: river density, city density, and land percentage
- **Rendering**: smooth color gradients and hillshaded relief, shared between CLI and GUI
- **Dual Interface**:
  - CLI version with ASCII preview, PNG export, and command-line arguments
  - GUI version with graphical map display and a settings dialog

## Project Structure

The code is organized as a library (`src/lib.rs`) with two thin binaries:

```
mapper/
├── src/
│   ├── lib.rs                   # Library root (all shared code)
│   ├── terrain_generator/       # Core terrain generation
│   │   ├── mod.rs               # TerrainGenerator struct and orchestration
│   │   ├── types.rs             # Data types (TerrainMap, City, Road, GenerationSettings, ...)
│   │   ├── elevation.rs         # Continent plans + domain-warped fBm elevation
│   │   ├── climate.rs           # Moisture and temperature fields
│   │   ├── biome.rs             # Biome classification and colors
│   │   ├── hydrology.rs         # Pit filling, lakes, flow accumulation, river tracing
│   │   ├── settlements.rs       # City placement, A* road pathfinding, bridges
│   │   ├── labels.rs            # Region labeling
│   │   └── names.rs             # Procedural name generation
│   ├── terrain_renderer.rs      # Shared rendering for CLI and GUI
│   ├── main_terrain.rs          # CLI entry point (mapper-terrain-cli)
│   └── main_gui_terrain.rs      # GUI entry point (mapper-terrain-gui)
├── ui/
│   └── mapper.slint             # Slint UI definition
├── build.rs                     # Build script for Slint
└── Cargo.toml                   # Dependencies
```

## Building and Running

### Prerequisites

Slint has minimal dependencies and should work on most systems with Rust installed.

```bash
# Install Rust if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Build

```bash
# Build both CLI and GUI versions
cargo build

# Build specific version
cargo build --bin mapper-terrain-cli
cargo build --bin mapper-terrain-gui
```

Note: the packaging scripts and CI rename the binaries for distribution —
`mapper-terrain-cli` ships as `mapper-cli` and `mapper-terrain-gui` ships as
`mapper-gui` (with `.exe` on Windows).

### Run

#### CLI Version
```bash
cargo run --bin mapper-terrain-cli
# or
make run-cli
```

With no arguments, the CLI presents an interactive menu:
- `1` Generate a new terrain map
- `2` Generate with a custom seed
- `3` About
- `4` Exit

Generated maps are shown as an ASCII preview and exported as PNG.

Passing any option switches to non-interactive quick mode:

```bash
mapper-terrain-cli --rivers 0.8 --cities 0.3 --land 0.6 --seed 42 --output map.png
```

| Option | Description |
|--------|-------------|
| `--rivers <0.0-1.0>` | River density (default: 0.5) |
| `--cities <0.0-1.0>` | City density (default: 0.5) |
| `--land <0.0-1.0>` | Land percentage (default: 0.4) |
| `--seed <u32>` | Seed for reproducible maps (default: current time) |
| `--output <file>` | Output PNG filename (default: `terrain_map_<seed>.png`) |
| `--help` | Show usage information |

#### GUI Version
```bash
cargo run --bin mapper-terrain-gui
# or
make run-gui
```

The GUI version provides:
- Menu bar with File and Help menus
- Visual map display with hillshaded terrain rendering
- File → Settings dialog with sliders for river density, city density, and
  land percentage (with real-time percentage display and a reset-to-defaults
  button); settings apply to the next generated map

## Testing

```bash
# Run all tests
cargo test
```

## Map Generation

Generation runs as a pipeline over a tile grid:

1. **Continent plans**: each seed lays out soft blob masks that decide where landmasses go
2. **Elevation**: domain-warped fractal noise (fBm + ridged) biased by the continent plan, then histogram-equalized with a quantile sea level so the requested land percentage is exact
3. **Climate**: moisture from noise + distance-to-ocean, temperature from latitude + elevation
4. **Biomes**: classified from elevation, moisture, and temperature (thresholds are area shares)
5. **Hydrology**: priority-flood pit filling guarantees drainage, depressions become lakes, and flow accumulation traces rivers that join and widen on their way to the sea
6. **Settlements**: cities are placed at favorable sites and connected by A* roads, with bridges where roads cross rivers
7. **Names and labels**: procedurally generated names for cities and regions

The same `GenerationSettings` (river density, city density, land percentage) drive both the CLI and GUI, and a given seed always reproduces the same map.

## Development

To extend the map generation:
1. Modify the relevant module under `src/terrain_generator/` (e.g. `biome.rs` for new biomes, `hydrology.rs` for water features)
2. Update `src/terrain_renderer.rs` if new features need to be drawn (both CLI PNG export and GUI use it)
3. Add corresponding tests

## Cross-Compilation

### Building for Windows (from Linux/macOS)

You can build Windows executables from Linux or macOS using cross-compilation:

### Method 1: Using MinGW-w64 (Traditional)

```bash
# Install MinGW-w64
sudo apt install mingw-w64  # Ubuntu/Debian
sudo dnf install mingw64-gcc  # Fedora
sudo pacman -S mingw-w64-gcc  # Arch

# Build Windows binaries
./build-windows.sh
# or
make windows-gnu
```

### Method 2: Using cargo-xwin (No system dependencies)

```bash
# Install cargo-xwin
cargo install cargo-xwin

# Build Windows binaries
./build-windows-xwin.sh
# or
make windows-msvc
```

### Method 3: Automatic (uses available tools)

```bash
# Automatically detects and uses available cross-compilation tools
make windows
```

Windows binaries will be created in `target/windows-release/`:
- `mapper-cli.exe` - Command-line version
- `mapper-gui.exe` - Graphical version

### Building for macOS

**Important:** macOS cross-compilation from Linux is **not supported** due to Apple's licensing restrictions and toolchain requirements.

#### On macOS (Native Build)

```bash
# Build universal binary (Intel + Apple Silicon)
./build-macos.sh
# or
make macos-universal

# Build for specific architecture
make macos-x86   # Intel Macs only
make macos-arm   # Apple Silicon only
```

macOS binaries will be created in `target/macos-release/`:
- `mapper-cli` - Command-line version
- `mapper-gui` - Graphical version
- `run-mapper.command` - Double-clickable launcher

#### From Linux (Not Supported)

```bash
make macos  # Will show helpful error message with alternatives
```

**Alternatives for building macOS binaries from Linux:**

1. **GitHub Actions** (Recommended)
   - Push to GitHub and let CI build automatically
   - See `.github/workflows/build.yml`

2. **Cloud CI Services**
   - GitHub Actions (free for public repos)
   - Azure Pipelines
   - CircleCI

3. **macOS VM**
   - Check Apple's licensing terms
   - Requires macOS installation media

4. **Remote Mac**
   - Mac mini in the cloud
   - MacStadium, MacinCloud, etc.

## Makefile Targets

```bash
make build         # Build debug versions
make build-release # Build release versions
make test          # Run tests
make clean         # Clean build artifacts
make run-cli       # Run CLI version
make run-gui       # Run GUI version
make windows       # Cross-compile for Windows
make macos         # Build for macOS (requires Mac)
make macos-universal # Universal binary (Intel + ARM)
make macos-x86     # Intel Macs only
make macos-arm     # Apple Silicon only
make help          # Show all targets
```

## Dependencies

- `slint` - Native GUI framework for Rust
- `noise` - Perlin noise for terrain generation
- `rand` / `rand_chacha` - Seeded random number generation
- `image` / `imageproc` - PNG export and drawing
- `rusttype` - Font rendering for map labels
- `serde` / `serde_json` - Serialization framework
- No runtime dependencies required!

### Cross-Compilation Dependencies (Optional)

For Windows cross-compilation, choose one:
- **MinGW-w64**: Traditional cross-compiler (requires system package)
- **cargo-xwin**: Rust-based solution (no system packages needed)

## Automated Builds

The project includes GitHub Actions workflow (`.github/workflows/build.yml`) that automatically:
- Builds binaries for Linux, Windows, and macOS
- Cross-compiles Windows binaries from Linux
- Creates releases when you push tags (e.g., `v1.0.0`)
- Uploads artifacts for each platform

## Files Created for Cross-Compilation

### Windows Build Files
- `build-windows.sh` - Build script using MinGW-w64
- `build-windows-xwin.sh` - Build script using cargo-xwin
- `Cross.toml` - Configuration for cargo-cross tool

### macOS Build Files
- `build-macos.sh` - Native build script for macOS
- Creates universal binaries (Intel + Apple Silicon)

### General Build Files
- `Makefile` - Convenient build targets for all platforms
- `.github/workflows/build.yml` - CI/CD pipeline for automated builds

## Platform Support Summary

| Platform | Build From | Method | Notes |
|----------|------------|--------|-------|
| **Linux** | Linux | Native | `cargo build` |
| **Windows** | Linux | Cross-compile | MinGW-w64 or cargo-xwin |
| **Windows** | Windows | Native | Visual Studio or MinGW |
| **macOS** | macOS | Native | `cargo build` or `make macos` |
| **macOS** | Linux | ❌ Not Supported | Use GitHub Actions instead |

All executables are standalone and don't require runtime dependencies!
