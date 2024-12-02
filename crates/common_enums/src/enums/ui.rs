use serde;
use utoipa::ToSchema;

#[derive(Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ElementPosition {
    Left,
    #[default]
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Center,
}

#[derive(Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub enum ElementSize {
    Variants(SizeVariants),
    Percentage(u32),
    Pixels(u32),
}

#[derive(Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub enum SizeVariants {
    Cover,
    Contain,
}
