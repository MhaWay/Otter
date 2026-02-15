# Makefile for Otter CLI release builds

.PHONY: all release release-windows release-linux release-macos clean help

# Default target
all: help

# Build optimized release binary
release:
	@echo "Building Otter release..."
	cargo build --release -p otter-cli
	@echo "✓ Build complete: target/release/otter"

# Build Windows release with all files
release-windows:
	@echo "Building Otter Windows release..."
	cargo build --release -p otter-cli --target x86_64-pc-windows-gnu || cargo build --release -p otter-cli
	@mkdir -p dist/otter-windows
	@echo "Copying files to dist/otter-windows/..."
	@if [ -f target/x86_64-pc-windows-gnu/release/otter.exe ]; then \
		cp target/x86_64-pc-windows-gnu/release/otter.exe dist/otter-windows/; \
	elif [ -f target/release/otter.exe ]; then \
		cp target/release/otter.exe dist/otter-windows/; \
	else \
		echo "Warning: Could not find Windows binary"; \
	fi
	@cp QUICKSTART.md dist/otter-windows/README.txt || echo "QUICKSTART.md not found"
	@cp run_otter.bat dist/otter-windows/ || echo "run_otter.bat not found"
	@echo "✓ Windows release ready in dist/otter-windows/"

# Build Linux release
release-linux:
	@echo "Building Otter Linux release..."
	cargo build --release -p otter-cli
	@mkdir -p dist/otter-linux
	@cp target/release/otter dist/otter-linux/
	@cp QUICKSTART.md dist/otter-linux/README.md || echo "QUICKSTART.md not found"
	@cp run_otter.sh dist/otter-linux/ || echo "run_otter.sh not found"
	@chmod +x dist/otter-linux/otter
	@chmod +x dist/otter-linux/run_otter.sh || true
	@echo "✓ Linux release ready in dist/otter-linux/"

# Build macOS release
release-macos:
	@echo "Building Otter macOS release..."
	cargo build --release -p otter-cli
	@mkdir -p dist/otter-macos
	@cp target/release/otter dist/otter-macos/
	@cp QUICKSTART.md dist/otter-macos/README.md || echo "QUICKSTART.md not found"
	@cp run_otter.sh dist/otter-macos/ || echo "run_otter.sh not found"
	@chmod +x dist/otter-macos/otter
	@chmod +x dist/otter-macos/run_otter.sh || true
	@echo "✓ macOS release ready in dist/otter-macos/"

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	cargo clean
	rm -rf dist/
	@echo "✓ Clean complete"

# Show help
help:
	@echo "Otter Build System"
	@echo "=================="
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@echo "  release         - Build optimized release binary"
	@echo "  release-windows - Build Windows release package"
	@echo "  release-linux   - Build Linux release package"
	@echo "  release-macos   - Build macOS release package"
	@echo "  clean           - Remove build artifacts"
	@echo "  help            - Show this help message"
