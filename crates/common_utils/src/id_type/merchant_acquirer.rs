use std::str::FromStr;

crate::id_type!(
    MerchantAcquirerId,
    "A type for merchant_acquirer_id that can be used for merchant acquirer ids"
);
crate::impl_id_type_methods!(MerchantAcquirerId, "merchant_acquirer_id");

// This is to display the `MerchantAcquirerId` as MerchantAcquirerId(abcd)
crate::impl_debug_id_type!(MerchantAcquirerId);
crate::impl_try_from_cow_str_id_type!(MerchantAcquirerId, "merchant_acquirer_id");

crate::impl_generate_id_id_type!(MerchantAcquirerId, "mer_acq");
crate::impl_serializable_secret_id_type!(MerchantAcquirerId);
crate::impl_queryable_id_type!(MerchantAcquirerId);
crate::impl_to_sql_from_sql_id_type!(MerchantAcquirerId);

impl crate::events::ApiEventMetric for MerchantAcquirerId {
    fn get_api_event_type(&self) -> Option<crate::events::ApiEventsType> {
        Some(crate::events::ApiEventsType::MerchantAcquirer {
            merchant_acquirer_id: self.clone(),
        })
    }
}

impl FromStr for MerchantAcquirerId {
    type Err = error_stack::Report<crate::errors::ValidationError>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cow_string = std::borrow::Cow::Owned(s.to_string());
        Self::try_from(cow_string)
    }
}

// This is implemented so that we can use merchant acquirer id directly as attribute in metrics
#[cfg(feature = "metrics")]
impl From<MerchantAcquirerId> for router_env::opentelemetry::Value {
    fn from(val: MerchantAcquirerId) -> Self {
        Self::from(val.0 .0 .0)
    }
}
