use std::{collections::HashMap, str::FromStr};

use api_models::{
    enums,
    payment_methods::{self, BankAccountAccessCreds},
};
use common_enums::{enums::MerchantStorageScheme, PaymentMethodType};
use hex;
pub mod helpers;
pub mod transformers;

use common_utils::{
    consts,
    crypto::{HmacSha256, SignMessage},
    ext_traits::{AsyncExt, ValueExt},
    generate_id,
    types::{self as util_types, AmountConvertor},
};
use error_stack::ResultExt;
use helpers::PaymentAuthConnectorDataExt;
use hyperswitch_domain_models::payments::PaymentIntent;
use masking::{ExposeInterface, PeekInterface, Secret};
use pm_auth::{
    connector::plaid::transformers::PlaidAuthType,
    types::{
        self as pm_auth_types,
        api::{
            auth_service::{BankAccountCredentials, ExchangeToken, LinkToken},
            BoxedConnectorIntegration, PaymentAuthConnectorData,
        },
    },
};

use crate::{
    core::{
        errors::{self, ApiErrorResponse, RouterResponse, RouterResult, StorageErrorExt},
        payment_methods::cards,
        payments::helpers as oss_helpers,
        pm_auth::helpers as pm_auth_helpers,
    },
    db::StorageInterface,
    logger,
    routes::SessionState,
    services::{pm_auth as pm_auth_services, ApplicationResponse},
    types::{self, domain, storage, transformers::ForeignTryFrom},
};

