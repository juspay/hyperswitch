use error_stack::{report, FutureExt, ResultExt};
use uuid::Uuid;

use crate::{
    core::errors::{self, RouterResponse, RouterResult, StorageErrorExt},
    db::StorageInterface,
    env::{self, Env},
    pii::Secret,
    services::api as service_api,
    types::{
        self, api,
        storage::{self, MerchantAccount},
        transformers::{ForeignInto, ForeignTryInto},
    },
    utils::{self, OptionExt, ValueExt},
};

#[inline]
fn create_merchant_api_key() -> String {
    let id = Uuid::new_v4().simple();
    match env::which() {
        Env::Development => format!("dev_{id}"),
        Env::Production => format!("prd_{id}"),
        Env::Sandbox => format!("snd_{id}"),
    }
}

pub async fn create_merchant_account(
    db: &dyn StorageInterface,
    req: api::CreateMerchantAccount,
) -> RouterResponse<api::MerchantAccountResponse> {
    let publishable_key = &format!("pk_{}", create_merchant_api_key());
    let api_key = create_merchant_api_key();
    let mut response = req.clone();
    response.api_key = Some(api_key.to_owned().into());
    response.publishable_key = Some(publishable_key.to_owned());
    let merchant_details =
        utils::Encode::<api::MerchantDetails>::encode_to_value(&req.merchant_details)
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "merchant_details",
            })?;
    let webhook_details =
        utils::Encode::<api::WebhookDetails>::encode_to_value(&req.webhook_details)
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "webhook details",
            })?;

    if let Some(ref routing_algorithm) = req.routing_algorithm {
        let _: api::RoutingAlgorithm = routing_algorithm
            .clone()
            .parse_value("RoutingAlgorithm")
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "routing_algorithm",
            })
            .attach_printable("Invalid routing algorithm given")?;
    }

    let merchant_account = storage::MerchantAccountNew {
        merchant_id: req.merchant_id,
        merchant_name: req.merchant_name,
        api_key: Some(api_key.to_string().into()),
        merchant_details: Some(merchant_details),
        return_url: req.return_url,
        webhook_details: Some(webhook_details),
        routing_algorithm: req.routing_algorithm,
        sub_merchants_enabled: req.sub_merchants_enabled,
        parent_merchant_id: get_parent_merchant(
            db,
            &req.sub_merchants_enabled,
            req.parent_merchant_id,
        )
        .await?,
        enable_payment_response_hash: req.enable_payment_response_hash,
        payment_response_hash_key: req.payment_response_hash_key,
        redirect_to_merchant_with_http_post: req.redirect_to_merchant_with_http_post,
        publishable_key: Some(publishable_key.to_owned()),
        locker_id: req.locker_id,
    };

    db.insert_merchant(merchant_account)
        .await
        .map_err(|error| {
            error.to_duplicate_response(errors::ApiErrorResponse::DuplicateMerchantAccount)
        })?;
    Ok(service_api::ApplicationResponse::Json(response))
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
    let merchant_details = merchant_account
        .merchant_details
        .parse_value("MerchantDetails")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let webhook_details = merchant_account
        .webhook_details
        .parse_value("WebhookDetails")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let response = api::MerchantAccountResponse {
        merchant_id: req.merchant_id,
        merchant_name: merchant_account.merchant_name,
        api_key: merchant_account.api_key,
        merchant_details,
        return_url: merchant_account.return_url,
        webhook_details,
        routing_algorithm: merchant_account.routing_algorithm,
        sub_merchants_enabled: merchant_account.sub_merchants_enabled,
        parent_merchant_id: merchant_account.parent_merchant_id,
        enable_payment_response_hash: Some(merchant_account.enable_payment_response_hash),
        payment_response_hash_key: merchant_account.payment_response_hash_key,
        redirect_to_merchant_with_http_post: Some(
            merchant_account.redirect_to_merchant_with_http_post,
        ),
        metadata: None,
        publishable_key: merchant_account.publishable_key,
        locker_id: merchant_account.locker_id,
    };
    Ok(service_api::ApplicationResponse::Json(response))
}

