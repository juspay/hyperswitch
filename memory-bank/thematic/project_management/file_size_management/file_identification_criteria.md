# File Identification Criteria

This document defines the criteria for identifying Memory Bank documentation files that require splitting to maintain optimal file sizes for AI tools and readability. These criteria guide the systematic identification of files that should be considered for splitting as part of the file size management process.

## Primary Criteria

Files should be flagged for potential splitting if they meet any of the following primary criteria:

1. **Size Threshold**
   - Files with **more than 300 lines** of content
   - Files approaching the threshold (270-300 lines) that are likely to grow
   - Files exceeding 15KB in size regardless of line count

2. **Section Size**
   - Files containing any single section exceeding 100 lines
   - Files with more than 3 sections of 50+ lines each

3. **Content Complexity**
   - Files covering multiple distinct topics or domains
   - Files documenting complex systems with many components
   - Files containing multiple levels of nested information

## Secondary Criteria

In addition to the primary criteria, the following factors should be considered when evaluating files for splitting:

1. **Update Frequency**
   - Files that are updated frequently are more likely to grow beyond thresholds
   - Files that serve as active documentation for evolving components

2. **Reference Density**
   - Files that contain numerous cross-references to other documents
   - Files that are heavily referenced by other documents

3. **Content Type**
   - API documentation with multiple endpoints
   - Configuration documentation with many options
   - Architectural documentation covering multiple components

4. **Navigation Complexity**
   - Files requiring extensive scrolling to find information
   - Files with complex TOC structures

## File Types to Prioritize

Based on the Memory Bank structure, the following file types should be given priority when applying these criteria:

1. **Core Documentation**
   - `systemPatterns.md`
   - `techContext.md`
   - `productContext.md`

2. **Crate Documentation**
   - Overview files for major crates (`router/overview.md`, etc.)
   - Files documenting complex subsystems

3. **Flow Documentation**
   - Payment flows
   - Webhook flows
   - Refund flows

## Implementation Process

To implement these criteria effectively:

1. **Automated Scanning**
   - Create a script to scan the Memory Bank directory for files exceeding size thresholds
   - Generate reports of files approaching thresholds

2. **Manual Assessment**
   - Review flagged files for secondary criteria
   - Assess content structure and logical breaking points

3. **Documentation**
   - Document findings in the implementation status table
   - Record decisions about which files to split and which to leave intact

## File Identification Report Template

```markdown
## File Identification Report

| File Path | Line Count | Size (KB) | Sections >50 Lines | Topics Covered | Update Frequency | Action |
|-----------|------------|-----------|-------------------|----------------|------------------|--------|
| path/to/file.md | 325 | 18.5 | 4 | Topic1, Topic2 | High | Split |
| path/to/another.md | 280 | 14.2 | 2 | Topic3 | Medium | Monitor |
```

## Identification Script

To assist with file identification, we can use a simple bash script that identifies Markdown files exceeding the threshold:

```bash
#!/bin/bash

# Directory to search
SEARCH_DIR="/Users/arunraj/github/hyperswitch/memory-bank"

# Thresholds
LINE_THRESHOLD=300
SIZE_THRESHOLD=15000  # 15KB in bytes

echo "Files exceeding size thresholds in $SEARCH_DIR:"
echo "----------------------------------------------"
echo "File Path | Line Count | Size (bytes)"
echo "----------------------------------------------"

find "$SEARCH_DIR" -name "*.md" | while read file; do
    # Get line count
    line_count=$(wc -l < "$file")
    
    # Get file size
    file_size=$(wc -c < "$file")
    
    # Check if either threshold is exceeded
    if [ "$line_count" -ge "$LINE_THRESHOLD" ] || [ "$file_size" -ge "$SIZE_THRESHOLD" ]; then
        echo "$file | $line_count | $file_size"
    fi
done
```

This script can be saved as `identify_large_files.sh` in the file size management directory and executed to generate a list of files exceeding the thresholds.

## Decision Criteria for Splitting

After identifying files that meet the criteria, use these decision points to determine whether and how to split them:

1. **Split if:**
   - File exceeds 300 lines AND contains multiple distinct topics
   - File contains sections that can stand alone as separate documents
   - File covers multiple aspects of a system that different users might need independently

2. **Do not split if:**
   - Content is tightly integrated and splitting would reduce clarity
   - File slightly exceeds thresholds but is unlikely to grow further
   - File serves as a comprehensive reference that benefits from being in one place

3. **Consider alternative approaches if:**
   - File exceeds thresholds but has no clear logical separation points
   - Splitting would create excessive cross-references between files
   - Content is frequently read as a complete unit

## Related Documents

- [File Size Management Guide](../file_size_management_guide.md)
- [File Splitting Strategy](./file_splitting_strategy.md) (to be created)
- [Index Creation Process](./index_creation_process.md) (to be created)
- [Cross-Reference Update System](./cross_reference_update_system.md) (to be created)
