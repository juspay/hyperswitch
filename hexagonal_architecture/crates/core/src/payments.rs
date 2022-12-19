use crate::connector::ConnectorPort;
use crate::store::PaymentsPort;
use crate::types::{self, Verify};

/// It is used to retrieve a list of payments from a source.
pub async fn list(payments: &impl PaymentsPort) -> Vec<types::Payment> {
    payments.list().await
}

/// Create is a function that is used to create a payment.
/// It takes in two parameters, payments and connector, and a payment object.
/// It then uses the connector to create the payment and stores it in the payments.
pub async fn create(
    payments: &impl PaymentsPort,
    connector: &impl ConnectorPort,
    payment: types::NewPayment,
) -> types::Payment {
    let payment = connector.create_payment(payment).await;
    payments.create(payment).await
}

/// Confirm is a function that is used to verify a payment.
/// It takes in two parameters, payments and connector, and a payment ID.
/// It then checks if the payment exists in the payments database and if it does, it verifies the payment using the connector.
/// If the payment does not exist, it returns an error message.
pub async fn confirm(
    payments: &impl PaymentsPort,
    connector: &impl ConnectorPort,
    payment_id: u64,
) -> types::Verify {
    match payments.find_by_id(payment_id).await {
        Some(payment) => connector.verify_payment(payment.id).await,
        None => Verify::Error { message: String::from("404") },
    }
}

#[cfg(test)]
mod tests {
    /// Mockall is a powerful library for Rust that allows you to create mock versions of traits or structs.
    /// This means that you can use them in unit tests as a substitute for the real object.
    /// It can be used in two ways: with #[automock] or with mock!.
    /// The mock structs have the same name as the original, but with "Mock" prepended.
    /// When you use it, you set expectations on the mock struct and supply it to the code you are testing.
    /// It will then return the preprogrammed return values and any accesses that don't match your expectations will cause a panic.

    /// Mock is a type of Test Double, which is a generic term used for simplified objects that look and behave like their production equivalents.
    /// Mocks are used in automated testing to reduce complexity and allow code to be verified independently from the rest of the system.
    /// Misunderstanding and mixing test doubles implementation can lead to test fragility and make refactoring difficult.
    use crate::connector::MockConnectorPort;
    use crate::store::MockPaymentsPort;
    use crate::types::*;

    /// MockPaymentsPort is a type of Test Double, which is a simplified object that looks and behaves like the real thing.
    /// It is used in automated testing to reduce complexity and allow code to be tested independently from the rest of the system.
    /// In this case, it is used to create a payment.

    #[tokio::test]
    // payment_create is a function that is used to create a payment.
    // It is used to set up the payment details, such as the amount, the recipient, and any other necessary information.
    // It is usually used in unit tests to simulate a real payment.
    async fn payment_create() {
        let mut payments = MockPaymentsPort::new();
        let mut connector = MockConnectorPort::new();

        // return_once is a method used to set an expectation's return value with a constant that is not cloneable.
        // It allows the expectation to return the same value each time it is called.
        payments.expect_create().return_once(|new| Payment { id: 42, amount: new.amount });
        payments.expect_find_by_id().return_once(|id| Payment { id, amount: 15 }.into());

        connector.expect_create_payment().return_once(|new| Payment { id: 42, amount: new.amount });
        connector.expect_verify_payment().return_once(|_| Verify::Ok);

        let json = NewPayment { amount: 15 };
        let payment = super::create(&payments, &connector, json).await;

        assert_eq!(payment.id, 42);
        assert_eq!(payment.amount, 15);

        assert_eq!(super::confirm(&payments, &connector, payment.id).await, Verify::Ok);
    }

    #[tokio::test]
    async fn payment_list() {
        let mut payments = MockPaymentsPort::new();
        payments.expect_list().return_once(|| vec![Payment { id: 0, amount: 15 }]);

        assert_eq!(super::list(&payments).await, vec![Payment { id: 0, amount: 15 }]);
    }
}