#[cfg(feature = "v1")]
pub async fn create_link_token(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    payload: api_models::pm_auth::LinkTokenCreateRequest,
    headers: Option<hyperswitch_domain_models::payments::HeaderPayload>,
) -> RouterResponse<api_models::pm_auth::LinkTokenCreateResponse> {
    let db = &*state.store;

    let redis_conn = db
        .get_redis_conn()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get redis connection")?;

    let pm_auth_key = payload.payment_id.get_pm_auth_key();

    redis_conn
        .exists::<Vec<u8>>(&pm_auth_key.as_str().into())
        .await
        .change_context(ApiErrorResponse::InvalidRequestData {
            message: "Incorrect payment_id provided in request".to_string(),
        })
        .attach_printable("Corresponding pm_auth_key does not exist in redis")?
        .then_some(())
        .ok_or(ApiErrorResponse::InvalidRequestData {
            message: "Incorrect payment_id provided in request".to_string(),
        })
        .attach_printable("Corresponding pm_auth_key does not exist in redis")?;

    let pm_auth_configs = redis_conn
        .get_and_deserialize_key::<Vec<api_models::pm_auth::PaymentMethodAuthConnectorChoice>>(
            &pm_auth_key.as_str().into(),
            "Vec<PaymentMethodAuthConnectorChoice>",
        )
        .await
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get payment method auth choices from redis")?;

    let selected_config = pm_auth_configs
        .into_iter()
        .find(|config| {
            config.payment_method == payload.payment_method
                && config.payment_method_type == payload.payment_method_type
        })
        .ok_or(ApiErrorResponse::GenericNotFoundError {
            message: "payment method auth connector name not found".to_string(),
        })?;

    let connector_name = selected_config.connector_name.as_str();

    let connector = PaymentAuthConnectorData::get_connector_by_name(connector_name)?;
    let connector_integration: BoxedConnectorIntegration<
        '_,
        LinkToken,
        pm_auth_types::LinkTokenRequest,
        pm_auth_types::LinkTokenResponse,
    > = connector.connector.get_connector_integration();

    let payment_intent = oss_helpers::verify_payment_intent_time_and_client_secret(
        &state,
        &merchant_account,
        &key_store,
        payload.client_secret,
    )
    .await?;

    let billing_country = payment_intent
        .as_ref()
        .async_map(|pi| async {
            oss_helpers::get_address_by_id(
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
        .flatten()
        .and_then(|address| address.country)
        .map(|country| country.to_string());

    #[cfg(feature = "v1")]
    let merchant_connector_account = state
        .store
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            &(&state).into(),
            merchant_account.get_id(),
            &selected_config.mca_id,
            &key_store,
        )
        .await
        .change_context(ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: merchant_account.get_id().get_string_repr().to_owned(),
        })?;

    #[cfg(feature = "v2")]
    let merchant_connector_account = {
        let _ = billing_country;
        todo!()
    };

    let auth_type = helpers::get_connector_auth_type(merchant_connector_account)?;

    let router_data = pm_auth_types::LinkTokenRouterData {
        flow: std::marker::PhantomData,
        merchant_id: Some(merchant_account.get_id().clone()),
        connector: Some(connector_name.to_string()),
        request: pm_auth_types::LinkTokenRequest {
            client_name: "HyperSwitch".to_string(),
            country_codes: Some(vec![billing_country.ok_or(
                ApiErrorResponse::MissingRequiredField {
                    field_name: "billing_country",
                },
            )?]),
            language: payload.language,
            user_info: payment_intent.and_then(|pi| pi.customer_id),
            client_platform: headers
                .as_ref()
                .and_then(|header| header.x_client_platform.clone()),
            android_package_name: headers.as_ref().and_then(|header| header.x_app_id.clone()),
            redirect_uri: headers
                .as_ref()
                .and_then(|header| header.x_redirect_uri.clone()),
        },
        response: Ok(pm_auth_types::LinkTokenResponse {
            link_token: "".to_string(),
        }),
        connector_http_status_code: None,
        connector_auth_type: auth_type,
    };

    let connector_resp = pm_auth_services::execute_connector_processing_step(
        &state,
        connector_integration,
        &router_data,
        &connector.connector_name,
    )
    .await
    .change_context(ApiErrorResponse::InternalServerError)
    .attach_printable("Failed while calling link token creation connector api")?;

    let link_token_resp =
        connector_resp
            .response
            .map_err(|err| ApiErrorResponse::ExternalConnectorError {
                code: err.code,
                message: err.message,
                connector: connector.connector_name.to_string(),
                status_code: err.status_code,
                reason: err.reason,
            })?;

    let response = api_models::pm_auth::LinkTokenCreateResponse {
        link_token: link_token_resp.link_token,
        connector: connector.connector_name.to_string(),
    };

    Ok(ApplicationResponse::Json(response))
}

#[cfg(feature = "v2")]
pub async fn create_link_token(
    _state: SessionState,
    _merchant_account: domain::MerchantAccount,
    _key_store: domain::MerchantKeyStore,
    _payload: api_models::pm_auth::LinkTokenCreateRequest,
    _headers: Option<hyperswitch_domain_models::payments::HeaderPayload>,
) -> RouterResponse<api_models::pm_auth::LinkTokenCreateResponse> {
    todo!()
}

impl ForeignTryFrom<&types::ConnectorAuthType> for PlaidAuthType {
    type Error = errors::ConnectorError;

    fn foreign_try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1 } => {
                Ok::<Self, errors::ConnectorError>(Self {
                    client_id: api_key.to_owned(),
                    secret: key1.to_owned(),
                })
            }
            _ => Err(errors::ConnectorError::FailedToObtainAuthType),
        }
    }
}

pub async fn exchange_token_core(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    payload: api_models::pm_auth::ExchangeTokenCreateRequest,
) -> RouterResponse<()> {
    let db = &*state.store;

    let config = get_selected_config_from_redis(db, &payload).await?;

    let connector_name = config.connector_name.as_str();

    let connector = PaymentAuthConnectorData::get_connector_by_name(connector_name)?;

    #[cfg(feature = "v1")]
    let merchant_connector_account = state
        .store
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            &(&state).into(),
            merchant_account.get_id(),
            &config.mca_id,
            &key_store,
        )
        .await
        .change_context(ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: merchant_account.get_id().get_string_repr().to_owned(),
        })?;

    #[cfg(feature = "v2")]
    let merchant_connector_account: domain::MerchantConnectorAccount = {
        let _ = merchant_account;
        let _ = connector;
        let _ = key_store;
        todo!()
    };

    let auth_type = helpers::get_connector_auth_type(merchant_connector_account.clone())?;

    let access_token = get_access_token_from_exchange_api(
        &connector,
        connector_name,
        &payload,
        &auth_type,
        &state,
    )
    .await?;

    let bank_account_details_resp = get_bank_account_creds(
        connector,
        &merchant_account,
        connector_name,
        &access_token,
        auth_type,
        &state,
        None,
    )
    .await?;

    Box::pin(store_bank_details_in_payment_methods(
        key_store,
        payload,
        merchant_account,
        state,
        bank_account_details_resp,
        (connector_name, access_token),
        merchant_connector_account.get_id(),
    ))
    .await?;

    Ok(ApplicationResponse::StatusOk)
}

