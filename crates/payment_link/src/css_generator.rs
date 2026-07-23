use std::collections::HashMap;

use api_models::admin::PaymentLinkConfig;
use error_stack::Result;
use lightningcss::{
    declaration::DeclarationBlock,
    dependencies::DependencyOptions,
    properties::Property,
    rules::CssRule,
    stylesheet::{ParserOptions, PrinterOptions, StyleSheet},
    traits::ToCss,
};
use tracing as logger;

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

fn parse_css_declaration(property: &str, value: &str) -> Option<String> {
    let property = property.trim();
    let value = value.trim();
    if property.is_empty() {
        logger::error!("Payment link CSS property is empty");
        None
    } else if value.is_empty() {
        logger::error!(css_property = %property, "Payment link CSS value is empty");
        None
    } else {
        let declaration = format!("{property}: {value};");
        let declaration_block =
            DeclarationBlock::parse_string(&declaration, ParserOptions::default())
                .inspect_err(|error| {
                    logger::error!(
                        ?error,
                        css_property = %property,
                        css_value = %value,
                        "Failed to parse payment link CSS declaration"
                    );
                })
                .ok()?;

        let filtered_property_kinds = filtered_property_kinds(&declaration_block);

        if declaration_block.len() != 1 {
            logger::error!(
                css_property = %property,
                css_value = %value,
                declaration = %declaration,
                parsed_declaration_count = declaration_block.len(),
                "Payment link CSS declaration expanded into an unexpected number of declarations"
            );
            None
        } else if !filtered_property_kinds.is_empty() {
            logger::error!(
                css_property = %property,
                css_value = %value,
                declaration = %declaration,
                ?filtered_property_kinds,
                "Payment link CSS declaration was filtered out because it contains unsupported property kinds"
            );
            None
        } else {
            let serialized_declaration = declaration_block
                .to_css_string(PrinterOptions::default())
                .inspect_err(|error| {
                    logger::error!(
                        ?error,
                        css_property = %property,
                        css_value = %value,
                        "Failed to serialize payment link CSS declaration"
                    );
                })
                .ok()?;
            let probe_rule = format!(":root {{ {serialized_declaration} }}");
            let probe_stylesheet = StyleSheet::parse(&probe_rule, ParserOptions::default())
                .inspect_err(|error| {
                    logger::error!(
                        ?error,
                        css_property = %property,
                        css_value = %value,
                        serialized_declaration = %serialized_declaration,
                        "Failed to validate serialized payment link CSS declaration"
                    );
                })
                .ok()?;

            if stylesheet_has_dependencies(&probe_stylesheet) {
                logger::error!(
                    css_property = %property,
                    css_value = %value,
                    serialized_declaration = %serialized_declaration,
                    "Payment link CSS declaration was rejected because it generated stylesheet dependencies"
                );
                None
            } else {
                Some(serialized_declaration)
            }
        }
    }
}

