use std::str::FromStr;

crate::id_type!(
    ProfileId,
    "A type for profile_id that can be used for business profile ids"
);
crate::impl_id_type_methods!(ProfileId, "profile_id");

// This is to display the `ProfileId` as ProfileId(abcd)
crate::impl_debug_id_type!(ProfileId);
crate::impl_try_from_cow_str_id_type!(ProfileId, "profile_id");

crate::impl_generate_id_id_type!(ProfileId, "pro");
crate::impl_serializable_secret_id_type!(ProfileId);
crate::impl_queryable_id_type!(ProfileId);
crate::impl_to_sql_from_sql_id_type!(ProfileId);

impl crate::events::ApiEventMetric for ProfileId {
    fn get_api_event_type(&self) -> Option<crate::events::ApiEventsType> {
        Some(crate::events::ApiEventsType::BusinessProfile {
            profile_id: self.clone(),
        })
    }
}

impl FromStr for ProfileId {
    type Err = error_stack::Report<crate::errors::ValidationError>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cow_string = std::borrow::Cow::Owned(s.to_string());
        Self::try_from(cow_string)
    }
}

// This is implemented so that we can use profile id directly as attribute in metrics
#[cfg(feature = "metrics")]
impl From<ProfileId> for router_env::opentelemetry::Value {
    fn from(val: ProfileId) -> Self {
        Self::from(val.0 .0 .0)
    }
}

// Manually define AcquirerId to control Serde behavior precisely
/// A type for acquirer_id that can be used for acquirer ids
#[derive(
    Clone,
    Hash,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    diesel::expression::AsExpression,
    utoipa::ToSchema,
)]
#[diesel(sql_type = diesel::sql_types::Text)]
#[schema(value_type = String)]
#[serde(try_from = "String")]
pub struct AcquirerId(
    crate::id_type::LengthId<
        { crate::consts::MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH },
        { crate::consts::MIN_REQUIRED_MERCHANT_REFERENCE_ID_LENGTH },
    >,
);

crate::impl_id_type_methods!(AcquirerId, "acquirer_id");
crate::impl_debug_id_type!(AcquirerId);

const ACQUIRER_ID_PREFIX: &str = "acq_";
const MIN_LENGTH_AFTER_ACQUIRER_ID_PREFIX: usize = 1;

impl TryFrom<std::borrow::Cow<'static, str>> for AcquirerId {
    type Error = error_stack::Report<crate::errors::ValidationError>;

    fn try_from(value: std::borrow::Cow<'static, str>) -> Result<Self, Self::Error> {
        use error_stack::ResultExt;

        // Existing prefix and length validation logic
        if !value.starts_with(ACQUIRER_ID_PREFIX) {
            return Err(error_stack::report!(
                crate::errors::ValidationError::IncorrectValueProvided {
                    field_name: "acquirer_id expected to start with prefix acq_",
                }
            )
            .attach_printable(format!(
                "Acquirer ID must start with '{}'",
                ACQUIRER_ID_PREFIX
            )));
        }

        if value.len() < ACQUIRER_ID_PREFIX.len() + MIN_LENGTH_AFTER_ACQUIRER_ID_PREFIX {
            return Err(error_stack::report!(
                crate::errors::ValidationError::IncorrectValueProvided {
                    field_name: "acquirer_id",
                }
            )
            .attach_printable(format!(
                "Acquirer ID must have at least {} character(s) after the prefix '{}'",
                MIN_LENGTH_AFTER_ACQUIRER_ID_PREFIX, ACQUIRER_ID_PREFIX
            )));
        }

        let length_id = crate::id_type::LengthId::from(value).change_context(
            crate::errors::ValidationError::IncorrectValueProvided {
                field_name: "acquirer_id",
            },
        )?;

        Ok(Self(length_id))
    }
}

// Implement TryFrom<String> for AcquirerId to be used by #[serde(try_from = "String")]
impl TryFrom<String> for AcquirerId {
    type Error = error_stack::Report<crate::errors::ValidationError>;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(std::borrow::Cow::Owned(value))
    }
}

crate::impl_generate_id_id_type!(AcquirerId, "acq");
crate::impl_serializable_secret_id_type!(AcquirerId);
crate::impl_queryable_id_type!(AcquirerId);
crate::impl_to_sql_from_sql_id_type!(AcquirerId);

impl FromStr for AcquirerId {
    type Err = error_stack::Report<crate::errors::ValidationError>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cow_string = std::borrow::Cow::Owned(s.to_string());
        Self::try_from(cow_string)
    }
}

// This is implemented so that we can use profile id directly as attribute in metrics
#[cfg(feature = "metrics")]
impl From<AcquirerId> for router_env::opentelemetry::Value {
    fn from(val: AcquirerId) -> Self {
        Self::from(val.0 .0 .0)
    }
}
