use api_models::admin::PrimaryBusinessDetails;
use common_utils::{
    crypto::{generate_cryptographically_secure_random_string, OptionalSecretValue},
    date_time,
    ext_traits::{Encode, ValueExt},
};
use diesel_models::enums;
use error_stack::{report, FutureExt, ResultExt};
use masking::{PeekInterface, Secret};
use uuid::Uuid;

use crate::{
    consts,
    core::{
        errors::{self, RouterResponse, RouterResult, StorageErrorExt},
        payments::helpers,
    },
    db::StorageInterface,
    routes::metrics,
    services::{self, api as service_api},
    types::{
        self,
        api::{self, ConnectorData},
        domain::{
            self,
            types::{self as domain_types, AsyncLift},
        },
        storage,
    },
    utils::{self, OptionExt},
};

#[inline]
pub fn create_merchant_publishable_key() -> String {
    format!(
        "pk_{}_{}",
        router_env::env::prefix_for_env(),
        Uuid::new_v4().simple()
    )
}

fn get_primary_business_details(
    request: &api::MerchantAccountCreate,
) -> Vec<PrimaryBusinessDetails> {
    // In this case, business details is not optional, it will always be passed
    #[cfg(feature = "multiple_mca")]
    {
        request.primary_business_details.to_owned()
    }

    // In this case, business details will be optional, if it is not passed, then create the
    // default value
    #[cfg(not(feature = "multiple_mca"))]
    {
        request
            .primary_business_details
            .to_owned()
            .unwrap_or_else(|| {
                vec![PrimaryBusinessDetails {
                    country: enums::CountryAlpha2::US,
                    business: "default".to_string(),
                }]
            })
    }
}

pub async fn create_merchant_account(
    db: &dyn StorageInterface,
    req: api::MerchantAccountCreate,
) -> RouterResponse<api::MerchantAccountResponse> {
    let master_key = db.get_master_key();

    let key = services::generate_aes256_key()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to generate aes 256 key")?;

    let publishable_key = Some(create_merchant_publishable_key());

    let primary_business_details = utils::Encode::<Vec<PrimaryBusinessDetails>>::encode_to_value(
        &get_primary_business_details(&req),
    )
    .change_context(errors::ApiErrorResponse::InvalidDataValue {
        field_name: "primary_business_details",
    })?;

    let merchant_details: OptionalSecretValue =
        req.merchant_details
            .as_ref()
            .map(|merchant_details| {
                utils::Encode::<api::MerchantDetails>::encode_to_value(merchant_details)
                    .change_context(errors::ApiErrorResponse::InvalidDataValue {
                        field_name: "merchant_details",
                    })
            })
            .transpose()?
            .map(Into::into);

    let webhook_details =
        req.webhook_details
            .as_ref()
            .map(|webhook_details| {
                utils::Encode::<api::WebhookDetails>::encode_to_value(webhook_details)
                    .change_context(errors::ApiErrorResponse::InvalidDataValue {
                        field_name: "webhook details",
                    })
            })
            .transpose()?;

    if let Some(ref routing_algorithm) = req.routing_algorithm {
        let _: api::RoutingAlgorithm = routing_algorithm
            .clone()
            .parse_value("RoutingAlgorithm")
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "routing_algorithm",
            })
            .attach_printable("Invalid routing algorithm given")?;
    }

    let key_store = domain::MerchantKeyStore {
        merchant_id: req.merchant_id.clone(),
        key: domain_types::encrypt(key.to_vec().into(), master_key)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to decrypt data from key store")?,
        created_at: date_time::now(),
    };

    let enable_payment_response_hash = req.enable_payment_response_hash.unwrap_or(true);

    let payment_response_hash_key = req
        .payment_response_hash_key
        .or(Some(generate_cryptographically_secure_random_string(64)));

    db.insert_merchant_key_store(key_store.clone(), &master_key.to_vec().into())
        .await
        .to_duplicate_response(errors::ApiErrorResponse::DuplicateMerchantAccount)?;

    let parent_merchant_id = get_parent_merchant(
        db,
        req.sub_merchants_enabled,
        req.parent_merchant_id,
        &key_store,
    )
    .await?;

    let merchant_account = async {
        Ok(domain::MerchantAccount {
            merchant_id: req.merchant_id,
            merchant_name: req
                .merchant_name
                .async_lift(|inner| domain_types::encrypt_optional(inner, &key))
                .await?,
            merchant_details: merchant_details
                .async_lift(|inner| domain_types::encrypt_optional(inner, &key))
                .await?,
            return_url: req.return_url.map(|a| a.to_string()),
            webhook_details,
            routing_algorithm: req.routing_algorithm,
            sub_merchants_enabled: req.sub_merchants_enabled,
            parent_merchant_id,
            enable_payment_response_hash,
            payment_response_hash_key,
            redirect_to_merchant_with_http_post: req
                .redirect_to_merchant_with_http_post
                .unwrap_or_default(),
            publishable_key,
            locker_id: req.locker_id,
            metadata: req.metadata,
            storage_scheme: diesel_models::enums::MerchantStorageScheme::PostgresOnly,
            primary_business_details,
            created_at: date_time::now(),
            modified_at: date_time::now(),
            frm_routing_algorithm: req.frm_routing_algorithm,
            intent_fulfillment_time: req.intent_fulfillment_time.map(i64::from),
            id: None,
            organization_id: req.organization_id,
        })
    }
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let merchant_account = db
        .insert_merchant(merchant_account, &key_store)
        .await
        .to_duplicate_response(errors::ApiErrorResponse::DuplicateMerchantAccount)?;
    Ok(service_api::ApplicationResponse::Json(
        merchant_account
            .try_into()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed while generating response")?,
    ))
}

