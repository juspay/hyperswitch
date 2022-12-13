//! Personal Identifiable Information protection.

use std::{convert::AsRef, fmt};

use masking::{Strategy, WithType};

use crate::validation::validate_email;

/// Card number
#[derive(Debug)]
pub struct CardNumber;

impl<T> Strategy<T> for CardNumber
where
    T: AsRef<str>,
{
    fn fmt(val: &T, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val_str: &str = val.as_ref();

        if val_str.len() < 15 && val_str.len() > 19 {
            return WithType::fmt(val, f);
        }

        write!(f, "{}{}", &val_str[..6], "*".repeat(val_str.len() - 6))
    }
}

/*
/// Phone number
#[derive(Debug)]
pub struct PhoneNumber;

impl<T> Strategy<T> for PhoneNumber
where
    T: AsRef<str>,
{
    fn fmt(val: &T, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val_str: &str = val.as_ref();

        if val_str.len() < 10 || val_str.len() > 12 {
            return WithType::fmt(val, f);
        }

        write!(
            f,
            "{}{}{}",
            &val_str[..2],
            "*".repeat(val_str.len() - 5),
            &val_str[(val_str.len() - 3)..]
        )
    }
}
*/

/// Email address
#[derive(Debug)]
pub struct Email;

// FIXME(kos): Here should be not generic T, but newtype Email.
impl<T> Strategy<T> for Email
where
    T: AsRef<str>,
{
    fn fmt(val: &T, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val_str: &str = val.as_ref();
        // FIXME(kos): We should assume constructed value object is already validated.
        // Email validation is quite a heavy operation. Doing it on
        // each formatting is quite a subtle performance penalty,
        // while being... unnecessary?
        // Another problem, that validation here is
        // violation of the "Separation of concerns" design
        // principle. Formatting is not a validation in any way.
        // https://en.wikipedia.org/wiki/Separation_of_concerns
        // Consider to provide a newtype for email strings.
        // https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html
        // This way you do the validation only once, when creating a
        // value of the type, and then you may fearlessly reuse it
        // as the type system protects you.
        // https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate
        // Thus, as the result, you will be able to remove any
        // validation code from the formatting, as the compiler will
        // guarantee that you would have valid values here.
        // The same is true for other formatting strategies in this
        // module too, as they're effectively validators too.
        let is_valid = validate_email(val_str);

        if is_valid.is_err() {
            return WithType::fmt(val, f);
        }

        if let Some((a, b)) = val_str.split_once('@') {
            write!(f, "{}@{}", "*".repeat(a.len()), b)
        } else {
            WithType::fmt(val, f)
        }
    }
}

/// IP address
#[derive(Debug)]
pub struct IpAddress;

// FIXME(kos): Here should be not generic T, but std::net::IpAddr.
impl<T> Strategy<T> for IpAddress
where
    T: AsRef<str>,
{
    fn fmt(val: &T, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO(kos): Consider to not use `String`s for IP addresses.
        // There is a `std::net::IpAddr` for that.
        let val_str: &str = val.as_ref();
        let segments: Vec<&str> = val_str.split('.').collect();

        if segments.len() != 4 {
            return WithType::fmt(val, f);
        }

        for seg in segments.iter() {
            if seg.is_empty() || seg.len() > 3 {
                return WithType::fmt(val, f);
            }
        }

        write!(f, "{}.**.**.**", segments[0])
    }
}

#[cfg(test)]
mod pii_masking_strategy_tests {
    use masking::Secret;

    use super::{CardNumber, Email, IpAddress};

    #[test]
    fn test_valid_card_number_masking() {
        let secret: Secret<String, CardNumber> = Secret::new("1234567890987654".to_string());
        assert_eq!("123456**********", format!("{:?}", secret));
    }

    #[test]
    fn test_invalid_card_number_masking() {
        let secret: Secret<String, CardNumber> = Secret::new("1234567890".to_string());
        assert_eq!("123456****", format!("{:?}", secret));
    }

    /*
    #[test]
    fn test_valid_phone_number_masking() {
        let secret: Secret<String, PhoneNumber> = Secret::new("9922992299".to_string());
        assert_eq!("99*****299", format!("{}", secret));
    }

    #[test]
    fn test_invalid_phone_number_masking() {
        let secret: Secret<String, PhoneNumber> = Secret::new("99229922".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{}", secret));

        let secret: Secret<String, PhoneNumber> = Secret::new("9922992299229922".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{}", secret));
    }
    */

    #[test]
    fn test_valid_email_masking() {
        let secret: Secret<String, Email> = Secret::new("myemail@gmail.com".to_string());
        assert_eq!("*******@gmail.com", format!("{:?}", secret));
    }

    #[test]
    fn test_invalid_email_masking() {
        let secret: Secret<String, Email> = Secret::new("myemailgmail.com".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{:?}", secret));

        let secret: Secret<String, Email> = Secret::new("myemail@gmail@com".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{:?}", secret));
    }

    #[test]
    fn test_valid_ip_addr_masking() {
        let secret: Secret<String, IpAddress> = Secret::new("123.23.1.78".to_string());
        assert_eq!("123.**.**.**", format!("{:?}", secret));
    }

    #[test]
    fn test_invalid_ip_addr_masking() {
        let secret: Secret<String, IpAddress> = Secret::new("123.4.56".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{:?}", secret));

        let secret: Secret<String, IpAddress> = Secret::new("123.4567.12.4".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{:?}", secret));

        let secret: Secret<String, IpAddress> = Secret::new("123..4.56".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{:?}", secret));
    }
}
