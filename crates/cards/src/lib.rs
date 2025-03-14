pub mod validate;
use std::ops::Deref;

use common_utils::{date_time, errors};
use error_stack::report;
use masking::{PeekInterface, StrongSecret};
use serde::{de, Deserialize, Serialize};
use time::{util::days_in_year_month, Date, Duration, PrimitiveDateTime, Time};

pub use crate::validate::{CardNumber, CardNumberStrategy, CardNumberValidationErr, NetworkToken};

#[derive(Serialize)]
pub struct CardSecurityCode(StrongSecret<u16>);

impl TryFrom<u16> for CardSecurityCode {
    type Error = error_stack::Report<errors::ValidationError>;
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
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let csc = u16::deserialize(deserializer)?;
        csc.try_into().map_err(de::Error::custom)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct CardExpirationMonth(StrongSecret<u8>);

impl CardExpirationMonth {
    pub fn two_digits(&self) -> String {
        format!("{:02}", self.peek())
    }
}

impl TryFrom<u8> for CardExpirationMonth {
    type Error = error_stack::Report<errors::ValidationError>;
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
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let month = u8::deserialize(deserializer)?;
        month.try_into().map_err(de::Error::custom)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct CardExpirationYear(StrongSecret<u16>);

impl CardExpirationYear {
    pub fn four_digits(&self) -> String {
        self.peek().to_string()
    }

    pub fn two_digits(&self) -> String {
        let year = self.peek() % 100;
        year.to_string()
    }
}

impl TryFrom<u16> for CardExpirationYear {
    type Error = error_stack::Report<errors::ValidationError>;
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

    pub fn get_month(&self) -> &CardExpirationMonth {
        &self.month
    }

    pub fn get_year(&self) -> &CardExpirationYear {
        &self.year
    }
}

impl TryFrom<(u8, u16)> for CardExpiration {
    type Error = error_stack::Report<errors::ValidationError>;
    fn try_from(items: (u8, u16)) -> errors::CustomResult<Self, errors::ValidationError> {
        let month = CardExpirationMonth::try_from(items.0)?;
        let year = CardExpirationYear::try_from(items.1)?;
        Ok(Self { month, year })
    }
}

impl Deref for CardSecurityCode {
    type Target = StrongSecret<u16>;
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
