#[cfg(all(
    any(feature = "v1", feature = "v2", feature = "olap", feature = "oltp"),
    not(feature = "customer_v2")
))]
use actix_multipart::form::MultipartForm;
use actix_web::{web, HttpRequest, HttpResponse};
use common_utils::{errors::CustomResult, id_type};
use diesel_models::enums::IntentStatus;
use error_stack::ResultExt;
use hyperswitch_domain_models::merchant_key_store::MerchantKeyStore;
use router_env::{instrument, logger, tracing, Flow};

use super::app::{AppState, SessionState};
use crate::{
    core::{
        api_locking,
        errors::{self, utils::StorageErrorExt},
        payment_methods::{self as payment_methods_routes, cards},
    },
    services::{api, authentication as auth, authorization::permissions::Permission},
    types::{
        api::payment_methods::{self, PaymentMethodId},
        domain,
        storage::payment_method::PaymentTokenData,
    },
};
#[cfg(all(
    any(feature = "v1", feature = "v2", feature = "olap", feature = "oltp"),
    not(feature = "customer_v2")
))]
use crate::{
    core::{customers, payment_methods::migration},
    types::api::customers::CustomerRequest,
};

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
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
                &auth.merchant_account,
                &auth.key_store,
            ))
            .await
        },
        &auth::HeaderAuth(auth::ApiKeyAuth),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
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
            Box::pin(payment_methods_routes::create_payment_method(
                &state,
                req,
                &auth.merchant_account,
                &auth.key_store,
                &auth.profile,
            ))
            .await
        },
        &auth::HeaderAuth(auth::ApiKeyAuth),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
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
                &auth.merchant_account,
                &auth.key_store,
            ))
            .await
        },
        &auth::V2ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// This struct is used internally only
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct PaymentMethodIntentConfirmInternal {
    pub id: id_type::GlobalPaymentMethodId,
    pub request: payment_methods::PaymentMethodIntentConfirm,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
impl From<PaymentMethodIntentConfirmInternal> for payment_methods::PaymentMethodIntentConfirm {
    fn from(item: PaymentMethodIntentConfirmInternal) -> Self {
        item.request
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
impl common_utils::events::ApiEventMetric for PaymentMethodIntentConfirmInternal {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::PaymentMethod {
            payment_method_id: self.id.clone(),
            payment_method_type: Some(self.request.payment_method_type),
            payment_method_subtype: Some(self.request.payment_method_subtype),
        })
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
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

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            payment_methods_routes::update_payment_method(
                state,
                auth.merchant_account,
                auth.key_store,
                req,
                &payment_method_id,
            )
        },
        &auth::V2ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
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
        |state, auth: auth::AuthenticationData, pm, _| {
            payment_methods_routes::retrieve_payment_method(
                state,
                pm,
                auth.key_store,
                auth.merchant_account,
            )
        },
        &auth::HeaderAuth(auth::ApiKeyAuth),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
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

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, pm, _| {
            payment_methods_routes::delete_payment_method(
                state,
                pm,
                auth.key_store,
                auth.merchant_account,
            )
        },
        &auth::HeaderAuth(auth::ApiKeyAuth),
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
            Box::pin(cards::migrate_payment_method(
                state,
                req,
                &merchant_id,
                &merchant_account,
                &key_store,
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
    let key_manager_state = &state.into();
    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            key_manager_state,
            merchant_id,
            &state.store.get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let merchant_account = state
        .store
        .find_merchant_account_by_merchant_id(key_manager_state, merchant_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;
    Ok((key_store, merchant_account))
}

#[cfg(all(
    any(feature = "v1", feature = "v2", feature = "olap", feature = "oltp"),
    not(feature = "customer_v2")
))]
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsMigrate))]
pub async fn migrate_payment_methods(
    state: web::Data<AppState>,
    req: HttpRequest,
    MultipartForm(form): MultipartForm<migration::PaymentMethodsMigrateForm>,
) -> HttpResponse {
    let flow = Flow::PaymentMethodsMigrate;
    let (merchant_id, records, merchant_connector_id) =
        match migration::get_payment_method_records(form) {
            Ok((merchant_id, records, merchant_connector_id)) => {
                (merchant_id, records, merchant_connector_id)
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
            let merchant_connector_id = merchant_connector_id.clone();
            async move {
                let (key_store, merchant_account) =
                    get_merchant_account(&state, &merchant_id).await?;
                // Create customers if they are not already present
                customers::migrate_customers(
                    state.clone(),
                    req.iter()
                        .map(|e| CustomerRequest::from((e.clone(), merchant_id.clone())))
                        .collect(),
                    merchant_account.clone(),
                    key_store.clone(),
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)?;
                Box::pin(migration::migrate_payment_methods(
                    state,
                    req,
                    &merchant_id,
                    &merchant_account,
                    &key_store,
                    merchant_connector_id,
                ))
                .await
            }
        },
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
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
    let (auth, _) = match auth::check_client_secret_and_get_auth(req.headers(), &payload) {
        Ok((auth, _auth_flow)) => (auth, _auth_flow),
        Err(e) => return api::log_and_return_error_response(e),
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            Box::pin(cards::add_payment_method_data(
                state,
                req,
                auth.merchant_account,
                auth.key_store,
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
    let (auth, _) = match auth::check_client_secret_and_get_auth(req.headers(), &payload) {
        Ok((auth, _auth_flow)) => (auth, _auth_flow),
        Err(e) => return api::log_and_return_error_response(e),
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            // TODO (#7195): Fill platform_merchant_account in the client secret auth and pass it here.
            cards::list_payment_methods(state, auth.merchant_account, auth.key_store, req)
        },
        &*auth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(
    any(feature = "v2", feature = "v1"),
    not(feature = "payment_methods_v2"),
    not(feature = "customer_v2")
))]
/// List payment methods for a Customer
///
/// To filter and list the applicable payment methods for a particular Customer ID
#[utoipa::path(
    get,
    path = "/customers/{customer_id}/payment_methods",
    params (
        ("accepted_country" = Vec<String>, Query, description = "The two-letter ISO currency code"),
        ("accepted_currency" = Vec<Currency>, Path, description = "The three-letter ISO currency code"),
        ("minimum_amount" = i64, Query, description = "The minimum amount accepted for processing by the particular payment method."),
        ("maximum_amount" = i64, Query, description = "The maximum amount amount accepted for processing by the particular payment method."),
        ("recurring_payment_enabled" = bool, Query, description = "Indicates whether the payment method is eligible for recurring payments"),
        ("installment_payment_enabled" = bool, Query, description = "Indicates whether the payment method is eligible for installment payments"),
    ),
    responses(
        (status = 200, description = "Payment Methods retrieved", body = CustomerPaymentMethodsListResponse),
        (status = 400, description = "Invalid Data"),
        (status = 404, description = "Payment Methods does not exist in records")
    ),
    tag = "Payment Methods",
    operation_id = "List all Payment Methods for a Customer",
    security(("api_key" = []))
)]
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

    let ephemeral_auth = match auth::is_ephemeral_auth(req.headers()) {
        Ok(auth) => auth,
        Err(err) => return api::log_and_return_error_response(err),
    };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            cards::do_list_customer_pm_fetch_customer_if_not_passed(
                state,
                auth.merchant_account,
                auth.key_store,
                Some(req),
                Some(&customer_id),
                None,
            )
        },
        &*ephemeral_auth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(
    any(feature = "v2", feature = "v1"),
    not(feature = "payment_methods_v2"),
    not(feature = "customer_v2")
))]
/// List payment methods for a Customer
///
/// To filter and list the applicable payment methods for a particular Customer ID
#[utoipa::path(
    get,
    path = "/customers/payment_methods",
    params (
        ("client-secret" = String, Path, description = "A secret known only to your application and the authorization server"),
        ("accepted_country" = Vec<String>, Query, description = "The two-letter ISO currency code"),
        ("accepted_currency" = Vec<Currency>, Path, description = "The three-letter ISO currency code"),
        ("minimum_amount" = i64, Query, description = "The minimum amount accepted for processing by the particular payment method."),
        ("maximum_amount" = i64, Query, description = "The maximum amount amount accepted for processing by the particular payment method."),
        ("recurring_payment_enabled" = bool, Query, description = "Indicates whether the payment method is eligible for recurring payments"),
        ("installment_payment_enabled" = bool, Query, description = "Indicates whether the payment method is eligible for installment payments"),
    ),
    responses(
        (status = 200, description = "Payment Methods retrieved for customer tied to its respective client-secret passed in the param", body = CustomerPaymentMethodsListResponse),
        (status = 400, description = "Invalid Data"),
        (status = 404, description = "Payment Methods does not exist in records")
    ),
    tag = "Payment Methods",
    operation_id = "List all Payment Methods for a Customer",
    security(("publishable_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::CustomerPaymentMethodsList))]
pub async fn list_customer_payment_method_api_client(
    state: web::Data<AppState>,
    req: HttpRequest,
    query_payload: web::Query<payment_methods::PaymentMethodListRequest>,
) -> HttpResponse {
    let flow = Flow::CustomerPaymentMethodsList;
    let payload = query_payload.into_inner();
    let api_key = auth::get_api_key(req.headers()).ok();
    let (auth, _, is_ephemeral_auth) =
        match auth::get_ephemeral_or_other_auth(req.headers(), false, Some(&payload)).await {
            Ok((auth, _auth_flow, is_ephemeral_auth)) => (auth, _auth_flow, is_ephemeral_auth),
            Err(e) => return api::log_and_return_error_response(e),
        };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            cards::do_list_customer_pm_fetch_customer_if_not_passed(
                state,
                auth.merchant_account,
                auth.key_store,
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
            payment_methods_routes::initiate_pm_collect_link(
                state,
                auth.merchant_account,
                auth.key_store,
                req,
            )
        },
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
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
                auth.merchant_account,
                auth.key_store,
                req,
            )
        },
        &auth::MerchantIdAuth(merchant_id),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
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
        |state, auth: auth::AuthenticationData, pm, _| {
            cards::retrieve_payment_method(state, pm, auth.key_store, auth.merchant_account)
        },
        &auth::HeaderAuth(auth::ApiKeyAuth),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
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

    let (auth, _) = match auth::check_client_secret_and_get_auth(req.headers(), &payload) {
        Ok((auth, _auth_flow)) => (auth, _auth_flow),
        Err(e) => return api::log_and_return_error_response(e),
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            cards::update_customer_payment_method(
                state,
                auth.merchant_account,
                req,
                &payment_method_id,
                auth.key_store,
            )
        },
        &*auth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
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
    let ephemeral_auth = match auth::is_ephemeral_auth(req.headers()) {
        Ok(auth) => auth,
        Err(err) => return api::log_and_return_error_response(err),
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        pm,
        |state, auth: auth::AuthenticationData, req, _| {
            cards::delete_payment_method(state, auth.merchant_account, req, auth.key_store)
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
                auth.profile_id,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::ProfileConnectorWrite,
            },
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth {
            permission: Permission::ProfileConnectorWrite,
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

    let ephemeral_auth = match auth::is_ephemeral_auth(req.headers()) {
        Ok(auth) => auth,
        Err(err) => return api::log_and_return_error_response(err),
    };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, default_payment_method, _| async move {
            cards::set_default_payment_method(
                &state,
                auth.merchant_account.get_id(),
                auth.key_store,
                customer_id,
                default_payment_method.payment_method_id,
                auth.merchant_account.storage_scheme,
            )
            .await
        },
        &*ephemeral_auth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
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
    pub fn create_key_for_token(
        (parent_pm_token, payment_method): (&String, api_models::enums::PaymentMethod),
    ) -> Self {
        Self {
            key_for_token: format!(
                "pm_token_{}_{}_hyperswitch",
                parent_pm_token, payment_method
            ),
        }
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
                auth.merchant_account,
                auth.key_store,
                request,
            )
            .await
        },
        &auth::V2ApiKeyAuth,
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
                auth.merchant_account,
                auth.key_store,
                payment_method_session_id,
            )
            .await
        },
        auth::api_or_client_auth(
            &auth::V2ApiKeyAuth,
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

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
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
                auth.merchant_account,
                auth.key_store,
                auth.profile,
                payment_method_session_id,
            )
        },
        &auth::V2ClientAuth(
            common_utils::types::authentication::ResourceId::PaymentMethodSession(
                payment_method_session_id,
            ),
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

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
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
                auth.merchant_account,
                auth.key_store,
                request.payment_method_session_id,
                request.request,
            )
        },
        &auth::V2ClientAuth(
            common_utils::types::authentication::ResourceId::PaymentMethodSession(
                payment_method_session_id,
            ),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
