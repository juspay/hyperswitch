use crate::types;

#[mockall::automock]
#[async_trait::async_trait]
pub trait Connector: 'static {
    async fn create_payment(&self, payment: types::NewPayment) -> types::Payment;
    async fn verify_payment(&self, payment_id: u64) -> types::Verify;
}