pub async fn merchant_account_update(
    db: &dyn StorageInterface,
    merchant_id: &String,
    req: api::CreateMerchantAccount,
) -> RouterResponse<api::MerchantAccountResponse> {
    let merchant_account = db
        .find_merchant_account_by_merchant_id(merchant_id)
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
        })?;

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

    let mut response = req.clone();

    let encode_error_handler =
        |value: &str| format!("Unable to encode to serde_json::Value, {value}");

    let updated_merchant_account = storage::MerchantAccountUpdate::Update {
        merchant_id: merchant_id.to_string(),
        merchant_name: req
            .merchant_name
            .or_else(|| merchant_account.merchant_name.to_owned()),
        api_key: merchant_account.api_key.clone(),
        merchant_details: if req.merchant_details.is_some() {
            Some(
                utils::Encode::<api::MerchantDetails>::encode_to_value(&req.merchant_details)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable_lazy(|| encode_error_handler("MerchantDetails"))?,
            )
        } else {
            merchant_account.merchant_details.to_owned()
        },
        return_url: req
            .return_url
            .or_else(|| merchant_account.return_url.to_owned()),
        webhook_details: if req.webhook_details.is_some() {
            Some(
                utils::Encode::<api::WebhookDetails>::encode_to_value(&req.webhook_details)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable_lazy(|| encode_error_handler("WebhookDetails"))?,
            )
        } else {
            merchant_account.webhook_details.to_owned()
        },
        routing_algorithm: req
            .routing_algorithm
            .or_else(|| merchant_account.routing_algorithm.clone()),
        sub_merchants_enabled: req
            .sub_merchants_enabled
            .or(merchant_account.sub_merchants_enabled),
        parent_merchant_id: get_parent_merchant(
            db,
            &req.sub_merchants_enabled
                .or(merchant_account.sub_merchants_enabled),
            req.parent_merchant_id
                .or_else(|| merchant_account.parent_merchant_id.clone()),
        )
        .await?,
        enable_payment_response_hash: req
            .enable_payment_response_hash
            .or(Some(merchant_account.enable_payment_response_hash)),
        payment_response_hash_key: req
            .payment_response_hash_key
            .or_else(|| merchant_account.payment_response_hash_key.to_owned()),
        redirect_to_merchant_with_http_post: req
            .redirect_to_merchant_with_http_post
            .or(Some(merchant_account.redirect_to_merchant_with_http_post)),
        publishable_key: req
            .publishable_key
            .or_else(|| merchant_account.publishable_key.clone()),
        locker_id: req
            .locker_id
            .or_else(|| merchant_account.locker_id.to_owned()),
    };
    response.merchant_id = merchant_id.to_string();
    response.api_key = merchant_account.api_key.to_owned();

    db.update_merchant(merchant_account, updated_merchant_account)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| format!("Failed while updating merchant: {}", merchant_id))?;
    Ok(service_api::ApplicationResponse::Json(response))
}

pub async fn merchant_account_delete(
    db: &dyn StorageInterface,
    merchant_id: String,
) -> RouterResponse<api::DeleteResponse> {
    let is_deleted = db
        .delete_merchant_account_by_merchant_id(&merchant_id)
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
        })?;
    let response = api::DeleteResponse {
        merchant_id,
        deleted: is_deleted,
    };
    Ok(service_api::ApplicationResponse::Json(response))
}

async fn get_parent_merchant(
    db: &dyn StorageInterface,
    sub_merchants_enabled: &Option<bool>,
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
                ))?.await?.merchant_id
            )
        }
        _ => None,
    })
}

async fn validate_merchant_id<S: Into<String>>(
    db: &dyn StorageInterface,
    merchant_id: S,
) -> RouterResult<MerchantAccount> {
    db.find_merchant_account_by_merchant_id(&merchant_id.into())
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
        })
}
// Payment Connector API -  Every merchant and connector can have an instance of (merchant <> connector)
//                          with unique merchant_connector_id for Create Operation

