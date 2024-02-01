use error_stack::ResultExt;

use crate::{
    core::errors::{self, RouterResult},
    headers, logger,
    types::{self, api},
    utils::{Encode, ValueExt},
};

/// Populates the IP address into the browser information of the given PaymentsRequest payload
pub fn populate_ip_into_browser_info(
    req: &actix_web::HttpRequest,
    payload: &mut api::PaymentsRequest,
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
        });

    // Parse the IP Address from the "X-Forwarded-For" header
    // This header will contain multiple IP addresses for each ALB hop which has
    // a comma separated list of IP addresses: 'X.X.X.X, Y.Y.Y.Y, Z.Z.Z.Z'
    // The first one here will be the client IP which we want to retrieve
    let ip_address_from_header = req.headers()
        .get(headers::X_FORWARDED_FOR)
        .map(|val| val.to_str())
        .transpose()
        .unwrap_or_else(|e| {
            logger::error!(error=?e, message="failed to retrieve ip address from X-Forwarded-For header");
            None
        })
        .and_then(|ips| ips.split(',').next());

    browser_info.ip_address = browser_info.ip_address.or_else(|| {
        ip_address_from_header
            .map(|ip| ip.parse())
            .transpose()
            .unwrap_or_else(|e| {
                logger::error!(error=?e, message="failed to parse ip address from X-Forwarded-For");
                None
            })
    });

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
            .or_else(|| ip_address_from_header.map(|ip| masking::Secret::new(ip.to_string())));
    }

    let encoded = Encode::<types::BrowserInformation>::encode_to_value(&browser_info)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "failed to re-encode browser information to json after setting ip address",
        )?;

    payload.browser_info = Some(encoded);
    Ok(())
}
