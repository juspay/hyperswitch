use api_models::admin;
#[cfg(feature = "olap")]
use common_utils::errors::CustomResult;
use common_utils::{
    ext_traits::ValueExt,
    id_type::CustomerId,
    link_utils::{GenericLinkStatus, GenericLinkUiConfig, PayoutLinkData, PayoutLinkStatus},
    types::MinorUnit,
};
use diesel_models::{
    business_profile::BusinessProfile,
    generic_link::{GenericLinkNew, PayoutLink},
};
use error_stack::{report, ResultExt};
pub use hyperswitch_domain_models::errors::StorageError;
use masking::Secret;
use router_env::{instrument, tracing};
use time::Duration;

use super::helpers;
use crate::{
    consts,
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        utils as core_utils,
    },
    db::StorageInterface,
    routes::SessionState,
    types::{api::payouts, domain, storage},
    utils::{self, OptionExt},
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
    state: &SessionState,
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
            let customer_id = req
                .customer_id
                .to_owned()
                .unwrap_or_else(common_utils::generate_customer_id_of_default_length);
            helpers::make_payout_method_data(
                state,
                req.payout_method_data.as_ref(),
                Some(&payout_token),
                &customer_id,
                &merchant_account.merchant_id,
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

#[allow(clippy::too_many_arguments)]
pub async fn create_payout_link(
    state: &SessionState,
    business_profile: &BusinessProfile,
    customer_id: &CustomerId,
    merchant_id: &String,
    req: &payouts::PayoutCreateRequest,
    payout_id: &String,
) -> RouterResult<PayoutLink> {
    let payout_link_config_req = req.payout_link_config.to_owned();
    // Create payment method collect link ID
    let payout_link_id = core_utils::get_or_generate_id(
        "payout_link_id",
        &payout_link_config_req
            .as_ref()
            .and_then(|config| config.payout_link_id.clone()),
        "payout_link",
    )?;

    // Fetch all configs
    let default_config = &state.conf.generic_link.payout_link;
    let profile_config = business_profile
        .payout_link_config
        .as_ref()
        .map(|config| {
            config
                .clone()
                .parse_value::<admin::BusinessPayoutLinkConfig>("BusinessPayoutLinkConfig")
        })
        .transpose()
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "payout_link_config in business_profile",
        })?;
    let profile_ui_config = profile_config.as_ref().map(|c| c.config.ui_config.clone());
    let ui_config = payout_link_config_req
        .as_ref()
        .and_then(|config| config.ui_config.clone())
        .or(profile_ui_config);

    // Form data to be injected in the link
    let (logo, merchant_name, theme) = match ui_config {
        Some(config) => (config.logo, config.merchant_name, config.theme),
        _ => (None, None, None),
    };
    let payout_link_config = GenericLinkUiConfig {
        logo,
        merchant_name,
        theme,
    };
    let client_secret = utils::generate_id(consts::ID_LENGTH, "payout_link_secret");
    let base_url = profile_config
        .as_ref()
        .and_then(|c| c.config.domain_name.as_ref())
        .map(|domain| format!("https://{}", domain))
        .unwrap_or(state.base_url.clone());
    let session_expiry = req
        .session_expiry
        .as_ref()
        .map_or(default_config.expiry, |expiry| *expiry);
    let url = format!("{base_url}/payout_link/{merchant_id}/{payout_id}");
    let link = url::Url::parse(&url)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| format!("Failed to form payout link URL - {}", url))?;
    let req_enabled_payment_methods = payout_link_config_req
        .as_ref()
        .and_then(|req| req.enabled_payment_methods.to_owned());
    let amount = req
        .amount
        .as_ref()
        .get_required_value("amount")
        .attach_printable("amount is a required value when creating payout links")?;
    let currency = req
        .currency
        .as_ref()
        .get_required_value("currency")
        .attach_printable("currency is a required value when creating payout links")?;

    let data = PayoutLinkData {
        payout_link_id: payout_link_id.clone(),
        customer_id: customer_id.clone(),
        payout_id: payout_id.to_string(),
        link,
        client_secret: Secret::new(client_secret),
        session_expiry,
        ui_config: payout_link_config,
        enabled_payment_methods: req_enabled_payment_methods,
        amount: MinorUnit::from(*amount),
        currency: *currency,
    };

    create_payout_link_db_entry(state, merchant_id, &data, req.return_url.clone()).await
}

pub async fn create_payout_link_db_entry(
    state: &SessionState,
    merchant_id: &String,
    payout_link_data: &PayoutLinkData,
    return_url: Option<String>,
) -> RouterResult<PayoutLink> {
    let db: &dyn StorageInterface = &*state.store;

    let link_data = serde_json::to_value(payout_link_data)
        .map_err(|_| report!(errors::ApiErrorResponse::InternalServerError))
        .attach_printable("Failed to convert PayoutLinkData to Value")?;

    let payout_link = GenericLinkNew {
        link_id: payout_link_data.payout_link_id.to_string(),
        primary_reference: payout_link_data.payout_id.to_string(),
        merchant_id: merchant_id.to_string(),
        link_type: common_enums::GenericLinkType::PayoutLink,
        link_status: GenericLinkStatus::PayoutLink(PayoutLinkStatus::Initiated),
        link_data,
        url: payout_link_data.link.to_string().into(),
        return_url,
        expiry: common_utils::date_time::now()
            + Duration::seconds(payout_link_data.session_expiry.into()),
        ..Default::default()
    };

    db.insert_payout_link(payout_link)
        .await
        .to_duplicate_response(errors::ApiErrorResponse::GenericDuplicateError {
            message: "payout link already exists".to_string(),
        })
}
