use actix_http::header;
use api_models::admin::PaymentLinkConfig;
use common_utils::validation::validate_domain_against_allowed_domains;
use error_stack::{report, ResultExt};
use url::Url;

use crate::{
    core::errors::{self, RouterResult},
    types::storage::PaymentLink,
};

pub fn validate_secure_payment_link_render_request(
    request_headers: &header::HeaderMap,
    payment_link: &PaymentLink,
    payment_link_config: &PaymentLinkConfig,
) -> RouterResult<()> {
    let link_id = payment_link.payment_link_id.clone();
    let allowed_domains = payment_link_config
        .allowed_domains
        .clone()
        .ok_or(report!(errors::ApiErrorResponse::InvalidRequestUrl))
        .attach_printable_lazy(|| {
            format!(
                "Secure payment link was not generated for {}\nmissing allowed_domains",
                link_id
            )
        })?;

    // Validate secure_link was generated
    if payment_link.secure_link.clone().is_none() {
        return Err(report!(errors::ApiErrorResponse::InvalidRequestUrl)).attach_printable_lazy(
            || {
                format!(
                    "Secure payment link was not generated for {}\nmissing secure_link",
                    link_id
                )
            },
        );
    }

    // Fetch destination is "iframe"
    match request_headers.get("sec-fetch-dest").and_then(|v| v.to_str().ok()) {
        Some("iframe") => Ok(()),
        Some(requestor) => Err(report!(errors::ApiErrorResponse::AccessForbidden {
            resource: "payment_link".to_string(),
        }))
        .attach_printable_lazy(|| {
            format!(
                "Access to payment_link [{}] is forbidden when requested through {}",
                link_id, requestor
            )
        }),
        None => Err(report!(errors::ApiErrorResponse::AccessForbidden {
            resource: "payment_link".to_string(),
        }))
        .attach_printable_lazy(|| {
            format!(
                "Access to payment_link [{}] is forbidden when sec-fetch-dest is not present in request headers",
                link_id
            )
        }),
    }?;

    // Validate origin / referer
    let domain_in_req = {
        let origin_or_referer = request_headers
            .get("origin")
            .or_else(|| request_headers.get("referer"))
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                report!(errors::ApiErrorResponse::AccessForbidden {
                    resource: "payment_link".to_string(),
                })
            })
            .attach_printable_lazy(|| {
                format!(
                    "Access to payment_link [{}] is forbidden when origin or referer is not present in request headers",
                    link_id
                )
            })?;

        let url = Url::parse(origin_or_referer)
            .map_err(|_| {
                report!(errors::ApiErrorResponse::AccessForbidden {
                    resource: "payment_link".to_string(),
                })
            })
            .attach_printable_lazy(|| {
                format!("Invalid URL found in request headers {}", origin_or_referer)
            })?;

        url.host_str()
            .and_then(|host| url.port().map(|port| format!("{}:{}", host, port)))
            .or_else(|| url.host_str().map(String::from))
            .ok_or_else(|| {
                report!(errors::ApiErrorResponse::AccessForbidden {
                    resource: "payment_link".to_string(),
                })
            })
            .attach_printable_lazy(|| {
                format!("host or port not found in request headers {:?}", url)
            })?
    };

    if validate_domain_against_allowed_domains(&domain_in_req, allowed_domains) {
        Ok(())
    } else {
        Err(report!(errors::ApiErrorResponse::AccessForbidden {
            resource: "payment_link".to_string(),
        }))
        .attach_printable_lazy(|| {
            format!(
                "Access to payment_link [{}] is forbidden from requestor - {}",
                link_id, domain_in_req
            )
        })
    }
}
