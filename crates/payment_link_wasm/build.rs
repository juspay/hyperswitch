use router::configs::settings::Settings;
use hyperswitch_interfaces::secrets_interface::secret_state::SecuredSecret;

fn main() {
    // Load configuration using the existing Settings infrastructure
    // This automatically handles environment detection (RUN_ENV) and config file loading
    let settings = Settings::<SecuredSecret>::with_config_path(None)
        .expect("Failed to load settings from config files");

    // Extract SDK URL from payment_link configuration
    let sdk_url = settings.payment_link.sdk_url.to_string();

    // Embed SDK URL as environment variable for compilation
    println!("cargo:rustc-env=SDK_URL={}", sdk_url);

    // Rerun build if environment changes
    println!("cargo:rerun-if-env-changed=RUN_ENV");
}
