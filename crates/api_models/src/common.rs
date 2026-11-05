//! Common types shared across multiple API model modules.

use hyperswitch_masking::Secret;
use smithy::SmithyModel;
use utoipa::ToSchema;

/// Phone contact details of a party.
#[derive(
    Debug,
    Clone,
    Default,
    Eq,
    PartialEq,
    ToSchema,
    serde::Deserialize,
    serde::Serialize,
    SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct PhoneDetails {
    /// The contact number
    #[schema(value_type = Option<String>, example = "9123456789")]
    #[smithy(value_type = "Option<String>")]
    pub number: Option<Secret<String>>,
    /// The country code attached to the number
    #[schema(example = "+1")]
    #[smithy(value_type = "Option<String>")]
    pub country_code: Option<String>,
}
