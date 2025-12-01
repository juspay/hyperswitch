use std::collections::HashSet;

use actix_web::http::header;
#[cfg(feature = "olap")]
use common_utils::errors::CustomResult;
use common_utils::{
    id_type::{self, GenerateId},
    validation::validate_domain_against_allowed_domains,
};
use diesel_models::generic_link::PayoutLink;
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::payment_methods::PaymentMethod;
use router_env::{instrument, tracing, which as router_env_which, Env};
use url::Url;

use super::helpers;
#[cfg(feature = "v1")]
use crate::core::payment_methods::cards::get_pm_list_context;
use crate::{
    core::{
        errors::{self, RouterResult},
        utils as core_utils,
    },
    db::StorageInterface,
    errors::StorageError,
    routes::SessionState,
    types::{api::payouts, domain, storage},
    utils,
    utils::OptionExt,
};

#[instrument(skip(db))]
pub async fn validate_uniqueness_of_payout_id_against_merchant_id(
    db: &dyn StorageInterface,
    payout_id: &id_type::PayoutId,
    merchant_id: &id_type::MerchantId,
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

#[cfg(feature = "v2")]
pub async fn validate_create_request(
    _state: &SessionState,
    _platform: &domain::Platform,
    _req: &payouts::PayoutCreateRequest,
) -> RouterResult<(
    String,
    Option<payouts::PayoutMethodData>,
    String,
    Option<domain::Customer>,
    Option<PaymentMethod>,
)> {
    todo!()
}

/// Validates the request on below checks
/// - merchant_id passed is same as the one in merchant_account table
/// - payout_id is unique against merchant_id
/// - payout_token provided is legitimate
#[cfg(feature = "v1")]
pub async fn validate_create_request(
    state: &SessionState,
    platform: &domain::Platform,
    req: &payouts::PayoutCreateRequest,
) -> RouterResult<(
    id_type::PayoutId,
    Option<payouts::PayoutMethodData>,
    id_type::ProfileId,
    Option<domain::Customer>,
    Option<PaymentMethod>,
)> {
    if req.payout_method_id.is_some() && req.confirm != Some(true) {
        return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
            message: "Confirm must be true for recurring payouts".to_string(),
        }));
    }
    let merchant_id = platform.get_processor().get_account().get_id();

    if let Some(payout_link) = &req.payout_link {
        if *payout_link {
            validate_payout_link_request(req)?;
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
    let payout_id = match req.payout_id.as_ref() {
        Some(provided_payout_id) => provided_payout_id.clone(),
        None => id_type::PayoutId::generate(),
    };

    match validate_uniqueness_of_payout_id_against_merchant_id(
        db,
        &payout_id,
        merchant_id,
        platform.get_processor().get_account().storage_scheme,
    )
    .await
    .attach_printable_lazy(|| {
        format!(
            "Unique violation while checking payout_id: {payout_id:?} against merchant_id: {merchant_id:?}"
        )
    })? {
        Some(_) => Err(report!(errors::ApiErrorResponse::DuplicatePayout {
            payout_id: payout_id.clone()
        })),
        None => Ok(()),
    }?;

    // Fetch customer details (merge of loose fields + customer object) and create DB entry
    let customer_in_request = helpers::get_customer_details_from_request(req);
    let customer = if customer_in_request.customer_id.is_some()
        || customer_in_request.name.is_some()
        || customer_in_request.email.is_some()
        || customer_in_request.phone.is_some()
        || customer_in_request.phone_country_code.is_some()
    {
        helpers::get_or_create_customer_details(state, &customer_in_request, platform).await?
    } else {
        None
    };

    #[cfg(feature = "v1")]
    let profile_id = core_utils::get_profile_id_from_business_details(
        req.business_country,
        req.business_label.as_ref(),
        platform,
        req.profile_id.as_ref(),
        &*state.store,
        false,
    )
    .await?;

    #[cfg(feature = "v2")]
    // Profile id will be mandatory in v2 in the request / headers
    let profile_id = req
        .profile_id
        .clone()
        .ok_or(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "profile_id",
        })
        .attach_printable("Profile id is a mandatory parameter")?;

    let payment_method: Option<PaymentMethod> =
        match (req.payout_token.as_ref(), req.payout_method_id.clone()) {
            (Some(_), Some(_)) => Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "Only one of payout_method_id or payout_token should be provided."
                    .to_string(),
            })),
            (None, Some(payment_method_id)) => match customer.as_ref() {
                Some(customer) => {
                    let payment_method = db
                        .find_payment_method(
                            platform.get_processor().get_key_store(),
                            &payment_method_id,
                            platform.get_processor().get_account().storage_scheme,
                        )
                        .await
                        .change_context(errors::ApiErrorResponse::PaymentMethodNotFound)
                        .attach_printable("Unable to find payment method")?;

                    utils::when(payment_method.customer_id != customer.customer_id, || {
                        Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                        message: "Payment method does not belong to this customer_id".to_string(),
                    })
                    .attach_printable(
                        "customer_id in payment_method does not match with customer_id in request",
                    ))
                    })?;
                    Ok(Some(payment_method))
                }
                None => Err(report!(errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "customer_id when payment_method_id is passed",
                })),
            },
            _ => Ok(None),
        }?;

    // payout_token
    let payout_method_data = match (
        req.payout_token.as_ref(),
        customer.as_ref(),
        payment_method.as_ref(),
    ) {
        (Some(_), None, _) => Err(report!(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "customer or customer_id when payout_token is provided"
        })),
        (Some(payout_token), Some(customer), _) => {
            helpers::make_payout_method_data(
                state,
                req.payout_method_data.as_ref(),
                Some(payout_token),
                &customer.customer_id,
                platform.get_processor().get_account().get_id(),
                req.payout_type,
                platform.get_processor().get_key_store(),
                None,
                platform.get_processor().get_account().storage_scheme,
            )
            .await
        }
        (_, Some(_), Some(payment_method)) => {
            // Check if we have a stored transfer_method_id first
            if payment_method
                .get_common_mandate_reference()
                .ok()
                .and_then(|common_mandate_ref| common_mandate_ref.payouts)
                .map(|payouts_mandate_ref| !payouts_mandate_ref.0.is_empty())
                .unwrap_or(false)
            {
                Ok(None)
            } else {
                // No transfer_method_id available, proceed with vault fetch for raw card details
                match get_pm_list_context(
                    state,
                    payment_method
                        .payment_method
                        .as_ref()
                        .get_required_value("payment_method_id")?,
                    platform.get_processor().get_key_store(),
                    payment_method,
                    None,
                    false,
                    true,
                    platform,
                )
                .await?
                {
                    Some(pm) => match (pm.card_details, pm.bank_transfer_details) {
                        (Some(card), _) => Ok(Some(payouts::PayoutMethodData::Card(
                            api_models::payouts::CardPayout {
                                card_number: card.card_number.get_required_value("card_number")?,
                                card_holder_name: card.card_holder_name,
                                expiry_month: card
                                    .expiry_month
                                    .get_required_value("expiry_month")?,
                                expiry_year: card.expiry_year.get_required_value("expiry_year")?,
                                card_network: card.card_network.clone(),
                            },
                        ))),
                        (_, Some(bank)) => Ok(Some(payouts::PayoutMethodData::Bank(bank))),
                        _ => Ok(None),
                    },
                    None => Ok(None),
                }
            }
        }
        _ => Ok(None),
    }?;

    Ok((
        payout_id,
        payout_method_data,
        profile_id,
        customer,
        payment_method,
    ))
}

