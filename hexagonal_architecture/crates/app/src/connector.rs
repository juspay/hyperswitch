use router_core::connector::Connector;
use router_core::types;

pub struct Stripe;

#[async_trait::async_trait]
impl Connector for Stripe {
    async fn create_payment(&self, payment: types::NewPayment) -> types::Payment {
        types::Payment { id: 0, amount: payment.amount }
    }

    async fn verify_payment(&self, _payment_id: u64) -> types::Verify {
        types::Verify::Ok
    }
}