pub async fn create_payment_connector(
    store: &dyn StorageInterface,
    req: api::PaymentConnectorCreate,
    merchant_id: &String,
) -> RouterResponse<api::PaymentConnectorCreate> {
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
                let pm_value = utils::Encode::<api::PaymentMethods>::encode_to_value(&pm)
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

    let merchant_connector_account = storage::MerchantConnectorAccountNew {
        merchant_id: Some(merchant_id.to_string()),
        connector_type: Some(req.connector_type.foreign_into()),
        connector_name: Some(req.connector_name),
        merchant_connector_id: None,
        connector_account_details: req.connector_account_details,
        payment_methods_enabled,
        test_mode: req.test_mode,
        disabled: req.disabled,
        metadata: req.metadata,
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
    merchant_connector_id: i32,
) -> RouterResponse<api::PaymentConnectorCreate> {
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

    Ok(service_api::ApplicationResponse::Json(
        mca.foreign_try_into()?,
    ))
}

pub async fn list_payment_connectors(
    store: &dyn StorageInterface,
    merchant_id: String,
) -> RouterResponse<Vec<api::PaymentConnectorCreate>> {
    // Validate merchant account
    store
        .find_merchant_account_by_merchant_id(&merchant_id)
        .await
        .map_err(|err| {
            err.to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
        })?;

    let merchant_connector_accounts = store
        .find_merchant_connector_account_by_merchant_id_list(&merchant_id)
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound)
        })?;
    let mut response = vec![];

    // The can be eliminated once [#79711](https://github.com/rust-lang/rust/issues/79711) is stabilized
    for mca in merchant_connector_accounts.into_iter() {
        response.push(mca.foreign_try_into()?);
    }

    Ok(service_api::ApplicationResponse::Json(response))
}

pub async fn update_payment_connector(
    db: &dyn StorageInterface,
    merchant_id: &str,
    merchant_connector_id: i32,
    req: api::PaymentConnectorCreate,
) -> RouterResponse<api::PaymentConnectorCreate> {
    let _merchant_account = db
        .find_merchant_account_by_merchant_id(merchant_id)
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
        })?;

    let mca = db
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            merchant_id,
            &merchant_connector_id,
        )
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound)
        })?;
    let mut vec = mca.payment_methods_enabled.to_owned().unwrap_or_default();

    let payment_methods_enabled = match req.payment_methods_enabled.clone() {
        Some(val) => {
            for pm in val.into_iter() {
                let pm_value = utils::Encode::<api::PaymentMethods>::encode_to_value(&pm)
                    .change_context(errors::ApiErrorResponse::InvalidDataValue {
                        field_name: "payment method",
                    })?;
                vec.push(pm_value)
            }
            Some(vec)
        }
        None => Some(vec),
    };
    let payment_connector = storage::MerchantConnectorAccountUpdate::Update {
        merchant_id: Some(merchant_id.to_string()),
        connector_type: Some(req.connector_type.foreign_into()),
        connector_name: Some(req.connector_name),
        merchant_connector_id: Some(merchant_connector_id),
        connector_account_details: req
            .connector_account_details
            .or_else(|| Some(Secret::new(mca.connector_account_details.to_owned()))),
        payment_methods_enabled,
        test_mode: mca.test_mode,
        disabled: req.disabled.or(mca.disabled),
    };

    let updated_mca = db
        .update_merchant_connector_account(mca, payment_connector)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!(
                "Failed while updating MerchantConnectorAccount: id: {}",
                merchant_connector_id
            )
        })?;
    let response = api::PaymentConnectorCreate {
        connector_type: updated_mca.connector_type.foreign_into(),
        connector_name: updated_mca.connector_name,
        merchant_connector_id: Some(updated_mca.merchant_connector_id),
        connector_account_details: Some(Secret::new(updated_mca.connector_account_details)),
        test_mode: updated_mca.test_mode,
        disabled: updated_mca.disabled,
        payment_methods_enabled: req.payment_methods_enabled,
        metadata: req.metadata,
    };
    Ok(service_api::ApplicationResponse::Json(response))
}

pub async fn delete_payment_connector(
    db: &dyn StorageInterface,
    merchant_id: String,
    merchant_connector_id: i32,
) -> RouterResponse<api::DeleteMcaResponse> {
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
    let response = api::DeleteMcaResponse {
        merchant_id,
        merchant_connector_id,
        deleted: is_deleted,
    };
    Ok(service_api::ApplicationResponse::Json(response))
}
