use crate::{
    errors::{CustomResult, ValidationError},
    generate_id_with_default_len,
    id_type::{AlphaNumericId, LengthId},
};

crate::id_type!(
    PaymentId,
    "A type for payment_id that can be used for payment ids"
);
crate::impl_id_type_methods!(PaymentId, "payment_id");

// This is to display the `PaymentId` as PaymentId(abcd)
crate::impl_debug_id_type!(PaymentId);
crate::impl_default_id_type!(PaymentId, "pay");
crate::impl_try_from_cow_str_id_type!(PaymentId, "payment_id");

// Database related implementations so that this field can be used directly in the database tables
crate::impl_queryable_id_type!(PaymentId);
crate::impl_to_sql_from_sql_id_type!(PaymentId);

impl PaymentId {
    /// Get the hash key to be stored in redis
    pub fn get_hash_key_for_kv_store(&self) -> String {
        format!("pi_{}", self.0 .0 .0)
    }

    // This function should be removed once we have a better way to handle mandatory payment id in other flows
    /// Get payment id in the format of irrelevant_payment_id_in_{flow}
    pub fn get_irrelevant_id(flow: &str) -> Self {
        let alphanumeric_id =
            AlphaNumericId::new_unchecked(format!("irrelevant_payment_id_in_{flow}"));
        let id = LengthId::new_unchecked(alphanumeric_id);
        Self(id)
    }

    /// Get the attempt id for the payment id based on the attempt count
    pub fn get_attempt_id(&self, attempt_count: i16) -> String {
        format!("{}_{attempt_count}", self.get_string_repr())
    }

    /// Generate a client id for the payment id
    pub fn generate_client_secret(&self) -> String {
        generate_id_with_default_len(&format!("{}_secret", self.get_string_repr()))
    }

    /// Generate a key for pm_auth
    pub fn get_pm_auth_key(&self) -> String {
        format!("pm_auth_{}", self.get_string_repr())
    }

    /// Get external authentication request poll id
    pub fn get_external_authentication_request_poll_id(&self) -> String {
        format!("external_authentication_{}", self.get_string_repr())
    }

    /// Generate a test payment id with prefix test_
    pub fn generate_test_payment_id_for_sample_data() -> Self {
        let id = generate_id_with_default_len("test");
        let alphanumeric_id = AlphaNumericId::new_unchecked(id);
        let id = LengthId::new_unchecked(alphanumeric_id);
        Self(id)
    }

    /// Wrap a string inside PaymentId
    pub fn wrap(payment_id_string: String) -> CustomResult<Self, ValidationError> {
        Self::try_from(std::borrow::Cow::from(payment_id_string))
    }
}

crate::id_type!(PaymentReferenceId, "A type for payment_reference_id");
crate::impl_id_type_methods!(PaymentReferenceId, "payment_reference_id");

// This is to display the `PaymentReferenceId` as PaymentReferenceId(abcd)
crate::impl_debug_id_type!(PaymentReferenceId);
crate::impl_try_from_cow_str_id_type!(PaymentReferenceId, "payment_reference_id");

// Database related implementations so that this field can be used directly in the database tables
crate::impl_queryable_id_type!(PaymentReferenceId);
crate::impl_to_sql_from_sql_id_type!(PaymentReferenceId);

// This is implemented so that we can use payment id directly as attribute in metrics
#[cfg(feature = "metrics")]
impl From<PaymentId> for router_env::opentelemetry::Value {
    fn from(val: PaymentId) -> Self {
        Self::from(val.0 .0 .0)
    }
}

impl std::str::FromStr for PaymentReferenceId {
    type Err = error_stack::Report<ValidationError>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cow_string = std::borrow::Cow::Owned(s.to_string());
        Self::try_from(cow_string)
    }
}
