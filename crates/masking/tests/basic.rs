#![allow(dead_code, clippy::panic_in_result_fn)]

use masking::Secret;
#[cfg(feature = "serde")]
use masking::SerializableSecret;
#[cfg(feature = "alloc")]
use masking::ZeroizableSecret;
#[cfg(feature = "serde")]
use serde::Serialize;

#[test]
fn basic() {
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
}

#[allow(clippy::indexing_slicing)]
#[cfg(feature = "serde")]
#[test]
fn test_flatten_with_secret() {
    use masking::Secret;
    use serde::Serialize;

    #[derive(Serialize)]
    pub struct Inner {
        secret: Secret<String>,
        inner_field: String,
    }

    #[derive(Serialize)]
    pub struct Outer {
        #[serde(flatten)]
        inner: Inner,
        outer_field: String,
    }

    let inner = Inner {
        secret: Secret::new("secret_value".to_string()),
        inner_field: "inner".to_string(),
    };
    let outer = Outer {
        inner,
        outer_field: "outer".to_string(),
    };

    // Regular serialization should work (exposes the secret)
    let json = serde_json::to_string(&outer).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["inner_field"], "inner");
    assert_eq!(parsed["outer_field"], "outer");
    assert_eq!(parsed["secret"], "secret_value");

    // Masked serialization should also work
    let masked = masking::masked_serialize(&outer).unwrap();
    assert_eq!(masked["inner_field"], "inner");
    assert_eq!(masked["outer_field"], "outer");
    // The secret should be masked in the masked serialization
    assert!(masked["secret"].as_str().unwrap().contains("***"));
}

#[test]
fn without_serialize() {
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
}

#[test]
fn for_string() {
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
}