pub async fn get_merchant_account(
    db: &dyn StorageInterface,
    req: api::MerchantId,
) -> RouterResponse<api::MerchantAccountResponse> {
    let key_store = db
        .get_merchant_key_store_by_merchant_id(
            &req.merchant_id,
            &db.get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let merchant_account = db
        .find_merchant_account_by_merchant_id(&req.merchant_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    Ok(service_api::ApplicationResponse::Json(
        merchant_account
            .try_into()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to construct response")?,
    ))
}
pub async fn merchant_account_update(
    db: &dyn StorageInterface,
    merchant_id: &String,
    req: api::MerchantAccountUpdate,
) -> RouterResponse<api::MerchantAccountResponse> {
    let key_store = db
        .get_merchant_key_store_by_merchant_id(
            &req.merchant_id,
            &db.get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    if &req.merchant_id != merchant_id {
        Err(report!(errors::ValidationError::IncorrectValueProvided {
            field_name: "parent_merchant_id"
        })
        .attach_printable(
            "If `sub_merchants_enabled` is true, then `parent_merchant_id` is mandatory",
        )
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "parent_merchant_id",
        }))?;
    }

    if let Some(ref routing_algorithm) = req.routing_algorithm {
        let _: api::RoutingAlgorithm = routing_algorithm
            .clone()
            .parse_value("RoutingAlgorithm")
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "routing_algorithm",
            })
            .attach_printable("Invalid routing algorithm given")?;
    }

    let primary_business_details = req
        .primary_business_details
        .as_ref()
        .map(|primary_business_details| {
            utils::Encode::<Vec<PrimaryBusinessDetails>>::encode_to_value(primary_business_details)
                .change_context(errors::ApiErrorResponse::InvalidDataValue {
                    field_name: "primary_business_details",
                })
        })
        .transpose()?;

    let key = key_store.key.get_inner().peek();

    let updated_merchant_account = storage::MerchantAccountUpdate::Update {
        merchant_name: req
            .merchant_name
            .map(masking::Secret::new)
            .async_lift(|inner| domain_types::encrypt_optional(inner, key))
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to encrypt merchant name")?,

        merchant_details: req
            .merchant_details
            .as_ref()
            .map(utils::Encode::<api::MerchantDetails>::encode_to_value)
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to convert merchant_details to a value")?
            .map(masking::Secret::new)
            .async_lift(|inner| domain_types::encrypt_optional(inner, key))
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to encrypt merchant details")?,

        return_url: req.return_url.map(|a| a.to_string()),

        webhook_details: req
            .webhook_details
            .as_ref()
            .map(utils::Encode::<api::WebhookDetails>::encode_to_value)
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)?,

        routing_algorithm: req.routing_algorithm,
        sub_merchants_enabled: req.sub_merchants_enabled,

        parent_merchant_id: get_parent_merchant(
            db,
            req.sub_merchants_enabled,
            req.parent_merchant_id,
            &key_store,
        )
        .await?,
        enable_payment_response_hash: req.enable_payment_response_hash,
        payment_response_hash_key: req.payment_response_hash_key,
        redirect_to_merchant_with_http_post: req.redirect_to_merchant_with_http_post,
        locker_id: req.locker_id,
        metadata: req.metadata,
        publishable_key: None,
        primary_business_details,
        frm_routing_algorithm: req.frm_routing_algorithm,
        intent_fulfillment_time: req.intent_fulfillment_time.map(i64::from),
    };

    let response = db
        .update_specific_fields_in_merchant(merchant_id, updated_merchant_account, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    Ok(service_api::ApplicationResponse::Json(
        response
            .try_into()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed while generating response")?,
    ))
}

