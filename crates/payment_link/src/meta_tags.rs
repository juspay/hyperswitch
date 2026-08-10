use api_models::payments::PaymentLinkDetails;

fn escape_html(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#x27;"),
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            _ => escaped.push(c),
        }
    }
    escaped
}

pub fn get_meta_tags_html(payment_details: &PaymentLinkDetails) -> String {
    format!(
        r#"<meta property="og:title" content="Payment request from {0}"/>
        <meta property="og:description" content="{1}"/>"#,
        escape_html(&payment_details.merchant_name),
        escape_html(
            &payment_details
                .merchant_description
                .clone()
                .unwrap_or_default()
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_html() {
        let input = "f'S\"/><script src={PAYLOAD_URL}></script><meta x=\"'";
        let expected = "f&#x27;S&quot;/&gt;&lt;script src={PAYLOAD_URL}&gt;&lt;/script&gt;&lt;meta x=&quot;&#x27;";
        assert_eq!(escape_html(input), expected);
    }
}
