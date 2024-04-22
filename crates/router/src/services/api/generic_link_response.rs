use common_utils::errors::CustomResult;
use tera::{Context, Tera};

use super::{GenericExpiredLinkData, GenericLinkFormData, GenericLinks};
use crate::{core::errors, logger};

pub fn build_generic_link_html(
    boxed_generic_link_data: Box<GenericLinks>,
) -> CustomResult<String, errors::ApiErrorResponse> {
    match *boxed_generic_link_data {
        GenericLinks::ExpiredLink(link_data) => build_generic_expired_link_html(&link_data),
        GenericLinks::PaymentMethodCollect(pm_collect_data) => {
            build_pm_collect_link_html(&pm_collect_data)
        }
        GenericLinks::PaymentMethodCollectStatus(pm_collect_data) => {
            build_pm_collect_link_status_html(&pm_collect_data)
        }
    }
}

pub fn build_generic_expired_link_html(
    link_data: &GenericExpiredLinkData,
) -> CustomResult<String, errors::ApiErrorResponse> {
    let mut tera = Tera::default();
    let mut context = Context::new();

    // Build HTML
    let html_template = include_str!("../../core/generic_link/expired_link/index.html").to_string();
    let _ = tera.add_raw_template("generic_expired_link", &html_template);
    context.insert("title", &link_data.title);
    context.insert("message", &link_data.message);
    context.insert("theme", &link_data.theme);

    match tera.render("generic_expired_link", &context) {
        Ok(rendered_html) => Ok(rendered_html),
        Err(tera_error) => {
            logger::error!("{tera_error}");
            Err(errors::ApiErrorResponse::InternalServerError)?
        }
    }
}

pub fn build_pm_collect_link_html(
    link_data: &GenericLinkFormData,
) -> CustomResult<String, errors::ApiErrorResponse> {
    let mut tera = Tera::default();
    let mut context = Context::new();

    // Insert dynamic context in CSS
    let css_dynamic_context = "{{ color_scheme }}";
    let css_template =
        include_str!("../../core/generic_link/payment_method_collect/initiate/styles.css")
            .to_string();
    let final_css = format!("{}\n{}", css_dynamic_context, css_template);
    let _ = tera.add_raw_template("generic_link_styles", &final_css);
    context.insert("color_scheme", &link_data.css_data);

    let css_style_tag = match tera.render("generic_link_styles", &context) {
        Ok(css) => format!("<style>{}</style>", css),
        Err(tera_error) => {
            logger::error!("{tera_error}");
            Err(errors::ApiErrorResponse::InternalServerError)?
        }
    };

    // Insert dynamic context in JS
    let js_dynamic_context = "{{ collect_link_context }}";
    let js_template =
        include_str!("../../core/generic_link/payment_method_collect/initiate/script.js")
            .to_string();
    let final_js = format!("{}\n{}", js_dynamic_context, js_template);
    let _ = tera.add_raw_template("generic_link_script", &final_js);
    context.insert("collect_link_context", &link_data.js_data);

    let js_script_tag = match tera.render("generic_link_script", &context) {
        Ok(js) => format!("<script>{}</script>", js),
        Err(tera_error) => {
            logger::error!("{tera_error}");
            Err(errors::ApiErrorResponse::InternalServerError)?
        }
    };

    // Build HTML
    let html_template =
        include_str!("../../core/generic_link/payment_method_collect/initiate/index.html")
            .to_string();
    let _ = tera.add_raw_template("payment_method_collect_link", &html_template);
    context.insert("css_style_tag", &css_style_tag);
    context.insert("js_script_tag", &js_script_tag);
    context.insert(
        "hyper_sdk_loader_script_tag",
        &format!(
            r#"<script src="{}" onload="initializeCollectSDK()"></script>"#,
            link_data.sdk_url
        ),
    );

    match tera.render("payment_method_collect_link", &context) {
        Ok(rendered_html) => Ok(rendered_html),
        Err(tera_error) => {
            logger::error!("{tera_error}");
            Err(errors::ApiErrorResponse::InternalServerError)?
        }
    }
}

pub fn build_pm_collect_link_status_html(
    link_data: &GenericLinkFormData,
) -> CustomResult<String, errors::ApiErrorResponse> {
    let mut tera = Tera::default();
    let mut context = Context::new();

    // Build HTML
    let html_template =
        include_str!("../../core/generic_link/payment_method_collect/status/index.html")
            .to_string();
    let _ = tera.add_raw_template("payment_method_collect_status_link", &html_template);

    match tera.render("payment_method_collect_status_link", &context) {
        Ok(rendered_html) => Ok(rendered_html),
        Err(tera_error) => {
            logger::error!("{tera_error}");
            Err(errors::ApiErrorResponse::InternalServerError)?
        }
    }
}
