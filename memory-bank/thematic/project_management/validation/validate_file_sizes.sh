#!/bin/bash

# File Size Validation Script for Memory Bank Documentation
# This script checks all Markdown files in the Memory Bank directory
# and identifies files that exceed or approach the 300-line limit.

# Configuration
MEMORY_BANK_DIR="/Users/arunraj/github/hyperswitch/memory-bank"
MAX_LINES=300
WARNING_THRESHOLD=270
OUTPUT_FILE="file_size_report.md"

# Colors for output
RED='\033[0;31m'
YELLOW='\033[0;33m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Starting file size validation for Memory Bank documentation...${NC}"

# Create report header
cat > "$OUTPUT_FILE" << EOF
# Memory Bank File Size Report

Generated on $(date)

This report identifies Markdown files in the Memory Bank documentation that exceed or approach the 300-line limit.

## Summary

EOF

# Initialize counters
total_files=0
critical_files=0
warning_files=0
compliant_files=0

# Create arrays to store file information
declare -a critical_files_array
declare -a warning_files_array

# Find all Markdown files and check their line count
while IFS= read -r file; do
    line_count=$(wc -l < "$file")
    total_files=$((total_files + 1))
    
    if [ "$line_count" -ge "$MAX_LINES" ]; then
        critical_files=$((critical_files + 1))
        critical_files_array+=("$file:$line_count")
        echo -e "${RED}CRITICAL: $file has $line_count lines (exceeds $MAX_LINES limit)${NC}"
    elif [ "$line_count" -ge "$WARNING_THRESHOLD" ]; then
        warning_files=$((warning_files + 1))
        warning_files_array+=("$file:$line_count")
        echo -e "${YELLOW}WARNING: $file has $line_count lines (approaching $MAX_LINES limit)${NC}"
    else
        compliant_files=$((compliant_files + 1))
    fi
done < <(find "$MEMORY_BANK_DIR" -name "*.md" -type f)

# Calculate compliance percentage
compliance_percentage=$(awk "BEGIN {printf \"%.1f\", ($compliant_files / $total_files) * 100}")

# Add summary to report
cat >> "$OUTPUT_FILE" << EOF
- **Total Files Analyzed**: $total_files
- **Compliant Files**: $compliant_files ($compliance_percentage%)
- **Files Approaching Limit**: $warning_files
- **Files Exceeding Limit**: $critical_files

## Critical Issues (Files Exceeding 300 Lines)

Files that exceed the 300-line limit require immediate attention and should be split according to the [File Size Management Process](../file_size_management/file_size_management_process.md).

| File Path | Line Count | Recommendation |
|-----------|------------|----------------|
EOF

# Add critical files to report
for entry in "${critical_files_array[@]}"; do
    IFS=':' read -r file line_count <<< "$entry"
    # Calculate relative path
    rel_path=${file#$MEMORY_BANK_DIR/}
    cat >> "$OUTPUT_FILE" << EOF
| $rel_path | $line_count | Split file using appropriate [splitting strategy](../file_size_management/file_splitting_strategy.md) |
EOF
done

# Add warning section to report
cat >> "$OUTPUT_FILE" << EOF

## Warning Issues (Files Approaching 300 Lines)

Files approaching the 300-line limit should be monitored and considered for proactive splitting in the next documentation update.

| File Path | Line Count | Recommendation |
|-----------|------------|----------------|
EOF

# Add warning files to report
for entry in "${warning_files_array[@]}"; do
    IFS=':' read -r file line_count <<< "$entry"
    # Calculate relative path
    rel_path=${file#$MEMORY_BANK_DIR/}
    cat >> "$OUTPUT_FILE" << EOF
| $rel_path | $line_count | Monitor for growth and plan splitting strategy |
EOF
done

# Add next steps section
cat >> "$OUTPUT_FILE" << EOF

## Next Steps

1. Address critical issues first by splitting files according to the [File Size Management Process](../file_size_management/file_size_management_process.md)
2. Monitor warning files and proactively plan splitting if they continue to grow
3. Re-run this validation after making changes to verify compliance

## Related Documentation

- [File Size Management Process](../file_size_management/file_size_management_process.md)
- [File Splitting Strategy](../file_size_management/file_splitting_strategy.md)
- [Index Creation Process](../file_size_management/index_creation_process.md)
- [Cross-Reference Update System](../file_size_management/cross_reference_update_system.md)
EOF

echo -e "${GREEN}File size validation complete!${NC}"
echo -e "${GREEN}Report generated at $OUTPUT_FILE${NC}"
echo -e "${BLUE}Summary:${NC}"
echo -e "${BLUE}- Total files: $total_files${NC}"
echo -e "${GREEN}- Compliant files: $compliant_files ($compliance_percentage%)${NC}"
echo -e "${YELLOW}- Files approaching limit: $warning_files${NC}"
echo -e "${RED}- Files exceeding limit: $critical_files${NC}"

exit 0
