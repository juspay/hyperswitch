#!/usr/bin/env python3
"""
Script to move Conversion implementations from hyperswitch_domain_models to storage_impl.
This extracts the implementations and creates corresponding files in storage_impl/src/conversions/
"""

import re
import os
from pathlib import Path

# Map of source files to their target conversion files
CONVERSION_FILES = [
    ("merchant_key_store.rs", "merchant_key_store.rs"),
    ("merchant_account.rs", "merchant_account.rs"),
    ("business_profile.rs", "business_profile.rs"),
    ("payment_methods.rs", "payment_methods.rs"),
    ("merchant_connector_account.rs", "merchant_connector_account.rs"),
    ("tokenization.rs", "tokenization.rs"),
    ("relay.rs", "relay.rs"),
    ("invoice.rs", "invoice.rs"),
    ("subscription.rs", "subscription.rs"),
    ("authentication.rs", "authentication.rs"),
    ("payments/payment_attempt.rs", "payments/payment_attempt.rs"),
    ("payments/payment_intent.rs", "payments/payment_intent.rs"),
]

DOMAIN_MODELS_DIR = Path("/Users/anurag.thakur/Work/switchhyper/hyperswitch/crates/hyperswitch_domain_models/src")
STORAGE_IMPL_DIR = Path("/Users/anurag.thakur/Work/switchhyper/hyperswitch/crates/storage_impl/src/conversions")

def extract_conversions(source_file):
    """Extract Conversion implementations from a source file."""
    with open(source_file, 'r') as f:
        content = f.read()
    
    # Pattern to match Conversion implementations
    # Matches: #[cfg(...)]\n#[async_trait::async_trait]\nimpl ... Conversion for Type { ... }
    pattern = r'(#\[cfg\([^]]+\)]\s*)?#\[async_trait::async_trait\]\s*impl\s+(?:(?:super::)?behaviour::)?Conversion\s+for\s+\w+\s*\{[^}]*\}(?:\s*\})*'
    
    # This is a simplified pattern - in practice we'd need a proper Rust parser
    # For now, let's just find the line numbers
    impls = []
    lines = content.split('\n')
    i = 0
    while i < len(lines):
        line = lines[i]
        if '#[async_trait::async_trait]' in line and i + 1 < len(lines):
            next_line = lines[i + 1]
            if 'impl' in next_line and 'Conversion' in next_line and 'for' in next_line:
                # Found the start of a Conversion impl
                start = i
                # Find the end (count braces)
                brace_count = 0
                end = i + 1
                while end < len(lines):
                    brace_count += lines[end].count('{')
                    brace_count -= lines[end].count('}')
                    if brace_count == 0 and '{' in ''.join(lines[i+1:end+1]):
                        break
                    end += 1
                impl_content = '\n'.join(lines[start:end+1])
                impls.append(impl_content)
                i = end + 1
                continue
        i += 1
    
    return impls

def main():
    print("Moving Conversion implementations...")
    
    for source_rel, target_rel in CONVERSION_FILES:
        source_path = DOMAIN_MODELS_DIR / source_rel
        if not source_path.exists():
            print(f"  Skipping {source_rel} - file not found")
            continue
        
        print(f"  Processing {source_rel}...")
        impls = extract_conversions(source_path)
        
        if not impls:
            print(f"    No Conversion implementations found")
            continue
        
        # Create target directory
        target_path = STORAGE_IMPL_DIR / target_rel
        target_path.parent.mkdir(parents=True, exist_ok=True)
        
        # Create conversion file
        module_name = source_rel.replace('.rs', '').replace('/', '_')
        content = f"//! Conversion implementations for {module_name}\n\n"
        content += "use crate::behaviour::Conversion;\n"
        content += "\n"
        for impl in impls:
            content += impl + "\n\n"
        
        with open(target_path, 'w') as f:
            f.write(content)
        
        print(f"    Created {target_rel} with {len(impls)} implementations")
    
    print("\nDone!")

if __name__ == "__main__":
    main()
