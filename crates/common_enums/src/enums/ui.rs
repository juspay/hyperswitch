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
    strum::AsRefStr,
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
            Self::Variants(variant) => serializer.serialize_str(variant.as_ref()),
            Self::Pixels(pixel_count) => serializer.collect_str(&format_args!("{pixel_count}px")),
            Self::Percentage(pixel_count) => {
                serializer.collect_str(&format_args!("{pixel_count}%"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size_variants_serialize_cover() {
        let variant = SizeVariants::Cover;
        let serialized = serde_json::to_string(&variant).unwrap();
        assert_eq!(serialized, r#""cover""#);
    }

    #[test]
    fn test_size_variants_serialize_contain() {
        let variant = SizeVariants::Contain;
        let serialized = serde_json::to_string(&variant).unwrap();
        assert_eq!(serialized, r#""contain""#);
    }

    #[test]
    fn test_size_variants_deserialize_cover() {
        let json = r#""cover""#;
        let variant: SizeVariants = serde_json::from_str(json).unwrap();
        assert_eq!(variant, SizeVariants::Cover);
    }

    #[test]
    fn test_size_variants_deserialize_contain() {
        let json = r#""contain""#;
        let variant: SizeVariants = serde_json::from_str(json).unwrap();
        assert_eq!(variant, SizeVariants::Contain);
    }

    #[test]
    fn test_element_size_serialize_variants_cover() {
        let size = ElementSize::Variants(SizeVariants::Cover);
        let serialized = serde_json::to_string(&size).unwrap();
        assert_eq!(serialized, r#""cover""#);
    }

    #[test]
    fn test_element_size_serialize_variants_contain() {
        let size = ElementSize::Variants(SizeVariants::Contain);
        let serialized = serde_json::to_string(&size).unwrap();
        assert_eq!(serialized, r#""contain""#);
    }

    #[test]
    fn test_element_size_serialize_pixels() {
        let size = ElementSize::Pixels(42);
        let serialized = serde_json::to_string(&size).unwrap();
        assert_eq!(serialized, r#""42px""#);

        let size = ElementSize::Pixels(100);
        let serialized = serde_json::to_string(&size).unwrap();
        assert_eq!(serialized, r#""100px""#);
    }

    #[test]
    fn test_element_size_serialize_percentage() {
        let size = ElementSize::Percentage(50);
        let serialized = serde_json::to_string(&size).unwrap();
        assert_eq!(serialized, r#""50%""#);

        let size = ElementSize::Percentage(100);
        let serialized = serde_json::to_string(&size).unwrap();
        assert_eq!(serialized, r#""100%""#);
    }

    #[test]
    fn test_element_size_deserialize_cover() {
        let json = r#""cover""#;
        let size: ElementSize = serde_json::from_str(json).unwrap();
        assert_eq!(size, ElementSize::Variants(SizeVariants::Cover));
    }

    #[test]
    fn test_element_size_deserialize_contain() {
        let json = r#""contain""#;
        let size: ElementSize = serde_json::from_str(json).unwrap();
        assert_eq!(size, ElementSize::Variants(SizeVariants::Contain));
    }

    #[test]
    fn test_element_size_deserialize_pixels() {
        let json = r#""42px""#;
        let size: ElementSize = serde_json::from_str(json).unwrap();
        assert_eq!(size, ElementSize::Pixels(42));

        let json = r#""320px""#;
        let size: ElementSize = serde_json::from_str(json).unwrap();
        assert_eq!(size, ElementSize::Pixels(320));
    }

    #[test]
    fn test_element_size_deserialize_percentage() {
        let json = r#""50%""#;
        let size: ElementSize = serde_json::from_str(json).unwrap();
        assert_eq!(size, ElementSize::Percentage(50));

        let json = r#""100%""#;
        let size: ElementSize = serde_json::from_str(json).unwrap();
        assert_eq!(size, ElementSize::Percentage(100));
    }

    #[test]
    fn test_element_size_deserialize_invalid_variant() {
        let json = r#""invalid""#;
        let result: Result<ElementSize, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_element_size_deserialize_invalid_pixel_format() {
        let json = r#""42""#;
        let result: Result<ElementSize, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_element_size_deserialize_invalid_number() {
        let json = r#""notanumberpx""#;
        let result: Result<ElementSize, _> = serde_json::from_str(json);
        assert!(result.is_err());

        let json = r#""notanumber%""#;
        let result: Result<ElementSize, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_element_size_round_trip_variants() {
        let original = ElementSize::Variants(SizeVariants::Cover);
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: ElementSize = serde_json::from_str(&serialized).unwrap();
        assert_eq!(original, deserialized);

        let original = ElementSize::Variants(SizeVariants::Contain);
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: ElementSize = serde_json::from_str(&serialized).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_element_size_round_trip_pixels() {
        let original = ElementSize::Pixels(768);
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: ElementSize = serde_json::from_str(&serialized).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_element_size_round_trip_percentage() {
        let original = ElementSize::Percentage(75);
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: ElementSize = serde_json::from_str(&serialized).unwrap();
        assert_eq!(original, deserialized);
    }
}
