.PHONY: all build build-release test clean run-cli run-gui windows windows-gnu windows-msvc macos macos-universal macos-x86 macos-arm help

# Default target
all: build

# Build debug versions
build:
	cargo build --all

# Build release versions
build-release:
	cargo build --release --all

# Run tests
test:
	cargo test --all

# Clean build artifacts
clean:
	cargo clean
	rm -rf target/windows-release
	rm -rf target/macos-release

# Run CLI version
run-cli:
	cargo run --bin mapper-terrain-cli

# Run GUI version
run-gui:
	cargo run --bin mapper-terrain-gui

# Build for Windows (tries multiple methods)
windows: windows-check

windows-check:
	@echo "Checking available Windows cross-compilation methods..."
	@if command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then \
		echo "Found mingw-w64, using GNU target..."; \
		$(MAKE) windows-gnu; \
	elif command -v cargo-xwin >/dev/null 2>&1; then \
		echo "Found cargo-xwin, using MSVC target..."; \
		$(MAKE) windows-msvc; \
	else \
		echo "No Windows cross-compilation tools found."; \
		echo ""; \
		echo "Option 1: Install mingw-w64"; \
		echo "  Ubuntu/Debian: sudo apt install mingw-w64"; \
		echo "  Fedora: sudo dnf install mingw64-gcc"; \
		echo "  Arch: sudo pacman -S mingw-w64-gcc"; \
		echo ""; \
		echo "Option 2: Install cargo-xwin (no system dependencies)"; \
		echo "  cargo install cargo-xwin"; \
		echo ""; \
		echo "Then run 'make windows' again."; \
		exit 1; \
	fi

# Build for Windows using MinGW
windows-gnu:
	@echo "Building for Windows (GNU target)..."
	@rustup target add x86_64-pc-windows-gnu 2>/dev/null || true
	@echo "Building terrain versions..."
	cargo build --target x86_64-pc-windows-gnu --release --bin mapper-terrain-cli
	cargo build --target x86_64-pc-windows-gnu --release --bin mapper-terrain-gui
	@mkdir -p target/windows-release
	@cp target/x86_64-pc-windows-gnu/release/mapper-terrain-cli.exe target/windows-release/mapper-cli.exe 2>/dev/null || true
	@cp target/x86_64-pc-windows-gnu/release/mapper-terrain-gui.exe target/windows-release/mapper-gui.exe 2>/dev/null || true
	@echo "Windows binaries created in target/windows-release/"
	@echo "  mapper-cli.exe - Terrain CLI with PNG export"
	@echo "  mapper-gui.exe - Terrain GUI with smooth rendering"

# Build for Windows using cargo-xwin
windows-msvc:
	@echo "Building for Windows (MSVC target)..."
	@rustup target add x86_64-pc-windows-msvc 2>/dev/null || true
	@echo "Building terrain versions..."
	cargo xwin build --target x86_64-pc-windows-msvc --release --bin mapper-terrain-cli
	cargo xwin build --target x86_64-pc-windows-msvc --release --bin mapper-terrain-gui
	@mkdir -p target/windows-release
	@cp target/x86_64-pc-windows-msvc/release/mapper-terrain-cli.exe target/windows-release/mapper-cli.exe
	@cp target/x86_64-pc-windows-msvc/release/mapper-terrain-gui.exe target/windows-release/mapper-gui.exe
	@echo "Windows binaries created in target/windows-release/"
	@echo "  mapper-cli.exe - Terrain CLI with PNG export"
	@echo "  mapper-gui.exe - Terrain GUI with smooth rendering"

# Build for macOS (native build only, no cross-compilation from Linux)
macos:
	@if [ "$$(uname)" = "Darwin" ]; then \
		$(MAKE) macos-universal; \
	else \
		echo "=============================================="; \
		echo "macOS Cross-Compilation Not Supported"; \
		echo "=============================================="; \
		echo ""; \
		echo "Cross-compiling to macOS from Linux is not officially supported"; \
		echo "due to Apple's licensing restrictions and toolchain requirements."; \
		echo ""; \
		echo "Options:"; \
		echo "1. Build on a real Mac"; \
		echo "2. Use GitHub Actions (included in .github/workflows/build.yml)"; \
		echo "3. Use a macOS VM (check Apple's licensing terms)"; \
		echo "4. Use a cloud CI service (GitHub Actions, Azure Pipelines, etc.)"; \
		echo ""; \
		echo "The GitHub Actions workflow in this project will automatically"; \
		echo "build macOS binaries when you push to GitHub."; \
		exit 1; \
	fi

