use common_utils::{
    crypto::OptionalSecretValue,
    date_time,
    ext_traits::{AsyncExt, ValueExt},
};
use error_stack::{report, FutureExt, IntoReport, ResultExt};
use masking::PeekInterface;
use storage_models::enums;
use uuid::Uuid;

use crate::{
    consts,
    core::{
        api_keys,
        errors::{self, RouterResponse, RouterResult, StorageErrorExt},
    },
    db::StorageInterface,
    routes::AppState,
    services::{self, api as service_api},
    types::{
        self, api,
        domain::{
            self, merchant_account as merchant_domain, merchant_key_store,
            types::{self as domain_types, AsyncLift},
        },
        storage,
        transformers::ForeignInto,
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

pub async fn create_merchant_account(
    state: &AppState,
    req: api::MerchantAccountCreate,
) -> RouterResponse<api::MerchantAccountResponse> {
    let db = &*state.store;
    let master_key = db.get_master_key();

    let key = services::generate_aes256_key()
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let publishable_key = Some(create_merchant_publishable_key());

    let api_key_request = api::CreateApiKeyRequest {
        name: "Default API key".into(),
        description: Some(
            "An API key created by default when a user signs up on the HyperSwitch dashboard"
                .into(),
        ),
        expiration: api::ApiKeyExpiration::Never,
    };

    let api_key = match api_keys::create_api_key(
        db,
        &state.conf.api_keys,
        #[cfg(feature = "kms")]
        &state.conf.kms,
        api_key_request,
        req.merchant_id.clone(),
    )
    .await?
    {
        service_api::ApplicationResponse::Json(api::CreateApiKeyResponse { api_key, .. }) => {
            Ok(api_key)
        }
        _ => Err(errors::ApiErrorResponse::InternalServerError)
            .into_report()
            .attach_printable("Unexpected create API key response"),
    }?;

    let merchant_details: OptionalSecretValue = Some(
        utils::Encode::<api::MerchantDetails>::encode_to_value(&req.merchant_details)
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "merchant_details",
            })?
            .into(),
    );

    let webhook_details = Some(
        utils::Encode::<api::WebhookDetails>::encode_to_value(&req.webhook_details)
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "webhook details",
            })?,
    );

    if let Some(ref routing_algorithm) = req.routing_algorithm {
        let _: api::RoutingAlgorithm = routing_algorithm
            .clone()
            .parse_value("RoutingAlgorithm")
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "routing_algorithm",
            })
            .attach_printable("Invalid routing algorithm given")?;
    }

    let key_store = merchant_key_store::MerchantKeyStore {
        merchant_id: req.merchant_id.clone(),
        key: domain_types::encrypt(key.to_vec().into(), master_key)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to decrypt data from key store")?,
        created_at: date_time::now(),
    };

    db.insert_merchant_key_store(key_store)
        .await
        .map_err(|error| {
            error.to_duplicate_response(errors::ApiErrorResponse::DuplicateMerchantAccount)
        })?;

    let encrypt_string = |inner: Option<masking::Secret<String>>| async {
        inner
            .async_map(|value| domain_types::encrypt(value, &key))
            .await
            .transpose()
    };

    let encrypt_value = |inner: OptionalSecretValue| async {
        inner
            .async_map(|value| domain_types::encrypt(value, &key))
            .await
            .transpose()
    };

    let parent_merchant_id =
        get_parent_merchant(db, req.sub_merchants_enabled, req.parent_merchant_id).await?;

    let merchant_account = async {
        Ok(merchant_domain::MerchantAccount {
            merchant_id: req.merchant_id,
            merchant_name: req.merchant_name.async_lift(encrypt_string).await?,
            api_key: Some(domain_types::encrypt(api_key.peek().clone().into(), &key).await?),
            merchant_details: merchant_details.async_lift(encrypt_value).await?,
            return_url: req.return_url.map(|a| a.to_string()),
            webhook_details,
            routing_algorithm: req.routing_algorithm,
            sub_merchants_enabled: req.sub_merchants_enabled,
            parent_merchant_id,
            enable_payment_response_hash: req.enable_payment_response_hash.unwrap_or_default(),
            payment_response_hash_key: req.payment_response_hash_key,
            redirect_to_merchant_with_http_post: req
                .redirect_to_merchant_with_http_post
                .unwrap_or_default(),
            publishable_key,
            locker_id: req.locker_id,
            metadata: req.metadata,
            storage_scheme: storage_models::enums::MerchantStorageScheme::PostgresOnly,
            id: None,
        })
    }
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let merchant_account = db
        .insert_merchant(merchant_account)
        .await
        .map_err(|error| {
            error.to_duplicate_response(errors::ApiErrorResponse::DuplicateMerchantAccount)
        })?;
    Ok(service_api::ApplicationResponse::Json(
        merchant_account.into(),
    ))
}

