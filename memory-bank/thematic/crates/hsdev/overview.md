# Hsdev Overview

The `hsdev` crate provides a simple PostgreSQL migration utility that uses TOML configuration files. This document provides an overview of its purpose, structure, and usage within the Hyperswitch ecosystem.

---
**Last Updated:** 2025-05-27  
**Documentation Status:** Complete
---

## Purpose

The `hsdev` crate is responsible for:

1. Managing database migrations for PostgreSQL databases
2. Reading database connection information from TOML configuration files
3. Executing Diesel migrations
4. Providing a simple CLI interface for migration operations
5. Supporting development and deployment database setup

## Key Modules

The `hsdev` crate is organized into the following key modules:

- **main.rs**: Core functionality and CLI implementation using clap
- **input_file.rs**: TOML configuration parsing and database URL generation

## Core Features

### TOML Configuration Parsing

The crate can parse database connection information from TOML files:

- Username, password, database name, host, and port
- Support for nested configuration (accessing specific tables within the TOML file)
- Automatic conversion to PostgreSQL connection strings

### Diesel Migration Integration

Integrates with the Diesel ORM's migration framework:

- Discovers migrations from the standard migrations directory
- Runs pending migrations against the configured database
- Provides output logging for migration operations

### Command-Line Interface

Provides a simple CLI for executing migrations:

- Specifies the TOML configuration file path
- Optionally specifies a specific table within the TOML file
- Handles errors with meaningful messages

## Public Interface

The crate provides a command-line interface with the following arguments:

```
Usage: hsdev --toml-file <TOML_FILE> [--toml-table <TOML_TABLE>]

Options:
  -t, --toml-file <TOML_FILE>      Path to the TOML configuration file
      --toml-table <TOML_TABLE>    Optional table name within the TOML file [default: ""]
  -h, --help                       Print help
  -V, --version                    Print version
```

## Usage Examples

### Basic Usage

Run migrations using a TOML configuration file:

```bash
hsdev --toml-file config/database.toml
```

### With Specific TOML Table

Run migrations using a specific table within the TOML file:

```bash
hsdev --toml-file config/config.toml --toml-table database
```

## Integration with Other Crates

The `hsdev` crate integrates with several other parts of the Hyperswitch ecosystem:

1. It uses the same migration files that are used by the `storage_impl` crate
2. It supports the configuration format used by other Hyperswitch components
3. It's used in development and deployment scripts for database setup

## Error Handling

The crate handles several types of errors with clear messaging:

- File I/O errors when reading TOML files
- TOML parsing errors for malformed configuration files
- Missing table errors when specified TOML tables don't exist
- Database connection errors
- Migration directory discovery errors
- Migration execution errors

## Performance Considerations

As a utility tool for development and deployment, performance is not a primary concern. However, the crate:

- Parses configuration files efficiently
- Uses Diesel's optimized migration mechanisms
- Provides clear feedback during migration operations

## Testing Strategy

The crate includes unit tests covering:

- TOML parsing and URL generation
- Table selection from TOML structures
- Configuration parsing edge cases

## Conclusion

The `hsdev` crate serves as a utility tool for managing database migrations in the Hyperswitch ecosystem, providing a simple and consistent way to apply database schema changes across development and deployment environments.

## See Also

- [Storage Implementation Documentation](../storage_impl/overview.md)
- [Diesel Models Documentation](../diesel_models/overview.md)