pub fn validate_payout_link_request(
    req: &payouts::PayoutCreateRequest,
) -> Result<(), errors::ApiErrorResponse> {
    if req.confirm.unwrap_or(false) {
        return Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "cannot confirm a payout while creating a payout link".to_string(),
        });
    }

    if req.customer_id.is_none() {
        return Err(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "customer or customer_id when payout_link is true",
        });
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
                message: format!("limit should be in between 1 and {PAYOUTS_LIST_MAX_LIMIT_GET}"),
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
            message: format!("limit should be in between 1 and {PAYOUTS_LIST_MAX_LIMIT_POST}"),
        })
    })?;
    Ok(())
}

pub fn validate_payout_link_render_request_and_get_allowed_domains(
    request_headers: &header::HeaderMap,
    payout_link: &PayoutLink,
) -> RouterResult<HashSet<String>> {
    let link_id = payout_link.link_id.to_owned();
    let link_data = payout_link.link_data.to_owned();

    let is_test_mode_enabled = link_data.test_mode.unwrap_or(false);

    match (router_env_which(), is_test_mode_enabled) {
        // Throw error in case test_mode was enabled in production
        (Env::Production, true) => Err(report!(errors::ApiErrorResponse::LinkConfigurationError {
            message: "test_mode cannot be true for rendering payout_links in production"
                .to_string()
        })),
        // Skip all validations when test mode is enabled in non prod env
        (_, true) => Ok(HashSet::new()),
        // Otherwise, perform validations
        (_, false) => {
            // Fetch destination is "iframe"
            match request_headers.get("sec-fetch-dest").and_then(|v| v.to_str().ok()) {
                Some("iframe") => Ok(()),
                Some(requestor) => Err(report!(errors::ApiErrorResponse::AccessForbidden {
                    resource: "payout_link".to_string(),
                }))
                .attach_printable_lazy(|| {
                    format!(
                        "Access to payout_link [{link_id}] is forbidden when requested through {requestor}",

                    )
                }),
                None => Err(report!(errors::ApiErrorResponse::AccessForbidden {
                    resource: "payout_link".to_string(),
                }))
                .attach_printable_lazy(|| {
                    format!(
                        "Access to payout_link [{link_id}] is forbidden when sec-fetch-dest is not present in request headers",

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
                            "Access to payout_link [{link_id}] is forbidden when origin or referer is not present in request headers",

                        )
                    })?;

                let url = Url::parse(origin_or_referer)
                    .map_err(|_| {
                        report!(errors::ApiErrorResponse::AccessForbidden {
                            resource: "payout_link".to_string(),
                        })
                    })
                    .attach_printable_lazy(|| {
                        format!("Invalid URL found in request headers {origin_or_referer}")
                    })?;

                url.host_str()
                    .and_then(|host| url.port().map(|port| format!("{host}:{port}")))
                    .or_else(|| url.host_str().map(String::from))
                    .ok_or_else(|| {
                        report!(errors::ApiErrorResponse::AccessForbidden {
                            resource: "payout_link".to_string(),
                        })
                    })
                    .attach_printable_lazy(|| {
                        format!("host or port not found in request headers {url:?}")
                    })?
            };

            if validate_domain_against_allowed_domains(
                &domain_in_req,
                link_data.allowed_domains.clone(),
            ) {
                Ok(link_data.allowed_domains)
            } else {
                Err(report!(errors::ApiErrorResponse::AccessForbidden {
                    resource: "payout_link".to_string(),
                }))
                .attach_printable_lazy(|| {
                    format!(
                        "Access to payout_link [{link_id}] is forbidden from requestor - {domain_in_req}",

                    )
                })
            }
        }
    }
}
