use std::{collections::HashMap, str::FromStr};

use api_models::{
    enums,
    payment_methods::{self, BankAccountAccessCreds},
    payments::{AddressDetails, BankDebitBilling, BankDebitData, PaymentMethodData},
};
use common_enums::PaymentMethodType;
use hex;
pub mod helpers;
pub mod transformers;

use common_utils::{
    consts,
    crypto::{HmacSha256, SignMessage},
    ext_traits::AsyncExt,
    generate_id,
};
use data_models::payments::PaymentIntent;
use error_stack::{IntoReport, ResultExt};
use helpers::PaymentAuthConnectorDataExt;
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
        pm_auth::helpers::{self as pm_auth_helpers},
    },
    db::StorageInterface,
    logger,
    routes::AppState,
    services::{
        pm_auth::{self as pm_auth_services},
        ApplicationResponse,
    },
    types::{
        self,
        domain::{self, types::decrypt},
        storage,
        transformers::ForeignTryFrom,
    },
    utils::ext_traits::OptionExt,
};

pub async fn create_link_token(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    payload: api_models::pm_auth::LinkTokenCreateRequest,
) -> RouterResponse<api_models::pm_auth::LinkTokenCreateResponse> {
    let db = &*state.store;

    let redis_conn = db
        .get_redis_conn()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get redis connection")?;

    let pm_auth_key = format!("pm_auth_{}", payload.payment_id);

    let pm_auth_configs = redis_conn
        .get_and_deserialize_key::<Vec<api_models::pm_auth::PaymentMethodAuthConnectorChoice>>(
            pm_auth_key.as_str(),
            "Vec<PaymentMethodAuthConnectorChoice>",
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get payment method auth choices from redis")?;

    let selected_config = pm_auth_configs
        .into_iter()
        .find(|config| {
            config.payment_method == payload.payment_method
                && config.payment_method_type == payload.payment_method_type
        })
        .ok_or(ApiErrorResponse::GenericNotFoundError {
            message: "payment method auth connector name not found".to_string(),
        })
        .into_report()?;

    let connector_name = selected_config.connector_name.as_str();

    let connector = PaymentAuthConnectorData::get_connector_by_name(connector_name)?;
    let connector_integration: BoxedConnectorIntegration<
        '_,
        LinkToken,
        pm_auth_types::LinkTokenRequest,
        pm_auth_types::LinkTokenResponse,
    > = connector.connector.get_connector_integration();

    let payment_intent = oss_helpers::verify_payment_intent_time_and_client_secret(
        &*state.store,
        &merchant_account,
        payload.client_secret,
    )
    .await?;

    let billing_country = payment_intent
        .as_ref()
        .async_map(|pi| async {
            oss_helpers::get_address_by_id(
                &*state.store,
                pi.billing_address_id.clone(),
                &key_store,
                &pi.payment_id,
                &merchant_account.merchant_id,
                merchant_account.storage_scheme,
            )
            .await
        })
        .await
        .transpose()?
        .flatten()
        .and_then(|address| address.country)
        .map(|country| country.to_string());

    let merchant_connector_account = state
        .store
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            merchant_account.merchant_id.as_str(),
            &selected_config.mca_id,
            &key_store,
        )
        .await
        .change_context(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: merchant_account.merchant_id.clone(),
        })?;

    let auth_type = helpers::get_connector_auth_type(merchant_connector_account)?;

    let router_data = pm_auth_types::LinkTokenRouterData {
        flow: std::marker::PhantomData,
        merchant_id: Some(merchant_account.merchant_id),
        connector: Some(connector_name.to_string()),
        request: pm_auth_types::LinkTokenRequest {
            client_name: "HyperSwitch".to_string(),
            country_codes: Some(vec![billing_country.ok_or(
                errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "billing_country",
                },
            )?]),
            language: payload.language,
            user_info: payment_intent.and_then(|pi| pi.customer_id),
        },
        response: Ok(pm_auth_types::LinkTokenResponse {
            link_token: "".to_string(),
        }),
        connector_http_status_code: None,
        connector_auth_type: auth_type,
    };

    let connector_resp = pm_auth_services::execute_connector_processing_step(
        state.as_ref(),
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
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    payload: api_models::pm_auth::ExchangeTokenCreateRequest,
) -> RouterResponse<()> {
    let db = &*state.store;

    let config = get_selected_config_from_redis(db, &payload).await?;

    let connector_name = config.connector_name.as_str();

    let connector =
        pm_auth_types::api::PaymentAuthConnectorData::get_connector_by_name(connector_name)?;

    let merchant_connector_account = state
        .store
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            merchant_account.merchant_id.as_str(),
            &config.mca_id,
            &key_store,
        )
        .await
        .change_context(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: merchant_account.merchant_id.clone(),
        })?;

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
        merchant_connector_account.merchant_connector_id,
    ))
    .await?;

    Ok(ApplicationResponse::StatusOk)
}

