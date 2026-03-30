#!/bin/bash
# Script to extract Conversion implementations from hyperswitch_domain_models
# and create corresponding files in storage_impl

set -e

DOMAIN_MODELS_DIR="/Users/anurag.thakur/Work/switchhyper/hyperswitch/crates/hyperswitch_domain_models/src"
STORAGE_IMPL_DIR="/Users/anurag.thakur/Work/switchhyper/hyperswitch/crates/storage_impl/src/conversions"

# Create conversions directory
mkdir -p "$STORAGE_IMPL_DIR"

# List of files with Conversion implementations
FILES=(
    "customer.rs"
    "merchant_key_store.rs"
    "merchant_account.rs"
    "business_profile.rs"
    "payment_methods.rs"
    "merchant_connector_account.rs"
    "tokenization.rs"
    "relay.rs"
    "invoice.rs"
    "subscription.rs"
    "authentication.rs"
    "payments/payment_attempt.rs"
    "payments/payment_intent.rs"
)

echo "Extracting Conversion implementations..."

for file in "${FILES[@]}"; do
    if [ -f "$DOMAIN_MODELS_DIR/$file" ]; then
        echo "Processing: $file"
        # Create target directory if needed
        target_dir="$STORAGE_IMPL_DIR/$(dirname $file)"
        mkdir -p "$target_dir"
        
        # Extract line numbers of Conversion implementations
        grep -n "impl.*behaviour::Conversion for\|impl.*super::behaviour::Conversion for\|impl Conversion for" "$DOMAIN_MODELS_DIR/$file" | head -5
    else
        echo "File not found: $file"
    fi
done

echo "Done! Check the output above for Conversion implementation locations."
