pub(super) mod customer;
pub(super) mod payment;
pub(super) mod payment_methods;
pub(super) mod refunds;

use diesel::{backend::Backend, deserialize::FromSql, serialize::ToSql, sql_types};
use error_stack::ResultExt;
use thiserror::Error;

use crate::{
    consts::{CELL_IDENTIFIER_LENGTH, MAX_GLOBAL_ID_LENGTH, MIN_GLOBAL_ID_LENGTH},
    errors, generate_time_ordered_id,
    id_type::{AlphaNumericId, AlphaNumericIdError, LengthId, LengthIdError},
};

#[derive(Debug, Clone, Hash, PartialEq, Eq, serde::Serialize)]
/// A global id that can be used to identify any entity
/// This id will have information about the entity and cell in a distributed system architecture
pub(crate) struct GlobalId(LengthId<MAX_GLOBAL_ID_LENGTH, MIN_GLOBAL_ID_LENGTH>);

#[derive(Clone, Copy)]
/// Entities that can be identified by a global id
pub(crate) enum GlobalEntity {
    Customer,
    Payment,
    Attempt,
    PaymentMethod,
    Refund,
    PaymentMethodSession,
}

impl GlobalEntity {
    fn prefix(self) -> &'static str {
        match self {
            Self::Customer => "cus",
            Self::Payment => "pay",
            Self::PaymentMethod => "pm",
            Self::Attempt => "att",
            Self::Refund => "ref",
            Self::PaymentMethodSession => "pms",
        }
    }
}

/// Cell identifier for an instance / deployment of application
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct CellId(LengthId<CELL_IDENTIFIER_LENGTH, CELL_IDENTIFIER_LENGTH>);

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CellIdError {
    #[error("cell id error: {0}")]
    InvalidCellLength(LengthIdError),

    #[error("{0}")]
    InvalidCellIdFormat(AlphaNumericIdError),
}

impl From<LengthIdError> for CellIdError {
    fn from(error: LengthIdError) -> Self {
        Self::InvalidCellLength(error)
    }
}

impl From<AlphaNumericIdError> for CellIdError {
    fn from(error: AlphaNumericIdError) -> Self {
        Self::InvalidCellIdFormat(error)
    }
}

impl CellId {
    /// Create a new cell id from a string
    fn from_str(cell_id_string: impl AsRef<str>) -> Result<Self, CellIdError> {
        let trimmed_input_string = cell_id_string.as_ref().trim().to_string();
        let alphanumeric_id = AlphaNumericId::from(trimmed_input_string.into())?;
        let length_id = LengthId::from_alphanumeric_id(alphanumeric_id)?;
        Ok(Self(length_id))
    }

    /// Create a new cell id from a string
    pub fn from_string(
        input_string: impl AsRef<str>,
    ) -> error_stack::Result<Self, errors::ValidationError> {
        Self::from_str(input_string).change_context(
            errors::ValidationError::IncorrectValueProvided {
                field_name: "cell_id",
            },
        )
    }

    /// Get the string representation of the cell id
    fn get_string_repr(&self) -> &str {
        &self.0 .0 .0
    }
}

impl<'de> serde::Deserialize<'de> for CellId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let deserialized_string = String::deserialize(deserializer)?;
        Self::from_str(deserialized_string.as_str()).map_err(serde::de::Error::custom)
    }
}