fn filtered_property_kinds(declaration_block: &DeclarationBlock<'_>) -> Vec<&'static str> {
    // Surface which parsed property kinds were filtered so unexpected UI changes are easier to debug.
    declaration_block
        .iter()
        .filter_map(|(property, _)| match property {
            Property::Unparsed(_) => Some("unparsed"),
            Property::Custom(_) => Some("custom"),
            _ => None,
        })
        .collect()
}

fn stylesheet_has_dependencies(stylesheet: &StyleSheet<'_, '_>) -> bool {
    stylesheet
        .to_css(PrinterOptions {
            analyze_dependencies: Some(DependencyOptions::default()),
            ..PrinterOptions::default()
        })
        .map(|result| {
            matches!(
                result.dependencies.as_ref(),
                Some(dependencies) if !dependencies.is_empty()
            )
        })
        .unwrap_or(true)
}

fn parse_css_rule(selector: &str, declarations: &[String]) -> Option<String> {
    let selector = selector.trim();
    if selector.is_empty() {
        logger::error!("Payment link CSS selector is empty");
        None
    } else if declarations.is_empty() {
        logger::error!(css_selector = %selector, "Payment link CSS selector has no declarations");
        None
    } else {
        let rule_css = format!("{selector} {{ {} }}", declarations.join("; "));
        let stylesheet = StyleSheet::parse(&rule_css, ParserOptions::default())
            .inspect_err(|error| {
                logger::error!(
                    ?error,
                    css_selector = %selector,
                    rule_css = %rule_css,
                    "Failed to parse payment link CSS rule"
                );
            })
            .ok()?;

        // Accept only a single plain style rule that preserves all validated declarations.
        // Example accepted: `body { color: red; }`.
        // Example rejected: `@media screen { body { color: red; } }` or anything that gets
        // rewritten into extra/generated CSS during serialization.
        let valid_style_rule = matches!(
            stylesheet.rules.0.as_slice(),
            [CssRule::Style(style_rule)]
                if style_rule.rules.0.is_empty()
                    && style_rule.declarations.len() == declarations.len()
                    && !stylesheet_has_dependencies(&stylesheet)
        );

        if valid_style_rule {
            stylesheet
                .to_css(PrinterOptions::default())
                .map(|result| result.code)
                .inspect_err(|error| {
                    logger::error!(
                        ?error,
                        css_selector = %selector,
                        rule_css = %rule_css,
                        "Failed to serialize payment link CSS rule"
                    );
                })
                .ok()
        } else {
            match stylesheet.rules.0.as_slice() {
                [CssRule::Style(style_rule)] => {
                    logger::error!(
                        css_selector = %selector,
                        has_nested_rules = !style_rule.rules.0.is_empty(),
                        declaration_count = style_rule.declarations.len(),
                        expected_declaration_count = declarations.len(),
                        has_dependencies = stylesheet_has_dependencies(&stylesheet),
                        "Payment link CSS rule was rejected after validation"
                    );
                    None
                }
                _ => {
                    logger::error!(
                        css_selector = %selector,
                        parsed_rule_count = stylesheet.rules.0.len(),
                        "Payment link CSS selector did not resolve to exactly one style rule"
                    );
                    None
                }
            }
        }
    }
}

fn generate_dynamic_css(
    rules: &HashMap<String, HashMap<String, String>>,
) -> Result<String, PaymentLinkError> {
    if rules.is_empty() {
        Ok(String::new())
    } else {
        let mut css_string = String::new();
        css_string.push_str("/* Dynamically Injected UI Rules */\n");

        for (selector, styles_map) in rules {
            let mut declarations = Vec::with_capacity(styles_map.len());
            for (prop_camel_case, css_value) in styles_map {
                let css_property = camel_to_kebab(prop_camel_case);
                if let Some(declaration) = parse_css_declaration(&css_property, css_value) {
                    declarations.push(declaration);
                }
            }

            if let Some(rule_css) = parse_css_rule(selector, &declarations) {
                css_string.push_str(&rule_css);
                if !rule_css.ends_with('\n') {
                    css_string.push('\n');
                }
            }
        }

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
    fn test_parse_css_rule() {
        let declaration = parse_css_declaration("color", "#4E6ADD").unwrap();
        let declarations = [declaration];

        assert!(parse_css_rule("body", &declarations).is_some());
        assert!(parse_css_rule(".class-name", &declarations).is_some());
        assert!(parse_css_rule("#id-name", &declarations).is_some());
        assert!(parse_css_rule("#some-parent > .Input--invalid", &declarations).is_some());
        assert!(parse_css_rule(":has(> .TermsTextLabel)", &declarations).is_some());

        assert!(parse_css_rule("body { color: red }", &declarations).is_none());
        assert!(parse_css_rule("</style><script>", &declarations).is_none());
        assert!(parse_css_rule("@media screen", &declarations).is_none());
    }

    #[test]
    fn test_parse_css_declaration() {
        assert!(parse_css_declaration("font-size", "body { color: red }").is_none());
        assert!(parse_css_declaration("color", "</style><script>").is_none());
        assert!(parse_css_declaration("color", "red; background: blue").is_none());
        assert!(parse_css_declaration("content", "' *'").is_none());
        assert!(parse_css_declaration("width", "expression(alert(1))").is_none());
        assert!(parse_css_declaration("-ms-overflow-style", "none").is_none());
        assert!(parse_css_declaration("background-image", "javascript:alert(1)").is_none());
        assert!(parse_css_declaration("background-image", "url(javascript:alert)").is_none());
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
        assert!(css.contains("background-color: #4e6add;"));
        assert!(css.contains("font-size: 14px;"));
    }

    #[test]
    fn test_generate_dynamic_css_allows_child_combinator_selector() {
        let mut rules = HashMap::new();
        let mut styles = HashMap::new();
        styles.insert("color".to_string(), "#4E6ADD".to_string());
        rules.insert("#some-parent > .Input--invalid".to_string(), styles);

        let css = generate_dynamic_css(&rules).unwrap();
        assert!(css.contains("#some-parent > .Input--invalid {"));
        assert!(css.contains("color: #4e6add;"));
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
    fn test_generate_dynamic_css_rejects_url_dependency() {
        let mut rules = HashMap::new();
        let mut styles = HashMap::new();
        styles.insert(
            "backgroundImage".to_string(),
            "url(javascript:confirm(1))".to_string(),
        );
        styles.insert("color".to_string(), "#4E6ADD".to_string());
        rules.insert(".safe-selector".to_string(), styles);

        let css = generate_dynamic_css(&rules).unwrap();
        assert_eq!(css, "/* Dynamically Injected UI Rules */\n");
    }

    #[test]
    fn test_generate_dynamic_css_rejects_unsafe_selector() {
        let mut rules = HashMap::new();
        let mut styles = HashMap::new();
        styles.insert("color".to_string(), "#4E6ADD".to_string());
        rules.insert("</style><script>".to_string(), styles);

        let css = generate_dynamic_css(&rules).unwrap();
        assert_eq!(css, "/* Dynamically Injected UI Rules */\n");
    }
}
