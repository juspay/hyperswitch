use actix_web::{web, HttpRequest, HttpResponse};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::payment_methods::cards,
    services::{api, authentication as auth},
    types::api::payment_methods::{self, PaymentMethodId},
};

// PaymentMethods - Create

///
/// To create a payment method against a customer object. In case of cards, this API could be used only by PCI compliant merchants
#[utoipa::path(
    post,
    path = "/payment_methods",
    request_body = CreatePaymentMethod,
    responses(
        (status = 200, description = "Payment Method Created", body = PaymentMethodResponse),
        (status = 400, description = "Invalid Data")
    ),
    tag = "Payment Methods",
    operation_id = "Create a Payment Method"
)]
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsCreate))]
pub async fn create_payment_method_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<payment_methods::CreatePaymentMethod>,
) -> HttpResponse {
    api::server_wrap(
        state.get_ref(),
        &req,
        json_payload.into_inner(),
        |state, merchant_account, req| async move {
            cards::add_payment_method(state, req, &merchant_account).await
        },
        &auth::ApiKeyAuth,
    )
    .await
}

// List payment methods for a Merchant

///
/// To filter and list the applicable payment methods for a particular Merchant ID
#[utoipa::path(
    get,
    path = "/payment_methods/{account_id}",
    params (
        ("account_id" = String, Path, description = "The unique identifier for the merchant account"),
        ("accepted_country" = Vec<String>, Query, description = "The two-letter ISO currency code"),
        ("accepted_currency" = Vec<Currency>, Path, description = "The three-letter ISO currency code"),
        ("minimum_amount" = i64, Query, description = "The minimum amount accepted for processing by the particular payment method."),
        ("maximum_amount" = i64, Query, description = "The maximum amount amount accepted for processing by the particular payment method."),
        ("recurring_payment_enabled" = bool, Query, description = "Indicates whether the payment method is eligible for recurring payments"),
        ("installment_payment_enabled" = bool, Query, description = "Indicates whether the payment method is eligible for installment payments"),
    ),
    responses(
        (status = 200, description = "Payment Methods retrieved", body = ListPaymentMethodResponse),
        (status = 400, description = "Invalid Data"),
        (status = 404, description = "Payment Methods does not exist in records")
    ),
    tag = "Payment Methods",
    operation_id = "List all Payment Methods for a Merchant"
)]
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsList))]
pub async fn list_payment_method_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Query<payment_methods::ListPaymentMethodRequest>,
) -> HttpResponse {
    let payload = json_payload.into_inner();

    let (auth, _) = match auth::check_client_secret_and_get_auth(req.headers(), &payload) {
        Ok((auth, _auth_flow)) => (auth, _auth_flow),
        Err(e) => return api::log_and_return_error_response(e),
    };

    api::server_wrap(
        state.get_ref(),
        &req,
        payload,
        |state, merchant_account, req| {
            cards::list_payment_methods(&*state.store, merchant_account, req)
        },
        &*auth,
    )
    .await
}

// List payment methods for a Customer

///
/// To filter and list the applicable payment methods for a particular Customer ID
#[utoipa::path(
    get,
    path = "/payment_methods/{customer_id}",
    params (
        ("customer_id" = String, Path, description = "The unique identifier for the customer account"),
        ("accepted_country" = Vec<String>, Query, description = "The two-letter ISO currency code"),
        ("accepted_currency" = Vec<Currency>, Path, description = "The three-letter ISO currency code"),
        ("minimum_amount" = i64, Query, description = "The minimum amount accepted for processing by the particular payment method."),
        ("maximum_amount" = i64, Query, description = "The maximum amount amount accepted for processing by the particular payment method."),
        ("recurring_payment_enabled" = bool, Query, description = "Indicates whether the payment method is eligible for recurring payments"),
        ("installment_payment_enabled" = bool, Query, description = "Indicates whether the payment method is eligible for installment payments"),
    ),
    responses(
        (status = 200, description = "Payment Methods retrieved", body = ListCustomerPaymentMethodsResponse),
        (status = 400, description = "Invalid Data"),
        (status = 404, description = "Payment Methods does not exist in records")
    ),
    tag = "Payment Methods",
    operation_id = "List all Payment Methods for a Customer"
)]
#[instrument(skip_all, fields(flow = ?Flow::CustomerPaymentMethodsList))]
pub async fn list_customer_payment_method_api(
    state: web::Data<AppState>,
    customer_id: web::Path<(String,)>,
    req: HttpRequest,
    json_payload: web::Query<payment_methods::ListPaymentMethodRequest>,
) -> HttpResponse {
    let customer_id = customer_id.into_inner().0;

    let auth_type = match auth::is_ephemeral_auth(req.headers(), &*state.store, &customer_id).await
    {
        Ok(auth_type) => auth_type,
        Err(err) => return api::log_and_return_error_response(err),
    };

    api::server_wrap(
        state.get_ref(),
        &req,
        json_payload.into_inner(),
        |state, merchant_account, _| {
            cards::list_customer_payment_method(state, merchant_account, &customer_id)
        },
        &*auth_type,
    )
    .await
}

