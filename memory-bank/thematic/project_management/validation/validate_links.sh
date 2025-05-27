#!/bin/bash

# Link Validation Script for Memory Bank Documentation
# This script checks all Markdown files in the Memory Bank directory
# and identifies broken internal links.

# Configuration
MEMORY_BANK_DIR="/Users/arunraj/github/hyperswitch/memory-bank"
OUTPUT_FILE="link_validation_report.md"

# Colors for output
RED='\033[0;31m'
YELLOW='\033[0;33m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Starting link validation for Memory Bank documentation...${NC}"

# Create report header
cat > "$OUTPUT_FILE" << EOF
# Memory Bank Link Validation Report

Generated on $(date)

This report identifies broken internal links in the Memory Bank documentation.

## Summary

EOF

# Initialize variables
total_links=0
broken_links=0
suspect_links=0
valid_links=0

# Arrays to store broken and suspect links
declare -a broken_links_array
declare -a suspect_links_array

# Function to resolve relative path
resolve_path() {
    local source_dir="$1"
    local target_path="$2"
    
    # Handle various relative path formats
    if [[ "$target_path" == /* ]]; then
        # Absolute path relative to memory-bank
        echo "$MEMORY_BANK_DIR$target_path"
    elif [[ "$target_path" == ../* ]]; then
        # Parent directory
        local parent_dir=$(dirname "$source_dir")
        local rel_path="${target_path:3}" # Remove ../
        echo "$(resolve_path "$parent_dir" "$rel_path")"
    elif [[ "$target_path" == ./* ]]; then
        # Same directory with ./
        echo "$source_dir/${target_path:2}" # Remove ./
    else
        # Same directory without ./
        echo "$source_dir/$target_path"
    fi
}

# Find all Markdown files and check their links
while IFS= read -r file; do
    source_dir=$(dirname "$file")
    
    # Extract all markdown links with regex
    links=$(grep -o '\[.*\]([^)]*\.md[^)]*)' "$file" | sed -E 's/\[.*\]\(([^#]*)(#[^)]*)?(\))$/\1/')
    
    # Process each link
    while IFS= read -r link; do
        # Skip empty lines
        [ -z "$link" ] && continue
        
        total_links=$((total_links + 1))
        
        # Skip external links
        if [[ "$link" == http* ]]; then
            suspect_links=$((suspect_links + 1))
            suspect_links_array+=("$file:$link:External link - needs manual verification")
            continue
        fi
        
        # Skip links with special format like mdc:
        if [[ "$link" == *":"* && "$link" != *"/"* ]]; then
            suspect_links=$((suspect_links + 1))
            suspect_links_array+=("$file:$link:Special link format - needs manual verification")
            continue
        fi
        
        # Resolve the target path
        target_path=$(resolve_path "$source_dir" "$link")
        
        # Check if file exists
        if [ ! -f "$target_path" ]; then
            broken_links=$((broken_links + 1))
            rel_source=${file#$MEMORY_BANK_DIR/}
            broken_links_array+=("$rel_source:$link:Target file not found")
            echo -e "${RED}BROKEN: $rel_source -> $link (Target file not found)${NC}"
        else
            valid_links=$((valid_links + 1))
        fi
    done <<< "$links"
    
    # Also check for anchor links within the same file
    anchors=$(grep -o '\[.*\](#[^)]*)' "$file" | sed -E 's/\[.*\]\(#([^)]*)\)/\1/')
    
    while IFS= read -r anchor; do
        # Skip empty lines
        [ -z "$anchor" ] && continue
        
        total_links=$((total_links + 1))
        
        # Check if anchor exists in the file
        # Convert anchor to lowercase and replace spaces with hyphens for proper matching
        normalized_anchor=$(echo "$anchor" | tr '[:upper:]' '[:lower:]' | sed 's/ /-/g')
        
        # Look for heading with that anchor
        if ! grep -q "^#.*${anchor}" "$file" && ! grep -q "<a.*id=\"${anchor}\"" "$file"; then
            broken_links=$((broken_links + 1))
            rel_source=${file#$MEMORY_BANK_DIR/}
            broken_links_array+=("$rel_source:#$anchor:Anchor not found in file")
            echo -e "${RED}BROKEN: $rel_source -> #$anchor (Anchor not found in file)${NC}"
        else
            valid_links=$((valid_links + 1))
        fi
    done <<< "$anchors"
    
done < <(find "$MEMORY_BANK_DIR" -name "*.md" -type f)

# Calculate percentages
valid_percentage=$(awk "BEGIN {printf \"%.1f\", ($valid_links / $total_links) * 100}")

# Add summary to report
cat >> "$OUTPUT_FILE" << EOF
- **Total Links Checked**: $total_links
- **Valid Links**: $valid_links ($valid_percentage%)
- **Broken Links**: $broken_links
- **Links Needing Manual Verification**: $suspect_links

## Broken Links

These links point to non-existent files or anchors and need to be fixed.

| Source File | Link | Issue |
|-------------|------|-------|
EOF

# Add broken links to report
for entry in "${broken_links_array[@]}"; do
    IFS=':' read -r source link issue <<< "$entry"
    cat >> "$OUTPUT_FILE" << EOF
| $source | $link | $issue |
EOF
done

# Add suspect links section to report
cat >> "$OUTPUT_FILE" << EOF

## Links Needing Manual Verification

These links need to be checked manually as they could not be automatically verified.

| Source File | Link | Note |
|-------------|------|------|
EOF

# Add suspect links to report
for entry in "${suspect_links_array[@]}"; do
    IFS=':' read -r source link note <<< "$entry"
    cat >> "$OUTPUT_FILE" << EOF
| $source | $link | $note |
EOF
done

# Add next steps section
cat >> "$OUTPUT_FILE" << EOF

## Next Steps

1. Fix all broken internal links by updating them to point to valid files
2. Manually verify external links and special format links
3. Re-run this validation after making changes to verify fixes

## Recommendations

- Use relative paths for internal links within the Memory Bank
- Always verify links after moving or renaming files
- Consider using link reference definitions at the bottom of documents for frequently used links
EOF

echo -e "${GREEN}Link validation complete!${NC}"
echo -e "${GREEN}Report generated at $OUTPUT_FILE${NC}"
echo -e "${BLUE}Summary:${NC}"
echo -e "${BLUE}- Total links: $total_links${NC}"
echo -e "${GREEN}- Valid links: $valid_links ($valid_percentage%)${NC}"
echo -e "${RED}- Broken links: $broken_links${NC}"
echo -e "${YELLOW}- Links needing manual verification: $suspect_links${NC}"

exit 0
