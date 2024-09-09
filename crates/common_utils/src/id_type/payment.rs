use crate::{
    consts::{self, MAX_GLOBAL_ID_LENGTH, MIN_GLOBAL_ID_LENGTH},
    errors::{CustomResult, ValidationError},
    generate_id_with_default_len,
    id_type::{global_id, AlphaNumericId, LengthId},
    impl_queryable_id_type,
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

#[cfg(feature = "metrics")]
/// This is implemented so that we can use payment id directly as attribute in metrics
impl From<PaymentId> for router_env::opentelemetry::Value {
    fn from(val: PaymentId) -> Self {
        let string_value = val.0 .0 .0;
        Self::String(router_env::opentelemetry::StringValue::from(string_value))
    }
}

/// A global id that can be used to identify a payment
#[derive(
    Debug,
    Clone,
    Hash,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    diesel::expression::AsExpression,
)]
#[diesel(sql_type = diesel::sql_types::Text)]
pub struct PaymentGlobalId(global_id::GlobalId);

impl_queryable_id_type!(PaymentGlobalId);

impl PaymentGlobalId {
    /// Get the hash key to be stored in redis
    pub fn get_hash_key_for_kv_store(&self) -> String {
        format!("pi_{}", self.0.get_string_repr())
    }

    /// Get the string representation of the global id
    pub fn get_string_repr(&self) -> &str {
        &self.0.get_string_repr()
    }
}

impl<DB> diesel::serialize::ToSql<diesel::sql_types::Text, DB> for PaymentGlobalId
where
    DB: diesel::backend::Backend,
    global_id::GlobalId: diesel::serialize::ToSql<diesel::sql_types::Text, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        self.0.to_sql(out)
    }
}

impl<DB> diesel::deserialize::FromSql<diesel::sql_types::Text, DB> for PaymentGlobalId
where
    DB: diesel::backend::Backend,
    global_id::GlobalId: diesel::deserialize::FromSql<diesel::sql_types::Text, DB>,
{
    fn from_sql(value: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        global_id::GlobalId::from_sql(value).map(Self)
    }
}
