pub mod types;

use actix_web::web;
use router_env::logger;

use super::app;
use crate::core::payment_methods::cards;

// #[actix_web::get("/migrate-legacy-to-basilisk")]
/// 
/// # Panics
/// 
/// When the merchant account or payment method or card reference is not found in the database.
#[allow (clippy::expect_used)]
pub async fn migrate_data_from_legacy_to_basilisk_hs(
    state: web::Data<app::AppState>,
    _req: actix_web::HttpRequest,
    json_payload: web::Json<types::MigrateLegacyToBasiliskRequest>,
) -> impl actix_web::Responder {
    logger::info!("migrate-legacy-to-basilisk was called");
    let request_data = json_payload.into_inner();
    let merchant_account = state
        .store
        .find_merchant_account_by_merchant_id(&request_data.merchant_id)
        .await
        .expect("Failed to fetch merchant account from db");
    let payment_method = state
        .store
        .find_payment_method_by_customer_id_merchant_id_list(
            request_data.customer_id.as_str(),
            request_data.merchant_id.as_str(),
        )
        .await
        .expect("Failed to fetch payment method from db")
        .pop()
        .expect("payment method not found");
    let card_reference = payment_method.token.expect("card reference not found");
    cards::migrate_data_from_legacy_to_basilisk_hs(
        &state,
        request_data.customer_id.as_str(),
        &merchant_account,
        card_reference.as_str(),
        merchant_account.locker_id.clone(),
    )
    .await
    .expect("Failed to migrate data from legacy to basilisk");

    actix_web::HttpResponse::Ok().body("migrate-legacy-to-basilisk is good")
}