// Payment Method - Retrieve

///
/// To retrieve a payment method
#[utoipa::path(
    get,
    path = "/payment_methods/{method_id}",
    params (
        ("method_id" = String, Path, description = "The unique identifier for the Payment Method"),
    ),
    responses(
        (status = 200, description = "Payment Method retrieved", body = PaymentMethodResponse),
        (status = 404, description = "Payment Method does not exist in records")
    ),
    tag = "Payment Methods",
    operation_id = "Retrieve a Payment method"
)]
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsRetrieve))]
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
        state.get_ref(),
        &req,
        payload,
        |state, merchant_account, pm| cards::retrieve_payment_method(state, pm, merchant_account),
        &auth::ApiKeyAuth,
    )
    .await
}

// Payment Method - Update

///
/// To update an existing payment method attached to a customer object. This API is useful for use cases such as updating the card number for expired cards to prevent discontinuity in recurring payments
#[utoipa::path(
    post,
    path = "/payment_methods/{method_id}",
    params (
        ("method_id" = String, Path, description = "The unique identifier for the Payment Method"),
    ),
    request_body = UpdatePaymentMethod,
    responses(
        (status = 200, description = "Payment Method updated", body = PaymentMethodResponse),
        (status = 404, description = "Payment Method does not exist in records")
    ),
    tag = "Payment Methods",
    operation_id = "Update a Payment method"
)]
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsUpdate))]
pub async fn payment_method_update_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    json_payload: web::Json<payment_methods::UpdatePaymentMethod>,
) -> HttpResponse {
    let payment_method_id = path.into_inner();

    api::server_wrap(
        state.get_ref(),
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
        &auth::ApiKeyAuth,
    )
    .await
}

// Payment Method - Delete

///
/// Delete payment method
#[utoipa::path(
    delete,
    path = "/payment_methods/{method_id}",
    params (
        ("method_id" = String, Path, description = "The unique identifier for the Payment Method"),
    ),
    responses(
        (status = 200, description = "Payment Method deleted", body = DeletePaymentMethodResponse),
        (status = 404, description = "Payment Method does not exist in records")
    ),
    tag = "Payment Methods",
    operation_id = "Delete a Payment method"
)]
#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsDelete))]
pub async fn payment_method_delete_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    payment_method_id: web::Path<(String,)>,
) -> HttpResponse {
    let pm = PaymentMethodId {
        payment_method_id: payment_method_id.into_inner().0,
    };
    api::server_wrap(
        state.get_ref(),
        &req,
        pm,
        cards::delete_payment_method,
        &auth::ApiKeyAuth,
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
        let dummy_data = "amount=120&recurring_enabled=true&installment_payment_enabled=true";
        let de_query: web::Query<ListPaymentMethodRequest> =
            web::Query::from_query(dummy_data).unwrap();
        let de_struct = de_query.into_inner();
        assert_eq!(de_struct.installment_payment_enabled, Some(true))
    }

    #[test]
    fn test_custom_list_deserialization_multi_amount() {
        let dummy_data = "amount=120&recurring_enabled=true&amount=1000";
        let de_query: Result<web::Query<ListPaymentMethodRequest>, _> =
            web::Query::from_query(dummy_data);
        assert!(de_query.is_err())
    }
}
