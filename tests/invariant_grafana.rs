#[cfg(test)]
mod security_tests {
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_no_plaintext_secrets_in_grafana_config() {
        // Invariant: Configuration files must never contain plaintext API keys,
        // passwords, or secret tokens committed to version control.
        let config_path = Path::new("config/grafana.ini");

        if !config_path.exists() {
            // If the file doesn't exist, the invariant holds trivially
            return;
        }

        let content = fs::read_to_string(config_path)
            .expect("Should be able to read config/grafana.ini");

        // Patterns that indicate plaintext secrets (key = actual_secret_value)
        let secret_patterns = vec![
            // Exact exploit patterns: API keys or passwords with actual values
            r"api_key\s*=\s*[A-Za-z0-9+/=_\-]{16,}",
            // Auth tokens with real values
            r"secret_key\s*=\s*[A-Za-z0-9+/=_\-]{16,}",
            // Admin password set to a real value (not a placeholder)
            r"admin_password\s*=\s*[^\s${\}]{8,}",
            // Generic secret/token assignments with real-looking values
            r"(?i)(token|secret|password|api.?key)\s*=\s*[A-Za-z0-9+/=_\-]{20,}",
        ];

        let placeholder_indicators = vec!["${", "{{", "CHANGE_ME", "your_", "<", "PLACEHOLDER"];

        for pattern in &secret_patterns {
            let re = regex::Regex::new(pattern).unwrap();
            for mat in re.find_iter(&content) {
                let matched = mat.as_str();
                // Allow if it contains placeholder indicators
                let is_placeholder = placeholder_indicators
                    .iter()
                    .any(|p| matched.contains(p));
                assert!(
                    is_placeholder,
                    "Potential plaintext secret found in config/grafana.ini: '{}'. \
                     Secrets must use environment variables, vault references, or placeholders.",
                    matched
                );
            }
        }
    }
}}}}