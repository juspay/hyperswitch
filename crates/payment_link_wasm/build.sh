#!/bin/bash

# Build script for payment_link_wasm

set -e

echo "Building payment_link_wasm..."

# Build the WASM package
wasm-pack build --target web --out-dir pkg --scope hyperswitch

echo "WASM build complete! Output in pkg/ directory"
echo ""
echo "Usage in frontend:"
echo "import init, { generate_payment_link_preview, validate_payment_link_config } from './pkg/payment_link_wasm.js';"
echo ""
echo "await init();"
echo "const html = generate_payment_link_preview(JSON.stringify(config));"
