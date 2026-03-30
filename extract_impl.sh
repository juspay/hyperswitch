#!/bin/bash
# Extract Conversion implementations from a Rust file

if [ $# -lt 2 ]; then
    echo "Usage: $0 <source_file> <output_file>"
    exit 1
fi

SOURCE_FILE="$1"
OUTPUT_FILE="$2"

echo "Extracting Conversion implementations from $SOURCE_FILE to $OUTPUT_FILE"

# Create output directory if needed
mkdir -p "$(dirname "$OUTPUT_FILE")"

# Use awk to extract the implementations
awk '
/^#\[cfg\(feature = "v[12]"\)\]$/ { 
    cfg_line = $0
    getline
    if (/^#\[async_trait::async_trait\]$/) {
        getline
        if (/impl (?:super::)?behaviour::Conversion for/ || /impl Conversion for/) {
            # Found the start of a Conversion impl
            print cfg_line
            print "#[async_trait::async_trait]"
            print $0
            brace_count = 0
            in_impl = 1
        }
    }
    next
}
/^#\[async_trait::async_trait\]$/ && !in_impl {
    getline
    if (/impl (?:super::)?behaviour::Conversion for/ || /impl Conversion for/) {
        print "#[async_trait::async_trait]"
        print $0
        brace_count = 0
        in_impl = 1
    }
    next
}
in_impl {
    print $0
    brace_count += gsub(/{/, "{")
    brace_count -= gsub(/}/, "}")
    if (brace_count == 0) {
        in_impl = 0
        print ""
    }
}
' "$SOURCE_FILE" > "$OUTPUT_FILE"

echo "Done! Output written to $OUTPUT_FILE"
