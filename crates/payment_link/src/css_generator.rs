use std::collections::HashMap;

use api_models::admin::PaymentLinkConfig;
use error_stack::Result;

#[derive(Debug, thiserror::Error)]
pub enum PaymentLinkError {
    #[error("Invalid CSS selector: {0}")]
    InvalidCssSelector(String),
    #[error("Invalid CSS property: {0}")]
    InvalidCssProperty(String),
    #[error("Invalid CSS value: {0}")]
    InvalidCssValue(String),
}

pub fn get_css_script(payment_link_config: &PaymentLinkConfig) -> Result<String, PaymentLinkError> {
    let custom_rules_css_option = payment_link_config
        .payment_link_ui_rules
        .as_ref()
        .map(generate_dynamic_css)
        .transpose()?;

    let color_scheme_css = get_color_scheme_css(payment_link_config);

    if let Some(custom_rules_css) = custom_rules_css_option {
        Ok(format!("{color_scheme_css}\n{custom_rules_css}"))
    } else {
        Ok(color_scheme_css)
    }
}

fn get_color_scheme_css(payment_link_config: &PaymentLinkConfig) -> String {
    let background_primary_color_raw = payment_link_config
        .background_colour
        .clone()
        .unwrap_or_else(|| payment_link_config.theme.clone());

    let background_primary_color =
        sanitize_color(&background_primary_color_raw).unwrap_or_else(|| "#4E6ADD".to_string());

    format!(
        ":root {{
      --primary-color: {background_primary_color};
    }}"
    )
}

fn sanitize_color(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else if let Some(hex) = trimmed.strip_prefix('#') {
        if (hex.len() == 3 || hex.len() == 4 || hex.len() == 6 || hex.len() == 8)
            && hex.chars().all(|c| c.is_ascii_hexdigit())
        {
            Some(trimmed.to_string())
        } else {
            None
        }
    } else if (trimmed.starts_with("rgb(")
        || trimmed.starts_with("rgba(")
        || trimmed.starts_with("hsl(")
        || trimmed.starts_with("hsla("))
        && trimmed.ends_with(')')
    {
        trimmed.find('(').and_then(|start_idx| {
            let inner = &trimmed[start_idx + 1..trimmed.len() - 1];
            if inner
                .chars()
                .all(|c| c.is_ascii_digit() || c == ',' || c == '.' || c == '%' || c == ' ')
            {
                Some(trimmed.to_string())
            } else {
                None
            }
        })
    } else if trimmed.chars().all(|c| c.is_ascii_alphabetic() || c == '-') {
        Some(trimmed.to_string())
    } else {
        None
    }
}

fn is_safe_css_token(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();

    !s.chars()
        .any(|c| matches!(c, '<' | '>' | '{' | '}' | ';' | '"' | '\'' | '\\' | '@'))
        && !lower.contains("/*")
        && !lower.contains("*/")
        && !lower.contains("</")
        && !lower.contains("expression(")
        && !lower.contains("javascript:")
        && !lower.contains("url(")
}

fn generate_dynamic_css(
    rules: &HashMap<String, HashMap<String, String>>,
) -> Result<String, PaymentLinkError> {
    if rules.is_empty() {
        Ok(String::new())
    } else {
        let mut css_string = String::new();
        css_string.push_str("/* Dynamically Injected UI Rules */\n");

        rules.iter().for_each(|(selector, styles_map)| {
            let selector = selector.trim();
            // Drop the entire rule if the selector itself is empty or unsafe.
            if selector.is_empty() || !is_safe_css_token(selector) {
                return;
            }

            let mut rule_block = String::new();
            let mut has_safe_declaration = false;

            styles_map.iter().for_each(|(prop_camel_case, css_value)| {
                let css_property = camel_to_kebab(prop_camel_case);

                // Keep only declarations whose normalized property name and value are both safe.
                let is_safe_property = css_property
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c == '-');

                // Only serialize declarations that pass both checks so unsafe entries are
                // silently dropped while valid styles continue to be emitted.
                if is_safe_property && is_safe_css_token(css_value) {
                    rule_block.push_str("  ");
                    rule_block.push_str(&css_property);
                    rule_block.push_str(": ");
                    rule_block.push_str(css_value);
                    rule_block.push_str(";\n");
                    has_safe_declaration = true;
                }
            });

            // Emit the selector only if at least one declaration survived sanitization.
            if has_safe_declaration {
                css_string.push_str(selector);
                css_string.push_str(" {\n");
                css_string.push_str(&rule_block);
                css_string.push_str("}\n");
            }
        });

        Ok(css_string)
    }
}

