use api_models::payments::OrderDetailsWithAmount as ApiOrderDetailsWithAmount;
use diesel_models::types::OrderDetailsWithAmount as DieselOrderDetailsWithAmount;
use serde;

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct OrderDetailsWithAmount {
    /// Name of the product that is being purchased
    pub product_name: String,
    /// The quantity of the product to be purchased
    pub quantity: u16,
    /// the amount per quantity of product
    pub amount: i64,
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
}

impl masking::SerializableSecret for OrderDetailsWithAmount {}

impl crate::ApiDieselConvertor<ApiOrderDetailsWithAmount, DieselOrderDetailsWithAmount>
    for OrderDetailsWithAmount
{
    fn from_api(api_model: ApiOrderDetailsWithAmount) -> Self {
        Self::from(api_model)
    }

    fn to_api(&self) -> ApiOrderDetailsWithAmount {
        let Self {
            product_name,
            quantity,
            amount,
            requires_shipping,
            product_img_link,
            product_id,
            category,
            sub_category,
            brand,
            product_type,
            product_tax_code,
        } = self.clone();
        ApiOrderDetailsWithAmount {
            product_name,
            quantity,
            amount,
            requires_shipping,
            product_img_link,
            product_id,
            category,
            sub_category,
            brand,
            product_type,
            product_tax_code,
        }
    }

    fn from_diesel(diesel_model: DieselOrderDetailsWithAmount) -> Self {
        Self::from(diesel_model)
    }

    fn to_diesel(&self) -> DieselOrderDetailsWithAmount {
        let Self {
            product_name,
            quantity,
            amount,
            requires_shipping,
            product_img_link,
            product_id,
            category,
            sub_category,
            brand,
            product_type,
            product_tax_code,
        } = self.clone();
        DieselOrderDetailsWithAmount {
            product_name,
            quantity,
            amount,
            requires_shipping,
            product_img_link,
            product_id,
            category,
            sub_category,
            brand,
            product_type,
            product_tax_code,
        }
    }
}

impl From<ApiOrderDetailsWithAmount> for OrderDetailsWithAmount {
    fn from(value: ApiOrderDetailsWithAmount) -> Self {
        let ApiOrderDetailsWithAmount {
            product_name,
            quantity,
            amount,
            requires_shipping,
            product_img_link,
            product_id,
            category,
            sub_category,
            brand,
            product_type,
            product_tax_code,
        } = value;
        Self {
            product_name,
            quantity,
            amount,
            requires_shipping,
            product_img_link,
            product_id,
            category,
            sub_category,
            brand,
            product_type,
            product_tax_code,
        }
    }
}

impl From<DieselOrderDetailsWithAmount> for OrderDetailsWithAmount {
    fn from(value: DieselOrderDetailsWithAmount) -> Self {
        let DieselOrderDetailsWithAmount {
            product_name,
            quantity,
            amount,
            requires_shipping,
            product_img_link,
            product_id,
            category,
            sub_category,
            brand,
            product_type,
            product_tax_code,
        } = value;
        Self {
            product_name,
            quantity,
            amount,
            requires_shipping,
            product_img_link,
            product_id,
            category,
            sub_category,
            brand,
            product_type,
            product_tax_code,
        }
    }
}