#[cfg(feature = "v1")]
async fn store_bank_details_in_payment_methods(
    key_store: domain::MerchantKeyStore,
    payload: api_models::pm_auth::ExchangeTokenCreateRequest,
    merchant_account: domain::MerchantAccount,
    state: SessionState,
    bank_account_details_resp: pm_auth_types::BankAccountCredentialsResponse,
    connector_details: (&str, Secret<String>),
    mca_id: common_utils::id_type::MerchantConnectorAccountId,
) -> RouterResult<()> {
    let db = &*state.clone().store;
    let (connector_name, access_token) = connector_details;

    let payment_intent = db
        .find_payment_intent_by_payment_id_merchant_id(
            &(&state).into(),
            &payload.payment_id,
            merchant_account.get_id(),
            &key_store,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(ApiErrorResponse::PaymentNotFound)?;

    let customer_id = payment_intent
        .customer_id
        .ok_or(ApiErrorResponse::CustomerNotFound)?;

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    let payment_methods = db
        .find_payment_method_by_customer_id_merchant_id_list(
            &((&state).into()),
            &key_store,
            &customer_id,
            merchant_account.get_id(),
            None,
        )
        .await
        .change_context(ApiErrorResponse::InternalServerError)?;

    #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
    let payment_methods = db
        .find_payment_method_by_customer_id_merchant_id_status(
            &((&state).into()),
            &key_store,
            &customer_id,
            merchant_account.get_id(),
            common_enums::enums::PaymentMethodStatus::Active,
            None,
            merchant_account.storage_scheme,
        )
        .await
        .change_context(ApiErrorResponse::InternalServerError)?;

    let mut hash_to_payment_method: HashMap<
        String,
        (
            domain::PaymentMethod,
            payment_methods::PaymentMethodDataBankCreds,
        ),
    > = HashMap::new();
    let key_manager_state = (&state).into();
    for pm in payment_methods {
        if pm.get_payment_method_type() == Some(enums::PaymentMethod::BankDebit)
            && pm.payment_method_data.is_some()
        {
            let bank_details_pm_data = pm
                .payment_method_data
                .clone()
                .map(|x| x.into_inner().expose())
                .map(|v| v.parse_value("PaymentMethodsData"))
                .transpose()
                .unwrap_or_else(|error| {
                    logger::error!(?error);
                    None
                })
                .and_then(|pmd| match pmd {
                    payment_methods::PaymentMethodsData::BankDetails(bank_creds) => {
                        Some(bank_creds)
                    }
                    _ => None,
                })
                .ok_or(ApiErrorResponse::InternalServerError)
                .attach_printable("Unable to parse PaymentMethodsData")?;

            hash_to_payment_method.insert(
                bank_details_pm_data.hash.clone(),
                (pm, bank_details_pm_data),
            );
        }
    }

    let pm_auth_key = state
        .conf
        .payment_method_auth
        .get_inner()
        .pm_auth_key
        .clone()
        .expose();

    let mut update_entries: Vec<(domain::PaymentMethod, storage::PaymentMethodUpdate)> = Vec::new();
    let mut new_entries: Vec<domain::PaymentMethod> = Vec::new();

    for creds in bank_account_details_resp.credentials {
        let (account_number, hash_string) = match creds.account_details {
            pm_auth_types::PaymentMethodTypeDetails::Ach(ach) => (
                ach.account_number.clone(),
                format!(
                    "{}-{}-{}",
                    ach.account_number.peek(),
                    ach.routing_number.peek(),
                    PaymentMethodType::Ach,
                ),
            ),
            pm_auth_types::PaymentMethodTypeDetails::Bacs(bacs) => (
                bacs.account_number.clone(),
                format!(
                    "{}-{}-{}",
                    bacs.account_number.peek(),
                    bacs.sort_code.peek(),
                    PaymentMethodType::Bacs
                ),
            ),
            pm_auth_types::PaymentMethodTypeDetails::Sepa(sepa) => (
                sepa.iban.clone(),
                format!("{}-{}", sepa.iban.expose(), PaymentMethodType::Sepa),
            ),
        };

        let generated_hash = hex::encode(
            HmacSha256::sign_message(&HmacSha256, pm_auth_key.as_bytes(), hash_string.as_bytes())
                .change_context(ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to sign the message")?,
        );

        let contains_account = hash_to_payment_method.get(&generated_hash);
        let mut pmd = payment_methods::PaymentMethodDataBankCreds {
            mask: account_number
                .peek()
                .chars()
                .rev()
                .take(4)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>(),
            hash: generated_hash,
            account_type: creds.account_type,
            account_name: creds.account_name,
            payment_method_type: creds.payment_method_type,
            connector_details: vec![payment_methods::BankAccountConnectorDetails {
                connector: connector_name.to_string(),
                mca_id: mca_id.clone(),
                access_token: BankAccountAccessCreds::AccessToken(access_token.clone()),
                account_id: creds.account_id,
            }],
        };

        if let Some((pm, details)) = contains_account {
            pmd.connector_details.extend(
                details
                    .connector_details
                    .clone()
                    .into_iter()
                    .filter(|conn| conn.mca_id != mca_id),
            );

            let payment_method_data = payment_methods::PaymentMethodsData::BankDetails(pmd);

            let encrypted_data =
                cards::create_encrypted_data(&key_manager_state, &key_store, payment_method_data)
                    .await
                    .change_context(ApiErrorResponse::InternalServerError)
                    .attach_printable("Unable to encrypt customer details")?;

            let pm_update = storage::PaymentMethodUpdate::PaymentMethodDataUpdate {
                payment_method_data: Some(encrypted_data.into()),
            };

            update_entries.push((pm.clone(), pm_update));
        } else {
            let payment_method_data = payment_methods::PaymentMethodsData::BankDetails(pmd);
            let encrypted_data = cards::create_encrypted_data(
                &key_manager_state,
                &key_store,
                Some(payment_method_data),
            )
            .await
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to encrypt customer details")?;

            #[cfg(all(
                any(feature = "v1", feature = "v2"),
                not(feature = "payment_methods_v2")
            ))]
            let pm_id = generate_id(consts::ID_LENGTH, "pm");

            #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
            let pm_id = common_utils::id_type::GlobalPaymentMethodId::generate("random_cell_id")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unable to generate GlobalPaymentMethodId")?;

            let now = common_utils::date_time::now();
            #[cfg(all(
                any(feature = "v1", feature = "v2"),
                not(feature = "payment_methods_v2")
            ))]
            let pm_new = domain::PaymentMethod {
                customer_id: customer_id.clone(),
                merchant_id: merchant_account.get_id().clone(),
                payment_method_id: pm_id,
                payment_method: Some(enums::PaymentMethod::BankDebit),
                payment_method_type: Some(creds.payment_method_type),
                status: enums::PaymentMethodStatus::Active,
                payment_method_issuer: None,
                scheme: None,
                metadata: None,
                payment_method_data: Some(encrypted_data),
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
                created_at: now,
                last_modified: now,
                locker_id: None,
                last_used_at: now,
                connector_mandate_details: None,
                customer_acceptance: None,
                network_transaction_id: None,
                client_secret: None,
                payment_method_billing_address: None,
                updated_by: None,
                version: domain::consts::API_VERSION,
                network_token_requestor_reference_id: None,
                network_token_locker_id: None,
                network_token_payment_method_data: None,
            };

            #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
            let pm_new = domain::PaymentMethod {
                customer_id: customer_id.clone(),
                merchant_id: merchant_account.get_id().clone(),
                id: pm_id,
                payment_method_type: Some(enums::PaymentMethod::BankDebit),
                payment_method_subtype: Some(creds.payment_method_type),
                status: enums::PaymentMethodStatus::Active,
                metadata: None,
                payment_method_data: Some(encrypted_data.into()),
                created_at: now,
                last_modified: now,
                locker_id: None,
                last_used_at: now,
                connector_mandate_details: None,
                customer_acceptance: None,
                network_transaction_id: None,
                client_secret: None,
                payment_method_billing_address: None,
                updated_by: None,
                locker_fingerprint_id: None,
                version: domain::consts::API_VERSION,
                network_token_requestor_reference_id: None,
                network_token_locker_id: None,
                network_token_payment_method_data: None,
            };

            new_entries.push(pm_new);
        };
    }

    store_in_db(
        &state,
        &key_store,
        update_entries,
        new_entries,
        db,
        merchant_account.storage_scheme,
    )
    .await?;

    Ok(())
}

