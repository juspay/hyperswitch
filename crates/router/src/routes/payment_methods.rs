#[cfg(all(feature = "v1", any(feature = "olap", feature = "oltp")))]
use std::collections::HashMap;

use ::payment_methods::{
    controller::PaymentMethodsController,
    core::{migration, migration::payment_methods::migrate_payment_method},
};
#[cfg(all(feature = "v1", any(feature = "olap", feature = "oltp")))]
use actix_multipart::form::MultipartForm;
use actix_web::{web, HttpRequest, HttpResponse};
use common_utils::{errors::CustomResult, id_type, transformers::ForeignFrom};
use diesel_models::enums::IntentStatus;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    bulk_tokenization::CardNetworkTokenizeRequest, merchant_key_store::MerchantKeyStore,
    payment_methods::PaymentMethodCustomerMigrate, transformers::ForeignTryFrom,
};
use router_env::{instrument, logger, tracing, Flow};

use super::app::{AppState, SessionState};
#[cfg(all(feature = "v1", any(feature = "olap", feature = "oltp")))]
use crate::core::{
    customers,
    payment_methods::{batch_retrieve, tokenize},
};
use crate::{
    core::{
        api_locking,
        errors::{self, utils::StorageErrorExt},
        payment_methods::{self as payment_methods_routes, cards, migration as update_migration},
    },
    services::{self, api, authentication as auth, authorization::permissions::Permission},
    types::{
        api::payment_methods::{self, PaymentMethodId},
        domain,
        storage::payment_method::PaymentTokenData,
    },
};

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsCreate))]
pub async fn create_payment_method_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<payment_methods::PaymentMethodCreate>,
) -> HttpResponse {
    let flow = Flow::PaymentMethodsCreate;

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, req, _| async move {
            Box::pin(cards::get_client_secret_or_add_payment_method(
                &state,
                req,
                auth.platform.get_provider(),
                auth.platform.get_initiator(),
            ))
            .await
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            allow_connected_scope_operation: true,
            allow_platform_self_operation: true,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsCreate))]
pub async fn create_payment_method_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<payment_methods::PaymentMethodCreate>,
) -> HttpResponse {
    let flow = Flow::PaymentMethodsCreate;

    let api_auth = auth::V2ApiKeyAuth {
        allow_connected_scope_operation: true,
        allow_platform_self_operation: true,
    };
    let (auth_type, _api_key_type) = match auth::check_internal_api_key_auth_no_client_secret(
        req.headers(),
        api_auth,
        state.conf.internal_merchant_id_profile_id_auth.clone(),
    ) {
        Ok(auth) => auth,
        Err(err) => return api::log_and_return_error_response(error_stack::report!(err)),
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, req, req_state| async move {
            Box::pin(payment_methods_routes::create_payment_method(
                &state,
                &req_state,
                req,
                &auth.platform,
                &auth.profile,
            ))
            .await
        },
        &*auth_type,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::NetworkTokenEligibilityCheck))]
pub async fn get_pm_nt_eligibility_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<payment_methods::NetworkTokenEligibilityRequest>,
) -> HttpResponse {
    let flow = Flow::NetworkTokenEligibilityCheck;

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        query.into_inner(),
        |state, auth: auth::AuthenticationData, req, _req_state| async move {
            Box::pin(payment_methods_routes::get_card_nt_eligibility(
                &state,
                req,
                &auth.platform,
                &auth.profile,
            ))
            .await
        },
        &auth::V2ApiKeyAuth {
            allow_connected_scope_operation: true,
            allow_platform_self_operation: true,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsCreate))]
pub async fn create_payment_method_intent_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<payment_methods::PaymentMethodIntentCreate>,
) -> HttpResponse {
    let flow = Flow::PaymentMethodsCreate;

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, req, _| async move {
            Box::pin(payment_methods_routes::payment_method_intent_create(
                &state,
                req,
                auth.platform.get_provider().clone(),
                auth.platform.get_initiator(),
            ))
            .await
        },
        &auth::V2ApiKeyAuth {
            allow_connected_scope_operation: true,
            allow_platform_self_operation: true,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// This struct is used internally only
#[cfg(feature = "v2")]
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct PaymentMethodIntentConfirmInternal {
    pub id: id_type::GlobalPaymentMethodId,
    pub request: payment_methods::PaymentMethodIntentConfirm,
}

#[cfg(feature = "v2")]
impl From<PaymentMethodIntentConfirmInternal> for payment_methods::PaymentMethodIntentConfirm {
    fn from(item: PaymentMethodIntentConfirmInternal) -> Self {
        item.request
    }
}

#[cfg(feature = "v2")]
impl common_utils::events::ApiEventMetric for PaymentMethodIntentConfirmInternal {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::PaymentMethod {
            payment_method_id: self.id.clone(),
            payment_method_type: Some(self.request.payment_method_type),
            payment_method_subtype: Some(self.request.payment_method_subtype),
        })
    }
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsUpdate))]
pub async fn payment_method_update_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<id_type::GlobalPaymentMethodId>,
    json_payload: web::Json<payment_methods::PaymentMethodUpdate>,
) -> HttpResponse {
    let flow = Flow::PaymentMethodsUpdate;
    let payment_method_id = path.into_inner();
    let payload = json_payload.into_inner();

    let api_auth = auth::V2ApiKeyAuth {
        allow_connected_scope_operation: true,
        allow_platform_self_operation: true,
    };

    let (auth_type, api_key_type) = match auth::check_internal_api_key_auth_no_client_secret(
        req.headers(),
        api_auth,
        state.conf.internal_merchant_id_profile_id_auth.clone(),
    ) {
        Ok(auth) => auth,
        Err(err) => return api::log_and_return_error_response(error_stack::report!(err)),
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            payment_methods_routes::update_payment_method(
                state,
                auth.platform,
                auth.profile,
                req,
                &payment_method_id,
            )
        },
        &*auth_type,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsRetrieve))]
