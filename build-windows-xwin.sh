#!/bin/bash

# Cross-compilation script using cargo-xwin (no mingw-w64 required)
# This uses Microsoft's C++ Build Tools redistributables

set -e

echo "=============================================="
echo "Windows Cross-Compilation Build Script (xwin)"
echo "=============================================="
echo ""

# Check if cargo-xwin is installed
if ! command -v cargo-xwin &> /dev/null; then
    echo "Installing cargo-xwin..."
    cargo install cargo-xwin
    echo "✓ cargo-xwin installed"
else
    echo "✓ cargo-xwin already installed"
fi

# Add Windows MSVC target
if ! rustup target list --installed | grep -q "x86_64-pc-windows-msvc"; then
    echo "Adding Windows MSVC target to Rust..."
    rustup target add x86_64-pc-windows-msvc
    echo "✓ Windows MSVC target added"
else
    echo "✓ Windows MSVC target already installed"
fi

# Build the Windows binaries
echo ""
echo "Building Windows binaries with xwin..."
echo "======================================="
echo "Note: First run will download Windows SDK files (~300MB)"
echo ""

# Build terrain CLI version
echo "Building mapper-terrain-cli for Windows..."
cargo xwin build --target x86_64-pc-windows-msvc --release --bin mapper-terrain-cli
echo "✓ Terrain CLI version built"

# Build terrain GUI version
echo "Building mapper-terrain-gui for Windows..."
cargo xwin build --target x86_64-pc-windows-msvc --release --bin mapper-terrain-gui
echo "✓ Terrain GUI version built"

# Create output directory
OUTPUT_DIR="target/windows-release"
mkdir -p "$OUTPUT_DIR"

# Copy binaries to output directory with standard names
cp target/x86_64-pc-windows-msvc/release/mapper-terrain-cli.exe "$OUTPUT_DIR/mapper-cli.exe"
cp target/x86_64-pc-windows-msvc/release/mapper-terrain-gui.exe "$OUTPUT_DIR/mapper-gui.exe"

# Create a simple batch file to run the GUI
cat > "$OUTPUT_DIR/run-mapper.bat" << 'EOF'
@echo off
echo Starting Mapper GUI...
start mapper-gui.exe
EOF

# Create README for Windows users
cat > "$OUTPUT_DIR/README-Windows.txt" << 'EOF'
Mapper - Procedural Terrain Generator for Windows
==============================================

This package contains two executables:

1. mapper-cli.exe - Command-line version
   - Run in Command Prompt or PowerShell
   - Interactive menu-based interface
   
2. mapper-gui.exe - Graphical version
   - Double-click to run
   - Or use run-mapper.bat
   
System Requirements:
- Windows 10 or later (64-bit)
- Visual C++ Redistributables may be required
  (usually already installed on most Windows systems)

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
ls -lh "$OUTPUT_DIR" 2>/dev/null || echo "Build directory will be created when build succeeds"
echo ""
echo "You can now copy the $OUTPUT_DIR folder to a Windows machine."