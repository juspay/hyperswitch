#!/bin/bash

# Main Documentation Validation Script for Memory Bank
# This script runs all validation scripts and generates a consolidated report

# Configuration
MEMORY_BANK_DIR="/Users/arunraj/github/hyperswitch/memory-bank"
VALIDATION_DIR="/Users/arunraj/github/hyperswitch/memory-bank/thematic/project_management/validation"
OUTPUT_DIR="$VALIDATION_DIR/reports"
CONSOLIDATED_REPORT="$OUTPUT_DIR/consolidated_validation_report.md"
DATE=$(date "+%Y-%m-%d")

# Colors for output
RED='\033[0;31m'
YELLOW='\033[0;33m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Create output directory if it doesn't exist
mkdir -p "$OUTPUT_DIR"

echo -e "${BLUE}Starting comprehensive Memory Bank documentation validation...${NC}"

# Create report header
cat > "$CONSOLIDATED_REPORT" << EOF
# Memory Bank Documentation Validation Report

Generated on $(date)

This report consolidates the results of various validation checks performed on the Memory Bank documentation.

## Summary

EOF

# Initialize validation status
validation_passed=true

# Function to run a validation script and extract summary
run_validation() {
    script_name=$1
    script_description=$2
    output_file="$OUTPUT_DIR/${script_name%.sh}_report.md"
    
    echo -e "${BLUE}Running $script_description...${NC}"
    
    # Run the script, redirecting its output file to our reports directory
    cd "$VALIDATION_DIR"
    OUTPUT_FILE="$output_file" bash "./$script_name"
    result=$?
    
    # Check if script ran successfully
    if [ $result -ne 0 ]; then
        echo -e "${RED}Error running $script_name${NC}"
        validation_passed=false
        return 1
    fi
    
    # Extract summary information
    if [ -f "$output_file" ]; then
        # Add section to consolidated report
        script_title=$(echo "$script_description" | sed 's/\.\.\.$//')
        
        cat >> "$CONSOLIDATED_REPORT" << EOF

## $script_title Results

EOF

        # Extract and add summary section from the individual report
        awk '/^## Summary$/,/^##[^#]/ {if (!/^## Summary$/ && !/^##[^#]/) print}' "$output_file" >> "$CONSOLIDATED_REPORT"
        
        # Add link to detailed report
        cat >> "$CONSOLIDATED_REPORT" << EOF

[View detailed $script_title report]($(basename "$output_file"))

EOF

        # Check for critical issues
        if grep -q "Files Exceeding Limit\|Broken Links\|Files with Errors" "$output_file"; then
            validation_passed=false
        fi
    else
        echo -e "${RED}Output file not found for $script_name${NC}"
        validation_passed=false
    fi
}

# Run validation scripts
run_validation "validate_file_sizes.sh" "File size validation..."
run_validation "validate_links.sh" "Link validation..."
run_validation "validate_headings.sh" "Heading structure validation..."

# Add overall status to consolidated report
cat >> "$CONSOLIDATED_REPORT" << EOF

## Overall Validation Status

EOF

if [ "$validation_passed" = true ]; then
    cat >> "$CONSOLIDATED_REPORT" << EOF
✅ **PASSED**: All automated validation checks completed without critical issues.
EOF
    echo -e "${GREEN}Overall validation PASSED${NC}"
else
    cat >> "$CONSOLIDATED_REPORT" << EOF
❌ **FAILED**: One or more validation checks detected critical issues that need to be addressed.
EOF
    echo -e "${RED}Overall validation FAILED${NC}"
fi

# Add manual validation checklist section
cat >> "$CONSOLIDATED_REPORT" << EOF

## Manual Validation Checklist

The following aspects of documentation require manual review and cannot be automatically validated:

1. **Technical Accuracy**
   - [ ] All technical statements are factually correct
   - [ ] Code examples and snippets are correct and functional
   - [ ] API descriptions match the actual implementation
   - [ ] Architectural descriptions accurately reflect the current system

2. **Completeness**
   - [ ] Documentation covers the entire scope of the subject matter
   - [ ] All required sections for each document type are present
   - [ ] All relevant features and functionality are documented
   - [ ] Error conditions and handling procedures are documented

3. **Clarity & Readability**
   - [ ] Content is appropriate for the intended audience
   - [ ] Concepts are explained clearly and logically
   - [ ] Technical terms are either explained or linked to explanations
   - [ ] Information is presented concisely

4. **Document Type-Specific Reviews**
   - [ ] Crate Documentation: Purpose, interfaces, and integration are clearly explained
   - [ ] Flow Documentation: All steps and component interactions are documented
   - [ ] API Documentation: All endpoints, parameters, and responses are documented
   - [ ] Configuration Documentation: All options, defaults, and constraints are documented

## Next Steps

1. Address all critical issues identified in the automated validation
2. Perform manual validation using the checklist above
3. Re-run validation after making changes
4. Document any exceptions or special cases that cannot be fixed

## Related Documentation

- [Review Criteria](../../documentation_process/review_process/02_review_criteria.md)
- [Review Checklists](../../documentation_process/review_process/03_review_checklists.md)
- [File Size Management Process](../file_size_management/file_size_management_process.md)
EOF

echo -e "${GREEN}Consolidated validation report generated at ${CONSOLIDATED_REPORT}${NC}"

exit 0