pub async fn payment_method_retrieve_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    query_payload: web::Query<api_models::payment_methods::PaymentMethodRetrieveRequest>,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::PaymentMethodsRetrieve;
    let payload = web::Json(PaymentMethodId {
        payment_method_id: path.into_inner(),
    })
    .into_inner();

    let api_auth = auth::V2ApiKeyAuth {
        allow_connected_scope_operation: true,
        allow_platform_self_operation: true,
    };

    let (auth_type, api_key_type) = match auth::check_internal_api_key_auth_no_client_secret(
        req.headers(),
        api_auth,
        state.conf.internal_merchant_id_profile_id_auth.clone(),
    ) {
        Ok(auth) => auth,
        Err(err) => return api::log_and_return_error_response(error_stack::report!(err)),
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, pm, _| {
            payment_methods_routes::retrieve_payment_method(
                state,
                pm,
                auth.profile,
                auth.platform,
                api_key_type,
                query_payload.fetch_raw_detail,
            )
        },
        &*auth_type,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsDelete))]
pub async fn payment_method_delete_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::PaymentMethodsDelete;
    let payload = web::Json(PaymentMethodId {
        payment_method_id: path.into_inner(),
    })
    .into_inner();

    let api_auth = auth::V2ApiKeyAuth {
        allow_connected_scope_operation: true,
        allow_platform_self_operation: true,
    };

    let (auth_type, api_key_type) = match auth::check_internal_api_key_auth_no_client_secret(
        req.headers(),
        api_auth,
        state.conf.internal_merchant_id_profile_id_auth.clone(),
    ) {
        Ok(auth) => auth,
        Err(err) => return api::log_and_return_error_response(error_stack::report!(err)),
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, pm, _| {
            payment_methods_routes::delete_payment_method(state, pm, auth.platform, auth.profile)
        },
        &*auth_type,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsMigrate))]
pub async fn migrate_payment_method_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<payment_methods::PaymentMethodMigrate>,
) -> HttpResponse {
    let flow = Flow::PaymentMethodsMigrate;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, _, req, _| async move {
            let merchant_id = req.merchant_id.clone();
            let (key_store, merchant_account) = get_merchant_account(&state, &merchant_id).await?;
            let platform = domain::Platform::new(
                merchant_account.clone(),
                key_store.clone(),
                merchant_account,
                key_store,
                None,
            );
            Box::pin(migrate_payment_method(
                &(&state).into(),
                req,
                &merchant_id,
                &platform,
                &cards::PmCards {
                    state: &state,
                    provider: platform.get_provider(),
                },
            ))
            .await
        },
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

async fn get_merchant_account(
    state: &SessionState,
    merchant_id: &id_type::MerchantId,
) -> CustomResult<(MerchantKeyStore, domain::MerchantAccount), errors::ApiErrorResponse> {
    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            merchant_id,
            &state.store.get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let merchant_account = state
        .store
        .find_merchant_account_by_merchant_id(merchant_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;
    Ok((key_store, merchant_account))
}

#[cfg(all(feature = "v1", any(feature = "olap", feature = "oltp")))]
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsMigrate))]
pub async fn migrate_payment_methods(
    state: web::Data<AppState>,
    req: HttpRequest,
    MultipartForm(form): MultipartForm<migration::PaymentMethodsMigrateForm>,
) -> HttpResponse {
    let flow = Flow::PaymentMethodsMigrate;
    let (merchant_id, records, merchant_connector_ids) =
        match form.validate_and_get_payment_method_records() {
            Ok((merchant_id, records, merchant_connector_ids)) => {
                (merchant_id, records, merchant_connector_ids)
            }
            Err(e) => return api::log_and_return_error_response(e.into()),
        };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        records,
        |state, _, req, _| {
            let merchant_id = merchant_id.clone();
            let merchant_connector_ids = merchant_connector_ids.clone();
            async move {
                let (key_store, merchant_account) =
                    get_merchant_account(&state, &merchant_id).await?;
                // Create customers if they are not already present
                let platform = domain::Platform::new(
                    merchant_account.clone(),
                    key_store.clone(),
                    merchant_account,
                    key_store,
                    None,
                );

                let mut mca_cache = HashMap::new();
                let customers = Vec::<PaymentMethodCustomerMigrate>::foreign_try_from((
                    &req,
                    merchant_id.clone(),
                ))
                .map_err(|e| errors::ApiErrorResponse::InvalidRequestData {
                    message: e.to_string(),
                })?;

                for record in &customers {
                    if let Some(connector_customer_details) = &record.connector_customer_details {
                        for connector_customer in connector_customer_details {
                            if !mca_cache.contains_key(&connector_customer.merchant_connector_id) {
                                let mca = state
                        .store
                        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                            &merchant_id,
                            &connector_customer.merchant_connector_id,
                            platform.get_processor().get_key_store(),
                        )
                        .await
                        .to_not_found_response(
                            errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                                id: connector_customer.merchant_connector_id.get_string_repr().to_string(),
                            },
                        )?;
                                mca_cache
                                    .insert(connector_customer.merchant_connector_id.clone(), mca);
                            }
                        }
                    }
                }

                customers::migrate_customers(state.clone(), customers, platform.clone())
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)?;
                let controller = cards::PmCards {
                    state: &state,
                    provider: platform.get_provider(),
                };
                Box::pin(migration::migrate_payment_methods(
                    &(&state).into(),
                    req,
                    &merchant_id,
                    &platform,
                    merchant_connector_ids,
                    &controller,
                ))
                .await
            }
        },
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "v1", any(feature = "olap", feature = "oltp")))]
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsBatchUpdate))]
pub async fn update_payment_methods(
    state: web::Data<AppState>,
    req: HttpRequest,
    MultipartForm(form): MultipartForm<update_migration::PaymentMethodsUpdateForm>,
) -> HttpResponse {
    let flow = Flow::PaymentMethodsBatchUpdate;
    let (merchant_id, records) = match form.validate_and_get_payment_method_records() {
        Ok((merchant_id, records)) => (merchant_id, records),
        Err(e) => return api::log_and_return_error_response(e.into()),
    };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        records,
        |state, _, req, _| {
            let merchant_id = merchant_id.clone();
            async move {
                let (key_store, merchant_account) =
                    get_merchant_account(&state, &merchant_id).await?;
                let platform = domain::Platform::new(
                    merchant_account.clone(),
                    key_store.clone(),
                    merchant_account,
                    key_store,
                    None,
                );
                Box::pin(update_migration::update_payment_methods(
                    &state,
                    req,
                    &merchant_id,
                    &platform,
                ))
                .await
            }
        },
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "v1", any(feature = "olap", feature = "oltp")))]
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsBatchRetrieve))]
pub async fn payment_methods_batch_retrieve_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    MultipartForm(form): MultipartForm<batch_retrieve::PaymentMethodsBatchRetrieveForm>,
) -> HttpResponse {
    let flow = Flow::PaymentMethodsBatchRetrieve;
    let (merchant_id, records) = match batch_retrieve::get_payment_method_batch_records(form) {
        Ok(result) => result,
        Err(error) => return api::log_and_return_error_response(error.into()),
    };

    if records.is_empty() {
        return api::log_and_return_error_response(
            errors::ApiErrorResponse::InvalidRequestData {
                message: "No payment_method_ids provided".to_string(),
            }
            .into(),
        );
    }

    if records.len() > 200 {
        return api::log_and_return_error_response(
            errors::ApiErrorResponse::InvalidRequestData {
                message: "A maximum of 200 payment_method_ids are allowed per request".to_string(),
            }
            .into(),
        );
    }

    let payload_records = records.clone();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload_records,
        |state, _, _, _| {
            let merchant_id = merchant_id.clone();
            let records = records.clone();
            async move {
                let (key_store, merchant_account) =
                    get_merchant_account(&state, &merchant_id).await?;

                let platform = domain::Platform::new(
                    merchant_account.clone(),
                    key_store.clone(),
                    merchant_account,
                    key_store,
                    None,
                );

                let responses = batch_retrieve::retrieve_payment_method_data(
                    &state,
                    &merchant_id,
                    &platform,
                    records,
                )
                .await?;

                Ok(services::ApplicationResponse::Json(responses))
            }
        },
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodSave))]
pub async fn save_payment_method_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<payment_methods::PaymentMethodCreate>,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::PaymentMethodSave;
    let payload = json_payload.into_inner();
    let pm_id = path.into_inner();
    let api_auth = auth::ApiKeyAuth {
        allow_connected_scope_operation: true,
        allow_platform_self_operation: true,
    };

    let (auth, _) = match auth::check_sdk_auth_and_get_auth(req.headers(), &payload, api_auth) {
        Ok((auth, _auth_flow)) => (auth, _auth_flow),
        Err(e) => return api::log_and_return_error_response(e),
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, mut req, _| {
            if let Some(client_secret) = auth.client_secret {
                req.client_secret = Some(client_secret);
            }

            Box::pin(cards::add_payment_method_data(
                state,
                req,
                auth.platform.get_provider().clone(),
                auth.platform.get_initiator().cloned(),
                pm_id.clone(),
            ))
        },
        &*auth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsList))]
