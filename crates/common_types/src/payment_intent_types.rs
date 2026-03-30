//! Types for payment_intent that need to be shared between diesel_models and domain_models

use common_utils::{impl_to_sql_from_sql_json, types::MinorUnit};
use diesel::{sql_types::Jsonb, AsExpression, FromSqlRow};
use serde::{Deserialize, Serialize};

/// Tax details for payment intent
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Jsonb)]
pub struct TaxDetails {
    /// This is the tax related information that is calculated irrespective of any payment method.
    /// This is calculated when the order is created with shipping details
    pub default: Option<DefaultTax>,

    /// This is the tax related information that is calculated based on the payment method
    /// This is calculated when calling the /calculate_tax API
    pub payment_method_type: Option<PaymentMethodTypeTax>,
}

impl TaxDetails {
    /// Get the tax amount
    /// If default tax is present, return the default tax amount
    /// If default tax is not present, return the tax amount based on the payment method if it matches the provided payment method type
    pub fn get_tax_amount(
        &self,
        payment_method: Option<common_enums::PaymentMethodType>,
    ) -> Option<MinorUnit> {
        self.payment_method_type
            .as_ref()
            .zip(payment_method)
            .filter(|(payment_method_type_tax, payment_method)| {
                payment_method_type_tax.pmt == *payment_method
            })
            .map(|(payment_method_type_tax, _)| payment_method_type_tax.order_tax_amount)
            .or_else(|| self.get_default_tax_amount())
    }

    /// Get the default tax amount
    pub fn get_default_tax_amount(&self) -> Option<MinorUnit> {
        self.default
            .as_ref()
            .map(|default_tax_details| default_tax_details.order_tax_amount)
    }
}

impl_to_sql_from_sql_json!(TaxDetails);

/// Tax information for a specific payment method type
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PaymentMethodTypeTax {
    /// The tax amount for the order
    pub order_tax_amount: MinorUnit,
    /// The payment method type
    pub pmt: common_enums::PaymentMethodType,
}

/// Default tax information
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct DefaultTax {
    /// The tax amount for the order
    pub order_tax_amount: MinorUnit,
}

/// Order details with amount information
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
    /// tax rate applicable to the product
    pub tax_rate: Option<f64>,
    /// total tax amount applicable to the product
    pub total_tax_amount: Option<MinorUnit>,
    /// description of the product
    pub description: Option<String>,
    /// stock keeping unit of the product
    pub sku: Option<String>,
    /// universal product code of the product
    pub upc: Option<String>,
    /// commodity code of the product
    pub commodity_code: Option<String>,
    /// unit of measure of the product
    pub unit_of_measure: Option<String>,
    /// total amount of the product
    pub total_amount: Option<MinorUnit>,
    /// discount amount on the unit
    pub unit_discount_amount: Option<MinorUnit>,
}

impl hyperswitch_masking::SerializableSecret for OrderDetailsWithAmount {}

impl_to_sql_from_sql_json!(OrderDetailsWithAmount);
