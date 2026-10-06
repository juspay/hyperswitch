use std::{collections::HashMap, fs, str::FromStr};

use router_env::env::Env;
use serde::Deserialize;

fn main() {
    // Manually deserialize ENVIRONMENT variable to Env enum
    let environment = std::env::var("ENVIRONMENT")
        .ok()
        .and_then(|env_str| Env::from_str(&env_str).ok())
        .unwrap_or_else(|| {
            #[cfg(debug_assertions)]
            let default = Env::Development;
            #[cfg(not(debug_assertions))]
            let default = Env::Production;
            default
        });

    // Use the payment_link specific SDK URL configuration
    let sdk_config_path = "config/sdk_urls.toml";

    // Tell cargo to rerun if config file changes
    println!("cargo:rerun-if-changed={}", sdk_config_path);
    println!("cargo:rerun-if-env-changed=ENVIRONMENT");

    // Read and parse SDK URLs TOML file
    #[allow(clippy::panic)]
    let config_content = fs::read_to_string(sdk_config_path).unwrap_or_else(|e| {
        panic!(
            "Failed to read SDK config file '{}': {}",
            sdk_config_path, e
        );
    });

    #[allow(clippy::panic)]
    let config: HashMap<Env, EnvConfig> = toml::from_str(&config_content).unwrap_or_else(|e| {
        panic!("Failed to parse TOML in '{}': {}", sdk_config_path, e);
    });

    // Extract SDK URL for the current environment
    #[allow(clippy::panic)]
    let sdk_url = config
        .get(&environment)
        .map(|c| c.sdk_url.as_str())
        .unwrap_or_else(|| {
            panic!(
                "Missing [{}] section in config file: {}",
                environment, sdk_config_path
            )
        });

    // Set environment variable for compile-time access
    println!("cargo:rustc-env=SDK_URL={}", sdk_url);
}

#[derive(Debug, Deserialize)]
struct EnvConfig {
    sdk_url: String,
}