pub async fn merchant_account_delete(
    db: &dyn StorageInterface,
    merchant_id: String,
) -> RouterResponse<api::MerchantAccountDeleteResponse> {
    let is_deleted = db
        .delete_merchant_account_by_merchant_id(&merchant_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;
    let response = api::MerchantAccountDeleteResponse {
        merchant_id,
        deleted: is_deleted,
    };
    Ok(service_api::ApplicationResponse::Json(response))
}

async fn get_parent_merchant(
    db: &dyn StorageInterface,
    sub_merchants_enabled: Option<bool>,
    parent_merchant: Option<String>,
    key_store: &domain::MerchantKeyStore,
) -> RouterResult<Option<String>> {
    Ok(match sub_merchants_enabled {
        Some(true) => {
            Some(
                parent_merchant.ok_or_else(|| {
                    report!(errors::ValidationError::MissingRequiredField {
                        field_name: "parent_merchant_id".to_string()
                    })
                    .change_context(errors::ApiErrorResponse::PreconditionFailed {
                        message: "If `sub_merchants_enabled` is `true`, then `parent_merchant_id` is mandatory".to_string(),
                    })
                })
                .map(|id| validate_merchant_id(db, id,key_store).change_context(
                    errors::ApiErrorResponse::InvalidDataValue { field_name: "parent_merchant_id" }
                ))?
                .await?
                .merchant_id
            )
        }
        _ => None,
    })
}

async fn validate_merchant_id<S: Into<String>>(
    db: &dyn StorageInterface,
    merchant_id: S,
    key_store: &domain::MerchantKeyStore,
) -> RouterResult<domain::MerchantAccount> {
    db.find_merchant_account_by_merchant_id(&merchant_id.into(), key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
}

fn get_business_details_wrapper(
    request: &api::MerchantConnectorCreate,
    _merchant_account: &domain::MerchantAccount,
) -> RouterResult<(enums::CountryAlpha2, String)> {
    #[cfg(feature = "multiple_mca")]
    {
        // The fields are mandatory
        Ok((request.business_country, request.business_label.to_owned()))
    }

    #[cfg(not(feature = "multiple_mca"))]
    {
        // If the value is not passed, then take it from Merchant account
        helpers::get_business_details(
            request.business_country,
            request.business_label.as_ref(),
            _merchant_account,
        )
    }
}

fn validate_certificate_in_mca_metadata(
    connector_metadata: Secret<serde_json::Value>,
) -> RouterResult<()> {
    let parsed_connector_metadata = connector_metadata
        .parse_value::<api_models::payments::ConnectorMetadata>("ConnectorMetadata")
        .change_context(errors::ParsingError::StructParseFailure("Metadata"))
        .change_context(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "metadata".to_string(),
            expected_format: "connector metadata".to_string(),
        })?;

    parsed_connector_metadata
        .apple_pay
        .and_then(|applepay_metadata| {
            applepay_metadata
                .session_token_data
                .map(|session_token_data| {
                    let api_models::payments::SessionTokenInfo {
                        certificate,
                        certificate_keys,
                        ..
                    } = session_token_data;

                    helpers::create_identity_from_certificate_and_key(certificate, certificate_keys)
                        .change_context(errors::ApiErrorResponse::InvalidDataValue {
                            field_name: "certificate/certificate key",
                        })
                        .map(|_identity_result| ())
                })
        })
        .transpose()?;

    Ok(())
}

pub async fn create_payment_connector(
    store: &dyn StorageInterface,
    req: api::MerchantConnectorCreate,
    merchant_id: &String,
) -> RouterResponse<api_models::admin::MerchantConnectorResponse> {
    let key_store = store
        .get_merchant_key_store_by_merchant_id(merchant_id, &store.get_master_key().to_vec().into())
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    req.metadata
        .clone()
        .map(validate_certificate_in_mca_metadata)
        .transpose()?;

    let merchant_account = store
        .find_merchant_account_by_merchant_id(merchant_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let (business_country, business_label) = get_business_details_wrapper(&req, &merchant_account)?;

    let connector_label = helpers::get_connector_label(
        business_country,
        &business_label,
        req.business_sub_label.as_ref(),
        &req.connector_name.to_string(),
    );

    let mut vec = Vec::new();
    let payment_methods_enabled = match req.payment_methods_enabled {
        Some(val) => {
            for pm in val.into_iter() {
                let pm_value = utils::Encode::<api::PaymentMethodsEnabled>::encode_to_value(&pm)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "Failed while encoding to serde_json::Value, PaymentMethod",
                    )?;
                vec.push(pm_value)
            }
            Some(vec)
        }
        None => None,
    };

    // Validate Merchant api details and return error if not in correct format
    let auth: types::ConnectorAuthType = req
        .connector_account_details
        .clone()
        .parse_value("ConnectorAuthType")
        .change_context(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "connector_account_details".to_string(),
            expected_format: "auth_type and api_key".to_string(),
        })?;
    let conn_name = ConnectorData::convert_connector(&req.connector_name.to_string())?;
    conn_name.validate_auth_type(&auth).change_context(
        errors::ApiErrorResponse::InvalidRequestData {
            message: "The auth type is not supported for connector".to_string(),
        },
    )?;
    let frm_configs = match req.frm_configs {
        Some(frm_value) => {
            let configs_for_frm_value: serde_json::Value =
                utils::Encode::<api_models::admin::FrmConfigs>::encode_to_value(&frm_value)
                    .change_context(errors::ApiErrorResponse::ConfigNotFound)?;
            Some(Secret::new(configs_for_frm_value))
        }
        None => None,
    };

    let merchant_connector_account = domain::MerchantConnectorAccount {
        merchant_id: merchant_id.to_string(),
        connector_type: req.connector_type,
        connector_name: req.connector_name.to_string(),
        merchant_connector_id: utils::generate_id(consts::ID_LENGTH, "mca"),
        connector_account_details: domain_types::encrypt(
            req.connector_account_details.ok_or(
                errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "connector_account_details",
                },
            )?,
            key_store.key.peek(),
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to encrypt connector account details")?,
        payment_methods_enabled,
        test_mode: req.test_mode,
        disabled: req.disabled,
        metadata: req.metadata,
        frm_configs,
        connector_label: connector_label.clone(),
        business_country,
        business_label,
        business_sub_label: req.business_sub_label,
        created_at: common_utils::date_time::now(),
        modified_at: common_utils::date_time::now(),
        id: None,
        connector_webhook_details: match req.connector_webhook_details {
            Some(connector_webhook_details) => {
                Encode::<api_models::admin::MerchantConnectorWebhookDetails>::encode_to_value(
                    &connector_webhook_details,
                )
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(format!("Failed to serialize api_models::admin::MerchantConnectorWebhookDetails for Merchant: {}", merchant_id))
                .map(Some)?
                .map(masking::Secret::new)
            }
            None => None,
        },
    };

    let mca = store
        .insert_merchant_connector_account(merchant_connector_account, &key_store)
        .await
        .to_duplicate_response(
            errors::ApiErrorResponse::DuplicateMerchantConnectorAccount {
                connector_label: connector_label.clone(),
            },
        )?;

    metrics::MCA_CREATE.add(
        &metrics::CONTEXT,
        1,
        &[
            metrics::request::add_attributes("connector", req.connector_name.to_string()),
            metrics::request::add_attributes("merchant", merchant_id.to_string()),
        ],
    );

    let mca_response = mca.try_into()?;
    Ok(service_api::ApplicationResponse::Json(mca_response))
}

