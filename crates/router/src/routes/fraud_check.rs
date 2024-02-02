use actix_web::{web, HttpRequest, HttpResponse};
use common_utils::events::{ApiEventMetric, ApiEventsType};
use router_env::Flow;

use crate::{
    core::{api_locking, fraud_check as frm_core},
    services::{self, api},
    types::fraud_check::FraudCheckResponseData,
    AppState,
};

/// Handles the fulfillment of a Flow Request Manager (FRM) fulfillment request. 
pub async fn frm_fulfillment(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<frm_core::types::FrmFulfillmentRequest>,
) -> HttpResponse {
    let flow = Flow::FrmFulfillment;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        json_payload.into_inner(),
        |state, auth, req| {
            frm_core::frm_fulfillment_core(state, auth.merchant_account, auth.key_store, req)
        },
        &services::authentication::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

impl ApiEventMetric for FraudCheckResponseData {
        /// Retrieves the API event type associated with the current instance.
    /// 
    /// # Returns
    /// 
    /// Returns an `Option` containing the API event type, or `None` if the event type is not available.
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::FraudCheck)
    }
}

impl ApiEventMetric for frm_core::types::FrmFulfillmentRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::FraudCheck)
    }
}
