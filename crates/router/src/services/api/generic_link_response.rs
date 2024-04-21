use crate::core::errors;
use common_utils::errors::CustomResult;
use tera::{Context, Tera};

use super::{GenericLinkFormData, GenericLinks};

pub fn build_generic_link_html(
    boxed_generic_link_data: Box<GenericLinks>,
) -> CustomResult<String, errors::ApiErrorResponse> {
    match *boxed_generic_link_data {
        GenericLinks::PaymentMethodCollect(pm_collect_data) => {
            build_pm_collect_link_html(pm_collect_data)
        }
    }
}

pub fn build_pm_collect_link_html(
    generic_link_data: GenericLinkFormData,
) -> CustomResult<String, errors::ApiErrorResponse> {
    let mut tera = Tera::default();
    let mut context = Context::new();
    match tera.render("payment_method_collect_link", &context) {
        Ok(rendered_html) => Ok(rendered_html),
        Err(tera_error) => {
            crate::logger::warn!("{tera_error}");
            Err(errors::ApiErrorResponse::InternalServerError)?
        }
    }
}