pub async fn retrieve_payment_connector(
    store: &dyn StorageInterface,
    merchant_id: String,
    merchant_connector_id: String,
) -> RouterResponse<api_models::admin::MerchantConnectorResponse> {
    let key_store = store
        .get_merchant_key_store_by_merchant_id(
            &merchant_id,
            &store.get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let _merchant_account = store
        .find_merchant_account_by_merchant_id(&merchant_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let mca = store
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            &merchant_id,
            &merchant_connector_id,
            &key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: merchant_connector_id.clone(),
        })?;

    Ok(service_api::ApplicationResponse::Json(mca.try_into()?))
}

pub async fn list_payment_connectors(
    store: &dyn StorageInterface,
    merchant_id: String,
) -> RouterResponse<Vec<api_models::admin::MerchantConnectorResponse>> {
    let key_store = store
        .get_merchant_key_store_by_merchant_id(
            &merchant_id,
            &store.get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    // Validate merchant account
    store
        .find_merchant_account_by_merchant_id(&merchant_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let merchant_connector_accounts = store
        .find_merchant_connector_account_by_merchant_id_and_disabled_list(
            &merchant_id,
            true,
            &key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::InternalServerError)?;
    let mut response = vec![];

    // The can be eliminated once [#79711](https://github.com/rust-lang/rust/issues/79711) is stabilized
    for mca in merchant_connector_accounts.into_iter() {
        response.push(mca.try_into()?);
    }

    Ok(service_api::ApplicationResponse::Json(response))
}

pub async fn update_payment_connector(
    db: &dyn StorageInterface,
    merchant_id: &str,
    merchant_connector_id: &str,
    req: api_models::admin::MerchantConnectorUpdate,
) -> RouterResponse<api_models::admin::MerchantConnectorResponse> {
    let key_store = db
        .get_merchant_key_store_by_merchant_id(merchant_id, &db.get_master_key().to_vec().into())
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let _merchant_account = db
        .find_merchant_account_by_merchant_id(merchant_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let mca = db
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            merchant_id,
            merchant_connector_id,
            &key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: merchant_connector_id.to_string(),
        })?;

    let payment_methods_enabled = req.payment_methods_enabled.map(|pm_enabled| {
        pm_enabled
            .iter()
            .flat_map(|payment_method| {
                utils::Encode::<api::PaymentMethodsEnabled>::encode_to_value(payment_method)
            })
            .collect::<Vec<serde_json::Value>>()
    });

    let frm_configs = match req.frm_configs.as_ref() {
        Some(frm_value) => {
            let configs_for_frm_value: serde_json::Value =
                utils::Encode::<api_models::admin::FrmConfigs>::encode_to_value(&frm_value)
                    .change_context(errors::ApiErrorResponse::ConfigNotFound)?;
            Some(Secret::new(configs_for_frm_value))
        }
        None => None,
    };

    let payment_connector = storage::MerchantConnectorAccountUpdate::Update {
        merchant_id: None,
        connector_type: Some(req.connector_type),
        connector_name: None,
        merchant_connector_id: None,
        connector_account_details: req
            .connector_account_details
            .async_lift(|inner| {
                domain_types::encrypt_optional(inner, key_store.key.get_inner().peek())
            })
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed while encrypting data")?,
        test_mode: mca.test_mode,
        disabled: mca.disabled,
        payment_methods_enabled,
        metadata: req.metadata,
        frm_configs,
        connector_webhook_details: match &req.connector_webhook_details {
            Some(connector_webhook_details) => {
                Encode::<api_models::admin::MerchantConnectorWebhookDetails>::encode_to_value(
                    connector_webhook_details,
                )
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .map(Some)?
                .map(masking::Secret::new)
            }
            None => None,
        },
    };

    let updated_mca = db
        .update_merchant_connector_account(mca, payment_connector.into(), &key_store)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!("Failed while updating MerchantConnectorAccount: id: {merchant_connector_id}")
        })?;

    let response = updated_mca.try_into()?;

    Ok(service_api::ApplicationResponse::Json(response))
}