#[cfg(feature = "v2")]
async fn store_bank_details_in_payment_methods(
    _key_store: domain::MerchantKeyStore,
    _payload: api_models::pm_auth::ExchangeTokenCreateRequest,
    _merchant_account: domain::MerchantAccount,
    _state: SessionState,
    _bank_account_details_resp: pm_auth_types::BankAccountCredentialsResponse,
    _connector_details: (&str, Secret<String>),
    _mca_id: common_utils::id_type::MerchantConnectorAccountId,
) -> RouterResult<()> {
    todo!()
}

async fn store_in_db(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    update_entries: Vec<(domain::PaymentMethod, storage::PaymentMethodUpdate)>,
    new_entries: Vec<domain::PaymentMethod>,
    db: &dyn StorageInterface,
    storage_scheme: MerchantStorageScheme,
) -> RouterResult<()> {
    let key_manager_state = &(state.into());
    let update_entries_futures = update_entries
        .into_iter()
        .map(|(pm, pm_update)| {
            db.update_payment_method(key_manager_state, key_store, pm, pm_update, storage_scheme)
        })
        .collect::<Vec<_>>();

    let new_entries_futures = new_entries
        .into_iter()
        .map(|pm_new| {
            db.insert_payment_method(key_manager_state, key_store, pm_new, storage_scheme)
        })
        .collect::<Vec<_>>();

    let update_futures = futures::future::join_all(update_entries_futures);
    let new_futures = futures::future::join_all(new_entries_futures);

    let (update, new) = tokio::join!(update_futures, new_futures);

    let _ = update
        .into_iter()
        .map(|res| res.map_err(|err| logger::error!("Payment method storage failed {err:?}")));

    let _ = new
        .into_iter()
        .map(|res| res.map_err(|err| logger::error!("Payment method storage failed {err:?}")));

    Ok(())
}

