use common_utils::{hashing::HashedString, pii, types::MinorUnit};
use diesel::{
    sql_types::{Json, Jsonb},
    AsExpression, FromSqlRow,
};
use masking::{Secret, WithType};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Jsonb)]
pub struct OrderDetailsWithAmount {
    /// Name of the product that is being purchased
    pub product_name: String,
    /// The quantity of the product to be purchased
    pub quantity: u16,
    /// the amount per quantity of product
    pub amount: MinorUnit,
    // Does the order includes shipping
    pub requires_shipping: Option<bool>,
    /// The image URL of the product
    pub product_img_link: Option<String>,
    /// ID of the product that is being purchased
    pub product_id: Option<String>,
    /// Category of the product that is being purchased
    pub category: Option<String>,
    /// Sub category of the product that is being purchased
    pub sub_category: Option<String>,
    /// Brand of the product that is being purchased
    pub brand: Option<String>,
    /// Type of the product that is being purchased
    pub product_type: Option<common_enums::ProductType>,
    /// The tax code for the product
    pub product_tax_code: Option<String>,
    pub tax_rate: Option<i64>,
    pub total_tax_amount: Option<MinorUnit>,
}

impl masking::SerializableSecret for OrderDetailsWithAmount {}

common_utils::impl_to_sql_from_sql_json!(OrderDetailsWithAmount);

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Json)]
pub struct FeatureMetadata {
    /// Redirection response coming in request as metadata field only for redirection scenarios
    pub redirect_response: Option<RedirectResponse>,
    // TODO: Convert this to hashedstrings to avoid PII sensitive data
    /// Additional tags to be used for global search
    pub search_tags: Option<Vec<HashedString<WithType>>>,
}
impl masking::SerializableSecret for FeatureMetadata {}
common_utils::impl_to_sql_from_sql_json!(FeatureMetadata);

#[derive(Default, Debug, Eq, PartialEq, Deserialize, Serialize, Clone)]
pub struct RedirectResponse {
    pub param: Option<Secret<String>>,
    pub json_payload: Option<pii::SecretSerdeValue>,
}
impl masking::SerializableSecret for RedirectResponse {}
common_utils::impl_to_sql_from_sql_json!(RedirectResponse);
