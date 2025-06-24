#!/bin/bash

# Heading Structure Validation Script for Memory Bank Documentation
# This script checks all Markdown files in the Memory Bank directory
# and validates their heading structure.

# Configuration
MEMORY_BANK_DIR="/Users/arunraj/github/hyperswitch/memory-bank"
OUTPUT_FILE="heading_validation_report.md"

# Colors for output
RED='\033[0;31m'
YELLOW='\033[0;33m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Starting heading structure validation for Memory Bank documentation...${NC}"

# Create report header
cat > "$OUTPUT_FILE" << EOF
# Memory Bank Heading Structure Validation Report

Generated on $(date)

This report validates the heading structure in Memory Bank documentation to ensure logical organization and proper hierarchy.

## Summary

EOF

# Initialize variables
total_files=0
valid_files=0
warning_files=0
error_files=0

# Arrays to store issue information
declare -a warning_files_array
declare -a error_files_array

# Find all Markdown files and check their heading structure
while IFS= read -r file; do
    total_files=$((total_files + 1))
    rel_path=${file#$MEMORY_BANK_DIR/}
    
    # Extract headings with their levels
    headings=$(grep -n "^#\+ " "$file" | sed -E 's/([0-9]+):([#]+) (.*)/\1:\2:\3/')
    
    # Initialize variables for this file
    has_h1=false
    prev_level=0
    issues=""
    error_found=false
    warning_found=false
    
    # Check each heading
    while IFS= read -r heading; do
        # Skip empty lines
        [ -z "$heading" ] && continue
        
        # Parse heading info
        line_num=$(echo "$heading" | cut -d':' -f1)
        hashes=$(echo "$heading" | cut -d':' -f2)
        heading_text=$(echo "$heading" | cut -d':' -f3-)
        level=${#hashes}
        
        # Check if file has H1
        if [ "$level" -eq 1 ]; then
            has_h1=true
        fi
        
        # Check for heading level jumps (e.g., H1 -> H3)
        if [ "$prev_level" -ne 0 ] && [ "$level" -gt "$prev_level" + 1 ]; then
            issues="${issues}Line $line_num: Heading level jump from H$prev_level to H$level\n"
            warning_found=true
        fi
        
        # Check for heading level inversions (e.g., H2 -> H1)
        if [ "$prev_level" -ne 0 ] && [ "$level" -eq 1 ] && [ "$prev_level" -ne 1 ]; then
            issues="${issues}Line $line_num: Multiple H1 headings found\n"
            error_found=true
        fi
        
        # Update previous level
        prev_level=$level
        
    done <<< "$headings"
    
    # Check if file has an H1
    if [ "$has_h1" = false ] && [ -n "$headings" ]; then
        issues="${issues}No H1 heading found in file\n"
        error_found=true
    fi
    
    # Process results for this file
    if [ "$error_found" = true ]; then
        error_files=$((error_files + 1))
        error_files_array+=("$rel_path:${issues}")
        echo -e "${RED}ERROR: $rel_path has heading structure issues${NC}"
    elif [ "$warning_found" = true ]; then
        warning_files=$((warning_files + 1))
        warning_files_array+=("$rel_path:${issues}")
        echo -e "${YELLOW}WARNING: $rel_path has heading structure warnings${NC}"
    else
        valid_files=$((valid_files + 1))
    fi
    
done < <(find "$MEMORY_BANK_DIR" -name "*.md" -type f)

# Calculate percentages
valid_percentage=$(awk "BEGIN {printf \"%.1f\", ($valid_files / $total_files) * 100}")

# Add summary to report
cat >> "$OUTPUT_FILE" << EOF
- **Total Files Analyzed**: $total_files
- **Valid Heading Structure**: $valid_files ($valid_percentage%)
- **Files with Warnings**: $warning_files
- **Files with Errors**: $error_files

## Error Details

These files have heading structure errors that should be fixed:

| File | Issues |
|------|--------|
EOF

# Add error files to report
for entry in "${error_files_array[@]}"; do
    IFS=':' read -r file issues <<< "$entry"
    # Replace newlines in issues with <br> for markdown table
    formatted_issues=$(echo -e "$issues" | sed -E ':a;N;$!ba;s/\n/<br>/g')
    cat >> "$OUTPUT_FILE" << EOF
| $file | $formatted_issues |
EOF
done

# Add warning section to report
cat >> "$OUTPUT_FILE" << EOF

## Warning Details

These files have heading structure warnings that should be reviewed:

| File | Issues |
|------|--------|
EOF

# Add warning files to report
for entry in "${warning_files_array[@]}"; do
    IFS=':' read -r file issues <<< "$entry"
    # Replace newlines in issues with <br> for markdown table
    formatted_issues=$(echo -e "$issues" | sed -E ':a;N;$!ba;s/\n/<br>/g')
    cat >> "$OUTPUT_FILE" << EOF
| $file | $formatted_issues |
EOF
done

# Add recommendations section
cat >> "$OUTPUT_FILE" << EOF

## Heading Structure Best Practices

1. **Single H1**: Each document should have exactly one H1 heading as the main title
2. **Logical Progression**: Heading levels should progress logically (H1 → H2 → H3) without skipping levels
3. **Proper Nesting**: Content should be properly nested under appropriate headings
4. **Descriptive Headings**: Use clear, descriptive headings that summarize the content
5. **Consistent Capitalization**: Use consistent capitalization in headings (Title Case or Sentence case)

## Next Steps

1. Fix all error-level heading structure issues
2. Review and address warning-level heading structure issues
3. Re-run this validation after making changes to verify fixes
EOF

echo -e "${GREEN}Heading structure validation complete!${NC}"
echo -e "${GREEN}Report generated at $OUTPUT_FILE${NC}"
echo -e "${BLUE}Summary:${NC}"
echo -e "${BLUE}- Total files: $total_files${NC}"
echo -e "${GREEN}- Valid heading structure: $valid_files ($valid_percentage%)${NC}"
echo -e "${YELLOW}- Files with warnings: $warning_files${NC}"
echo -e "${RED}- Files with errors: $error_files${NC}"

exit 0
