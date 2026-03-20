use api_models::payments::PaymentLinkDetails;

pub fn get_meta_tags_html(payment_details: &PaymentLinkDetails) -> String {
    format!(
        r#"<meta property="og:title" content="Payment request from {0}"/>
        <meta property="og:description" content="{1}"/>"#,
        payment_details.merchant_name.clone(),
        payment_details
            .merchant_description
            .clone()
            .unwrap_or_default()
    )
}
