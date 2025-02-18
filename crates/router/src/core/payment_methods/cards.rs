use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    str::FromStr,
};

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
use api_models::admin::PaymentMethodsEnabled;
use api_models::{
    enums as api_enums,
    payment_methods::{
        BankAccountTokenData, Card, CardDetailUpdate, CardDetailsPaymentMethod, CardNetworkTypes,
        CountryCodeWithName, ListCountriesCurrenciesRequest, ListCountriesCurrenciesResponse,
        MaskedBankDetails, PaymentExperienceTypes, PaymentMethodsData, RequestPaymentMethodTypes,
        RequiredFieldInfo, ResponsePaymentMethodIntermediate, ResponsePaymentMethodTypes,
        ResponsePaymentMethodsEnabled,
    },
    payments::BankCodeResponse,
    pm_auth::PaymentMethodAuthConfig,
    surcharge_decision_configs as api_surcharge_decision_configs,
};
use common_enums::{enums::MerchantStorageScheme, ConnectorType};
use common_utils::{
    consts,
    crypto::{self, Encryptable},
    encryption::Encryption,
    ext_traits::{AsyncExt, BytesExt, ConfigExt, Encode, StringExt, ValueExt},
    generate_id, id_type,
    request::Request,
    type_name,
    types::{
        keymanager::{Identifier, KeyManagerState},
        MinorUnit,
    },
};
use diesel_models::payment_method;
use error_stack::{report, ResultExt};
#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
use euclid::dssa::graph::{AnalysisContext, CgraphExt};
use euclid::frontend::dir;
use hyperswitch_constraint_graph as cgraph;
#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
use hyperswitch_domain_models::customer::CustomerUpdate;
use hyperswitch_domain_models::mandates::CommonMandateReference;
#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
use kgraph_utils::transformers::IntoDirValue;
use masking::Secret;
use router_env::{instrument, tracing};
use serde_json::json;
use strum::IntoEnumIterator;

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
use super::migration;
#[cfg(feature = "v1")]
use super::surcharge_decision_configs::{
    perform_surcharge_decision_management_for_payment_method_list,
    perform_surcharge_decision_management_for_saved_cards,
};
#[cfg(all(
    any(feature = "v2", feature = "v1"),
    not(feature = "payment_methods_v2"),
    not(feature = "customer_v2")
))]
use crate::routes::app::SessionStateInfo;
#[cfg(feature = "payouts")]
use crate::types::domain::types::AsyncLift;
use crate::{
    configs::{
        defaults::{get_billing_required_fields, get_shipping_required_fields},
        settings,
    },
    core::{
        errors::{self, StorageErrorExt},
        payment_methods::{network_tokenization, transformers as payment_methods, vault},
        payments::{
            helpers,
            routing::{self, SessionFlowRoutingInput},
        },
        utils as core_utils,
    },
    db, logger,
    pii::prelude::*,
    routes::{self, metrics, payment_methods::ParentPaymentMethodToken},
    services,
    types::{
        api::{self, routing as routing_types, PaymentMethodCreateExt},
        domain::{self, Profile},
        storage::{self, enums, PaymentMethodListContext, PaymentTokenData},
        transformers::ForeignTryFrom,
    },
    utils,
    utils::OptionExt,
};
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
use crate::{
    consts as router_consts, core::payment_methods as pm_core, headers,
    types::payment_methods as pm_types, utils::ConnectorResponseExt,
};
#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
use crate::{
    core::payment_methods::{
        add_payment_method_status_update_task,
        utils::{get_merchant_pm_filter_graph, make_pm_graph, refresh_pm_filters_cache},
    },
    types::transformers::ForeignFrom,
};

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2"),
    not(feature = "customer_v2")
))]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn create_payment_method(
    state: &routes::SessionState,
    req: &api::PaymentMethodCreate,
    customer_id: &id_type::CustomerId,
    payment_method_id: &str,
    locker_id: Option<String>,
    merchant_id: &id_type::MerchantId,
    pm_metadata: Option<serde_json::Value>,
    customer_acceptance: Option<serde_json::Value>,
    payment_method_data: crypto::OptionalEncryptableValue,
    key_store: &domain::MerchantKeyStore,
    connector_mandate_details: Option<serde_json::Value>,
    status: Option<enums::PaymentMethodStatus>,
    network_transaction_id: Option<String>,
    storage_scheme: MerchantStorageScheme,
    payment_method_billing_address: crypto::OptionalEncryptableValue,
    card_scheme: Option<String>,
    network_token_requestor_reference_id: Option<String>,
    network_token_locker_id: Option<String>,
    network_token_payment_method_data: crypto::OptionalEncryptableValue,
) -> errors::CustomResult<domain::PaymentMethod, errors::ApiErrorResponse> {
    let db = &*state.store;
    let customer = db
        .find_customer_by_customer_id_merchant_id(
            &state.into(),
            customer_id,
            merchant_id,
            key_store,
            storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)?;

    let client_secret = generate_id(
        consts::ID_LENGTH,
        format!("{payment_method_id}_secret").as_str(),
    );

    let current_time = common_utils::date_time::now();

    let response = db
        .insert_payment_method(
            &state.into(),
            key_store,
            domain::PaymentMethod {
                customer_id: customer_id.to_owned(),
                merchant_id: merchant_id.to_owned(),
                payment_method_id: payment_method_id.to_string(),
                locker_id,
                payment_method: req.payment_method,
                payment_method_type: req.payment_method_type,
                payment_method_issuer: req.payment_method_issuer.clone(),
                scheme: req.card_network.clone().or(card_scheme),
                metadata: pm_metadata.map(Secret::new),
                payment_method_data,
                connector_mandate_details,
                customer_acceptance: customer_acceptance.map(Secret::new),
                client_secret: Some(client_secret),
                status: status.unwrap_or(enums::PaymentMethodStatus::Active),
                network_transaction_id: network_transaction_id.to_owned(),
                payment_method_issuer_code: None,
                accepted_currency: None,
                token: None,
                cardholder_name: None,
                issuer_name: None,
                issuer_country: None,
                payer_country: None,
                is_stored: None,
                swift_code: None,
                direct_debit_token: None,
                created_at: current_time,
                last_modified: current_time,
                last_used_at: current_time,
                payment_method_billing_address,
                updated_by: None,
                version: domain::consts::API_VERSION,
                network_token_requestor_reference_id,
                network_token_locker_id,
                network_token_payment_method_data,
            },
            storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to add payment method in db")?;

    if customer.default_payment_method_id.is_none() && req.payment_method.is_some() {
        let _ = set_default_payment_method(
            state,
            merchant_id,
            key_store.clone(),
            customer_id,
            payment_method_id.to_owned(),
            storage_scheme,
        )
        .await
        .map_err(|error| logger::error!(?error, "Failed to set the payment method as default"));
    }
    Ok(response)
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
pub fn store_default_payment_method(
    req: &api::PaymentMethodCreate,
    customer_id: &id_type::CustomerId,
    merchant_id: &id_type::MerchantId,
) -> (
    api::PaymentMethodResponse,
    Option<payment_methods::DataDuplicationCheck>,
) {
    let pm_id = generate_id(consts::ID_LENGTH, "pm");
    let payment_method_response = api::PaymentMethodResponse {
        merchant_id: merchant_id.to_owned(),
        customer_id: Some(customer_id.to_owned()),
        payment_method_id: pm_id,
        payment_method: req.payment_method,
        payment_method_type: req.payment_method_type,
        #[cfg(feature = "payouts")]
        bank_transfer: None,
        card: None,
        metadata: req.metadata.clone(),
        created: Some(common_utils::date_time::now()),
        recurring_enabled: false,           //[#219]
        installment_payment_enabled: false, //[#219]
        payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]),
        last_used_at: Some(common_utils::date_time::now()),
        client_secret: None,
    };

    (payment_method_response, None)
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
pub fn store_default_payment_method(
    _req: &api::PaymentMethodCreate,
    _customer_id: &id_type::CustomerId,
    _merchant_id: &id_type::MerchantId,
) -> (
    api::PaymentMethodResponse,
    Option<payment_methods::DataDuplicationCheck>,
) {
    todo!()
}
#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
#[instrument(skip_all)]
pub async fn get_or_insert_payment_method(
    state: &routes::SessionState,
    req: api::PaymentMethodCreate,
    resp: &mut api::PaymentMethodResponse,
    merchant_account: &domain::MerchantAccount,
    customer_id: &id_type::CustomerId,
    key_store: &domain::MerchantKeyStore,
) -> errors::RouterResult<domain::PaymentMethod> {
    let mut payment_method_id = resp.payment_method_id.clone();
    let mut locker_id = None;
    let db = &*state.store;
    let key_manager_state = &(state.into());
    let payment_method = {
        let existing_pm_by_pmid = db
            .find_payment_method(
                key_manager_state,
                key_store,
                &payment_method_id,
                merchant_account.storage_scheme,
            )
            .await;

        if let Err(err) = existing_pm_by_pmid {
            if err.current_context().is_db_not_found() {
                locker_id = Some(payment_method_id.clone());
                let existing_pm_by_locker_id = db
                    .find_payment_method_by_locker_id(
                        key_manager_state,
                        key_store,
                        &payment_method_id,
                        merchant_account.storage_scheme,
                    )
                    .await;

                match &existing_pm_by_locker_id {
                    Ok(pm) => payment_method_id.clone_from(pm.get_id()),
                    Err(_) => payment_method_id = generate_id(consts::ID_LENGTH, "pm"),
                };
                existing_pm_by_locker_id
            } else {
                Err(err)
            }
        } else {
            existing_pm_by_pmid
        }
    };
    payment_method_id.clone_into(&mut resp.payment_method_id);

    match payment_method {
        Ok(pm) => Ok(pm),
        Err(err) => {
            if err.current_context().is_db_not_found() {
                insert_payment_method(
                    state,
                    resp,
                    &req,
                    key_store,
                    merchant_account.get_id(),
                    customer_id,
                    resp.metadata.clone().map(|val| val.expose()),
                    None,
                    locker_id,
                    None,
                    req.network_transaction_id.clone(),
                    merchant_account.storage_scheme,
                    None,
                    None,
                    None,
                    None,
                )
                .await
            } else {
                Err(err)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Error while finding payment method")
            }
        }
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[instrument(skip_all)]
pub async fn get_or_insert_payment_method(
    _state: &routes::SessionState,
    _req: api::PaymentMethodCreate,
    _resp: &mut api::PaymentMethodResponse,
    _merchant_account: &domain::MerchantAccount,
    _customer_id: &id_type::CustomerId,
    _key_store: &domain::MerchantKeyStore,
) -> errors::RouterResult<domain::PaymentMethod> {
    todo!()
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
pub async fn migrate_payment_method(
    state: routes::SessionState,
    req: api::PaymentMethodMigrate,
    merchant_id: &id_type::MerchantId,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
) -> errors::RouterResponse<api::PaymentMethodMigrateResponse> {
    let mut req = req;
    let card_details = &req.card.get_required_value("card")?;

    let card_number_validation_result =
        cards::CardNumber::from_str(card_details.card_number.peek());

    let card_bin_details =
        populate_bin_details_for_masked_card(card_details, &*state.store).await?;

    req.card = Some(api_models::payment_methods::MigrateCardDetail {
        card_issuing_country: card_bin_details.issuer_country.clone(),
        card_network: card_bin_details.card_network.clone(),
        card_issuer: card_bin_details.card_issuer.clone(),
        card_type: card_bin_details.card_type.clone(),
        ..card_details.clone()
    });

    if let Some(connector_mandate_details) = &req.connector_mandate_details {
        helpers::validate_merchant_connector_ids_in_connector_mandate_details(
            &state,
            key_store,
            connector_mandate_details,
            merchant_id,
            card_bin_details.card_network.clone(),
        )
        .await?;
    };

    let should_require_connector_mandate_details = req.network_token.is_none();

    let mut migration_status = migration::RecordMigrationStatusBuilder::new();

    let resp = match card_number_validation_result {
        Ok(card_number) => {
            let payment_method_create_request =
                api::PaymentMethodCreate::get_payment_method_create_from_payment_method_migrate(
                    card_number,
                    &req,
                );

            logger::debug!("Storing the card in locker and migrating the payment method");
            get_client_secret_or_add_payment_method_for_migration(
                &state,
                payment_method_create_request,
                merchant_account,
                key_store,
                &mut migration_status,
            )
            .await?
        }
        Err(card_validation_error) => {
            logger::debug!("Card number to be migrated is invalid, skip saving in locker {card_validation_error}");
            skip_locker_call_and_migrate_payment_method(
                &state,
                &req,
                merchant_id.to_owned(),
                key_store,
                merchant_account,
                card_bin_details.clone(),
                should_require_connector_mandate_details,
                &mut migration_status,
            )
            .await?
        }
    };
    let payment_method_response = match resp {
        services::ApplicationResponse::Json(response) => response,
        _ => Err(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to fetch the payment method response")?,
    };

    let pm_id = payment_method_response.payment_method_id.clone();

    let network_token = req.network_token.clone();

    let network_token_migrated = match network_token {
        Some(nt_detail) => {
            logger::debug!("Network token migration");
            let network_token_requestor_ref_id = nt_detail.network_token_requestor_ref_id.clone();
            let network_token_data = &nt_detail.network_token_data;

            Some(
                save_network_token_and_update_payment_method(
                    &state,
                    &req,
                    key_store,
                    merchant_account,
                    network_token_data,
                    network_token_requestor_ref_id,
                    pm_id,
                )
                .await
                .map_err(|err| logger::error!(?err, "Failed to save network token"))
                .ok()
                .unwrap_or_default(),
            )
        }
        None => {
            logger::debug!("Network token data is not available");
            None
        }
    };
    migration_status.network_token_migrated(network_token_migrated);
    let migrate_status = migration_status.build();

    Ok(services::ApplicationResponse::Json(
        api::PaymentMethodMigrateResponse {
            payment_method_response,
            card_migrated: migrate_status.card_migrated,
            network_token_migrated: migrate_status.network_token_migrated,
            connector_mandate_details_migrated: migrate_status.connector_mandate_details_migrated,
            network_transaction_id_migrated: migrate_status.network_transaction_migrated,
        },
    ))
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
pub async fn migrate_payment_method(
    _state: routes::SessionState,
    _req: api::PaymentMethodMigrate,
    _merchant_id: &id_type::MerchantId,
    _merchant_account: &domain::MerchantAccount,
    _key_store: &domain::MerchantKeyStore,
) -> errors::RouterResponse<api::PaymentMethodMigrateResponse> {
    todo!()
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
pub async fn populate_bin_details_for_masked_card(
    card_details: &api_models::payment_methods::MigrateCardDetail,
    db: &dyn db::StorageInterface,
) -> errors::CustomResult<api::CardDetailFromLocker, errors::ApiErrorResponse> {
    migration::validate_card_expiry(&card_details.card_exp_month, &card_details.card_exp_year)?;
    let card_number = card_details.card_number.clone();

    let (card_isin, _last4_digits) = get_card_bin_and_last4_digits_for_masked_card(
        card_number.peek(),
    )
    .change_context(errors::ApiErrorResponse::InvalidRequestData {
        message: "Invalid card number".to_string(),
    })?;

    let card_bin_details = if card_details.card_issuer.is_some()
        && card_details.card_network.is_some()
        && card_details.card_type.is_some()
        && card_details.card_issuing_country.is_some()
    {
        api_models::payment_methods::CardDetailFromLocker::foreign_try_from((card_details, None))?
    } else {
        let card_info = db
            .get_card_info(&card_isin)
            .await
            .map_err(|error| services::logger::error!(card_info_error=?error))
            .ok()
            .flatten();

        api_models::payment_methods::CardDetailFromLocker::foreign_try_from((
            card_details,
            card_info,
        ))?
    };
    Ok(card_bin_details)
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
impl
    ForeignTryFrom<(
        &api_models::payment_methods::MigrateCardDetail,
        Option<diesel_models::CardInfo>,
    )> for api_models::payment_methods::CardDetailFromLocker
{
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn foreign_try_from(
        (card_details, card_info): (
            &api_models::payment_methods::MigrateCardDetail,
            Option<diesel_models::CardInfo>,
        ),
    ) -> Result<Self, Self::Error> {
        let (card_isin, last4_digits) =
            get_card_bin_and_last4_digits_for_masked_card(card_details.card_number.peek())
                .change_context(errors::ApiErrorResponse::InvalidRequestData {
                    message: "Invalid card number".to_string(),
                })?;
        if let Some(card_bin_info) = card_info {
            Ok(Self {
                scheme: card_details
                    .card_network
                    .clone()
                    .or(card_bin_info.card_network.clone())
                    .map(|card_network| card_network.to_string()),
                last4_digits: Some(last4_digits.clone()),
                issuer_country: card_details
                    .card_issuing_country
                    .clone()
                    .or(card_bin_info.card_issuing_country),
                card_number: None,
                expiry_month: Some(card_details.card_exp_month.clone()),
                expiry_year: Some(card_details.card_exp_year.clone()),
                card_token: None,
                card_fingerprint: None,
                card_holder_name: card_details.card_holder_name.clone(),
                nick_name: card_details.nick_name.clone(),
                card_isin: Some(card_isin.clone()),
                card_issuer: card_details
                    .card_issuer
                    .clone()
                    .or(card_bin_info.card_issuer),
                card_network: card_details
                    .card_network
                    .clone()
                    .or(card_bin_info.card_network),
                card_type: card_details.card_type.clone().or(card_bin_info.card_type),
                saved_to_locker: false,
            })
        } else {
            Ok(Self {
                scheme: card_details
                    .card_network
                    .clone()
                    .map(|card_network| card_network.to_string()),
                last4_digits: Some(last4_digits.clone()),
                issuer_country: card_details.card_issuing_country.clone(),
                card_number: None,
                expiry_month: Some(card_details.card_exp_month.clone()),
                expiry_year: Some(card_details.card_exp_year.clone()),
                card_token: None,
                card_fingerprint: None,
                card_holder_name: card_details.card_holder_name.clone(),
                nick_name: card_details.nick_name.clone(),
                card_isin: Some(card_isin.clone()),
                card_issuer: card_details.card_issuer.clone(),
                card_network: card_details.card_network.clone(),
                card_type: card_details.card_type.clone(),
                saved_to_locker: false,
            })
        }
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
impl
    ForeignTryFrom<(
        &api_models::payment_methods::MigrateCardDetail,
        Option<diesel_models::CardInfo>,
    )> for api_models::payment_methods::CardDetailFromLocker
{
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn foreign_try_from(
        (card_details, card_info): (
            &api_models::payment_methods::MigrateCardDetail,
            Option<diesel_models::CardInfo>,
        ),
    ) -> Result<Self, Self::Error> {
        let (card_isin, last4_digits) =
            get_card_bin_and_last4_digits_for_masked_card(card_details.card_number.peek())
                .change_context(errors::ApiErrorResponse::InvalidRequestData {
                    message: "Invalid card number".to_string(),
                })?;
        if let Some(card_bin_info) = card_info {
            Ok(Self {
                last4_digits: Some(last4_digits.clone()),
                issuer_country: card_details
                    .card_issuing_country
                    .as_ref()
                    .map(|c| api_enums::CountryAlpha2::from_str(c))
                    .transpose()
                    .ok()
                    .flatten()
                    .or(card_bin_info
                        .card_issuing_country
                        .as_ref()
                        .map(|c| api_enums::CountryAlpha2::from_str(c))
                        .transpose()
                        .ok()
                        .flatten()),
                card_number: None,
                expiry_month: Some(card_details.card_exp_month.clone()),
                expiry_year: Some(card_details.card_exp_year.clone()),
                card_fingerprint: None,
                card_holder_name: card_details.card_holder_name.clone(),
                nick_name: card_details.nick_name.clone(),
                card_isin: Some(card_isin.clone()),
                card_issuer: card_details
                    .card_issuer
                    .clone()
                    .or(card_bin_info.card_issuer),
                card_network: card_details
                    .card_network
                    .clone()
                    .or(card_bin_info.card_network),
                card_type: card_details.card_type.clone().or(card_bin_info.card_type),
                saved_to_locker: false,
            })
        } else {
            Ok(Self {
                last4_digits: Some(last4_digits.clone()),
                issuer_country: card_details
                    .card_issuing_country
                    .as_ref()
                    .map(|c| api_enums::CountryAlpha2::from_str(c))
                    .transpose()
                    .ok()
                    .flatten(),
                card_number: None,
                expiry_month: Some(card_details.card_exp_month.clone()),
                expiry_year: Some(card_details.card_exp_year.clone()),
                card_fingerprint: None,
                card_holder_name: card_details.card_holder_name.clone(),
                nick_name: card_details.nick_name.clone(),
                card_isin: Some(card_isin.clone()),
                card_issuer: card_details.card_issuer.clone(),
                card_network: card_details.card_network.clone(),
                card_type: card_details.card_type.clone(),
                saved_to_locker: false,
            })
        }
    }
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2"),
    not(feature = "customer_v2")
))]
#[allow(clippy::too_many_arguments)]
pub async fn skip_locker_call_and_migrate_payment_method(
    state: &routes::SessionState,
    req: &api::PaymentMethodMigrate,
    merchant_id: id_type::MerchantId,
    key_store: &domain::MerchantKeyStore,
    merchant_account: &domain::MerchantAccount,
    card: api_models::payment_methods::CardDetailFromLocker,
    should_require_connector_mandate_details: bool,
    migration_status: &mut migration::RecordMigrationStatusBuilder,
) -> errors::RouterResponse<api::PaymentMethodResponse> {
    let db = &*state.store;
    let customer_id = req.customer_id.clone().get_required_value("customer_id")?;

    // In this case, since we do not have valid card details, recurring payments can only be done through connector mandate details.
    //if network token data is present, then connector mandate details are not mandatory

    let connector_mandate_details = if should_require_connector_mandate_details {
        let connector_mandate_details_req = req
            .connector_mandate_details
            .clone()
            .get_required_value("connector mandate details")?;

        Some(
            serde_json::to_value(&connector_mandate_details_req)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to parse connector mandate details")?,
        )
    } else {
        req.connector_mandate_details
            .clone()
            .map(|connector_mandate_details_req| {
                serde_json::to_value(&connector_mandate_details_req)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to parse connector mandate details")
            })
            .transpose()?
    };
    let key_manager_state = &state.into();
    let payment_method_billing_address: Option<Encryptable<Secret<serde_json::Value>>> = req
        .billing
        .clone()
        .async_map(|billing| create_encrypted_data(key_manager_state, key_store, billing))
        .await
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to encrypt Payment method billing address")?;

    let customer = db
        .find_customer_by_customer_id_merchant_id(
            &state.into(),
            &customer_id,
            &merchant_id,
            key_store,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)?;

    let payment_method_card_details =
        PaymentMethodsData::Card(CardDetailsPaymentMethod::from(card.clone()));

    let payment_method_data_encrypted: Option<Encryptable<Secret<serde_json::Value>>> = Some(
        create_encrypted_data(key_manager_state, key_store, payment_method_card_details)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to encrypt Payment method card details")?,
    );

    let payment_method_metadata: Option<serde_json::Value> =
        req.metadata.as_ref().map(|data| data.peek()).cloned();

    let network_transaction_id = req.network_transaction_id.clone();

    let payment_method_id = generate_id(consts::ID_LENGTH, "pm");

    let current_time = common_utils::date_time::now();

    let response = db
        .insert_payment_method(
            &state.into(),
            key_store,
            domain::PaymentMethod {
                customer_id: customer_id.to_owned(),
                merchant_id: merchant_id.to_owned(),
                payment_method_id: payment_method_id.to_string(),
                locker_id: None,
                payment_method: req.payment_method,
                payment_method_type: req.payment_method_type,
                payment_method_issuer: req.payment_method_issuer.clone(),
                scheme: req.card_network.clone().or(card.scheme.clone()),
                metadata: payment_method_metadata.map(Secret::new),
                payment_method_data: payment_method_data_encrypted.map(Into::into),
                connector_mandate_details: connector_mandate_details.clone(),
                customer_acceptance: None,
                client_secret: None,
                status: enums::PaymentMethodStatus::Active,
                network_transaction_id: network_transaction_id.clone(),
                payment_method_issuer_code: None,
                accepted_currency: None,
                token: None,
                cardholder_name: None,
                issuer_name: None,
                issuer_country: None,
                payer_country: None,
                is_stored: None,
                swift_code: None,
                direct_debit_token: None,
                created_at: current_time,
                last_modified: current_time,
                last_used_at: current_time,
                payment_method_billing_address: payment_method_billing_address.map(Into::into),
                updated_by: None,
                version: domain::consts::API_VERSION,
                network_token_requestor_reference_id: None,
                network_token_locker_id: None,
                network_token_payment_method_data: None,
            },
            merchant_account.storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to add payment method in db")?;

    logger::debug!("Payment method inserted in db");

    migration_status.network_transaction_id_migrated(
        network_transaction_id.and_then(|val| (!val.is_empty_after_trim()).then_some(true)),
    );

    migration_status.connector_mandate_details_migrated(
        connector_mandate_details
            .clone()
            .and_then(|val| if val == json!({}) { None } else { Some(true) })
            .or_else(|| {
                req.connector_mandate_details.clone().and_then(|val| {
                    val.payments
                        .and_then(|payin_val| (!payin_val.0.is_empty()).then_some(false))
                })
            }),
    );

    if customer.default_payment_method_id.is_none() && req.payment_method.is_some() {
        let _ = set_default_payment_method(
            state,
            &merchant_id,
            key_store.clone(),
            &customer_id,
            payment_method_id.to_owned(),
            merchant_account.storage_scheme,
        )
        .await
        .map_err(|error| logger::error!(?error, "Failed to set the payment method as default"));
    }
    Ok(services::api::ApplicationResponse::Json(
        api::PaymentMethodResponse::foreign_from((Some(card), response)),
    ))
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2"),
    not(feature = "customer_v2")
))]
#[allow(clippy::too_many_arguments)]
pub async fn save_network_token_and_update_payment_method(
    state: &routes::SessionState,
    req: &api::PaymentMethodMigrate,
    key_store: &domain::MerchantKeyStore,
    merchant_account: &domain::MerchantAccount,
    network_token_data: &api_models::payment_methods::MigrateNetworkTokenData,
    network_token_requestor_ref_id: String,
    pm_id: String,
) -> errors::RouterResult<bool> {
    let payment_method_create_request =
        api::PaymentMethodCreate::get_payment_method_create_from_payment_method_migrate(
            network_token_data.network_token_number.clone(),
            req,
        );
    let customer_id = req.customer_id.clone().get_required_value("customer_id")?;

    let network_token_details = api::CardDetail {
        card_number: network_token_data.network_token_number.clone(),
        card_exp_month: network_token_data.network_token_exp_month.clone(),
        card_exp_year: network_token_data.network_token_exp_year.clone(),
        card_holder_name: network_token_data.card_holder_name.clone(),
        nick_name: network_token_data.nick_name.clone(),
        card_issuing_country: network_token_data.card_issuing_country.clone(),
        card_network: network_token_data.card_network.clone(),
        card_issuer: network_token_data.card_issuer.clone(),
        card_type: network_token_data.card_type.clone(),
    };

    logger::debug!(
        "Adding network token to locker for customer_id: {:?}",
        customer_id
    );

    let token_resp = Box::pin(add_card_to_locker(
        state,
        payment_method_create_request.clone(),
        &network_token_details,
        &customer_id,
        merchant_account,
        None,
    ))
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Add Network Token failed");
    let key_manager_state = &state.into();

    match token_resp {
        Ok(resp) => {
            logger::debug!("Network token added to locker");
            let (token_pm_resp, _duplication_check) = resp;
            let pm_token_details = token_pm_resp
                .card
                .as_ref()
                .map(|card| PaymentMethodsData::Card(CardDetailsPaymentMethod::from(card.clone())));
            let pm_network_token_data_encrypted = pm_token_details
                .async_map(|pm_card| create_encrypted_data(key_manager_state, key_store, pm_card))
                .await
                .transpose()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unable to encrypt payment method data")?;

            let pm_update = storage::PaymentMethodUpdate::NetworkTokenDataUpdate {
                network_token_requestor_reference_id: Some(network_token_requestor_ref_id),
                network_token_locker_id: Some(token_pm_resp.payment_method_id),
                network_token_payment_method_data: pm_network_token_data_encrypted.map(Into::into),
            };
            let db = &*state.store;
            let existing_pm = db
                .find_payment_method(
                    &state.into(),
                    key_store,
                    &pm_id,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(format!(
                    "Failed to fetch payment method for existing pm_id: {:?} in db",
                    pm_id
                ))?;

            db.update_payment_method(
                &state.into(),
                key_store,
                existing_pm,
                pm_update,
                merchant_account.storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(format!(
                "Failed to update payment method for existing pm_id: {:?} in db",
                pm_id
            ))?;

            logger::debug!("Network token added to locker and payment method updated");
            Ok(true)
        }
        Err(err) => {
            logger::debug!("Network token added to locker failed {:?}", err);
            Ok(false)
        }
    }
}

// need to discuss regarding the migration APIs for v2
#[cfg(all(
    feature = "v2",
    feature = "payment_methods_v2",
    feature = "customer_v2"
))]
pub async fn skip_locker_call_and_migrate_payment_method(
    _state: routes::SessionState,
    _req: &api::PaymentMethodMigrate,
    _merchant_id: id_type::MerchantId,
    _key_store: &domain::MerchantKeyStore,
    _merchant_account: &domain::MerchantAccount,
    _card: api_models::payment_methods::CardDetailFromLocker,
) -> errors::RouterResponse<api::PaymentMethodResponse> {
    todo!()
}

pub fn get_card_bin_and_last4_digits_for_masked_card(
    masked_card_number: &str,
) -> Result<(String, String), cards::CardNumberValidationErr> {
    let last4_digits = masked_card_number
        .chars()
        .rev()
        .take(4)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();

    let card_isin = masked_card_number.chars().take(6).collect::<String>();

    cards::validate::validate_card_number_chars(&card_isin)
        .and_then(|_| cards::validate::validate_card_number_chars(&last4_digits))?;

    Ok((card_isin, last4_digits))
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
#[instrument(skip_all)]
pub async fn get_client_secret_or_add_payment_method(
    state: &routes::SessionState,
    req: api::PaymentMethodCreate,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
) -> errors::RouterResponse<api::PaymentMethodResponse> {
    let merchant_id = merchant_account.get_id();
    let customer_id = req.customer_id.clone().get_required_value("customer_id")?;

    #[cfg(not(feature = "payouts"))]
    let condition = req.card.is_some();
    #[cfg(feature = "payouts")]
    let condition = req.card.is_some() || req.bank_transfer.is_some() || req.wallet.is_some();
    let key_manager_state = state.into();
    let payment_method_billing_address: Option<Encryptable<Secret<serde_json::Value>>> = req
        .billing
        .clone()
        .async_map(|billing| create_encrypted_data(&key_manager_state, key_store, billing))
        .await
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to encrypt Payment method billing address")?;

    let connector_mandate_details = req
        .connector_mandate_details
        .clone()
        .map(serde_json::to_value)
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    if condition {
        Box::pin(add_payment_method(state, req, merchant_account, key_store)).await
    } else {
        let payment_method_id = generate_id(consts::ID_LENGTH, "pm");

        let res = create_payment_method(
            state,
            &req,
            &customer_id,
            payment_method_id.as_str(),
            None,
            merchant_id,
            None,
            None,
            None,
            key_store,
            connector_mandate_details,
            Some(enums::PaymentMethodStatus::AwaitingData),
            None,
            merchant_account.storage_scheme,
            payment_method_billing_address.map(Into::into),
            None,
            None,
            None,
            None,
        )
        .await?;

        if res.status == enums::PaymentMethodStatus::AwaitingData {
            add_payment_method_status_update_task(
                &*state.store,
                &res,
                enums::PaymentMethodStatus::AwaitingData,
                enums::PaymentMethodStatus::Inactive,
                merchant_id,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "Failed to add payment method status update task in process tracker",
            )?;
        }

        Ok(services::api::ApplicationResponse::Json(
            api::PaymentMethodResponse::foreign_from((None, res)),
        ))
    }
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
#[instrument(skip_all)]
pub async fn get_client_secret_or_add_payment_method_for_migration(
    state: &routes::SessionState,
    req: api::PaymentMethodCreate,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    migration_status: &mut migration::RecordMigrationStatusBuilder,
) -> errors::RouterResponse<api::PaymentMethodResponse> {
    let merchant_id = merchant_account.get_id();
    let customer_id = req.customer_id.clone().get_required_value("customer_id")?;

    #[cfg(not(feature = "payouts"))]
    let condition = req.card.is_some();
    #[cfg(feature = "payouts")]
    let condition = req.card.is_some() || req.bank_transfer.is_some() || req.wallet.is_some();
    let key_manager_state = state.into();

    let payment_method_billing_address: Option<Encryptable<Secret<serde_json::Value>>> = req
        .billing
        .clone()
        .async_map(|billing| create_encrypted_data(&key_manager_state, key_store, billing))
        .await
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to encrypt Payment method billing address")?;

    let connector_mandate_details = req
        .connector_mandate_details
        .clone()
        .map(serde_json::to_value)
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    if condition {
        Box::pin(save_migration_payment_method(
            state,
            req,
            merchant_account,
            key_store,
            migration_status,
        ))
        .await
    } else {
        let payment_method_id = generate_id(consts::ID_LENGTH, "pm");

        let res = create_payment_method(
            state,
            &req,
            &customer_id,
            payment_method_id.as_str(),
            None,
            merchant_id,
            None,
            None,
            None,
            key_store,
            connector_mandate_details.clone(),
            Some(enums::PaymentMethodStatus::AwaitingData),
            None,
            merchant_account.storage_scheme,
            payment_method_billing_address.map(Into::into),
            None,
            None,
            None,
            None,
        )
        .await?;
        migration_status.connector_mandate_details_migrated(
            connector_mandate_details
                .clone()
                .and_then(|val| (val != json!({})).then_some(true))
                .or_else(|| {
                    req.connector_mandate_details
                        .clone()
                        .and_then(|val| (!val.0.is_empty()).then_some(false))
                }),
        );
        //card is not migrated in this case
        migration_status.card_migrated(false);

        if res.status == enums::PaymentMethodStatus::AwaitingData {
            add_payment_method_status_update_task(
                &*state.store,
                &res,
                enums::PaymentMethodStatus::AwaitingData,
                enums::PaymentMethodStatus::Inactive,
                merchant_id,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "Failed to add payment method status update task in process tracker",
            )?;
        }

        Ok(services::api::ApplicationResponse::Json(
            api::PaymentMethodResponse::foreign_from((None, res)),
        ))
    }
}

#[instrument(skip_all)]
pub fn authenticate_pm_client_secret_and_check_expiry(
    req_client_secret: &String,
    payment_method: &domain::PaymentMethod,
) -> errors::CustomResult<bool, errors::ApiErrorResponse> {
    let stored_client_secret = payment_method
        .client_secret
        .clone()
        .get_required_value("client_secret")
        .change_context(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "client_secret",
        })
        .attach_printable("client secret not found in db")?;

    if req_client_secret != &stored_client_secret {
        Err((errors::ApiErrorResponse::ClientSecretInvalid).into())
    } else {
        let current_timestamp = common_utils::date_time::now();
        let session_expiry = payment_method
            .created_at
            .saturating_add(time::Duration::seconds(consts::DEFAULT_SESSION_EXPIRY));

        let expired = current_timestamp > session_expiry;

        Ok(expired)
    }
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2"),
    not(feature = "customer_v2")
))]
#[instrument(skip_all)]
pub async fn add_payment_method_data(
    state: routes::SessionState,
    req: api::PaymentMethodCreate,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    pm_id: String,
) -> errors::RouterResponse<api::PaymentMethodResponse> {
    let db = &*state.store;

    let pmd = req
        .payment_method_data
        .clone()
        .get_required_value("payment_method_data")?;
    req.payment_method.get_required_value("payment_method")?;
    let client_secret = req
        .client_secret
        .clone()
        .get_required_value("client_secret")?;
    let payment_method = db
        .find_payment_method(
            &((&state).into()),
            &key_store,
            pm_id.as_str(),
            merchant_account.storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::PaymentMethodNotFound)
        .attach_printable("Unable to find payment method")?;

    if payment_method.status != enums::PaymentMethodStatus::AwaitingData {
        return Err((errors::ApiErrorResponse::ClientSecretExpired).into());
    }

    let customer_id = payment_method.customer_id.clone();

    let customer = db
        .find_customer_by_customer_id_merchant_id(
            &(&state).into(),
            &customer_id,
            merchant_account.get_id(),
            &key_store,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)?;

    let client_secret_expired =
        authenticate_pm_client_secret_and_check_expiry(&client_secret, &payment_method)?;

    if client_secret_expired {
        return Err((errors::ApiErrorResponse::ClientSecretExpired).into());
    };
    let key_manager_state = (&state).into();
    match pmd {
        api_models::payment_methods::PaymentMethodCreateData::Card(card) => {
            helpers::validate_card_expiry(&card.card_exp_month, &card.card_exp_year)?;
            let resp = Box::pin(add_card_to_locker(
                &state,
                req.clone(),
                &card,
                &customer_id,
                &merchant_account,
                None,
            ))
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError);

            match resp {
                Ok((mut pm_resp, duplication_check)) => {
                    if duplication_check.is_some() {
                        let pm_update = storage::PaymentMethodUpdate::StatusUpdate {
                            status: Some(enums::PaymentMethodStatus::Inactive),
                        };

                        db.update_payment_method(
                            &((&state).into()),
                            &key_store,
                            payment_method,
                            pm_update,
                            merchant_account.storage_scheme,
                        )
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to add payment method in db")?;

                        get_or_insert_payment_method(
                            &state,
                            req.clone(),
                            &mut pm_resp,
                            &merchant_account,
                            &customer_id,
                            &key_store,
                        )
                        .await?;

                        return Ok(services::ApplicationResponse::Json(pm_resp));
                    } else {
                        let locker_id = pm_resp.payment_method_id.clone();
                        pm_resp.payment_method_id.clone_from(&pm_id);
                        pm_resp.client_secret = Some(client_secret.clone());

                        let card_isin = card.card_number.get_card_isin();

                        let card_info = db
                            .get_card_info(card_isin.as_str())
                            .await
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Failed to get card info")?;

                        let updated_card = CardDetailsPaymentMethod {
                            issuer_country: card_info
                                .as_ref()
                                .and_then(|ci| ci.card_issuing_country.clone()),
                            last4_digits: Some(card.card_number.get_last4()),
                            expiry_month: Some(card.card_exp_month),
                            expiry_year: Some(card.card_exp_year),
                            nick_name: card.nick_name,
                            card_holder_name: card.card_holder_name,
                            card_network: card_info.as_ref().and_then(|ci| ci.card_network.clone()),
                            card_isin: Some(card_isin),
                            card_issuer: card_info.as_ref().and_then(|ci| ci.card_issuer.clone()),
                            card_type: card_info.as_ref().and_then(|ci| ci.card_type.clone()),
                            saved_to_locker: true,
                        };
                        let pm_data_encrypted: Encryptable<Secret<serde_json::Value>> =
                            create_encrypted_data(
                                &key_manager_state,
                                &key_store,
                                PaymentMethodsData::Card(updated_card),
                            )
                            .await
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Unable to encrypt payment method data")?;

                        let pm_update = storage::PaymentMethodUpdate::AdditionalDataUpdate {
                            payment_method_data: Some(pm_data_encrypted.into()),
                            status: Some(enums::PaymentMethodStatus::Active),
                            locker_id: Some(locker_id),
                            network_token_requestor_reference_id: None,
                            payment_method: req.payment_method,
                            payment_method_issuer: req.payment_method_issuer,
                            payment_method_type: req.payment_method_type,
                            network_token_locker_id: None,
                            network_token_payment_method_data: None,
                        };

                        db.update_payment_method(
                            &((&state).into()),
                            &key_store,
                            payment_method,
                            pm_update,
                            merchant_account.storage_scheme,
                        )
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to add payment method in db")?;

                        if customer.default_payment_method_id.is_none() {
                            let _ = set_default_payment_method(
                                &state,
                                merchant_account.get_id(),
                                key_store.clone(),
                                &customer_id,
                                pm_id,
                                merchant_account.storage_scheme,
                            )
                            .await
                            .map_err(|error| {
                                logger::error!(
                                    ?error,
                                    "Failed to set the payment method as default"
                                )
                            });
                        }

                        return Ok(services::ApplicationResponse::Json(pm_resp));
                    }
                }
                Err(e) => {
                    let pm_update = storage::PaymentMethodUpdate::StatusUpdate {
                        status: Some(enums::PaymentMethodStatus::Inactive),
                    };

                    db.update_payment_method(
                        &((&state).into()),
                        &key_store,
                        payment_method,
                        pm_update,
                        merchant_account.storage_scheme,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to update payment method in db")?;

                    return Err(e.attach_printable("Failed to add card to locker"));
                }
            }
        }
    }
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
#[instrument(skip_all)]
pub async fn add_payment_method(
    state: &routes::SessionState,
    req: api::PaymentMethodCreate,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
) -> errors::RouterResponse<api::PaymentMethodResponse> {
    req.validate()?;
    let db = &*state.store;
    let merchant_id = merchant_account.get_id();
    let customer_id = req.customer_id.clone().get_required_value("customer_id")?;
    let payment_method = req.payment_method.get_required_value("payment_method")?;
    let key_manager_state = state.into();
    let payment_method_billing_address: Option<Encryptable<Secret<serde_json::Value>>> = req
        .billing
        .clone()
        .async_map(|billing| create_encrypted_data(&key_manager_state, key_store, billing))
        .await
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to encrypt Payment method billing address")?;

    let connector_mandate_details = req
        .connector_mandate_details
        .clone()
        .map(serde_json::to_value)
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let response = match payment_method {
        #[cfg(feature = "payouts")]
        api_enums::PaymentMethod::BankTransfer => match req.bank_transfer.clone() {
            Some(bank) => add_bank_to_locker(
                state,
                req.clone(),
                merchant_account,
                key_store,
                &bank,
                &customer_id,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Add PaymentMethod Failed"),
            _ => Ok(store_default_payment_method(
                &req,
                &customer_id,
                merchant_id,
            )),
        },
        api_enums::PaymentMethod::Card => match req.card.clone() {
            Some(card) => {
                let mut card_details = card;
                card_details = helpers::populate_bin_details_for_payment_method_create(
                    card_details.clone(),
                    db,
                )
                .await;
                helpers::validate_card_expiry(
                    &card_details.card_exp_month,
                    &card_details.card_exp_year,
                )?;
                Box::pin(add_card_to_locker(
                    state,
                    req.clone(),
                    &card_details,
                    &customer_id,
                    merchant_account,
                    None,
                ))
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Add Card Failed")
            }
            _ => Ok(store_default_payment_method(
                &req,
                &customer_id,
                merchant_id,
            )),
        },
        _ => Ok(store_default_payment_method(
            &req,
            &customer_id,
            merchant_id,
        )),
    };

    let (mut resp, duplication_check) = response?;

    match duplication_check {
        Some(duplication_check) => match duplication_check {
            payment_methods::DataDuplicationCheck::Duplicated => {
                let existing_pm = get_or_insert_payment_method(
                    state,
                    req.clone(),
                    &mut resp,
                    merchant_account,
                    &customer_id,
                    key_store,
                )
                .await?;

                resp.client_secret = existing_pm.client_secret;
            }
            payment_methods::DataDuplicationCheck::MetaDataChanged => {
                if let Some(card) = req.card.clone() {
                    let existing_pm = get_or_insert_payment_method(
                        state,
                        req.clone(),
                        &mut resp,
                        merchant_account,
                        &customer_id,
                        key_store,
                    )
                    .await?;

                    let client_secret = existing_pm.client_secret.clone();

                    delete_card_from_locker(
                        state,
                        &customer_id,
                        merchant_id,
                        existing_pm
                            .locker_id
                            .as_ref()
                            .unwrap_or(&existing_pm.payment_method_id),
                    )
                    .await?;

                    let add_card_resp = add_card_hs(
                        state,
                        req.clone(),
                        &card,
                        &customer_id,
                        merchant_account,
                        api::enums::LockerChoice::HyperswitchCardVault,
                        Some(
                            existing_pm
                                .locker_id
                                .as_ref()
                                .unwrap_or(&existing_pm.payment_method_id),
                        ),
                    )
                    .await;

                    if let Err(err) = add_card_resp {
                        logger::error!(vault_err=?err);
                        db.delete_payment_method_by_merchant_id_payment_method_id(
                            &(state.into()),
                            key_store,
                            merchant_id,
                            &resp.payment_method_id,
                        )
                        .await
                        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

                        Err(report!(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Failed while updating card metadata changes"))?
                    };

                    let existing_pm_data =
                        get_card_details_without_locker_fallback(&existing_pm, state).await?;

                    let updated_card = Some(api::CardDetailFromLocker {
                        scheme: existing_pm.scheme.clone(),
                        last4_digits: Some(card.card_number.get_last4()),
                        issuer_country: card
                            .card_issuing_country
                            .or(existing_pm_data.issuer_country),
                        card_isin: Some(card.card_number.get_card_isin()),
                        card_number: Some(card.card_number),
                        expiry_month: Some(card.card_exp_month),
                        expiry_year: Some(card.card_exp_year),
                        card_token: None,
                        card_fingerprint: None,
                        card_holder_name: card
                            .card_holder_name
                            .or(existing_pm_data.card_holder_name),
                        nick_name: card.nick_name.or(existing_pm_data.nick_name),
                        card_network: card.card_network.or(existing_pm_data.card_network),
                        card_issuer: card.card_issuer.or(existing_pm_data.card_issuer),
                        card_type: card.card_type.or(existing_pm_data.card_type),
                        saved_to_locker: true,
                    });

                    let updated_pmd = updated_card.as_ref().map(|card| {
                        PaymentMethodsData::Card(CardDetailsPaymentMethod::from(card.clone()))
                    });
                    let pm_data_encrypted: Option<Encryptable<Secret<serde_json::Value>>> =
                        updated_pmd
                            .async_map(|updated_pmd| {
                                create_encrypted_data(&key_manager_state, key_store, updated_pmd)
                            })
                            .await
                            .transpose()
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Unable to encrypt payment method data")?;

                    let pm_update = storage::PaymentMethodUpdate::PaymentMethodDataUpdate {
                        payment_method_data: pm_data_encrypted.map(Into::into),
                    };

                    db.update_payment_method(
                        &(state.into()),
                        key_store,
                        existing_pm,
                        pm_update,
                        merchant_account.storage_scheme,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to add payment method in db")?;

                    resp.client_secret = client_secret;
                }
            }
        },
        None => {
            let pm_metadata = resp.metadata.as_ref().map(|data| data.peek());

            let locker_id = if resp.payment_method == Some(api_enums::PaymentMethod::Card)
                || resp.payment_method == Some(api_enums::PaymentMethod::BankTransfer)
            {
                Some(resp.payment_method_id)
            } else {
                None
            };
            resp.payment_method_id = generate_id(consts::ID_LENGTH, "pm");
            let pm = insert_payment_method(
                state,
                &resp,
                &req,
                key_store,
                merchant_id,
                &customer_id,
                pm_metadata.cloned(),
                None,
                locker_id,
                connector_mandate_details,
                req.network_transaction_id.clone(),
                merchant_account.storage_scheme,
                payment_method_billing_address.map(Into::into),
                None,
                None,
                None,
            )
            .await?;

            resp.client_secret = pm.client_secret;
        }
    }

    Ok(services::ApplicationResponse::Json(resp))
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
#[instrument(skip_all)]
pub async fn save_migration_payment_method(
    state: &routes::SessionState,
    req: api::PaymentMethodCreate,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    migration_status: &mut migration::RecordMigrationStatusBuilder,
) -> errors::RouterResponse<api::PaymentMethodResponse> {
    req.validate()?;
    let db = &*state.store;
    let merchant_id = merchant_account.get_id();
    let customer_id = req.customer_id.clone().get_required_value("customer_id")?;
    let payment_method = req.payment_method.get_required_value("payment_method")?;
    let key_manager_state = state.into();
    let payment_method_billing_address: Option<Encryptable<Secret<serde_json::Value>>> = req
        .billing
        .clone()
        .async_map(|billing| create_encrypted_data(&key_manager_state, key_store, billing))
        .await
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to encrypt Payment method billing address")?;

    let connector_mandate_details = req
        .connector_mandate_details
        .clone()
        .map(serde_json::to_value)
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let network_transaction_id = req.network_transaction_id.clone();

    let response = match payment_method {
        #[cfg(feature = "payouts")]
        api_enums::PaymentMethod::BankTransfer => match req.bank_transfer.clone() {
            Some(bank) => add_bank_to_locker(
                state,
                req.clone(),
                merchant_account,
                key_store,
                &bank,
                &customer_id,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Add PaymentMethod Failed"),
            _ => Ok(store_default_payment_method(
                &req,
                &customer_id,
                merchant_id,
            )),
        },
        api_enums::PaymentMethod::Card => match req.card.clone() {
            Some(card) => {
                let mut card_details = card;
                card_details = helpers::populate_bin_details_for_payment_method_create(
                    card_details.clone(),
                    db,
                )
                .await;
                migration::validate_card_expiry(
                    &card_details.card_exp_month,
                    &card_details.card_exp_year,
                )?;
                Box::pin(add_card_to_locker(
                    state,
                    req.clone(),
                    &card_details,
                    &customer_id,
                    merchant_account,
                    None,
                ))
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Add Card Failed")
            }
            _ => Ok(store_default_payment_method(
                &req,
                &customer_id,
                merchant_id,
            )),
        },
        _ => Ok(store_default_payment_method(
            &req,
            &customer_id,
            merchant_id,
        )),
    };

    let (mut resp, duplication_check) = response?;

    migration_status.card_migrated(true);
    match duplication_check {
        Some(duplication_check) => match duplication_check {
            payment_methods::DataDuplicationCheck::Duplicated => {
                let existing_pm = get_or_insert_payment_method(
                    state,
                    req.clone(),
                    &mut resp,
                    merchant_account,
                    &customer_id,
                    key_store,
                )
                .await?;

                resp.client_secret = existing_pm.client_secret;
            }
            payment_methods::DataDuplicationCheck::MetaDataChanged => {
                if let Some(card) = req.card.clone() {
                    let existing_pm = get_or_insert_payment_method(
                        state,
                        req.clone(),
                        &mut resp,
                        merchant_account,
                        &customer_id,
                        key_store,
                    )
                    .await?;

                    let client_secret = existing_pm.client_secret.clone();

                    delete_card_from_locker(
                        state,
                        &customer_id,
                        merchant_id,
                        existing_pm
                            .locker_id
                            .as_ref()
                            .unwrap_or(&existing_pm.payment_method_id),
                    )
                    .await?;

                    let add_card_resp = add_card_hs(
                        state,
                        req.clone(),
                        &card,
                        &customer_id,
                        merchant_account,
                        api::enums::LockerChoice::HyperswitchCardVault,
                        Some(
                            existing_pm
                                .locker_id
                                .as_ref()
                                .unwrap_or(&existing_pm.payment_method_id),
                        ),
                    )
                    .await;

                    if let Err(err) = add_card_resp {
                        logger::error!(vault_err=?err);
                        db.delete_payment_method_by_merchant_id_payment_method_id(
                            &(state.into()),
                            key_store,
                            merchant_id,
                            &resp.payment_method_id,
                        )
                        .await
                        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

                        Err(report!(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Failed while updating card metadata changes"))?
                    };

                    let existing_pm_data =
                        get_card_details_without_locker_fallback(&existing_pm, state).await?;

                    let updated_card = Some(api::CardDetailFromLocker {
                        scheme: existing_pm.scheme.clone(),
                        last4_digits: Some(card.card_number.get_last4()),
                        issuer_country: card
                            .card_issuing_country
                            .or(existing_pm_data.issuer_country),
                        card_isin: Some(card.card_number.get_card_isin()),
                        card_number: Some(card.card_number),
                        expiry_month: Some(card.card_exp_month),
                        expiry_year: Some(card.card_exp_year),
                        card_token: None,
                        card_fingerprint: None,
                        card_holder_name: card
                            .card_holder_name
                            .or(existing_pm_data.card_holder_name),
                        nick_name: card.nick_name.or(existing_pm_data.nick_name),
                        card_network: card.card_network.or(existing_pm_data.card_network),
                        card_issuer: card.card_issuer.or(existing_pm_data.card_issuer),
                        card_type: card.card_type.or(existing_pm_data.card_type),
                        saved_to_locker: true,
                    });

                    let updated_pmd = updated_card.as_ref().map(|card| {
                        PaymentMethodsData::Card(CardDetailsPaymentMethod::from(card.clone()))
                    });
                    let pm_data_encrypted: Option<Encryptable<Secret<serde_json::Value>>> =
                        updated_pmd
                            .async_map(|updated_pmd| {
                                create_encrypted_data(&key_manager_state, key_store, updated_pmd)
                            })
                            .await
                            .transpose()
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Unable to encrypt payment method data")?;

                    let pm_update = storage::PaymentMethodUpdate::PaymentMethodDataUpdate {
                        payment_method_data: pm_data_encrypted.map(Into::into),
                    };

                    db.update_payment_method(
                        &(state.into()),
                        key_store,
                        existing_pm,
                        pm_update,
                        merchant_account.storage_scheme,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to add payment method in db")?;

                    resp.client_secret = client_secret;
                }
            }
        },
        None => {
            let pm_metadata = resp.metadata.as_ref().map(|data| data.peek());

            let locker_id = if resp.payment_method == Some(api_enums::PaymentMethod::Card)
                || resp.payment_method == Some(api_enums::PaymentMethod::BankTransfer)
            {
                Some(resp.payment_method_id)
            } else {
                None
            };
            resp.payment_method_id = generate_id(consts::ID_LENGTH, "pm");
            let pm = insert_payment_method(
                state,
                &resp,
                &req,
                key_store,
                merchant_id,
                &customer_id,
                pm_metadata.cloned(),
                None,
                locker_id,
                connector_mandate_details.clone(),
                network_transaction_id.clone(),
                merchant_account.storage_scheme,
                payment_method_billing_address.map(Into::into),
                None,
                None,
                None,
            )
            .await?;

            resp.client_secret = pm.client_secret;
        }
    }

    migration_status.card_migrated(true);
    migration_status.network_transaction_id_migrated(
        network_transaction_id.and_then(|val| (!val.is_empty_after_trim()).then_some(true)),
    );

    migration_status.connector_mandate_details_migrated(
        connector_mandate_details
            .and_then(|val| if val == json!({}) { None } else { Some(true) })
            .or_else(|| {
                req.connector_mandate_details
                    .and_then(|val| (!val.0.is_empty()).then_some(false))
            }),
    );

    Ok(services::ApplicationResponse::Json(resp))
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
#[allow(clippy::too_many_arguments)]
pub async fn insert_payment_method(
    state: &routes::SessionState,
    resp: &api::PaymentMethodResponse,
    req: &api::PaymentMethodCreate,
    key_store: &domain::MerchantKeyStore,
    merchant_id: &id_type::MerchantId,
    customer_id: &id_type::CustomerId,
    pm_metadata: Option<serde_json::Value>,
    customer_acceptance: Option<serde_json::Value>,
    locker_id: Option<String>,
    connector_mandate_details: Option<serde_json::Value>,
    network_transaction_id: Option<String>,
    storage_scheme: MerchantStorageScheme,
    payment_method_billing_address: crypto::OptionalEncryptableValue,
    network_token_requestor_reference_id: Option<String>,
    network_token_locker_id: Option<String>,
    network_token_payment_method_data: crypto::OptionalEncryptableValue,
) -> errors::RouterResult<domain::PaymentMethod> {
    let pm_card_details = resp
        .card
        .clone()
        .map(|card| PaymentMethodsData::Card(CardDetailsPaymentMethod::from(card.clone())));
    let key_manager_state = state.into();
    let pm_data_encrypted: crypto::OptionalEncryptableValue = pm_card_details
        .clone()
        .async_map(|pm_card| create_encrypted_data(&key_manager_state, key_store, pm_card))
        .await
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to encrypt payment method data")?;

    create_payment_method(
        state,
        req,
        customer_id,
        &resp.payment_method_id,
        locker_id,
        merchant_id,
        pm_metadata,
        customer_acceptance,
        pm_data_encrypted,
        key_store,
        connector_mandate_details,
        None,
        network_transaction_id,
        storage_scheme,
        payment_method_billing_address,
        resp.card.clone().and_then(|card| {
            card.card_network
                .map(|card_network| card_network.to_string())
        }),
        network_token_requestor_reference_id,
        network_token_locker_id,
        network_token_payment_method_data,
    )
    .await
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[allow(clippy::too_many_arguments)]
pub async fn insert_payment_method(
    state: &routes::SessionState,
    resp: &api::PaymentMethodResponse,
    req: &api::PaymentMethodCreate,
    key_store: &domain::MerchantKeyStore,
    merchant_id: &id_type::MerchantId,
    customer_id: &id_type::CustomerId,
    pm_metadata: Option<serde_json::Value>,
    customer_acceptance: Option<serde_json::Value>,
    locker_id: Option<String>,
    connector_mandate_details: Option<serde_json::Value>,
    network_transaction_id: Option<String>,
    storage_scheme: MerchantStorageScheme,
    payment_method_billing_address: Option<Encryption>,
) -> errors::RouterResult<domain::PaymentMethod> {
    todo!()
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
#[instrument(skip_all)]
pub async fn update_customer_payment_method(
    state: routes::SessionState,
    merchant_account: domain::MerchantAccount,
    req: api::PaymentMethodUpdate,
    payment_method_id: &str,
    key_store: domain::MerchantKeyStore,
) -> errors::RouterResponse<api::PaymentMethodResponse> {
    // Currently update is supported only for cards
    if let Some(card_update) = req.card.clone() {
        let db = state.store.as_ref();

        let pm = db
            .find_payment_method(
                &((&state).into()),
                &key_store,
                payment_method_id,
                merchant_account.storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

        if let Some(cs) = &req.client_secret {
            let is_client_secret_expired = authenticate_pm_client_secret_and_check_expiry(cs, &pm)?;

            if is_client_secret_expired {
                return Err((errors::ApiErrorResponse::ClientSecretExpired).into());
            };
        };

        if pm.status == enums::PaymentMethodStatus::AwaitingData {
            return Err(report!(errors::ApiErrorResponse::NotSupported {
                message: "Payment method is awaiting data so it cannot be updated".into()
            }));
        }

        if pm.payment_method_data.is_none() {
            return Err(report!(errors::ApiErrorResponse::GenericNotFoundError {
                message: "payment_method_data not found".to_string()
            }));
        }

        // Fetch the existing payment method data from db
        let existing_card_data =
            pm.payment_method_data
                .clone()
                .map(|x| x.into_inner().expose())
                .map(
                    |value| -> Result<
                        PaymentMethodsData,
                        error_stack::Report<errors::ApiErrorResponse>,
                    > {
                        value
                            .parse_value::<PaymentMethodsData>("PaymentMethodsData")
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Failed to deserialize payment methods data")
                    },
                )
                .transpose()?
                .and_then(|pmd| match pmd {
                    PaymentMethodsData::Card(crd) => Some(api::CardDetailFromLocker::from(crd)),
                    _ => None,
                })
                .ok_or(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to obtain decrypted card object from db")?;

        let is_card_updation_required =
            validate_payment_method_update(card_update.clone(), existing_card_data.clone());

        let response = if is_card_updation_required {
            // Fetch the existing card data from locker for getting card number
            let card_data_from_locker = get_card_from_locker(
                &state,
                &pm.customer_id,
                &pm.merchant_id,
                pm.locker_id.as_ref().unwrap_or(&pm.payment_method_id),
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error getting card from locker")?;

            if card_update.card_exp_month.is_some() || card_update.card_exp_year.is_some() {
                helpers::validate_card_expiry(
                    card_update
                        .card_exp_month
                        .as_ref()
                        .unwrap_or(&card_data_from_locker.card_exp_month),
                    card_update
                        .card_exp_year
                        .as_ref()
                        .unwrap_or(&card_data_from_locker.card_exp_year),
                )?;
            }

            let updated_card_details = card_update.apply(card_data_from_locker.clone());

            // Construct new payment method object from request
            let new_pm = api::PaymentMethodCreate {
                payment_method: pm.get_payment_method_type(),
                payment_method_type: pm.get_payment_method_subtype(),
                payment_method_issuer: pm.payment_method_issuer.clone(),
                payment_method_issuer_code: pm.payment_method_issuer_code,
                #[cfg(feature = "payouts")]
                bank_transfer: None,
                card: Some(updated_card_details.clone()),
                #[cfg(feature = "payouts")]
                wallet: None,
                metadata: None,
                customer_id: Some(pm.customer_id.clone()),
                client_secret: pm.client_secret.clone(),
                payment_method_data: None,
                card_network: None,
                billing: None,
                connector_mandate_details: None,
                network_transaction_id: None,
            };
            new_pm.validate()?;

            // Delete old payment method from locker
            delete_card_from_locker(
                &state,
                &pm.customer_id,
                &pm.merchant_id,
                pm.locker_id.as_ref().unwrap_or(&pm.payment_method_id),
            )
            .await?;

            // Add the updated payment method data to locker
            let (mut add_card_resp, _) = Box::pin(add_card_to_locker(
                &state,
                new_pm.clone(),
                &updated_card_details,
                &pm.customer_id,
                &merchant_account,
                Some(pm.locker_id.as_ref().unwrap_or(&pm.payment_method_id)),
            ))
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to add updated payment method to locker")?;

            // Construct new updated card object. Consider a field if passed in request or else populate it with the existing value from existing_card_data
            let updated_card = Some(api::CardDetailFromLocker {
                scheme: existing_card_data.scheme,
                last4_digits: Some(card_data_from_locker.card_number.get_last4()),
                issuer_country: existing_card_data.issuer_country,
                card_number: existing_card_data.card_number,
                expiry_month: card_update
                    .card_exp_month
                    .or(existing_card_data.expiry_month),
                expiry_year: card_update.card_exp_year.or(existing_card_data.expiry_year),
                card_token: existing_card_data.card_token,
                card_fingerprint: existing_card_data.card_fingerprint,
                card_holder_name: card_update
                    .card_holder_name
                    .or(existing_card_data.card_holder_name),
                nick_name: card_update.nick_name.or(existing_card_data.nick_name),
                card_network: existing_card_data.card_network,
                card_isin: existing_card_data.card_isin,
                card_issuer: existing_card_data.card_issuer,
                card_type: existing_card_data.card_type,
                saved_to_locker: true,
            });

            let updated_pmd = updated_card
                .as_ref()
                .map(|card| PaymentMethodsData::Card(CardDetailsPaymentMethod::from(card.clone())));
            let key_manager_state = (&state).into();
            let pm_data_encrypted: Option<Encryptable<Secret<serde_json::Value>>> = updated_pmd
                .async_map(|updated_pmd| {
                    create_encrypted_data(&key_manager_state, &key_store, updated_pmd)
                })
                .await
                .transpose()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unable to encrypt payment method data")?;

            let pm_update = storage::PaymentMethodUpdate::PaymentMethodDataUpdate {
                payment_method_data: pm_data_encrypted.map(Into::into),
            };

            add_card_resp
                .payment_method_id
                .clone_from(&pm.payment_method_id);

            db.update_payment_method(
                &((&state).into()),
                &key_store,
                pm,
                pm_update,
                merchant_account.storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to update payment method in db")?;

            add_card_resp
        } else {
            // Return existing payment method data as response without any changes
            api::PaymentMethodResponse {
                merchant_id: pm.merchant_id.to_owned(),
                customer_id: Some(pm.customer_id.clone()),
                payment_method_id: pm.payment_method_id.clone(),
                payment_method: pm.get_payment_method_type(),
                payment_method_type: pm.get_payment_method_subtype(),
                #[cfg(feature = "payouts")]
                bank_transfer: None,
                card: Some(existing_card_data),
                metadata: pm.metadata,
                created: Some(pm.created_at),
                recurring_enabled: false,
                installment_payment_enabled: false,
                payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]),
                last_used_at: Some(common_utils::date_time::now()),
                client_secret: pm.client_secret.clone(),
            }
        };

        Ok(services::ApplicationResponse::Json(response))
    } else {
        Err(report!(errors::ApiErrorResponse::NotSupported {
            message: "Payment method update for the given payment method is not supported".into()
        }))
    }
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
pub fn validate_payment_method_update(
    card_updation_obj: CardDetailUpdate,
    existing_card_data: api::CardDetailFromLocker,
) -> bool {
    // Return true If any one of the below condition returns true,
    // If a field is not passed in the update request, return false.
    // If the field is present, it depends on the existing field data:
    // - If existing field data is not present, or if it is present and doesn't match
    //   the update request data, then return true.
    // - Or else return false
    card_updation_obj
        .card_exp_month
        .map(|exp_month| exp_month.expose())
        .is_some_and(|new_exp_month| {
            existing_card_data
                .expiry_month
                .map(|exp_month| exp_month.expose())
                != Some(new_exp_month)
        })
        || card_updation_obj
            .card_exp_year
            .map(|exp_year| exp_year.expose())
            .is_some_and(|new_exp_year| {
                existing_card_data
                    .expiry_year
                    .map(|exp_year| exp_year.expose())
                    != Some(new_exp_year)
            })
        || card_updation_obj
            .card_holder_name
            .map(|name| name.expose())
            .is_some_and(|new_card_holder_name| {
                existing_card_data
                    .card_holder_name
                    .map(|name| name.expose())
                    != Some(new_card_holder_name)
            })
        || card_updation_obj
            .nick_name
            .map(|nick_name| nick_name.expose())
            .is_some_and(|new_nick_name| {
                existing_card_data
                    .nick_name
                    .map(|nick_name| nick_name.expose())
                    != Some(new_nick_name)
            })
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
pub fn validate_payment_method_update(
    _card_updation_obj: CardDetailUpdate,
    _existing_card_data: api::CardDetailFromLocker,
) -> bool {
    todo!()
}

// Wrapper function to switch lockers

#[cfg(feature = "payouts")]
pub async fn add_bank_to_locker(
    state: &routes::SessionState,
    req: api::PaymentMethodCreate,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    bank: &api::BankPayout,
    customer_id: &id_type::CustomerId,
) -> errors::CustomResult<
    (
        api::PaymentMethodResponse,
        Option<payment_methods::DataDuplicationCheck>,
    ),
    errors::VaultError,
> {
    let key = key_store.key.get_inner().peek();
    let payout_method_data = api::PayoutMethodData::Bank(bank.clone());
    let key_manager_state: KeyManagerState = state.into();
    let enc_data = async {
        serde_json::to_value(payout_method_data.to_owned())
            .map_err(|err| {
                logger::error!("Error while encoding payout method data: {err:?}");
                errors::VaultError::SavePaymentMethodFailed
            })
            .change_context(errors::VaultError::SavePaymentMethodFailed)
            .attach_printable("Unable to encode payout method data")
            .ok()
            .map(|v| {
                let secret: Secret<String> = Secret::new(v.to_string());
                secret
            })
            .async_lift(|inner| async {
                domain::types::crypto_operation(
                    &key_manager_state,
                    type_name!(payment_method::PaymentMethod),
                    domain::types::CryptoOperation::EncryptOptional(inner),
                    Identifier::Merchant(key_store.merchant_id.clone()),
                    key,
                )
                .await
                .and_then(|val| val.try_into_optionaloperation())
            })
            .await
    }
    .await
    .change_context(errors::VaultError::SavePaymentMethodFailed)
    .attach_printable("Failed to encrypt payout method data")?
    .map(Encryption::from)
    .map(|e| e.into_inner())
    .map_or(Err(errors::VaultError::SavePaymentMethodFailed), |e| {
        Ok(hex::encode(e.peek()))
    })?;

    let payload =
        payment_methods::StoreLockerReq::LockerGeneric(payment_methods::StoreGenericReq {
            merchant_id: merchant_account.get_id().to_owned(),
            merchant_customer_id: customer_id.to_owned(),
            enc_data,
            ttl: state.conf.locker.ttl_for_storage_in_secs,
        });
    let store_resp = add_card_to_hs_locker(
        state,
        &payload,
        customer_id,
        api_enums::LockerChoice::HyperswitchCardVault,
    )
    .await?;
    let payment_method_resp = payment_methods::mk_add_bank_response_hs(
        bank.clone(),
        store_resp.card_reference,
        req,
        merchant_account.get_id(),
    );
    Ok((payment_method_resp, store_resp.duplication_check))
}

/// The response will be the tuple of PaymentMethodResponse and the duplication check of payment_method
pub async fn add_card_to_locker(
    state: &routes::SessionState,
    req: api::PaymentMethodCreate,
    card: &api::CardDetail,
    customer_id: &id_type::CustomerId,
    merchant_account: &domain::MerchantAccount,
    card_reference: Option<&str>,
) -> errors::CustomResult<
    (
        api::PaymentMethodResponse,
        Option<payment_methods::DataDuplicationCheck>,
    ),
    errors::VaultError,
> {
    metrics::STORED_TO_LOCKER.add(1, &[]);
    let add_card_to_hs_resp = Box::pin(common_utils::metrics::utils::record_operation_time(
        async {
            add_card_hs(
                state,
                req.clone(),
                card,
                customer_id,
                merchant_account,
                api_enums::LockerChoice::HyperswitchCardVault,
                card_reference,
            )
            .await
            .inspect_err(|_| {
                metrics::CARD_LOCKER_FAILURES.add(
                    1,
                    router_env::metric_attributes!(("locker", "rust"), ("operation", "add")),
                );
            })
        },
        &metrics::CARD_ADD_TIME,
        router_env::metric_attributes!(("locker", "rust")),
    ))
    .await?;

    logger::debug!("card added to hyperswitch-card-vault");
    Ok(add_card_to_hs_resp)
}

pub async fn get_card_from_locker(
    state: &routes::SessionState,
    customer_id: &id_type::CustomerId,
    merchant_id: &id_type::MerchantId,
    card_reference: &str,
) -> errors::RouterResult<Card> {
    metrics::GET_FROM_LOCKER.add(1, &[]);

    let get_card_from_rs_locker_resp = common_utils::metrics::utils::record_operation_time(
        async {
            get_card_from_hs_locker(
                state,
                customer_id,
                merchant_id,
                card_reference,
                api_enums::LockerChoice::HyperswitchCardVault,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed while getting card from hyperswitch card vault")
            .inspect_err(|_| {
                metrics::CARD_LOCKER_FAILURES.add(
                    1,
                    router_env::metric_attributes!(("locker", "rust"), ("operation", "get")),
                );
            })
        },
        &metrics::CARD_GET_TIME,
        router_env::metric_attributes!(("locker", "rust")),
    )
    .await?;

    logger::debug!("card retrieved from rust locker");
    Ok(get_card_from_rs_locker_resp)
}

pub async fn delete_card_from_locker(
    state: &routes::SessionState,
    customer_id: &id_type::CustomerId,
    merchant_id: &id_type::MerchantId,
    card_reference: &str,
) -> errors::RouterResult<payment_methods::DeleteCardResp> {
    metrics::DELETE_FROM_LOCKER.add(1, &[]);

    common_utils::metrics::utils::record_operation_time(
        async move {
            delete_card_from_hs_locker(state, customer_id, merchant_id, card_reference)
                .await
                .inspect_err(|_| {
                    metrics::CARD_LOCKER_FAILURES.add(
                        1,
                        router_env::metric_attributes!(("locker", "rust"), ("operation", "delete")),
                    );
                })
        },
        &metrics::CARD_DELETE_TIME,
        &[],
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed while deleting card from locker")
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
pub async fn delete_card_by_locker_id(
    state: &routes::SessionState,
    id: &id_type::GlobalCustomerId,
    merchant_id: &id_type::MerchantId,
) -> errors::RouterResult<payment_methods::DeleteCardResp> {
    todo!()
}

#[instrument(skip_all)]
pub async fn add_card_hs(
    state: &routes::SessionState,
    req: api::PaymentMethodCreate,
    card: &api::CardDetail,
    customer_id: &id_type::CustomerId,
    merchant_account: &domain::MerchantAccount,
    locker_choice: api_enums::LockerChoice,
    card_reference: Option<&str>,
) -> errors::CustomResult<
    (
        api::PaymentMethodResponse,
        Option<payment_methods::DataDuplicationCheck>,
    ),
    errors::VaultError,
> {
    let payload = payment_methods::StoreLockerReq::LockerCard(payment_methods::StoreCardReq {
        merchant_id: merchant_account.get_id().to_owned(),
        merchant_customer_id: customer_id.to_owned(),
        requestor_card_reference: card_reference.map(str::to_string),
        card: Card {
            card_number: card.card_number.to_owned(),
            name_on_card: card.card_holder_name.to_owned(),
            card_exp_month: card.card_exp_month.to_owned(),
            card_exp_year: card.card_exp_year.to_owned(),
            card_brand: card.card_network.as_ref().map(ToString::to_string),
            card_isin: None,
            nick_name: card.nick_name.as_ref().map(Secret::peek).cloned(),
        },
        ttl: state.conf.locker.ttl_for_storage_in_secs,
    });

    let store_card_payload =
        add_card_to_hs_locker(state, &payload, customer_id, locker_choice).await?;

    let payment_method_resp = payment_methods::mk_add_card_response_hs(
        card.clone(),
        store_card_payload.card_reference,
        req,
        merchant_account.get_id(),
    );
    Ok((payment_method_resp, store_card_payload.duplication_check))
}

#[instrument(skip_all)]
pub async fn decode_and_decrypt_locker_data(
    state: &routes::SessionState,
    key_store: &domain::MerchantKeyStore,
    enc_card_data: String,
) -> errors::CustomResult<Secret<String>, errors::VaultError> {
    // Fetch key
    let key = key_store.key.get_inner().peek();
    // Decode
    let decoded_bytes = hex::decode(&enc_card_data)
        .change_context(errors::VaultError::ResponseDeserializationFailed)
        .attach_printable("Failed to decode hex string into bytes")?;
    // Decrypt
    domain::types::crypto_operation(
        &state.into(),
        type_name!(payment_method::PaymentMethod),
        domain::types::CryptoOperation::DecryptOptional(Some(Encryption::new(
            decoded_bytes.into(),
        ))),
        Identifier::Merchant(key_store.merchant_id.clone()),
        key,
    )
    .await
    .and_then(|val| val.try_into_optionaloperation())
    .change_context(errors::VaultError::FetchPaymentMethodFailed)?
    .map_or(
        Err(report!(errors::VaultError::FetchPaymentMethodFailed)),
        |d| Ok(d.into_inner()),
    )
}

#[instrument(skip_all)]
pub async fn get_payment_method_from_hs_locker<'a>(
    state: &'a routes::SessionState,
    key_store: &domain::MerchantKeyStore,
    customer_id: &id_type::CustomerId,
    merchant_id: &id_type::MerchantId,
    payment_method_reference: &'a str,
    locker_choice: Option<api_enums::LockerChoice>,
) -> errors::CustomResult<Secret<String>, errors::VaultError> {
    let locker = &state.conf.locker;
    let jwekey = state.conf.jwekey.get_inner();

    let payment_method_data = if !locker.mock_locker {
        let request = payment_methods::mk_get_card_request_hs(
            jwekey,
            locker,
            customer_id,
            merchant_id,
            payment_method_reference,
            locker_choice,
            state.tenant.tenant_id.clone(),
            state.request_id,
        )
        .await
        .change_context(errors::VaultError::FetchPaymentMethodFailed)
        .attach_printable("Making get payment method request failed")?;

        let get_card_resp = call_locker_api::<payment_methods::RetrieveCardResp>(
            state,
            request,
            "get_pm_from_locker",
            locker_choice,
        )
        .await
        .change_context(errors::VaultError::FetchPaymentMethodFailed)?;

        let retrieve_card_resp = get_card_resp
            .payload
            .get_required_value("RetrieveCardRespPayload")
            .change_context(errors::VaultError::FetchPaymentMethodFailed)
            .attach_printable("Failed to retrieve field - payload from RetrieveCardResp")?;
        let enc_card_data = retrieve_card_resp
            .enc_card_data
            .get_required_value("enc_card_data")
            .change_context(errors::VaultError::FetchPaymentMethodFailed)
            .attach_printable(
                "Failed to retrieve field - enc_card_data from RetrieveCardRespPayload",
            )?;
        decode_and_decrypt_locker_data(state, key_store, enc_card_data.peek().to_string()).await?
    } else {
        mock_get_payment_method(state, key_store, payment_method_reference)
            .await?
            .payment_method
            .payment_method_data
    };
    Ok(payment_method_data)
}

#[instrument(skip_all)]
pub async fn add_card_to_hs_locker(
    state: &routes::SessionState,
    payload: &payment_methods::StoreLockerReq,
    customer_id: &id_type::CustomerId,
    locker_choice: api_enums::LockerChoice,
) -> errors::CustomResult<payment_methods::StoreCardRespPayload, errors::VaultError> {
    let locker = &state.conf.locker;
    let jwekey = state.conf.jwekey.get_inner();
    let db = &*state.store;
    let stored_card_response = if !locker.mock_locker {
        let request = payment_methods::mk_add_locker_request_hs(
            jwekey,
            locker,
            payload,
            locker_choice,
            state.tenant.tenant_id.clone(),
            state.request_id,
        )
        .await?;
        call_locker_api::<payment_methods::StoreCardResp>(
            state,
            request,
            "add_card_to_hs_locker",
            Some(locker_choice),
        )
        .await
        .change_context(errors::VaultError::SaveCardFailed)?
    } else {
        let card_id = generate_id(consts::ID_LENGTH, "card");
        mock_call_to_locker_hs(db, &card_id, payload, None, None, Some(customer_id)).await?
    };

    let stored_card = stored_card_response
        .payload
        .get_required_value("StoreCardRespPayload")
        .change_context(errors::VaultError::SaveCardFailed)?;
    Ok(stored_card)
}

#[instrument(skip_all)]
pub async fn call_locker_api<T>(
    state: &routes::SessionState,
    request: Request,
    flow_name: &str,
    locker_choice: Option<api_enums::LockerChoice>,
) -> errors::CustomResult<T, errors::VaultError>
where
    T: serde::de::DeserializeOwned,
{
    let locker = &state.conf.locker;
    let jwekey = state.conf.jwekey.get_inner();
    let response_type_name = type_name!(T);

    let response = services::call_connector_api(state, request, flow_name)
        .await
        .change_context(errors::VaultError::ApiError)?;

    let is_locker_call_succeeded = response.is_ok();

    let jwe_body = response
        .unwrap_or_else(|err| err)
        .response
        .parse_struct::<services::JweBody>("JweBody")
        .change_context(errors::VaultError::ResponseDeserializationFailed)
        .attach_printable("Failed while parsing locker response into JweBody")?;

    let decrypted_payload = payment_methods::get_decrypted_response_payload(
        jwekey,
        jwe_body,
        locker_choice,
        locker.decryption_scheme.clone(),
    )
    .await
    .change_context(errors::VaultError::ResponseDeserializationFailed)
    .attach_printable("Failed while decrypting locker payload response")?;

    // Irrespective of locker's response status, payload is JWE + JWS decrypted. But based on locker's status,
    // if Ok, deserialize the decrypted payload into given type T
    // if Err, raise an error including locker error message too
    if is_locker_call_succeeded {
        let stored_card_resp: Result<T, error_stack::Report<errors::VaultError>> =
            decrypted_payload
                .parse_struct(response_type_name)
                .change_context(errors::VaultError::ResponseDeserializationFailed)
                .attach_printable_lazy(|| {
                    format!("Failed while parsing locker response into {response_type_name}")
                });
        stored_card_resp
    } else {
        Err::<T, error_stack::Report<errors::VaultError>>((errors::VaultError::ApiError).into())
            .attach_printable_lazy(|| format!("Locker error response: {decrypted_payload:?}"))
    }
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
pub async fn update_payment_method_metadata_and_last_used(
    state: &routes::SessionState,
    key_store: &domain::MerchantKeyStore,
    db: &dyn db::StorageInterface,
    pm: domain::PaymentMethod,
    pm_metadata: Option<serde_json::Value>,
    storage_scheme: MerchantStorageScheme,
) -> errors::CustomResult<(), errors::VaultError> {
    let pm_update = payment_method::PaymentMethodUpdate::MetadataUpdateAndLastUsed {
        metadata: pm_metadata,
        last_used_at: common_utils::date_time::now(),
    };
    db.update_payment_method(&(state.into()), key_store, pm, pm_update, storage_scheme)
        .await
        .change_context(errors::VaultError::UpdateInPaymentMethodDataTableFailed)?;
    Ok(())
}

pub async fn update_payment_method_and_last_used(
    state: &routes::SessionState,
    key_store: &domain::MerchantKeyStore,
    db: &dyn db::StorageInterface,
    pm: domain::PaymentMethod,
    payment_method_update: Option<Encryption>,
    storage_scheme: MerchantStorageScheme,
    card_scheme: Option<String>,
) -> errors::CustomResult<(), errors::VaultError> {
    let pm_update = payment_method::PaymentMethodUpdate::UpdatePaymentMethodDataAndLastUsed {
        payment_method_data: payment_method_update,
        scheme: card_scheme,
        last_used_at: common_utils::date_time::now(),
    };
    db.update_payment_method(&(state.into()), key_store, pm, pm_update, storage_scheme)
        .await
        .change_context(errors::VaultError::UpdateInPaymentMethodDataTableFailed)?;
    Ok(())
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
pub async fn update_payment_method_connector_mandate_details(
    state: &routes::SessionState,
    key_store: &domain::MerchantKeyStore,
    db: &dyn db::StorageInterface,
    pm: domain::PaymentMethod,
    connector_mandate_details: Option<CommonMandateReference>,
    storage_scheme: MerchantStorageScheme,
) -> errors::CustomResult<(), errors::VaultError> {
    let pm_update = payment_method::PaymentMethodUpdate::ConnectorMandateDetailsUpdate {
        connector_mandate_details: connector_mandate_details.map(|cmd| cmd.into()),
    };

    db.update_payment_method(&(state.into()), key_store, pm, pm_update, storage_scheme)
        .await
        .change_context(errors::VaultError::UpdateInPaymentMethodDataTableFailed)?;
    Ok(())
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
pub async fn update_payment_method_connector_mandate_details(
    state: &routes::SessionState,
    key_store: &domain::MerchantKeyStore,
    db: &dyn db::StorageInterface,
    pm: domain::PaymentMethod,
    connector_mandate_details: Option<CommonMandateReference>,
    storage_scheme: MerchantStorageScheme,
) -> errors::CustomResult<(), errors::VaultError> {
    let connector_mandate_details_value = connector_mandate_details
        .map(|common_mandate| {
            common_mandate.get_mandate_details_value().map_err(|err| {
                router_env::logger::error!("Failed to get get_mandate_details_value : {:?}", err);
                errors::VaultError::UpdateInPaymentMethodDataTableFailed
            })
        })
        .transpose()?;

    let pm_update = payment_method::PaymentMethodUpdate::ConnectorMandateDetailsUpdate {
        connector_mandate_details: connector_mandate_details_value,
    };

    db.update_payment_method(&(state.into()), key_store, pm, pm_update, storage_scheme)
        .await
        .change_context(errors::VaultError::UpdateInPaymentMethodDataTableFailed)?;
    Ok(())
}
#[instrument(skip_all)]
pub async fn get_card_from_hs_locker<'a>(
    state: &'a routes::SessionState,
    customer_id: &id_type::CustomerId,
    merchant_id: &id_type::MerchantId,
    card_reference: &'a str,
    locker_choice: api_enums::LockerChoice,
) -> errors::CustomResult<Card, errors::VaultError> {
    let locker = &state.conf.locker;
    let jwekey = &state.conf.jwekey.get_inner();

    if !locker.mock_locker {
        let request = payment_methods::mk_get_card_request_hs(
            jwekey,
            locker,
            customer_id,
            merchant_id,
            card_reference,
            Some(locker_choice),
            state.tenant.tenant_id.clone(),
            state.request_id,
        )
        .await
        .change_context(errors::VaultError::FetchCardFailed)
        .attach_printable("Making get card request failed")?;
        let get_card_resp = call_locker_api::<payment_methods::RetrieveCardResp>(
            state,
            request,
            "get_card_from_locker",
            Some(locker_choice),
        )
        .await
        .change_context(errors::VaultError::FetchCardFailed)?;

        let retrieve_card_resp = get_card_resp
            .payload
            .get_required_value("RetrieveCardRespPayload")
            .change_context(errors::VaultError::FetchCardFailed)?;
        retrieve_card_resp
            .card
            .get_required_value("Card")
            .change_context(errors::VaultError::FetchCardFailed)
    } else {
        let (get_card_resp, _) = mock_get_card(&*state.store, card_reference).await?;
        payment_methods::mk_get_card_response(get_card_resp)
            .change_context(errors::VaultError::ResponseDeserializationFailed)
    }
}

#[instrument(skip_all)]
pub async fn delete_card_from_hs_locker<'a>(
    state: &routes::SessionState,
    customer_id: &id_type::CustomerId,
    merchant_id: &id_type::MerchantId,
    card_reference: &'a str,
) -> errors::CustomResult<payment_methods::DeleteCardResp, errors::VaultError> {
    let locker = &state.conf.locker;
    let jwekey = &state.conf.jwekey.get_inner();

    let request = payment_methods::mk_delete_card_request_hs(
        jwekey,
        locker,
        customer_id,
        merchant_id,
        card_reference,
        state.tenant.tenant_id.clone(),
        state.request_id,
    )
    .await
    .change_context(errors::VaultError::DeleteCardFailed)
    .attach_printable("Making delete card request failed")?;

    if !locker.mock_locker {
        call_locker_api::<payment_methods::DeleteCardResp>(
            state,
            request,
            "delete_card_from_locker",
            Some(api_enums::LockerChoice::HyperswitchCardVault),
        )
        .await
        .change_context(errors::VaultError::DeleteCardFailed)
    } else {
        Ok(mock_delete_card_hs(&*state.store, card_reference)
            .await
            .change_context(errors::VaultError::DeleteCardFailed)?)
    }
}

// Need to fix this function while completing v2
#[cfg(all(feature = "v2", feature = "customer_v2"))]
#[instrument(skip_all)]
pub async fn delete_card_from_hs_locker_by_global_id<'a>(
    state: &routes::SessionState,
    id: &str,
    merchant_id: &id_type::MerchantId,
    card_reference: &'a str,
) -> errors::RouterResult<payment_methods::DeleteCardResp> {
    todo!()
}

///Mock api for local testing
pub async fn mock_call_to_locker_hs(
    db: &dyn db::StorageInterface,
    card_id: &str,
    payload: &payment_methods::StoreLockerReq,
    card_cvc: Option<String>,
    payment_method_id: Option<String>,
    customer_id: Option<&id_type::CustomerId>,
) -> errors::CustomResult<payment_methods::StoreCardResp, errors::VaultError> {
    let mut locker_mock_up = storage::LockerMockUpNew {
        card_id: card_id.to_string(),
        external_id: uuid::Uuid::new_v4().to_string(),
        card_fingerprint: uuid::Uuid::new_v4().to_string(),
        card_global_fingerprint: uuid::Uuid::new_v4().to_string(),
        merchant_id: id_type::MerchantId::default(),
        card_number: "4111111111111111".to_string(),
        card_exp_year: "2099".to_string(),
        card_exp_month: "12".to_string(),
        card_cvc,
        payment_method_id,
        customer_id: customer_id.map(ToOwned::to_owned),
        name_on_card: None,
        nickname: None,
        enc_card_data: None,
    };
    locker_mock_up = match payload {
        payment_methods::StoreLockerReq::LockerCard(store_card_req) => storage::LockerMockUpNew {
            merchant_id: store_card_req.merchant_id.to_owned(),
            card_number: store_card_req.card.card_number.peek().to_string(),
            card_exp_year: store_card_req.card.card_exp_year.peek().to_string(),
            card_exp_month: store_card_req.card.card_exp_month.peek().to_string(),
            name_on_card: store_card_req.card.name_on_card.to_owned().expose_option(),
            nickname: store_card_req.card.nick_name.to_owned(),
            ..locker_mock_up
        },
        payment_methods::StoreLockerReq::LockerGeneric(store_generic_req) => {
            storage::LockerMockUpNew {
                merchant_id: store_generic_req.merchant_id.to_owned(),
                enc_card_data: Some(store_generic_req.enc_data.to_owned()),
                ..locker_mock_up
            }
        }
    };

    let response = db
        .insert_locker_mock_up(locker_mock_up)
        .await
        .change_context(errors::VaultError::SaveCardFailed)?;
    let payload = payment_methods::StoreCardRespPayload {
        card_reference: response.card_id,
        duplication_check: None,
    };
    Ok(payment_methods::StoreCardResp {
        status: "Ok".to_string(),
        error_code: None,
        error_message: None,
        payload: Some(payload),
    })
}

#[instrument(skip_all)]
pub async fn mock_get_card<'a>(
    db: &dyn db::StorageInterface,
    card_id: &'a str,
) -> errors::CustomResult<(payment_methods::GetCardResponse, Option<String>), errors::VaultError> {
    let locker_mock_up = db
        .find_locker_by_card_id(card_id)
        .await
        .change_context(errors::VaultError::FetchCardFailed)?;
    let add_card_response = payment_methods::AddCardResponse {
        card_id: locker_mock_up
            .payment_method_id
            .unwrap_or(locker_mock_up.card_id),
        external_id: locker_mock_up.external_id,
        card_fingerprint: locker_mock_up.card_fingerprint.into(),
        card_global_fingerprint: locker_mock_up.card_global_fingerprint.into(),
        merchant_id: Some(locker_mock_up.merchant_id),
        card_number: cards::CardNumber::try_from(locker_mock_up.card_number)
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Invalid card number format from the mock locker")
            .map(Some)?,
        card_exp_year: Some(locker_mock_up.card_exp_year.into()),
        card_exp_month: Some(locker_mock_up.card_exp_month.into()),
        name_on_card: locker_mock_up.name_on_card.map(|card| card.into()),
        nickname: locker_mock_up.nickname,
        customer_id: locker_mock_up.customer_id,
        duplicate: locker_mock_up.duplicate,
    };
    Ok((
        payment_methods::GetCardResponse {
            card: add_card_response,
        },
        locker_mock_up.card_cvc,
    ))
}

#[instrument(skip_all)]
pub async fn mock_get_payment_method<'a>(
    state: &routes::SessionState,
    key_store: &domain::MerchantKeyStore,
    card_id: &'a str,
) -> errors::CustomResult<payment_methods::GetPaymentMethodResponse, errors::VaultError> {
    let db = &*state.store;
    let locker_mock_up = db
        .find_locker_by_card_id(card_id)
        .await
        .change_context(errors::VaultError::FetchPaymentMethodFailed)?;
    let dec_data = if let Some(e) = locker_mock_up.enc_card_data {
        decode_and_decrypt_locker_data(state, key_store, e).await
    } else {
        Err(report!(errors::VaultError::FetchPaymentMethodFailed))
    }?;
    let payment_method_response = payment_methods::AddPaymentMethodResponse {
        payment_method_id: locker_mock_up
            .payment_method_id
            .unwrap_or(locker_mock_up.card_id),
        external_id: locker_mock_up.external_id,
        merchant_id: Some(locker_mock_up.merchant_id.to_owned()),
        nickname: locker_mock_up.nickname,
        customer_id: locker_mock_up.customer_id,
        duplicate: locker_mock_up.duplicate,
        payment_method_data: dec_data,
    };
    Ok(payment_methods::GetPaymentMethodResponse {
        payment_method: payment_method_response,
    })
}

#[instrument(skip_all)]
pub async fn mock_delete_card_hs<'a>(
    db: &dyn db::StorageInterface,
    card_id: &'a str,
) -> errors::CustomResult<payment_methods::DeleteCardResp, errors::VaultError> {
    db.delete_locker_mock_up(card_id)
        .await
        .change_context(errors::VaultError::FetchCardFailed)?;
    Ok(payment_methods::DeleteCardResp {
        status: "Ok".to_string(),
        error_code: None,
        error_message: None,
    })
}

#[instrument(skip_all)]
pub async fn mock_delete_card<'a>(
    db: &dyn db::StorageInterface,
    card_id: &'a str,
) -> errors::CustomResult<payment_methods::DeleteCardResponse, errors::VaultError> {
    let locker_mock_up = db
        .delete_locker_mock_up(card_id)
        .await
        .change_context(errors::VaultError::FetchCardFailed)?;
    Ok(payment_methods::DeleteCardResponse {
        card_id: Some(locker_mock_up.card_id),
        external_id: Some(locker_mock_up.external_id),
        card_isin: None,
        status: "Ok".to_string(),
    })
}
//------------------------------------------------------------------------------
pub fn get_banks(
    state: &routes::SessionState,
    pm_type: common_enums::enums::PaymentMethodType,
    connectors: Vec<String>,
) -> Result<Vec<BankCodeResponse>, errors::ApiErrorResponse> {
    let mut bank_names_hm: HashMap<String, HashSet<common_enums::enums::BankNames>> =
        HashMap::new();

    if matches!(
        pm_type,
        api_enums::PaymentMethodType::Giropay | api_enums::PaymentMethodType::Sofort
    ) {
        Ok(vec![BankCodeResponse {
            bank_name: vec![],
            eligible_connectors: connectors,
        }])
    } else {
        let mut bank_code_responses = vec![];
        for connector in &connectors {
            if let Some(connector_bank_names) = state.conf.bank_config.0.get(&pm_type) {
                if let Some(connector_hash_set) = connector_bank_names.0.get(connector) {
                    bank_names_hm.insert(connector.clone(), connector_hash_set.banks.clone());
                } else {
                    logger::error!("Could not find any configured connectors for payment_method -> {pm_type} for connector -> {connector}");
                }
            } else {
                logger::error!("Could not find any configured banks for payment_method -> {pm_type} for connector -> {connector}");
            }
        }

        let vector_of_hashsets = bank_names_hm
            .values()
            .map(|bank_names_hashset| bank_names_hashset.to_owned())
            .collect::<Vec<_>>();

        let mut common_bank_names = HashSet::new();
        if let Some(first_element) = vector_of_hashsets.first() {
            common_bank_names = vector_of_hashsets
                .iter()
                .skip(1)
                .fold(first_element.to_owned(), |acc, hs| {
                    acc.intersection(hs).copied().collect()
                });
        }

        if !common_bank_names.is_empty() {
            bank_code_responses.push(BankCodeResponse {
                bank_name: common_bank_names.clone().into_iter().collect(),
                eligible_connectors: connectors.clone(),
            });
        }

        for connector in connectors {
            if let Some(all_bank_codes_for_connector) = bank_names_hm.get(&connector) {
                let remaining_bank_codes: HashSet<_> = all_bank_codes_for_connector
                    .difference(&common_bank_names)
                    .collect();

                if !remaining_bank_codes.is_empty() {
                    bank_code_responses.push(BankCodeResponse {
                        bank_name: remaining_bank_codes
                            .into_iter()
                            .map(|ele| ele.to_owned())
                            .collect(),
                        eligible_connectors: vec![connector],
                    })
                }
            } else {
                logger::error!("Could not find any configured banks for payment_method -> {pm_type} for connector -> {connector}");
            }
        }
        Ok(bank_code_responses)
    }
}

fn get_val(str: String, val: &serde_json::Value) -> Option<String> {
    str.split('.')
        .try_fold(val, |acc, x| acc.get(x))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "customer_v2"),
    not(feature = "payment_methods_v2")
))]
pub async fn list_payment_methods(
    state: routes::SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    mut req: api::PaymentMethodListRequest,
) -> errors::RouterResponse<api::PaymentMethodListResponse> {
    let db = &*state.store;
    let pm_config_mapping = &state.conf.pm_filters;
    let key_manager_state = &(&state).into();
    let payment_intent = if let Some(cs) = &req.client_secret {
        if cs.starts_with("pm_") {
            validate_payment_method_and_client_secret(
                &state,
                cs,
                db,
                &merchant_account,
                &key_store,
            )
            .await?;
            None
        } else {
            helpers::verify_payment_intent_time_and_client_secret(
                &state,
                &merchant_account,
                &key_store,
                req.client_secret.clone(),
            )
            .await?
        }
    } else {
        None
    };

    let shipping_address = payment_intent
        .as_ref()
        .async_map(|pi| async {
            helpers::get_address_by_id(
                &state,
                pi.shipping_address_id.clone(),
                &key_store,
                &pi.payment_id,
                merchant_account.get_id(),
                merchant_account.storage_scheme,
            )
            .await
        })
        .await
        .transpose()?
        .flatten();

    let billing_address = payment_intent
        .as_ref()
        .async_map(|pi| async {
            helpers::get_address_by_id(
                &state,
                pi.billing_address_id.clone(),
                &key_store,
                &pi.payment_id,
                merchant_account.get_id(),
                merchant_account.storage_scheme,
            )
            .await
        })
        .await
        .transpose()?
        .flatten();

    let customer = payment_intent
        .as_ref()
        .async_and_then(|pi| async {
            pi.customer_id
                .as_ref()
                .async_and_then(|cust| async {
                    db.find_customer_by_customer_id_merchant_id(
                        key_manager_state,
                        cust,
                        &pi.merchant_id,
                        &key_store,
                        merchant_account.storage_scheme,
                    )
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)
                    .ok()
                })
                .await
        })
        .await;

    let payment_attempt = payment_intent
        .as_ref()
        .async_map(|pi| async {
            db.find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                &pi.payment_id,
                &pi.merchant_id,
                &pi.active_attempt.get_id(),
                merchant_account.storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::PaymentNotFound)
        })
        .await
        .transpose()?;
    let setup_future_usage = payment_intent.as_ref().and_then(|pi| pi.setup_future_usage);
    let is_cit_transaction = payment_attempt
        .as_ref()
        .map(|pa| pa.mandate_details.is_some())
        .unwrap_or(false)
        || setup_future_usage
            .map(|future_usage| future_usage == common_enums::FutureUsage::OffSession)
            .unwrap_or(false);
    let payment_type = payment_attempt.as_ref().map(|pa| {
        let amount = api::Amount::from(pa.net_amount.get_order_amount());
        let mandate_type = if pa.mandate_id.is_some() {
            Some(api::MandateTransactionType::RecurringMandateTransaction)
        } else if is_cit_transaction {
            Some(api::MandateTransactionType::NewMandateTransaction)
        } else {
            None
        };

        helpers::infer_payment_type(amount, mandate_type.as_ref())
    });

    let all_mcas = db
        .find_merchant_connector_account_by_merchant_id_and_disabled_list(
            key_manager_state,
            merchant_account.get_id(),
            false,
            &key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let profile_id = payment_intent
        .as_ref()
        .map(|payment_intent| {
            payment_intent
                .profile_id
                .clone()
                .get_required_value("profile_id")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("profile_id is not set in payment_intent")
        })
        .transpose()?;
    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        key_manager_state,
        &key_store,
        profile_id.as_ref(),
        merchant_account.get_id(),
    )
    .await?;

    let profile_id = profile_id
        .clone()
        .get_required_value("profile_id")
        .change_context(errors::ApiErrorResponse::GenericNotFoundError {
            message: "Profile id not found".to_string(),
        })?;

    // filter out payment connectors based on profile_id
    let filtered_mcas = helpers::filter_mca_based_on_profile_and_connector_type(
        all_mcas.clone(),
        &profile_id,
        ConnectorType::PaymentProcessor,
    );

    logger::debug!(mca_before_filtering=?filtered_mcas);

    let mut response: Vec<ResponsePaymentMethodIntermediate> = vec![];
    // Key creation for storing PM_FILTER_CGRAPH
    let key = {
        format!(
            "pm_filters_cgraph_{}_{}",
            merchant_account.get_id().get_string_repr(),
            profile_id.get_string_repr()
        )
    };

    if let Some(graph) = get_merchant_pm_filter_graph(&state, &key).await {
        // Derivation of PM_FILTER_CGRAPH from MokaCache successful
        for mca in &filtered_mcas {
            let payment_methods = match &mca.payment_methods_enabled {
                Some(pm) => pm,
                None => continue,
            };
            filter_payment_methods(
                &graph,
                mca.get_id(),
                payment_methods,
                &mut req,
                &mut response,
                payment_intent.as_ref(),
                payment_attempt.as_ref(),
                billing_address.as_ref(),
                mca.connector_name.clone(),
                &state.conf.saved_payment_methods,
            )
            .await?;
        }
    } else {
        // No PM_FILTER_CGRAPH Cache present in MokaCache
        let mut builder = cgraph::ConstraintGraphBuilder::new();
        for mca in &filtered_mcas {
            let domain_id = builder.make_domain(
                mca.get_id().get_string_repr().to_string(),
                mca.connector_name.as_str(),
            );

            let Ok(domain_id) = domain_id else {
                logger::error!("Failed to construct domain for list payment methods");
                return Err(errors::ApiErrorResponse::InternalServerError.into());
            };

            let payment_methods = match &mca.payment_methods_enabled {
                Some(pm) => pm,
                None => continue,
            };
            if let Err(e) = make_pm_graph(
                &mut builder,
                domain_id,
                payment_methods,
                mca.connector_name.clone(),
                pm_config_mapping,
                &state.conf.mandates.supported_payment_methods,
                &state.conf.mandates.update_mandate_supported,
            ) {
                logger::error!(
                    "Failed to construct constraint graph for list payment methods {e:?}"
                );
            }
        }

        // Refreshing our CGraph cache
        let graph = refresh_pm_filters_cache(&state, &key, builder.build()).await;

        for mca in &filtered_mcas {
            let payment_methods = match &mca.payment_methods_enabled {
                Some(pm) => pm,
                None => continue,
            };
            filter_payment_methods(
                &graph,
                mca.get_id().clone(),
                payment_methods,
                &mut req,
                &mut response,
                payment_intent.as_ref(),
                payment_attempt.as_ref(),
                billing_address.as_ref(),
                mca.connector_name.clone(),
                &state.conf.saved_payment_methods,
            )
            .await?;
        }
    }
    logger::info!(
        "The Payment Methods available after Constraint Graph filtering are {:?}",
        response
    );

    // Filter out wallet payment method from mca if customer has already saved it
    customer
        .as_ref()
        .async_map(|customer| async {
            let wallet_pm_exists = response
                .iter()
                .any(|mca| mca.payment_method == enums::PaymentMethod::Wallet);
            if wallet_pm_exists {
                match db
                    .find_payment_method_by_customer_id_merchant_id_status(
                        &((&state).into()),
                        &key_store,
                        &customer.customer_id,
                        merchant_account.get_id(),
                        common_enums::PaymentMethodStatus::Active,
                        None,
                        merchant_account.storage_scheme,
                    )
                    .await
                {
                    Ok(customer_payment_methods) => {
                        let customer_wallet_pm = customer_payment_methods
                            .iter()
                            .filter(|cust_pm| {
                                cust_pm.get_payment_method_type() == Some(enums::PaymentMethod::Wallet)
                            })
                            .collect::<Vec<_>>();

                        response.retain(|mca| {
                            !(mca.payment_method == enums::PaymentMethod::Wallet
                                && customer_wallet_pm.iter().any(|cust_pm| {
                                    cust_pm.get_payment_method_subtype() == Some(mca.payment_method_type)
                                }))
                        });

                        logger::debug!("Filtered out wallet payment method from mca since customer has already saved it");
                        Ok(())
                    }
                    Err(error) => {
                        if error.current_context().is_db_not_found() {
                            Ok(())
                        } else {
                            Err(error)
                                .change_context(errors::ApiErrorResponse::InternalServerError)
                                .attach_printable("failed to find payment methods for a customer")
                        }
                    }
                }
            } else {
                Ok(())
            }
        })
        .await
        .transpose()?;

    let mut pmt_to_auth_connector: HashMap<
        enums::PaymentMethod,
        HashMap<enums::PaymentMethodType, String>,
    > = HashMap::new();

    if let Some((payment_attempt, payment_intent)) =
        payment_attempt.as_ref().zip(payment_intent.as_ref())
    {
        let routing_enabled_pms = HashSet::from([
            api_enums::PaymentMethod::BankTransfer,
            api_enums::PaymentMethod::BankDebit,
            api_enums::PaymentMethod::BankRedirect,
        ]);

        let routing_enabled_pm_types = HashSet::from([
            api_enums::PaymentMethodType::GooglePay,
            api_enums::PaymentMethodType::ApplePay,
            api_enums::PaymentMethodType::Klarna,
            api_enums::PaymentMethodType::Paypal,
            api_enums::PaymentMethodType::SamsungPay,
        ]);

        let mut chosen = Vec::<api::SessionConnectorData>::new();
        for intermediate in &response {
            if routing_enabled_pm_types.contains(&intermediate.payment_method_type)
                || routing_enabled_pms.contains(&intermediate.payment_method)
            {
                let connector_data = api::ConnectorData::get_connector_by_name(
                    &state.clone().conf.connectors,
                    &intermediate.connector,
                    api::GetToken::from(intermediate.payment_method_type),
                    None,
                )
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("invalid connector name received")?;

                chosen.push(api::SessionConnectorData {
                    payment_method_type: intermediate.payment_method_type,
                    connector: connector_data,
                    business_sub_label: None,
                });
            }
        }
        let sfr = SessionFlowRoutingInput {
            state: &state,
            country: billing_address.clone().and_then(|ad| ad.country),
            key_store: &key_store,
            merchant_account: &merchant_account,
            payment_attempt,
            payment_intent,
            chosen,
        };
        let result = routing::perform_session_flow_routing(sfr, &enums::TransactionType::Payment)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("error performing session flow routing")?;

        response.retain(|intermediate| {
            if !routing_enabled_pm_types.contains(&intermediate.payment_method_type)
                && !routing_enabled_pms.contains(&intermediate.payment_method)
            {
                return true;
            }

            if let Some(choice) = result.get(&intermediate.payment_method_type) {
                if let Some(first_routable_connector) = choice.first() {
                    intermediate.connector
                        == first_routable_connector
                            .connector
                            .connector_name
                            .to_string()
                        && first_routable_connector
                            .connector
                            .merchant_connector_id
                            .as_ref()
                            .map(|merchant_connector_id| {
                                *merchant_connector_id.get_string_repr()
                                    == intermediate.merchant_connector_id
                            })
                            .unwrap_or_default()
                } else {
                    false
                }
            } else {
                false
            }
        });

        let mut routing_info: storage::PaymentRoutingInfo = payment_attempt
            .straight_through_algorithm
            .clone()
            .map(|val| val.parse_value("PaymentRoutingInfo"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Invalid PaymentRoutingInfo format found in payment attempt")?
            .unwrap_or(storage::PaymentRoutingInfo {
                algorithm: None,
                pre_routing_results: None,
            });

        let mut pre_routing_results: HashMap<
            api_enums::PaymentMethodType,
            storage::PreRoutingConnectorChoice,
        > = HashMap::new();

        for (pm_type, routing_choice) in result {
            let mut routable_choice_list = vec![];
            for choice in routing_choice {
                let routable_choice = routing_types::RoutableConnectorChoice {
                    choice_kind: routing_types::RoutableChoiceKind::FullStruct,
                    connector: choice
                        .connector
                        .connector_name
                        .to_string()
                        .parse::<api_enums::RoutableConnectors>()
                        .change_context(errors::ApiErrorResponse::InternalServerError)?,
                    merchant_connector_id: choice.connector.merchant_connector_id.clone(),
                };
                routable_choice_list.push(routable_choice);
            }
            pre_routing_results.insert(
                pm_type,
                storage::PreRoutingConnectorChoice::Multiple(routable_choice_list),
            );
        }

        let redis_conn = db
            .get_redis_conn()
            .map_err(|redis_error| logger::error!(?redis_error))
            .ok();

        let mut val = Vec::new();

        for (payment_method_type, routable_connector_choice) in &pre_routing_results {
            let routable_connector_list = match routable_connector_choice {
                storage::PreRoutingConnectorChoice::Single(routable_connector) => {
                    vec![routable_connector.clone()]
                }
                storage::PreRoutingConnectorChoice::Multiple(routable_connector_list) => {
                    routable_connector_list.clone()
                }
            };

            let first_routable_connector = routable_connector_list
                .first()
                .ok_or(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration)?;

            let matched_mca = filtered_mcas.iter().find(|m| {
                first_routable_connector.merchant_connector_id.as_ref() == Some(&m.get_id())
            });

            if let Some(m) = matched_mca {
                let pm_auth_config = m
                    .pm_auth_config
                    .as_ref()
                    .map(|config| {
                        serde_json::from_value::<PaymentMethodAuthConfig>(config.clone().expose())
                            .change_context(errors::StorageError::DeserializationFailed)
                            .attach_printable("Failed to deserialize Payment Method Auth config")
                    })
                    .transpose()
                    .unwrap_or_else(|error| {
                        logger::error!(?error);
                        None
                    });

                if let Some(config) = pm_auth_config {
                    for inner_config in config.enabled_payment_methods.iter() {
                        let is_active_mca = all_mcas
                            .iter()
                            .any(|mca| mca.get_id() == inner_config.mca_id);

                        if inner_config.payment_method_type == *payment_method_type && is_active_mca
                        {
                            let pm = pmt_to_auth_connector
                                .get(&inner_config.payment_method)
                                .cloned();

                            let inner_map = if let Some(mut inner_map) = pm {
                                inner_map.insert(
                                    *payment_method_type,
                                    inner_config.connector_name.clone(),
                                );
                                inner_map
                            } else {
                                HashMap::from([(
                                    *payment_method_type,
                                    inner_config.connector_name.clone(),
                                )])
                            };

                            pmt_to_auth_connector.insert(inner_config.payment_method, inner_map);
                            val.push(inner_config.clone());
                        }
                    }
                };
            }
        }

        let pm_auth_key = payment_intent.payment_id.get_pm_auth_key();
        let redis_expiry = state.conf.payment_method_auth.get_inner().redis_expiry;

        if let Some(rc) = redis_conn {
            rc.serialize_and_set_key_with_expiry(&pm_auth_key.as_str().into(), val, redis_expiry)
                .await
                .attach_printable("Failed to store pm auth data in redis")
                .unwrap_or_else(|error| {
                    logger::error!(?error);
                })
        };

        routing_info.pre_routing_results = Some(pre_routing_results);

        let encoded = routing_info
            .encode_to_value()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to serialize payment routing info to value")?;

        let attempt_update = storage::PaymentAttemptUpdate::UpdateTrackers {
            payment_token: None,
            connector: None,
            straight_through_algorithm: Some(encoded),
            amount_capturable: None,
            updated_by: merchant_account.storage_scheme.to_string(),
            merchant_connector_id: None,
            surcharge_amount: None,
            tax_amount: None,
        };

        state
            .store
            .update_payment_attempt_with_attempt_id(
                payment_attempt.clone(),
                attempt_update,
                merchant_account.storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
    }

    // Check for `use_billing_as_payment_method_billing` config under business_profile
    // If this is disabled, then the billing details in required fields will be empty and have to be collected by the customer
    let billing_address_for_calculating_required_fields = business_profile
        .as_ref()
        .and_then(|business_profile| business_profile.use_billing_as_payment_method_billing)
        .unwrap_or(true)
        .then_some(billing_address.as_ref())
        .flatten();

    let req = api_models::payments::PaymentsRequest::foreign_try_from((
        payment_attempt.as_ref(),
        payment_intent.as_ref(),
        shipping_address.as_ref(),
        billing_address_for_calculating_required_fields,
        customer.as_ref(),
    ))?;

    let req_val = serde_json::to_value(req).ok();
    logger::debug!(filtered_payment_methods=?response);

    let mut payment_experiences_consolidated_hm: HashMap<
        api_enums::PaymentMethod,
        HashMap<api_enums::PaymentMethodType, HashMap<api_enums::PaymentExperience, Vec<String>>>,
    > = HashMap::new();

    let mut card_networks_consolidated_hm: HashMap<
        api_enums::PaymentMethod,
        HashMap<api_enums::PaymentMethodType, HashMap<api_enums::CardNetwork, Vec<String>>>,
    > = HashMap::new();

    let mut banks_consolidated_hm: HashMap<api_enums::PaymentMethodType, Vec<String>> =
        HashMap::new();

    let mut bank_debits_consolidated_hm =
        HashMap::<api_enums::PaymentMethodType, Vec<String>>::new();

    let mut bank_transfer_consolidated_hm =
        HashMap::<api_enums::PaymentMethodType, Vec<String>>::new();

    // All the required fields will be stored here and later filtered out based on business profile config
    let mut required_fields_hm = HashMap::<
        api_enums::PaymentMethod,
        HashMap<api_enums::PaymentMethodType, HashMap<String, RequiredFieldInfo>>,
    >::new();

    for element in response.clone() {
        let payment_method = element.payment_method;
        let payment_method_type = element.payment_method_type;
        let connector = element.connector.clone();

        let connector_variant = api_enums::Connector::from_str(connector.as_str())
            .change_context(errors::ConnectorError::InvalidConnectorName)
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "connector",
            })
            .attach_printable_lazy(|| format!("unable to parse connector name {connector:?}"))?;
        state.conf.required_fields.0.get(&payment_method).map(
            |required_fields_hm_for_each_payment_method_type| {
                required_fields_hm_for_each_payment_method_type
                    .0
                    .get(&payment_method_type)
                    .map(|required_fields_hm_for_each_connector| {
                        required_fields_hm.entry(payment_method).or_default();
                        required_fields_hm_for_each_connector
                            .fields
                            .get(&connector_variant)
                            .map(|required_fields_final| {
                                let mut required_fields_hs = required_fields_final.common.clone();
                                    if is_cit_transaction {
                                        required_fields_hs
                                            .extend(required_fields_final.mandate.clone());
                                    } else {
                                        required_fields_hs
                                            .extend(required_fields_final.non_mandate.clone());
                                    }
                                 required_fields_hs = should_collect_shipping_or_billing_details_from_wallet_connector(
                                    payment_method,
                                    element.payment_experience.as_ref(),
                                    business_profile.as_ref(),
                                    required_fields_hs.clone(),
                                );

                                // get the config, check the enums while adding
                                {
                                    for (key, val) in &mut required_fields_hs {
                                        let temp = req_val
                                            .as_ref()
                                            .and_then(|r| get_val(key.to_owned(), r));
                                        if let Some(s) = temp {
                                            val.value = Some(s.into())
                                        };
                                    }
                                }

                                let existing_req_fields_hs = required_fields_hm
                                    .get_mut(&payment_method)
                                    .and_then(|inner_hm| inner_hm.get_mut(&payment_method_type));

                                // If payment_method_type already exist in required_fields_hm, extend the required_fields hs to existing hs.
                                if let Some(inner_hs) = existing_req_fields_hs {
                                    inner_hs.extend(required_fields_hs);
                                } else {
                                    required_fields_hm.get_mut(&payment_method).map(|inner_hm| {
                                        inner_hm.insert(payment_method_type, required_fields_hs)
                                    });
                                }
                            })
                    })
            },
        );

        if let Some(payment_experience) = element.payment_experience {
            if let Some(payment_method_hm) =
                payment_experiences_consolidated_hm.get_mut(&payment_method)
            {
                if let Some(payment_method_type_hm) =
                    payment_method_hm.get_mut(&payment_method_type)
                {
                    if let Some(vector_of_connectors) =
                        payment_method_type_hm.get_mut(&payment_experience)
                    {
                        vector_of_connectors.push(connector);
                    } else {
                        payment_method_type_hm.insert(payment_experience, vec![connector]);
                    }
                } else {
                    payment_method_hm.insert(
                        payment_method_type,
                        HashMap::from([(payment_experience, vec![connector])]),
                    );
                }
            } else {
                let inner_hm = HashMap::from([(payment_experience, vec![connector])]);
                let payment_method_type_hm = HashMap::from([(payment_method_type, inner_hm)]);
                payment_experiences_consolidated_hm.insert(payment_method, payment_method_type_hm);
            }
        }

        if let Some(card_networks) = element.card_networks {
            if let Some(payment_method_hm) = card_networks_consolidated_hm.get_mut(&payment_method)
            {
                if let Some(payment_method_type_hm) =
                    payment_method_hm.get_mut(&payment_method_type)
                {
                    for card_network in card_networks {
                        if let Some(vector_of_connectors) =
                            payment_method_type_hm.get_mut(&card_network)
                        {
                            let connector = element.connector.clone();
                            vector_of_connectors.push(connector);
                        } else {
                            let connector = element.connector.clone();
                            payment_method_type_hm.insert(card_network, vec![connector]);
                        }
                    }
                } else {
                    let mut inner_hashmap: HashMap<api_enums::CardNetwork, Vec<String>> =
                        HashMap::new();
                    for card_network in card_networks {
                        if let Some(vector_of_connectors) = inner_hashmap.get_mut(&card_network) {
                            let connector = element.connector.clone();
                            vector_of_connectors.push(connector);
                        } else {
                            let connector = element.connector.clone();
                            inner_hashmap.insert(card_network, vec![connector]);
                        }
                    }
                    payment_method_hm.insert(payment_method_type, inner_hashmap);
                }
            } else {
                let mut inner_hashmap: HashMap<api_enums::CardNetwork, Vec<String>> =
                    HashMap::new();
                for card_network in card_networks {
                    if let Some(vector_of_connectors) = inner_hashmap.get_mut(&card_network) {
                        let connector = element.connector.clone();
                        vector_of_connectors.push(connector);
                    } else {
                        let connector = element.connector.clone();
                        inner_hashmap.insert(card_network, vec![connector]);
                    }
                }
                let payment_method_type_hm = HashMap::from([(payment_method_type, inner_hashmap)]);
                card_networks_consolidated_hm.insert(payment_method, payment_method_type_hm);
            }
        }

        if element.payment_method == api_enums::PaymentMethod::BankRedirect {
            let connector = element.connector.clone();
            if let Some(vector_of_connectors) =
                banks_consolidated_hm.get_mut(&element.payment_method_type)
            {
                vector_of_connectors.push(connector);
            } else {
                banks_consolidated_hm.insert(element.payment_method_type, vec![connector]);
            }
        }

        if element.payment_method == api_enums::PaymentMethod::BankDebit {
            let connector = element.connector.clone();
            if let Some(vector_of_connectors) =
                bank_debits_consolidated_hm.get_mut(&element.payment_method_type)
            {
                vector_of_connectors.push(connector);
            } else {
                bank_debits_consolidated_hm.insert(element.payment_method_type, vec![connector]);
            }
        }

        if element.payment_method == api_enums::PaymentMethod::BankTransfer {
            let connector = element.connector.clone();
            if let Some(vector_of_connectors) =
                bank_transfer_consolidated_hm.get_mut(&element.payment_method_type)
            {
                vector_of_connectors.push(connector);
            } else {
                bank_transfer_consolidated_hm.insert(element.payment_method_type, vec![connector]);
            }
        }
    }

    let mut payment_method_responses: Vec<ResponsePaymentMethodsEnabled> = vec![];
    for key in payment_experiences_consolidated_hm.iter() {
        let mut payment_method_types = vec![];
        for payment_method_types_hm in key.1 {
            let mut payment_experience_types = vec![];
            for payment_experience_type in payment_method_types_hm.1 {
                payment_experience_types.push(PaymentExperienceTypes {
                    payment_experience_type: *payment_experience_type.0,
                    eligible_connectors: payment_experience_type.1.clone(),
                })
            }

            payment_method_types.push(ResponsePaymentMethodTypes {
                payment_method_type: *payment_method_types_hm.0,
                payment_experience: Some(payment_experience_types),
                card_networks: None,
                bank_names: None,
                bank_debits: None,
                bank_transfers: None,
                // Required fields for PayLater payment method
                required_fields: required_fields_hm
                    .get(key.0)
                    .and_then(|inner_hm| inner_hm.get(payment_method_types_hm.0))
                    .cloned(),
                surcharge_details: None,
                pm_auth_connector: pmt_to_auth_connector
                    .get(key.0)
                    .and_then(|pm_map| pm_map.get(payment_method_types_hm.0))
                    .cloned(),
            })
        }

        payment_method_responses.push(ResponsePaymentMethodsEnabled {
            payment_method: *key.0,
            payment_method_types,
        })
    }

    for key in card_networks_consolidated_hm.iter() {
        let mut payment_method_types = vec![];
        for payment_method_types_hm in key.1 {
            let mut card_network_types = vec![];
            for card_network_type in payment_method_types_hm.1 {
                card_network_types.push(CardNetworkTypes {
                    card_network: card_network_type.0.clone(),
                    eligible_connectors: card_network_type.1.clone(),
                    surcharge_details: None,
                })
            }

            payment_method_types.push(ResponsePaymentMethodTypes {
                payment_method_type: *payment_method_types_hm.0,
                card_networks: Some(card_network_types),
                payment_experience: None,
                bank_names: None,
                bank_debits: None,
                bank_transfers: None,
                // Required fields for Card payment method
                required_fields: required_fields_hm
                    .get(key.0)
                    .and_then(|inner_hm| inner_hm.get(payment_method_types_hm.0))
                    .cloned(),
                surcharge_details: None,
                pm_auth_connector: pmt_to_auth_connector
                    .get(key.0)
                    .and_then(|pm_map| pm_map.get(payment_method_types_hm.0))
                    .cloned(),
            })
        }

        payment_method_responses.push(ResponsePaymentMethodsEnabled {
            payment_method: *key.0,
            payment_method_types,
        })
    }

    let mut bank_redirect_payment_method_types = vec![];

    for key in banks_consolidated_hm.iter() {
        let payment_method_type = *key.0;
        let connectors = key.1.clone();
        let bank_names = get_banks(&state, payment_method_type, connectors)?;
        bank_redirect_payment_method_types.push({
            ResponsePaymentMethodTypes {
                payment_method_type,
                bank_names: Some(bank_names),
                payment_experience: None,
                card_networks: None,
                bank_debits: None,
                bank_transfers: None,
                // Required fields for BankRedirect payment method
                required_fields: required_fields_hm
                    .get(&api_enums::PaymentMethod::BankRedirect)
                    .and_then(|inner_hm| inner_hm.get(key.0))
                    .cloned(),
                surcharge_details: None,
                pm_auth_connector: pmt_to_auth_connector
                    .get(&enums::PaymentMethod::BankRedirect)
                    .and_then(|pm_map| pm_map.get(key.0))
                    .cloned(),
            }
        })
    }

    if !bank_redirect_payment_method_types.is_empty() {
        payment_method_responses.push(ResponsePaymentMethodsEnabled {
            payment_method: api_enums::PaymentMethod::BankRedirect,
            payment_method_types: bank_redirect_payment_method_types,
        });
    }

    let mut bank_debit_payment_method_types = vec![];

    for key in bank_debits_consolidated_hm.iter() {
        let payment_method_type = *key.0;
        let connectors = key.1.clone();
        bank_debit_payment_method_types.push({
            ResponsePaymentMethodTypes {
                payment_method_type,
                bank_names: None,
                payment_experience: None,
                card_networks: None,
                bank_debits: Some(api_models::payment_methods::BankDebitTypes {
                    eligible_connectors: connectors.clone(),
                }),
                bank_transfers: None,
                // Required fields for BankDebit payment method
                required_fields: required_fields_hm
                    .get(&api_enums::PaymentMethod::BankDebit)
                    .and_then(|inner_hm| inner_hm.get(key.0))
                    .cloned(),
                surcharge_details: None,
                pm_auth_connector: pmt_to_auth_connector
                    .get(&enums::PaymentMethod::BankDebit)
                    .and_then(|pm_map| pm_map.get(key.0))
                    .cloned(),
            }
        })
    }

    if !bank_debit_payment_method_types.is_empty() {
        payment_method_responses.push(ResponsePaymentMethodsEnabled {
            payment_method: api_enums::PaymentMethod::BankDebit,
            payment_method_types: bank_debit_payment_method_types,
        });
    }

    let mut bank_transfer_payment_method_types = vec![];

    for key in bank_transfer_consolidated_hm.iter() {
        let payment_method_type = *key.0;
        let connectors = key.1.clone();
        bank_transfer_payment_method_types.push({
            ResponsePaymentMethodTypes {
                payment_method_type,
                bank_names: None,
                payment_experience: None,
                card_networks: None,
                bank_debits: None,
                bank_transfers: Some(api_models::payment_methods::BankTransferTypes {
                    eligible_connectors: connectors,
                }),
                // Required fields for BankTransfer payment method
                required_fields: required_fields_hm
                    .get(&api_enums::PaymentMethod::BankTransfer)
                    .and_then(|inner_hm| inner_hm.get(key.0))
                    .cloned(),
                surcharge_details: None,
                pm_auth_connector: pmt_to_auth_connector
                    .get(&enums::PaymentMethod::BankTransfer)
                    .and_then(|pm_map| pm_map.get(key.0))
                    .cloned(),
            }
        })
    }

    if !bank_transfer_payment_method_types.is_empty() {
        payment_method_responses.push(ResponsePaymentMethodsEnabled {
            payment_method: api_enums::PaymentMethod::BankTransfer,
            payment_method_types: bank_transfer_payment_method_types,
        });
    }
    let currency = payment_intent.as_ref().and_then(|pi| pi.currency);
    let skip_external_tax_calculation = payment_intent
        .as_ref()
        .and_then(|intent| intent.skip_external_tax_calculation)
        .unwrap_or(false);
    let request_external_three_ds_authentication = payment_intent
        .as_ref()
        .and_then(|intent| intent.request_external_three_ds_authentication)
        .unwrap_or(false);
    let merchant_surcharge_configs =
        if let Some((payment_attempt, payment_intent, business_profile)) = payment_attempt
            .as_ref()
            .zip(payment_intent)
            .zip(business_profile.as_ref())
            .map(|((pa, pi), bp)| (pa, pi, bp))
        {
            Box::pin(call_surcharge_decision_management(
                state,
                &merchant_account,
                &key_store,
                business_profile,
                payment_attempt,
                payment_intent,
                billing_address,
                &mut payment_method_responses,
            ))
            .await?
        } else {
            api_surcharge_decision_configs::MerchantSurchargeConfigs::default()
        };

    let collect_shipping_details_from_wallets =
        business_profile.as_ref().and_then(|business_profile| {
            if business_profile
                .always_collect_shipping_details_from_wallet_connector
                .unwrap_or(false)
            {
                business_profile.always_collect_shipping_details_from_wallet_connector
            } else {
                business_profile.collect_shipping_details_from_wallet_connector
            }
        });

    let collect_billing_details_from_wallets =
        business_profile.as_ref().and_then(|business_profile| {
            if business_profile
                .always_collect_billing_details_from_wallet_connector
                .unwrap_or(false)
            {
                business_profile.always_collect_billing_details_from_wallet_connector
            } else {
                business_profile.collect_billing_details_from_wallet_connector
            }
        });

    let is_tax_connector_enabled = business_profile
        .as_ref()
        .is_some_and(|business_profile| business_profile.get_is_tax_connector_enabled());

    Ok(services::ApplicationResponse::Json(
        api::PaymentMethodListResponse {
            redirect_url: business_profile
                .as_ref()
                .and_then(|business_profile| business_profile.return_url.clone()),
            merchant_name: merchant_account.merchant_name,
            payment_type,
            payment_methods: payment_method_responses,
            mandate_payment: payment_attempt.and_then(|inner| inner.mandate_details).map(
                |d| match d {
                    hyperswitch_domain_models::mandates::MandateDataType::SingleUse(i) => {
                        api::MandateType::SingleUse(api::MandateAmountData {
                            amount: i.amount,
                            currency: i.currency,
                            start_date: i.start_date,
                            end_date: i.end_date,
                            metadata: i.metadata,
                        })
                    }
                    hyperswitch_domain_models::mandates::MandateDataType::MultiUse(Some(i)) => {
                        api::MandateType::MultiUse(Some(api::MandateAmountData {
                            amount: i.amount,
                            currency: i.currency,
                            start_date: i.start_date,
                            end_date: i.end_date,
                            metadata: i.metadata,
                        }))
                    }
                    hyperswitch_domain_models::mandates::MandateDataType::MultiUse(None) => {
                        api::MandateType::MultiUse(None)
                    }
                },
            ),
            show_surcharge_breakup_screen: merchant_surcharge_configs
                .show_surcharge_breakup_screen
                .unwrap_or_default(),
            currency,
            request_external_three_ds_authentication,
            collect_shipping_details_from_wallets,
            collect_billing_details_from_wallets,
            is_tax_calculation_enabled: is_tax_connector_enabled && !skip_external_tax_calculation,
        },
    ))
}

fn should_collect_shipping_or_billing_details_from_wallet_connector(
    payment_method: api_enums::PaymentMethod,
    payment_experience_optional: Option<&api_enums::PaymentExperience>,
    business_profile: Option<&Profile>,
    mut required_fields_hs: HashMap<String, RequiredFieldInfo>,
) -> HashMap<String, RequiredFieldInfo> {
    match (payment_method, payment_experience_optional) {
        (api_enums::PaymentMethod::Wallet, Some(api_enums::PaymentExperience::InvokeSdkClient))
        | (
            api_enums::PaymentMethod::PayLater,
            Some(api_enums::PaymentExperience::InvokeSdkClient),
        ) => {
            let always_send_billing_details = business_profile.and_then(|business_profile| {
                business_profile.always_collect_billing_details_from_wallet_connector
            });

            let always_send_shipping_details = business_profile.and_then(|business_profile| {
                business_profile.always_collect_shipping_details_from_wallet_connector
            });

            if always_send_billing_details == Some(true) {
                let billing_details = get_billing_required_fields();
                required_fields_hs.extend(billing_details)
            };
            if always_send_shipping_details == Some(true) {
                let shipping_details = get_shipping_required_fields();
                required_fields_hs.extend(shipping_details)
            };

            required_fields_hs
        }
        _ => required_fields_hs,
    }
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2"),
))]
async fn validate_payment_method_and_client_secret(
    state: &routes::SessionState,
    cs: &String,
    db: &dyn db::StorageInterface,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
) -> Result<(), error_stack::Report<errors::ApiErrorResponse>> {
    let pm_vec = cs.split("_secret").collect::<Vec<&str>>();
    let pm_id = pm_vec
        .first()
        .ok_or(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "client_secret",
        })?;

    let payment_method = db
        .find_payment_method(
            &(state.into()),
            key_store,
            pm_id,
            merchant_account.storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::PaymentMethodNotFound)
        .attach_printable("Unable to find payment method")?;

    let client_secret_expired =
        authenticate_pm_client_secret_and_check_expiry(cs, &payment_method)?;
    if client_secret_expired {
        return Err::<(), error_stack::Report<errors::ApiErrorResponse>>(
            (errors::ApiErrorResponse::ClientSecretExpired).into(),
        );
    }
    Ok(())
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
pub async fn call_surcharge_decision_management(
    state: routes::SessionState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    business_profile: &Profile,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: storage::PaymentIntent,
    billing_address: Option<domain::Address>,
    response_payment_method_types: &mut [ResponsePaymentMethodsEnabled],
) -> errors::RouterResult<api_surcharge_decision_configs::MerchantSurchargeConfigs> {
    #[cfg(feature = "v1")]
    let algorithm_ref: routing_types::RoutingAlgorithmRef = merchant_account
        .routing_algorithm
        .clone()
        .map(|val| val.parse_value("routing algorithm"))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Could not decode the routing algorithm")?
        .unwrap_or_default();

    // TODO: Move to business profile surcharge decision column
    #[cfg(feature = "v2")]
    let algorithm_ref: routing_types::RoutingAlgorithmRef = todo!();

    let (surcharge_results, merchant_sucharge_configs) =
        perform_surcharge_decision_management_for_payment_method_list(
            &state,
            algorithm_ref,
            payment_attempt,
            &payment_intent,
            billing_address.as_ref().map(Into::into),
            response_payment_method_types,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("error performing surcharge decision operation")?;
    if !surcharge_results.is_empty_result() {
        surcharge_results
            .persist_individual_surcharge_details_in_redis(&state, business_profile)
            .await?;
        let _ = state
            .store
            .update_payment_intent(
                &(&state).into(),
                payment_intent,
                storage::PaymentIntentUpdate::SurchargeApplicableUpdate {
                    surcharge_applicable: true,
                    updated_by: merchant_account.storage_scheme.to_string(),
                },
                key_store,
                merchant_account.storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            .attach_printable("Failed to update surcharge_applicable in Payment Intent");
    }
    Ok(merchant_sucharge_configs)
}

#[cfg(feature = "v1")]
pub async fn call_surcharge_decision_management_for_saved_card(
    state: &routes::SessionState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    business_profile: &Profile,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: storage::PaymentIntent,
    customer_payment_method_response: &mut api::CustomerPaymentMethodsListResponse,
) -> errors::RouterResult<()> {
    #[cfg(feature = "v1")]
    let algorithm_ref: routing_types::RoutingAlgorithmRef = merchant_account
        .routing_algorithm
        .clone()
        .map(|val| val.parse_value("routing algorithm"))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Could not decode the routing algorithm")?
        .unwrap_or_default();
    #[cfg(feature = "v2")]
    let algorithm_ref: routing_types::RoutingAlgorithmRef = todo!();

    // TODO: Move to business profile surcharge column
    let surcharge_results = perform_surcharge_decision_management_for_saved_cards(
        state,
        algorithm_ref,
        payment_attempt,
        &payment_intent,
        &mut customer_payment_method_response.customer_payment_methods,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("error performing surcharge decision operation")?;
    if !surcharge_results.is_empty_result() {
        surcharge_results
            .persist_individual_surcharge_details_in_redis(state, business_profile)
            .await?;
        let _ = state
            .store
            .update_payment_intent(
                &state.into(),
                payment_intent,
                storage::PaymentIntentUpdate::SurchargeApplicableUpdate {
                    surcharge_applicable: true,
                    updated_by: merchant_account.storage_scheme.to_string(),
                },
                key_store,
                merchant_account.storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            .attach_printable("Failed to update surcharge_applicable in Payment Intent");
    }
    Ok(())
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
#[allow(clippy::too_many_arguments)]
pub async fn filter_payment_methods(
    graph: &cgraph::ConstraintGraph<dir::DirValue>,
    mca_id: id_type::MerchantConnectorAccountId,
    payment_methods: &[Secret<serde_json::Value>],
    req: &mut api::PaymentMethodListRequest,
    resp: &mut Vec<ResponsePaymentMethodIntermediate>,
    payment_intent: Option<&storage::PaymentIntent>,
    payment_attempt: Option<&storage::PaymentAttempt>,
    address: Option<&domain::Address>,
    connector: String,
    saved_payment_methods: &settings::EligiblePaymentMethods,
) -> errors::CustomResult<(), errors::ApiErrorResponse> {
    for payment_method in payment_methods.iter() {
        let parse_result = serde_json::from_value::<PaymentMethodsEnabled>(
            payment_method.clone().expose().clone(),
        );
        if let Ok(payment_methods_enabled) = parse_result {
            let payment_method = payment_methods_enabled.payment_method;

            let allowed_payment_method_types = payment_intent.and_then(|payment_intent| {
                payment_intent
                    .allowed_payment_method_types
                    .clone()
                    .map(|val| val.parse_value("Vec<PaymentMethodType>"))
                    .transpose()
                    .unwrap_or_else(|error| {
                        logger::error!(
                            ?error,
                            "Failed to deserialize PaymentIntent allowed_payment_method_types"
                        );
                        None
                    })
            });

            for payment_method_type_info in payment_methods_enabled
                .payment_method_types
                .unwrap_or_default()
            {
                if filter_recurring_based(&payment_method_type_info, req.recurring_enabled)
                    && filter_installment_based(
                        &payment_method_type_info,
                        req.installment_payment_enabled,
                    )
                    && filter_amount_based(&payment_method_type_info, req.amount)
                {
                    let payment_method_object = payment_method_type_info.clone();

                    let pm_dir_value: dir::DirValue =
                        (payment_method_type_info.payment_method_type, payment_method)
                            .into_dir_value()
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("pm_value_node not created")?;

                    let connector_variant = api_enums::Connector::from_str(connector.as_str())
                        .change_context(errors::ConnectorError::InvalidConnectorName)
                        .change_context(errors::ApiErrorResponse::InvalidDataValue {
                            field_name: "connector",
                        })
                        .attach_printable_lazy(|| {
                            format!("unable to parse connector name {connector:?}")
                        })?;

                    let mut context_values: Vec<dir::DirValue> = Vec::new();
                    context_values.push(pm_dir_value.clone());

                    payment_intent.map(|intent| {
                        intent.currency.map(|currency| {
                            context_values.push(dir::DirValue::PaymentCurrency(currency))
                        })
                    });
                    address.map(|address| {
                        address.country.map(|country| {
                            context_values.push(dir::DirValue::BillingCountry(
                                common_enums::Country::from_alpha2(country),
                            ))
                        })
                    });

                    // Addition of Connector to context
                    if let Ok(connector) = api_enums::RoutableConnectors::from_str(
                        connector_variant.to_string().as_str(),
                    ) {
                        context_values.push(dir::DirValue::Connector(Box::new(
                            api_models::routing::ast::ConnectorChoice { connector },
                        )));
                    };

                    let filter_pm_based_on_allowed_types = filter_pm_based_on_allowed_types(
                        allowed_payment_method_types.as_ref(),
                        payment_method_object.payment_method_type,
                    );

                    if payment_attempt
                        .and_then(|attempt| attempt.mandate_details.as_ref())
                        .is_some()
                        || payment_intent
                            .and_then(|intent| intent.setup_future_usage)
                            .map(|future_usage| {
                                future_usage == common_enums::FutureUsage::OffSession
                            })
                            .unwrap_or(false)
                    {
                        context_values.push(dir::DirValue::PaymentType(
                            euclid::enums::PaymentType::NewMandate,
                        ));
                    };

                    payment_attempt
                        .and_then(|attempt| attempt.mandate_data.as_ref())
                        .map(|mandate_detail| {
                            if mandate_detail.update_mandate_id.is_some() {
                                context_values.push(dir::DirValue::PaymentType(
                                    euclid::enums::PaymentType::UpdateMandate,
                                ));
                            }
                        });

                    payment_attempt
                        .map(|attempt| {
                            attempt.mandate_data.is_none()
                                && attempt.mandate_details.is_none()
                                && payment_intent
                                    .and_then(|intent| intent.setup_future_usage)
                                    .map(|future_usage| {
                                        future_usage == common_enums::FutureUsage::OnSession
                                    })
                                    .unwrap_or(true)
                        })
                        .and_then(|res| {
                            res.then(|| {
                                context_values.push(dir::DirValue::PaymentType(
                                    euclid::enums::PaymentType::NonMandate,
                                ))
                            })
                        });

                    payment_attempt
                        .and_then(|inner| inner.capture_method)
                        .map(|capture_method| {
                            context_values.push(dir::DirValue::CaptureMethod(capture_method));
                        });

                    let filter_pm_card_network_based = filter_pm_card_network_based(
                        payment_method_object.card_networks.as_ref(),
                        req.card_networks.as_ref(),
                        payment_method_object.payment_method_type,
                    );

                    let saved_payment_methods_filter = req
                        .client_secret
                        .as_ref()
                        .map(|cs| {
                            if cs.starts_with("pm_") {
                                saved_payment_methods
                                    .sdk_eligible_payment_methods
                                    .contains(payment_method.to_string().as_str())
                            } else {
                                true
                            }
                        })
                        .unwrap_or(true);

                    let context = AnalysisContext::from_dir_values(context_values.clone());
                    logger::info!("Context created for List Payment method is {:?}", context);

                    let domain_ident: &[String] = &[mca_id.clone().get_string_repr().to_string()];
                    let result = graph.key_value_analysis(
                        pm_dir_value.clone(),
                        &context,
                        &mut cgraph::Memoization::new(),
                        &mut cgraph::CycleCheck::new(),
                        Some(domain_ident),
                    );
                    if let Err(ref e) = result {
                        logger::error!(
                            "Error while performing Constraint graph's key value analysis
                            for list payment methods {:?}",
                            e
                        );
                    } else if filter_pm_based_on_allowed_types
                        && filter_pm_card_network_based
                        && saved_payment_methods_filter
                        && matches!(result, Ok(()))
                    {
                        let response_pm_type = ResponsePaymentMethodIntermediate::new(
                            payment_method_object,
                            connector.clone(),
                            mca_id.get_string_repr().to_string(),
                            payment_method,
                        );
                        resp.push(response_pm_type);
                    } else {
                        logger::error!("Filtering Payment Methods Failed");
                    }
                }
            }
        }
    }
    Ok(())
}

// v2 type for PaymentMethodListRequest will not have the installment_payment_enabled field,
// need to re-evaluate filter logic
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[allow(clippy::too_many_arguments)]
pub async fn filter_payment_methods(
    _graph: &cgraph::ConstraintGraph<dir::DirValue>,
    _mca_id: String,
    _payment_methods: &[Secret<serde_json::Value>],
    _req: &mut api::PaymentMethodListRequest,
    _resp: &mut [ResponsePaymentMethodIntermediate],
    _payment_intent: Option<&storage::PaymentIntent>,
    _payment_attempt: Option<&storage::PaymentAttempt>,
    _address: Option<&domain::Address>,
    _connector: String,
    _saved_payment_methods: &settings::EligiblePaymentMethods,
) -> errors::CustomResult<(), errors::ApiErrorResponse> {
    todo!()
}

fn filter_amount_based(
    payment_method: &RequestPaymentMethodTypes,
    amount: Option<MinorUnit>,
) -> bool {
    let min_check = amount
        .and_then(|amt| payment_method.minimum_amount.map(|min_amt| amt >= min_amt))
        .unwrap_or(true);
    let max_check = amount
        .and_then(|amt| payment_method.maximum_amount.map(|max_amt| amt <= max_amt))
        .unwrap_or(true);
    (min_check && max_check) || amount == Some(MinorUnit::zero())
}

fn filter_installment_based(
    payment_method: &RequestPaymentMethodTypes,
    installment_payment_enabled: Option<bool>,
) -> bool {
    installment_payment_enabled.map_or(true, |enabled| {
        payment_method.installment_payment_enabled == enabled
    })
}

fn filter_pm_card_network_based(
    pm_card_networks: Option<&Vec<api_enums::CardNetwork>>,
    request_card_networks: Option<&Vec<api_enums::CardNetwork>>,
    pm_type: api_enums::PaymentMethodType,
) -> bool {
    match pm_type {
        api_enums::PaymentMethodType::Credit | api_enums::PaymentMethodType::Debit => {
            match (pm_card_networks, request_card_networks) {
                (Some(pm_card_networks), Some(request_card_networks)) => request_card_networks
                    .iter()
                    .all(|card_network| pm_card_networks.contains(card_network)),
                (None, Some(_)) => false,
                _ => true,
            }
        }
        _ => true,
    }
}

fn filter_pm_based_on_allowed_types(
    allowed_types: Option<&Vec<api_enums::PaymentMethodType>>,
    payment_method_type: api_enums::PaymentMethodType,
) -> bool {
    allowed_types.map_or(true, |pm| pm.contains(&payment_method_type))
}

fn filter_recurring_based(
    payment_method: &RequestPaymentMethodTypes,
    recurring_enabled: Option<bool>,
) -> bool {
    recurring_enabled.map_or(true, |enabled| payment_method.recurring_enabled == enabled)
}

#[cfg(all(
    any(feature = "v2", feature = "v1"),
    not(feature = "payment_methods_v2"),
    not(feature = "customer_v2")
))]
pub async fn do_list_customer_pm_fetch_customer_if_not_passed(
    state: routes::SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: Option<api::PaymentMethodListRequest>,
    customer_id: Option<&id_type::CustomerId>,
    ephemeral_api_key: Option<&str>,
) -> errors::RouterResponse<api::CustomerPaymentMethodsListResponse> {
    let limit = req.clone().and_then(|pml_req| pml_req.limit);

    let auth_cust = if let Some(key) = ephemeral_api_key {
        let key = state
            .store()
            .get_ephemeral_key(key)
            .await
            .change_context(errors::ApiErrorResponse::Unauthorized)?;

        Some(key.customer_id.clone())
    } else {
        None
    };

    let customer_id = customer_id.or(auth_cust.as_ref());

    if let Some(customer_id) = customer_id {
        Box::pin(list_customer_payment_method(
            &state,
            merchant_account,
            key_store,
            None,
            customer_id,
            limit,
        ))
        .await
    } else {
        let cloned_secret = req.and_then(|r| r.client_secret.as_ref().cloned());
        let payment_intent: Option<hyperswitch_domain_models::payments::PaymentIntent> =
            helpers::verify_payment_intent_time_and_client_secret(
                &state,
                &merchant_account,
                &key_store,
                cloned_secret,
            )
            .await?;

        match payment_intent
            .as_ref()
            .and_then(|intent| intent.customer_id.to_owned())
        {
            Some(customer_id) => {
                Box::pin(list_customer_payment_method(
                    &state,
                    merchant_account,
                    key_store,
                    payment_intent,
                    &customer_id,
                    limit,
                ))
                .await
            }
            None => {
                let response = api::CustomerPaymentMethodsListResponse {
                    customer_payment_methods: Vec::new(),
                    is_guest_customer: Some(true),
                };
                Ok(services::ApplicationResponse::Json(response))
            }
        }
    }
}

#[cfg(all(
    any(feature = "v2", feature = "v1"),
    not(feature = "payment_methods_v2"),
    not(feature = "customer_v2")
))]
pub async fn list_customer_payment_method(
    state: &routes::SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    payment_intent: Option<storage::PaymentIntent>,
    customer_id: &id_type::CustomerId,
    limit: Option<i64>,
) -> errors::RouterResponse<api::CustomerPaymentMethodsListResponse> {
    let db = &*state.store;
    let key_manager_state = &state.into();
    let off_session_payment_flag = payment_intent
        .as_ref()
        .map(|pi| {
            matches!(
                pi.setup_future_usage,
                Some(common_enums::FutureUsage::OffSession)
            )
        })
        .unwrap_or(false);

    let customer = db
        .find_customer_by_customer_id_merchant_id(
            &state.into(),
            customer_id,
            merchant_account.get_id(),
            &key_store,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)?;

    let is_requires_cvv = db
        .find_config_by_key_unwrap_or(
            &merchant_account.get_id().get_requires_cvv_key(),
            Some("true".to_string()),
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch requires_cvv config")?;

    let requires_cvv = is_requires_cvv.config != "false";

    let resp = db
        .find_payment_method_by_customer_id_merchant_id_status(
            &(state.into()),
            &key_store,
            customer_id,
            merchant_account.get_id(),
            common_enums::PaymentMethodStatus::Active,
            limit,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;
    let mut customer_pms = Vec::new();

    let profile_id = payment_intent
        .as_ref()
        .map(|payment_intent| {
            payment_intent
                .profile_id
                .clone()
                .get_required_value("profile_id")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("profile_id is not set in payment_intent")
        })
        .transpose()?;

    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        key_manager_state,
        &key_store,
        profile_id.as_ref(),
        merchant_account.get_id(),
    )
    .await?;

    let is_connector_agnostic_mit_enabled = business_profile
        .as_ref()
        .and_then(|business_profile| business_profile.is_connector_agnostic_mit_enabled)
        .unwrap_or(false);

    for pm in resp.into_iter() {
        let parent_payment_method_token = generate_id(consts::ID_LENGTH, "token");

        let payment_method = pm
            .get_payment_method_type()
            .get_required_value("payment_method")?;

        let pm_list_context = get_pm_list_context(
            state,
            &payment_method,
            &key_store,
            &pm,
            Some(parent_payment_method_token.clone()),
            true,
        )
        .await?;

        if pm_list_context.is_none() {
            continue;
        }

        let pm_list_context = pm_list_context.get_required_value("PaymentMethodListContext")?;

        // Retrieve the masked bank details to be sent as a response
        let bank_details = if payment_method == enums::PaymentMethod::BankDebit {
            get_masked_bank_details(&pm).await.unwrap_or_else(|error| {
                logger::error!(?error);
                None
            })
        } else {
            None
        };

        let payment_method_billing = pm
            .payment_method_billing_address
            .clone()
            .map(|decrypted_data| decrypted_data.into_inner().expose())
            .map(|decrypted_value| decrypted_value.parse_value("payment method billing address"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("unable to decrypt payment method billing address details")?;
        let connector_mandate_details = pm
            .get_common_mandate_reference()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to deserialize to Payment Mandate Reference ")?;
        let mca_enabled = get_mca_status(
            state,
            &key_store,
            profile_id.clone(),
            merchant_account.get_id(),
            is_connector_agnostic_mit_enabled,
            Some(connector_mandate_details),
            pm.network_transaction_id.as_ref(),
        )
        .await?;

        let requires_cvv = if is_connector_agnostic_mit_enabled {
            requires_cvv
                && !(off_session_payment_flag
                    && (pm.connector_mandate_details.is_some()
                        || pm.network_transaction_id.is_some()))
        } else {
            requires_cvv && !(off_session_payment_flag && pm.connector_mandate_details.is_some())
        };
        // Need validation for enabled payment method ,querying MCA
        let pma = api::CustomerPaymentMethod {
            payment_token: parent_payment_method_token.to_owned(),
            payment_method_id: pm.payment_method_id.clone(),
            customer_id: pm.customer_id.clone(),
            payment_method,
            payment_method_type: pm.get_payment_method_subtype(),
            payment_method_issuer: pm.payment_method_issuer,
            card: pm_list_context.card_details,
            metadata: pm.metadata,
            payment_method_issuer_code: pm.payment_method_issuer_code,
            recurring_enabled: mca_enabled,
            installment_payment_enabled: false,
            payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]),
            created: Some(pm.created_at),
            #[cfg(feature = "payouts")]
            bank_transfer: pm_list_context.bank_transfer_details,
            bank: bank_details,
            surcharge_details: None,
            requires_cvv,
            last_used_at: Some(pm.last_used_at),
            default_payment_method_set: customer.default_payment_method_id.is_some()
                && customer.default_payment_method_id == Some(pm.payment_method_id),
            billing: payment_method_billing,
        };
        if requires_cvv || mca_enabled {
            customer_pms.push(pma.to_owned());
        }

        let redis_conn = state
            .store
            .get_redis_conn()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to get redis connection")?;

        let intent_fulfillment_time = business_profile
            .as_ref()
            .and_then(|b_profile| b_profile.get_order_fulfillment_time())
            .unwrap_or(consts::DEFAULT_INTENT_FULFILLMENT_TIME);

        let hyperswitch_token_data = pm_list_context
            .hyperswitch_token_data
            .get_required_value("PaymentTokenData")?;

        ParentPaymentMethodToken::create_key_for_token((
            &parent_payment_method_token,
            pma.payment_method,
        ))
        .insert(intent_fulfillment_time, hyperswitch_token_data, state)
        .await?;

        if let Some(metadata) = pma.metadata {
            let pm_metadata_vec: payment_methods::PaymentMethodMetadata = metadata
                .parse_value("PaymentMethodMetadata")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "Failed to deserialize metadata to PaymentmethodMetadata struct",
                )?;

            for pm_metadata in pm_metadata_vec.payment_method_tokenization {
                let key = format!(
                    "pm_token_{}_{}_{}",
                    parent_payment_method_token, pma.payment_method, pm_metadata.0
                );

                redis_conn
                    .set_key_with_expiry(&key.into(), pm_metadata.1, intent_fulfillment_time)
                    .await
                    .change_context(errors::StorageError::KVError)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to add data in redis")?;
            }
        }
    }

    let mut response = api::CustomerPaymentMethodsListResponse {
        customer_payment_methods: customer_pms,
        is_guest_customer: payment_intent.as_ref().map(|_| false), //to return this key only when the request is tied to a payment intent
    };

    Box::pin(perform_surcharge_ops(
        payment_intent,
        state,
        merchant_account,
        key_store,
        business_profile,
        &mut response,
    ))
    .await?;

    Ok(services::ApplicationResponse::Json(response))
}

#[cfg(all(
    any(feature = "v2", feature = "v1"),
    not(feature = "payment_methods_v2"),
    not(feature = "customer_v2")
))]
pub async fn get_pm_list_context(
    state: &routes::SessionState,
    payment_method: &enums::PaymentMethod,
    #[cfg(feature = "payouts")] key_store: &domain::MerchantKeyStore,
    #[cfg(not(feature = "payouts"))] _key_store: &domain::MerchantKeyStore,
    pm: &domain::PaymentMethod,
    #[cfg(feature = "payouts")] parent_payment_method_token: Option<String>,
    #[cfg(not(feature = "payouts"))] _parent_payment_method_token: Option<String>,
    is_payment_associated: bool,
) -> Result<Option<PaymentMethodListContext>, error_stack::Report<errors::ApiErrorResponse>> {
    let payment_method_retrieval_context = match payment_method {
        enums::PaymentMethod::Card => {
            let card_details = get_card_details_with_locker_fallback(pm, state).await?;

            card_details.as_ref().map(|card| PaymentMethodListContext {
                card_details: Some(card.clone()),
                #[cfg(feature = "payouts")]
                bank_transfer_details: None,
                hyperswitch_token_data: is_payment_associated.then_some(
                    PaymentTokenData::permanent_card(
                        Some(pm.get_id().clone()),
                        pm.locker_id.clone().or(Some(pm.get_id().clone())),
                        pm.locker_id.clone().unwrap_or(pm.get_id().clone()),
                        pm.network_token_requestor_reference_id
                            .clone()
                            .or(Some(pm.get_id().clone())),
                    ),
                ),
            })
        }

        enums::PaymentMethod::BankDebit => {
            // Retrieve the pm_auth connector details so that it can be tokenized
            let bank_account_token_data = get_bank_account_connector_details(pm)
                .await
                .unwrap_or_else(|err| {
                    logger::error!(error=?err);
                    None
                });

            bank_account_token_data.map(|data| {
                let token_data = PaymentTokenData::AuthBankDebit(data);

                PaymentMethodListContext {
                    card_details: None,
                    #[cfg(feature = "payouts")]
                    bank_transfer_details: None,
                    hyperswitch_token_data: is_payment_associated.then_some(token_data),
                }
            })
        }

        enums::PaymentMethod::Wallet => Some(PaymentMethodListContext {
            card_details: None,
            #[cfg(feature = "payouts")]
            bank_transfer_details: None,
            hyperswitch_token_data: is_payment_associated
                .then_some(PaymentTokenData::wallet_token(pm.get_id().clone())),
        }),

        #[cfg(feature = "payouts")]
        enums::PaymentMethod::BankTransfer => Some(PaymentMethodListContext {
            card_details: None,
            bank_transfer_details: Some(
                get_bank_from_hs_locker(
                    state,
                    key_store,
                    parent_payment_method_token.as_ref(),
                    &pm.customer_id,
                    &pm.merchant_id,
                    pm.locker_id.as_ref().unwrap_or(pm.get_id()),
                )
                .await?,
            ),
            hyperswitch_token_data: parent_payment_method_token
                .map(|token| PaymentTokenData::temporary_generic(token.clone())),
        }),

        _ => Some(PaymentMethodListContext {
            card_details: None,
            #[cfg(feature = "payouts")]
            bank_transfer_details: None,
            hyperswitch_token_data: is_payment_associated.then_some(
                PaymentTokenData::temporary_generic(generate_id(consts::ID_LENGTH, "token")),
            ),
        }),
    };

    Ok(payment_method_retrieval_context)
}

#[cfg(feature = "v1")]
async fn perform_surcharge_ops(
    payment_intent: Option<storage::PaymentIntent>,
    state: &routes::SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    business_profile: Option<Profile>,
    response: &mut api::CustomerPaymentMethodsListResponse,
) -> Result<(), error_stack::Report<errors::ApiErrorResponse>> {
    let payment_attempt = payment_intent
        .as_ref()
        .async_map(|payment_intent| async {
            state
                .store
                .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                    payment_intent.get_id(),
                    merchant_account.get_id(),
                    &payment_intent.active_attempt.get_id(),
                    merchant_account.storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
        })
        .await
        .transpose()?;
    if let Some((payment_attempt, payment_intent, business_profile)) = payment_attempt
        .zip(payment_intent)
        .zip(business_profile)
        .map(|((pa, pi), bp)| (pa, pi, bp))
    {
        call_surcharge_decision_management_for_saved_card(
            state,
            &merchant_account,
            &key_store,
            &business_profile,
            &payment_attempt,
            payment_intent,
            response,
        )
        .await?;
    }

    Ok(())
}

#[cfg(feature = "v2")]
pub async fn perform_surcharge_ops(
    _payment_intent: Option<storage::PaymentIntent>,
    _state: &routes::SessionState,
    _merchant_account: &domain::MerchantAccount,
    _key_store: domain::MerchantKeyStore,
    _business_profile: Option<Profile>,
    _response: &mut api::CustomerPaymentMethodsListResponse,
) -> Result<(), error_stack::Report<errors::ApiErrorResponse>> {
    todo!()
}

#[cfg(feature = "v1")]
pub async fn get_mca_status(
    state: &routes::SessionState,
    key_store: &domain::MerchantKeyStore,
    profile_id: Option<id_type::ProfileId>,
    merchant_id: &id_type::MerchantId,
    is_connector_agnostic_mit_enabled: bool,
    connector_mandate_details: Option<CommonMandateReference>,
    network_transaction_id: Option<&String>,
) -> errors::RouterResult<bool> {
    if is_connector_agnostic_mit_enabled && network_transaction_id.is_some() {
        return Ok(true);
    }
    if let Some(connector_mandate_details) = connector_mandate_details {
        let mcas = state
            .store
            .find_merchant_connector_account_by_merchant_id_and_disabled_list(
                &state.into(),
                merchant_id,
                true,
                key_store,
            )
            .await
            .change_context(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                id: merchant_id.get_string_repr().to_owned(),
            })?;
        let merchant_connector_accounts = domain::MerchantConnectorAccounts::new(mcas);

        return Ok(merchant_connector_accounts
            .is_merchant_connector_account_id_in_connector_mandate_details(
                profile_id.as_ref(),
                &connector_mandate_details,
            ));
    }
    Ok(false)
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[allow(clippy::too_many_arguments)]
pub async fn get_mca_status(
    state: &routes::SessionState,
    key_store: &domain::MerchantKeyStore,
    profile_id: Option<id_type::ProfileId>,
    merchant_id: &id_type::MerchantId,
    is_connector_agnostic_mit_enabled: bool,
    connector_mandate_details: Option<&CommonMandateReference>,
    network_transaction_id: Option<&String>,
    merchant_connector_accounts: &domain::MerchantConnectorAccounts,
) -> bool {
    if is_connector_agnostic_mit_enabled && network_transaction_id.is_some() {
        return true;
    }
    match connector_mandate_details {
        Some(connector_mandate_details) => merchant_connector_accounts
            .is_merchant_connector_account_id_in_connector_mandate_details(
                profile_id.as_ref(),
                connector_mandate_details,
            ),
        None => false,
    }
}

pub async fn decrypt_generic_data<T>(
    state: &routes::SessionState,
    data: Option<Encryption>,
    key_store: &domain::MerchantKeyStore,
) -> errors::RouterResult<Option<T>>
where
    T: serde::de::DeserializeOwned,
{
    let key = key_store.key.get_inner().peek();
    let identifier = Identifier::Merchant(key_store.merchant_id.clone());
    let decrypted_data = domain::types::crypto_operation::<serde_json::Value, masking::WithType>(
        &state.into(),
        type_name!(T),
        domain::types::CryptoOperation::DecryptOptional(data),
        identifier,
        key,
    )
    .await
    .and_then(|val| val.try_into_optionaloperation())
    .change_context(errors::StorageError::DecryptionError)
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("unable to decrypt data")?;

    decrypted_data
        .map(|decrypted_data| decrypted_data.into_inner().expose())
        .map(|decrypted_value| decrypted_value.parse_value("generic_data"))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("unable to parse generic data value")
}

#[cfg(all(
    any(feature = "v2", feature = "v1"),
    not(feature = "payment_methods_v2")
))]
pub async fn get_card_details_with_locker_fallback(
    pm: &domain::PaymentMethod,
    state: &routes::SessionState,
) -> errors::RouterResult<Option<api::CardDetailFromLocker>> {
    let card_decrypted = pm
        .payment_method_data
        .clone()
        .map(|x| x.into_inner().expose())
        .and_then(|v| serde_json::from_value::<PaymentMethodsData>(v).ok())
        .and_then(|pmd| match pmd {
            PaymentMethodsData::Card(crd) => Some(api::CardDetailFromLocker::from(crd)),
            _ => None,
        });

    Ok(if let Some(mut crd) = card_decrypted {
        crd.scheme.clone_from(&pm.scheme);
        Some(crd)
    } else {
        logger::debug!(
            "Getting card details from locker as it is not found in payment methods table"
        );
        Some(get_card_details_from_locker(state, pm).await?)
    })
}

#[cfg(all(
    any(feature = "v2", feature = "v1"),
    not(feature = "payment_methods_v2")
))]
pub async fn get_card_details_without_locker_fallback(
    pm: &domain::PaymentMethod,
    state: &routes::SessionState,
) -> errors::RouterResult<api::CardDetailFromLocker> {
    let card_decrypted = pm
        .payment_method_data
        .clone()
        .map(|x| x.into_inner().expose())
        .and_then(|v| serde_json::from_value::<PaymentMethodsData>(v).ok())
        .and_then(|pmd| match pmd {
            PaymentMethodsData::Card(crd) => Some(api::CardDetailFromLocker::from(crd)),
            _ => None,
        });

    Ok(if let Some(mut crd) = card_decrypted {
        crd.scheme.clone_from(&pm.scheme);
        crd
    } else {
        logger::debug!(
            "Getting card details from locker as it is not found in payment methods table"
        );
        get_card_details_from_locker(state, pm).await?
    })
}

#[cfg(all(
    any(feature = "v2", feature = "v1"),
    not(feature = "payment_methods_v2")
))]
pub async fn get_card_details_from_locker(
    state: &routes::SessionState,
    pm: &domain::PaymentMethod,
) -> errors::RouterResult<api::CardDetailFromLocker> {
    let card = get_card_from_locker(
        state,
        &pm.customer_id,
        &pm.merchant_id,
        pm.locker_id.as_ref().unwrap_or(pm.get_id()),
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Error getting card from card vault")?;

    payment_methods::get_card_detail(pm, card)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Get Card Details Failed")
}

#[cfg(all(
    any(feature = "v2", feature = "v1"),
    not(feature = "payment_methods_v2")
))]
pub async fn get_lookup_key_from_locker(
    state: &routes::SessionState,
    payment_token: &str,
    pm: &domain::PaymentMethod,
    merchant_key_store: &domain::MerchantKeyStore,
) -> errors::RouterResult<api::CardDetailFromLocker> {
    let card_detail = get_card_details_from_locker(state, pm).await?;
    let card = card_detail.clone();

    let resp = TempLockerCardSupport::create_payment_method_data_in_temp_locker(
        state,
        payment_token,
        card,
        pm,
        merchant_key_store,
    )
    .await?;
    Ok(resp)
}

