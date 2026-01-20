#!/bin/bash

# validate_migrations.sh
# Validates database migrations for breaking changes and warnings
# Usage: ./validate_migrations.sh <migration_folder>

set -euo pipefail

MIGRATION_FOLDER="${1:-migrations}"
BASE_REF="${BASE_REF:-origin/main}"
RULES_FILE="${RULES_FILE:-.github/api-migration-compatibility/migration-rules.yaml}"

echo "Validating migrations in: $MIGRATION_FOLDER"

# Check if migration directory exists
if [[ ! -d "$MIGRATION_FOLDER" ]]; then
  echo "breaking_changes=0" >> "$GITHUB_OUTPUT"
  echo "warnings=0" >> "$GITHUB_OUTPUT"
  echo "Directory $MIGRATION_FOLDER does not exist, skipping validation"
  exit 0
fi

# Find new migrations
NEW_MIGRATIONS=$(git diff --name-only --diff-filter=AM "$BASE_REF"...HEAD -- "$MIGRATION_FOLDER/**/up.sql" 2>/dev/null || echo "")

if [[ -z "$NEW_MIGRATIONS" ]]; then
  echo "breaking_changes=0" >> "$GITHUB_OUTPUT"
  echo "warnings=0" >> "$GITHUB_OUTPUT"
  echo "No new migrations found in $MIGRATION_FOLDER"
  exit 0
fi

# Extract breaking rule patterns
BREAKING_PATTERNS=$(
  for rule in $(yq ".folder_rules.${MIGRATION_FOLDER}.breaking[]" "$RULES_FILE" 2>/dev/null || echo ""); do
    if [[ -n "$rule" ]]; then
      yq ".rule_patterns.$rule.pattern" "$RULES_FILE"
    fi
  done | paste -sd'|' -
)

# Extract warning rule patterns
WARNING_PATTERNS=$(
  for rule in $(yq ".folder_rules.${MIGRATION_FOLDER}.warnings[]" "$RULES_FILE" 2>/dev/null || echo ""); do
    if [[ -n "$rule" ]]; then
      yq ".rule_patterns.$rule.pattern" "$RULES_FILE"
    fi
  done | paste -sd'|' -
)

BREAKING=0
WARNINGS=0

while IFS= read -r file; do
  [[ -z "$file" ]] && continue

  # Check for breaking changes
  if grep --ignore-case --extended-regexp --quiet "$BREAKING_PATTERNS" "$file" 2>/dev/null; then
    echo "BREAKING: $file"
    MATCH_COUNT=$(grep --ignore-case --count --extended-regexp "$BREAKING_PATTERNS" "$file" 2>/dev/null || echo "0")
    grep --ignore-case --line-number --extended-regexp "$BREAKING_PATTERNS" "$file" 2>/dev/null | sed 's/^/  /' || true
    BREAKING=$((BREAKING + MATCH_COUNT))
  fi

  # Check for warnings
  if grep --ignore-case --extended-regexp --quiet "$WARNING_PATTERNS" "$file" 2>/dev/null; then
    echo "WARNING: $file"
    MATCH_COUNT=$(grep --ignore-case --count --extended-regexp "$WARNING_PATTERNS" "$file" 2>/dev/null || echo "0")
    grep --ignore-case --line-number --extended-regexp "$WARNING_PATTERNS" "$file" 2>/dev/null | sed 's/^/  /' || true
    WARNINGS=$((WARNINGS + MATCH_COUNT))
  fi
done <<< "$NEW_MIGRATIONS"

echo "breaking_changes=$BREAKING" >> "$GITHUB_OUTPUT"
echo "warnings=$WARNINGS" >> "$GITHUB_OUTPUT"

if [[ $BREAKING -gt 0 ]]; then
  echo "::error::Breaking changes detected in $MIGRATION_FOLDER database migrations."
  echo "::error::Found $BREAKING breaking change(s)."
  exit 1
fi