fn camel_to_kebab(s: &str) -> String {
    let mut result = String::new();
    if s.is_empty() {
        return result;
    }

    let chars: Vec<char> = s.chars().collect();

    for (i, &ch) in chars.iter().enumerate() {
        if ch.is_uppercase() {
            let should_add_dash = i > 0
                && (chars.get(i - 1).map(|c| c.is_lowercase()).unwrap_or(false)
                    || (i + 1 < chars.len()
                        && chars.get(i + 1).map(|c| c.is_lowercase()).unwrap_or(false)
                        && chars.get(i - 1).map(|c| c.is_uppercase()).unwrap_or(false)));

            if should_add_dash {
                result.push('-');
            }
            result.push(ch.to_ascii_lowercase());
        } else {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_safe_css_token() {
        assert!(is_safe_css_token("body"));
        assert!(is_safe_css_token(".class-name"));
        assert!(is_safe_css_token("#id-name"));
        assert!(is_safe_css_token("14px"));
        assert!(is_safe_css_token("solid"));

        assert!(!is_safe_css_token("body { color: red }"));
        assert!(!is_safe_css_token("</style><script>"));
        assert!(!is_safe_css_token("expression(alert(1))"));
        assert!(!is_safe_css_token("javascript:alert(1)"));
        assert!(!is_safe_css_token("url(javascript:alert)"));
        assert!(!is_safe_css_token("/* comment */"));
    }

    #[test]
    fn test_sanitize_color() {
        assert_eq!(sanitize_color("#4E6ADD").unwrap(), "#4E6ADD");
        assert_eq!(sanitize_color("red").unwrap(), "red");
        assert_eq!(
            sanitize_color("rgb(255, 255, 255)").unwrap(),
            "rgb(255, 255, 255)"
        );
        assert_eq!(
            sanitize_color("rgba(0, 0, 0, 0.5)").unwrap(),
            "rgba(0, 0, 0, 0.5)"
        );
        assert_eq!(
            sanitize_color("hsl(120, 100%, 50%)").unwrap(),
            "hsl(120, 100%, 50%)"
        );

        assert!(sanitize_color("#4E6ADD; } </style>").is_none());
        assert!(sanitize_color("rgb(255, 255, 255); alert(1)").is_none());
    }

    #[test]
    fn test_generate_dynamic_css_safe() {
        let mut rules = HashMap::new();
        let mut styles = HashMap::new();
        styles.insert("backgroundColor".to_string(), "#4E6ADD".to_string());
        styles.insert("fontSize".to_string(), "14px".to_string());
        rules.insert("body".to_string(), styles);

        let css = generate_dynamic_css(&rules).unwrap();
        assert!(css.contains("body {"));
        assert!(css.contains("background-color: #4E6ADD;"));
        assert!(css.contains("font-size: 14px;"));
    }

    #[test]
    fn test_generate_dynamic_css_unsafe() {
        let mut rules = HashMap::new();
        let mut styles = HashMap::new();
        styles.insert(
            "backgroundColor".to_string(),
            "red; } </style><script>".to_string(),
        );
        rules.insert("body".to_string(), styles);

        let css = generate_dynamic_css(&rules).unwrap();
        assert_eq!(css, "/* Dynamically Injected UI Rules */\n");
    }

    #[test]
    fn test_generate_dynamic_css_sanitizes_unsafe_value_and_keeps_safe_value() {
        let mut rules = HashMap::new();
        let mut styles = HashMap::new();
        styles.insert(
            "backgroundImage".to_string(),
            "javascript:confirm(1)".to_string(),
        );
        styles.insert("color".to_string(), "#4E6ADD".to_string());
        rules.insert(".safe-selector".to_string(), styles);

        let css = generate_dynamic_css(&rules).unwrap();
        assert!(css.contains(".safe-selector {"));
        assert!(css.contains("color: #4E6ADD;"));
        assert!(!css.contains("background-image"));
        assert!(!css.contains("javascript:confirm(1)"));
    }

    #[test]
    fn test_generate_dynamic_css_skips_unsafe_selector() {
        let mut rules = HashMap::new();
        let mut styles = HashMap::new();
        styles.insert("color".to_string(), "#4E6ADD".to_string());
        rules.insert("</style><script>".to_string(), styles);

        let css = generate_dynamic_css(&rules).unwrap();
        assert_eq!(css, "/* Dynamically Injected UI Rules */\n");
    }
}