pub async fn list_payment_method_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Query<payment_methods::PaymentMethodListRequest>,
) -> HttpResponse {
    let flow = Flow::PaymentMethodsList;
    let payload = json_payload.into_inner();
    let api_auth = auth::ApiKeyAuth {
        allow_connected_scope_operation: true,
        allow_platform_self_operation: true,
    };

    let (auth, _) = match auth::check_sdk_auth_and_get_auth(req.headers(), &payload, api_auth) {
        Ok((auth, _auth_flow)) => (auth, _auth_flow),
        Err(e) => return api::log_and_return_error_response(e),
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, mut req, _| {
            if let Some(client_secret) = auth.client_secret {
                req.client_secret = Some(client_secret);
            }

            // TODO (#7195): Fill platform_merchant_account in the client secret auth and pass it here.
            cards::list_payment_methods(state, auth.platform, req)
        },
        &*auth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
/// List payment methods for a Customer
///
/// To filter and list the applicable payment methods for a particular Customer ID
#[instrument(skip_all, fields(flow = ?Flow::CustomerPaymentMethodsList))]
pub async fn list_customer_payment_method_api(
    state: web::Data<AppState>,
    customer_id: web::Path<(id_type::CustomerId,)>,
    req: HttpRequest,
    query_payload: web::Query<payment_methods::PaymentMethodListRequest>,
) -> HttpResponse {
    let flow = Flow::CustomerPaymentMethodsList;
    let payload = query_payload.into_inner();
    let customer_id = customer_id.into_inner().0;
    let api_auth = auth::ApiKeyAuth {
        allow_connected_scope_operation: true,
        allow_platform_self_operation: true,
    };

    let auth_type: Box<dyn auth::AuthenticateAndFetch<auth::AuthenticationData, _>> =
        match req.headers().get(actix_web::http::header::AUTHORIZATION) {
            Some(_) => Box::new(auth::SdkAuthorizationAuth {
                allow_connected_scope_operation: api_auth.allow_connected_scope_operation,
                allow_platform_self_operation: api_auth.allow_platform_self_operation,
            }),
            None => match auth::is_ephemeral_auth(req.headers(), api_auth) {
                Ok(auth) => auth,
                Err(err) => return api::log_and_return_error_response(err),
            },
        };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, mut req, _| {
            if let Some(client_secret) = auth.client_secret {
                req.client_secret = Some(client_secret);
            }

            cards::do_list_customer_pm_fetch_customer_if_not_passed(
                state,
                auth.platform,
                Some(req),
                Some(&customer_id),
                None,
            )
        },
        &*auth_type,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
/// List payment methods for a Customer
///
/// To filter and list the applicable payment methods for a particular Customer ID
#[instrument(skip_all, fields(flow = ?Flow::CustomerPaymentMethodsList))]
pub async fn list_customer_payment_method_api_client(
    state: web::Data<AppState>,
    req: HttpRequest,
    query_payload: web::Query<payment_methods::PaymentMethodListRequest>,
) -> HttpResponse {
    let flow = Flow::CustomerPaymentMethodsList;
    let payload = query_payload.into_inner();
    let api_key = auth::get_api_key(req.headers()).ok();
    let api_auth = auth::ApiKeyAuth {
        allow_connected_scope_operation: true,
        allow_platform_self_operation: false,
    };

    // Check if Authorization header is present for SDK authorization
    let (auth, _, is_ephemeral_auth) = match req
        .headers()
        .get(actix_web::http::header::AUTHORIZATION)
    {
        Some(_) => {
            // If Authorization header is present, use SdkAuthorizationAuth
            let auth: Box<dyn auth::AuthenticateAndFetch<auth::AuthenticationData, _>> =
                Box::new(auth::SdkAuthorizationAuth {
                    allow_connected_scope_operation: api_auth.allow_connected_scope_operation,
                    allow_platform_self_operation: api_auth.allow_platform_self_operation,
                });
            (auth, api::AuthFlow::Client, false)
        }
        None => {
            // If Authorization header is not present, use existing logic
            match auth::get_ephemeral_or_other_auth(req.headers(), false, Some(&payload), api_auth)
                .await
            {
                Ok((auth, _auth_flow, is_ephemeral_auth)) => (auth, _auth_flow, is_ephemeral_auth),
                Err(e) => return api::log_and_return_error_response(e),
            }
        }
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, mut req, _| {
            if let Some(client_secret) = auth.client_secret {
                req.client_secret = Some(client_secret);
            }

            cards::do_list_customer_pm_fetch_customer_if_not_passed(
                state,
                auth.platform,
                Some(req),
                None,
                is_ephemeral_auth.then_some(api_key).flatten(),
            )
        },
        &*auth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Generate a form link for collecting payment methods for a customer
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodCollectLink))]
pub async fn initiate_pm_collect_link_flow(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<payment_methods::PaymentMethodCollectLinkRequest>,
) -> HttpResponse {
    let flow = Flow::PaymentMethodCollectLink;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, req, _| {
            payment_methods_routes::initiate_pm_collect_link(state, auth.platform, req)
        },
        &auth::ApiKeyAuth {
            allow_connected_scope_operation: true,
            allow_platform_self_operation: true,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "v2", feature = "olap"))]
#[instrument(skip_all, fields(flow = ?Flow::CustomerPaymentMethodsList))]
pub async fn list_customer_payment_method_api(
    state: web::Data<AppState>,
    customer_id: web::Path<id_type::GlobalCustomerId>,
    req: HttpRequest,
    query_payload: web::Query<api_models::payment_methods::ListMethodsForPaymentMethodsRequest>,
) -> HttpResponse {
    let flow = Flow::CustomerPaymentMethodsList;
    let payload = query_payload.into_inner();
    let customer_id = customer_id.into_inner();

    let api_auth = auth::V2ApiKeyAuth {
        allow_connected_scope_operation: true,
        allow_platform_self_operation: true,
    };
    let jwt_auth = auth::JWTAuth {
        permission: Permission::MerchantCustomerRead,
        allow_connected: true,
        allow_platform: true,
    };
    let (auth_type, api_key_type) =
        match auth::check_internal_api_key_or_dashboard_auth_no_client_secret(
            req.headers(),
            api_auth,
            jwt_auth,
            state.conf.internal_merchant_id_profile_id_auth.clone(),
        ) {
            Ok(auth) => auth,
            Err(err) => return api::log_and_return_error_response(error_stack::report!(err)),
        };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, payload, _| {
            payment_methods_routes::list_saved_payment_methods_for_customer(
                state,
                auth.platform.get_provider().clone(),
                customer_id.clone(),
                payload.include_new.unwrap_or(false),
            )
        },
        &*auth_type,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "v2", feature = "olap"))]
