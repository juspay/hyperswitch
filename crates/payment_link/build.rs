use std::fs;

fn main() {
    // Use router_env to determine the current environment
    let environment = router_env::env::which();
    let env_str = environment.to_string();

    // Use the payment_link specific SDK URL configuration
    let sdk_config_path = "config/sdk_urls.toml";

    // Tell cargo to rerun if config file changes
    println!("cargo:rerun-if-changed={}", sdk_config_path);
    println!("cargo:rerun-if-env-changed=RUN_ENV");

    // Read and parse SDK URLs TOML file
    #[allow(clippy::panic)]
    let config_content = fs::read_to_string(sdk_config_path).unwrap_or_else(|e| {
        panic!(
            "Failed to read SDK config file '{}': {}",
            sdk_config_path, e
        );
    });

    #[allow(clippy::panic)]
    let config: toml::Value = toml::from_str(&config_content).unwrap_or_else(|e| {
        panic!("Failed to parse TOML in '{}': {}", sdk_config_path, e);
    });

    // Extract SDK URL for the current environment
    #[allow(clippy::panic)]
    let sdk_url = config
        .get(&env_str)
        .and_then(|env_config| env_config.get("sdk_url"))
        .and_then(|url| url.as_str())
        .unwrap_or_else(|| {
            panic!(
                "Missing [{}] section or sdk_url key in config file: {}. \
                 Please add:\n\n[{}]\nsdk_url = \"<your-sdk-url-here>\"\n\n\
                 Available environments: development, integ, sandbox, production",
                env_str, sdk_config_path, env_str
            );
        });

    // Set environment variable for compile-time access
    println!("cargo:rustc-env=SDK_URL={}", sdk_url);
}
