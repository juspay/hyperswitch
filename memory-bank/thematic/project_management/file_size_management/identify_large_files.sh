#!/bin/bash

# Script to identify large Markdown files in the Memory Bank
# This script scans the Memory Bank directory for Markdown files that exceed 
# size thresholds and generates a report of files that should be considered for splitting.

# Directory to search
SEARCH_DIR="/Users/arunraj/github/hyperswitch/memory-bank"

# Thresholds
LINE_THRESHOLD=300
SIZE_THRESHOLD=15000  # 15KB in bytes
APPROACH_THRESHOLD=270  # For files approaching the line threshold

# Output file
OUTPUT_FILE="/Users/arunraj/github/hyperswitch/memory-bank/thematic/project_management/file_size_management/large_files_report.md"

# Create output file with header
cat > "$OUTPUT_FILE" << EOF
# Large Files Report

This report identifies Markdown files in the Memory Bank that exceed or approach size thresholds.
Generated on $(date '+%Y-%m-%d %H:%M:%S')

## Files Exceeding Thresholds

Files that exceed the primary threshold criteria (${LINE_THRESHOLD} lines or ${SIZE_THRESHOLD} bytes):

| File Path | Line Count | Size (KB) | Status |
|-----------|------------|-----------|--------|
EOF

# Find files exceeding thresholds
find "$SEARCH_DIR" -name "*.md" | while read file; do
    # Get line count
    line_count=$(wc -l < "$file")
    
    # Get file size
    file_size=$(wc -c < "$file")
    file_size_kb=$(echo "scale=2; $file_size/1024" | bc)
    
    # Check if either threshold is exceeded
    if [ "$line_count" -ge "$LINE_THRESHOLD" ] || [ "$file_size" -ge "$SIZE_THRESHOLD" ]; then
        relative_path=${file#"$SEARCH_DIR/"}
        echo "| $relative_path | $line_count | $file_size_kb | Exceeds threshold |" >> "$OUTPUT_FILE"
    fi
done

# Add section for files approaching thresholds
cat >> "$OUTPUT_FILE" << EOF

## Files Approaching Thresholds

Files that are approaching the threshold (${APPROACH_THRESHOLD}-${LINE_THRESHOLD} lines) and may require monitoring:

| File Path | Line Count | Size (KB) | Status |
|-----------|------------|-----------|--------|
EOF

# Find files approaching thresholds
find "$SEARCH_DIR" -name "*.md" | while read file; do
    # Get line count
    line_count=$(wc -l < "$file")
    
    # Get file size
    file_size=$(wc -c < "$file")
    file_size_kb=$(echo "scale=2; $file_size/1024" | bc)
    
    # Check if approaching threshold
    if [ "$line_count" -ge "$APPROACH_THRESHOLD" ] && [ "$line_count" -lt "$LINE_THRESHOLD" ]; then
        relative_path=${file#"$SEARCH_DIR/"}
        echo "| $relative_path | $line_count | $file_size_kb | Approaching threshold |" >> "$OUTPUT_FILE"
    fi
done

# Add notes and next steps
cat >> "$OUTPUT_FILE" << EOF

## Analysis Notes

Files exceeding thresholds should be evaluated for splitting according to the criteria in [File Identification Criteria](file_identification_criteria.md).

## Next Steps

1. Review each file exceeding thresholds for logical split points
2. Determine the appropriate splitting strategy for each file
3. Implement splits for files that meet the decision criteria
4. Update this report with the actions taken

## Reference

- [File Identification Criteria](file_identification_criteria.md)
- [File Size Management Guide](../file_size_management_guide.md)
EOF

# Make script executable
chmod +x "$0"

echo "File analysis complete. Report generated at $OUTPUT_FILE"
echo "Found $(grep -c "Exceeds threshold" "$OUTPUT_FILE") files exceeding thresholds."
echo "Found $(grep -c "Approaching threshold" "$OUTPUT_FILE") files approaching thresholds."