pub async fn get_bank_account_creds(
    connector: PaymentAuthConnectorData,
    merchant_account: &domain::MerchantAccount,
    connector_name: &str,
    access_token: &Secret<String>,
    auth_type: pm_auth_types::ConnectorAuthType,
    state: &SessionState,
    bank_account_id: Option<Secret<String>>,
) -> RouterResult<pm_auth_types::BankAccountCredentialsResponse> {
    let connector_integration_bank_details: BoxedConnectorIntegration<
        '_,
        BankAccountCredentials,
        pm_auth_types::BankAccountCredentialsRequest,
        pm_auth_types::BankAccountCredentialsResponse,
    > = connector.connector.get_connector_integration();

    let router_data_bank_details = pm_auth_types::BankDetailsRouterData {
        flow: std::marker::PhantomData,
        merchant_id: Some(merchant_account.get_id().clone()),
        connector: Some(connector_name.to_string()),
        request: pm_auth_types::BankAccountCredentialsRequest {
            access_token: access_token.clone(),
            optional_ids: bank_account_id
                .map(|id| pm_auth_types::BankAccountOptionalIDs { ids: vec![id] }),
        },
        response: Ok(pm_auth_types::BankAccountCredentialsResponse {
            credentials: Vec::new(),
        }),
        connector_http_status_code: None,
        connector_auth_type: auth_type,
    };

    let bank_details_resp = pm_auth_services::execute_connector_processing_step(
        state,
        connector_integration_bank_details,
        &router_data_bank_details,
        &connector.connector_name,
    )
    .await
    .change_context(ApiErrorResponse::InternalServerError)
    .attach_printable("Failed while calling bank account details connector api")?;

    let bank_account_details_resp =
        bank_details_resp
            .response
            .map_err(|err| ApiErrorResponse::ExternalConnectorError {
                code: err.code,
                message: err.message,
                connector: connector.connector_name.to_string(),
                status_code: err.status_code,
                reason: err.reason,
            })?;

    Ok(bank_account_details_resp)
}

