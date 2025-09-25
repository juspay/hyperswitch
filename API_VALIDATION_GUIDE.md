# API Validation Guide for Hyperswitch

This guide explains how to use the API validation system for Hyperswitch to detect breaking changes and ensure API quality.

## Overview

The API validation system helps maintain backward compatibility by:
- 🔍 **Detecting breaking changes** between API versions
- 📝 **Enforcing API quality standards** with linting rules  
- 🤖 **Automating validation** in GitHub PRs
- 🛠️ **Providing local testing tools** for development

## Quick Start

### 1. Install Dependencies

```bash
# Install all required tools
just api-install-deps

# Or install manually:
npm install -g @stoplight/spectral-cli@6.11.0
brew install oasdiff  # macOS
```

### 2. Run Local Validation

```bash
# Full validation (compare current changes with main)
just api-validate

# Compare specific versions
just api-diff v1.0.0 v1.1.0

# Quick breaking changes check only
just api-breaking-changes origin/main HEAD
```

### 3. PR Validation

Add the `validate-api` label to any PR to trigger comprehensive API validation, or validation will run automatically for API-related changes.

## Breaking Change Classification

### ❌ Breaking Changes (Block PR)
These changes **will break** existing API clients:

- **Request Changes:**
  - Remove required fields
  - Remove enum values
  - Change field types
  - Make optional fields required

- **Response Changes:**
  - Remove response fields
  - Change response field types
  - Remove enum values

- **Endpoint Changes:**
  - Remove endpoints
  - Change HTTP methods
  - Remove required parameters

### ✅ Non-Breaking Changes (Allow)
These changes are **backward compatible**:

- Add optional request fields
- Add response fields
- Add new endpoints
- Add enum values
- Make required fields optional
- Relax validation constraints

### ⚠️ Unclassified Changes (Manual Review)
These changes need **manual review**:

- Complex schema restructuring
- Significant parameter changes
- Authentication/security changes

## Local Development Workflow

### Basic Validation
```bash
# Check your current changes
just api-validate
```

### Comparing Specific Versions
```bash
# Compare two tags
just api-diff v1.0.0 v1.1.0

# Compare branches
just api-diff origin/main feature-branch

# Compare commits
just api-diff abc123 def456
```

### Quick Checks
```bash
# Only check for breaking changes (fast)
just api-breaking-changes

# Only run linting (no diff comparison)
just api-lint

# Generate schemas without validation
just api-generate-schemas
```

### Understanding Results
The validation creates a `./validation-output/` directory with:

```
validation-output/
├── validation-summary.md          # Human-readable summary
├── v1-breaking-report.txt         # V1 breaking changes
├── v2-breaking-report.txt         # V2 breaking changes  
├── v1-detailed-diff.txt          # V1 detailed changes
├── v2-detailed-diff.txt          # V2 detailed changes
├── spectral-v1-report.json       # V1 linting issues
├── spectral-v2-report.json       # V2 linting issues
└── *-schema.json                 # Generated schemas
```

## GitHub PR Integration

### Automatic Triggers
Validation runs automatically when:
- PR title/branch contains "api" or "API"
- Files in `crates/api_models/`, `crates/openapi/`, or `crates/router/` change
- Configuration files (`.oasdiff-config.yaml`, `.spectral-hyperswitch.yml`) change

### Manual Triggers
Add the `validate-api` label to any PR to force validation.

### PR Comments
The workflow creates/updates a single comment with:
- 📊 Summary of changes and issues
- 🚨 Breaking changes (if any)
- ⚠️ Quality issues (if any)
- 💡 Recommendations for next steps

## Configuration

### oasdiff Configuration (`.oasdiff-config.yaml`)
Controls breaking change detection:

```yaml
breaking-changes:
  request:
    required-property-removed: error
    enum-value-removed: error
    property-type-changed: error
  response:
    property-removed: error
    property-type-changed: error
```

### Spectral Configuration (`.spectral-hyperswitch.yml`)
Controls API quality linting:

```yaml
extends: ["@stoplight/spectral:oas"]
rules:
  # Custom Hyperswitch rules
  hyperswitch-payment-amount-field: warn
  hyperswitch-currency-field: warn
  hyperswitch-error-response: warn
```

## Best Practices

### For API Changes

1. **Start with local validation**:
   ```bash
   just api-validate
   ```

2. **Review breaking changes carefully**:
   - Consider if the change is absolutely necessary
   - Explore backward-compatible alternatives
   - Plan deprecation timeline if needed

3. **Fix quality issues**:
   - Address Spectral errors before merging
   - Consider fixing warnings for better API design

### For New Features

1. **Add new endpoints instead of modifying existing ones**
2. **Make new fields optional when possible**
3. **Use enum extensions rather than replacements**
4. **Document new features thoroughly**

### For Refactoring

1. **Test with multiple versions**:
   ```bash
   just api-diff v1.0.0 HEAD
   just api-diff v1.1.0 HEAD
   ```

2. **Validate against multiple base branches if needed**

## Troubleshooting

### Common Issues

**"oasdiff not found"**
```bash
# Install oasdiff
brew install oasdiff  # macOS
# or download from GitHub releases for other platforms
```

**"spectral not found"**
```bash
npm install -g @stoplight/spectral-cli@6.11.0
```

**"Schema generation failed"**
- Ensure Rust toolchain is installed
- Check that the project builds: `cargo check`
- Verify openapi crate compiles: `cargo check -p openapi`

**"Git checkout failed during validation"**
- Ensure you have committed or stashed local changes
- Verify the target branch/commit exists
- Check git repository is in a clean state

### Debugging Validation

1. **Check individual components**:
   ```bash
   # Test schema generation
   just api-generate-schemas
   
   # Test Spectral rules
   just api-lint
   
   # Test breaking change detection only
   just api-breaking-changes
   ```

2. **Review detailed reports**:
   - Check `validation-output/validation-summary.md` for overview
   - Review specific report files for detailed issues

3. **Run with verbose output**:
   ```bash
   # Run the script directly for more detailed output
   ./scripts/local-api-validation.sh origin/main HEAD
   ```

## Advanced Usage

### Custom Comparisons
```bash
# Compare with a specific tag
./scripts/local-api-validation.sh v1.0.0 HEAD ./custom-output

# Compare two arbitrary commits
./scripts/local-api-validation.sh abc123 def456 ./commit-comparison
```

### Integration with CI/CD
The validation system is designed to integrate with your deployment pipeline:

1. **Pre-merge validation**: GitHub workflow blocks PRs with breaking changes
2. **Release validation**: Use locally before creating releases
3. **Documentation updates**: Generate compatibility reports for release notes

### Custom Rules
Add your own Spectral rules to `.spectral-hyperswitch.yml`:

```yaml
rules:
  my-custom-rule:
    description: "Description of the rule"
    message: "Error message to show"
    severity: error
    given: "$.paths[*][*]"
    then:
      # Your custom validation logic
```

## Getting Help

1. **Check the logs**: Most issues are visible in validation output
2. **Review configuration**: Ensure `.oasdiff-config.yaml` and `.spectral-hyperswitch.yml` are correct
3. **Test components individually**: Use the specific just commands to isolate issues
4. **Update dependencies**: Ensure you have the latest versions of tools

For questions or issues with the validation system, check the workflow logs in GitHub Actions or run validation locally with verbose output.