#[instrument(skip_all, fields(flow = ?Flow::GetPaymentMethodTokenData))]
pub async fn get_payment_method_token_data(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<id_type::GlobalPaymentMethodId>,
    json_payload: web::Json<api_models::payment_methods::GetTokenDataRequest>,
) -> HttpResponse {
    let flow = Flow::GetPaymentMethodTokenData;
    let payment_method_id = path.into_inner();
    let payload = json_payload.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            payment_methods_routes::get_token_data_for_payment_method(
                state,
                auth.platform.get_provider().clone(),
                auth.profile,
                req,
                payment_method_id.clone(),
            )
        },
        auth::auth_type(
            &auth::V2ApiKeyAuth {
                allow_connected_scope_operation: true,
                allow_platform_self_operation: true,
            },
            &auth::JWTAuth {
                permission: Permission::MerchantCustomerRead,
                allow_connected: true,
                allow_platform: true,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "v2", feature = "olap"))]
#[instrument(skip_all, fields(flow = ?Flow::TotalPaymentMethodCount))]
pub async fn get_total_payment_method_count(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> HttpResponse {
    let flow = Flow::TotalPaymentMethodCount;

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, auth: auth::AuthenticationData, _, _| {
            payment_methods_routes::get_total_saved_payment_methods_for_merchant(
                state,
                auth.platform.get_provider().clone(),
            )
        },
        auth::auth_type(
            &auth::V2ApiKeyAuth {
                allow_connected_scope_operation: true,
                allow_platform_self_operation: true,
            },
            &auth::JWTAuth {
                permission: Permission::MerchantCustomerRead,
                allow_connected: true,
                allow_platform: true,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
/// Generate a form link for collecting payment methods for a customer
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodCollectLink))]
pub async fn render_pm_collect_link(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(id_type::MerchantId, String)>,
) -> HttpResponse {
    let flow = Flow::PaymentMethodCollectLink;
    let (merchant_id, pm_collect_link_id) = path.into_inner();
    let payload = payment_methods::PaymentMethodCollectLinkRenderRequest {
        merchant_id: merchant_id.clone(),
        pm_collect_link_id,
    };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            payment_methods_routes::render_pm_collect_link(
                state,
                auth.platform.get_provider().clone(),
                req,
            )
        },
        &auth::MerchantIdAuth(merchant_id),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsRetrieve))]