/// Error generated from violation of constraints for MerchantReferenceId
#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum GlobalIdError {
    /// The format for the global id is invalid
    #[error("The id format is invalid, expected format is {{cell_id:5}}_{{entity_prefix:3}}_{{uuid:32}}_{{random:24}}")]
    InvalidIdFormat,

    /// LengthIdError and AlphanumericIdError
    #[error("{0}")]
    LengthIdError(#[from] LengthIdError),

    /// CellIdError because of invalid cell id format
    #[error("{0}")]
    CellIdError(#[from] CellIdError),
}

impl GlobalId {
    /// Create a new global id from entity and cell information
    /// The entity prefix is used to identify the entity, `cus` for customers, `pay`` for payments etc.
    pub fn generate(cell_id: &CellId, entity: GlobalEntity) -> Self {
        let prefix = format!("{}_{}", cell_id.get_string_repr(), entity.prefix());
        let id = generate_time_ordered_id(&prefix);
        let alphanumeric_id = AlphaNumericId::new_unchecked(id);
        Self(LengthId::new_unchecked(alphanumeric_id))
    }

    pub(crate) fn from_string(
        input_string: std::borrow::Cow<'static, str>,
    ) -> Result<Self, GlobalIdError> {
        let length_id = LengthId::from(input_string)?;
        let input_string = &length_id.0 .0;
        let (cell_id, remaining) = input_string
            .split_once("_")
            .ok_or(GlobalIdError::InvalidIdFormat)?;

        CellId::from_str(cell_id)?;

        Ok(Self(length_id))
    }

    pub(crate) fn get_string_repr(&self) -> &str {
        &self.0 .0 .0
    }
}

impl<DB> ToSql<sql_types::Text, DB> for GlobalId
where
    DB: Backend,
    String: ToSql<sql_types::Text, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        self.0 .0 .0.to_sql(out)
    }
}

impl<DB> FromSql<sql_types::Text, DB> for GlobalId
where
    DB: Backend,
    String: FromSql<sql_types::Text, DB>,
{
    fn from_sql(value: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let string_val = String::from_sql(value)?;
        let alphanumeric_id = AlphaNumericId::from(string_val.into())?;
        let length_id = LengthId::from_alphanumeric_id(alphanumeric_id)?;
        Ok(Self(length_id))
    }
}

/// Deserialize the global id from string
/// The format should match {cell_id:5}_{entity_prefix:3}_{time_ordered_id:32}_{.*:24}
impl<'de> serde::Deserialize<'de> for GlobalId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let deserialized_string = String::deserialize(deserializer)?;
        Self::from_string(deserialized_string.into()).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod global_id_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn test_cell_id_from_str() {
        let cell_id_string = "12345";
        let cell_id = CellId::from_str(cell_id_string).unwrap();
        assert_eq!(cell_id.get_string_repr(), cell_id_string);
    }

    #[test]
    fn test_global_id_generate() {
        let cell_id_string = "12345";
        let entity = GlobalEntity::Customer;
        let cell_id = CellId::from_str(cell_id_string).unwrap();
        let global_id = GlobalId::generate(&cell_id, entity);

        // Generate a regex for globalid
        // Eg - 12abc_cus_abcdefghijklmnopqrstuvwxyz1234567890
        let regex = regex::Regex::new(r"[a-z0-9]{5}_cus_[a-z0-9]{32}").unwrap();

        assert!(regex.is_match(&global_id.0 .0 .0));
    }

    #[test]
    fn test_global_id_from_string() {
        let input_string = "12345_cus_abcdefghijklmnopqrstuvwxyz1234567890";
        let global_id = GlobalId::from_string(input_string.into()).unwrap();
        assert_eq!(global_id.0 .0 .0, input_string);
    }

    #[test]
    fn test_global_id_deser() {
        let input_string_for_serde_json_conversion =
            r#""12345_cus_abcdefghijklmnopqrstuvwxyz1234567890""#;

        let input_string = "12345_cus_abcdefghijklmnopqrstuvwxyz1234567890";
        let global_id =
            serde_json::from_str::<GlobalId>(input_string_for_serde_json_conversion).unwrap();
        assert_eq!(global_id.0 .0 .0, input_string);
    }

    #[test]
    fn test_global_id_deser_error() {
        let input_string_for_serde_json_conversion =
            r#""123_45_cus_abcdefghijklmnopqrstuvwxyz1234567890""#;

        let global_id = serde_json::from_str::<GlobalId>(input_string_for_serde_json_conversion);
        assert!(global_id.is_err());

        let expected_error_message = format!(
            "cell id error: the minimum required length for this field is {CELL_IDENTIFIER_LENGTH}"
        );

        let error_message = global_id.unwrap_err().to_string();
        assert_eq!(error_message, expected_error_message);
    }
}
