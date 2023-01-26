use common_utils::ext_traits::ValueExt;
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
    utils::{self, OptionExt},
};

#[inline]
pub fn create_merchant_api_key() -> String {
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
    let publishable_key = Some(format!("pk_{}", create_merchant_api_key()));

    let api_key = Some(create_merchant_api_key().into());

    let merchant_details = Some(
        utils::Encode::<api::MerchantDetails>::encode_to_value(&req.merchant_details)
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "merchant_details",
            })?,
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

    let merchant_account = storage::MerchantAccountNew {
        merchant_id: req.merchant_id,
        merchant_name: req.merchant_name,
        api_key,
        merchant_details,
        return_url: req.return_url,
        webhook_details,
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
        publishable_key,
        locker_id: req.locker_id,
        metadata: req.metadata,
    };

    let merchant_account = db
        .insert_merchant(merchant_account)
        .await
        .map_err(|error| {
            error.to_duplicate_response(errors::ApiErrorResponse::DuplicateMerchantAccount)
        })?;

    Ok(service_api::ApplicationResponse::Json(
        merchant_account.foreign_into(),
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
        merchant_account.foreign_into(),
    ))
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

    let updated_merchant_account = storage::MerchantAccountUpdate::Update {
        merchant_name: req.merchant_name,

        merchant_details: req
            .merchant_details
            .as_ref()
            .map(utils::Encode::<api::MerchantDetails>::encode_to_value)
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)?,

        return_url: req.return_url,

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
            req.sub_merchants_enabled
                .or(merchant_account.sub_merchants_enabled),
            req.parent_merchant_id
                .or_else(|| merchant_account.parent_merchant_id.clone()),
        )
        .await?,
        enable_payment_response_hash: req.enable_payment_response_hash,
        payment_response_hash_key: req.payment_response_hash_key,
        redirect_to_merchant_with_http_post: req.redirect_to_merchant_with_http_post,
        locker_id: req.locker_id,
        metadata: req.metadata,
        merchant_id: merchant_account.merchant_id.to_owned(),
        api_key: None,
        publishable_key: None,
    };

    let response = db
        .update_merchant(merchant_account, updated_merchant_account)
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
        })?;

    Ok(service_api::ApplicationResponse::Json(
        response.foreign_into(),
    ))
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

    let payment_methods_enabled = req.payment_methods_enabled.map(|pm_enabled| {
        pm_enabled
            .iter()
            .flat_map(|payment_method| {
                utils::Encode::<api::PaymentMethods>::encode_to_value(payment_method)
            })
            .collect::<Vec<serde_json::Value>>()
    });

    let payment_connector = storage::MerchantConnectorAccountUpdate::Update {
        merchant_id: Some(merchant_id.to_string()),
        connector_type: Some(req.connector_type.foreign_into()),
        connector_name: Some(req.connector_name),
        merchant_connector_id: Some(merchant_connector_id),
        connector_account_details: req.connector_account_details,
        payment_methods_enabled,
        test_mode: req.test_mode,
        disabled: req.disabled,
        metadata: req.metadata,
    };

    let updated_mca = db
        .update_merchant_connector_account(mca, payment_connector)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!(
                "Failed while updating MerchantConnectorAccount: id: {merchant_connector_id}"
            )
        })?;

    let updated_pm_enabled = updated_mca.payment_methods_enabled.map(|pm| {
        pm.into_iter()
            .flat_map(|pm_value| {
                ValueExt::<api_models::admin::PaymentMethods>::parse_value(
                    pm_value,
                    "PaymentMethods",
                )
            })
            .collect::<Vec<api_models::admin::PaymentMethods>>()
    });

    let response = api::PaymentConnectorCreate {
        connector_type: updated_mca.connector_type.foreign_into(),
        connector_name: updated_mca.connector_name,
        merchant_connector_id: Some(updated_mca.merchant_connector_id),
        connector_account_details: Some(Secret::new(updated_mca.connector_account_details)),
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
