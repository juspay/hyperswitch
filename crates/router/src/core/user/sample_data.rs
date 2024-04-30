use api_models::user::sample_data::SampleDataRequest;
use common_utils::errors::ReportSwitchExt;
use data_models::payments::payment_intent::PaymentIntentNew;
use diesel_models::{user::sample_data::PaymentAttemptBatchNew, RefundNew};

pub type SampleDataApiResponse<T> = SampleDataResult<ApplicationResponse<T>>;

use crate::{
    core::errors::sample_data::SampleDataResult,
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

    let (payment_intents, payment_attempts, refunds): (
        Vec<PaymentIntentNew>,
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
        .insert_payment_intents_batch_for_sample_data(payment_intents)
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

    state
        .store
        .delete_payment_intents_for_sample_data(merchant_id_del)
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
