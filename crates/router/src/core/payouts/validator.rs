use api_models::admin;
#[cfg(feature = "olap")]
use common_utils::errors::CustomResult;
use common_utils::ext_traits::ValueExt;
use diesel_models::{enums::CollectLinkConfig, generic_link::PaymentMethodCollectLinkData};
use error_stack::{report, ResultExt};
pub use hyperswitch_domain_models::errors::StorageError;
use masking::Secret;
use router_env::{instrument, tracing};

use super::helpers;
use crate::{
    consts,
    core::{
        errors::{self, RouterResult},
        payment_methods::create_pm_collect_db_entry,
        utils as core_utils,
    },
    db::StorageInterface,
    routes::AppState,
    types::{api::payouts, domain, storage},
    utils,
};

#[instrument(skip(db))]
pub async fn validate_uniqueness_of_payout_id_against_merchant_id(
    db: &dyn StorageInterface,
    payout_id: &str,
    merchant_id: &str,
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
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    req: &payouts::PayoutCreateRequest,
    merchant_key_store: &domain::MerchantKeyStore,
) -> RouterResult<(String, Option<payouts::PayoutMethodData>, String)> {
    let merchant_id = &merchant_account.merchant_id;

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
            "Unique violation while checking payout_id: {} against merchant_id: {}",
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
            let customer_id = req.customer_id.to_owned().map_or("".to_string(), |c| c);
            helpers::make_payout_method_data(
                state,
                req.payout_method_data.as_ref(),
                Some(&payout_token),
                &customer_id,
                &merchant_account.merchant_id,
                req.payout_type.as_ref(),
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
        if !cnf {
            return Ok(());
        } else {
            return Err(errors::ApiErrorResponse::InvalidRequestData {
                message: "cannot confirm a payout while creating a payout link".to_string(),
            });
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

pub async fn create_payout_link(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    req: &api_models::payouts::PayoutCreatePayoutLinkConfig,
    customer_id: &String,
    return_url: Option<String>,
    payout_id: &String,
) -> RouterResult<Option<PaymentMethodCollectLinkData>> {
    // Create payment method collect link ID
    let payout_link_id =
        core_utils::get_or_generate_id("payout_link_id", &req.payout_link_id, "payout_link")?;

    // Fetch all configs
    let default_config = &state.conf.generic_link.payment_method_collect;
    let merchant_config = merchant_account
        .pm_collect_link_config
        .clone()
        .map(|config| {
            config.parse_value::<admin::MerchantCollectLinkConfig>("MerchantCollectLinkConfig")
        })
        .transpose()
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "pm_collect_link_config in merchant_account",
        })?;
    let ui_config = &req.ui_config;
    // Create client secret
    let client_secret = utils::generate_id(consts::ID_LENGTH, "payout_link_secret");

    let fallback_ui_config = match &merchant_account.pm_collect_link_config {
        Some(config) => {
            config
                .clone()
                .parse_value::<admin::MerchantCollectLinkConfig>("MerchantCollectLinkConfig")
                .change_context(errors::ApiErrorResponse::InvalidDataValue {
                    field_name: "pm_collect_link_config in merchant_account",
                })?
                .ui_config
        }
        None => default_config.ui_config.clone(),
    };

    // Form data to be injected in HTML
    let sdk_host = default_config.sdk_url.clone();

    let domain = merchant_config
        .clone()
        .and_then(|c| c.domain_name.clone())
        .unwrap_or_else(|| state.conf.server.base_url.clone());

    let (collector_name, logo, theme) = match ui_config {
        Some(config) => (
            config.collector_name.clone(),
            config.logo.clone(),
            config.theme.clone(),
        ),
        None => (
            fallback_ui_config.collector_name.clone(),
            fallback_ui_config.logo.clone(),
            fallback_ui_config.theme.clone(),
        ),
    };

    let session_expiry = match req.session_expiry {
        Some(expiry) => expiry,
        None => default_config.expiry,
    };
    let merchant_id = merchant_account.merchant_id.clone();
    let link = Secret::new(format!("{domain}/payout_link/{merchant_id}/{payout_id}"));

    let payout_link_config = CollectLinkConfig {
        theme,
        logo,
        collector_name,
    };

    let enabled_payment_methods = match (&req.enabled_payment_methods, &merchant_config) {
        (Some(enabled_payment_methods), _) => enabled_payment_methods.clone(),
        (None, Some(config)) => config.enabled_payment_methods.clone(),
        _ => default_config.enabled_payment_methods.clone(),
    };

    let data = PaymentMethodCollectLinkData {
        pm_collect_link_id: payout_link_id.clone(),
        customer_id: customer_id.to_string(),
        link,
        sdk_host,
        client_secret: Secret::new(client_secret),
        session_expiry,
        ui_config: payout_link_config,
        enabled_payment_methods,
    };

    let _db_link_data =
        create_pm_collect_db_entry(state, merchant_account, &data, return_url).await?;

    Ok(Some(data))
}
