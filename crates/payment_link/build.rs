use std::env;

#[allow(clippy::panic)]
fn main() {
    // Try environment variable first, then fall back to default
    // This matches router's pattern of env vars taking precedence
    let sdk_url = env::var("ROUTER__PAYMENT_LINK__SDK_URL").unwrap_or_else(|_| {
        // Default SDK URL for development
        "https://beta.hyperswitch.io/v0/HyperLoader.js".to_string()
    });

    println!("cargo:rustc-env=SDK_URL={}", sdk_url);
    println!("cargo:rerun-if-env-changed=ROUTER__PAYMENT_LINK__SDK_URL");
    println!("cargo:rerun-if-env-changed=RUN_ENV");
}
