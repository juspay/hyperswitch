/// PaymentsPort is a trait that provides functions related to payments,
/// such as listing payments, creating payments, and finding payments by ID.
#[mockall::automock]
#[async_trait::async_trait]
pub trait PaymentsPort {
    /// list is a function that is part of the PaymentsPort trait.
    /// It is used to retrieve a list of payments from a source.
    async fn list(&self) -> Vec<crate::types::Payment>;
    /// Create is a function that is used to create a new payment.
    /// It takes in a payment object as an argument and returns a payment object.
    async fn create(&self, payment: crate::types::Payment) -> crate::types::Payment;
    /// find_by_id is a function that allows you to search for a payment by its unique identifier (id).
    /// It will return an Option type, which is either Some (the payment) or None (if the payment is not found).
    async fn find_by_id(&self, id: u64) -> Option<crate::types::Payment>;
}
