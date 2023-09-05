use error_stack::IntoReport;

use crate::{
    connection,
    core::errors::{self, CustomResult},
    types::storage,
};

#[async_trait::async_trait]
pub trait PaymentLinkInterface {
    async fn insert_payment_link(
        &self,
        payment_link: storage::paymentLinkNew
    )
}