async fn store_bank_details_in_payment_methods(
    key_store: domain::MerchantKeyStore,
    payload: api_models::pm_auth::ExchangeTokenCreateRequest,
    merchant_account: domain::MerchantAccount,
    state: AppState,
    bank_account_details_resp: pm_auth_types::BankAccountCredentialsResponse,
    connector_details: (&str, Secret<String>),
    mca_id: String,
) -> RouterResult<()> {
    let key = key_store.key.get_inner().peek();
    let db = &*state.clone().store;
    let (connector_name, access_token) = connector_details;

    let payment_intent = db
        .find_payment_intent_by_payment_id_merchant_id(
            &payload.payment_id,
            &merchant_account.merchant_id,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(ApiErrorResponse::PaymentNotFound)?;

    let customer_id = payment_intent
        .customer_id
        .ok_or(ApiErrorResponse::CustomerNotFound)?;

    let payment_methods = db
        .find_payment_method_by_customer_id_merchant_id_list(
            &customer_id,
            &merchant_account.merchant_id,
            None,
        )
        .await
        .change_context(ApiErrorResponse::InternalServerError)?;

    let mut hash_to_payment_method: HashMap<
        String,
        (
            storage::PaymentMethod,
            payment_methods::PaymentMethodDataBankCreds,
        ),
    > = HashMap::new();

    for pm in payment_methods {
        if pm.payment_method == enums::PaymentMethod::BankDebit {
            let bank_details_pm_data = decrypt::<serde_json::Value, masking::WithType>(
                pm.payment_method_data.clone(),
                key,
            )
            .await
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("unable to decrypt bank account details")?
            .map(|x| x.into_inner().expose())
            .map(|v| {
                serde_json::from_value::<payment_methods::PaymentMethodsData>(v)
                    .into_report()
                    .change_context(errors::StorageError::DeserializationFailed)
                    .attach_printable("Failed to deserialize Payment Method Auth config")
            })
            .transpose()
            .unwrap_or_else(|err| {
                logger::error!(error=?err);
                None
            })
            .and_then(|pmd| match pmd {
                payment_methods::PaymentMethodsData::BankDetails(bank_creds) => Some(bank_creds),
                _ => None,
            })
            .ok_or(ApiErrorResponse::InternalServerError)?;

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

    let mut update_entries: Vec<(storage::PaymentMethod, storage::PaymentMethodUpdate)> =
        Vec::new();
    let mut new_entries: Vec<storage::PaymentMethodNew> = Vec::new();

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
                access_token: payment_methods::BankAccountAccessCreds::AccessToken(
                    access_token.clone(),
                ),
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
                cards::create_encrypted_payment_method_data(&key_store, Some(payment_method_data))
                    .await
                    .ok_or(ApiErrorResponse::InternalServerError)?;
            let pm_update = storage::PaymentMethodUpdate::PaymentMethodDataUpdate {
                payment_method_data: Some(encrypted_data),
            };

            update_entries.push((pm.clone(), pm_update));
        } else {
            let payment_method_data = payment_methods::PaymentMethodsData::BankDetails(pmd);
            let encrypted_data =
                cards::create_encrypted_payment_method_data(&key_store, Some(payment_method_data))
                    .await
                    .ok_or(ApiErrorResponse::InternalServerError)?;
            let pm_id = generate_id(consts::ID_LENGTH, "pm");
            let pm_new = storage::PaymentMethodNew {
                customer_id: customer_id.clone(),
                merchant_id: merchant_account.merchant_id.clone(),
                payment_method_id: pm_id,
                payment_method: enums::PaymentMethod::BankDebit,
                payment_method_type: Some(creds.payment_method_type),
                payment_method_issuer: None,
                scheme: None,
                metadata: None,
                payment_method_data: Some(encrypted_data),
                ..storage::PaymentMethodNew::default()
            };

            new_entries.push(pm_new);
        };
    }

    store_in_db(update_entries, new_entries, db).await?;

    Ok(())
}

async fn store_in_db(
    update_entries: Vec<(storage::PaymentMethod, storage::PaymentMethodUpdate)>,
    new_entries: Vec<storage::PaymentMethodNew>,
    db: &dyn StorageInterface,
) -> RouterResult<()> {
    let update_entries_futures = update_entries
        .into_iter()
        .map(|(pm, pm_update)| db.update_payment_method(pm, pm_update))
        .collect::<Vec<_>>();

    let new_entries_futures = new_entries
        .into_iter()
        .map(|pm_new| db.insert_payment_method(pm_new))
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
    state: &AppState,
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
        merchant_id: Some(merchant_account.merchant_id.clone()),
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
    state: &AppState,
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
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get redis connection")?;

    let pm_auth_key = format!("pm_auth_{}", payload.payment_id);

    let pm_auth_configs = redis_conn
        .get_and_deserialize_key::<Vec<api_models::pm_auth::PaymentMethodAuthConnectorChoice>>(
            pm_auth_key.as_str(),
            "Vec<PaymentMethodAuthConnectorChoice>",
        )
        .await
        .change_context(errors::ApiErrorResponse::GenericNotFoundError {
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
        })
        .into_report()?
        .clone();

    Ok(selected_config)
}

pub async fn retrieve_payment_method_from_auth_service(
    state: &AppState,
    key_store: &domain::MerchantKeyStore,
    auth_token: &payment_methods::BankAccountTokenData,
    payment_intent: &PaymentIntent,
    customer: &Option<domain::Customer>,
) -> RouterResult<Option<(PaymentMethodData, enums::PaymentMethod)>> {
    let db = state.store.as_ref();

    let connector = pm_auth_types::api::PaymentAuthConnectorData::get_connector_by_name(
        auth_token.connector_details.connector.as_str(),
    )?;

    let merchant_account = db
        .find_merchant_account_by_merchant_id(&payment_intent.merchant_id, key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let mca = db
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            &payment_intent.merchant_id,
            &auth_token.connector_details.mca_id,
            key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: auth_token.connector_details.mca_id.clone(),
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
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .into_report()
        .attach_printable("Bank account details not found")?;

    let mut bank_type = None;
    if let Some(account_type) = bank_account.account_type.clone() {
        bank_type = api_models::enums::BankType::from_str(account_type.as_str())
            .map_err(|error| logger::error!(%error,"unable to parse account_type {account_type:?}"))
            .ok();
    }

    let address = oss_helpers::get_address_by_id(
        &*state.store,
        payment_intent.billing_address_id.clone(),
        key_store,
        &payment_intent.payment_id,
        &merchant_account.merchant_id,
        merchant_account.storage_scheme,
    )
    .await?;

    let name = address
        .as_ref()
        .and_then(|addr| addr.first_name.clone().map(|name| name.into_inner()))
        .ok_or(errors::ApiErrorResponse::GenericNotFoundError {
            message: "billing_first_name not found".to_string(),
        })
        .into_report()
        .attach_printable("billing_first_name not found")?;

    let address_details = address.clone().map(|addr| {
        let line1 = addr.line1.map(|line1| line1.into_inner());
        let line2 = addr.line2.map(|line2| line2.into_inner());
        let line3 = addr.line3.map(|line3| line3.into_inner());
        let zip = addr.zip.map(|zip| zip.into_inner());
        let state = addr.state.map(|state| state.into_inner());
        let first_name = addr.first_name.map(|first_name| first_name.into_inner());
        let last_name = addr.last_name.map(|last_name| last_name.into_inner());

        AddressDetails {
            city: addr.city,
            country: addr.country,
            line1,
            line2,
            line3,
            zip,
            state,
            first_name,
            last_name,
        }
    });

    let email = customer
        .as_ref()
        .and_then(|customer| customer.email.clone())
        .map(common_utils::pii::Email::from)
        .get_required_value("email")?;

    let billing_details = BankDebitBilling {
        name,
        email,
        address: address_details,
    };

    let payment_method_data = match &bank_account.account_details {
        pm_auth_types::PaymentMethodTypeDetails::Ach(ach) => {
            PaymentMethodData::BankDebit(BankDebitData::AchBankDebit {
                billing_details,
                account_number: ach.account_number.clone(),
                routing_number: ach.routing_number.clone(),
                card_holder_name: None,
                bank_account_holder_name: None,
                bank_name: None,
                bank_type,
                bank_holder_type: None,
            })
        }
        pm_auth_types::PaymentMethodTypeDetails::Bacs(bacs) => {
            PaymentMethodData::BankDebit(BankDebitData::BacsBankDebit {
                billing_details,
                account_number: bacs.account_number.clone(),
                sort_code: bacs.sort_code.clone(),
                bank_account_holder_name: None,
                bank_account_holder_email: None,
            })
        }
        pm_auth_types::PaymentMethodTypeDetails::Sepa(sepa) => {
            PaymentMethodData::BankDebit(BankDebitData::SepaBankDebit {
                billing_details,
                iban: sepa.iban.clone(),
                bank_account_holder_name: None,
                bank_account_holder_email: None,
            })
        }
    };

    Ok(Some((payment_method_data, enums::PaymentMethod::BankDebit)))
}
