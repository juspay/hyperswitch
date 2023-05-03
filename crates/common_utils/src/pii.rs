//! Personal Identifiable Information protection.

use std::{convert::AsRef, fmt};

use masking::{Strategy, WithType};

use crate::validation::validate_email;

/// Type alias for serde_json value which has Secret Information
pub type SecretSerdeValue = masking::Secret<serde_json::Value>;

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

/// Client secret
#[derive(Debug)]
pub struct ClientSecret;

impl<T> Strategy<T> for ClientSecret
where
    T: AsRef<str>,
{
    fn fmt(val: &T, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val_str: &str = val.as_ref();

        let client_secret_segments: Vec<&str> = val_str.split('_').collect();

        if client_secret_segments.len() != 4
            || !client_secret_segments.contains(&"pay")
            || !client_secret_segments.contains(&"secret")
        {
            return WithType::fmt(val, f);
        }
        write!(
            f,
            "{}_{}_{}",
            client_secret_segments[0],
            client_secret_segments[1],
            "*".repeat(
                val_str.len()
                    - (client_secret_segments[0].len() + client_secret_segments[1].len() + 2)
            )
        )
    }
}

/// Email address
#[derive(Debug)]
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

        write!(f, "{}.**.**.**", segments[0])
    }
}

#[cfg(test)]
mod pii_masking_strategy_tests {
    use masking::Secret;

    use super::{ClientSecret, Email, IpAddress};

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
        assert_eq!("*******@gmail.com", format!("{secret:?}"));
    }

    #[test]
    fn test_invalid_email_masking() {
        let secret: Secret<String, Email> = Secret::new("myemailgmail.com".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{secret:?}"));

        let secret: Secret<String, Email> = Secret::new("myemail@gmail@com".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{secret:?}"));
    }

    #[test]
    fn test_valid_ip_addr_masking() {
        let secret: Secret<String, IpAddress> = Secret::new("123.23.1.78".to_string());
        assert_eq!("123.**.**.**", format!("{secret:?}"));
    }

    #[test]
    fn test_invalid_ip_addr_masking() {
        let secret: Secret<String, IpAddress> = Secret::new("123.4.56".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{secret:?}"));

        let secret: Secret<String, IpAddress> = Secret::new("123.4567.12.4".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{secret:?}"));

        let secret: Secret<String, IpAddress> = Secret::new("123..4.56".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{secret:?}"));
    }

    #[test]
    fn test_valid_client_secret_masking() {
        let secret: Secret<String, ClientSecret> =
            Secret::new("pay_uszFB2QGe9MmLY65ojhT_secret_tLjTz9tAQxUVEFqfmOIP".to_string());
        assert_eq!(
            "pay_uszFB2QGe9MmLY65ojhT_***************************",
            format!("{secret:?}")
        );
    }

    #[test]
    fn test_invalid_client_secret_masking() {
        let secret: Secret<String, IpAddress> =
            Secret::new("pay_uszFB2QGe9MmLY65ojhT_secret".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{secret:?}"));
    }
}
