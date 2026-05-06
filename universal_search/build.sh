#!/bin/bash
# Build script for universal-search-service on Linux/macOS

set -e

PROFILE="${1:-release}"
CLEAN="${2:-false}"

echo "Building universal-search-service ($PROFILE)..."

if [ "$CLEAN" = "true" ]; then
    cargo clean -p universal-search-service
fi

cargo build --package universal-search-service --profile "$PROFILE"

OUTPUT_DIR="target/$PROFILE"
EXE_PATH="$OUTPUT_DIR/universal-search-service"

if [ ! -f "$EXE_PATH" ]; then
    echo "Binary not found at $EXE_PATH"
    exit 1
fi

echo "Build successful: $EXE_PATH"

# Copy binary to universal_search directory (alongside config.jsonc)
cp "$EXE_PATH" "universal_search/universal-search-service" 2>/dev/null || cp "$EXE_PATH" "$SCRIPT_DIR/universal-search-service"
echo "Binary copied to: universal_search/universal-search-service"

# Create distribution folder
DIST_DIR="dist/universal-search"
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

# Copy binary
cp "$EXE_PATH" "$DIST_DIR/universal-search-service"

# Copy config template if exists
if [ -f "universal_search/config.jsonc.template" ]; then
    cp "universal_search/config.jsonc.template" "$DIST_DIR/config.jsonc.template"
fi

# Create start script
cat > "$DIST_DIR/start.sh" << 'EOF'
#!/bin/bash
echo "Starting Universal Search Service..."
"$(dirname "$0")/universal-search-service" run
EOF
chmod +x "$DIST_DIR/start.sh"

echo "Distribution created at $DIST_DIR"
