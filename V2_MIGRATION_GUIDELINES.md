# Migration Guidelines

This document explains the migration structure and guidelines for adding database migrations in the Hyperswitch project.

## Migration Directory Structure

We have three migration directories with specific purposes:

### 1. `migrations/` - V1 Database Migrations
- Contains all original v1 database migrations
- These are the base migrations for the v1 application
- **DO NOT** add new migrations here unless specifically for v1-only features

### 2. `v2_compatible_migrations/` - Backwards Compatible V2 Migrations
- For introducing v2 columns and tables that work with both v1 and v2 applications
- **USE THIS** for:
  - Adding new columns with `ADD COLUMN IF NOT EXISTS`
  - Creating new tables with `CREATE TABLE IF NOT EXISTS`
  - Adding indexes
  - Any non-destructive changes
- These migrations run while v1 is still active

### 3. `v2_migrations/` - V2-Only Destructive Migrations
- For destructive changes that can only run AFTER v1 is deprecated
- **USE THIS** for:
  - Adding Not Null constraints on v2 only columns
  - Dropping v1 columns with `DROP COLUMN`
  - Dropping unused tables with `DROP TABLE`
  - Removing constraints that v1 depends on
  - Any breaking changes
- These migrations will only run after v1 is fully deprecated

## Common Mistakes to Avoid

### ❌ WRONG: Adding new columns in v2_migrations
```sql
-- v2_migrations/2025-01-17-042122_add_feature_metadata_in_payment_attempt/up.sql
ALTER TABLE payment_attempt ADD COLUMN feature_metadata JSONB;
```

### ✅ CORRECT: Adding new columns in v2_compatible_migrations
```sql
-- v2_compatible_migrations/2025-01-17-042122_add_feature_metadata_in_payment_attempt/up.sql
ALTER TABLE payment_attempt ADD COLUMN IF NOT EXISTS feature_metadata JSONB;
```

## Build-Time Validation

The project includes build-time validation that will fail compilation if:
- You add `ADD COLUMN` statements in `v2_migrations/`
- You add `CREATE TABLE` statements in `v2_migrations/`
- You add `DROP COLUMN` or `DROP TABLE` statements in `v2_compatible_migrations/`

## Decision Flow

When adding a migration, ask yourself:

1. **Is this a destructive change?** (DROP, ALTER that breaks compatibility)
   - YES → Use `v2_migrations/`
   - NO → Continue to question 2

2. **Does this add new functionality that both v1 and v2 can use?**
   - YES → Use `v2_compatible_migrations/`
   - NO → Consider if this migration is necessary

## Examples

### Example 1: Adding a new column
```bash
# Create migration in v2_compatible_migrations
diesel migration generate add_new_feature_column --migration-dir v2_compatible_migrations

# In up.sql
ALTER TABLE payment_attempt 
ADD COLUMN IF NOT EXISTS new_feature VARCHAR(255) DEFAULT NULL;

# In down.sql
ALTER TABLE payment_attempt 
DROP COLUMN IF EXISTS new_feature;
```

### Example 2: Dropping v1 columns (after v1 deprecation)
```bash
# Create migration in v2_migrations
diesel migration generate drop_v1_legacy_columns --migration-dir v2_migrations

# In up.sql
ALTER TABLE payment_attempt 
DROP COLUMN old_v1_column,
DROP COLUMN another_v1_column;

# In down.sql
-- Usually not reversible, but you could add columns back if needed
```

## Questions?

If you're unsure which directory to use, ask yourself:
- Can v1 continue working with this change? → `v2_compatible_migrations/`
- Will this break v1? → `v2_migrations/` (and wait for v1 deprecation)
