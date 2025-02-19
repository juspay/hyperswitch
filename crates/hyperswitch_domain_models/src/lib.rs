pub mod address;
pub mod api;
pub mod behaviour;
pub mod business_profile;
pub mod callback_mapper;
pub mod consts;
pub mod customer;
pub mod disputes;
pub mod errors;
pub mod mandates;
pub mod merchant_account;
pub mod merchant_connector_account;
pub mod merchant_key_store;
pub mod network_tokenization;
pub mod payment_address;
pub mod payment_method_data;
pub mod payment_methods;
pub mod payments;
#[cfg(feature = "payouts")]
pub mod payouts;
pub mod refunds;
pub mod relay;
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

use api_models::payments::{
    ApplePayRecurringDetails as ApiApplePayRecurringDetails,
    ApplePayRegularBillingDetails as ApiApplePayRegularBillingDetails,
    FeatureMetadata as ApiFeatureMetadata, OrderDetailsWithAmount as ApiOrderDetailsWithAmount,
    RecurringPaymentIntervalUnit as ApiRecurringPaymentIntervalUnit,
    RedirectResponse as ApiRedirectResponse,
};
#[cfg(feature = "v2")]
use api_models::payments::{
    BillingConnectorPaymentDetails as ApiBillingConnectorPaymentDetails,
    PaymentRevenueRecoveryMetadata as ApiRevenueRecoveryMetadata,
};
use diesel_models::types::{
    ApplePayRecurringDetails, ApplePayRegularBillingDetails, FeatureMetadata,
    OrderDetailsWithAmount, RecurringPaymentIntervalUnit, RedirectResponse,
};
#[cfg(feature = "v2")]
use diesel_models::types::{BillingConnectorPaymentDetails, PaymentRevenueRecoveryMetadata};

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
    fn convert_back(self) -> F;
}

#[cfg(feature = "v1")]
impl ApiModelToDieselModelConvertor<ApiFeatureMetadata> for FeatureMetadata {
    fn convert_from(from: ApiFeatureMetadata) -> Self {
        let ApiFeatureMetadata {
            redirect_response,
            search_tags,
            apple_pay_recurring_details,
        } = from;

        Self {
            redirect_response: redirect_response.map(RedirectResponse::convert_from),
            search_tags,
            apple_pay_recurring_details: apple_pay_recurring_details
                .map(ApplePayRecurringDetails::convert_from),
        }
    }

    fn convert_back(self) -> ApiFeatureMetadata {
        let Self {
            redirect_response,
            search_tags,
            apple_pay_recurring_details,
        } = self;

        ApiFeatureMetadata {
            redirect_response: redirect_response
                .map(|redirect_response| redirect_response.convert_back()),
            search_tags,
            apple_pay_recurring_details: apple_pay_recurring_details
                .map(|value| value.convert_back()),
        }
    }
}

#[cfg(feature = "v2")]
impl ApiModelToDieselModelConvertor<ApiFeatureMetadata> for FeatureMetadata {
    fn convert_from(from: ApiFeatureMetadata) -> Self {
        let ApiFeatureMetadata {
            redirect_response,
            search_tags,
            apple_pay_recurring_details,
            payment_revenue_recovery_metadata,
        } = from;

        Self {
            redirect_response: redirect_response.map(RedirectResponse::convert_from),
            search_tags,
            apple_pay_recurring_details: apple_pay_recurring_details
                .map(ApplePayRecurringDetails::convert_from),
            payment_revenue_recovery_metadata: payment_revenue_recovery_metadata
                .map(PaymentRevenueRecoveryMetadata::convert_from),
        }
    }

    fn convert_back(self) -> ApiFeatureMetadata {
        let Self {
            redirect_response,
            search_tags,
            apple_pay_recurring_details,
            payment_revenue_recovery_metadata,
        } = self;

        ApiFeatureMetadata {
            redirect_response: redirect_response
                .map(|redirect_response| redirect_response.convert_back()),
            search_tags,
            apple_pay_recurring_details: apple_pay_recurring_details
                .map(|value| value.convert_back()),
            payment_revenue_recovery_metadata: payment_revenue_recovery_metadata
                .map(|value| value.convert_back()),
        }
    }
}

impl ApiModelToDieselModelConvertor<ApiRedirectResponse> for RedirectResponse {
    fn convert_from(from: ApiRedirectResponse) -> Self {
        let ApiRedirectResponse {
            param,
            json_payload,
        } = from;
        Self {
            param,
            json_payload,
        }
    }