pub async fn payment_method_retrieve_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::PaymentMethodsRetrieve;
    let payload = web::Json(PaymentMethodId {
        payment_method_id: path.into_inner(),
    })
    .into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, pm, _| async move {
            cards::PmCards {
                state: &state,
                provider: auth.platform.get_provider(),
            }
            .retrieve_payment_method(pm)
            .await
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            allow_connected_scope_operation: true,
            allow_platform_self_operation: true,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsUpdate))]
pub async fn payment_method_update_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    json_payload: web::Json<payment_methods::PaymentMethodUpdate>,
) -> HttpResponse {
    let flow = Flow::PaymentMethodsUpdate;
    let payment_method_id = path.into_inner();
    let payload = json_payload.into_inner();
    let api_auth = auth::ApiKeyAuth {
        allow_connected_scope_operation: true,
        allow_platform_self_operation: true,
    };

    let (auth, _) = match auth::check_sdk_auth_and_get_auth(req.headers(), &payload, api_auth) {
        Ok((auth, _auth_flow)) => (auth, _auth_flow),
        Err(e) => return api::log_and_return_error_response(e),
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, mut req, _| {
            if let Some(client_secret) = auth.client_secret {
                req.client_secret = Some(client_secret);
            }

            cards::update_customer_payment_method(
                state,
                auth.platform.get_provider().clone(),
                auth.platform.get_initiator().cloned(),
                req,
                &payment_method_id,
                None,
            )
        },
        &*auth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsDelete))]
pub async fn payment_method_delete_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    payment_method_id: web::Path<(String,)>,
) -> HttpResponse {
    let flow = Flow::PaymentMethodsDelete;
    let pm = PaymentMethodId {
        payment_method_id: payment_method_id.into_inner().0,
    };
    let api_auth = auth::ApiKeyAuth {
        allow_connected_scope_operation: true,
        allow_platform_self_operation: true,
    };

    let ephemeral_auth = match auth::is_ephemeral_auth(req.headers(), api_auth) {
        Ok(auth) => auth,
        Err(err) => return api::log_and_return_error_response(err),
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        pm,
        |state, auth: auth::AuthenticationData, req, _| async move {
            cards::PmCards {
                state: &state,
                provider: auth.platform.get_provider(),
            }
            .delete_payment_method(req, auth.platform.get_initiator())
            .await
        },
        &*ephemeral_auth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::ListCountriesCurrencies))]
pub async fn list_countries_currencies_for_connector_payment_method(
    state: web::Data<AppState>,
    req: HttpRequest,
    query_payload: web::Query<payment_methods::ListCountriesCurrenciesRequest>,
) -> HttpResponse {
    let flow = Flow::ListCountriesCurrencies;
    let payload = query_payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            cards::list_countries_currencies_for_connector_payment_method(
                state,
                req,
                auth.profile.map(|profile| profile.get_id().clone()),
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                allow_connected_scope_operation: true,
                allow_platform_self_operation: true,
            }),
            &auth::JWTAuth {
                permission: Permission::ProfileConnectorWrite,
                allow_connected: true,
                allow_platform: true,
            },
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth {
            permission: Permission::ProfileConnectorWrite,
            allow_connected: true,
            allow_platform: true,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::ListCountriesCurrencies))]
pub async fn list_countries_currencies_for_connector_payment_method(
    state: web::Data<AppState>,
    req: HttpRequest,
    query_payload: web::Query<payment_methods::ListCountriesCurrenciesRequest>,
) -> HttpResponse {
    let flow = Flow::ListCountriesCurrencies;
    let payload = query_payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            cards::list_countries_currencies_for_connector_payment_method(
                state,
                req,
                Some(auth.profile.get_id().clone()),
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::V2ApiKeyAuth {
                allow_connected_scope_operation: true,
                allow_platform_self_operation: true,
            },
            &auth::JWTAuth {
                permission: Permission::ProfileConnectorRead,
                allow_connected: true,
                allow_platform: true,
            },
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth {
            permission: Permission::ProfileConnectorRead,
            allow_connected: true,
            allow_platform: true,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::DefaultPaymentMethodsSet))]