pub async fn get_masked_bank_details(
    pm: &domain::PaymentMethod,
) -> errors::RouterResult<Option<MaskedBankDetails>> {
    #[cfg(all(
        any(feature = "v2", feature = "v1"),
        not(feature = "payment_methods_v2")
    ))]
    let payment_method_data = pm
        .payment_method_data
        .clone()
        .map(|x| x.into_inner().expose())
        .map(
            |v| -> Result<PaymentMethodsData, error_stack::Report<errors::ApiErrorResponse>> {
                v.parse_value::<PaymentMethodsData>("PaymentMethodsData")
                    .change_context(errors::StorageError::DeserializationFailed)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to deserialize Payment Method Auth config")
            },
        )
        .transpose()?;

    #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
    let payment_method_data = pm.payment_method_data.clone().map(|x| x.into_inner());

    match payment_method_data {
        Some(pmd) => match pmd {
            PaymentMethodsData::Card(_) => Ok(None),
            PaymentMethodsData::BankDetails(bank_details) => Ok(Some(MaskedBankDetails {
                mask: bank_details.mask,
            })),
            PaymentMethodsData::WalletDetails(_) => Ok(None),
        },
        None => Err(report!(errors::ApiErrorResponse::InternalServerError))
            .attach_printable("Unable to fetch payment method data"),
    }
}

