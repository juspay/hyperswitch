use std::str::FromStr;

use api_models::{
    admin::{self as admin_types},
    enums as api_enums, routing as routing_types,
};
use common_utils::{
    crypto::{generate_cryptographically_secure_random_string, OptionalSecretValue},
    date_time,
    ext_traits::{AsyncExt, ConfigExt, Encode, ValueExt},
    pii,
};
use error_stack::{report, FutureExt, ResultExt};
use futures::future::try_join_all;
use masking::{PeekInterface, Secret};
use uuid::Uuid;

use crate::{
    consts,
    core::{
        errors::{self, RouterResponse, RouterResult, StorageErrorExt},
        payments::helpers,
        routing::helpers as routing_helpers,
        utils as core_utils,
    },
    db::StorageInterface,
    routes::{metrics, AppState},
    services::{self, api as service_api},
    types::{
        self, api,
        domain::{
            self,
            types::{self as domain_types, AsyncLift},
        },
        storage::{self, enums::MerchantStorageScheme},
        transformers::{ForeignFrom, ForeignTryFrom},
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
    state: AppState,
    req: api::MerchantAccountCreate,
) -> RouterResponse<api::MerchantAccountResponse> {
    let db = state.store.as_ref();
    let master_key = db.get_master_key();

    let key = services::generate_aes256_key()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to generate aes 256 key")?;

    let publishable_key = Some(create_merchant_publishable_key());

    let primary_business_details =
        utils::Encode::<Vec<admin_types::PrimaryBusinessDetails>>::encode_to_value(
            &req.primary_business_details.clone().unwrap_or_default(),
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
        let _: api_models::routing::RoutingAlgorithm = routing_algorithm
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

    let metadata = req
        .metadata
        .as_ref()
        .map(|meta| {
            utils::Encode::<admin_types::MerchantAccountMetadata>::encode_to_value(meta)
                .change_context(errors::ApiErrorResponse::InvalidDataValue {
                    field_name: "metadata",
                })
        })
        .transpose()?
        .map(Secret::new);

    let payment_link_config = req
        .payment_link_config
        .as_ref()
        .map(|pl_metadata| {
            utils::Encode::<admin_types::PaymentLinkConfig>::encode_to_value(pl_metadata)
                .change_context(errors::ApiErrorResponse::InvalidDataValue {
                    field_name: "payment_link_config",
                })
        })
        .transpose()?;

    let organization_id = if let Some(organization_id) = req.organization_id.as_ref() {
        db.find_organization_by_org_id(organization_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
                message: "organization with the given id does not exist".to_string(),
            })?;
        organization_id.to_string()
    } else {
        let new_organization = api_models::organization::OrganizationNew::new(None);
        let db_organization = ForeignFrom::foreign_from(new_organization);
        let organization = db
            .insert_organization(db_organization)
            .await
            .to_duplicate_response(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error when creating organization")?;
        organization.org_id
    };

    let mut merchant_account = async {
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
            routing_algorithm: Some(serde_json::json!({
                "algorithm_id": null,
                "timestamp": 0
            })),
            sub_merchants_enabled: req.sub_merchants_enabled,
            parent_merchant_id,
            enable_payment_response_hash,
            payment_response_hash_key,
            redirect_to_merchant_with_http_post: req
                .redirect_to_merchant_with_http_post
                .unwrap_or_default(),
            publishable_key,
            locker_id: req.locker_id,
            metadata,
            storage_scheme: MerchantStorageScheme::PostgresOnly,
            primary_business_details,
            created_at: date_time::now(),
            modified_at: date_time::now(),
            frm_routing_algorithm: req.frm_routing_algorithm,
            intent_fulfillment_time: req.intent_fulfillment_time.map(i64::from),
            payout_routing_algorithm: req.payout_routing_algorithm,
            id: None,
            organization_id,
            is_recon_enabled: false,
            default_profile: None,
            recon_status: diesel_models::enums::ReconStatus::NotRequested,
            payment_link_config,
        })
    }
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)?;

    // Create a default business profile
    // If business_labels are passed, then use it as the profile_name
    // else use `default` as the profile_name
    if let Some(business_details) = req.primary_business_details.as_ref() {
        for business_profile in business_details {
            let profile_name =
                format!("{}_{}", business_profile.country, business_profile.business);

            let business_profile_create_request = api_models::admin::BusinessProfileCreate {
                profile_name: Some(profile_name),
                ..Default::default()
            };

            let _ = create_and_insert_business_profile(
                db,
                business_profile_create_request,
                merchant_account.clone(),
            )
            .await
            .map_err(|business_profile_insert_error| {
                crate::logger::warn!(
                    "Business profile already exists {business_profile_insert_error:?}"
                );
            })
            .map(|business_profile| {
                if business_details.len() == 1 && merchant_account.default_profile.is_none() {
                    merchant_account.default_profile = Some(business_profile.profile_id);
                }
            });
        }
    } else {
        let business_profile = create_and_insert_business_profile(
            db,
            api_models::admin::BusinessProfileCreate::default(),
            merchant_account.clone(),
        )
        .await?;

        // Update merchant account with the business profile id
        merchant_account.default_profile = Some(business_profile.profile_id);
    };

    let merchant_account = db
        .insert_merchant(merchant_account, &key_store)
        .await
        .to_duplicate_response(errors::ApiErrorResponse::DuplicateMerchantAccount)?;

    db.insert_config(diesel_models::configs::ConfigNew {
        key: format!("{}_requires_cvv", merchant_account.merchant_id),
        config: "true".to_string(),
    })
    .await
    .map_err(|err| {
        crate::logger::error!("Error while setting requires_cvv config: {err:?}");
    })
    .ok();

    Ok(service_api::ApplicationResponse::Json(
        merchant_account
            .try_into()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed while generating response")?,
    ))
}

