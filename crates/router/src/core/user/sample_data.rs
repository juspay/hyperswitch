use api_models::user::sample_data::SampleDataRequest;
use common_utils::errors::ReportSwitchExt;
use diesel_models::{DisputeNew, RefundNew};
use error_stack::ResultExt;
use hyperswitch_domain_models::payments::PaymentIntent;

pub type SampleDataApiResponse<T> = SampleDataResult<ApplicationResponse<T>>;

use crate::{
    core::errors::sample_data::{SampleDataError, SampleDataResult},
    routes::{app::ReqState, SessionState},
    services::{authentication::UserFromToken, ApplicationResponse},
    utils,
};

#[cfg(feature = "v1")]
pub async fn generate_sample_data_for_user(
    state: SessionState,
    user_from_token: UserFromToken,
    req: SampleDataRequest,
    _req_state: ReqState,
) -> SampleDataApiResponse<()> {
    let sample_data = utils::user::sample_data::generate_sample_data(
        &state,
        req,
        &user_from_token.merchant_id,
        &user_from_token.org_id,
    )
    .await?;

    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            &(&state).into(),
            &user_from_token.merchant_id,
            &state.store.get_master_key().to_vec().into(),
        )
        .await
        .change_context(SampleDataError::InternalServerError)
        .attach_printable("Not able to fetch merchant key store")?; // If not able to fetch merchant key store for any reason, this should be an internal server error

    let (payment_intents, payment_attempts, refunds, disputes): (
        Vec<PaymentIntent>,
        Vec<diesel_models::user::sample_data::PaymentAttemptBatchNew>,
        Vec<RefundNew>,
        Vec<DisputeNew>,
    ) = sample_data.into_iter().fold(
        (Vec::new(), Vec::new(), Vec::new(), Vec::new()),
        |(mut pi, mut pa, mut rf, mut dp), (payment_intent, payment_attempt, refund, dispute)| {
            pi.push(payment_intent);
            pa.push(payment_attempt);
            if let Some(refund) = refund {
                rf.push(refund);
            }
            if let Some(dispute) = dispute {
                dp.push(dispute);
            }
            (pi, pa, rf, dp)
        },
    );

    state
        .store
        .insert_payment_intents_batch_for_sample_data(&(&state).into(), payment_intents, &key_store)
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
    state
        .store
        .insert_disputes_batch_for_sample_data(disputes)
        .await
        .switch()?;

    Ok(ApplicationResponse::StatusOk)
}

#[cfg(feature = "v1")]
pub async fn delete_sample_data_for_user(
    state: SessionState,
    user_from_token: UserFromToken,
    _req: SampleDataRequest,
    _req_state: ReqState,
) -> SampleDataApiResponse<()> {
    let merchant_id_del = user_from_token.merchant_id;
    let key_manager_state = &(&state).into();
    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            key_manager_state,
            &merchant_id_del,
            &state.store.get_master_key().to_vec().into(),
        )
        .await
        .change_context(SampleDataError::InternalServerError)
        .attach_printable("Not able to fetch merchant key store")?; // If not able to fetch merchant key store for any reason, this should be an internal server error

    state
        .store
        .delete_payment_intents_for_sample_data(key_manager_state, &merchant_id_del, &key_store)
        .await
        .switch()?;
    state
        .store
        .delete_payment_attempts_for_sample_data(&merchant_id_del)
        .await
        .switch()?;
    state
        .store
        .delete_refunds_for_sample_data(&merchant_id_del)
        .await
        .switch()?;
    state
        .store
        .delete_disputes_for_sample_data(&merchant_id_del)
        .await
        .switch()?;

    Ok(ApplicationResponse::StatusOk)
}
