use frunk::LabelledGeneric;
use serde::{Deserialize, Serialize};

/// NewPayment is a structure that contains a single field called amount,
/// which is a number (u64) that represents a monetary value.
#[derive(Debug, Serialize, Deserialize, Default, LabelledGeneric)]
pub struct NewPayment {
    /// Amount is a numerical value that is used to represent the size of a payment.
    /// It is typically used to measure the amount of money being exchanged in a transaction.
    pub amount: u64,
}

/// It is composed of two parts: an id and an amount. The id is a unique number that identifies the payment, while the amount is the amount of money associated with the payment.
/// The Payment structure can be used to store information about payments made by a user.
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq, Eq, LabelledGeneric)]
pub struct Payment {
    /// id is a unique identifier that is used to identify a particular payment.
    /// It is usually a number that is assigned to a payment so that it can be tracked and identified.
    pub id: u64,
    /// Amount is a numerical value that is used to represent the size of a payment.
    ///  It is typically used to measure the amount of money being exchanged in a transaction.
    pub amount: u64,
}

/// Verify is an enum (enumeration) which is a data type that consists of a set of named values.
/// It is used to represent a set of possible values that a variable can take.
/// In this case, the enum is used to represent the result of a verification process,
/// with the possible values being "Ok" or "Error" with an associated message.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, frunk::LabelledGeneric)]
pub enum Verify {
    /// Ok is a variant used to indicate that something has been successful.
    /// In this context, it is used to indicate that the payment has been verified.
    Ok,
    /// Error is a type of value that is used to indicate that something has gone wrong.
    Error {
        /// Message is a type of data that is used to store a string of characters.
        /// It is used in the context of the paragraph to store an error message associated with the Verify enum.
        message: String,
    },
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_ser_de() {
        use serde_test::{assert_tokens, Token::*};

        let verify = super::Verify::Error { message: "hello".into() };

        assert_tokens(
            &verify,
            &[
                StructVariant { name: "Verify", variant: "Error", len: 1 },
                Str("message"),
                Str("hello"),
                StructVariantEnd,
            ],
        );
    }
}
