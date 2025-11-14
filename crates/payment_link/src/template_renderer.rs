use error_stack::{Result, ResultExt};
use tera::{Context, Tera};

use crate::types::{PaymentLinkFormData, PaymentLinkStatusData};

#[derive(Debug, thiserror::Error)]
pub enum PaymentLinkError {
    #[error("Failed to render template")]
    TemplateRenderError,
    #[error("Failed to build template")]
    TemplateBuildError,
}

pub fn build_payment_link_html(
    payment_link_data: PaymentLinkFormData,
) -> Result<String, PaymentLinkError> {
    let (tera, mut context) = build_payment_link_template(payment_link_data)
        .change_context(PaymentLinkError::TemplateBuildError)
        .attach_printable("Failed to build payment link's HTML template")?;

    let payment_link_initiator = include_str!(
        "../../router/src/core/payment_link/payment_link_initiate/payment_link_initiator.js"
    )
    .to_string();
    context.insert("payment_link_initiator", &payment_link_initiator);

    tera.render("payment_link", &context)
        .change_context(PaymentLinkError::TemplateRenderError)
        .attach_printable("Error while rendering open payment link's HTML template")
}

pub fn build_secure_payment_link_html(
    payment_link_data: PaymentLinkFormData,
) -> Result<String, PaymentLinkError> {
    let (tera, mut context) = build_payment_link_template(payment_link_data)
        .change_context(PaymentLinkError::TemplateBuildError)
        .attach_printable("Failed to build payment link's HTML template")?;

    let payment_link_initiator = include_str!(
        "../../router/src/core/payment_link/payment_link_initiate/secure_payment_link_initiator.js"
    )
    .to_string();
    context.insert("payment_link_initiator", &payment_link_initiator);

    tera.render("payment_link", &context)
        .change_context(PaymentLinkError::TemplateRenderError)
        .attach_printable("Error while rendering secure payment link's HTML template")
}

fn build_payment_link_template(
    payment_link_data: PaymentLinkFormData,
) -> Result<(Tera, Context), PaymentLinkError> {
    let mut tera = Tera::default();

    let css_template =
        include_str!("../../router/src/core/payment_link/payment_link_initiate/payment_link.css")
            .to_string();
    let _ = tera.add_raw_template("payment_link_css", &css_template);
    let mut context = Context::new();
    context.insert("css_color_scheme", &payment_link_data.css_script);

    let rendered_css = tera
        .render("payment_link_css", &context)
        .change_context(PaymentLinkError::TemplateRenderError)?;

    let js_template =
        include_str!("../../router/src/core/payment_link/payment_link_initiate/payment_link.js")
            .to_string();
    let _ = tera.add_raw_template("payment_link_js", &js_template);

    context.insert("payment_details_js_script", &payment_link_data.js_script);
    let sdk_origin = payment_link_data
        .sdk_url
        .host_str()
        .ok_or(PaymentLinkError::TemplateBuildError)
        .attach_printable("Host missing for payment link SDK URL")
        .and_then(|host| {
            if host == "localhost" {
                let port = payment_link_data
                    .sdk_url
                    .port()
                    .ok_or(PaymentLinkError::TemplateBuildError)
                    .attach_printable("Port missing for localhost in SDK URL")?;
                Ok(format!(
                    "{}://{}:{}",
                    payment_link_data.sdk_url.scheme(),
                    host,
                    port
                ))
            } else {
                Ok(format!("{}://{}", payment_link_data.sdk_url.scheme(), host))
            }
        })?;
    context.insert("sdk_origin", &sdk_origin);

    let rendered_js = tera
        .render("payment_link_js", &context)
        .change_context(PaymentLinkError::TemplateRenderError)?;

    let logging_template =
        include_str!("../../router/src/services/redirection/assets/redirect_error_logs_push.js")
            .to_string();
    let locale_template = include_str!("../../router/src/core/payment_link/locale.js").to_string();

    let html_template =
        include_str!("../../router/src/core/payment_link/payment_link_initiate/payment_link.html")
            .to_string();
    let _ = tera.add_raw_template("payment_link", &html_template);

    context.insert("rendered_meta_tag_html", &payment_link_data.html_meta_tags);
    context.insert(
        "preload_link_tags",
        &get_preload_link_html_template(&payment_link_data.sdk_url),
    );
    context.insert(
        "hyperloader_sdk_link",
        &get_hyper_loader_sdk(&payment_link_data.sdk_url),
    );
    context.insert("locale_template", &locale_template);
    context.insert("rendered_css", &rendered_css);
    context.insert("rendered_js", &rendered_js);
    context.insert("logging_template", &logging_template);

    Ok((tera, context))
}

fn get_hyper_loader_sdk(sdk_url: &url::Url) -> String {
    format!("<script src=\"{sdk_url}\" onload=\"initializeSDK()\"></script>")
}

fn get_preload_link_html_template(sdk_url: &url::Url) -> String {
    format!(
        r#"<link rel="preload" href="https://fonts.googleapis.com/css2?family=Montserrat:wght@400;500;600;700;800" as="style">
            <link rel="preload" href="{sdk_url}" as="script">"#,
    )
}

pub fn get_payment_link_status(
    payment_link_data: PaymentLinkStatusData,
) -> Result<String, PaymentLinkError> {
    let mut tera = Tera::default();

    // Add modification to css template with dynamic data
    let css_template =
        include_str!("../../router/src/core/payment_link/payment_link_status/status.css")
            .to_string();
    let _ = tera.add_raw_template("payment_link_css", &css_template);
    let mut context = Context::new();
    context.insert("css_color_scheme", &payment_link_data.css_script);

    let rendered_css = tera
        .render("payment_link_css", &context)
        .change_context(PaymentLinkError::TemplateRenderError)?;

    //Locale template
    let locale_template = include_str!("../../router/src/core/payment_link/locale.js");

    // Logging template
    let logging_template =
        include_str!("../../router/src/services/redirection/assets/redirect_error_logs_push.js")
            .to_string();

    // Add modification to js template with dynamic data
    let js_template =
        include_str!("../../router/src/core/payment_link/payment_link_status/status.js")
            .to_string();
    let _ = tera.add_raw_template("payment_link_js", &js_template);
    context.insert("payment_details_js_script", &payment_link_data.js_script);

    let rendered_js = tera
        .render("payment_link_js", &context)
        .change_context(PaymentLinkError::TemplateRenderError)?;

    // Modify Html template with rendered js and rendered css files
    let html_template =
        include_str!("../../router/src/core/payment_link/payment_link_status/status.html")
            .to_string();
    let _ = tera.add_raw_template("payment_link_status", &html_template);

    context.insert("rendered_css", &rendered_css);
    context.insert("locale_template", &locale_template);

    context.insert("rendered_js", &rendered_js);
    context.insert("logging_template", &logging_template);

    tera.render("payment_link_status", &context)
        .change_context(PaymentLinkError::TemplateRenderError)
        .attach_printable("Error while rendering payment link status page")
}