pub async fn get_merchant_account(
    db: &dyn StorageInterface,
    req: api::MerchantId,
) -> RouterResponse<api::MerchantAccountResponse> {
    let merchant_account = db
        .find_merchant_account_by_merchant_id(&req.merchant_id)
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
        })?;

    Ok(service_api::ApplicationResponse::Json(
        merchant_account.into(),
    ))
}

pub async fn merchant_account_update(
    db: &dyn StorageInterface,
    merchant_id: &String,
    req: api::MerchantAccountUpdate,
) -> RouterResponse<api::MerchantAccountResponse> {
    let key = domain_types::get_merchant_enc_key(db, merchant_id.clone())
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

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

    let updated_merchant_account = storage::MerchantAccountUpdate::Update {
        merchant_name: req
            .merchant_name
            .async_map(|inner| domain_types::encrypt(inner.into(), &key))
            .await
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)?,

        merchant_details: req
            .merchant_details
            .as_ref()
            .map(utils::Encode::<api::MerchantDetails>::encode_to_value)
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)?
            .async_map(|inner| domain_types::encrypt(inner.into(), &key))
            .await
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)?,

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
        )
        .await?,
        enable_payment_response_hash: req.enable_payment_response_hash,
        payment_response_hash_key: req.payment_response_hash_key,
        redirect_to_merchant_with_http_post: req.redirect_to_merchant_with_http_post,
        locker_id: req.locker_id,
        metadata: req.metadata,
        publishable_key: None,
    };

    let response = db
        .update_specific_fields_in_merchant(merchant_id, updated_merchant_account)
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
        })?;

    Ok(service_api::ApplicationResponse::Json(response.into()))
}

pub async fn merchant_account_delete(
    db: &dyn StorageInterface,
    merchant_id: String,
) -> RouterResponse<api::MerchantAccountDeleteResponse> {
    let is_deleted = db
        .delete_merchant_account_by_merchant_id(&merchant_id)
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
        })?;
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
                .map(|id| validate_merchant_id(db, id).change_context(
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
) -> RouterResult<merchant_domain::MerchantAccount> {
    db.find_merchant_account_by_merchant_id(&merchant_id.into())
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
        })
}

// Merchant Connector API -  Every merchant and connector can have an instance of (merchant <> connector)
//                          with unique merchant_connector_id for Create Operation

pub async fn create_payment_connector(
    store: &dyn StorageInterface,
    req: api::MerchantConnector,
    merchant_id: &String,
) -> RouterResponse<api::MerchantConnector> {
    let key = domain_types::get_merchant_enc_key(store, merchant_id.clone())
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let _merchant_account = store
        .find_merchant_account_by_merchant_id(merchant_id)
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
        })?;

    let mut vec = Vec::new();
    let mut response = req.clone();
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
    let _: types::ConnectorAuthType = req
        .connector_account_details
        .clone()
        .parse_value("ConnectorAuthType")
        .change_context(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "connector_account_details".to_string(),
            expected_format: "auth_type and api_key".to_string(),
        })?;

    let merchant_connector_account = domain::merchant_connector_account::MerchantConnectorAccount {
        merchant_id: merchant_id.to_string(),
        connector_type: req.connector_type.foreign_into(),
        connector_name: req.connector_name,
        merchant_connector_id: utils::generate_id(consts::ID_LENGTH, "mca"),
        connector_account_details: domain_types::encrypt(
            req.connector_account_details.ok_or(
                errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "connector_account_details",
                },
            )?,
            &key,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?,
        payment_methods_enabled,
        test_mode: req.test_mode,
        disabled: req.disabled,
        metadata: req.metadata,
        id: None,
    };

    let mca = store
        .insert_merchant_connector_account(merchant_connector_account)
        .await
        .map_err(|error| {
            error.to_duplicate_response(errors::ApiErrorResponse::DuplicateMerchantConnectorAccount)
        })?;

    response.merchant_connector_id = Some(mca.merchant_connector_id);
    Ok(service_api::ApplicationResponse::Json(response))
}

pub async fn retrieve_payment_connector(
    store: &dyn StorageInterface,
    merchant_id: String,
    merchant_connector_id: String,
) -> RouterResponse<api::MerchantConnector> {
    let _merchant_account = store
        .find_merchant_account_by_merchant_id(&merchant_id)
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
        })?;

    let mca = store
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            &merchant_id,
            &merchant_connector_id,
        )
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound)
        })?;

    Ok(service_api::ApplicationResponse::Json(mca.try_into()?))
}

