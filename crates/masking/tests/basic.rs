#![allow(dead_code, clippy::unwrap_used, clippy::panic_in_result_fn)]

use masking as pii;

#[test]
fn basic() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use pii::{Secret, SerializableSecret, ZeroizableSecret};
    use serde::Serialize;

    #[derive(Clone, Debug, Serialize, PartialEq, Eq)]
    pub struct AccountNumber(String);

    impl ZeroizableSecret for AccountNumber {
        fn zeroize(&mut self) {
            self.0.zeroize();
        }
    }

    impl SerializableSecret for AccountNumber {}

    #[derive(Clone, Debug, Serialize, PartialEq, Eq)]
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

    let composite2 = composite.clone();
    assert_eq!(composite, composite2);

    // format

    let got = format!("{:?}", composite);
    let exp = "Composite { secret_number: *** basic::basic::AccountNumber ***, not_secret: \"not secret\" }";
    assert_eq!(got, exp);

    // serialize

    let got = serde_json::to_string(&composite).unwrap();
    let exp = "{\"secret_number\":\"abc\",\"not_secret\":\"not secret\"}";
    assert_eq!(got, exp);

    // end

    Ok(())
}

#[test]
fn without_serialize() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use pii::{Secret, ZeroizableSecret};
    use serde::Serialize;

    #[derive(Clone, Debug, Serialize, PartialEq, Eq)]
    pub struct AccountNumber(String);

    impl ZeroizableSecret for AccountNumber {
        fn zeroize(&mut self) {
            self.0.zeroize();
        }
    }

    #[derive(Clone, Debug, Serialize, PartialEq, Eq)]
    pub struct Composite {
        #[serde(skip)]
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

    let got = format!("{:?}", composite);
    let exp = "Composite { secret_number: *** basic::without_serialize::AccountNumber ***, not_secret: \"not secret\" }";
    assert_eq!(got, exp);

    // serialize

    let got = serde_json::to_string(&composite).unwrap();
    let exp = "{\"not_secret\":\"not secret\"}";
    assert_eq!(got, exp);

    // end

    Ok(())
}

#[test]
fn for_string() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use pii::Secret;
    use serde::Serialize;

    #[derive(Clone, Debug, Serialize, PartialEq, Eq)]
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

    let composite2 = composite.clone();
    assert_eq!(composite, composite2);

    // format

    let got = format!("{:?}", composite);
    let exp =
        "Composite { secret_number: *** alloc::string::String ***, not_secret: \"not secret\" }";
    assert_eq!(got, exp);

    // serialize

    let got = serde_json::to_string(&composite).unwrap();
    let exp = "{\"secret_number\":\"abc\",\"not_secret\":\"not secret\"}";
    assert_eq!(got, exp);

    // end

    Ok(())
}
