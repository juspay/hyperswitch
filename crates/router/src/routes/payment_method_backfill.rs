use actix_multipart::form::MultipartForm;
use actix_web::{web, HttpRequest, HttpResponse};
use api_models::payment_method_backfill::PaymentMethodDataBackfillForm;
use router_env::{instrument, tracing, Flow};

use crate::{
    core::{payment_method_backfill,api_locking},
    routes::AppState,
    services::{api, authentication as auth},
    types::domain,
};

#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodDataBackfill))]
pub async fn payment_method_data_backfill(
    state: web::Data<AppState>,
    req: HttpRequest,
    MultipartForm(form): MultipartForm<PaymentMethodDataBackfillForm>,
) -> HttpResponse {
    let flow = Flow::PaymentMethodDataBackfill;
    
    let records = match form.validate_and_get_records() {
        Ok(records) => records,
        Err(e) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": e.to_string()
            }));
        }
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        records,
        |state, auth: auth::AuthenticationData, records, _req| {
        let merchant_context = domain::MerchantContext::NormalMerchant(Box::new(
                domain::Context(auth.merchant_account, auth.key_store),
            ));
            payment_method_backfill::payment_method_data_backfill(state, records, merchant_context, auth.profile)
        },
        &auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
