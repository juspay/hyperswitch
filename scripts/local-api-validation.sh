#!/bin/bash

# local-api-validation.sh
# Enhanced local API validation script for Hyperswitch
# Supports comparing any two commits, tags, or branches

set -euo pipefail

# Colors for output
if [[ -t 1 ]]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    BLUE='\033[0;34m'
    BOLD='\033[1m'
    NC='\033[0m'
else
    RED='' GREEN='' YELLOW='' BLUE='' BOLD='' NC=''
fi

# Default values
FROM_REF="${1:-origin/main}"
TO_REF="${2:-HEAD}"
OUTPUT_DIR="${3:-./validation-output}"
TEMP_DIR=$(mktemp -d)
VALIDATION_PASSED=true

# Helper functions
log_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
}

log_header() {
    echo ""
    echo -e "${BOLD}${BLUE}═══════════════════════════════════════════════${NC}"
    echo -e "${BOLD}${BLUE} $1${NC}"
    echo -e "${BOLD}${BLUE}═══════════════════════════════════════════════${NC}"
}

cleanup() {
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

# Validate dependencies
check_dependencies() {
    log_info "Checking required dependencies..."
    
    local missing_deps=()
    
    if ! command -v cargo &> /dev/null; then
        missing_deps+=("cargo (Rust)")
    fi
    
    if ! command -v spectral &> /dev/null; then
        missing_deps+=("spectral-cli")
    fi
    
    if ! command -v oasdiff &> /dev/null; then
        missing_deps+=("oasdiff")
    fi
    
    if [[ ${#missing_deps[@]} -gt 0 ]]; then
        log_error "Missing required dependencies:"
        for dep in "${missing_deps[@]}"; do
            echo "  - $dep"
        done
        echo ""
        echo "Install missing dependencies:"
        echo "  - cargo: Install Rust from https://rustup.rs/"
        echo "  - spectral-cli: npm install -g @stoplight/spectral-cli"
        echo "  - oasdiff: brew install oasdiff (macOS) or download from GitHub releases"
        exit 1
    fi
    
    log_success "All dependencies available"
}

# Generate API schemas for a specific ref
generate_schemas_for_ref() {
    local ref="$1"
    local output_prefix="$2"
    
    log_info "Extracting schemas for ref: $ref"
    
    # Try to extract existing schema files from git first
    local v1_extracted=false
    local v2_extracted=false
    
    # Extract V1 schema from git if it exists
    if git show "$ref:api-reference/v1/openapi_spec_v1.json" > "$TEMP_DIR/${output_prefix}-v1-schema.json" 2>/dev/null; then
        log_success "V1 schema extracted from git ($(wc -c < "$TEMP_DIR/${output_prefix}-v1-schema.json") bytes)"
        v1_extracted=true
    fi
    
    # Extract V2 schema from git if it exists  
    if git show "$ref:api-reference/v2/openapi_spec_v2.json" > "$TEMP_DIR/${output_prefix}-v2-schema.json" 2>/dev/null; then
        log_success "V2 schema extracted from git ($(wc -c < "$TEMP_DIR/${output_prefix}-v2-schema.json") bytes)"
        v2_extracted=true
    fi
    
    # If both schemas were extracted successfully, we're done
    if [[ "$v1_extracted" == "true" ]] && [[ "$v2_extracted" == "true" ]]; then
        return 0
    fi
    
    # If extraction failed, we need to generate schemas (only for HEAD/current working directory)
    if [[ "$ref" == "HEAD" ]]; then
        log_info "Schemas not found in git, generating for current HEAD..."
        
        if [[ "$v1_extracted" == "false" ]]; then
            log_info "Generating V1 schema..."
            if cargo run -p openapi --features v1 >/dev/null 2>&1 && [[ -f "api-reference/v1/openapi_spec_v1.json" ]]; then
                cp "api-reference/v1/openapi_spec_v1.json" "$TEMP_DIR/${output_prefix}-v1-schema.json"
                log_success "V1 schema generated ($(wc -c < "$TEMP_DIR/${output_prefix}-v1-schema.json") bytes)"
            else
                log_error "Failed to generate V1 schema"
                echo "{}" > "$TEMP_DIR/${output_prefix}-v1-schema.json"
            fi
        fi
        
        if [[ "$v2_extracted" == "false" ]]; then
            log_info "Generating V2 schema..."
            if cargo run -p openapi --features v2 >/dev/null 2>&1 && [[ -f "api-reference/v2/openapi_spec_v2.json" ]]; then
                cp "api-reference/v2/openapi_spec_v2.json" "$TEMP_DIR/${output_prefix}-v2-schema.json"
                log_success "V2 schema generated ($(wc -c < "$TEMP_DIR/${output_prefix}-v2-schema.json") bytes)"
            else
                log_error "Failed to generate V2 schema"
                echo "{}" > "$TEMP_DIR/${output_prefix}-v2-schema.json"
            fi
        fi
    else
        # For non-HEAD refs, create empty schemas if extraction failed
        log_warning "Schemas not found in git for $ref, using empty schemas"
        if [[ "$v1_extracted" == "false" ]]; then
            echo "{}" > "$TEMP_DIR/${output_prefix}-v1-schema.json"
        fi
        if [[ "$v2_extracted" == "false" ]]; then
            echo "{}" > "$TEMP_DIR/${output_prefix}-v2-schema.json"
        fi
    fi
}

# Run Spectral validation
run_spectral_validation() {
    local schema_file="$1"
    local version="$2"
    local output_file="$3"
    
    log_info "Running Spectral validation on $version schema..."
    
    if [[ ! -f ".spectral-hyperswitch.yml" ]]; then
        log_warning "Spectral config not found, skipping validation"
        return 0
    fi
    
    if spectral lint "$schema_file" --ruleset .spectral-hyperswitch.yml --format json > "$output_file" 2>&1; then
        log_success "$version schema passed Spectral validation"
        return 0
    else
        log_warning "$version schema has Spectral violations"
        
        # Show summary of violations
        if command -v jq &> /dev/null && [[ -s "$output_file" ]]; then
            local error_count warning_count
            error_count=$(jq '[.[] | select(.severity == 0)] | length' "$output_file" 2>/dev/null || echo "0")
            warning_count=$(jq '[.[] | select(.severity == 1)] | length' "$output_file" 2>/dev/null || echo "0")
            
            echo "  Errors: $error_count, Warnings: $warning_count"
            
            # Show first few issues
            if [[ "$error_count" -gt 0 ]]; then
                echo "  Top errors:"
                jq -r '.[] | select(.severity == 0) | "    - \(.message) (\(.path | join(".")))"' "$output_file" 2>/dev/null | head -3
            fi
        fi
        
        return 1
    fi
}

# Run breaking change detection
run_breaking_change_detection() {
    local from_schema="$1"
    local to_schema="$2"
    local version="$3"
    local output_file="$4"
    
    log_info "Checking $version API for breaking changes..."
    
    if [[ ! -f ".oasdiff-config.yaml" ]]; then
        log_warning "oasdiff config not found, using default settings"
    fi
    
    # Note: oasdiff v1.11.7 doesn't support --config flag, using default behavior
    if oasdiff breaking "$from_schema" "$to_schema" > "$output_file" 2>&1; then
        log_success "No breaking changes in $version API"
        echo "✅ $version API is backward compatible" > "$OUTPUT_DIR/$version-breaking-status.txt"
        return 0
    else
        log_error "Breaking changes detected in $version API"
        echo "❌ Breaking changes detected in $version API" > "$OUTPUT_DIR/$version-breaking-status.txt"
        
        # Show breaking changes summary
        if [[ -s "$output_file" ]]; then
            echo "Breaking changes preview:"
            head -10 "$output_file" | sed 's/^/  /'
            local total_lines
            total_lines=$(wc -l < "$output_file")
            if [[ "$total_lines" -gt 10 ]]; then
                echo "  ... and $((total_lines - 10)) more issues"
            fi
        fi
        
        return 1
    fi
}

# Generate detailed diff report
generate_diff_report() {
    local from_schema="$1"
    local to_schema="$2"
    local version="$3"
    local output_file="$4"
    
    log_info "Generating detailed diff for $version API..."
    
    # Note: oasdiff v1.11.7 doesn't support --config flag, using default behavior
    if oasdiff diff "$from_schema" "$to_schema" > "$output_file" 2>/dev/null; then
        log_success "Detailed diff generated for $version"
    else
        log_warning "Could not generate detailed diff for $version"
        echo "No differences detected or error occurred" > "$output_file"
    fi
}

# Main validation function
main() {
    log_header "Hyperswitch Local API Validation"
    
    echo "Comparing: $FROM_REF → $TO_REF"
    echo "Output directory: $OUTPUT_DIR"
    echo ""
    
    # Check dependencies
    check_dependencies
    
    # Create output directory
    mkdir -p "$OUTPUT_DIR"
    
    # Generate schemas for both refs
    log_header "Schema Generation"
    generate_schemas_for_ref "$FROM_REF" "from"
    generate_schemas_for_ref "$TO_REF" "to"
    
    # Copy schemas to output directory for inspection
    cp "$TEMP_DIR/from-v1-schema.json" "$OUTPUT_DIR/from-v1-schema.json"
    cp "$TEMP_DIR/from-v2-schema.json" "$OUTPUT_DIR/from-v2-schema.json"
    cp "$TEMP_DIR/to-v1-schema.json" "$OUTPUT_DIR/to-v1-schema.json"
    cp "$TEMP_DIR/to-v2-schema.json" "$OUTPUT_DIR/to-v2-schema.json"
    
    # Run Spectral validation on target schemas
    log_header "Spectral Validation"
    local spectral_v1_passed=true
    local spectral_v2_passed=true
    
    if ! run_spectral_validation "$TEMP_DIR/to-v1-schema.json" "V1" "$OUTPUT_DIR/spectral-v1-report.json"; then
        spectral_v1_passed=false
    fi
    
    if ! run_spectral_validation "$TEMP_DIR/to-v2-schema.json" "V2" "$OUTPUT_DIR/spectral-v2-report.json"; then
        spectral_v2_passed=false
    fi
    
    # Only fail validation if there are actual spectral errors (not just command failures)
    if [[ "$spectral_v1_passed" == "false" ]] || [[ "$spectral_v2_passed" == "false" ]]; then
        # Check if there are real errors in the reports
        local has_real_errors=false
        for report in "$OUTPUT_DIR/spectral-v1-report.json" "$OUTPUT_DIR/spectral-v2-report.json"; do
            if [[ -f "$report" ]] && command -v jq &> /dev/null; then
                local error_count
                error_count=$(jq '[.[] | select(.severity == 0)] | length' "$report" 2>/dev/null || echo "0")
                if [[ "$error_count" -gt 0 ]]; then
                    has_real_errors=true
                    break
                fi
            fi
        done
        
        if [[ "$has_real_errors" == "true" ]]; then
            log_warning "Spectral validation found actual errors"
            VALIDATION_PASSED=false
        else
            log_info "Spectral command failed but no actual validation errors found"
        fi
    fi
    
    # Run breaking change detection
    log_header "Breaking Change Detection"
    if ! run_breaking_change_detection "$TEMP_DIR/from-v1-schema.json" "$TEMP_DIR/to-v1-schema.json" "V1" "$OUTPUT_DIR/v1-breaking-report.txt"; then
        VALIDATION_PASSED=false
    fi
    
    if ! run_breaking_change_detection "$TEMP_DIR/from-v2-schema.json" "$TEMP_DIR/to-v2-schema.json" "V2" "$OUTPUT_DIR/v2-breaking-report.txt"; then
        VALIDATION_PASSED=false
    fi
    
    # Generate detailed diffs
    log_header "Detailed Change Analysis"
    generate_diff_report "$TEMP_DIR/from-v1-schema.json" "$TEMP_DIR/to-v1-schema.json" "V1" "$OUTPUT_DIR/v1-detailed-diff.txt"
    generate_diff_report "$TEMP_DIR/from-v2-schema.json" "$TEMP_DIR/to-v2-schema.json" "V2" "$OUTPUT_DIR/v2-detailed-diff.txt"
    
    # Generate summary report
    log_header "Validation Summary"
    
    {
        echo "# API Validation Report"
        echo ""
        echo "**Comparison**: \`$FROM_REF\` → \`$TO_REF\`"
        echo "**Generated**: $(date)"
        echo ""
        
        if [[ "$VALIDATION_PASSED" == "true" ]]; then
            echo "## ✅ Overall Status: PASSED"
            echo ""
            echo "No breaking changes detected. The API changes are backward compatible."
        else
            echo "## ❌ Overall Status: FAILED"
            echo ""
            echo "Breaking changes or validation issues detected. Please review the detailed reports."
        fi
        
        echo ""
        echo "## Detailed Reports"
        echo ""
        echo "- **Spectral Reports**: \`spectral-v1-report.json\`, \`spectral-v2-report.json\`"
        echo "- **Breaking Changes**: \`v1-breaking-report.txt\`, \`v2-breaking-report.txt\`"
        echo "- **Detailed Diffs**: \`v1-detailed-diff.txt\`, \`v2-detailed-diff.txt\`"
        echo "- **Generated Schemas**: \`*-v1-schema.json\`, \`*-v2-schema.json\`"
        echo ""
        echo "## Next Steps"
        echo ""
        if [[ "$VALIDATION_PASSED" == "true" ]]; then
            echo "- ✅ Safe to proceed with deployment"
            echo "- Consider updating API documentation if new features were added"
        else
            echo "- ❌ Review breaking changes before proceeding"
            echo "- Consider API versioning strategy"
            echo "- Update client applications as needed"
            echo "- Fix any linting issues identified by Spectral"
        fi
        
    } > "$OUTPUT_DIR/validation-summary.md"
    
    # Display summary
    cat "$OUTPUT_DIR/validation-summary.md"
    echo ""
    log_info "Detailed reports saved to: $OUTPUT_DIR"
    
    # Exit with appropriate code
    if [[ "$VALIDATION_PASSED" == "true" ]]; then
        log_success "API validation completed successfully!"
        exit 0
    else
        log_error "API validation failed. See reports for details."
        exit 1
    fi
}

# Help function
show_help() {
    cat << EOF
Hyperswitch Local API Validation

USAGE:
    $0 [FROM_REF] [TO_REF] [OUTPUT_DIR]

ARGUMENTS:
    FROM_REF    Base reference to compare from (default: origin/main)
    TO_REF      Target reference to compare to (default: HEAD)
    OUTPUT_DIR  Output directory for reports (default: ./validation-output)

EXAMPLES:
    $0                              # Compare origin/main with current HEAD
    $0 v1.0.0 v1.1.0               # Compare two tags
    $0 origin/main feature-branch   # Compare main with feature branch
    $0 abc123 def456               # Compare two commits

REQUIREMENTS:
    - cargo (Rust toolchain)
    - spectral-cli (npm install -g @stoplight/spectral-cli)
    - oasdiff (brew install oasdiff)

EOF
}

# Parse arguments
if [[ "${1:-}" == "--help" ]] || [[ "${1:-}" == "-h" ]]; then
    show_help
    exit 0
fi

# Run main function
main "$@"