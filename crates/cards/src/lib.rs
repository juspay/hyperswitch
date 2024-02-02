pub mod validate;
use std::ops::Deref;

use common_utils::{date_time, errors};
use error_stack::report;
use masking::{PeekInterface, StrongSecret};
use serde::{de, Deserialize, Serialize};
use time::{util::days_in_year_month, Date, Duration, PrimitiveDateTime, Time};

pub use crate::validate::{CCValError, CardNumber, CardNumberStrategy};

#[derive(Serialize)]
pub struct CardSecurityCode(StrongSecret<u16>);

impl TryFrom<u16> for CardSecurityCode {
    type Error = error_stack::Report<errors::ValidationError>;
        /// Tries to create a new instance of the current type from a given 16-bit unsigned integer, representing a card security code (CSC). 
    /// If the given CSC is within the range of 0 to 9999 (inclusive), it creates a new instance with the provided CSC. 
    /// If the given CSC is not within the valid range, it returns a validation error with a message indicating that the CSC is invalid.
    fn try_from(csc: u16) -> Result<Self, Self::Error> {
        if (0..=9999).contains(&csc) {
            Ok(Self(StrongSecret::new(csc)))
        } else {
            Err(report!(errors::ValidationError::InvalidValue {
                message: "invalid card security code".to_string()
            }))
        }
    }
}

impl<'de> Deserialize<'de> for CardSecurityCode {
        /// Deserialize the given input using a serde deserializer and convert it into a Result.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let csc = u16::deserialize(deserializer)?;
        csc.try_into().map_err(de::Error::custom)
    }
}

#[derive(Serialize)]
pub struct CardExpirationMonth(StrongSecret<u8>);

impl CardExpirationMonth {
        /// Formats the result of the `peek` method as a two-digit string, padding with leading zero if necessary.
    pub fn two_digits(&self) -> String {
        format!("{:02}", self.peek())
    }
}

impl TryFrom<u8> for CardExpirationMonth {
    type Error = error_stack::Report<errors::ValidationError>;
        /// Attempts to create a new instance of the struct from a u8 value representing a month.
    /// If the input value is within the range of 1 to 12 (inclusive), it returns a Result containing the newly created instance.
    /// If the input value is outside of the valid range, it returns a Result with a ValidationError error indicating an invalid value.
    fn try_from(month: u8) -> Result<Self, Self::Error> {
        if (1..=12).contains(&month) {
            Ok(Self(StrongSecret::new(month)))
        } else {
            Err(report!(errors::ValidationError::InvalidValue {
                message: "invalid card expiration month".to_string()
            }))
        }
    }
}

impl<'de> Deserialize<'de> for CardExpirationMonth {
        /// Deserializes the given input into a Result containing the deserialized value or an error.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let month = u8::deserialize(deserializer)?;
        month.try_into().map_err(de::Error::custom)
    }
}

#[derive(Serialize)]
pub struct CardExpirationYear(StrongSecret<u16>);

impl CardExpirationYear {
        /// Retrieves the four digits from the peeked element in the data structure and returns them as a string.
    pub fn four_digits(&self) -> String {
        self.peek().to_string()
    }

        /// Returns the last two digits of the year obtained by peeking into the struct.
    pub fn two_digits(&self) -> String {
        let year = self.peek() % 100;
        year.to_string()
    }
}

impl TryFrom<u16> for CardExpirationYear {
    type Error = error_stack::Report<errors::ValidationError>;
        /// Tries to create a new instance from the given year, returning a Result.
    /// If the year is valid and not expired, it creates a new instance with the provided year.
    /// If the year is invalid or expired, it returns an error with a message indicating the issue.
    fn try_from(year: u16) -> Result<Self, Self::Error> {
        let curr_year = u16::try_from(date_time::now().year()).map_err(|_| {
            report!(errors::ValidationError::InvalidValue {
                message: "invalid year".to_string()
            })
        })?;

        if year >= curr_year {
            Ok(Self(StrongSecret::<u16>::new(year)))
        } else {
            Err(report!(errors::ValidationError::InvalidValue {
                message: "invalid card expiration year".to_string()
            }))
        }
    }
}

impl<'de> Deserialize<'de> for CardExpirationYear {
        /// Deserialize the given value using the provided deserializer and return a Result containing
    /// the deserialized value or an error.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let year = u16::deserialize(deserializer)?;
        year.try_into().map_err(de::Error::custom)
    }
}

#[derive(Serialize, Deserialize)]
pub struct CardExpiration {
    pub month: CardExpirationMonth,
    pub year: CardExpirationYear,
}

impl CardExpiration {
        /// Checks if the card is expired based on the expiration date and time specified, compared to the current UTC datetime.
    pub fn is_expired(&self) -> Result<bool, error_stack::Report<errors::ValidationError>> {
        let current_datetime_utc = date_time::now();

        let expiration_month = (*self.month.peek()).try_into().map_err(|_| {
            report!(errors::ValidationError::InvalidValue {
                message: "invalid month".to_string()
            })
        })?;

        let expiration_year = *self.year.peek();

        let expiration_day = days_in_year_month(i32::from(expiration_year), expiration_month);

        let expiration_date =
            Date::from_calendar_date(i32::from(expiration_year), expiration_month, expiration_day)
                .map_err(|_| {
                    report!(errors::ValidationError::InvalidValue {
                        message: "error while constructing calendar date".to_string()
                    })
                })?;

        let expiration_time = Time::MIDNIGHT;

        // actual expiry date specified on card w.r.t. local timezone
        // max diff b/w utc and other timezones is 14 hours
        let mut expiration_datetime_utc = PrimitiveDateTime::new(expiration_date, expiration_time);

        // compensating time difference b/w local and utc timezone by adding a day
        expiration_datetime_utc = expiration_datetime_utc.saturating_add(Duration::days(1));

        Ok(current_datetime_utc > expiration_datetime_utc)
    }

        /// Returns a reference to the CardExpirationMonth associated with the current instance.
    pub fn get_month(&self) -> &CardExpirationMonth {
        &self.month
    }

        /// This method returns a reference to the CardExpirationYear associated with the current instance of the struct.
    pub fn get_year(&self) -> &CardExpirationYear {
        &self.year
    }
}

impl TryFrom<(u8, u16)> for CardExpiration {
    type Error = error_stack::Report<errors::ValidationError>;
        /// Attempts to create a new instance of Self from a tuple of u8 and u16, representing the month and year of a card expiration date respectively.
    /// Returns a Result containing the newly created instance or a validation error if the conversion from u8 and u16 to CardExpirationMonth and CardExpirationYear fails.
    fn try_from(items: (u8, u16)) -> errors::CustomResult<Self, errors::ValidationError> {
            let month = CardExpirationMonth::try_from(items.0)?;
            let year = CardExpirationYear::try_from(items.1)?;
            Ok(Self { month, year })
        }
}

impl Deref for CardSecurityCode {
    type Target = StrongSecret<u16>;
        /// This method returns a reference to the value that is being dereferenced.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for CardExpirationMonth {
    type Target = StrongSecret<u8>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for CardExpirationYear {
    type Target = StrongSecret<u16>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
