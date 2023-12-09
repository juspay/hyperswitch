mod cli;

use std::io::{BufWriter, Write};

use anyhow::Context;

/// The separator used in environment variable names.
const ENV_VAR_SEPARATOR: &str = "__";

#[cfg(not(feature = "preserve_order"))]
type EnvironmentVariableMap = std::collections::HashMap<String, String>;

#[cfg(feature = "preserve_order")]
type EnvironmentVariableMap = indexmap::IndexMap<String, String>;

fn main() -> anyhow::Result<()> {
    let args = <cli::Args as clap::Parser>::parse();

    // Read input TOML file
    let toml_contents =
        std::fs::read_to_string(args.input_file).context("Failed to read input file")?;
    let table = toml_contents
        .parse::<toml::Table>()
        .context("Failed to parse TOML file contents")?;

    // Parse TOML file contents to a `HashMap` of environment variable name and value pairs
    let env_vars = table
        .iter()
        .flat_map(|(key, value)| process_toml_value(&args.prefix, key, value))
        .collect::<EnvironmentVariableMap>();

    let writer: BufWriter<Box<dyn Write>> = match args.output_file {
        // Write to file if output file is specified
        Some(file) => BufWriter::new(Box::new(
            std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(file)
                .context("Failed to open output file")?,
        )),
        // Write to stdout otherwise
        None => BufWriter::new(Box::new(std::io::stdout().lock())),
    };

    // Write environment variables in specified format
    match args.output_format {
        cli::OutputFormat::KubernetesJson => {
            let k8s_env_vars = env_vars
                .into_iter()
                .map(|(name, value)| KubernetesEnvironmentVariable { name, value })
                .collect::<Vec<_>>();
            serde_json::to_writer_pretty(writer, &k8s_env_vars)
                .context("Failed to serialize environment variables as JSON")?
        }
    }

    Ok(())
}

fn process_toml_value(
    prefix: impl std::fmt::Display + Clone,
    key: impl std::fmt::Display + Clone,
    value: &toml::Value,
) -> Vec<(String, String)> {
    let key_with_prefix = format!("{prefix}{ENV_VAR_SEPARATOR}{key}").to_ascii_uppercase();

    match value {
        toml::Value::String(s) => vec![(key_with_prefix, s.to_owned())],
        toml::Value::Integer(i) => vec![(key_with_prefix, i.to_string())],
        toml::Value::Float(f) => vec![(key_with_prefix, f.to_string())],
        toml::Value::Boolean(b) => vec![(key_with_prefix, b.to_string())],
        toml::Value::Datetime(dt) => vec![(key_with_prefix, dt.to_string())],
        toml::Value::Array(values) => {
            if values.is_empty() {
                return vec![(key_with_prefix, String::new())];
            }

            // This logic does not support / account for arrays of tables or arrays of arrays.
            let (_processed_keys, processed_values) = values
                .iter()
                .flat_map(|v| process_toml_value(prefix.clone(), key.clone(), v))
                .unzip::<_, _, Vec<String>, Vec<String>>();
            vec![(key_with_prefix, processed_values.join(","))]
        }
        toml::Value::Table(map) => map
            .into_iter()
            .flat_map(|(k, v)| process_toml_value(key_with_prefix.clone(), k, v))
            .collect(),
    }
}

/// The Kubernetes environment variable structure containing a name and a value.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct KubernetesEnvironmentVariable {
    name: String,
    value: String,
}