pub async fn default_payment_method_set_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<payment_methods::DefaultPaymentMethod>,
) -> HttpResponse {
    let flow = Flow::DefaultPaymentMethodsSet;
    let payload = path.into_inner();
    let pc = payload.clone();
    let customer_id = &pc.customer_id;
    let api_auth = auth::ApiKeyAuth {
        allow_connected_scope_operation: true,
        allow_platform_self_operation: true,
    };

    let auth_type: Box<dyn auth::AuthenticateAndFetch<auth::AuthenticationData, _>> =
        match req.headers().get(actix_web::http::header::AUTHORIZATION) {
            Some(_) => Box::new(auth::SdkAuthorizationAuth {
                allow_connected_scope_operation: api_auth.allow_connected_scope_operation,
                allow_platform_self_operation: api_auth.allow_platform_self_operation,
            }),
            None => match auth::is_ephemeral_auth(req.headers(), api_auth) {
                Ok(auth) => auth,
                Err(err) => return api::log_and_return_error_response(err),
            },
        };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, default_payment_method, _| async move {
            cards::PmCards {
                state: &state,
                provider: auth.platform.get_provider(),
            }
            .set_default_payment_method(
                auth.platform.get_provider().get_account().get_id(),
                customer_id,
                default_payment_method.payment_method_id,
                auth.platform.get_initiator(),
            )
            .await
        },
        &*auth_type,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
#[cfg(test)]
mod tests {
    use api_models::payment_methods::PaymentMethodListRequest;

    use super::*;

    // #[test]
    // fn test_custom_list_deserialization() {
    //     let dummy_data = "amount=120&recurring_enabled=true&installment_payment_enabled=true";
    //     let de_query: web::Query<PaymentMethodListRequest> =
    //         web::Query::from_query(dummy_data).unwrap();
    //     let de_struct = de_query.into_inner();
    //     assert_eq!(de_struct.installment_payment_enabled, Some(true))
    // }

    #[test]
    fn test_custom_list_deserialization_multi_amount() {
        let dummy_data = "amount=120&recurring_enabled=true&amount=1000";
        let de_query: Result<web::Query<PaymentMethodListRequest>, _> =
            web::Query::from_query(dummy_data);
        assert!(de_query.is_err())
    }
}

#[derive(Clone)]
pub struct ParentPaymentMethodToken {
    key_for_token: String,
}

impl ParentPaymentMethodToken {
    #[cfg(feature = "v1")]
    pub fn create_key_for_token(
        (parent_pm_token, payment_method): (&String, api_models::enums::PaymentMethod),
    ) -> Self {
        Self {
            key_for_token: format!("pm_token_{parent_pm_token}_{payment_method}_hyperswitch"),
        }
    }

    #[cfg(feature = "v2")]
    pub fn create_key_for_token(parent_pm_token: &String) -> Self {
        Self {
            key_for_token: format!("pm_token_{parent_pm_token}_hyperswitch"),
        }
    }

    #[cfg(feature = "v2")]
    pub fn return_key_for_token(parent_pm_token: &String) -> String {
        format!("pm_token_{parent_pm_token}_hyperswitch")
    }

    pub async fn insert(
        &self,
        fulfillment_time: i64,
        token: PaymentTokenData,
        state: &SessionState,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        let redis_conn = state
            .store
            .get_redis_conn()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to get redis connection")?;
        redis_conn
            .serialize_and_set_key_with_expiry(
                &self.key_for_token.as_str().into(),
                token,
                fulfillment_time,
            )
            .await
            .change_context(errors::StorageError::KVError)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to add token in redis")?;

        Ok(())
    }

    pub fn should_delete_payment_method_token(&self, status: IntentStatus) -> bool {
        // RequiresMerchantAction: When the payment goes for merchant review incase of potential fraud allow payment_method_token to be stored until resolved
        ![
            IntentStatus::RequiresCustomerAction,
            IntentStatus::RequiresMerchantAction,
        ]
        .contains(&status)
    }

    pub async fn delete(&self, state: &SessionState) -> CustomResult<(), errors::ApiErrorResponse> {
        let redis_conn = state
            .store
            .get_redis_conn()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to get redis connection")?;
        match redis_conn
            .delete_key(&self.key_for_token.as_str().into())
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => {
                {
                    logger::info!("Error while deleting redis key: {:?}", err)
                };
                Ok(())
            }
        }
    }

    pub async fn get_data_for_token(
        &self,
        state: &SessionState,
    ) -> CustomResult<PaymentTokenData, errors::ApiErrorResponse> {
        let redis_conn = state
            .store
            .get_redis_conn()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to get redis connection")?;

        logger::debug!(
            "Fetching payment method token data from redis for key: {}",
            self.key_for_token
        );

        let pm_token_data = redis_conn
            .get_and_deserialize_key::<PaymentTokenData>(
                &self.key_for_token.as_str().into(),
                "Token Data",
            )
            .await
            .map_err(|e| error_stack::report!(storage_impl::StorageError::from(e)))
            .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
                message: "Payment method token either expired or does not exist".to_string(),
            })?;
        Ok(pm_token_data)
    }
}

