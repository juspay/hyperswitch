use std::collections::HashMap;

use api_models::admin::PaymentLinkConfig;
use error_stack::{Result, ResultExt};

#[derive(Debug, thiserror::Error)]
pub enum PaymentLinkError {
    #[error("Invalid CSS selector: {0}")]
    InvalidCssSelector(String),
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
    let background_primary_color = payment_link_config
        .background_colour
        .clone()
        .unwrap_or(payment_link_config.theme.clone());
    format!(
        ":root {{
      --primary-color: {background_primary_color};
    }}"
    )
}

fn generate_dynamic_css(
    rules: &HashMap<String, HashMap<String, String>>,
) -> Result<String, PaymentLinkError> {
    if rules.is_empty() {
        return Ok(String::new());
    }

    let mut css_string = String::new();
    css_string.push_str("/* Dynamically Injected UI Rules */\n");

    for (selector, styles_map) in rules {
        if selector.trim().is_empty() {
            return Err(PaymentLinkError::InvalidCssSelector(
                "CSS selector cannot be empty.".to_string(),
            ))
            .attach_printable("Empty CSS selector found in payment_link_ui_rules")?;
        }

        css_string.push_str(selector);
        css_string.push_str(" {\n");

        for (prop_camel_case, css_value) in styles_map {
            let css_property = camel_to_kebab(prop_camel_case);

            css_string.push_str("  ");
            css_string.push_str(&css_property);
            css_string.push_str(": ");
            css_string.push_str(css_value);
            css_string.push_str(";\n");
        }
        css_string.push_str("}\n");
    }
    Ok(css_string)
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