async fn get_access_token_from_exchange_api(
    connector: &PaymentAuthConnectorData,
    connector_name: &str,
    payload: &api_models::pm_auth::ExchangeTokenCreateRequest,
    auth_type: &pm_auth_types::ConnectorAuthType,
    state: &SessionState,
) -> RouterResult<Secret<String>> {
    let connector_integration: BoxedConnectorIntegration<
        '_,
        ExchangeToken,
        pm_auth_types::ExchangeTokenRequest,
        pm_auth_types::ExchangeTokenResponse,
    > = connector.connector.get_connector_integration();

    let router_data = pm_auth_types::ExchangeTokenRouterData {
        flow: std::marker::PhantomData,
        merchant_id: None,
        connector: Some(connector_name.to_string()),
        request: pm_auth_types::ExchangeTokenRequest {
            public_token: payload.public_token.clone(),
        },
        response: Ok(pm_auth_types::ExchangeTokenResponse {
            access_token: "".to_string(),
        }),
        connector_http_status_code: None,
        connector_auth_type: auth_type.clone(),
    };

    let resp = pm_auth_services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        &connector.connector_name,
    )
    .await
    .change_context(ApiErrorResponse::InternalServerError)
    .attach_printable("Failed while calling exchange token connector api")?;

    let exchange_token_resp =
        resp.response
            .map_err(|err| ApiErrorResponse::ExternalConnectorError {
                code: err.code,
                message: err.message,
                connector: connector.connector_name.to_string(),
                status_code: err.status_code,
                reason: err.reason,
            })?;

    let access_token = exchange_token_resp.access_token;
    Ok(Secret::new(access_token))
}

async fn get_selected_config_from_redis(
    db: &dyn StorageInterface,
    payload: &api_models::pm_auth::ExchangeTokenCreateRequest,
) -> RouterResult<api_models::pm_auth::PaymentMethodAuthConnectorChoice> {
    let redis_conn = db
        .get_redis_conn()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get redis connection")?;

    let pm_auth_key = payload.payment_id.get_pm_auth_key();

    redis_conn
        .exists::<Vec<u8>>(&pm_auth_key.as_str().into())
        .await
        .change_context(ApiErrorResponse::InvalidRequestData {
            message: "Incorrect payment_id provided in request".to_string(),
        })
        .attach_printable("Corresponding pm_auth_key does not exist in redis")?
        .then_some(())
        .ok_or(ApiErrorResponse::InvalidRequestData {
            message: "Incorrect payment_id provided in request".to_string(),
        })
        .attach_printable("Corresponding pm_auth_key does not exist in redis")?;

    let pm_auth_configs = redis_conn
        .get_and_deserialize_key::<Vec<api_models::pm_auth::PaymentMethodAuthConnectorChoice>>(
            &pm_auth_key.as_str().into(),
            "Vec<PaymentMethodAuthConnectorChoice>",
        )
        .await
        .change_context(ApiErrorResponse::GenericNotFoundError {
            message: "payment method auth connector name not found".to_string(),
        })
        .attach_printable("Failed to get payment method auth choices from redis")?;

    let selected_config = pm_auth_configs
        .iter()
        .find(|conf| {
            conf.payment_method == payload.payment_method
                && conf.payment_method_type == payload.payment_method_type
        })
        .ok_or(ApiErrorResponse::GenericNotFoundError {
            message: "payment method auth connector name not found".to_string(),
        })?
        .clone();

    Ok(selected_config)
}

#[cfg(feature = "v2")]
pub async fn retrieve_payment_method_from_auth_service(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    auth_token: &payment_methods::BankAccountTokenData,
    payment_intent: &PaymentIntent,
    _customer: &Option<domain::Customer>,
) -> RouterResult<Option<(domain::PaymentMethodData, enums::PaymentMethod)>> {
    todo!()
}