pub async fn list_payment_connectors(
    store: &dyn StorageInterface,
    merchant_id: String,
) -> RouterResponse<Vec<api::MerchantConnector>> {
    // Validate merchant account
    store
        .find_merchant_account_by_merchant_id(&merchant_id)
        .await
        .map_err(|err| {
            err.to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
        })?;

    let merchant_connector_accounts = store
        .find_merchant_connector_account_by_merchant_id_and_disabled_list(&merchant_id, true)
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound)
        })?;
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
    req: api::MerchantConnector,
) -> RouterResponse<api::MerchantConnector> {
    let key = domain_types::get_merchant_enc_key(db, merchant_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let _merchant_account = db
        .find_merchant_account_by_merchant_id(merchant_id)
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
        })?;

    let mca = db
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            merchant_id,
            merchant_connector_id,
        )
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound)
        })?;

    let payment_methods_enabled = req.payment_methods_enabled.map(|pm_enabled| {
        pm_enabled
            .iter()
            .flat_map(|payment_method| {
                utils::Encode::<api::PaymentMethodsEnabled>::encode_to_value(payment_method)
            })
            .collect::<Vec<serde_json::Value>>()
    });

    let encrypt = |inner: Option<masking::Secret<serde_json::Value>>| async {
        inner
            .async_map(|inner| domain_types::encrypt(inner, &key))
            .await
            .transpose()
    };

    let payment_connector = storage::MerchantConnectorAccountUpdate::Update {
        merchant_id: Some(merchant_id.to_string()),
        connector_type: Some(req.connector_type.foreign_into()),
        connector_name: Some(req.connector_name),
        merchant_connector_id: Some(merchant_connector_id.to_string()),
        connector_account_details: req
            .connector_account_details
            .async_lift(encrypt)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?,
        payment_methods_enabled,
        test_mode: req.test_mode,
        disabled: req.disabled,
        metadata: req.metadata,
    };

    let updated_mca = db
        .update_merchant_connector_account(mca, payment_connector.into())
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!("Failed while updating MerchantConnectorAccount: id: {merchant_connector_id}")
        })?;

    let updated_pm_enabled = updated_mca.payment_methods_enabled.map(|pm| {
        pm.into_iter()
            .flat_map(|pm_value| {
                ValueExt::<api_models::admin::PaymentMethodsEnabled>::parse_value(
                    pm_value,
                    "PaymentMethods",
                )
            })
            .collect::<Vec<api_models::admin::PaymentMethodsEnabled>>()
    });

    let response = api::MerchantConnector {
        connector_type: updated_mca.connector_type.foreign_into(),
        connector_name: updated_mca.connector_name,
        merchant_connector_id: Some(updated_mca.merchant_connector_id),
        connector_account_details: Some(updated_mca.connector_account_details.into_inner()),
        test_mode: updated_mca.test_mode,
        disabled: updated_mca.disabled,
        payment_methods_enabled: updated_pm_enabled,
        metadata: updated_mca.metadata,
    };
    Ok(service_api::ApplicationResponse::Json(response))
}

pub async fn delete_payment_connector(
    db: &dyn StorageInterface,
    merchant_id: String,
    merchant_connector_id: String,
) -> RouterResponse<api::MerchantConnectorDeleteResponse> {
    let _merchant_account = db
        .find_merchant_account_by_merchant_id(&merchant_id)
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
        })?;

    let is_deleted = db
        .delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
            &merchant_id,
            &merchant_connector_id,
        )
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound)
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
    // check if the merchant account exists
    let merchant_account = db
        .find_merchant_account_by_merchant_id(&merchant_id)
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
        })?;

    let updated_merchant_account = match (enable, merchant_account.storage_scheme) {
        (true, enums::MerchantStorageScheme::RedisKv)
        | (false, enums::MerchantStorageScheme::PostgresOnly) => Ok(merchant_account),
        (true, enums::MerchantStorageScheme::PostgresOnly) => {
            db.update_merchant(
                merchant_account,
                storage::MerchantAccountUpdate::StorageSchemeUpdate {
                    storage_scheme: enums::MerchantStorageScheme::RedisKv,
                },
            )
            .await
        }
        (false, enums::MerchantStorageScheme::RedisKv) => {
            db.update_merchant(
                merchant_account,
                storage::MerchantAccountUpdate::StorageSchemeUpdate {
                    storage_scheme: enums::MerchantStorageScheme::PostgresOnly,
                },
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
    // check if the merchant account exists
    let merchant_account = db
        .find_merchant_account_by_merchant_id(&merchant_id)
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
        })?;

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
