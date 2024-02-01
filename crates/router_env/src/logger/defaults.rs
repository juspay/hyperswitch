impl Default for super::config::LogFile {
        /// Returns a new instance of the struct with default values:
    /// - enabled set to true
    /// - path set to "logs"
    /// - file_name set to "debug.log"
    /// - level set to the DEBUG level from the tracing crate
    /// - filtering_directive set to None
    fn default() -> Self {
        Self {
            enabled: true,
            path: "logs".into(),
            file_name: "debug.log".into(),
            level: super::config::Level(tracing::Level::DEBUG),
            filtering_directive: None,
        }
    }
}

impl Default for super::config::LogConsole {
        /// Creates a new instance with default values.
    fn default() -> Self {
        Self {
            enabled: false,
            level: super::config::Level(tracing::Level::INFO),
            log_format: super::config::LogFormat::Json,
            filtering_directive: None,
        }
    }
}