#[cfg(all(feature = "v1", any(feature = "olap", feature = "oltp")))]
#[instrument(skip_all, fields(flow = ?Flow::TokenizeCard))]
pub async fn tokenize_card_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<payment_methods::CardNetworkTokenizeRequest>,
) -> HttpResponse {
    let flow = Flow::TokenizeCard;

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, _, req, _| async move {
            let merchant_id = req.merchant_id.clone();
            let (key_store, merchant_account) = get_merchant_account(&state, &merchant_id).await?;
            let platform = domain::Platform::new(
                merchant_account.clone(),
                key_store.clone(),
                merchant_account,
                key_store,
                None,
            );
            let res = Box::pin(cards::tokenize_card_flow(
                &state,
                CardNetworkTokenizeRequest::foreign_from(req),
                platform.get_provider(),
                platform.get_initiator(),
            ))
            .await?;
            Ok(services::ApplicationResponse::Json(res))
        },
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "v1", any(feature = "olap", feature = "oltp")))]
#[instrument(skip_all, fields(flow = ?Flow::TokenizeCardUsingPaymentMethodId))]
pub async fn tokenize_card_using_pm_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    json_payload: web::Json<payment_methods::CardNetworkTokenizeRequest>,
) -> HttpResponse {
    let flow = Flow::TokenizeCardUsingPaymentMethodId;
    let pm_id = path.into_inner();
    let mut payload = json_payload.into_inner();
    if let payment_methods::TokenizeDataRequest::ExistingPaymentMethod(ref mut pm_data) =
        payload.data
    {
        pm_data.payment_method_id = pm_id;
    } else {
        return api::log_and_return_error_response(error_stack::report!(
            errors::ApiErrorResponse::InvalidDataValue { field_name: "card" }
        ));
    }

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, _, req, _| async move {
            let merchant_id = req.merchant_id.clone();
            let (key_store, merchant_account) = get_merchant_account(&state, &merchant_id).await?;
            let platform = domain::Platform::new(
                merchant_account.clone(),
                key_store.clone(),
                merchant_account,
                key_store,
                None,
            );
            let res = Box::pin(cards::tokenize_card_flow(
                &state,
                CardNetworkTokenizeRequest::foreign_from(req),
                platform.get_provider(),
                platform.get_initiator(),
            ))
            .await?;
            Ok(services::ApplicationResponse::Json(res))
        },
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "v1", any(feature = "olap", feature = "oltp")))]
#[instrument(skip_all, fields(flow = ?Flow::TokenizeCardBatch))]
pub async fn tokenize_card_batch_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    MultipartForm(form): MultipartForm<tokenize::CardNetworkTokenizeForm>,
) -> HttpResponse {
    let flow = Flow::TokenizeCardBatch;
    let (merchant_id, records) = match tokenize::get_tokenize_card_form_records(form) {
        Ok(res) => res,
        Err(e) => return api::log_and_return_error_response(e.into()),
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        records,
        |state, _, req, _| {
            let merchant_id = merchant_id.clone();
            async move {
                let (key_store, merchant_account) =
                    get_merchant_account(&state, &merchant_id).await?;
                let platform = domain::Platform::new(
                    merchant_account.clone(),
                    key_store.clone(),
                    merchant_account,
                    key_store,
                    None,
                );
                Box::pin(tokenize::tokenize_cards(
                    &state,
                    req,
                    platform.get_provider(),
                    platform.get_initiator(),
                ))
                .await
            }
        },
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodSessionCreate))]
pub async fn payment_methods_session_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<api_models::payment_methods::PaymentMethodSessionRequest>,
) -> HttpResponse {
    let flow = Flow::PaymentMethodSessionCreate;
    let payload = json_payload.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, request, _| async move {
            payment_methods_routes::payment_methods_session_create(
                state,
                auth.platform,
                auth.profile,
                request,
            )
            .await
        },
        &auth::V2ApiKeyAuth {
            allow_connected_scope_operation: true,
            allow_platform_self_operation: true,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodSessionUpdate))]
pub async fn payment_methods_session_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<id_type::GlobalPaymentMethodSessionId>,
    json_payload: web::Json<api_models::payment_methods::PaymentMethodsSessionUpdateRequest>,
) -> HttpResponse {
    let flow = Flow::PaymentMethodSessionUpdate;
    let payment_method_session_id = path.into_inner();
    let payload = json_payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            let value = payment_method_session_id.clone();
            async move {
                payment_methods_routes::payment_methods_session_update(
                    state,
                    auth.platform.get_provider().clone(),
                    value.clone(),
                    req,
                )
                .await
            }
        },
        &auth::V2ApiKeyAuth {
            allow_connected_scope_operation: true,
            allow_platform_self_operation: true,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodSessionRetrieve))]
pub async fn payment_methods_session_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<id_type::GlobalPaymentMethodSessionId>,
) -> HttpResponse {
    let flow = Flow::PaymentMethodSessionRetrieve;
    let payment_method_session_id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payment_method_session_id.clone(),
        |state, auth: auth::AuthenticationData, payment_method_session_id, _| async move {
            payment_methods_routes::payment_methods_session_retrieve(
                state,
                auth.platform.get_provider().clone(),
                payment_method_session_id,
            )
            .await
        },
        auth::sdk_or_api_or_client_auth(
            &auth::SdkAuthorizationAuth {
                allow_connected_scope_operation: true,
                allow_platform_self_operation: true,
                resource_id: common_utils::types::authentication::ResourceId::PaymentMethodSession(
                    payment_method_session_id.clone(),
                ),
            },
            &auth::V2ApiKeyAuth {
                allow_connected_scope_operation: true,
                allow_platform_self_operation: true,
            },
            &auth::V2ClientAuth(
                common_utils::types::authentication::ResourceId::PaymentMethodSession(
                    payment_method_session_id,
                ),
            ),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsList))]
pub async fn payment_method_session_list_payment_methods(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<id_type::GlobalPaymentMethodSessionId>,
) -> HttpResponse {
    let flow = Flow::PaymentMethodsList;
    let payment_method_session_id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payment_method_session_id.clone(),
        |state, auth: auth::AuthenticationData, payment_method_session_id, _| {
            payment_methods_routes::list_payment_methods_for_session(
                state,
                auth.platform,
                auth.profile,
                payment_method_session_id,
            )
        },
        auth::sdk_or_client_auth(
            &auth::SdkAuthorizationAuth {
                allow_connected_scope_operation: true,
                allow_platform_self_operation: true,
                resource_id: common_utils::types::authentication::ResourceId::PaymentMethodSession(
                    payment_method_session_id.clone(),
                ),
            },
            &auth::V2ClientAuth(
                common_utils::types::authentication::ResourceId::PaymentMethodSession(
                    payment_method_session_id,
                ),
            ),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, serde::Serialize)]
struct PaymentMethodsSessionGenericRequest<T: serde::Serialize> {
    payment_method_session_id: id_type::GlobalPaymentMethodSessionId,
    #[serde(flatten)]
    request: T,
}