#[cfg(feature = "v1")]
pub async fn retrieve_payment_method_from_auth_service(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    auth_token: &payment_methods::BankAccountTokenData,
    payment_intent: &PaymentIntent,
    _customer: &Option<domain::Customer>,
) -> RouterResult<Option<(domain::PaymentMethodData, enums::PaymentMethod)>> {
    let db = state.store.as_ref();

    let connector = PaymentAuthConnectorData::get_connector_by_name(
        auth_token.connector_details.connector.as_str(),
    )?;
    let key_manager_state = &state.into();
    let merchant_account = db
        .find_merchant_account_by_merchant_id(
            key_manager_state,
            &payment_intent.merchant_id,
            key_store,
        )
        .await
        .to_not_found_response(ApiErrorResponse::MerchantAccountNotFound)?;

    #[cfg(feature = "v1")]
    let mca = db
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            key_manager_state,
            &payment_intent.merchant_id,
            &auth_token.connector_details.mca_id,
            key_store,
        )
        .await
        .to_not_found_response(ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: auth_token
                .connector_details
                .mca_id
                .get_string_repr()
                .to_string()
                .clone(),
        })
        .attach_printable(
            "error while fetching merchant_connector_account from merchant_id and connector name",
        )?;

    let auth_type = pm_auth_helpers::get_connector_auth_type(mca)?;

    let BankAccountAccessCreds::AccessToken(access_token) =
        &auth_token.connector_details.access_token;

    let bank_account_creds = get_bank_account_creds(
        connector,
        &merchant_account,
        &auth_token.connector_details.connector,
        access_token,
        auth_type,
        state,
        Some(auth_token.connector_details.account_id.clone()),
    )
    .await?;

    let bank_account = bank_account_creds
        .credentials
        .iter()
        .find(|acc| {
            acc.payment_method_type == auth_token.payment_method_type
                && acc.payment_method == auth_token.payment_method
        })
        .ok_or(ApiErrorResponse::InternalServerError)
        .attach_printable("Bank account details not found")?;

    if let (Some(balance), Some(currency)) = (bank_account.balance, payment_intent.currency) {
        let required_conversion = util_types::FloatMajorUnitForConnector;
        let converted_amount = required_conversion
            .convert_back(balance, currency)
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Could not convert FloatMajorUnit to MinorUnit")?;

        if converted_amount < payment_intent.amount {
            return Err((ApiErrorResponse::PreconditionFailed {
                message: "selected bank account has insufficient balance".to_string(),
            })
            .into());
        }
    }

    let mut bank_type = None;
    if let Some(account_type) = bank_account.account_type.clone() {
        bank_type = common_enums::BankType::from_str(account_type.as_str())
            .map_err(|error| logger::error!(%error,"unable to parse account_type {account_type:?}"))
            .ok();
    }

    let payment_method_data = match &bank_account.account_details {
        pm_auth_types::PaymentMethodTypeDetails::Ach(ach) => {
            domain::PaymentMethodData::BankDebit(domain::BankDebitData::AchBankDebit {
                account_number: ach.account_number.clone(),
                routing_number: ach.routing_number.clone(),
                bank_name: None,
                bank_type,
                bank_holder_type: None,
                card_holder_name: None,
                bank_account_holder_name: None,
            })
        }
        pm_auth_types::PaymentMethodTypeDetails::Bacs(bacs) => {
            domain::PaymentMethodData::BankDebit(domain::BankDebitData::BacsBankDebit {
                account_number: bacs.account_number.clone(),
                sort_code: bacs.sort_code.clone(),
                bank_account_holder_name: None,
            })
        }
        pm_auth_types::PaymentMethodTypeDetails::Sepa(sepa) => {
            domain::PaymentMethodData::BankDebit(domain::BankDebitData::SepaBankDebit {
                iban: sepa.iban.clone(),
                bank_account_holder_name: None,
            })
        }
    };

    Ok(Some((payment_method_data, enums::PaymentMethod::BankDebit)))
}