#[cfg(feature = "olap")]
pub async fn list_merchant_account(
    state: AppState,
    req: api_models::admin::MerchantAccountListRequest,
) -> RouterResponse<Vec<api::MerchantAccountResponse>> {
    let merchant_accounts = state
        .store
        .list_merchant_accounts_by_organization_id(&req.organization_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let merchant_accounts = merchant_accounts
        .into_iter()
        .map(|merchant_account| {
            merchant_account
                .try_into()
                .change_context(errors::ApiErrorResponse::InvalidDataValue {
                    field_name: "merchant_account",
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(services::ApplicationResponse::Json(merchant_accounts))
}

pub async fn get_merchant_account(
    state: AppState,
    req: api::MerchantId,
) -> RouterResponse<api::MerchantAccountResponse> {
    let db = state.store.as_ref();
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

/// For backwards compatibility, whenever new business labels are passed in
/// primary_business_details, create a business profile
pub async fn create_business_profile_from_business_labels(
    db: &dyn StorageInterface,
    key_store: &domain::MerchantKeyStore,
    merchant_id: &str,
    new_business_details: Vec<admin_types::PrimaryBusinessDetails>,
) -> RouterResult<()> {
    let merchant_account = db
        .find_merchant_account_by_merchant_id(merchant_id, key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let old_business_details = merchant_account
        .primary_business_details
        .clone()
        .parse_value::<Vec<admin_types::PrimaryBusinessDetails>>("PrimaryBusinessDetails")
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "routing_algorithm",
        })
        .attach_printable("Invalid routing algorithm given")?;

    // find the diff between two vectors
    let business_profiles_to_create = new_business_details
        .into_iter()
        .filter(|business_details| !old_business_details.contains(business_details))
        .collect::<Vec<_>>();

    for business_profile in business_profiles_to_create {
        let profile_name = format!("{}_{}", business_profile.country, business_profile.business);

        let business_profile_create_request = admin_types::BusinessProfileCreate {
            profile_name: Some(profile_name),
            ..Default::default()
        };

        let business_profile_create_result = create_and_insert_business_profile(
            db,
            business_profile_create_request,
            merchant_account.clone(),
        )
        .await
        .map_err(|business_profile_insert_error| {
            // If there is any duplicate error, we need not take any action
            crate::logger::warn!(
                "Business profile already exists {business_profile_insert_error:?}"
            );
        });

        // If a business_profile is created, then unset the default profile
        if business_profile_create_result.is_ok() && merchant_account.default_profile.is_some() {
            let unset_default_profile = domain::MerchantAccountUpdate::UnsetDefaultProfile;
            db.update_merchant(merchant_account.clone(), unset_default_profile, key_store)
                .await
                .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;
        }
    }

    Ok(())
}

/// For backwards compatibility
/// If any of the fields of merchant account are updated, then update these fields in business profiles
pub async fn update_business_profile_cascade(
    state: AppState,
    merchant_account_update: api::MerchantAccountUpdate,
    merchant_id: String,
) -> RouterResult<()> {
    if merchant_account_update.return_url.is_some()
        || merchant_account_update.webhook_details.is_some()
        || merchant_account_update
            .enable_payment_response_hash
            .is_some()
        || merchant_account_update
            .redirect_to_merchant_with_http_post
            .is_some()
    {
        // Update these fields in all the business profiles
        let business_profiles = state
            .store
            .list_business_profile_by_merchant_id(&merchant_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::BusinessProfileNotFound {
                id: merchant_id.to_string(),
            })?;

        let business_profile_update = admin_types::BusinessProfileUpdate {
            profile_name: None,
            return_url: merchant_account_update.return_url,
            enable_payment_response_hash: merchant_account_update.enable_payment_response_hash,
            payment_response_hash_key: merchant_account_update.payment_response_hash_key,
            redirect_to_merchant_with_http_post: merchant_account_update
                .redirect_to_merchant_with_http_post,
            webhook_details: merchant_account_update.webhook_details,
            metadata: None,
            routing_algorithm: None,
            intent_fulfillment_time: None,
            frm_routing_algorithm: None,
            payout_routing_algorithm: None,
            applepay_verified_domains: None,
        };

        let update_futures = business_profiles.iter().map(|business_profile| async {
            let profile_id = &business_profile.profile_id;

            update_business_profile(
                state.clone(),
                profile_id,
                &merchant_id,
                business_profile_update.clone(),
            )
            .await
        });

        try_join_all(update_futures).await?;
    }

    Ok(())
}

pub async fn merchant_account_update(
    state: AppState,
    merchant_id: &String,
    req: api::MerchantAccountUpdate,
) -> RouterResponse<api::MerchantAccountResponse> {
    let db = state.store.as_ref();
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
        let _: api_models::routing::RoutingAlgorithm = routing_algorithm
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
            utils::Encode::<Vec<admin_types::PrimaryBusinessDetails>>::encode_to_value(
                primary_business_details,
            )
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "primary_business_details",
            })
        })
        .transpose()?;

    // In order to support backwards compatibility, if a business_labels are passed in the update
    // call, then create new business_profiles with the profile_name as business_label
    req.primary_business_details
        .clone()
        .async_map(|primary_business_details| async {
            let _ = create_business_profile_from_business_labels(
                db,
                &key_store,
                merchant_id,
                primary_business_details,
            )
            .await;
        })
        .await;

    let key = key_store.key.get_inner().peek();

    let business_profile_id_update = if let Some(ref profile_id) = req.default_profile {
        if !profile_id.is_empty_after_trim() {
            // Validate whether profile_id passed in request is valid and is linked to the merchant
            core_utils::validate_and_get_business_profile(db, Some(profile_id), merchant_id)
                .await?
                .map(|business_profile| Some(business_profile.profile_id))
        } else {
            // If empty, Update profile_id to None in the database
            Some(None)
        }
    } else {
        None
    };

    // Update the business profile, This is for backwards compatibility
    update_business_profile_cascade(state.clone(), req.clone(), merchant_id.to_string()).await?;

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
        payout_routing_algorithm: req.payout_routing_algorithm,
        default_profile: business_profile_id_update,
        payment_link_config: req.payment_link_config,
    };

    let response = db
        .update_specific_fields_in_merchant(merchant_id, updated_merchant_account, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    // If there are any new business labels generated, create business profile

    Ok(service_api::ApplicationResponse::Json(
        response
            .try_into()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed while generating response")?,
    ))
}

pub async fn merchant_account_delete(
    state: AppState,
    merchant_id: String,
) -> RouterResponse<api::MerchantAccountDeleteResponse> {
    let mut is_deleted = false;
    let db = state.store.as_ref();
    let is_merchant_account_deleted = db
        .delete_merchant_account_by_merchant_id(&merchant_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;
    if is_merchant_account_deleted {
        let is_merchant_key_store_deleted = db
            .delete_merchant_key_store_by_merchant_id(&merchant_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;
        is_deleted = is_merchant_account_deleted && is_merchant_key_store_deleted;
    }

    match db
        .delete_config_by_key(format!("{}_requires_cvv", merchant_id).as_str())
        .await
    {
        Ok(_) => Ok::<_, errors::ApiErrorResponse>(()),
        Err(err) => {
            if err.current_context().is_db_not_found() {
                crate::logger::error!("requires_cvv config not found in db: {err:?}");
                Ok(())
            } else {
                Err(err
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed while deleting requires_cvv config"))?
            }
        }
    }
    .ok();

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
    state: AppState,
    req: api::MerchantConnectorCreate,
    merchant_id: &String,
) -> RouterResponse<api_models::admin::MerchantConnectorResponse> {
    let store = state.store.as_ref();
    #[cfg(feature = "dummy_connector")]
    validate_dummy_connector_enabled(&state, &req.connector_name).await?;
    let key_store = store
        .get_merchant_key_store_by_merchant_id(
            merchant_id,
            &state.store.get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    req.metadata
        .clone()
        .map(validate_certificate_in_mca_metadata)
        .transpose()?;

    let merchant_account = state
        .store
        .find_merchant_account_by_merchant_id(merchant_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    helpers::validate_business_details(
        req.business_country,
        req.business_label.as_ref(),
        &merchant_account,
    )?;

    // Business label support will be deprecated soon
    let profile_id = core_utils::get_profile_id_from_business_details(
        req.business_country,
        req.business_label.as_ref(),
        &merchant_account,
        req.profile_id.as_ref(),
        store,
        true,
    )
    .await?;

    let routable_connector =
        api_enums::RoutableConnectors::from_str(&req.connector_name.to_string()).ok();

    let business_profile = state
        .store
        .find_business_profile_by_profile_id(&profile_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::BusinessProfileNotFound {
            id: profile_id.to_owned(),
        })?;

    // If connector label is not passed in the request, generate one
    let connector_label = req
        .connector_label
        .or(core_utils::get_connector_label(
            req.business_country,
            req.business_label.as_ref(),
            req.business_sub_label.as_ref(),
            &req.connector_name.to_string(),
        ))
        .unwrap_or(format!(
            "{}_{}",
            req.connector_name, business_profile.profile_name
        ));

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

    validate_auth_and_metadata_type(req.connector_name, &auth, &req.metadata).map_err(|err| {
        match *err.current_context() {
            errors::ConnectorError::InvalidConnectorName => {
                err.change_context(errors::ApiErrorResponse::InvalidRequestData {
                    message: "The connector name is invalid".to_string(),
                })
            }
            errors::ConnectorError::InvalidConnectorConfig { config: field_name } => err
                .change_context(errors::ApiErrorResponse::InvalidRequestData {
                    message: format!("The {} is invalid", field_name),
                }),
            errors::ConnectorError::FailedToObtainAuthType => {
                err.change_context(errors::ApiErrorResponse::InvalidRequestData {
                    message: "The auth type is invalid for the connector".to_string(),
                })
            }
            _ => err.change_context(errors::ApiErrorResponse::InvalidRequestData {
                message: "The request body is invalid".to_string(),
            }),
        }
    })?;

    let frm_configs = get_frm_config_as_secret(req.frm_configs);

    // The purpose of this merchant account update is just to update the
    // merchant account `modified_at` field for KGraph cache invalidation
    let merchant_account_update = storage::MerchantAccountUpdate::Update {
        merchant_name: None,
        merchant_details: None,
        return_url: None,
        webhook_details: None,
        sub_merchants_enabled: None,
        parent_merchant_id: None,
        enable_payment_response_hash: None,
        locker_id: None,
        payment_response_hash_key: None,
        primary_business_details: None,
        metadata: None,
        publishable_key: None,
        redirect_to_merchant_with_http_post: None,
        routing_algorithm: None,
        intent_fulfillment_time: None,
        frm_routing_algorithm: None,
        payout_routing_algorithm: None,
        default_profile: None,
        payment_link_config: None,
    };

    state
        .store
        .update_specific_fields_in_merchant(merchant_id, merchant_account_update, &key_store)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("error updating the merchant account when creating payment connector")?;

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
        connector_label: Some(connector_label),
        business_country: req.business_country,
        business_label: req.business_label.clone(),
        business_sub_label: req.business_sub_label.clone(),
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
        profile_id: Some(profile_id.clone()),
        applepay_verified_domains: None,
        pm_auth_config: req.pm_auth_config.clone(),
    };

    let mut default_routing_config =
        routing_helpers::get_merchant_default_config(&*state.store, merchant_id).await?;

    let mca = state
        .store
        .insert_merchant_connector_account(merchant_connector_account, &key_store)
        .await
        .to_duplicate_response(
            errors::ApiErrorResponse::DuplicateMerchantConnectorAccount {
                profile_id,
                connector_name: req.connector_name.to_string(),
            },
        )?;

    if let Some(routable_connector_val) = routable_connector {
        let choice = routing_types::RoutableConnectorChoice {
            #[cfg(feature = "backwards_compatibility")]
            choice_kind: routing_types::RoutableChoiceKind::FullStruct,
            connector: routable_connector_val,
            #[cfg(feature = "connector_choice_mca_id")]
            merchant_connector_id: Some(mca.merchant_connector_id.clone()),
            #[cfg(not(feature = "connector_choice_mca_id"))]
            sub_label: req.business_sub_label,
        };

        if !default_routing_config.contains(&choice) {
            default_routing_config.push(choice);
            routing_helpers::update_merchant_default_config(
                &*state.store,
                merchant_id,
                default_routing_config,
            )
            .await?;
        }
    }

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
    state: AppState,
    merchant_id: String,
    merchant_connector_id: String,
) -> RouterResponse<api_models::admin::MerchantConnectorResponse> {
    let store = state.store.as_ref();
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
    state: AppState,
    merchant_id: String,
) -> RouterResponse<Vec<api_models::admin::MerchantConnectorResponse>> {
    let store = state.store.as_ref();
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
    state: AppState,
    merchant_id: &str,
    merchant_connector_id: &str,
    req: api_models::admin::MerchantConnectorUpdate,
) -> RouterResponse<api_models::admin::MerchantConnectorResponse> {
    let db = state.store.as_ref();
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

    let frm_configs = get_frm_config_as_secret(req.frm_configs);

    let payment_connector = storage::MerchantConnectorAccountUpdate::Update {
        merchant_id: None,
        connector_type: Some(req.connector_type),
        connector_name: None,
        merchant_connector_id: None,
        connector_label: req.connector_label,
        connector_account_details: req
            .connector_account_details
            .async_lift(|inner| {
                domain_types::encrypt_optional(inner, key_store.key.get_inner().peek())
            })
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed while encrypting data")?,
        test_mode: req.test_mode,
        disabled: req.disabled,
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
        applepay_verified_domains: None,
        pm_auth_config: req.pm_auth_config,
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
    state: AppState,
    merchant_id: String,
    merchant_connector_id: String,
) -> RouterResponse<api::MerchantConnectorDeleteResponse> {
    let db = state.store.as_ref();
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
    state: AppState,
    merchant_id: String,
    enable: bool,
) -> RouterResponse<api_models::admin::ToggleKVResponse> {
    let db = state.store.as_ref();
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
        (true, MerchantStorageScheme::RedisKv) | (false, MerchantStorageScheme::PostgresOnly) => {
            Ok(merchant_account)
        }
        (true, MerchantStorageScheme::PostgresOnly) => {
            db.update_merchant(
                merchant_account,
                storage::MerchantAccountUpdate::StorageSchemeUpdate {
                    storage_scheme: MerchantStorageScheme::RedisKv,
                },
                &key_store,
            )
            .await
        }
        (false, MerchantStorageScheme::RedisKv) => {
            db.update_merchant(
                merchant_account,
                storage::MerchantAccountUpdate::StorageSchemeUpdate {
                    storage_scheme: MerchantStorageScheme::PostgresOnly,
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
        MerchantStorageScheme::RedisKv
    );

    Ok(service_api::ApplicationResponse::Json(
        api_models::admin::ToggleKVResponse {
            merchant_id: updated_merchant_account.merchant_id,
            kv_enabled: kv_status,
        },
    ))
}

pub async fn check_merchant_account_kv_status(
    state: AppState,
    merchant_id: String,
) -> RouterResponse<api_models::admin::ToggleKVResponse> {
    let db = state.store.as_ref();
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
        MerchantStorageScheme::RedisKv
    );

    Ok(service_api::ApplicationResponse::Json(
        api_models::admin::ToggleKVResponse {
            merchant_id: merchant_account.merchant_id,
            kv_enabled: kv_status,
        },
    ))
}

pub fn get_frm_config_as_secret(
    frm_configs: Option<Vec<api_models::admin::FrmConfigs>>,
) -> Option<Vec<Secret<serde_json::Value>>> {
    match frm_configs.as_ref() {
        Some(frm_value) => {
            let configs_for_frm_value: Vec<Secret<serde_json::Value>> = frm_value
                .iter()
                .map(|config| {
                    utils::Encode::<api_models::admin::FrmConfigs>::encode_to_value(&config)
                        .change_context(errors::ApiErrorResponse::ConfigNotFound)
                        .map(masking::Secret::new)
                })
                .collect::<Result<Vec<_>, _>>()
                .ok()?;
            Some(configs_for_frm_value)
        }
        None => None,
    }
}

pub async fn create_and_insert_business_profile(
    db: &dyn StorageInterface,
    request: api::BusinessProfileCreate,
    merchant_account: domain::MerchantAccount,
) -> RouterResult<storage::business_profile::BusinessProfile> {
    let business_profile_new = storage::business_profile::BusinessProfileNew::foreign_try_from((
        merchant_account,
        request,
    ))?;

    let profile_name = business_profile_new.profile_name.clone();

    db.insert_business_profile(business_profile_new)
        .await
        .to_duplicate_response(errors::ApiErrorResponse::GenericDuplicateError {
            message: format!(
                "Business Profile with the profile_name {profile_name} already exists"
            ),
        })
        .attach_printable("Failed to insert Business profile because of duplication error")
}

pub async fn create_business_profile(
    state: AppState,
    request: api::BusinessProfileCreate,
    merchant_id: &str,
) -> RouterResponse<api_models::admin::BusinessProfileResponse> {
    let db = state.store.as_ref();
    let key_store = db
        .get_merchant_key_store_by_merchant_id(merchant_id, &db.get_master_key().to_vec().into())
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    // Get the merchant account, if few fields are not passed, then they will be inherited from
    // merchant account
    let merchant_account = db
        .find_merchant_account_by_merchant_id(merchant_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    if let Some(ref routing_algorithm) = request.routing_algorithm {
        let _: api_models::routing::RoutingAlgorithm = routing_algorithm
            .clone()
            .parse_value("RoutingAlgorithm")
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "routing_algorithm",
            })
            .attach_printable("Invalid routing algorithm given")?;
    }

    let business_profile =
        create_and_insert_business_profile(db, request, merchant_account.clone()).await?;

    if merchant_account.default_profile.is_some() {
        let unset_default_profile = domain::MerchantAccountUpdate::UnsetDefaultProfile;
        db.update_merchant(merchant_account, unset_default_profile, &key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;
    }

    Ok(service_api::ApplicationResponse::Json(
        api_models::admin::BusinessProfileResponse::foreign_try_from(business_profile)
            .change_context(errors::ApiErrorResponse::InternalServerError)?,
    ))
}

pub async fn list_business_profile(
    state: AppState,
    merchant_id: String,
) -> RouterResponse<Vec<api_models::admin::BusinessProfileResponse>> {
    let db = state.store.as_ref();
    let business_profiles = db
        .list_business_profile_by_merchant_id(&merchant_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::InternalServerError)?
        .into_iter()
        .map(|business_profile| {
            api_models::admin::BusinessProfileResponse::foreign_try_from(business_profile)
        })
        .collect::<Result<Vec<_>, _>>()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to parse business profile details")?;

    Ok(service_api::ApplicationResponse::Json(business_profiles))
}

pub async fn retrieve_business_profile(
    state: AppState,
    profile_id: String,
) -> RouterResponse<api_models::admin::BusinessProfileResponse> {
    let db = state.store.as_ref();
    let business_profile = db
        .find_business_profile_by_profile_id(&profile_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::BusinessProfileNotFound {
            id: profile_id,
        })?;

    Ok(service_api::ApplicationResponse::Json(
        api_models::admin::BusinessProfileResponse::foreign_try_from(business_profile)
            .change_context(errors::ApiErrorResponse::InternalServerError)?,
    ))
}

pub async fn delete_business_profile(
    state: AppState,
    profile_id: String,
    merchant_id: &str,
) -> RouterResponse<bool> {
    let db = state.store.as_ref();
    let delete_result = db
        .delete_business_profile_by_profile_id_merchant_id(&profile_id, merchant_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::BusinessProfileNotFound {
            id: profile_id,
        })?;

    Ok(service_api::ApplicationResponse::Json(delete_result))
}

pub async fn update_business_profile(
    state: AppState,
    profile_id: &str,
    merchant_id: &str,
    request: api::BusinessProfileUpdate,
) -> RouterResponse<api::BusinessProfileResponse> {
    let db = state.store.as_ref();
    let business_profile = db
        .find_business_profile_by_profile_id(profile_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::BusinessProfileNotFound {
            id: profile_id.to_owned(),
        })?;

    if business_profile.merchant_id != merchant_id {
        Err(errors::ApiErrorResponse::AccessForbidden {
            resource: profile_id.to_string(),
        })?
    }

    let webhook_details = request
        .webhook_details
        .as_ref()
        .map(|webhook_details| {
            utils::Encode::<api::WebhookDetails>::encode_to_value(webhook_details).change_context(
                errors::ApiErrorResponse::InvalidDataValue {
                    field_name: "webhook details",
                },
            )
        })
        .transpose()?;

    if let Some(ref routing_algorithm) = request.routing_algorithm {
        let _: api_models::routing::RoutingAlgorithm = routing_algorithm
            .clone()
            .parse_value("RoutingAlgorithm")
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "routing_algorithm",
            })
            .attach_printable("Invalid routing algorithm given")?;
    }

    let business_profile_update = storage::business_profile::BusinessProfileUpdateInternal {
        profile_name: request.profile_name,
        modified_at: Some(date_time::now()),
        return_url: request.return_url.map(|return_url| return_url.to_string()),
        enable_payment_response_hash: request.enable_payment_response_hash,
        payment_response_hash_key: request.payment_response_hash_key,
        redirect_to_merchant_with_http_post: request.redirect_to_merchant_with_http_post,
        webhook_details,
        metadata: request.metadata,
        routing_algorithm: request.routing_algorithm,
        intent_fulfillment_time: request.intent_fulfillment_time.map(i64::from),
        frm_routing_algorithm: request.frm_routing_algorithm,
        payout_routing_algorithm: request.payout_routing_algorithm,
        is_recon_enabled: None,
        applepay_verified_domains: request.applepay_verified_domains,
    };

    let updated_business_profile = db
        .update_business_profile_by_profile_id(business_profile, business_profile_update)
        .await
        .to_not_found_response(errors::ApiErrorResponse::BusinessProfileNotFound {
            id: profile_id.to_owned(),
        })?;

    Ok(service_api::ApplicationResponse::Json(
        api_models::admin::BusinessProfileResponse::foreign_try_from(updated_business_profile)
            .change_context(errors::ApiErrorResponse::InternalServerError)?,
    ))
}

pub(crate) fn validate_auth_and_metadata_type(
    connector_name: api_models::enums::Connector,
    val: &types::ConnectorAuthType,
    connector_meta_data: &Option<pii::SecretSerdeValue>,
) -> Result<(), error_stack::Report<errors::ConnectorError>> {
    use crate::connector::*;

    match connector_name {
        #[cfg(feature = "dummy_connector")]
        api_enums::Connector::DummyConnector1
        | api_enums::Connector::DummyConnector2
        | api_enums::Connector::DummyConnector3
        | api_enums::Connector::DummyConnector4
        | api_enums::Connector::DummyConnector5
        | api_enums::Connector::DummyConnector6
        | api_enums::Connector::DummyConnector7 => {
            dummyconnector::transformers::DummyConnectorAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Aci => {
            aci::transformers::AciAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Adyen => {
            adyen::transformers::AdyenAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Airwallex => {
            airwallex::transformers::AirwallexAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Authorizedotnet => {
            authorizedotnet::transformers::AuthorizedotnetAuthType::try_from(val)?;
            Ok(())
        }
        // api_enums::Connector::Bankofamerica => {
        //     bankofamerica::transformers::BankofamericaAuthType::try_from(val)?;
        //     Ok(())
        // } Added as template code for future usage
        api_enums::Connector::Bitpay => {
            bitpay::transformers::BitpayAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Bambora => {
            bambora::transformers::BamboraAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Boku => {
            boku::transformers::BokuAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Bluesnap => {
            bluesnap::transformers::BluesnapAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Braintree => {
            braintree::transformers::BraintreeAuthType::try_from(val)?;
            braintree::braintree_graphql_transformers::BraintreeMeta::try_from(
                connector_meta_data,
            )?;
            Ok(())
        }
        api_enums::Connector::Cashtocode => {
            cashtocode::transformers::CashtocodeAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Checkout => {
            checkout::transformers::CheckoutAuthType::try_from(val)?;
            Ok(())
        }

        api_enums::Connector::Coinbase => {
            coinbase::transformers::CoinbaseAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Cryptopay => {
            cryptopay::transformers::CryptopayAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Cybersource => {
            cybersource::transformers::CybersourceAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Dlocal => {
            dlocal::transformers::DlocalAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Fiserv => {
            fiserv::transformers::FiservAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Forte => {
            forte::transformers::ForteAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Globalpay => {
            globalpay::transformers::GlobalpayAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Globepay => {
            globepay::transformers::GlobepayAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Gocardless => {
            gocardless::transformers::GocardlessAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Helcim => {
            helcim::transformers::HelcimAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Iatapay => {
            iatapay::transformers::IatapayAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Klarna => {
            klarna::transformers::KlarnaAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Mollie => {
            mollie::transformers::MollieAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Multisafepay => {
            multisafepay::transformers::MultisafepayAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Nexinets => {
            nexinets::transformers::NexinetsAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Nmi => {
            nmi::transformers::NmiAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Noon => {
            noon::transformers::NoonAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Nuvei => {
            nuvei::transformers::NuveiAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Opennode => {
            opennode::transformers::OpennodeAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Payme => {
            payme::transformers::PaymeAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Paypal => {
            paypal::transformers::PaypalAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Payu => {
            payu::transformers::PayuAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Powertranz => {
            powertranz::transformers::PowertranzAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Rapyd => {
            rapyd::transformers::RapydAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Shift4 => {
            shift4::transformers::Shift4AuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Square => {
            square::transformers::SquareAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Stax => {
            stax::transformers::StaxAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Stripe => {
            stripe::transformers::StripeAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Trustpay => {
            trustpay::transformers::TrustpayAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Tsys => {
            tsys::transformers::TsysAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Volt => {
            volt::transformers::VoltAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Wise => {
            wise::transformers::WiseAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Worldline => {
            worldline::transformers::WorldlineAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Worldpay => {
            worldpay::transformers::WorldpayAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Zen => {
            zen::transformers::ZenAuthType::try_from(val)?;
            Ok(())
        }
        api_enums::Connector::Signifyd | api_enums::Connector::Plaid => {
            Err(report!(errors::ConnectorError::InvalidConnectorName)
                .attach_printable(format!("invalid connector name: {connector_name}")))
        }
    }
}

#[cfg(feature = "dummy_connector")]
pub async fn validate_dummy_connector_enabled(
    state: &AppState,
    connector_name: &api_enums::Connector,
) -> Result<(), errors::ApiErrorResponse> {
    if !state.conf.dummy_connector.enabled
        && matches!(
            connector_name,
            api_enums::Connector::DummyConnector1
                | api_enums::Connector::DummyConnector2
                | api_enums::Connector::DummyConnector3
                | api_enums::Connector::DummyConnector4
                | api_enums::Connector::DummyConnector5
                | api_enums::Connector::DummyConnector6
                | api_enums::Connector::DummyConnector7
        )
    {
        Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "Invalid connector name".to_string(),
        })
    } else {
        Ok(())
    }
}
