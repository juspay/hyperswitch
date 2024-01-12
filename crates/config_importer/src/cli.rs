use std::path::PathBuf;

/// Utility to import a hyperswitch TOML configuration file, convert it into environment variable
/// key-value pairs, and export it in the specified format.
#[derive(clap::Parser, Debug)]
#[command(arg_required_else_help = true)]
pub(crate) struct Args {
    /// Input TOML configuration file.
    #[arg(short, long, value_name = "FILE")]
    pub(crate) input_file: PathBuf,

    /// The format to convert the environment variables to.
    #[arg(
        value_enum,
        short = 'f',
        long,
        value_name = "FORMAT",
        default_value = "kubernetes-json"
    )]
    pub(crate) output_format: OutputFormat,

    /// Output file. Output will be written to stdout if not specified.
    #[arg(short, long, value_name = "FILE")]
    pub(crate) output_file: Option<PathBuf>,

    /// Prefix to be used for each environment variable in the generated output.
    #[arg(short, long, default_value = "ROUTER")]
    pub(crate) prefix: String,
}

/// The output format to convert environment variables to.
#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub(crate) enum OutputFormat {
    /// Converts each environment variable to an object containing `name` and `value` fields.
    ///
    /// ```json
    /// {
    ///   "name": "ENVIRONMENT",
    ///   "value": "PRODUCTION"
    /// }
    /// ```
    KubernetesJson,
}