#[cfg(all(
    any(feature = "v2", feature = "v1"),
    not(feature = "payment_methods_v2")
))]
pub async fn get_bank_account_connector_details(
    pm: &domain::PaymentMethod,
) -> errors::RouterResult<Option<BankAccountTokenData>> {
    let payment_method_data = pm
        .payment_method_data
        .clone()
        .map(|x| x.into_inner().expose())
        .map(
            |v| -> Result<PaymentMethodsData, error_stack::Report<errors::ApiErrorResponse>> {
                v.parse_value::<PaymentMethodsData>("PaymentMethodsData")
                    .change_context(errors::StorageError::DeserializationFailed)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to deserialize Payment Method Auth config")
            },
        )
        .transpose()?;

    match payment_method_data {
        Some(pmd) => match pmd {
            PaymentMethodsData::Card(_) => Err(errors::ApiErrorResponse::UnprocessableEntity {
                message: "Card is not a valid entity".to_string(),
            }
            .into()),
            PaymentMethodsData::WalletDetails(_) => {
                Err(errors::ApiErrorResponse::UnprocessableEntity {
                    message: "Wallet is not a valid entity".to_string(),
                }
                .into())
            }
            PaymentMethodsData::BankDetails(bank_details) => {
                let connector_details = bank_details
                    .connector_details
                    .first()
                    .ok_or(errors::ApiErrorResponse::InternalServerError)?;

                let pm_type = pm
                    .get_payment_method_subtype()
                    .get_required_value("payment_method_type")
                    .attach_printable("PaymentMethodType not found")?;

                let pm = pm
                    .get_payment_method_type()
                    .get_required_value("payment_method")
                    .attach_printable("PaymentMethod not found")?;

                let token_data = BankAccountTokenData {
                    payment_method_type: pm_type,
                    payment_method: pm,
                    connector_details: connector_details.clone(),
                };

                Ok(Some(token_data))
            }
        },
        None => Ok(None),
    }
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
pub async fn set_default_payment_method(
    state: &routes::SessionState,
    merchant_id: &id_type::MerchantId,
    key_store: domain::MerchantKeyStore,
    customer_id: &id_type::CustomerId,
    payment_method_id: String,
    storage_scheme: MerchantStorageScheme,
) -> errors::RouterResponse<api_models::payment_methods::CustomerDefaultPaymentMethodResponse> {
    let db = &*state.store;
    let key_manager_state = &state.into();
    // check for the customer
    // TODO: customer need not be checked again here, this function can take an optional customer and check for existence of customer based on the optional value
    let customer = db
        .find_customer_by_customer_id_merchant_id(
            key_manager_state,
            customer_id,
            merchant_id,
            &key_store,
            storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)?;
    // check for the presence of payment_method
    let payment_method = db
        .find_payment_method(
            &(state.into()),
            &key_store,
            &payment_method_id,
            storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;
    let pm = payment_method
        .get_payment_method_type()
        .get_required_value("payment_method")?;

    utils::when(
        &payment_method.customer_id != customer_id || payment_method.merchant_id != *merchant_id,
        || {
            Err(errors::ApiErrorResponse::PreconditionFailed {
                message: "The payment_method_id is not valid".to_string(),
            })
        },
    )?;

    utils::when(
        Some(payment_method_id.clone()) == customer.default_payment_method_id,
        || {
            Err(errors::ApiErrorResponse::PreconditionFailed {
                message: "Payment Method is already set as default".to_string(),
            })
        },
    )?;

    let customer_id = customer.customer_id.clone();

    let customer_update = CustomerUpdate::UpdateDefaultPaymentMethod {
        default_payment_method_id: Some(Some(payment_method_id.to_owned())),
    };
    // update the db with the default payment method id

    let updated_customer_details = db
        .update_customer_by_customer_id_merchant_id(
            key_manager_state,
            customer_id.to_owned(),
            merchant_id.to_owned(),
            customer,
            customer_update,
            &key_store,
            storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to update the default payment method id for the customer")?;

    let resp = api_models::payment_methods::CustomerDefaultPaymentMethodResponse {
        default_payment_method_id: updated_customer_details.default_payment_method_id,
        customer_id,
        payment_method_type: payment_method.get_payment_method_subtype(),
        payment_method: pm,
    };

    Ok(services::ApplicationResponse::Json(resp))
}

pub async fn update_last_used_at(
    payment_method: &domain::PaymentMethod,
    state: &routes::SessionState,
    storage_scheme: MerchantStorageScheme,
    key_store: &domain::MerchantKeyStore,
) -> errors::RouterResult<()> {
    let update_last_used = storage::PaymentMethodUpdate::LastUsedUpdate {
        last_used_at: common_utils::date_time::now(),
    };

    state
        .store
        .update_payment_method(
            &(state.into()),
            key_store,
            payment_method.clone(),
            update_last_used,
            storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to update the last_used_at in db")?;

    Ok(())
}
#[cfg(feature = "payouts")]
pub async fn get_bank_from_hs_locker(
    state: &routes::SessionState,
    key_store: &domain::MerchantKeyStore,
    temp_token: Option<&String>,
    customer_id: &id_type::CustomerId,
    merchant_id: &id_type::MerchantId,
    token_ref: &str,
) -> errors::RouterResult<api::BankPayout> {
    let payment_method = get_payment_method_from_hs_locker(
        state,
        key_store,
        customer_id,
        merchant_id,
        token_ref,
        None,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Error getting payment method from locker")?;
    let pm_parsed: api::PayoutMethodData = payment_method
        .peek()
        .to_string()
        .parse_struct("PayoutMethodData")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    match &pm_parsed {
        api::PayoutMethodData::Bank(bank) => {
            if let Some(token) = temp_token {
                vault::Vault::store_payout_method_data_in_locker(
                    state,
                    Some(token.clone()),
                    &pm_parsed,
                    Some(customer_id.to_owned()),
                    key_store,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error storing payout method data in temporary locker")?;
            }
            Ok(bank.to_owned())
        }
        api::PayoutMethodData::Card(_) => Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "Expected bank details, found card details instead".to_string(),
        }
        .into()),
        api::PayoutMethodData::Wallet(_) => Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "Expected bank details, found wallet details instead".to_string(),
        }
        .into()),
    }
}

#[cfg(all(
    any(feature = "v2", feature = "v1"),
    not(feature = "payment_methods_v2")
))]
pub struct TempLockerCardSupport;

