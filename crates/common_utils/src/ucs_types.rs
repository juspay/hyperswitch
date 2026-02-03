use crate::id_type;

/// Represents a reference ID for the Unified Connector Service (UCS).
///
/// This enum can hold either a payment reference ID or a refund reference ID,
/// allowing for a unified way to handle different types of transaction references
/// when interacting with the UCS.
#[derive(Debug)]
pub enum UcsReferenceId {
    /// A payment reference ID.
    ///
    /// This variant wraps a [`PaymentReferenceId`](id_type::PaymentReferenceId)
    /// and is used to identify a payment transaction within the UCS.
    Payment(id_type::PaymentReferenceId),
}

impl UcsReferenceId {
    /// Returns the string representation of the reference ID.
    ///
    /// This method matches the enum variant and calls the `get_string_repr`
    /// method of the underlying ID type (either `PaymentReferenceId` or `RefundReferenceId`)
    /// to get its string representation.
    ///
    /// # Returns
    ///
    /// A string slice (`&str`) representing the reference ID.
    pub fn get_string_repr(&self) -> &str {
        match self {
            Self::Payment(id) => id.get_string_repr(),
        }
    }
}

/// Represents a resource ID for the Unified Connector Service (UCS).
///
/// This enum can hold either a payment resource ID or a refund resource ID,
/// allowing for a unified way to handle different types of transaction resources
/// when interacting with the UCS.
#[derive(Debug)]
pub enum UcsResourceId {
    /// A payment resource ID.
    ///
    /// This variant wraps a [`PaymentResourceId`](id_type::PaymentResourceId)
    /// and is used to identify a payment transaction within the UCS.
    PaymentAttempt(id_type::PaymentResourceId),
    /// A refund resource ID.
    ///
    /// This variant wraps a [`RefundResourceId`](id_type::RefundResourceId)
    /// and is used to identify a refund transaction within the UCS.
    Refund(id_type::RefundReferenceId),
}

impl UcsResourceId {
    /// Returns the string representation of the resource ID.
    ///
    /// This method matches the enum variant and calls the `get_string_repr`
    /// method of the underlying ID type (either `PaymentResourceId` or `RefundResourceId`)
    /// to get its string representation.
    ///
    /// # Returns
    ///
    /// A string slice (`&str`) representing the resource ID.
    pub fn get_string_repr(&self) -> &str {
        match self {
            Self::PaymentAttempt(id) => id.get_string_repr(),
            Self::Refund(id) => id.get_string_repr(),
        }
    }
}