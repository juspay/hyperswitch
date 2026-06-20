#[test]
fn test_no_plaintext_secrets_in_grafana_config() {
    // Invariant: Configuration files must never contain plaintext API keys,
    // passwords, or secret tokens committed to version control.
    let config_path = std::path::Path::new("config/grafana.ini");

    if !config_path.exists() {
        return;
    }

    let content = std::fs::read_to_string(config_path)
        .expect("Should be able to read config/grafana.ini");

    // Sentinel key names that should never carry real values
    let sensitive_keys = [
        "api_key",
        "apikey",
        "secret_key",
        "admin_password",
        "password",
        "secret",
        "token",
        "private_key",
        "access_key",
        "client_secret",
        "signing_key",
        "encryption_key",
        "auth_token",
        "bearer_token",
    ];

    // Values that indicate a real credential (not a placeholder or disabled setting)
    let placeholder_indicators = [
        "${", "{{",
        "CHANGE_ME", "CHANGEME", "changeme",
        "TODO", "FIXME", "REPLACE_", "INSERT_",
        "YOUR_", "your_",
        "EXAMPLE", "SAMPLE",
        "00000000", "XXXXXXXX",
        "<", "PLACEHOLDER", "false", "true",
    ];

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        let lower = trimmed.to_lowercase();
        for key in &sensitive_keys {
            // Match key exactly: must be followed by whitespace or '=' (not part of a longer name)
            let after_key = lower.strip_prefix(key);
            let is_exact_key = after_key
                .map(|rest| rest.starts_with('=') || rest.starts_with(' ') || rest.starts_with('\t'))
                .unwrap_or(false);
            if is_exact_key {
                if let Some(value_part) = trimmed.splitn(2, '=').nth(1) {
                    let value = value_part.trim();
                    // Empty or short values are safe (disabled / default)
                    if value.len() < 8 {
                        continue;
                    }
                    let is_placeholder = placeholder_indicators
                        .iter()
                        .any(|p| value.contains(p));
                    assert!(
                        is_placeholder,
                        "Potential plaintext secret found in config/grafana.ini on line: '{}'. \
                         Secrets must use environment variables, vault references, or placeholders.",
                        trimmed
                    );
                }
            }
        }
    }
}