#[cfg(all(
    any(feature = "v2", feature = "v1"),
    not(feature = "payment_methods_v2")
))]
impl TempLockerCardSupport {
    #[instrument(skip_all)]
    async fn create_payment_method_data_in_temp_locker(
        state: &routes::SessionState,
        payment_token: &str,
        card: api::CardDetailFromLocker,
        pm: &domain::PaymentMethod,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> errors::RouterResult<api::CardDetailFromLocker> {
        let card_number = card.card_number.clone().get_required_value("card_number")?;
        let card_exp_month = card
            .expiry_month
            .clone()
            .expose_option()
            .get_required_value("expiry_month")?;
        let card_exp_year = card
            .expiry_year
            .clone()
            .expose_option()
            .get_required_value("expiry_year")?;
        let card_holder_name = card
            .card_holder_name
            .clone()
            .expose_option()
            .unwrap_or_default();
        let value1 = payment_methods::mk_card_value1(
            card_number,
            card_exp_year,
            card_exp_month,
            Some(card_holder_name),
            None,
            None,
            None,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error getting Value1 for locker")?;
        let value2 = payment_methods::mk_card_value2(
            None,
            None,
            None,
            Some(pm.customer_id.clone()),
            Some(pm.get_id().to_string()),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error getting Value2 for locker")?;

        let value1 = vault::VaultPaymentMethod::Card(value1);
        let value2 = vault::VaultPaymentMethod::Card(value2);

        let value1 = value1
            .encode_to_string_of_json()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Wrapped value1 construction failed when saving card to locker")?;

        let value2 = value2
            .encode_to_string_of_json()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Wrapped value2 construction failed when saving card to locker")?;

        let lookup_key = vault::create_tokenize(
            state,
            value1,
            Some(value2),
            payment_token.to_string(),
            merchant_key_store.key.get_inner(),
        )
        .await?;
        vault::add_delete_tokenized_data_task(
            &*state.store,
            &lookup_key,
            enums::PaymentMethod::Card,
        )
        .await?;
        metrics::TOKENIZED_DATA_COUNT.add(1, &[]);
        metrics::TASKS_ADDED_COUNT.add(
            1,
            router_env::metric_attributes!(("flow", "DeleteTokenizeData")),
        );
        Ok(card)
    }
}

#[cfg(all(
    any(feature = "v2", feature = "v1"),
    not(feature = "payment_methods_v2")
))]
#[instrument(skip_all)]
pub async fn retrieve_payment_method(
    state: routes::SessionState,
    pm: api::PaymentMethodId,
    key_store: domain::MerchantKeyStore,
    merchant_account: domain::MerchantAccount,
) -> errors::RouterResponse<api::PaymentMethodResponse> {
    let db = state.store.as_ref();
    let pm = db
        .find_payment_method(
            &((&state).into()),
            &key_store,
            &pm.payment_method_id,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

    let card = if pm.get_payment_method_type() == Some(enums::PaymentMethod::Card) {
        let card_detail = if state.conf.locker.locker_enabled {
            let card = get_card_from_locker(
                &state,
                &pm.customer_id,
                &pm.merchant_id,
                pm.locker_id.as_ref().unwrap_or(&pm.payment_method_id),
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error getting card from card vault")?;
            payment_methods::get_card_detail(&pm, card)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed while getting card details from locker")?
        } else {
            get_card_details_without_locker_fallback(&pm, &state).await?
        };
        Some(card_detail)
    } else {
        None
    };
    Ok(services::ApplicationResponse::Json(
        api::PaymentMethodResponse {
            merchant_id: pm.merchant_id.clone(),
            customer_id: Some(pm.customer_id.clone()),
            payment_method_id: pm.payment_method_id.clone(),
            payment_method: pm.get_payment_method_type(),
            payment_method_type: pm.get_payment_method_subtype(),
            #[cfg(feature = "payouts")]
            bank_transfer: None,
            card,
            metadata: pm.metadata,
            created: Some(pm.created_at),
            recurring_enabled: false,
            installment_payment_enabled: false,
            payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]),
            last_used_at: Some(pm.last_used_at),
            client_secret: pm.client_secret,
        },
    ))
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
#[instrument(skip_all)]
pub async fn delete_payment_method(
    state: routes::SessionState,
    merchant_account: domain::MerchantAccount,
    pm_id: api::PaymentMethodId,
    key_store: domain::MerchantKeyStore,
) -> errors::RouterResponse<api::PaymentMethodDeleteResponse> {
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();
    let key = db
        .find_payment_method(
            &((&state).into()),
            &key_store,
            pm_id.payment_method_id.as_str(),
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

    let customer = db
        .find_customer_by_customer_id_merchant_id(
            key_manager_state,
            &key.customer_id,
            merchant_account.get_id(),
            &key_store,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Customer not found for the payment method")?;

    if key.get_payment_method_type() == Some(enums::PaymentMethod::Card) {
        let response = delete_card_from_locker(
            &state,
            &key.customer_id,
            &key.merchant_id,
            key.locker_id.as_ref().unwrap_or(&key.payment_method_id),
        )
        .await?;

        if let Some(network_token_ref_id) = key.network_token_requestor_reference_id {
            let resp = network_tokenization::delete_network_token_from_locker_and_token_service(
                &state,
                &key.customer_id,
                &key.merchant_id,
                key.payment_method_id.clone(),
                key.network_token_locker_id,
                network_token_ref_id,
            )
            .await?;

            if resp.status == "Ok" {
                logger::info!("Token From locker deleted Successfully!");
            } else {
                logger::error!("Error: Deleting Token From Locker!\n{:#?}", resp);
            }
        }

        if response.status == "Ok" {
            logger::info!("Card From locker deleted Successfully!");
        } else {
            logger::error!("Error: Deleting Card From Locker!\n{:#?}", response);
            Err(errors::ApiErrorResponse::InternalServerError)?
        }
    }

    db.delete_payment_method_by_merchant_id_payment_method_id(
        &((&state).into()),
        &key_store,
        merchant_account.get_id(),
        pm_id.payment_method_id.as_str(),
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

    if customer.default_payment_method_id.as_ref() == Some(&pm_id.payment_method_id) {
        let customer_update = CustomerUpdate::UpdateDefaultPaymentMethod {
            default_payment_method_id: Some(None),
        };
        db.update_customer_by_customer_id_merchant_id(
            key_manager_state,
            key.customer_id,
            key.merchant_id,
            customer,
            customer_update,
            &key_store,
            merchant_account.storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to update the default payment method id for the customer")?;
    };

    Ok(services::ApplicationResponse::Json(
        api::PaymentMethodDeleteResponse {
            payment_method_id: key.payment_method_id.clone(),
            deleted: true,
        },
    ))
}

pub async fn create_encrypted_data<T>(
    key_manager_state: &KeyManagerState,
    key_store: &domain::MerchantKeyStore,
    data: T,
) -> Result<Encryptable<Secret<serde_json::Value>>, error_stack::Report<errors::StorageError>>
where
    T: Debug + serde::Serialize,
{
    let key = key_store.key.get_inner().peek();
    let identifier = Identifier::Merchant(key_store.merchant_id.clone());

    let encoded_data = Encode::encode_to_value(&data)
        .change_context(errors::StorageError::SerializationFailed)
        .attach_printable("Unable to encode data")?;

    let secret_data = Secret::<_, masking::WithType>::new(encoded_data);

    let encrypted_data = domain::types::crypto_operation(
        key_manager_state,
        type_name!(payment_method::PaymentMethod),
        domain::types::CryptoOperation::Encrypt(secret_data),
        identifier.clone(),
        key,
    )
    .await
    .and_then(|val| val.try_into_operation())
    .change_context(errors::StorageError::EncryptionError)
    .attach_printable("Unable to encrypt data")?;

    Ok(encrypted_data)
}

pub async fn list_countries_currencies_for_connector_payment_method(
    state: routes::SessionState,
    req: ListCountriesCurrenciesRequest,
    _profile_id: Option<id_type::ProfileId>,
) -> errors::RouterResponse<ListCountriesCurrenciesResponse> {
    Ok(services::ApplicationResponse::Json(
        list_countries_currencies_for_connector_payment_method_util(
            state.conf.pm_filters.clone(),
            req.connector,
            req.payment_method_type,
        )
        .await,
    ))
}

// This feature will be more efficient as a WASM function rather than as an API.
// So extracting this logic to a separate function so that it can be used in WASM as well.
pub async fn list_countries_currencies_for_connector_payment_method_util(
    connector_filters: settings::ConnectorFilters,
    connector: api_enums::Connector,
    payment_method_type: api_enums::PaymentMethodType,
) -> ListCountriesCurrenciesResponse {
    let payment_method_type =
        settings::PaymentMethodFilterKey::PaymentMethodType(payment_method_type);

    let (currencies, country_codes) = connector_filters
        .0
        .get(&connector.to_string())
        .and_then(|filter| filter.0.get(&payment_method_type))
        .map(|filter| (filter.currency.clone(), filter.country.clone()))
        .unwrap_or_else(|| {
            connector_filters
                .0
                .get("default")
                .and_then(|filter| filter.0.get(&payment_method_type))
                .map_or((None, None), |filter| {
                    (filter.currency.clone(), filter.country.clone())
                })
        });

    let currencies =
        currencies.unwrap_or_else(|| api_enums::Currency::iter().collect::<HashSet<_>>());
    let country_codes =
        country_codes.unwrap_or_else(|| api_enums::CountryAlpha2::iter().collect::<HashSet<_>>());

    ListCountriesCurrenciesResponse {
        currencies,
        countries: country_codes
            .into_iter()
            .map(|country_code| CountryCodeWithName {
                code: country_code,
                name: common_enums::Country::from_alpha2(country_code),
            })
            .collect(),
    }
}
