#!/bin/bash

# Cross-compilation script for building Windows binaries on Linux

set -e

echo "======================================"
echo "Windows Cross-Compilation Build Script"
echo "======================================"
echo ""

# Check if mingw-w64 is installed
if ! command -v x86_64-w64-mingw32-gcc &> /dev/null; then
    echo "MinGW-w64 not found. Installing..."
    
    if [ -f /etc/debian_version ]; then
        # Debian/Ubuntu
        echo "Detected Debian/Ubuntu system"
        sudo apt update
        sudo apt install -y mingw-w64
    elif [ -f /etc/fedora-release ]; then
        # Fedora
        echo "Detected Fedora system"
        sudo dnf install -y mingw64-gcc mingw64-gcc-c++
    elif [ -f /etc/arch-release ]; then
        # Arch Linux
        echo "Detected Arch Linux system"
        sudo pacman -S --needed --noconfirm mingw-w64-gcc
    else
        echo "Error: Unable to auto-install mingw-w64 on this system."
        echo "Please install mingw-w64 manually and run this script again."
        echo ""
        echo "For Ubuntu/Debian: sudo apt install mingw-w64"
        echo "For Fedora: sudo dnf install mingw64-gcc mingw64-gcc-c++"
        echo "For Arch: sudo pacman -S mingw-w64-gcc"
        exit 1
    fi
    
    echo "✓ MinGW-w64 installed"
fi

# Check if Windows target is added
if ! rustup target list --installed | grep -q "x86_64-pc-windows-gnu"; then
    echo "Adding Windows target to Rust..."
    rustup target add x86_64-pc-windows-gnu
    echo "✓ Windows target added"
else
    echo "✓ Windows target already installed"
fi

# Create .cargo/config.toml for cross-compilation
mkdir -p .cargo
cat > .cargo/config.toml << 'EOF'
[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"
ar = "x86_64-w64-mingw32-ar"

[profile.release-windows]
inherits = "release"
strip = true
lto = true
codegen-units = 1
opt-level = "z"
EOF

echo "✓ Cargo configuration created"

# Build the Windows binaries
echo ""
echo "Building Windows binaries..."
echo "=============================="

# Build terrain CLI version
echo "Building mapper-terrain-cli for Windows..."
cargo build --target x86_64-pc-windows-gnu --release --bin mapper-terrain-cli
echo "✓ Terrain CLI version built"

# Build terrain GUI version
echo "Building mapper-terrain-gui for Windows..."
cargo build --target x86_64-pc-windows-gnu --release --bin mapper-terrain-gui
echo "✓ Terrain GUI version built"

# Create output directory
OUTPUT_DIR="target/windows-release"
mkdir -p "$OUTPUT_DIR"

# Copy binaries to output directory with standard names
cp target/x86_64-pc-windows-gnu/release/mapper-terrain-cli.exe "$OUTPUT_DIR/mapper-cli.exe"
cp target/x86_64-pc-windows-gnu/release/mapper-terrain-gui.exe "$OUTPUT_DIR/mapper-gui.exe"

# Create a simple batch file to run the GUI
cat > "$OUTPUT_DIR/run-mapper.bat" << 'EOF'
@echo off
echo Starting Mapper GUI...
start mapper-gui.exe
EOF

# Create README for Windows users
cat > "$OUTPUT_DIR/README-Windows.txt" << 'EOF'
Mapper - Procedural Terrain Generator for Windows
==================================================

1. mapper-cli.exe - Command-line version
   - Perlin noise-based terrain generation
   - Multiple biomes (ocean, forest, mountains, etc.)
   - River generation with water flow
   - Procedural place names
   - Auto-saves high-resolution PNG images
   - Run in Command Prompt or PowerShell
   
2. mapper-gui.exe - Graphical version
   - Same features as CLI but with graphical display
   - Smooth color gradients
   - Double-click to run
   - Or use run-mapper.bat
   
System Requirements:
- Windows 7 or later (64-bit)
- No additional runtime required

Usage:
- Double-click mapper-gui.exe for the graphical version
- Run mapper-cli.exe in a terminal for the CLI version

Controls (GUI):
- File menu → Start: Generate a new map
- File menu → Exit: Close the application
- Help menu → About: Show application information

Controls (CLI):
- Press 1: Generate a new map
- Press 2: Show about information  
- Press 3: Exit

Enjoy generating procedural maps!
EOF

echo ""
echo "======================================"
echo "✓ Build Complete!"
echo "======================================"
echo ""
echo "Windows binaries created in: $OUTPUT_DIR"
echo ""
ls -lh "$OUTPUT_DIR"
echo ""
echo "You can now copy the $OUTPUT_DIR folder to a Windows machine."
echo "The executables are standalone and don't require any runtime dependencies."