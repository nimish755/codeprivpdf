#!/bin/bash
# Build all WASM modules for CodePRivpdf
# Requires: rustup, wasm-pack
# AI gen cause lazy

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
CRATES_DIR="$PROJECT_ROOT/crates"
OUTPUT_DIR="$PROJECT_ROOT/web/wasm"

echo "=== CodePRivpdf WASM Build ==="
echo "Project root: $PROJECT_ROOT"
echo ""

# Ensure wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "Installing wasm-pack..."
    cargo install wasm-pack
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"

# List of crates to build (only crates that export WASM bindings)
# pdf-codecs and pdf-core are internal libraries, not WASM modules
CRATES=(
    "pdf-merge"
    "pdf-split"
    "pdf-pages"
    "pdf-compress"
)

# Build each crate
for CRATE in "${CRATES[@]}"; do
    CRATE_DIR="$CRATES_DIR/$CRATE"
    
    if [ ! -d "$CRATE_DIR" ]; then
        echo "WARNING: Crate directory not found: $CRATE_DIR"
        continue
    fi
    
    echo "Building $CRATE..."
    
    cd "$CRATE_DIR"
    
    # Build with wasm-pack
    wasm-pack build \
        --target web \
        --out-dir "$OUTPUT_DIR/$CRATE/pkg" \
        --release
    
    # Copy the main wasm file to root wasm dir for easy access
    WASM_NAME="${CRATE//-/_}"
    if [ -f "$OUTPUT_DIR/$CRATE/pkg/${WASM_NAME}_bg.wasm" ]; then
        cp "$OUTPUT_DIR/$CRATE/pkg/${WASM_NAME}_bg.wasm" "$OUTPUT_DIR/${WASM_NAME}.wasm"
        cp "$OUTPUT_DIR/$CRATE/pkg/${WASM_NAME}_bg.js" "$OUTPUT_DIR/${WASM_NAME}_bg.js"
        cp "$OUTPUT_DIR/$CRATE/pkg/${WASM_NAME}.js" "$OUTPUT_DIR/${WASM_NAME}.js"
    fi
    
    echo "✓ $CRATE built successfully"
    echo ""
done

cd "$PROJECT_ROOT"

echo "=== Build Complete ==="
echo "WASM files output to: $OUTPUT_DIR"
echo ""

# Show file sizes
echo "File sizes:"
for CRATE in "${CRATES[@]}"; do
    WASM_NAME="${CRATE//-/_}"
    WASM_FILE="$OUTPUT_DIR/${WASM_NAME}.wasm"
    if [ -f "$WASM_FILE" ]; then
        SIZE=$(du -h "$WASM_FILE" | cut -f1)
        echo "  $WASM_NAME.wasm: $SIZE"
    fi
done

echo ""
echo "Run 'scripts/compress-wasm.sh' to apply Brotli compression"