pub async fn delete_payment_connector(
    db: &dyn StorageInterface,
    merchant_id: String,
    merchant_connector_id: String,
) -> RouterResponse<api::MerchantConnectorDeleteResponse> {
    let key_store = db
        .get_merchant_key_store_by_merchant_id(&merchant_id, &db.get_master_key().to_vec().into())
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let _merchant_account = db
        .find_merchant_account_by_merchant_id(&merchant_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let _mca = db
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            &merchant_id,
            &merchant_connector_id,
            &key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: merchant_connector_id.clone(),
        })?;

    let is_deleted = db
        .delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
            &merchant_id,
            &merchant_connector_id,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: merchant_connector_id.clone(),
        })?;

    let response = api::MerchantConnectorDeleteResponse {
        merchant_id,
        merchant_connector_id,
        deleted: is_deleted,
    };
    Ok(service_api::ApplicationResponse::Json(response))
}

pub async fn kv_for_merchant(
    db: &dyn StorageInterface,
    merchant_id: String,
    enable: bool,
) -> RouterResponse<api_models::admin::ToggleKVResponse> {
    let key_store = db
        .get_merchant_key_store_by_merchant_id(&merchant_id, &db.get_master_key().to_vec().into())
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    // check if the merchant account exists
    let merchant_account = db
        .find_merchant_account_by_merchant_id(&merchant_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let updated_merchant_account = match (enable, merchant_account.storage_scheme) {
        (true, enums::MerchantStorageScheme::RedisKv)
        | (false, enums::MerchantStorageScheme::PostgresOnly) => Ok(merchant_account),
        (true, enums::MerchantStorageScheme::PostgresOnly) => {
            db.update_merchant(
                merchant_account,
                storage::MerchantAccountUpdate::StorageSchemeUpdate {
                    storage_scheme: enums::MerchantStorageScheme::RedisKv,
                },
                &key_store,
            )
            .await
        }
        (false, enums::MerchantStorageScheme::RedisKv) => {
            db.update_merchant(
                merchant_account,
                storage::MerchantAccountUpdate::StorageSchemeUpdate {
                    storage_scheme: enums::MerchantStorageScheme::PostgresOnly,
                },
                &key_store,
            )
            .await
        }
    }
    .map_err(|error| {
        error
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("failed to switch merchant_storage_scheme")
    })?;
    let kv_status = matches!(
        updated_merchant_account.storage_scheme,
        enums::MerchantStorageScheme::RedisKv
    );

    Ok(service_api::ApplicationResponse::Json(
        api_models::admin::ToggleKVResponse {
            merchant_id: updated_merchant_account.merchant_id,
            kv_enabled: kv_status,
        },
    ))
}

pub async fn check_merchant_account_kv_status(
    db: &dyn StorageInterface,
    merchant_id: String,
) -> RouterResponse<api_models::admin::ToggleKVResponse> {
    let key_store = db
        .get_merchant_key_store_by_merchant_id(&merchant_id, &db.get_master_key().to_vec().into())
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    // check if the merchant account exists
    let merchant_account = db
        .find_merchant_account_by_merchant_id(&merchant_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let kv_status = matches!(
        merchant_account.storage_scheme,
        enums::MerchantStorageScheme::RedisKv
    );

    Ok(service_api::ApplicationResponse::Json(
        api_models::admin::ToggleKVResponse {
            merchant_id: merchant_account.merchant_id,
            kv_enabled: kv_status,
        },
    ))
}
