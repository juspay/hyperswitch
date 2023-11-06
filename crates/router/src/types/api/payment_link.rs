pub use api_models::payments::PaymentLinkResponse;

use crate::{
    core::errors::RouterResult,
    types::storage::{self},
};

#[async_trait::async_trait]
pub(crate) trait PaymentLinkResponseExt: Sized {
    async fn from_db_payment_link(payment_link: storage::PaymentLink) -> RouterResult<Self>;
}

#[async_trait::async_trait]
impl PaymentLinkResponseExt for PaymentLinkResponse {
    async fn from_db_payment_link(payment_link: storage::PaymentLink) -> RouterResult<Self> {
        Ok(Self {
            link: payment_link.link_to_pay,
            payment_link_id: payment_link.payment_link_id,
        })
    }
}