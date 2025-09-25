#!/bin/bash

# generate-compatibility-report.sh
# Enhanced compatibility report generation for Hyperswitch API validation
# Creates structured markdown reports with detailed change analysis

set -euo pipefail

# Input files (with defaults)
V1_BREAKING_REPORT="${V1_BREAKING_REPORT:-v1-breaking-report.txt}"
V2_BREAKING_REPORT="${V2_BREAKING_REPORT:-v2-breaking-report.txt}"
V1_DETAILED_DIFF="${V1_DETAILED_DIFF:-v1-detailed-diff.txt}"
V2_DETAILED_DIFF="${V2_DETAILED_DIFF:-v2-detailed-diff.txt}"
SPECTRAL_V1_REPORT="${SPECTRAL_V1_REPORT:-spectral-v1-report.json}"
SPECTRAL_V2_REPORT="${SPECTRAL_V2_REPORT:-spectral-v2-report.json}"

# Helper functions
count_file_issues() {
    local file="$1"
    local pattern="${2:-.*}"
    
    if [[ -f "$file" ]] && [[ -s "$file" ]]; then
        grep -c "$pattern" "$file" 2>/dev/null || echo "0"
    else
        echo "0"
    fi
}

count_json_issues() {
    local file="$1"
    local severity="${2:-0}"  # 0=error, 1=warn, 2=info, 3=hint
    
    if [[ -f "$file" ]] && [[ -s "$file" ]] && command -v jq &> /dev/null; then
        jq "[.[] | select(.severity == $severity)] | length" "$file" 2>/dev/null || echo "0"
    else
        echo "0"
    fi
}

# Count different types of issues
V1_BREAKING_COUNT=$(count_file_issues "$V1_BREAKING_REPORT" "error\|BREAKING")
V2_BREAKING_COUNT=$(count_file_issues "$V2_BREAKING_REPORT" "error\|BREAKING")
TOTAL_BREAKING=$((V1_BREAKING_COUNT + V2_BREAKING_COUNT))

V1_SPECTRAL_ERRORS=$(count_json_issues "$SPECTRAL_V1_REPORT" 0)
V2_SPECTRAL_ERRORS=$(count_json_issues "$SPECTRAL_V2_REPORT" 0)
TOTAL_SPECTRAL_ERRORS=$((V1_SPECTRAL_ERRORS + V2_SPECTRAL_ERRORS))

V1_SPECTRAL_WARNINGS=$(count_json_issues "$SPECTRAL_V1_REPORT" 1)
V2_SPECTRAL_WARNINGS=$(count_json_issues "$SPECTRAL_V2_REPORT" 1)
TOTAL_SPECTRAL_WARNINGS=$((V1_SPECTRAL_WARNINGS + V2_SPECTRAL_WARNINGS))

# Count changes from detailed diffs
V1_NEW_ENDPOINTS=$(count_file_issues "$V1_DETAILED_DIFF" "added.*path")
V2_NEW_ENDPOINTS=$(count_file_issues "$V2_DETAILED_DIFF" "added.*path")
TOTAL_NEW_ENDPOINTS=$((V1_NEW_ENDPOINTS + V2_NEW_ENDPOINTS))

V1_REMOVED_ENDPOINTS=$(count_file_issues "$V1_DETAILED_DIFF" "deleted.*path")
V2_REMOVED_ENDPOINTS=$(count_file_issues "$V2_DETAILED_DIFF" "deleted.*path")
TOTAL_REMOVED_ENDPOINTS=$((V1_REMOVED_ENDPOINTS + V2_REMOVED_ENDPOINTS))

V1_MODIFIED_ENDPOINTS=$(count_file_issues "$V1_DETAILED_DIFF" "modified.*path")
V2_MODIFIED_ENDPOINTS=$(count_file_issues "$V2_DETAILED_DIFF" "modified.*path")
TOTAL_MODIFIED_ENDPOINTS=$((V1_MODIFIED_ENDPOINTS + V2_MODIFIED_ENDPOINTS))

# Start generating the report
cat << EOF
## 📊 Summary

