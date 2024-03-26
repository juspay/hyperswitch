use common_utils::ext_traits::ValueExt;
use error_stack::{IntoReport, Report, ResultExt};

use crate::{
    core::errors::{self, ApiErrorResponse, CustomResult, RouterResult},
    utils::when,
};

pub trait OptionExt<T> {
    fn check_value_present(&self, field_name: &'static str) -> RouterResult<()>;

    fn get_required_value(self, field_name: &'static str) -> RouterResult<T>;

    fn parse_enum<E>(self, enum_name: &'static str) -> CustomResult<E, errors::ParsingError>
    where
        T: AsRef<str>,
        E: std::str::FromStr,
        // Requirement for converting the `Err` variant of `FromStr` to `Report<Err>`
        <E as std::str::FromStr>::Err: std::error::Error + Send + Sync + 'static;

    fn parse_value<U>(self, type_name: &'static str) -> CustomResult<U, errors::ParsingError>
    where
        T: ValueExt,
        U: serde::de::DeserializeOwned;

    fn update_value(&mut self, value: Option<T>);
}

impl<T> OptionExt<T> for Option<T>
where
    T: std::fmt::Debug,
{
    fn check_value_present(&self, field_name: &'static str) -> RouterResult<()> {
        when(self.is_none(), || {
            Err(
                Report::new(ApiErrorResponse::MissingRequiredField { field_name })
                    .attach_printable(format!("Missing required field {field_name} in {self:?}")),
            )
        })
    }

    // This will allow the error message that was generated in this function to point to the call site
    #[track_caller]
    fn get_required_value(self, field_name: &'static str) -> RouterResult<T> {
        match self {
            Some(v) => Ok(v),
            None => Err(
                Report::new(ApiErrorResponse::MissingRequiredField { field_name })
                    .attach_printable(format!("Missing required field {field_name} in {self:?}")),
            ),
        }
    }

    fn parse_enum<E>(self, enum_name: &'static str) -> CustomResult<E, errors::ParsingError>
    where
        T: AsRef<str>,
        E: std::str::FromStr,
        <E as std::str::FromStr>::Err: std::error::Error + Send + Sync + 'static,
    {
        let value = self
            .get_required_value(enum_name)
            .change_context(errors::ParsingError::UnknownError)?;

        E::from_str(value.as_ref())
            .into_report()
            .change_context(errors::ParsingError::UnknownError)
            .attach_printable_lazy(|| format!("Invalid {{ {enum_name}: {value:?} }} "))
    }

    fn parse_value<U>(self, type_name: &'static str) -> CustomResult<U, errors::ParsingError>
    where
        T: ValueExt,
        U: serde::de::DeserializeOwned,
    {
        let value = self
            .get_required_value(type_name)
            .change_context(errors::ParsingError::UnknownError)?;
        value.parse_value(type_name)
    }

    fn update_value(&mut self, value: Self) {
        if let Some(a) = value {
            *self = Some(a)
        }
    }
}

pub trait ValidateCall<T, F> {
    fn validate_opt(self, func: F) -> CustomResult<(), errors::ValidationError>;
}

impl<T, F> ValidateCall<T, F> for Option<&T>
where
    F: Fn(&T) -> CustomResult<(), errors::ValidationError>,
{
    fn validate_opt(self, func: F) -> CustomResult<(), errors::ValidationError> {
        match self {
            Some(val) => func(val),
            None => Ok(()),
        }
    }
}