# Build universal macOS binary (x86_64 + ARM64)
macos-universal:
	@echo "Building universal macOS binary (x86_64 + ARM64)..."
	@rustup target add x86_64-apple-darwin 2>/dev/null || true
	@rustup target add aarch64-apple-darwin 2>/dev/null || true
	
	# Build for Intel Macs
	cargo build --target x86_64-apple-darwin --release --bin mapper-terrain-cli
	cargo build --target x86_64-apple-darwin --release --bin mapper-terrain-gui
	
	# Build for Apple Silicon Macs
	cargo build --target aarch64-apple-darwin --release --bin mapper-terrain-cli
	cargo build --target aarch64-apple-darwin --release --bin mapper-terrain-gui
	
	# Create universal binaries using lipo
	@mkdir -p target/macos-release
	lipo -create \
		target/x86_64-apple-darwin/release/mapper-terrain-cli \
		target/aarch64-apple-darwin/release/mapper-terrain-cli \
		-output target/macos-release/mapper-cli
	lipo -create \
		target/x86_64-apple-darwin/release/mapper-terrain-gui \
		target/aarch64-apple-darwin/release/mapper-terrain-gui \
		-output target/macos-release/mapper-gui
	
	@chmod +x target/macos-release/mapper-cli
	@chmod +x target/macos-release/mapper-gui
	@echo "Universal macOS binaries created in target/macos-release/"

# Build for Intel Macs only
macos-x86:
	@echo "Building for Intel Macs (x86_64)..."
	@rustup target add x86_64-apple-darwin 2>/dev/null || true
	cargo build --target x86_64-apple-darwin --release --bin mapper-terrain-cli
	cargo build --target x86_64-apple-darwin --release --bin mapper-terrain-gui
	@mkdir -p target/macos-release
	@cp target/x86_64-apple-darwin/release/mapper-terrain-cli target/macos-release/mapper-cli-x86_64
	@cp target/x86_64-apple-darwin/release/mapper-terrain-gui target/macos-release/mapper-gui-x86_64
	@chmod +x target/macos-release/mapper-cli-x86_64
	@chmod +x target/macos-release/mapper-gui-x86_64
	@echo "Intel macOS binaries created in target/macos-release/"

# Build for Apple Silicon Macs only  
macos-arm:
	@echo "Building for Apple Silicon Macs (ARM64)..."
	@rustup target add aarch64-apple-darwin 2>/dev/null || true
	cargo build --target aarch64-apple-darwin --release --bin mapper-terrain-cli
	cargo build --target aarch64-apple-darwin --release --bin mapper-terrain-gui
	@mkdir -p target/macos-release
	@cp target/aarch64-apple-darwin/release/mapper-terrain-cli target/macos-release/mapper-cli-arm64
	@cp target/aarch64-apple-darwin/release/mapper-terrain-gui target/macos-release/mapper-gui-arm64
	@chmod +x target/macos-release/mapper-cli-arm64
	@chmod +x target/macos-release/mapper-gui-arm64
	@echo "Apple Silicon macOS binaries created in target/macos-release/"

# Help target
help:
	@echo "Mapper Build Targets:"
	@echo ""
	@echo "  make build         - Build debug versions for current platform"
	@echo "  make build-release - Build release versions for current platform"
	@echo "  make test          - Run all tests"
	@echo "  make clean         - Clean build artifacts"
	@echo "  make run-cli       - Run the CLI version"
	@echo "  make run-gui       - Run the GUI version"
	@echo "  make windows       - Cross-compile for Windows (auto-detects method)"
	@echo "  make windows-gnu   - Cross-compile for Windows using MinGW"
	@echo "  make windows-msvc  - Cross-compile for Windows using cargo-xwin"
	@echo "  make macos         - Build for macOS (requires Mac)"
	@echo "  make macos-universal - Build universal binary for Intel + Apple Silicon"
	@echo "  make macos-x86     - Build for Intel Macs only"
	@echo "  make macos-arm     - Build for Apple Silicon Macs only"
	@echo "  make help          - Show this help message"
	@echo ""
	@echo "Cross-compilation notes:"
	@echo "  Windows: Requires mingw-w64 or cargo-xwin"
	@echo "  macOS: Must build on macOS (or use GitHub Actions)"