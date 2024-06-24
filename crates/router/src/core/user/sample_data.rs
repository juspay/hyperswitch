use api_models::user::sample_data::SampleDataRequest;
use common_utils::errors::ReportSwitchExt;
use diesel_models::{user::sample_data::PaymentAttemptBatchNew, RefundNew};
use error_stack::ResultExt;
use hyperswitch_domain_models::payments::PaymentIntent;

pub type SampleDataApiResponse<T> = SampleDataResult<ApplicationResponse<T>>;

use crate::{
    core::errors::sample_data::{SampleDataError, SampleDataResult},
    routes::{app::ReqState, SessionState},
    services::{authentication::UserFromToken, ApplicationResponse},
    utils::user::sample_data::generate_sample_data,
};

pub async fn generate_sample_data_for_user(
    state: SessionState,
    user_from_token: UserFromToken,
    req: SampleDataRequest,
    _req_state: ReqState,
) -> SampleDataApiResponse<()> {
    let sample_data =
        generate_sample_data(&state, req, user_from_token.merchant_id.as_str()).await?;

    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            &user_from_token.merchant_id,
            &state.store.get_master_key().to_vec().into(),
        )
        .await
        .change_context(SampleDataError::InternalServerError)
        .attach_printable("Not able to fetch merchant key store")?; // If not able to fetch merchant key store for any reason, this should be an internal server error

    let (payment_intents, payment_attempts, refunds): (
        Vec<PaymentIntent>,
        Vec<PaymentAttemptBatchNew>,
        Vec<RefundNew>,
    ) = sample_data.into_iter().fold(
        (Vec::new(), Vec::new(), Vec::new()),
        |(mut pi, mut pa, mut rf), (payment_intent, payment_attempt, refund)| {
            pi.push(payment_intent);
            pa.push(payment_attempt);
            if let Some(refund) = refund {
                rf.push(refund);
            }
            (pi, pa, rf)
        },
    );

    state
        .store
        .insert_payment_intents_batch_for_sample_data(payment_intents, &key_store)
        .await
        .switch()?;
    state
        .store
        .insert_payment_attempts_batch_for_sample_data(payment_attempts)
        .await
        .switch()?;
    state
        .store
        .insert_refunds_batch_for_sample_data(refunds)
        .await
        .switch()?;

    Ok(ApplicationResponse::StatusOk)
}

pub async fn delete_sample_data_for_user(
    state: SessionState,
    user_from_token: UserFromToken,
    _req: SampleDataRequest,
    _req_state: ReqState,
) -> SampleDataApiResponse<()> {
    let merchant_id_del = user_from_token.merchant_id.as_str();

    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            &user_from_token.merchant_id,
            &state.store.get_master_key().to_vec().into(),
        )
        .await
        .change_context(SampleDataError::InternalServerError)
        .attach_printable("Not able to fetch merchant key store")?; // If not able to fetch merchant key store for any reason, this should be an internal server error

    state
        .store
        .delete_payment_intents_for_sample_data(merchant_id_del, &key_store)
        .await
        .switch()?;
    state
        .store
        .delete_payment_attempts_for_sample_data(merchant_id_del)
        .await
        .switch()?;
    state
        .store
        .delete_refunds_for_sample_data(merchant_id_del)
        .await
        .switch()?;

    Ok(ApplicationResponse::StatusOk)
}
