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
    // Normalize the raw inputs first so surrounding whitespace does not affect parsing.
    let property = property.trim();
    let value = value.trim();
    if property.is_empty() || value.is_empty() {
        return None;
    }

    // Parse the declaration through lightningcss instead of trusting the raw string.
    let declaration = format!("{property}: {value};");
    let declaration_block =
        DeclarationBlock::parse_string(&declaration, ParserOptions::default()).ok()?;

    // Accept only a single fully parsed standard property declaration.
    if declaration_block.len() != 1 || has_unparsed_or_custom_property(&declaration_block) {
        return None;
    }

    // Re-serialize the parsed declaration and validate it again inside a stylesheet context.
    let serialized_declaration = declaration_block
        .to_css_string(PrinterOptions::default())
        .ok()?;
    let probe_rule = format!(":root {{ {serialized_declaration} }}");
    let probe_stylesheet = StyleSheet::parse(&probe_rule, ParserOptions::default()).ok()?;

    // Reject declarations that require dependency expansion such as vendor-specific rewrites.
    if stylesheet_has_dependencies(&probe_stylesheet) {
        None
    } else {
        Some(serialized_declaration)
    }
}

fn has_unparsed_or_custom_property(declaration_block: &DeclarationBlock<'_>) -> bool {
    // Filter out declarations that lightningcss could not fully understand or that are custom props.
    declaration_block
        .iter()
        .any(|(property, _)| matches!(property, Property::Unparsed(_) | Property::Custom(_)))
}

fn dependency_printer_options() -> PrinterOptions<'static> {
    // Turn on dependency analysis so stylesheet serialization reports generated side artifacts.
    PrinterOptions {
        analyze_dependencies: Some(DependencyOptions::default()),
        ..PrinterOptions::default()
    }
}

fn stylesheet_has_dependencies(stylesheet: &StyleSheet<'_, '_>) -> bool {
    // Treat serialization failures as unsafe, otherwise inspect whether printing produced dependencies.
    let Ok(result) = stylesheet.to_css(dependency_printer_options()) else {
        return true;
    };

    matches!(result.dependencies.as_ref(), Some(dependencies) if !dependencies.is_empty())
}

