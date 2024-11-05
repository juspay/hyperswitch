pub mod api;
pub mod behaviour;
pub mod business_profile;
pub mod consts;
pub mod customer;
pub mod disputes;
pub mod errors;
pub mod mandates;
pub mod merchant_account;
pub mod merchant_connector_account;
pub mod merchant_key_store;
pub mod payment_address;
pub mod payment_method_data;
pub mod payment_methods;
pub mod payments;
#[cfg(feature = "payouts")]
pub mod payouts;
pub mod refunds;
pub mod router_data;
pub mod router_data_v2;
pub mod router_flow_types;
pub mod router_request_types;
pub mod router_response_types;
pub mod type_encryption;
pub mod types;

#[cfg(not(feature = "payouts"))]
pub trait PayoutAttemptInterface {}

#[cfg(not(feature = "payouts"))]
pub trait PayoutsInterface {}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub enum RemoteStorageObject<T: ForeignIDRef> {
    ForeignID(String),
    Object(T),
}

impl<T: ForeignIDRef> From<T> for RemoteStorageObject<T> {
    fn from(value: T) -> Self {
        Self::Object(value)
    }
}

pub trait ForeignIDRef {
    fn foreign_id(&self) -> String;
}

impl<T: ForeignIDRef> RemoteStorageObject<T> {
    pub fn get_id(&self) -> String {
        match self {
            Self::ForeignID(id) => id.clone(),
            Self::Object(i) => i.foreign_id(),
        }
    }
}

use std::fmt::Debug;

pub trait ApiModelToDieselModelConvertor<F> {
    /// Convert from a foreign type to the current type
    fn convert_from(from: F) -> Self;
}

#[cfg(feature = "v2")]
impl ApiModelToDieselModelConvertor<api_models::admin::PaymentLinkConfigRequest>
    for diesel_models::payment_intent::PaymentLinkConfigRequestForPayments
{
    fn convert_from(item: api_models::admin::PaymentLinkConfigRequest) -> Self {
        Self {
            theme: item.theme,
            logo: item.logo,
            seller_name: item.seller_name,
            sdk_layout: item.sdk_layout,
            display_sdk_only: item.display_sdk_only,
            enabled_saved_payment_method: item.enabled_saved_payment_method,
            transaction_details: item.transaction_details.map(|transaction_details| {
                transaction_details
                    .into_iter()
                    .map(|transaction_detail| {
                        diesel_models::PaymentLinkTransactionDetails::convert_from(
                            transaction_detail,
                        )
                    })
                    .collect()
            }),
        }
    }
}

#[cfg(feature = "v2")]
impl ApiModelToDieselModelConvertor<api_models::admin::PaymentLinkTransactionDetails>
    for diesel_models::PaymentLinkTransactionDetails
{
    fn convert_from(from: api_models::admin::PaymentLinkTransactionDetails) -> Self {
        Self {
            key: from.key,
            value: from.value,
            ui_configuration: from
                .ui_configuration
                .map(diesel_models::TransactionDetailsUiConfiguration::convert_from),
        }
    }
}

#[cfg(feature = "v2")]
impl ApiModelToDieselModelConvertor<api_models::admin::TransactionDetailsUiConfiguration>
    for diesel_models::TransactionDetailsUiConfiguration
{
    fn convert_from(from: api_models::admin::TransactionDetailsUiConfiguration) -> Self {
        Self {
            position: from.position,
            is_key_bold: from.is_key_bold,
            is_value_bold: from.is_value_bold,
        }
    }
}

#[cfg(feature = "v2")]
impl From<api_models::payments::AmountDetails> for payments::AmountDetails {
    fn from(amount_details: api_models::payments::AmountDetails) -> Self {
        Self {
            order_amount: amount_details.order_amount().into(),
            currency: amount_details.currency(),
            shipping_cost: amount_details.shipping_cost(),
            tax_details: amount_details.order_tax_amount().map(|order_tax_amount| {
                diesel_models::TaxDetails {
                    default: Some(diesel_models::DefaultTax { order_tax_amount }),
                    payment_method_type: None,
                }
            }),
            skip_external_tax_calculation: payments::TaxCalculationOverride::from(
                amount_details.skip_external_tax_calculation(),
            ),
            skip_surcharge_calculation: payments::SurchargeCalculationOverride::from(
                amount_details.skip_surcharge_calculation(),
            ),
            surcharge_amount: amount_details.surcharge_amount(),
            tax_on_surcharge: amount_details.tax_on_surcharge(),
            // We will not receive this in the request. This will be populated after calling the connector / processor
            amount_captured: None,
        }
    }
}

#[cfg(feature = "v2")]
impl From<common_enums::SurchargeCalculationOverride> for payments::SurchargeCalculationOverride {
    fn from(surcharge_calculation_override: common_enums::SurchargeCalculationOverride) -> Self {
        match surcharge_calculation_override {
            common_enums::SurchargeCalculationOverride::Calculate => Self::Calculate,
            common_enums::SurchargeCalculationOverride::Skip => Self::Skip,
        }
    }
}

#[cfg(feature = "v2")]
impl From<common_enums::TaxCalculationOverride> for payments::TaxCalculationOverride {
    fn from(tax_calculation_override: common_enums::TaxCalculationOverride) -> Self {
        match tax_calculation_override {
            common_enums::TaxCalculationOverride::Calculate => Self::Calculate,
            common_enums::TaxCalculationOverride::Skip => Self::Skip,
        }
    }
}
