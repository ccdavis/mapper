#!/bin/bash

# Build script for macOS binaries
# This script must be run on a macOS system

set -e

echo "======================================"
echo "macOS Build Script for Mapper"
echo "======================================"
echo ""

# Check if running on macOS
if [ "$(uname)" != "Darwin" ]; then
    echo "Error: This script must be run on macOS"
    echo ""
    echo "You're currently on: $(uname)"
    echo ""
    echo "To build macOS binaries from Linux:"
    echo "1. Use GitHub Actions (push to GitHub and let CI build)"
    echo "2. Use a macOS VM (check Apple's licensing)"
    echo "3. Use a cloud CI service"
    echo ""
    echo "Note: Cross-compiling to macOS from Linux is not officially"
    echo "supported due to Apple's licensing and toolchain requirements."
    exit 1
fi

# Check for Rust
if ! command -v cargo &> /dev/null; then
    echo "Rust not found. Installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo "✓ Rust installed"
else
    echo "✓ Rust already installed"
fi

# Detect system architecture
ARCH=$(uname -m)
echo "System architecture: $ARCH"

# Add both targets for universal binary
echo "Adding macOS targets..."
rustup target add x86_64-apple-darwin 2>/dev/null || true
rustup target add aarch64-apple-darwin 2>/dev/null || true
echo "✓ Targets added"

# Build selection menu
echo ""
echo "Select build type:"
echo "1) Universal binary (Intel + Apple Silicon) - Recommended"
echo "2) Intel only (x86_64)"
echo "3) Apple Silicon only (ARM64)"
echo "4) Current architecture only ($ARCH)"
echo ""
read -p "Enter choice (1-4): " choice

case $choice in
    1)
        echo ""
        echo "Building universal binary..."
        echo "=============================="
        
        # Build for Intel
        echo "Building for Intel Macs..."
        cargo build --target x86_64-apple-darwin --release --bin mapper-terrain-cli
        cargo build --target x86_64-apple-darwin --release --bin mapper-terrain-gui
        
        # Build for Apple Silicon
        echo "Building for Apple Silicon Macs..."
        cargo build --target aarch64-apple-darwin --release --bin mapper-terrain-cli
        cargo build --target aarch64-apple-darwin --release --bin mapper-terrain-gui
        
        # Create universal binaries
        echo "Creating universal binaries..."
        mkdir -p target/macos-release
        
        lipo -create \
            target/x86_64-apple-darwin/release/mapper-terrain-cli \
            target/aarch64-apple-darwin/release/mapper-terrain-cli \
            -output target/macos-release/mapper-cli
            
        lipo -create \
            target/x86_64-apple-darwin/release/mapper-terrain-gui \
            target/aarch64-apple-darwin/release/mapper-terrain-gui \
            -output target/macos-release/mapper-gui
            
        chmod +x target/macos-release/mapper-cli
        chmod +x target/macos-release/mapper-gui
        
        echo "✓ Universal binaries created"
        ;;
        
    2)
        echo ""
        echo "Building for Intel Macs only..."
        echo "================================"
        
        cargo build --target x86_64-apple-darwin --release --bin mapper-terrain-cli
        cargo build --target x86_64-apple-darwin --release --bin mapper-terrain-gui
        
        mkdir -p target/macos-release
        cp target/x86_64-apple-darwin/release/mapper-terrain-cli target/macos-release/mapper-cli
        cp target/x86_64-apple-darwin/release/mapper-terrain-gui target/macos-release/mapper-gui
        chmod +x target/macos-release/mapper-cli
        chmod +x target/macos-release/mapper-gui
        
        echo "✓ Intel binaries created"
        ;;
        
    3)
        echo ""
        echo "Building for Apple Silicon only..."
        echo "===================================="
        
        cargo build --target aarch64-apple-darwin --release --bin mapper-terrain-cli
        cargo build --target aarch64-apple-darwin --release --bin mapper-terrain-gui
        
        mkdir -p target/macos-release
        cp target/aarch64-apple-darwin/release/mapper-terrain-cli target/macos-release/mapper-cli
        cp target/aarch64-apple-darwin/release/mapper-terrain-gui target/macos-release/mapper-gui
        chmod +x target/macos-release/mapper-cli
        chmod +x target/macos-release/mapper-gui
        
        echo "✓ Apple Silicon binaries created"
        ;;
        
    4)
        echo ""
        echo "Building for current architecture ($ARCH)..."
        echo "=============================================="
        
        cargo build --release --bin mapper-terrain-cli
        cargo build --release --bin mapper-terrain-gui
        
        mkdir -p target/macos-release
        cp target/release/mapper-terrain-cli target/macos-release/mapper-cli
        cp target/release/mapper-terrain-gui target/macos-release/mapper-gui
        chmod +x target/macos-release/mapper-cli
        chmod +x target/macos-release/mapper-gui
        
        echo "✓ Native binaries created"
        ;;
        
    *)
        echo "Invalid choice"
        exit 1
        ;;
esac

# Create a simple launcher script
cat > target/macos-release/run-mapper.command << 'EOF'
#!/bin/bash
cd "$(dirname "$0")"
./mapper-gui
EOF
chmod +x target/macos-release/run-mapper.command

# Create README for macOS users
cat > target/macos-release/README-macOS.txt << 'EOF'
Mapper - Procedural Terrain Generator for macOS
============================================

This package contains:

1. mapper-cli - Command-line version
   - Run in Terminal
   - Interactive menu-based interface
   
2. mapper-gui - Graphical version
   - Double-click to run
   - Or use run-mapper.command
   
3. run-mapper.command - Launcher script
   - Double-click to launch the GUI

System Requirements:
- macOS 10.12 Sierra or later
- No additional runtime required

First Run Security:
If macOS blocks the app (unidentified developer):
1. Go to System Preferences → Security & Privacy
2. Click "Open Anyway" for mapper-gui
OR
Right-click the app and select "Open"

Usage:
- Double-click mapper-gui or run-mapper.command for the GUI
- Run ./mapper-cli in Terminal for the CLI version

Controls (GUI):
- File menu → Start: Generate a new map
- File menu → Exit: Close the application
- Help menu → About: Show application information

Controls (CLI):
- Press 1: Generate a new map
- Press 2: Show about information  
- Press 3: Exit

Universal Binary:
If you built a universal binary, it will run natively on both
Intel and Apple Silicon Macs.

Enjoy generating procedural maps!
EOF

echo ""
echo "======================================"
echo "✓ Build Complete!"
echo "======================================"
echo ""
echo "macOS binaries created in: target/macos-release/"
echo ""
ls -lh target/macos-release/
echo ""
echo "To run:"
echo "  GUI: ./target/macos-release/mapper-gui"
echo "  CLI: ./target/macos-release/mapper-cli"
echo ""
echo "To distribute, compress the folder:"
echo "  tar czf mapper-macos.tar.gz -C target/macos-release ."