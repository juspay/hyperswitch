use serde::Deserialize;
use std::{env, fs, path::PathBuf};

#[derive(Debug, Deserialize)]
struct PaymentLinkBuildSettings {
    payment_link: PaymentLinkBuildConfig,
}

#[derive(Debug, Deserialize)]
struct PaymentLinkBuildConfig {
    sdk_url: String,
}

fn main() {
    let environment = env::var("RUN_ENV").unwrap_or_else(|_| "development".to_string());
    let config_path = get_config_path(&environment);

    let config_content = fs::read_to_string(&config_path).unwrap_or_else(|e| {
        panic!(
            "Failed to read config file {:?}: {}",
            config_path, e
        )
    });

    let settings: PaymentLinkBuildSettings = toml::from_str(&config_content)
        .unwrap_or_else(|e| panic!("Failed to parse config file: {}", e));

    println!("cargo:rustc-env=SDK_URL={}", settings.payment_link.sdk_url);
    println!("cargo:rerun-if-env-changed=RUN_ENV");
    println!("cargo:rerun-if-changed={}", config_path.display());
}

fn get_config_path(environment: &str) -> PathBuf {
    let config_directory = PathBuf::from("../../config");

    match environment {
        "development" => config_directory.join("development.toml"),
        _ => config_directory
            .join("deployments")
            .join(format!("{}.toml", environment)),
    }
}
