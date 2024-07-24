use std::collections::HashSet;

use actix_web::http::header;
#[cfg(feature = "olap")]
use common_utils::errors::CustomResult;
use diesel_models::generic_link::PayoutLink;
use error_stack::{report, ResultExt};
use globset::Glob;
pub use hyperswitch_domain_models::errors::StorageError;
use router_env::{instrument, logger, tracing};
use url::Url;

use super::helpers;
use crate::{
    core::{
        errors::{self, RouterResult},
        utils as core_utils,
    },
    db::StorageInterface,
    routes::SessionState,
    types::{api::payouts, domain, storage},
    utils,
};

#[instrument(skip(db))]
pub async fn validate_uniqueness_of_payout_id_against_merchant_id(
    db: &dyn StorageInterface,
    payout_id: &str,
    merchant_id: &common_utils::id_type::MerchantId,
    storage_scheme: storage::enums::MerchantStorageScheme,
) -> RouterResult<Option<storage::Payouts>> {
    let maybe_payouts = db
        .find_optional_payout_by_merchant_id_payout_id(merchant_id, payout_id, storage_scheme)
        .await;
    match maybe_payouts {
        Err(err) => {
            let storage_err = err.current_context();
            match storage_err {
                StorageError::ValueNotFound(_) => Ok(None),
                _ => Err(err
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed while finding payout_attempt, database error")),
            }
        }
        Ok(payout) => Ok(payout),
    }
}

/// Validates the request on below checks
/// - merchant_id passed is same as the one in merchant_account table
/// - payout_id is unique against merchant_id
/// - payout_token provided is legitimate
pub async fn validate_create_request(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    req: &payouts::PayoutCreateRequest,
    merchant_key_store: &domain::MerchantKeyStore,
) -> RouterResult<(String, Option<payouts::PayoutMethodData>, String)> {
    let merchant_id = merchant_account.get_id();

    if let Some(payout_link) = &req.payout_link {
        if *payout_link {
            validate_payout_link_request(req.confirm)?;
        }
    };

    // Merchant ID
    let predicate = req.merchant_id.as_ref().map(|mid| mid != merchant_id);
    utils::when(predicate.unwrap_or(false), || {
        Err(report!(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "merchant_id".to_string(),
            expected_format: "merchant_id from merchant account".to_string(),
        })
        .attach_printable("invalid merchant_id in request"))
    })?;

    // Payout ID
    let db: &dyn StorageInterface = &*state.store;
    let payout_id = core_utils::get_or_generate_uuid("payout_id", req.payout_id.as_ref())?;
    match validate_uniqueness_of_payout_id_against_merchant_id(
        db,
        &payout_id,
        merchant_id,
        merchant_account.storage_scheme,
    )
    .await
    .attach_printable_lazy(|| {
        format!(
            "Unique violation while checking payout_id: {} against merchant_id: {:?}",
            payout_id.to_owned(),
            merchant_id
        )
    })? {
        Some(_) => Err(report!(errors::ApiErrorResponse::DuplicatePayout {
            payout_id: payout_id.to_owned()
        })),
        None => Ok(()),
    }?;

    // Payout token
    let payout_method_data = match req.payout_token.to_owned() {
        Some(payout_token) => {
            let customer_id = req
                .customer_id
                .to_owned()
                .unwrap_or_else(common_utils::generate_customer_id_of_default_length);
            helpers::make_payout_method_data(
                state,
                req.payout_method_data.as_ref(),
                Some(&payout_token),
                &customer_id,
                merchant_account.get_id(),
                req.payout_type,
                merchant_key_store,
                None,
                merchant_account.storage_scheme,
            )
            .await?
        }
        None => None,
    };

    // Profile ID
    let profile_id = core_utils::get_profile_id_from_business_details(
        req.business_country,
        req.business_label.as_ref(),
        merchant_account,
        req.profile_id.as_ref(),
        &*state.store,
        false,
    )
    .await?;

    Ok((payout_id, payout_method_data, profile_id))
}