| Metric | V1 API | V2 API | Total |
|--------|--------|--------|-------|
| **Breaking Changes** | $V1_BREAKING_COUNT | $V2_BREAKING_COUNT | **$TOTAL_BREAKING** |
| **Spectral Errors** | $V1_SPECTRAL_ERRORS | $V2_SPECTRAL_ERRORS | **$TOTAL_SPECTRAL_ERRORS** |
| **Spectral Warnings** | $V1_SPECTRAL_WARNINGS | $V2_SPECTRAL_WARNINGS | **$TOTAL_SPECTRAL_WARNINGS** |
| **New Endpoints** | $V1_NEW_ENDPOINTS | $V2_NEW_ENDPOINTS | **$TOTAL_NEW_ENDPOINTS** |
| **Removed Endpoints** | $V1_REMOVED_ENDPOINTS | $V2_REMOVED_ENDPOINTS | **$TOTAL_REMOVED_ENDPOINTS** |
| **Modified Endpoints** | $V1_MODIFIED_ENDPOINTS | $V2_MODIFIED_ENDPOINTS | **$TOTAL_MODIFIED_ENDPOINTS** |

EOF

# Overall status
if [[ $TOTAL_BREAKING -gt 0 ]]; then
    cat << EOF
### 🚨 Overall Status: BREAKING CHANGES DETECTED

**❌ This change introduces breaking changes that will affect existing API clients.**

EOF
elif [[ $TOTAL_SPECTRAL_ERRORS -gt 0 ]]; then
    cat << EOF
### ⚠️ Overall Status: VALIDATION ERRORS FOUND

**⚠️ This change has API specification errors that should be fixed.**

EOF
elif [[ $TOTAL_SPECTRAL_WARNINGS -gt 0 ]]; then
    cat << EOF
### 💭 Overall Status: REVIEW RECOMMENDED

**💭 This change has some API quality issues worth reviewing.**

EOF
else
    cat << EOF
### ✅ Overall Status: ALL CHECKS PASSED

**✅ This change is backward compatible and follows API best practices.**

EOF
fi

# Breaking Changes Section
if [[ $TOTAL_BREAKING -gt 0 ]]; then
    cat << EOF
---

## 🚨 Breaking Changes Detected

EOF
    
    if [[ $V1_BREAKING_COUNT -gt 0 ]]; then
        cat << EOF
### V1 API Breaking Changes ($V1_BREAKING_COUNT issues)

EOF
        if [[ -f "$V1_BREAKING_REPORT" ]] && [[ -s "$V1_BREAKING_REPORT" ]]; then
            # Show first 10 lines of breaking changes
            head -10 "$V1_BREAKING_REPORT" | while IFS= read -r line; do
                echo "- $line"
            done
            
            local total_lines
            total_lines=$(wc -l < "$V1_BREAKING_REPORT" 2>/dev/null || echo "0")
            if [[ $total_lines -gt 10 ]]; then
                echo ""
                echo "*... and $((total_lines - 10)) more breaking changes*"
            fi
        fi
        echo ""
    fi
    
    if [[ $V2_BREAKING_COUNT -gt 0 ]]; then
        cat << EOF
### V2 API Breaking Changes ($V2_BREAKING_COUNT issues)

EOF
        if [[ -f "$V2_BREAKING_REPORT" ]] && [[ -s "$V2_BREAKING_REPORT" ]]; then
            # Show first 10 lines of breaking changes
            head -10 "$V2_BREAKING_REPORT" | while IFS= read -r line; do
                echo "- $line"
            done
            
            local total_lines
            total_lines=$(wc -l < "$V2_BREAKING_REPORT" 2>/dev/null || echo "0")
            if [[ $total_lines -gt 10 ]]; then
                echo ""
                echo "*... and $((total_lines - 10)) more breaking changes*"
            fi
        fi
        echo ""
    fi
fi

# API Changes Section
if [[ $TOTAL_NEW_ENDPOINTS -gt 0 ]] || [[ $TOTAL_REMOVED_ENDPOINTS -gt 0 ]] || [[ $TOTAL_MODIFIED_ENDPOINTS -gt 0 ]]; then
    cat << EOF
---

## 🔄 API Changes

EOF
    
    if [[ $TOTAL_NEW_ENDPOINTS -gt 0 ]]; then
        cat << EOF
### ✅ New Endpoints ($TOTAL_NEW_ENDPOINTS)

These are safe, backward-compatible additions:

EOF
        
        # Extract new endpoints from diff reports
        for version in "V1" "V2"; do
            local diff_file="$([[ $version == "V1" ]] && echo "$V1_DETAILED_DIFF" || echo "$V2_DETAILED_DIFF")"
            local count="$([[ $version == "V1" ]] && echo "$V1_NEW_ENDPOINTS" || echo "$V2_NEW_ENDPOINTS")"
            
            if [[ $count -gt 0 ]] && [[ -f "$diff_file" ]]; then
                echo "**$version API:**"
                grep "added.*path" "$diff_file" 2>/dev/null | head -5 | while IFS= read -r line; do
                    # Extract method and path from oasdiff output
                    if [[ $line =~ method\ \'([^\']+)\'.*path\ \'([^\']+)\' ]]; then
                        method="${BASH_REMATCH[1]}"
                        path="${BASH_REMATCH[2]}"
                        echo "- **$method** \`$path\`"
                    else
                        echo "- $line"
                    fi
                done
                
                if [[ $count -gt 5 ]]; then
                    echo "- *... and $((count - 5)) more endpoints*"
                fi
                echo ""
            fi
        done
    fi
    
    if [[ $TOTAL_REMOVED_ENDPOINTS -gt 0 ]]; then
        cat << EOF
### ❌ Removed Endpoints ($TOTAL_REMOVED_ENDPOINTS) - BREAKING

These endpoint removals will break existing clients:

EOF
        
        for version in "V1" "V2"; do
            local diff_file="$([[ $version == "V1" ]] && echo "$V1_DETAILED_DIFF" || echo "$V2_DETAILED_DIFF")"
            local count="$([[ $version == "V1" ]] && echo "$V1_REMOVED_ENDPOINTS" || echo "$V2_REMOVED_ENDPOINTS")"
            
            if [[ $count -gt 0 ]] && [[ -f "$diff_file" ]]; then
                echo "**$version API:**"
                grep "deleted.*path" "$diff_file" 2>/dev/null | head -5 | while IFS= read -r line; do
                    if [[ $line =~ method\ \'([^\']+)\'.*path\ \'([^\']+)\' ]]; then
                        method="${BASH_REMATCH[1]}"
                        path="${BASH_REMATCH[2]}"
                        echo "- **$method** \`$path\` 🚨"
                    else
                        echo "- $line 🚨"
                    fi
                done
                
                if [[ $count -gt 5 ]]; then
                    echo "- *... and $((count - 5)) more endpoints*"
                fi
                echo ""
            fi
        done
    fi
    
    if [[ $TOTAL_MODIFIED_ENDPOINTS -gt 0 ]]; then
        cat << EOF
### 🔄 Modified Endpoints ($TOTAL_MODIFIED_ENDPOINTS)

These endpoints have been changed (review for breaking changes):

EOF
        
        for version in "V1" "V2"; do
            local diff_file="$([[ $version == "V1" ]] && echo "$V1_DETAILED_DIFF" || echo "$V2_DETAILED_DIFF")"
            local count="$([[ $version == "V1" ]] && echo "$V1_MODIFIED_ENDPOINTS" || echo "$V2_MODIFIED_ENDPOINTS")"
            
            if [[ $count -gt 0 ]] && [[ -f "$diff_file" ]]; then
                echo "**$version API:**"
                grep "modified.*path" "$diff_file" 2>/dev/null | head -5 | while IFS= read -r line; do
                    if [[ $line =~ method\ \'([^\']+)\'.*path\ \'([^\']+)\' ]]; then
                        method="${BASH_REMATCH[1]}"
                        path="${BASH_REMATCH[2]}"
                        echo "- **$method** \`$path\`"
                    else
                        echo "- $line"
                    fi
                done
                
                if [[ $count -gt 5 ]]; then
                    echo "- *... and $((count - 5)) more endpoints*"
                fi
                echo ""
            fi
        done
    fi
fi

# Spectral Validation Section
if [[ $TOTAL_SPECTRAL_ERRORS -gt 0 ]] || [[ $TOTAL_SPECTRAL_WARNINGS -gt 0 ]]; then
    cat << EOF
---

## 🔍 API Quality Issues

EOF
    
    if [[ $TOTAL_SPECTRAL_ERRORS -gt 0 ]]; then
        cat << EOF
### ❌ Spectral Errors ($TOTAL_SPECTRAL_ERRORS)

These issues should be fixed before merging:

EOF
        
        for version in "V1" "V2"; do
            local report_file="$([[ $version == "V1" ]] && echo "$SPECTRAL_V1_REPORT" || echo "$SPECTRAL_V2_REPORT")"
            local count="$([[ $version == "V1" ]] && echo "$V1_SPECTRAL_ERRORS" || echo "$V2_SPECTRAL_ERRORS")"
            
            if [[ $count -gt 0 ]] && [[ -f "$report_file" ]] && command -v jq &> /dev/null; then
                echo "**$version API:**"
                jq -r '.[] | select(.severity == 0) | "- \(.message) (\(.path | join(".")))"' "$report_file" 2>/dev/null | head -5
                
                if [[ $count -gt 5 ]]; then
                    echo "- *... and $((count - 5)) more errors*"
                fi
                echo ""
            fi
        done
    fi
    
    if [[ $TOTAL_SPECTRAL_WARNINGS -gt 0 ]]; then
        cat << EOF
### ⚠️ Spectral Warnings ($TOTAL_SPECTRAL_WARNINGS)

These issues are recommended to fix:

EOF
        
        for version in "V1" "V2"; do
            local report_file="$([[ $version == "V1" ]] && echo "$SPECTRAL_V1_REPORT" || echo "$SPECTRAL_V2_REPORT")"
            local count="$([[ $version == "V1" ]] && echo "$V1_SPECTRAL_WARNINGS" || echo "$V2_SPECTRAL_WARNINGS")"
            
            if [[ $count -gt 0 ]] && [[ -f "$report_file" ]] && command -v jq &> /dev/null; then
                echo "**$version API:**"
                jq -r '.[] | select(.severity == 1) | "- \(.message) (\(.path | join(".")))"' "$report_file" 2>/dev/null | head -3
                
                if [[ $count -gt 3 ]]; then
                    echo "- *... and $((count - 3)) more warnings*"
                fi
                echo ""
            fi
        done
    fi
fi

# Recommendations Section
cat << EOF
---

## 💡 Recommendations

EOF

if [[ $TOTAL_BREAKING -gt 0 ]]; then
    cat << EOF
### ❌ Breaking Changes Action Required

**Immediate Actions:**
- 🔄 **Consider API versioning** - Create new endpoints (e.g., \`/v3/\`) instead of modifying existing ones
- 📋 **Review change necessity** - Are these breaking changes absolutely required?
- 📞 **Coordinate with consumers** - Notify all API consumers before deployment
- 🗓️ **Plan deprecation timeline** - Provide advance notice and migration path

**Before Merging:**
- Update API documentation with breaking change notices
- Consider implementing deprecation warnings for removed endpoints
- Ensure client SDKs are updated to handle changes
- Plan communication strategy for API consumers

EOF
elif [[ $TOTAL_SPECTRAL_ERRORS -gt 0 ]]; then
    cat << EOF
### ⚠️ Quality Issues Should Be Fixed

**Recommended Actions:**
- 🔧 **Fix Spectral errors** - Address API specification issues
- 📝 **Improve documentation** - Add missing descriptions and examples
- ✅ **Validate changes** - Run \`just api-validate\` locally after fixes

EOF
elif [[ $TOTAL_SPECTRAL_WARNINGS -gt 0 ]]; then
    cat << EOF
### 💭 Consider Quality Improvements

**Optional Actions:**
- 📝 **Address warnings** - Improve API documentation and consistency
- 🎯 **Follow best practices** - Consider the Spectral suggestions for better API design

EOF
else
    cat << EOF
### ✅ Ready to Deploy

**Next Steps:**
- 🚀 **Safe to merge** - No breaking changes or critical issues detected
- 📚 **Update documentation** - Document any new features or endpoints added
- 🧪 **Test integration** - Verify changes work as expected with existing clients

EOF
fi

# Footer
cat << EOF
---

## 📁 Detailed Reports

For complete details, check these files:
- **Breaking Changes**: \`$V1_BREAKING_REPORT\`, \`$V2_BREAKING_REPORT\`
- **Detailed Diffs**: \`$V1_DETAILED_DIFF\`, \`$V2_DETAILED_DIFF\`
- **Spectral Reports**: \`$SPECTRAL_V1_REPORT\`, \`$SPECTRAL_V2_REPORT\`

Generated at: $(date)
EOF