#[cfg(feature = "v2")]
impl<T: serde::Serialize> common_utils::events::ApiEventMetric
    for PaymentMethodsSessionGenericRequest<T>
{
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::PaymentMethodSession {
            payment_method_session_id: self.payment_method_session_id.clone(),
        })
    }
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodSessionConfirm))]
pub async fn payment_method_session_confirm(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<id_type::GlobalPaymentMethodSessionId>,
    json_payload: web::Json<api_models::payment_methods::PaymentMethodSessionConfirmRequest>,
) -> HttpResponse {
    let flow = Flow::PaymentMethodSessionConfirm;
    let payload = json_payload.into_inner();
    let payment_method_session_id = path.into_inner();

    let request = PaymentMethodsSessionGenericRequest {
        payment_method_session_id: payment_method_session_id.clone(),
        request: payload,
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        request,
        |state, auth: auth::AuthenticationData, request, req_state| {
            payment_methods_routes::payment_methods_session_confirm(
                state,
                req_state,
                auth.platform,
                auth.profile,
                request.payment_method_session_id,
                request.request,
            )
        },
        auth::sdk_or_client_auth(
            &auth::SdkAuthorizationAuth {
                allow_connected_scope_operation: true,
                allow_platform_self_operation: true,
                resource_id: common_utils::types::authentication::ResourceId::PaymentMethodSession(
                    payment_method_session_id.clone(),
                ),
            },
            &auth::V2ClientAuth(
                common_utils::types::authentication::ResourceId::PaymentMethodSession(
                    payment_method_session_id,
                ),
            ),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodSessionUpdateSavedPaymentMethod))]
pub async fn payment_method_session_update_saved_payment_method(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<id_type::GlobalPaymentMethodSessionId>,
    json_payload: web::Json<
        api_models::payment_methods::PaymentMethodSessionUpdateSavedPaymentMethod,
    >,
) -> HttpResponse {
    let flow = Flow::PaymentMethodSessionUpdateSavedPaymentMethod;
    let payload = json_payload.into_inner();
    let payment_method_session_id = path.into_inner();

    let request = PaymentMethodsSessionGenericRequest {
        payment_method_session_id: payment_method_session_id.clone(),
        request: payload,
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        request,
        |state, auth: auth::AuthenticationData, request, _| {
            payment_methods_routes::payment_methods_session_update_payment_method(
                state,
                auth.platform,
                auth.profile,
                request.payment_method_session_id,
                request.request,
            )
        },
        auth::sdk_or_client_auth(
            &auth::SdkAuthorizationAuth {
                allow_connected_scope_operation: true,
                allow_platform_self_operation: true,
                resource_id: common_utils::types::authentication::ResourceId::PaymentMethodSession(
                    payment_method_session_id.clone(),
                ),
            },
            &auth::V2ClientAuth(
                common_utils::types::authentication::ResourceId::PaymentMethodSession(
                    payment_method_session_id,
                ),
            ),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodSessionUpdateSavedPaymentMethod))]
pub async fn payment_method_session_delete_saved_payment_method(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<id_type::GlobalPaymentMethodSessionId>,
    json_payload: web::Json<
        api_models::payment_methods::PaymentMethodSessionDeleteSavedPaymentMethod,
    >,
) -> HttpResponse {
    let flow = Flow::PaymentMethodSessionDeleteSavedPaymentMethod;
    let payload = json_payload.into_inner();
    let payment_method_session_id = path.into_inner();

    let request = PaymentMethodsSessionGenericRequest {
        payment_method_session_id: payment_method_session_id.clone(),
        request: payload,
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        request,
        |state, auth: auth::AuthenticationData, request, _| {
            payment_methods_routes::payment_methods_session_delete_payment_method(
                state,
                auth.platform,
                auth.profile,
                request.request.payment_method_token,
                request.payment_method_session_id,
            )
        },
        auth::sdk_or_client_auth(
            &auth::SdkAuthorizationAuth {
                allow_connected_scope_operation: true,
                allow_platform_self_operation: true,
                resource_id: common_utils::types::authentication::ResourceId::PaymentMethodSession(
                    payment_method_session_id.clone(),
                ),
            },
            &auth::V2ClientAuth(
                common_utils::types::authentication::ResourceId::PaymentMethodSession(
                    payment_method_session_id,
                ),
            ),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::NetworkTokenStatusCheck))]
pub async fn network_token_status_check_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<id_type::GlobalPaymentMethodId>,
) -> HttpResponse {
    let flow = Flow::NetworkTokenStatusCheck;
    let payment_method_id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payment_method_id,
        |state, auth: auth::AuthenticationData, payment_method_id, _| {
            payment_methods_routes::check_network_token_status(
                state,
                auth.platform.get_provider().clone(),
                payment_method_id,
            )
        },
        &auth::V2ApiKeyAuth {
            allow_connected_scope_operation: true,
            allow_platform_self_operation: true,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodGetTokenDetails))]
pub async fn payment_method_get_token_details_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::PaymentMethodGetTokenDetails;
    let temporary_token = path.into_inner();

    let api_auth = auth::V2ApiKeyAuth {
        allow_connected_scope_operation: true,
        allow_platform_self_operation: true,
    };

    let (auth_type, api_key_type) = match auth::check_internal_api_key_auth_no_client_secret(
        req.headers(),
        api_auth,
        state.conf.internal_merchant_id_profile_id_auth.clone(),
    ) {
        Ok(auth) => auth,
        Err(err) => return api::log_and_return_error_response(error_stack::report!(err)),
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, auth: auth::AuthenticationData, _, _| {
            let temporary_token = temporary_token.clone();
            async move {
                let platform: domain::Platform = auth.platform;
                payment_methods_routes::payment_method_get_token_details_core(
                    state,
                    platform.get_provider().clone(),
                    temporary_token,
                )
                .await
            }
        },
        &*auth_type,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
