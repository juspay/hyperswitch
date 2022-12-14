use crate::types;

#[async_trait::async_trait]
pub trait Connector {
    async fn create_payment(&self, payment: types::NewPayment) -> types::Payment;
    async fn verify_payment(&self, payment_id: u64) -> types::Verify;
}

pub struct FakeStripe;

#[async_trait::async_trait]
impl Connector for FakeStripe {
    async fn create_payment(&self, payment: types::NewPayment) -> types::Payment {
        types::Payment { id: 0, amount: payment.amount }
    }

    async fn verify_payment(&self, _payment_id: u64) -> types::Verify {
        types::Verify::Ok
    }
}