fn parse_css_rule(selector: &str, declarations: &[String]) -> Option<String> {
    // Clean up the selector input and require at least one already-validated declaration.
    let selector = selector.trim();
    if selector.is_empty() || declarations.is_empty() {
        return None;
    }

    // Parse the full rule and ensure it resolves to exactly one plain style rule.
    let rule_css = format!("{selector} {{ {} }}", declarations.join("; "));
    let stylesheet = StyleSheet::parse(&rule_css, ParserOptions::default()).ok()?;
    let style_rule = match stylesheet.rules.0.as_slice() {
        [CssRule::Style(style_rule)] => style_rule,
        _ => return None,
    };

    // Reject nested rules, declaration loss, and rules that trigger dependency generation.
    if style_rule.rules.0.is_empty()
        && style_rule.declarations.len() == declarations.len()
        && !stylesheet_has_dependencies(&stylesheet)
    {
        // Return the canonical serialized CSS so downstream injection uses normalized output.
        stylesheet
            .to_css(PrinterOptions::default())
            .ok()
            .map(|result| result.code)
    } else {
        None
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

        rules.iter().for_each(|(selector, styles_map)| {
            let declarations = styles_map
                .iter()
                .filter_map(|(prop_camel_case, css_value)| {
                    let css_property = camel_to_kebab(prop_camel_case);
                    parse_css_declaration(&css_property, css_value)
                })
                .collect::<Vec<_>>();

            if let Some(rule_css) = parse_css_rule(selector, &declarations) {
                css_string.push_str(&rule_css);
                if !rule_css.ends_with('\n') {
                    css_string.push('\n');
                }
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

    fn parse_csv_quoted_json(line: &str) -> String {
        let trimmed = line.trim();
        trimmed
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .unwrap_or(trimmed)
            .replace("\"\"", "\"")
    }

    fn assert_payment_link_ui_rules_parse(
        config: &serde_json::Value,
        context: &str,
        rule_block_count: &mut usize,
        selector_count: &mut usize,
        declaration_count: &mut usize,
        accepted_declaration_count: &mut usize,
    ) {
        if let Some(rules) = config
            .get("payment_link_ui_rules")
            .and_then(serde_json::Value::as_object)
        {
            *rule_block_count += 1;

            for (selector, styles) in rules {
                *selector_count += 1;
                let selector_probe_declaration = parse_css_declaration("color", "red").unwrap();
                assert!(
                    parse_css_rule(selector, &[selector_probe_declaration]).is_some(),
                    "payment_link_ui_rules selector failed to parse at {context}: {selector}"
                );

                let styles_object = styles.as_object().unwrap_or_else(|| {
                    panic!(
                        "payment_link_ui_rules styles must be an object at {context}: {selector}"
                    )
                });
                let declarations = styles_object
                    .iter()
                    .filter_map(|(property, value)| {
                        let css_value = value
                            .as_str()
                            .unwrap_or_else(|| panic!("payment_link_ui_rules style value must be a string at {context}: {selector}.{property}"));
                        let css_property = camel_to_kebab(property);
                        parse_css_declaration(&css_property, css_value)
                    })
                    .collect::<Vec<_>>();
                *declaration_count += styles_object.len();
                *accepted_declaration_count += declarations.len();

                if declarations.is_empty() {
                    continue;
                }

                let style_map = styles_object
                    .iter()
                    .map(|(property, value)| {
                        (
                            property.clone(),
                            value
                                .as_str()
                                .unwrap_or_else(|| panic!("payment_link_ui_rules style value must be a string at {context}: {selector}.{property}"))
                                .to_string(),
                        )
                    })
                    .collect::<HashMap<_, _>>();
                let rules = HashMap::from([(selector.clone(), style_map)]);
                let css = generate_dynamic_css(&rules).unwrap();
                assert!(
                    css != "/* Dynamically Injected UI Rules */\n",
                    "payment_link_ui_rules selector was parsed but not emitted at {context}: {selector}"
                );
            }
        }

        if let Some(business_configs) = config
            .get("business_specific_configs")
            .and_then(serde_json::Value::as_object)
        {
            for (business_key, business_config) in business_configs {
                let nested_context = format!("{context}.business_specific_configs.{business_key}");
                assert_payment_link_ui_rules_parse(
                    business_config,
                    &nested_context,
                    rule_block_count,
                    selector_count,
                    declaration_count,
                    accepted_declaration_count,
                );
            }
        }
    }

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
        assert_eq!(
            parse_css_declaration("font-size", "14px").as_deref(),
            Some("font-size: 14px")
        );
        assert_eq!(
            parse_css_declaration("border-style", "solid").as_deref(),
            Some("border-style: solid")
        );
        assert!(parse_css_declaration("padding-left", "22px !important")
            .as_deref()
            .is_some_and(|declaration| declaration.contains("!important")));
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
    fn test_payment_link_ui_rule_selectors_from_export_parse() {
        let configs = include_str!("../tests/fixtures/payment_link_configs.csv");
        let mut parsed_config_count = 0;
        let mut rule_block_count = 0;
        let mut selector_count = 0;
        let mut declaration_count = 0;
        let mut accepted_declaration_count = 0;

        for (line_index, line) in configs.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }

            let config: serde_json::Value = serde_json::from_str(&parse_csv_quoted_json(line))
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to parse payment link config fixture row {}: {error}",
                        line_index + 1
                    )
                });
            parsed_config_count += 1;

            assert_payment_link_ui_rules_parse(
                &config,
                &format!("row {}", line_index + 1),
                &mut rule_block_count,
                &mut selector_count,
                &mut declaration_count,
                &mut accepted_declaration_count,
            );
        }

        assert_eq!(parsed_config_count, 146);
        assert_eq!(rule_block_count, 106);
        assert_eq!(selector_count, 474);
        assert_eq!(declaration_count, 1238);
        assert_eq!(accepted_declaration_count, 1143);
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
    fn test_generate_dynamic_css_drops_url_dependency_and_keeps_safe_value() {
        let mut rules = HashMap::new();
        let mut styles = HashMap::new();
        styles.insert(
            "backgroundImage".to_string(),
            "url(javascript:confirm(1))".to_string(),
        );
        styles.insert("color".to_string(), "#4E6ADD".to_string());
        rules.insert(".safe-selector".to_string(), styles);

        let css = generate_dynamic_css(&rules).unwrap();
        assert!(css.contains(".safe-selector {"));
        assert!(css.contains("color: #4e6add;"));
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
