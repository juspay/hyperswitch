# Memory Bank Documentation Validation

This directory contains tools for validating Memory Bank documentation against established standards and best practices. These tools help ensure documentation consistency, correctness, and completeness.

## Overview

The validation suite includes:

1. **Automated validation scripts** for checking:
   - File size compliance
   - Internal link integrity
   - Heading structure consistency

2. **Manual validation checklists** for reviewing:
   - Technical accuracy
   - Completeness
   - Clarity and readability
   - Document type-specific requirements

## Validation Scripts

### Main Validation Script

The `validate_all.sh` script runs all validation checks and generates a consolidated report:

```bash
cd /Users/arunraj/github/hyperswitch/memory-bank/thematic/project_management/validation
bash ./validate_all.sh
```

After running, it will create reports in the `reports` subdirectory, including a consolidated report that summarizes all validation results.

### Individual Validation Scripts

You can also run individual validation scripts if you're focused on specific aspects:

#### File Size Validation

Checks for files exceeding the 300-line limit:

```bash
bash ./validate_file_sizes.sh
```

Output: `file_size_report.md`

#### Link Validation

Checks for broken internal links and identifies external links that need manual verification:

```bash
bash ./validate_links.sh
```

Output: `link_validation_report.md`

#### Heading Structure Validation

Checks for proper heading hierarchy and organization:

```bash
bash ./validate_headings.sh
```

Output: `heading_validation_report.md`

## Validation Reports

Reports are generated in Markdown format and include:

- Summary statistics for each validation area
- Detailed lists of issues organized by severity
- Recommendations for addressing identified problems
- Next steps for resolving issues

## Running Regular Validation

Best practices for using these validation tools:

1. **Run validation before releases** - Always validate documentation before finalizing documentation releases
2. **Run validation after major changes** - Validate after significant documentation updates
3. **Schedule periodic validation** - Set up regular validation checks (e.g., monthly)
4. **Prioritize critical issues** - Address critical issues before addressing warnings
5. **Document exceptions** - If certain issues cannot be fixed, document the reasons

## Validation Process

1. **Run the validation suite**
   ```bash
   bash ./validate_all.sh
   ```

2. **Review the consolidated report**
   - Check overall validation status
   - Identify critical issues that need immediate attention

3. **Address critical issues**
   - Fix files exceeding size limits by splitting them according to the [File Size Management Process](../file_size_management/file_size_management_process.md)
   - Fix broken internal links
   - Fix heading structure errors

4. **Perform manual validation**
   - Use the manual validation checklist in the consolidated report
   - Focus on areas that cannot be automatically validated

5. **Re-run validation**
   - After making fixes, re-run validation to confirm issues are resolved
   - Continue this cycle until all critical issues are addressed

## Integration with Documentation Workflow

These validation tools should be integrated into the Memory Bank documentation workflow:

1. **During creation**: Use templates that follow standards
2. **During review**: Run validation as part of the [review process](../../documentation_process/review_process/01_review_workflow.md)
3. **Before finalization**: Run full validation before finalizing documentation

## Related Documentation

- [Review Criteria](../../documentation_process/review_process/02_review_criteria.md)
- [Review Checklists](../../documentation_process/review_process/03_review_checklists.md)
- [File Size Management Process](../file_size_management/file_size_management_process.md)
- [Documentation Templates](../../documentation_process/templates/)
