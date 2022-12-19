use crate::types;

/// ConnectorPort is an interface that provides two asynchronous functions: create_payment and verify_payment.
/// These functions allow a user to create and verify payments, respectively.
#[mockall::automock]
#[async_trait::async_trait]
pub trait ConnectorPort: 'static {
    /// create_payment is a function that is part of the traitConnectorPort.
    /// It takes in a payment of type types::NewPayment and returns a payment of type types::Payment.
    async fn create_payment(&self, payment: types::NewPayment) -> types::Payment;
    /// verify_payment is a function that is used to check the validity of a payment.
    /// It takes a payment ID as an argument and returns a type of verification.
    async fn verify_payment(&self, payment_id: u64) -> types::Verify;
}
