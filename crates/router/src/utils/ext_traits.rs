pub use common_utils::ext_traits::{ByteSliceExt, BytesExt, Encode, StringExt, ValueExt};
use error_stack::{report, IntoReport, Report, ResultExt};
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    core::errors::{self, ApiErrorResponse, CustomResult, RouterResult, ValidateError},
    logger,
    types::api::AddressDetails,
    utils::when,
};

pub(crate) trait OptionExt<T> {
    fn check_value_present(&self, field_name: &str) -> RouterResult<()>;

    fn get_required_value(self, field_name: &str) -> RouterResult<T>;

    fn parse_enum<E>(self, enum_name: &str) -> CustomResult<E, errors::ParsingError>
    where
        T: AsRef<str>,
        E: std::str::FromStr,
        // Requirement for converting the `Err` variant of `FromStr` to `Report<Err>`
        <E as std::str::FromStr>::Err: std::error::Error + Send + Sync + 'static;

    fn parse_value<U>(self, type_name: &str) -> CustomResult<U, errors::ParsingError>
    where
        T: ValueExt<U>,
        U: serde::de::DeserializeOwned;

    fn update_value(&mut self, value: Option<T>);
}

impl<T> OptionExt<T> for Option<T>
where
    T: std::fmt::Debug,
{
    fn check_value_present(&self, field_name: &str) -> RouterResult<()> {
        when(
            self.is_none(),
            Err(Report::new(ApiErrorResponse::MissingRequiredField {
                field_name: field_name.to_string(),
            })
            .attach_printable(format!("Missing required field {field_name} in {self:?}"))),
        )
    }

    fn get_required_value(self, field_name: &str) -> RouterResult<T> {
        match self {
            Some(v) => Ok(v),
            None => Err(Report::new(ApiErrorResponse::MissingRequiredField {
                field_name: field_name.to_string(),
            })
            .attach_printable(format!("Missing required field {field_name} in {self:?}"))),
        }
    }

    fn parse_enum<E>(self, enum_name: &str) -> CustomResult<E, errors::ParsingError>
    where
        T: AsRef<str>,
        E: std::str::FromStr,
        <E as std::str::FromStr>::Err: std::error::Error + Send + Sync + 'static,
    {
        let value = self
            .get_required_value(enum_name)
            .change_context(errors::ParsingError)?;

        E::from_str(value.as_ref())
            .into_report()
            .change_context(errors::ParsingError)
            .attach_printable_lazy(|| format!("Invalid {{ {enum_name}: {value:?} }} "))
    }

    fn parse_value<U>(self, type_name: &str) -> CustomResult<U, errors::ParsingError>
    where
        T: ValueExt<U>,
        U: serde::de::DeserializeOwned,
    {
        let value = self
            .get_required_value(type_name)
            .change_context(errors::ParsingError)?;
        value.parse_value(type_name)
    }

    fn update_value(&mut self, value: Option<T>) {
        if let Some(a) = value {
            *self = Some(a)
        }
    }
}

#[allow(dead_code)]
/// Merge two `serde_json::Value` instances. Will need to be updated to handle merging arrays.
pub(crate) fn merge_json_values(a: &mut serde_json::Value, b: &serde_json::Value) {
    // Reference: https://github.com/serde-rs/json/issues/377#issuecomment-341490464
    // See also (for better implementations):
    //   - https://github.com/marirs/serde-json-utils
    //   - https://github.com/jmfiaschi/json_value_merge
    use serde_json::Value;

    match (a, b) {
        (&mut Value::Object(ref mut a), &Value::Object(ref b)) => {
            for (k, v) in b {
                merge_json_values(a.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (a, b) => {
            *a = b.clone();
        }
    }
}

// TODO: change Name
pub trait ValidateVar {
    fn validate(self) -> CustomResult<Self, ValidateError>
    where
        Self: std::marker::Sized;
}

pub trait ValidateCall<T, F> {
    fn validate_opt(self, func: F) -> CustomResult<(), ValidateError>;
}

impl<T, F> ValidateCall<T, F> for Option<&T>
where
    F: Fn(&T) -> CustomResult<(), ValidateError>,
{
    fn validate_opt(self, func: F) -> CustomResult<(), ValidateError> {
        match self {
            Some(val) => func(val),
            None => Ok(()),
        }
    }
}

pub fn validate_email(email: &str) -> CustomResult<(), ValidateError> {
    #[deny(clippy::invalid_regex)]
    static EMAIL_REGEX: Lazy<Option<Regex>> = Lazy::new(|| {
        match Regex::new(
            r"^(?i)[a-z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?(?:\.[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?)*$",
        ) {
            Ok(regex) => Some(regex),
            Err(error) => {
                logger::error!(?error);
                None
            }
        }
    });
    let email_regex = match EMAIL_REGEX.as_ref() {
        Some(regex) => Ok(regex),
        None => Err(report!(ValidateError).attach_printable("Invalid regex expression")),
    }?;

    const EMAIL_MAX_LENGTH: usize = 319;
    if email.is_empty() || email.chars().count() > EMAIL_MAX_LENGTH {
        return Err(report!(ValidateError).attach_printable("Invalid email address length"));
    }

    if !email_regex.is_match(email) {
        return Err(report!(ValidateError).attach_printable("Invalid email format"));
    }

    Ok(())
}

pub fn validate_address(address: &serde_json::Value) -> CustomResult<(), ValidateError> {
    if let Err(err) = serde_json::from_value::<AddressDetails>(address.clone()) {
        return Err(
            report!(ValidateError).attach_printable(format!("Address is invalid {:?}", err))
        );
    }
    Ok(())
}

pub(crate) trait FromExt<T, X> {
    fn from_ext(item: T, extra: X) -> Self;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_email() {
        let result = validate_email("abc@example.com");
        assert!(result.is_ok());

        let result = validate_email("abc+123@example.com");
        assert!(result.is_ok());

        let result = validate_email("");
        assert!(result.is_err());
    }
}
