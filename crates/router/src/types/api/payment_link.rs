pub use api_models::payments::RetrievePaymentLinkResponse;

use crate::{
    core::{errors::RouterResult, payment_link},
    types::storage::{self},
};

#[async_trait::async_trait]
pub(crate) trait PaymentLinkResponseExt: Sized {
    async fn from_db_payment_link(payment_link: storage::PaymentLink) -> RouterResult<Self>;
}

#[async_trait::async_trait]
impl PaymentLinkResponseExt for RetrievePaymentLinkResponse {
    async fn from_db_payment_link(payment_link: storage::PaymentLink) -> RouterResult<Self> {
        let status = payment_link::check_payment_link_status(payment_link.fulfilment_time);
        Ok(Self {
            link_to_pay: payment_link.link_to_pay,
            payment_link_id: payment_link.payment_link_id,
            amount: payment_link.amount,
            description: payment_link.description,
            created_at: payment_link.created_at,
            merchant_id: payment_link.merchant_id,
            link_expiry: payment_link.fulfilment_time,
            currency: payment_link.currency,
            status,
        })
    }
}
