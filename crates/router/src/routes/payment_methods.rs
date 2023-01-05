use actix_web::{web, HttpRequest, HttpResponse};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::payment_methods::cards,
    services,
    services::api,
    types::api::payment_methods::{self, PaymentMethodId},
};

#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsCreate))]
// #[post("")]
pub async fn create_payment_method_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<payment_methods::CreatePaymentMethod>,
) -> HttpResponse {
    api::server_wrap(
        &state,
        &req,
        json_payload.into_inner(),
        |state, merchant_account, req| async move {
            let merchant_id = merchant_account.merchant_id.clone();

            cards::add_payment_method(state, req, merchant_id).await
        },
        api::MerchantAuthentication::ApiKey,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsList))]
//#[get("{merchant_id}")]
pub async fn list_payment_method_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    _merchant_id: web::Path<String>,
    json_payload: web::Query<payment_methods::ListPaymentMethodRequest>,
) -> HttpResponse {
    //let merchant_id = merchant_id.into_inner();
    let (payload, auth_type) =
        match api::get_auth_type_and_check_client_secret(&req, json_payload.into_inner()) {
            Ok(values) => values,
            Err(err) => return api::log_and_return_error_response(err),
        };

    api::server_wrap(
        &state,
        &req,
        payload,
        |state, merchant_account, req| {
            cards::list_payment_methods(&*state.store, merchant_account, req)
        },
        auth_type,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::CustomerPaymentMethodsList))]
// #[get("/{customer_id}/payment_methods")]
pub async fn list_customer_payment_method_api(
    state: web::Data<AppState>,
    customer_id: web::Path<(String,)>,
    req: HttpRequest,
    json_payload: web::Query<payment_methods::ListPaymentMethodRequest>,
) -> HttpResponse {
    let customer_id = customer_id.into_inner().0;

    let auth_type =
        match services::authenticate_eph_key(&req, &*state.store, customer_id.clone()).await {
            Ok(auth_type) => auth_type,
            Err(err) => return api::log_and_return_error_response(err),
        };

    api::server_wrap(
        &state,
        &req,
        json_payload.into_inner(),
        |state, merchant_account, _| {
            cards::list_customer_payment_method(state, merchant_account, &customer_id)
        },
        auth_type,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsRetrieve))]
// #[get("/{payment_method_id}")]
pub async fn payment_method_retrieve_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let payload = web::Json(PaymentMethodId {
        payment_method_id: path.into_inner(),
    })
    .into_inner();

    api::server_wrap(
        &state,
        &req,
        payload,
        |state, _, pm| cards::retrieve_payment_method(state, pm),
        api::MerchantAuthentication::ApiKey,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsUpdate))]
pub async fn payment_method_update_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    json_payload: web::Json<payment_methods::UpdatePaymentMethod>,
) -> HttpResponse {
    let payment_method_id = path.into_inner();

    api::server_wrap(
        &state,
        &req,
        json_payload.into_inner(),
        |state, merchant_account, payload| {
            cards::update_customer_payment_method(
                state,
                merchant_account,
                payload,
                &payment_method_id,
            )
        },
        api::MerchantAuthentication::ApiKey,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsDelete))]
// #[post("/{payment_method_id}/detach")]
pub async fn payment_method_delete_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    payment_method_id: web::Path<(String,)>,
) -> HttpResponse {
    let pm = PaymentMethodId {
        payment_method_id: payment_method_id.into_inner().0,
    };
    api::server_wrap(
        &state,
        &req,
        pm,
        cards::delete_payment_method,
        api::MerchantAuthentication::ApiKey,
    )
    .await
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use api_models::payment_methods::ListPaymentMethodRequest;

    use super::*;

    #[test]
    fn test_custom_list_deserialization() {
        let dummy_data = "amount=120&recurring_enabled=true&installment_payment_enabled=true&accepted_countries=US&accepted_countries=IN";
        let de_query: web::Query<ListPaymentMethodRequest> =
            web::Query::from_query(dummy_data).unwrap();
        let de_struct = de_query.into_inner();
        assert_eq!(
            de_struct.accepted_countries,
            Some(vec!["US".to_string(), "IN".to_string()])
        )
    }

    #[test]
    fn test_custom_list_deserialization_multi_amount() {
        let dummy_data = "amount=120&recurring_enabled=true&amount=1000";
        let de_query: Result<web::Query<ListPaymentMethodRequest>, _> =
            web::Query::from_query(dummy_data);
        assert!(de_query.is_err())
    }
}
