use std::str::FromStr;

crate::id_type!(
    ProfileAcquirerId,
    "A type for profile_acquirer_id that can be used for profile acquirer ids"
);
crate::impl_id_type_methods!(ProfileAcquirerId, "profile_acquirer_id");

// This is to display the `ProfileAcquirerId` as ProfileAcquirerId(abcd)
crate::impl_debug_id_type!(ProfileAcquirerId);
crate::impl_try_from_cow_str_id_type!(ProfileAcquirerId, "profile_acquirer_id");

crate::impl_generate_id_id_type!(ProfileAcquirerId, "pro_acq");
crate::impl_serializable_secret_id_type!(ProfileAcquirerId);
crate::impl_queryable_id_type!(ProfileAcquirerId);
crate::impl_to_sql_from_sql_id_type!(ProfileAcquirerId);

impl Ord for ProfileAcquirerId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0 .0 .0.cmp(&other.0 .0 .0)
    }
}

impl PartialOrd for ProfileAcquirerId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl crate::events::ApiEventMetric for ProfileAcquirerId {
    fn get_api_event_type(&self) -> Option<crate::events::ApiEventsType> {
        Some(crate::events::ApiEventsType::ProfileAcquirer {
            profile_acquirer_id: self.clone(),
        })
    }
}

impl FromStr for ProfileAcquirerId {
    type Err = error_stack::Report<crate::errors::ValidationError>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cow_string = std::borrow::Cow::Owned(s.to_string());
        Self::try_from(cow_string)
    }
}

// This is implemented so that we can use profile acquirer id directly as attribute in metrics
#[cfg(feature = "metrics")]
impl From<ProfileAcquirerId> for router_env::opentelemetry::Value {
    fn from(val: ProfileAcquirerId) -> Self {
        Self::from(val.0 .0 .0)
    }
}
