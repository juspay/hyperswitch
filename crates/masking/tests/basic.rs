#![allow(dead_code, clippy::unwrap_used, clippy::panic_in_result_fn)]

use masking::Secret;
#[cfg(feature = "serde")]
use masking::SerializableSecret;
#[cfg(feature = "alloc")]
use masking::ZeroizableSecret;
#[cfg(feature = "serde")]
use serde::Serialize;

#[test]
fn basic() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg_attr(feature = "serde", derive(Serialize))]
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct AccountNumber(String);

    #[cfg(feature = "alloc")]
    impl ZeroizableSecret for AccountNumber {
        fn zeroize(&mut self) {
            self.0.zeroize();
        }
    }

    #[cfg(feature = "serde")]
    impl SerializableSecret for AccountNumber {}

    #[cfg_attr(feature = "serde", derive(Serialize))]
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Composite {
        secret_number: Secret<AccountNumber>,
        not_secret: String,
    }

    // construct

    let secret_number = Secret::<AccountNumber>::new(AccountNumber("abc".to_string()));
    let not_secret = "not secret".to_string();
    let composite = Composite {
        secret_number,
        not_secret,
    };

    // clone
    #[allow(clippy::redundant_clone)] // We are asserting that the cloned value is equal
    let composite2 = composite.clone();
    assert_eq!(composite, composite2);

    // format

    let got = format!("{composite:?}");
    let exp = r#"Composite { secret_number: *** basic::basic::AccountNumber ***, not_secret: "not secret" }"#;
    assert_eq!(got, exp);

    // serialize

    #[cfg(feature = "serde")]
    {
        let got = serde_json::to_string(&composite).unwrap();
        let exp = r#"{"secret_number":"abc","not_secret":"not secret"}"#;
        assert_eq!(got, exp);
    }

    // end

    Ok(())
}

#[test]
fn without_serialize() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg_attr(feature = "serde", derive(Serialize))]
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct AccountNumber(String);

    #[cfg(feature = "alloc")]
    impl ZeroizableSecret for AccountNumber {
        fn zeroize(&mut self) {
            self.0.zeroize();
        }
    }

    #[cfg_attr(feature = "serde", derive(Serialize))]
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Composite {
        #[cfg_attr(feature = "serde", serde(skip))]
        secret_number: Secret<AccountNumber>,
        not_secret: String,
    }

    // construct

    let secret_number = Secret::<AccountNumber>::new(AccountNumber("abc".to_string()));
    let not_secret = "not secret".to_string();
    let composite = Composite {
        secret_number,
        not_secret,
    };

    // format

    let got = format!("{composite:?}");
    let exp = r#"Composite { secret_number: *** basic::without_serialize::AccountNumber ***, not_secret: "not secret" }"#;
    assert_eq!(got, exp);

    // serialize

    #[cfg(feature = "serde")]
    {
        let got = serde_json::to_string(&composite).unwrap();
        let exp = r#"{"not_secret":"not secret"}"#;
        assert_eq!(got, exp);
    }

    // end

    Ok(())
}

#[test]
fn for_string() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg_attr(all(feature = "alloc", feature = "serde"), derive(Serialize))]
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Composite {
        secret_number: Secret<String>,
        not_secret: String,
    }

    // construct

    let secret_number = Secret::<String>::new("abc".to_string());
    let not_secret = "not secret".to_string();
    let composite = Composite {
        secret_number,
        not_secret,
    };

    // clone
    #[allow(clippy::redundant_clone)] // We are asserting that the cloned value is equal
    let composite2 = composite.clone();
    assert_eq!(composite, composite2);

    // format

    let got = format!("{composite:?}");
    let exp =
        r#"Composite { secret_number: *** alloc::string::String ***, not_secret: "not secret" }"#;
    assert_eq!(got, exp);

    // serialize

    #[cfg(all(feature = "alloc", feature = "serde"))]
    {
        let got = serde_json::to_string(&composite).unwrap();
        let exp = r#"{"secret_number":"abc","not_secret":"not secret"}"#;
        assert_eq!(got, exp);
    }

    // end

    Ok(())
}
