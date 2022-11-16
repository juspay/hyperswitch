//!
//! Personal Identifiable Information protection.
//!

use std::{convert::AsRef, fmt};

#[doc(inline)]
pub use masking::*;

use crate::utils::validate_email;

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

        f.write_str(&format!(
            "{}{}",
            &val_str[..6],
            "*".repeat(val_str.len() - 6)
        ))
    }
}

//pub struct PhoneNumber;

//impl<T> Strategy<T> for PhoneNumber
//where
//T: AsRef<str>,
//{
//fn fmt(val: &T, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//let val_str: &str = val.as_ref();

//if val_str.len() < 10 || val_str.len() > 12 {
//return WithType::fmt(val, f);
//}

//f.write_str(&format!(
//"{}{}{}",
//&val_str[..2],
//"*".repeat(val_str.len() - 5),
//&val_str[(val_str.len() - 3)..]
//))
//}
//}

pub struct Email;

impl<T> Strategy<T> for Email
where
    T: AsRef<str>,
{
    fn fmt(val: &T, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val_str: &str = val.as_ref();
        let is_valid = validate_email(val_str);

        if is_valid.is_err() {
            return WithType::fmt(val, f);
        }

        let parts: Vec<&str> = val_str.split('@').collect();
        if parts.len() != 2 {
            return WithType::fmt(val, f);
        }

        f.write_str(&format!("{}@{}", "*".repeat(parts[0].len()), parts[1]))
    }
}

pub struct IpAddress;

impl<T> Strategy<T> for IpAddress
where
    T: AsRef<str>,
{
    fn fmt(val: &T, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

        f.write_str(&format!("{}.**.**.**", segments[0]))
    }
}

#[cfg(test)]
mod pii_masking_strategy_tests {
    use super::{CardNumber, Email, IpAddress, Secret};

    #[test]
    fn test_valid_card_number_masking() {
        let secret: Secret<String, CardNumber> = Secret::new("1234567890987654".to_string());
        assert_eq!("123456**********", &format!("{:?}", secret));
    }

    #[test]
    fn test_invalid_card_number_masking() {
        let secret: Secret<String, CardNumber> = Secret::new("1234567890".to_string());
        assert_eq!("123456****", &format!("{:?}", secret));
    }

    /* #[test]
    fn test_valid_phone_number_masking() {
        let secret: Secret<String, PhoneNumber> = Secret::new("9922992299".to_string());
        assert_eq!("99*****299", &format!("{}", secret));
    }

    #[test]
    fn test_invalid_phone_number_masking() {
        let secret: Secret<String, PhoneNumber> = Secret::new("99229922".to_string());
        assert_eq!("*** alloc::string::String ***", &format!("{}", secret));

        let secret: Secret<String, PhoneNumber> = Secret::new("9922992299229922".to_string());
        assert_eq!("*** alloc::string::String ***", &format!("{}", secret));
    } */

    #[test]
    fn test_valid_email_masking() {
        let secret: Secret<String, Email> = Secret::new("myemail@gmail.com".to_string());
        assert_eq!("*******@gmail.com", &format!("{:?}", secret));
    }

    #[test]
    fn test_invalid_email_masking() {
        let secret: Secret<String, Email> = Secret::new("myemailgmail.com".to_string());
        assert_eq!("*** alloc::string::String ***", &format!("{:?}", secret));

        let secret: Secret<String, Email> = Secret::new("myemail@gmail@com".to_string());
        assert_eq!("*** alloc::string::String ***", &format!("{:?}", secret));
    }

    #[test]
    fn test_valid_ip_addr_masking() {
        let secret: Secret<String, IpAddress> = Secret::new("123.23.1.78".to_string());
        assert_eq!("123.**.**.**", &format!("{:?}", secret));
    }

    #[test]
    fn test_invalid_ip_addr_masking() {
        let secret: Secret<String, IpAddress> = Secret::new("123.4.56".to_string());
        assert_eq!("*** alloc::string::String ***", &format!("{:?}", secret));

        let secret: Secret<String, IpAddress> = Secret::new("123.4567.12.4".to_string());
        assert_eq!("*** alloc::string::String ***", &format!("{:?}", secret));

        let secret: Secret<String, IpAddress> = Secret::new("123..4.56".to_string());
        assert_eq!("*** alloc::string::String ***", &format!("{:?}", secret));
    }
}
