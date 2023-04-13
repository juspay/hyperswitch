use common_utils::{date_time, errors};
use error_stack::report;
use masking::{PeekInterface, StrongSecret};
use time::{util::days_in_year_month, Date, Duration, PrimitiveDateTime, Time};

pub struct CardSecurityCode(StrongSecret<u16>);

impl CardSecurityCode {
    pub fn new(secret: StrongSecret<u16>) -> errors::CustomResult<Self, errors::ValidationError> {
        let csc = secret.peek();

        if *csc > 99 && *csc < 10000 {
            Ok(Self(secret))
        } else {
            Err(report!(errors::ValidationError::InvalidValue {
                message: "invalid card security code".to_string()
            }))
        }
    }
}

pub struct CardExpirationMonth(StrongSecret<u8>);

impl CardExpirationMonth {
    pub fn new(secret: StrongSecret<u8>) -> errors::CustomResult<Self, errors::ValidationError> {
        let month = secret.peek();

        if *month >= 1 && *month <= 12 {
            Ok(Self(secret))
        } else {
            Err(report!(errors::ValidationError::InvalidValue {
                message: "invalid card expiration month".to_string()
            }))
        }
    }

    pub fn two_digits(&self) -> String {
        let month = self.0.peek();
        let month_str: String = month.to_string();
        if month_str.len() == 1 {
            format!("0{}", month_str)
        } else {
            month_str
        }
    }
}

pub struct CardExpirationYear(StrongSecret<u16>);

impl CardExpirationYear {
    pub fn new(secret: StrongSecret<u16>) -> errors::CustomResult<Self, errors::ValidationError> {
        let year = secret.peek();

        if *year >= 1997 {
            Ok(Self(secret))
        } else {
            Err(report!(errors::ValidationError::InvalidValue {
                message: "invalid card expiration year".to_string()
            }))
        }
    }

    pub fn four_digits(&self) -> String {
        self.0.peek().to_string()
    }

    pub fn two_digits(&self) -> String {
        let year = &self.0.peek().to_string()[2..4];
        year.to_string()
    }
}

pub struct CardExpiration {
    pub month: CardExpirationMonth,
    pub year: CardExpirationYear,
}

impl CardExpiration {
    pub fn new(
        secret_month: StrongSecret<u8>,
        secret_year: StrongSecret<u16>,
    ) -> errors::CustomResult<Self, errors::ValidationError> {
        let card_month = CardExpirationMonth::new(secret_month);

        match card_month {
            Ok(cm) => {
                let card_year = CardExpirationYear::new(secret_year);

                match card_year {
                    Ok(cy) => Ok(Self {
                        month: cm,
                        year: cy,
                    }),
                    Err(e) => Err(e),
                }
            }
            Err(e) => Err(e),
        }
    }

    pub fn is_expired(&self) -> bool {
        let current_datetime_utc = date_time::now();

        // card expiry day is last day of the expiration month
        let expiration_day = days_in_year_month(
            i32::from(*self.year.0.peek()),
            (*self.month.0.peek()).try_into().unwrap(),
        );

        let expiration_date = Date::from_calendar_date(
            i32::from(*self.year.0.peek()),
            (*self.month.0.peek()).try_into().unwrap(),
            expiration_day,
        )
        .unwrap();
        let expiration_time = Time::MIDNIGHT;

        // actual expiry date specified on card w.r.t. local timezone
        // max diff b/w utc and other timezones is 14 hours
        let mut expiration_datetime_utc = PrimitiveDateTime::new(expiration_date, expiration_time);

        // compensating time difference b/w local and utc timezone by adding a day
        expiration_datetime_utc = expiration_datetime_utc.saturating_add(Duration::days(1));

        if current_datetime_utc > expiration_datetime_utc {
            true
        } else {
            false
        }
    }

    pub fn get_month(&self) -> &CardExpirationMonth {
        &self.month
    }

    pub fn get_year(&self) -> &CardExpirationYear {
        &self.year
    }
}

impl PeekInterface<StrongSecret<u16>> for CardSecurityCode {
    fn peek(&self) -> &StrongSecret<u16> {
        &self.0
    }
}

impl PeekInterface<StrongSecret<u8>> for CardExpirationMonth {
    fn peek(&self) -> &StrongSecret<u8> {
        &self.0
    }
}

impl PeekInterface<StrongSecret<u16>> for CardExpirationYear {
    fn peek(&self) -> &StrongSecret<u16> {
        &self.0
    }
}