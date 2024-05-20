//! Concrete Id types as per the use case

use common_utils::{
    consts::MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH, id_type::MerchantReferenceId,
};

/// A type for customer_id that can be used in api models
/// This is not supposed to be used as a domain type because of the added validations when creating this type
///
/// Use case: Can be used in api models where we accept the reference id for customer
pub type CustomerId = MerchantReferenceId<MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH, 1>;
