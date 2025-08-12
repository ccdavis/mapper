# Mapper Project - Development Notes

## Important Build Configuration

### Terrain Algorithm Synchronization
**CRITICAL**: All build systems are configured to use the terrain generation versions (`mapper-terrain-cli` and `mapper-terrain-gui`) as the primary executables. When making improvements to the terrain generation algorithm, ensure all platforms receive the updates:

1. **Source Files**: The terrain algorithm is implemented in:
   - `src/terrain_generator.rs` - Core terrain generation logic with configurable settings
   - `src/terrain_renderer.rs` - Shared rendering module for both CLI and GUI
   - `src/main_terrain.rs` - CLI entry point with PNG export and command-line arguments
   - `src/main_terrain_gui.rs` - GUI entry point with Slint rendering and settings dialog

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
- Perlin noise-based elevation, moisture, and temperature maps
- Biome determination based on environmental factors
- River generation with water flow simulation
- Procedural place name generation
- PNG export at configurable resolutions
- Smooth color gradients in GUI rendering
- Configurable generation settings for river density, city density, and land percentage
- GUI settings dialog with visual feedback and sliders
- CLI command-line arguments for settings control

## Project Architecture

The project uses a modular architecture:
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
- Command-line arguments: `--rivers`, `--cities`, `--land`
- Each accepts values from 0.0 to 1.0
- Use `--help` for usage information
- Example: `./mapper-terrain-cli --rivers 0.8 --cities 0.3 --land 0.6`

## Future Improvements

When adding new features to terrain generation:
1. Update both CLI and GUI entry points if needed
2. Ensure PNG export handles any new visual elements
3. Test on all target platforms before release
4. Update this documentation with any new build requirements