    fn convert_back(self) -> ApiRedirectResponse {
        let Self {
            param,
            json_payload,
        } = self;
        ApiRedirectResponse {
            param,
            json_payload,
        }
    }
}

impl ApiModelToDieselModelConvertor<ApiRecurringPaymentIntervalUnit>
    for RecurringPaymentIntervalUnit
{
    fn convert_from(from: ApiRecurringPaymentIntervalUnit) -> Self {
        match from {
            ApiRecurringPaymentIntervalUnit::Year => Self::Year,
            ApiRecurringPaymentIntervalUnit::Month => Self::Month,
            ApiRecurringPaymentIntervalUnit::Day => Self::Day,
            ApiRecurringPaymentIntervalUnit::Hour => Self::Hour,
            ApiRecurringPaymentIntervalUnit::Minute => Self::Minute,
        }
    }
    fn convert_back(self) -> ApiRecurringPaymentIntervalUnit {
        match self {
            Self::Year => ApiRecurringPaymentIntervalUnit::Year,
            Self::Month => ApiRecurringPaymentIntervalUnit::Month,
            Self::Day => ApiRecurringPaymentIntervalUnit::Day,
            Self::Hour => ApiRecurringPaymentIntervalUnit::Hour,
            Self::Minute => ApiRecurringPaymentIntervalUnit::Minute,
        }
    }
}

impl ApiModelToDieselModelConvertor<ApiApplePayRegularBillingDetails>
    for ApplePayRegularBillingDetails
{
    fn convert_from(from: ApiApplePayRegularBillingDetails) -> Self {
        Self {
            label: from.label,
            recurring_payment_start_date: from.recurring_payment_start_date,
            recurring_payment_end_date: from.recurring_payment_end_date,
            recurring_payment_interval_unit: from
                .recurring_payment_interval_unit
                .map(RecurringPaymentIntervalUnit::convert_from),
            recurring_payment_interval_count: from.recurring_payment_interval_count,
        }
    }

    fn convert_back(self) -> ApiApplePayRegularBillingDetails {
        ApiApplePayRegularBillingDetails {
            label: self.label,
            recurring_payment_start_date: self.recurring_payment_start_date,
            recurring_payment_end_date: self.recurring_payment_end_date,
            recurring_payment_interval_unit: self
                .recurring_payment_interval_unit
                .map(|value| value.convert_back()),
            recurring_payment_interval_count: self.recurring_payment_interval_count,
        }
    }
}

impl ApiModelToDieselModelConvertor<ApiApplePayRecurringDetails> for ApplePayRecurringDetails {
    fn convert_from(from: ApiApplePayRecurringDetails) -> Self {
        Self {
            payment_description: from.payment_description,
            regular_billing: ApplePayRegularBillingDetails::convert_from(from.regular_billing),
            billing_agreement: from.billing_agreement,
            management_url: from.management_url,
        }
    }

    fn convert_back(self) -> ApiApplePayRecurringDetails {
        ApiApplePayRecurringDetails {
            payment_description: self.payment_description,
            regular_billing: self.regular_billing.convert_back(),
            billing_agreement: self.billing_agreement,
            management_url: self.management_url,
        }
    }
}

#[cfg(feature = "v2")]
impl ApiModelToDieselModelConvertor<ApiRevenueRecoveryMetadata> for PaymentRevenueRecoveryMetadata {
    fn convert_from(from: ApiRevenueRecoveryMetadata) -> Self {
        Self {
            total_retry_count: from.total_retry_count,
            payment_connector_transmission: from.payment_connector_transmission,
            billing_connector_id: from.billing_connector_id,
            active_attempt_payment_connector_id: from.active_attempt_payment_connector_id,
            billing_connector_payment_details: BillingConnectorPaymentDetails::convert_from(
                from.billing_connector_payment_details,
            ),
            payment_method_type: from.payment_method_type,
            payment_method_subtype: from.payment_method_subtype,
        }
    }

    fn convert_back(self) -> ApiRevenueRecoveryMetadata {
        ApiRevenueRecoveryMetadata {
            total_retry_count: self.total_retry_count,
            payment_connector_transmission: self.payment_connector_transmission,
            billing_connector_id: self.billing_connector_id,
            active_attempt_payment_connector_id: self.active_attempt_payment_connector_id,
            billing_connector_payment_details: self
                .billing_connector_payment_details
                .convert_back(),
            payment_method_type: self.payment_method_type,
            payment_method_subtype: self.payment_method_subtype,
        }
    }
}

