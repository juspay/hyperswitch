# config_importer

A simple utility tool to import a Hyperswitch TOML configuration file, convert
it into environment variable key-value pairs, and export it in the specified
format.
As of now, it supports only exporting the environment variables to a JSON format
compatible with Kubernetes, but it can be easily extended to export to a YAML
format compatible with Kubernetes or the env file format.

## Usage

You can find the usage information from the help message by specifying the
`--help` flag:

```shell
cargo run --bin config_importer -- --help
```

### Specifying the output location

If the `--output-file` flag is not specified, the utility prints the output to
stdout.
If you would like to write the output to a file instead, you can specify the
`--output-file` flag with the path to the output file:

```shell
cargo run --bin config_importer -- --input-file config/development.toml --output-file config/development.json
```

### Specifying a different prefix

If the `--prefix` flag is not specified, the default prefix `ROUTER` is
considered, which generates the environment variables accepted by the `router`
binary/application.
If you'd want to generate environment variables for the `drainer`
binary/application, then you can specify the `--prefix` flag with value
`drainer` (or `DRAINER`, both work).

```shell
cargo run --bin config_importer -- --input-file config/drainer.toml --prefix drainer
```
