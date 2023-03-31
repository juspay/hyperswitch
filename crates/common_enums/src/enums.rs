use router_derive;
use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Debug,
    Eq,
    Default,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    utoipa::ToSchema,
    Copy
)]
#[router_derive::diesel_enum(storage_type = "pg_enum")]
#[rustfmt::skip]
pub enum CountryCode {
    AF, AX, AL, DZ, AS, AD, AO, AI, AQ, AG, AR, AM, AW, AU, AT,
    AZ, BS, BH, BD, BB, BY, BE, BZ, BJ, BM, BT, BO, BQ, BA, BW,
    BV, BR, IO, BN, BG, BF, BI, KH, CM, CA, CV, KY, CF, TD, CL,
    CN, CX, CC, CO, KM, CG, CD, CK, CR, CI, HR, CU, CW, CY, CZ,
    DK, DJ, DM, DO, EC, EG, SV, GQ, ER, EE, ET, FK, FO, FJ, FI,
    FR, GF, PF, TF, GA, GM, GE, DE, GH, GI, GR, GL, GD, GP, GU,
    GT, GG, GN, GW, GY, HT, HM, VA, HN, HK, HU, IS, IN, ID, IR,
    IQ, IE, IM, IL, IT, JM, JP, JE, JO, KZ, KE, KI, KP, KR, KW,
    KG, LA, LV, LB, LS, LR, LY, LI, LT, LU, MO, MK, MG, MW, MY,
    MV, ML, MT, MH, MQ, MR, MU, YT, MX, FM, MD, MC, MN, ME, MS,
    MA, MZ, MM, NA, NR, NP, NL, NC, NZ, NI, NE, NG, NU, NF, MP,
    NO, OM, PK, PW, PS, PA, PG, PY, PE, PH, PN, PL, PT, PR, QA,
    RE, RO, RU, RW, BL, SH, KN, LC, MF, PM, VC, WS, SM, ST, SA,
    SN, RS, SC, SL, SG, SX, SK, SI, SB, SO, ZA, GS, SS, ES, LK,
    SD, SR, SJ, SZ, SE, CH, SY, TW, TJ, TZ, TH, TL, TG, TK, TO,
    TT, TN, TR, TM, TC, TV, UG, UA, AE, GB, UM, UY, UZ, VU,
    VE, VN, VG, VI, WF, EH, YE, ZM, ZW,
    #[default]
    US
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CountryAlpha2 {
    IN,
    AU,
    NZ,
    SA,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CountryAlpha3 {
    IND,
    AUS,
    SAF,
    NEZ,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CountryNumeric {
    one,
    two,
    three,
    four,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Country {
    Australia,
    NewZealand,
    India,
    SouthAfrica,
}

impl Country {
    pub const fn from_alpha2(code: CountryAlpha2) -> Self {
        match code {
            CountryAlpha2::IN => Self::India,
            CountryAlpha2::AU => Self::Australia,
            CountryAlpha2::NZ => Self::NewZealand,
            CountryAlpha2::SA => Self::SouthAfrica,
        }
    }
    pub const fn to_alpha2(&self) -> CountryAlpha2 {
        match self {
            Country::Australia => CountryAlpha2::AU,
            Country::NewZealand => CountryAlpha2::NZ,
            Country::India => CountryAlpha2::IN,
            Country::SouthAfrica => CountryAlpha2::SA,
        }
    }
    pub const fn from_alpha3(code: CountryAlpha3) -> Self {
        match code {
            CountryAlpha3::IND => Self::India,
            CountryAlpha3::AUS => Self::Australia,
            CountryAlpha3::NEZ => Self::NewZealand,
            CountryAlpha3::SAF => Self::SouthAfrica,
        }
    }
    pub const fn to_alpha3(&self) -> CountryAlpha3 {
        match self {
            Country::Australia => CountryAlpha3::AUS,
            Country::NewZealand => CountryAlpha3::NEZ,
            Country::India => CountryAlpha3::IND,
            Country::SouthAfrica => CountryAlpha3::SAF,
        }
    }
    pub const fn from_numeric(code: CountryNumeric) -> Self {
        match code {
            CountryNumeric::one => Self::India,
            CountryNumeric::two => Self::Australia,
            CountryNumeric::three => Self::NewZealand,
            CountryNumeric::four => Self::SouthAfrica,
        }
    }
    pub const fn to_numeric(&self) -> CountryNumeric {
        match self {
            Country::Australia => CountryNumeric::one,
            Country::NewZealand => CountryNumeric::two,
            Country::India => CountryNumeric::three,
            Country::SouthAfrica => CountryNumeric::four,
        }
    }
}

mod custom_serde {
    use super::*;

    pub mod alpha2_country_code {
        use std::{fmt, string::ParseError};

        use serde::de::Visitor;

        use super::*;

        pub fn serialize<S>(code: &Country, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            code.to_alpha2().serialize(serializer)
        }

        struct FieldVisitor;

        impl<'de> Visitor<'de> for FieldVisitor {
            type Value = CountryAlpha2;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("CountryAlpha2 as a string")
            }
        }

        pub fn deserialize<'a, D>(deserializer: D) -> Result<Country, D::Error>
        where
            D: serde::Deserializer<'a>,
        {
            dbg!("called????????????????????");
            let result = deserializer
                .deserialize_str(FieldVisitor)
                .map(Country::from_alpha2);
            dbg!(&result);
            return result;
        }
    }

    pub mod alpha3_country_code {
        use std::fmt;

        use serde::de::Visitor;

        use super::*;

        pub fn serialize<S>(code: &Country, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            code.to_alpha3().serialize(serializer)
        }

        struct FieldVisitor;

        impl<'de> Visitor<'de> for FieldVisitor {
            type Value = CountryAlpha3;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("CountryAlpha3 as a string")
            }
        }

        pub fn deserialize<'a, D>(deserializer: D) -> Result<Country, D::Error>
        where
            D: serde::Deserializer<'a>,
        {
            return deserializer
                .deserialize_str(FieldVisitor)
                .map(Country::from_alpha3);
        }
    }

    pub mod numeric_country_code {
        use std::fmt;

        use serde::de::Visitor;

        use super::*;

        pub fn serialize<S>(code: &Country, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            code.to_numeric().serialize(serializer)
        }

        struct FieldVisitor;

        impl<'de> Visitor<'de> for FieldVisitor {
            type Value = CountryNumeric;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("CountryNumeric as a string")
            }
        }

        pub fn deserialize<'a, D>(deserializer: D) -> Result<Country, D::Error>
        where
            D: serde::Deserializer<'a>,
        {
            return deserializer
                .deserialize_str(FieldVisitor)
                .map(Country::from_numeric);
        }
    }
}

#[derive(Deserialize, Serialize)]
struct Address {
    #[serde(with = "custom_serde::alpha2_country_code")]
    country: Country,
}
#[derive(serde::Serialize)]
struct Alpha2Request {
    #[serde(with = "custom_serde::alpha2_country_code")]
    pub country: Country,
}

#[derive(serde::Serialize)]
struct Alpha3Request {
    #[serde(with = "custom_serde::alpha3_country_code")]
    pub country: Country,
}

#[derive(serde::Serialize)]
struct NumericRequest {
    #[serde(with = "custom_serde::numeric_country_code")]
    pub country: Country,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct HyperswitchRequestAlpha2 {
    #[serde(with = "custom_serde::alpha2_country_code")]
    pub country: Country,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct HyperswitchRequestAlpha3 {
    #[serde(with = "custom_serde::alpha3_country_code")]
    pub country: Country,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct HyperswitchRequestNumeric {
    #[serde(with = "custom_serde::numeric_country_code")]
    pub country: Country,
}

// #[derive(PartialEq, Serialize, Deserialize, Debug)]
// struct Test {
//     #[serde(with = "custom_serde::alpha2_country_code")]
//     country: Country,
// }
//
// #[derive(PartialEq, Serialize, Deserialize, Debug)]
// struct TestWithOtherName {
//     #[serde(with = "custom_serde::alpha2_country_code")]
//     country: Country,
// }

#[cfg(test)]
mod tests {
    use super::*;

    /* #[test]
    fn INIT() {
        let test = Test {
            country: Country::India,
        };
        let serialized = serde_json::to_string(&test).unwrap();
        dbg!(&serialized);

        let get_test: TestWithOtherName =
            serde_json::from_str::<TestWithOtherName>(&serialized).unwrap();
        dbg!(&get_test);
        assert_eq!(test.country, get_test.country);
    } */

    /* #[test]
    fn checking() {
        let request = HyperswitchRequestAlpha2 {
            country: Country::India,
        };
        let serialized_country = serde_json::to_string(&request).unwrap();
        assert_eq!(serialized_country, r#"{"country":"IN"}"#);
    } */

    #[test]
    fn test_serialize_alpha2() {
        let x_request = Alpha2Request {
            country: Country::India,
        };
        let serialized_country = serde_json::to_string(&x_request).unwrap();
        assert_eq!(serialized_country, r#"{"country":"IN"}"#)
    }

    #[test]
    fn test_serialize_alpha3() {
        let y_request = Alpha3Request {
            country: Country::India,
        };
        let serialized_country = serde_json::to_string(&y_request).unwrap();
        assert_eq!(serialized_country, r#"{"country":"IND"}"#)
    }

    #[test]
    fn test_serialize_numeric() {
        let y_request = NumericRequest {
            country: Country::India,
        };
        let serialized_country = serde_json::to_string(&y_request).unwrap();
        assert_eq!(serialized_country, r#"{"country":"three"}"#)
    }

    #[test]
    fn test_deserialize_alpha2() {
        let request_str = r#"{"country":"IN"}"#;
        let request = serde_json::from_str::<HyperswitchRequestAlpha2>(request_str).unwrap();
        assert_eq!(request.country, Country::India)
    }

    #[test]
    fn test_deserialize_alpha3() {
        let request_str = r#"{"country":"IND"}"#;
        let request = serde_json::from_str::<HyperswitchRequestAlpha3>(request_str).unwrap();
        assert_eq!(request.country, Country::India)
    }

    #[test]
    fn test_deserialize_numeric() {
        let request_str = r#"{"country":"three"}"#;
        let request = serde_json::from_str::<HyperswitchRequestNumeric>(request_str).unwrap();
        assert_eq!(request.country, Country::India)
    }

    #[test]
    fn test_deserialize_and_serialize() {
        // Deserialize the country as alpha2 code
        // Serialize the country as alpha3 code
        let request_str = r#"{"country":"IN"}"#;
        let request = serde_json::from_str::<HyperswitchRequestAlpha2>(request_str).unwrap();
        let alpha3_request = Alpha3Request {
            country: request.country,
        };
        let response = serde_json::to_string::<Alpha3Request>(&alpha3_request).unwrap();
        assert_eq!(response, r#"{"country":"IND"}"#)
    }
}
