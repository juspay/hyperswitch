# Config Importer Overview

The `config_importer` crate provides a utility to convert TOML configuration files into environment variables for deployment in containerized environments. This document provides an overview of its purpose, structure, and usage within the Hyperswitch ecosystem.

---
**Last Updated:** 2025-05-27  
**Documentation Status:** Complete
---

## Purpose

The `config_importer` crate is responsible for:

1. Parsing TOML configuration files into key-value pairs
2. Converting nested TOML structures into flattened environment variables
3. Applying consistent naming conventions to environment variables
4. Outputting environment variables in Kubernetes-compatible JSON format
5. Supporting deployment configuration for containerized Hyperswitch instances

## Key Modules

The `config_importer` crate is organized into the following key modules:

- **main.rs**: Core functionality for TOML parsing and environment variable generation
- **cli.rs**: Command-line interface for the tool using the clap framework

## Core Features

### TOML Parsing and Transformation

The crate provides robust parsing of TOML configuration files with support for:

- Nested table structures
- Arrays and primitive values
- Datetime values
- Preserving the original order of entries (with the `preserve_order` feature)

### Environment Variable Generation

Converts TOML structures to environment variables following these rules:

- Flattens nested structures using double underscores (`__`) as separators
- Converts keys to uppercase for environment variable convention
- Applies a configurable prefix to all variables (default: "ROUTER")
- Handles arrays by converting them to comma-separated values

### Kubernetes Integration

Outputs environment variables in Kubernetes-compatible format:

- JSON format with `name` and `value` fields for each variable
- Suitable for direct use in Kubernetes deployment configurations
- Supports both stdout output and file output

## Public Interface

The crate provides a command-line interface with the following arguments:

```
Usage: config_importer [OPTIONS] --input-file <FILE>

Options:
  -i, --input-file <FILE>        Input TOML configuration file
  -f, --output-format <FORMAT>   The format to convert the environment variables to [default: kubernetes-json]
  -o, --output-file <FILE>       Output file. Output will be written to stdout if not specified
  -p, --prefix <PREFIX>          Prefix to be used for each environment variable in the generated output [default: ROUTER]
  -h, --help                     Print help
```

## Usage Examples

### Basic Usage

Convert a TOML configuration file to Kubernetes environment variables:

```bash
config_importer --input-file config/development.toml
```

### Custom Prefix

Use a custom prefix for all environment variables:

```bash
config_importer --input-file config/development.toml --prefix HYPERSWITCH
```

### Output to File

Write the output to a file instead of stdout:

```bash
config_importer --input-file config/development.toml --output-file env_vars.json
```

## Integration with Other Crates

The `config_importer` crate is primarily a standalone utility that integrates with the Hyperswitch deployment pipeline:

1. It consumes configuration files that define settings for various Hyperswitch components
2. It produces environment variable configurations used by containerized deployments
3. It indirectly interacts with the `router_env` crate which consumes these environment variables at runtime

## Configuration Options

The crate provides the following feature flags:

- **preserve_order** (enabled by default): Maintains the original order of TOML entries in the output using the indexmap crate.

## Error Handling

The crate uses the `anyhow` library for error handling, providing:

- Contextual error messages with source tracking
- Graceful handling of file I/O errors
- Detailed parsing error reporting

## Performance Considerations

The crate is optimized for configuration processing, not high-throughput data processing:

- **Memory Efficiency**: Processes the TOML file in a streaming fashion
- **Size Handling**: Capable of handling large configuration files with deeply nested structures
- **Order Preservation**: Optional order preservation with minimal performance impact

## Thread Safety and Async Support

The crate is designed as a synchronous command-line tool and does not provide asynchronous APIs.

## Testing Strategy

The crate is tested through unit tests covering:

- TOML parsing edge cases
- Transformation of various TOML structures to environment variables
- Command-line argument handling

## Conclusion

The `config_importer` crate serves as a critical utility for containerized deployments of Hyperswitch, translating human-readable TOML configuration into machine-friendly environment variables suitable for Kubernetes and other container orchestration platforms.

## See Also

- [Router Environment Documentation](../router_env/overview.md)
- [Kubernetes Deployment Guide](https://docs.hyperswitch.io/deployment)
