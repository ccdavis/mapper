# Mapper Project - Development Notes

## Important Build Configuration

### Terrain Algorithm Synchronization
**CRITICAL**: All build systems are configured to use the terrain generation versions (`mapper-terrain-cli` and `mapper-terrain-gui`) as the primary executables. When making improvements to the terrain generation algorithm, ensure all platforms receive the updates:

1. **Source Files**: The code is organized as a library (`src/lib.rs`) with two thin binaries:
   - `src/terrain_generator/` - Core terrain generation, split into focused modules:
     - `mod.rs` - `TerrainGenerator` struct and generation orchestration
     - `types.rs` - Data types (`TerrainMap`, `City`, `Road`, `GenerationSettings`, ...)
     - `elevation.rs` - Continent plans (soft blob masks) + domain-warped fBm elevation,
       histogram-equalized with a quantile sea level so `land_percentage` is exact
     - `climate.rs` - Moisture (noise + distance-to-ocean) and temperature fields
     - `biome.rs` - Biome classification (thresholds are area shares) and colors
     - `hydrology.rs` - Priority-flood pit filling, lakes, flow accumulation, river tracing
     - `settlements.rs` - City placement, A* road pathfinding, bridges
     - `labels.rs` / `names.rs` - Region labeling and procedural names
   - `src/terrain_renderer.rs` - Shared rendering module for both CLI and GUI
   - `src/main_terrain.rs` - CLI entry point with PNG export and command-line arguments
   - `src/main_gui_terrain.rs` - GUI entry point with Slint rendering and settings dialog

2. **Binary Names**: The build system automatically renames binaries:
   - `mapper-terrain-cli` → `mapper-cli` (or `.exe` on Windows)
   - `mapper-terrain-gui` → `mapper-gui` (or `.exe` on Windows)

3. **Build Targets**: All build scripts and CI/CD are configured to:
   - Build only the terrain versions by default
   - Package them with standard names for distribution
   - This includes: Makefile, GitHub Actions, and shell scripts

### Cross-Platform Build Commands

To ensure consistent builds across all platforms:

```bash
# Local development
make run-cli   # Runs mapper-terrain-cli
make run-gui   # Runs mapper-terrain-gui

# Cross-compilation
make windows   # Builds Windows executables
make macos     # Builds macOS binaries (on Mac only)

# Direct cargo commands (if needed)
cargo build --release --bin mapper-terrain-cli
cargo build --release --bin mapper-terrain-gui
```

### Testing After Algorithm Changes

After modifying the terrain generation algorithm:

1. Test locally with both CLI and GUI versions
2. Verify PNG export functionality in CLI
3. Test cross-compiled Windows builds if possible
4. Push to GitHub to trigger CI builds for all platforms

### Key Features to Maintain

The terrain generation system includes:
- Domain-warped fractal (fBm + ridged) elevation biased by per-seed continent plans
- Histogram-equalized elevations with a quantile sea level (land percentage is exact)
- Moisture from noise + distance-to-ocean; temperature from latitude + elevation
- Biome determination based on environmental factors (thresholds are area shares)
- Priority-flood hydrology: rivers always reach the sea, depressions become lakes,
  flow accumulation makes rivers join and widen downstream
- Procedural place name generation
- PNG export at configurable resolutions
- Smooth color gradients and hillshaded relief in rendering
- Configurable generation settings for river density, city density, and land percentage
- GUI settings dialog with visual feedback and sliders
- CLI command-line arguments for settings control

## Project Architecture

The project uses a modular architecture:
- All shared code lives in the `mapper` library (`src/lib.rs`); the binaries are thin wrappers
- Core terrain generation is separate from UI
- Shared rendering module (`terrain_renderer.rs`) eliminates code duplication
- Settings system (`GenerationSettings`) provides consistent configuration across versions
- Platform-specific build configurations handled by scripts
- GitHub Actions provides automated cross-platform builds

### Generation Settings

The `GenerationSettings` structure controls map generation with three parameters:
- `river_density` (0.0-1.0): Controls number of rivers (2-40 rivers)
- `city_density` (0.0-1.0): Controls number and size of cities
- `land_percentage` (0.0-1.0): Controls land/water ratio

#### GUI Settings Access
- File → Settings menu opens configuration dialog
- Real-time percentage display
- Reset to defaults button
- Settings apply to next generated map

#### CLI Settings Access
- Command-line arguments: `--rivers`, `--cities`, `--land` (each 0.0 to 1.0)
- `--seed <u32>` for reproducible maps, `--output <file>` for the PNG filename
- Any option switches to non-interactive quick mode; no options opens the menu
- Use `--help` for usage information
- Example: `./mapper-terrain-cli --rivers 0.8 --cities 0.3 --land 0.6 --seed 42 --output map.png`

## Future Improvements

When adding new features to terrain generation:
1. Update both CLI and GUI entry points if needed
2. Ensure PNG export handles any new visual elements
3. Test on all target platforms before release
4. Update this documentation with any new build requirements