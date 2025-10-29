use std::fmt;

use serde::{de::Visitor, Deserialize, Deserializer, Serialize};
use utoipa::ToSchema;

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "lowercase")]
pub enum ElementPosition {
    Left,
    #[default]
    #[serde(rename = "top left")]
    TopLeft,
    Top,
    #[serde(rename = "top right")]
    TopRight,
    Right,
    #[serde(rename = "bottom right")]
    BottomRight,
    Bottom,
    #[serde(rename = "bottom left")]
    BottomLeft,
    Center,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, strum::Display, strum::EnumString, ToSchema)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
pub enum ElementSize {
    Variants(SizeVariants),
    Percentage(u32),
    Pixels(u32),
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum SizeVariants {
    #[default]
    Cover,
    Contain,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum PaymentLinkDetailsLayout {
    #[default]
    Layout1,
    Layout2,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum PaymentLinkSdkLabelType {
    #[default]
    Above,
    Floating,
    Never,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum PaymentLinkShowSdkTerms {
    Always,
    #[default]
    Auto,
    Never,
}

impl<'de> Deserialize<'de> for ElementSize {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ElementSizeVisitor;

        impl Visitor<'_> for ElementSizeVisitor {
            type Value = ElementSize;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a string with possible values - contain, cover or values in percentage or pixels. For eg: 48px or 50%")
            }

            fn visit_str<E>(self, value: &str) -> Result<ElementSize, E>
            where
                E: serde::de::Error,
            {
                if let Some(percent) = value.strip_suffix('%') {
                    percent
                        .parse::<u32>()
                        .map(ElementSize::Percentage)
                        .map_err(E::custom)
                } else if let Some(px) = value.strip_suffix("px") {
                    px.parse::<u32>()
                        .map(ElementSize::Pixels)
                        .map_err(E::custom)
                } else {
                    match value {
                        "cover" => Ok(ElementSize::Variants(SizeVariants::Cover)),
                        "contain" => Ok(ElementSize::Variants(SizeVariants::Contain)),
                        _ => Err(E::custom("invalid size variant")),
                    }
                }
            }
        }

        deserializer.deserialize_str(ElementSizeVisitor)
    }
}

impl Serialize for ElementSize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        match self {
            Self::Variants(variant) => serializer.serialize_str(variant.to_string().as_str()),
            Self::Pixels(pixel_count) => {
                serializer.serialize_str(format!("{pixel_count}px").as_str())
            }
            Self::Percentage(pixel_count) => {
                serializer.serialize_str(format!("{pixel_count}%").as_str())
            }
        }
    }
}
