impl Default for super::config::LogFile {
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
    fn default() -> Self {
        Self {
            enabled: false,
            level: super::config::Level(tracing::Level::INFO),
            log_format: super::config::LogFormat::Json,
            filtering_directive: None,
        }
    }
}
