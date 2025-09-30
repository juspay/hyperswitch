use error_stack::ResultExt;

use crate::{
    core::errors::{self, RouterResult},
    logger,
    types::{self, api},
    utils::{Encode, ValueExt},
};

#[cfg(feature = "v1")]
pub fn populate_browser_info(
    req: &actix_web::HttpRequest,
    payload: &mut api::PaymentsRequest,
    header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
) -> RouterResult<()> {
    let mut browser_info: types::BrowserInformation = payload
        .browser_info
        .clone()
        .map(|v| v.parse_value("BrowserInformation"))
        .transpose()
        .change_context_lazy(|| errors::ApiErrorResponse::InvalidRequestData {
            message: "invalid format for 'browser_info' provided".to_string(),
        })?
        .unwrap_or(types::BrowserInformation {
            color_depth: None,
            java_enabled: None,
            java_script_enabled: None,
            language: None,
            screen_height: None,
            screen_width: None,
            time_zone: None,
            accept_header: None,
            user_agent: None,
            ip_address: None,
            os_type: None,
            os_version: None,
            device_model: None,
            accept_language: None,
            referer: None,
        });

    let ip_address = req
        .connection_info()
        .realip_remote_addr()
        .map(ToOwned::to_owned);

    if ip_address.is_some() {
        logger::debug!("Extracted ip address from request");
    }

    browser_info.ip_address = browser_info.ip_address.or_else(|| {
        ip_address
            .as_ref()
            .map(|ip| ip.parse())
            .transpose()
            .unwrap_or_else(|error| {
                logger::error!(
                    ?error,
                    "failed to parse ip address which is extracted from the request"
                );
                None
            })
    });

    // If the locale is present in the header payload, we will use it as the accept language
    if header_payload.locale.is_some() {
        browser_info.accept_language = browser_info
            .accept_language
            .or(header_payload.locale.clone());
    }

    if let Some(api::MandateData {
        customer_acceptance:
            Some(api::CustomerAcceptance {
                online:
                    Some(api::OnlineMandate {
                        ip_address: req_ip, ..
                    }),
                ..
            }),
        ..
    }) = &mut payload.mandate_data
    {
        *req_ip = req_ip
            .clone()
            .or_else(|| ip_address.map(|ip| masking::Secret::new(ip.to_string())));
    }

    let encoded = browser_info
        .encode_to_value()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "failed to re-encode browser information to json after setting ip address",
        )?;

    payload.browser_info = Some(encoded);
    Ok(())
}