#[cfg(feature = "v2")]
impl ApiModelToDieselModelConvertor<ApiBillingConnectorPaymentDetails>
    for BillingConnectorPaymentDetails
{
    fn convert_from(from: ApiBillingConnectorPaymentDetails) -> Self {
        Self {
            payment_processor_token: from.payment_processor_token,
            connector_customer_id: from.connector_customer_id,
        }
    }

    fn convert_back(self) -> ApiBillingConnectorPaymentDetails {
        ApiBillingConnectorPaymentDetails {
            payment_processor_token: self.payment_processor_token,
            connector_customer_id: self.connector_customer_id,
        }
    }
}

impl ApiModelToDieselModelConvertor<ApiOrderDetailsWithAmount> for OrderDetailsWithAmount {
    fn convert_from(from: ApiOrderDetailsWithAmount) -> Self {
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
            tax_rate,
            total_tax_amount,
        } = from;
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
            tax_rate,
            total_tax_amount,
        }
    }

    fn convert_back(self) -> ApiOrderDetailsWithAmount {
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
            tax_rate,
            total_tax_amount,
        } = self;
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
            tax_rate,
            total_tax_amount,
        }
    }
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
            hide_card_nickname_field: item.hide_card_nickname_field,
            show_card_form_by_default: item.show_card_form_by_default,
            details_layout: item.details_layout,
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
            background_image: item.background_image.map(|background_image| {
                diesel_models::business_profile::PaymentLinkBackgroundImageConfig::convert_from(
                    background_image,
                )
            }),
            payment_button_text: item.payment_button_text,
            custom_message_for_card_terms: item.custom_message_for_card_terms,
            payment_button_colour: item.payment_button_colour,
        }
    }
    fn convert_back(self) -> api_models::admin::PaymentLinkConfigRequest {
        let Self {
            theme,
            logo,
            seller_name,
            sdk_layout,
            display_sdk_only,
            enabled_saved_payment_method,
            hide_card_nickname_field,
            show_card_form_by_default,
            transaction_details,
            background_image,
            details_layout,
            payment_button_text,
            custom_message_for_card_terms,
            payment_button_colour,
        } = self;
        api_models::admin::PaymentLinkConfigRequest {
            theme,
            logo,
            seller_name,
            sdk_layout,
            display_sdk_only,
            enabled_saved_payment_method,
            hide_card_nickname_field,
            show_card_form_by_default,
            details_layout,
            transaction_details: transaction_details.map(|transaction_details| {
                transaction_details
                    .into_iter()
                    .map(|transaction_detail| transaction_detail.convert_back())
                    .collect()
            }),
            background_image: background_image
                .map(|background_image| background_image.convert_back()),
            payment_button_text,
            custom_message_for_card_terms,
            payment_button_colour,
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
    fn convert_back(self) -> api_models::admin::PaymentLinkTransactionDetails {
        let Self {
            key,
            value,
            ui_configuration,
        } = self;
        api_models::admin::PaymentLinkTransactionDetails {
            key,
            value,
            ui_configuration: ui_configuration
                .map(|ui_configuration| ui_configuration.convert_back()),
        }
    }
}

#[cfg(feature = "v2")]
impl ApiModelToDieselModelConvertor<api_models::admin::PaymentLinkBackgroundImageConfig>
    for diesel_models::business_profile::PaymentLinkBackgroundImageConfig
{
    fn convert_from(from: api_models::admin::PaymentLinkBackgroundImageConfig) -> Self {
        Self {
            url: from.url,
            position: from.position,
            size: from.size,
        }
    }
    fn convert_back(self) -> api_models::admin::PaymentLinkBackgroundImageConfig {
        let Self {
            url,
            position,
            size,
        } = self;
        api_models::admin::PaymentLinkBackgroundImageConfig {
            url,
            position,
            size,
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
    fn convert_back(self) -> api_models::admin::TransactionDetailsUiConfiguration {
        let Self {
            position,
            is_key_bold,
            is_value_bold,
        } = self;
        api_models::admin::TransactionDetailsUiConfiguration {
            position,
            is_key_bold,
            is_value_bold,
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
            skip_external_tax_calculation: amount_details.skip_external_tax_calculation(),
            skip_surcharge_calculation: amount_details.skip_surcharge_calculation(),
            surcharge_amount: amount_details.surcharge_amount(),
            tax_on_surcharge: amount_details.tax_on_surcharge(),
            // We will not receive this in the request. This will be populated after calling the connector / processor
            amount_captured: None,
        }
    }
}
