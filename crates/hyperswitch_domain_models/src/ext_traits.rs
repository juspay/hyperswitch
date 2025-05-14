use common_utils::{
    errors::{self, CustomResult},
    ext_traits::ValueExt,
    fp_utils::when,
};
use error_stack::ResultExt;

use crate::errors::api_error_response;

pub type DomainResult<T> = CustomResult<T, api_error_response::ApiErrorResponse>;
pub trait OptionExt<T> {
    fn check_value_present(&self, field_name: &'static str) -> DomainResult<()>;

    fn get_required_value(self, field_name: &'static str) -> DomainResult<T>;

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

impl<T> OptionExt<T> for Option<T> {
    fn check_value_present(&self, field_name: &'static str) -> DomainResult<()> {
        when(self.is_none(), || {
            Err(error_stack::Report::new(
                api_error_response::ApiErrorResponse::MissingRequiredField { field_name },
            )
            .attach_printable(format!("Missing required field {field_name}")))
        })
    }

    // This will allow the error message that was generated in this function to point to the call site
    #[track_caller]
    fn get_required_value(self, field_name: &'static str) -> DomainResult<T> {
        match self {
            Some(v) => Ok(v),
            None => Err(error_stack::Report::new(
                api_error_response::ApiErrorResponse::MissingRequiredField { field_name },
            )
            .attach_printable(format!("Missing required field {field_name}"))),
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
            .change_context(errors::ParsingError::UnknownError)
            .attach_printable_lazy(|| format!("Invalid {{ {enum_name} }} "))
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