pub fn validate_payout_link_request(confirm: Option<bool>) -> Result<(), errors::ApiErrorResponse> {
    if let Some(cnf) = confirm {
        if cnf {
            return Err(errors::ApiErrorResponse::InvalidRequestData {
                message: "cannot confirm a payout while creating a payout link".to_string(),
            });
        } else {
            return Ok(());
        }
    }
    Ok(())
}

#[cfg(feature = "olap")]
pub(super) fn validate_payout_list_request(
    req: &payouts::PayoutListConstraints,
) -> CustomResult<(), errors::ApiErrorResponse> {
    use common_utils::consts::PAYOUTS_LIST_MAX_LIMIT_GET;

    utils::when(
        req.limit > PAYOUTS_LIST_MAX_LIMIT_GET || req.limit < 1,
        || {
            Err(errors::ApiErrorResponse::InvalidRequestData {
                message: format!(
                    "limit should be in between 1 and {}",
                    PAYOUTS_LIST_MAX_LIMIT_GET
                ),
            })
        },
    )?;
    Ok(())
}

#[cfg(feature = "olap")]
pub(super) fn validate_payout_list_request_for_joins(
    limit: u32,
) -> CustomResult<(), errors::ApiErrorResponse> {
    use common_utils::consts::PAYOUTS_LIST_MAX_LIMIT_POST;

    utils::when(!(1..=PAYOUTS_LIST_MAX_LIMIT_POST).contains(&limit), || {
        Err(errors::ApiErrorResponse::InvalidRequestData {
            message: format!(
                "limit should be in between 1 and {}",
                PAYOUTS_LIST_MAX_LIMIT_POST
            ),
        })
    })?;
    Ok(())
}

pub fn validate_payout_link_render_request(
    request_headers: &header::HeaderMap,
    payout_link: &PayoutLink,
) -> RouterResult<()> {
    let link_id = payout_link.link_id.to_owned();
    let link_data = payout_link.link_data.to_owned();

    // Fetch destination is "iframe"
    match request_headers.get("sec-fetch-dest").and_then(|v| v.to_str().ok()) {
        Some("iframe") => Ok(()),
        Some(requestor) => Err(report!(errors::ApiErrorResponse::AccessForbidden {
            resource: "payout_link".to_string(),
        }))
        .attach_printable_lazy(|| {
            format!(
                "Access to payout_link [{}] is forbidden when requested through {}",
                link_id, requestor
            )
        }),
        None => Err(report!(errors::ApiErrorResponse::AccessForbidden {
            resource: "payout_link".to_string(),
        }))
        .attach_printable_lazy(|| {
            format!(
                "Access to payout_link [{}] is forbidden when sec-fetch-dest is not present in request headers",
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
                    resource: "payout_link".to_string(),
                })
            })
            .attach_printable_lazy(|| {
                format!(
                    "Access to payout_link [{}] is forbidden when origin or referer is not present in request headers",
                    link_id
                )
            })?;

        let url = Url::parse(origin_or_referer)
            .map_err(|_| {
                report!(errors::ApiErrorResponse::AccessForbidden {
                    resource: "payout_link".to_string(),
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
                    resource: "payout_link".to_string(),
                })
            })
            .attach_printable_lazy(|| {
                format!("host or port not found in request headers {:?}", url)
            })?
    };

    if is_domain_allowed(&domain_in_req, link_data.allowed_domains) {
        Ok(())
    } else {
        Err(report!(errors::ApiErrorResponse::AccessForbidden {
            resource: "payout_link".to_string(),
        }))
        .attach_printable_lazy(|| {
            format!(
                "Access to payout_link [{}] is forbidden from requestor - {}",
                link_id, domain_in_req
            )
        })
    }
}

fn is_domain_allowed(domain: &str, allowed_domains: HashSet<String>) -> bool {
    allowed_domains.iter().any(|allowed_domain| {
        Glob::new(allowed_domain)
            .map(|glob| glob.compile_matcher().is_match(domain))
            .map_err(|err| logger::error!("Invalid glob pattern! - {:?}", err))
            .unwrap_or(false)
    })
}
