# Mapper - Procedural Map Generator

A Rust application for generating and displaying procedural maps with both CLI and GUI interfaces using the Slint framework.

## Features

- **File Menu**: Start map generation and Exit options
- **Help Menu**: About dialog  
- **Map Generation**: Procedurally generates terrain maps with different tile types:
  - Water (blue)
  - Grass (green)
  - Dirt (brown)
  - Stone (gray)
  - Sand (tan)
- **Dual Interface**: 
  - CLI version with ASCII display
  - GUI version with graphical map display

## Project Structure

```
mapper/
├── src/
│   ├── main.rs              # CLI version
│   ├── main_gui.rs          # Slint GUI version
│   └── map_generator.rs     # Map generation logic
├── ui/
│   └── mapper.slint         # Slint UI definition
├── build.rs                 # Build script for Slint
└── Cargo.toml              # Dependencies
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
cargo build --bin mapper-cli
cargo build --bin mapper-gui
```

### Run

#### CLI Version
```bash
cargo run --bin mapper-cli
# or
./target/debug/mapper-cli
```

The CLI version presents a menu:
- Press `1` to generate a new map
- Press `2` to show about information
- Press `3` to exit

Maps are displayed using ASCII characters:
- `~` Water
- `.` Grass
- `#` Dirt
- `^` Stone
- `s` Sand

#### GUI Version
```bash
cargo run --bin mapper-gui
# or
./target/debug/mapper-gui
```

The GUI version provides:
- Menu bar with File and Help menus
- Visual map display with colored tiles
- 1024x768 window with map display area

## Testing

```bash
# Run all tests
cargo test

# Run tests for specific binary
cargo test --bin mapper-cli
cargo test --bin mapper-gui
```

## Map Generation

The application generates random terrain maps using a simple random algorithm. Each tile is assigned a terrain type based on random values:
- < 20% → Water
- 20-50% → Grass  
- 50-70% → Dirt
- 70-85% → Stone
- > 85% → Sand

Maps are 40x30 tiles in the GUI version and 60x20 tiles in the CLI version.

## Development

To extend the map generation:
1. Modify `src/map_generator.rs` to add new tile types or generation algorithms
2. Update display functions in `main.rs` (CLI) and `main_gui.rs` (GUI)
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

- `serde` - Serialization framework
- `slint` - Native GUI framework for Rust
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