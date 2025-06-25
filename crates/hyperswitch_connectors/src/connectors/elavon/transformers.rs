use cards::CardNumber;
use common_enums::{enums, Currency};
use common_utils::{pii::Email, types::StringMajorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::router_data::AdditionalPaymentMethodConnectorResponse;
use hyperswitch_domain_models::types::OrderDetailsWithAmount;
use hyperswitch_domain_models::{
    payment_method_data::{PaymentMethodData, WalletData},
    router_data::{ConnectorAuthType, ConnectorResponseData, ErrorResponse, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{PaymentsAuthorizeData, ResponseId},
    router_response_types::{MandateReference, PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, PaymentsSyncRouterData,
        RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{consts::NO_ERROR_CODE, errors};
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    types::{
        PaymentsCaptureResponseRouterData, PaymentsSyncResponseRouterData,
        RefundsResponseRouterData, ResponseRouterData,
    },
    utils::{
        CardData, PaymentsAuthorizeRequestData, RefundsRequestData, RouterData as _,
        WalletData as OtherWalletData,
    },
};

pub struct ElavonRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for ElavonRouterData<T> {
    fn from((amount, item): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    CcSale,
    CcAuthOnly,
    CcComplete,
    CcReturn,
    TxnQuery,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum SyncTransactionType {
    Sale,
    AuthOnly,
    Return,
}

// Enhanced CVV indicator enum
#[derive(Serialize, Deserialize, Debug)]
pub enum CvvIndicator {
    #[serde(rename = "0")]
    Bypassed,
    #[serde(rename = "1")]
    Present,
    #[serde(rename = "2")]
    Illegible,
    #[serde(rename = "9")]
    NotPresent,
}

impl Default for CvvIndicator {
    fn default() -> Self {
        Self::Present
    }
}

// Commerce indicator for different transaction types
#[derive(Serialize, Deserialize, Debug)]
pub enum CommerceIndicator {
    #[serde(rename = "internet")]
    Internet,
    #[serde(rename = "moto")]
    Moto,
    #[serde(rename = "recurring")]
    Recurring,
    #[serde(rename = "installment")]
    Installment,
}

impl Default for CommerceIndicator {
    fn default() -> Self {
        Self::Internet
    }
}

#[derive(Debug, Serialize)]
pub struct ApplePayPaymentRequest {
    pub ssl_transaction_type: TransactionType,
    pub ssl_account_id: Secret<String>,
    pub ssl_user_id: Secret<String>,
    pub ssl_pin: Secret<String>,
    pub ssl_amount: StringMajorUnit,
    pub ssl_email: Email,
    pub ssl_applepay_web: Secret<String>,
    pub ssl_transaction_currency: Currency,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_invoice_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_avs_zip: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_avs_address: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_first_name: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_last_name: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_company: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_city: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_state: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_country: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_phone: Option<Secret<String>>,
    // Additional metadata fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_merchant_txn_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GooglePayPaymentRequest {
    pub ssl_transaction_type: TransactionType,
    pub ssl_account_id: Secret<String>,
    pub ssl_user_id: Secret<String>,
    pub ssl_pin: Secret<String>,
    pub ssl_amount: StringMajorUnit,
    pub ssl_email: Email,
    pub ssl_google_pay: Secret<String>,
    pub ssl_transaction_currency: Currency,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_transaction_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_invoice_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_avs_zip: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_avs_address: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_first_name: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_last_name: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_company: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_city: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_state: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_country: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_phone: Option<Secret<String>>,
    // Additional metadata fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_merchant_txn_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ElavonPaymentsRequest {
    Card(CardPaymentRequest),
    MandatePayment(MandatePaymentRequest),
    ApplePay(ApplePayPaymentRequest),
    GooglePay(GooglePayPaymentRequest),
}

#[derive(Debug, Serialize)]
pub struct CardPaymentRequest {
    pub ssl_transaction_type: TransactionType,
    pub ssl_account_id: Secret<String>,
    pub ssl_user_id: Secret<String>,
    pub ssl_pin: Secret<String>,
    pub ssl_amount: StringMajorUnit,
    pub ssl_card_number: CardNumber,
    pub ssl_exp_date: Secret<String>,
    pub ssl_cvv2cvc2: Secret<String>,
    pub ssl_cvv2cvc2_indicator: CvvIndicator,
    pub ssl_email: Email,
    pub ssl_transaction_currency: Currency,
    pub ssl_transaction_source: Option<String>,

    // AVS fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_avs_zip: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_avs_address: Option<Secret<String>>,

    // Token generation fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_add_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_get_token: Option<String>,

    // Invoice and transaction identification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_invoice_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_merchant_txn_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_description: Option<String>,

    // Partial authorization
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_partial_auth_indicator: Option<u8>,

    // Customer information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_first_name: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_last_name: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_company: Option<Secret<String>>,

    // Billing address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_address2: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_city: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_state: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_country: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_phone: Option<Secret<String>>,

    // Shipping address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_ship_to_company: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_ship_to_first_name: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_ship_to_last_name: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_ship_to_address1: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_ship_to_address2: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_ship_to_city: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_ship_to_state: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_ship_to_zip: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_ship_to_country: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_ship_to_phone: Option<Secret<String>>,

    // Dynamic DBA fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_dynamic_dba: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_dynamic_dba_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_dynamic_dba_city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_dynamic_dba_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_dynamic_dba_postal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_dynamic_dba_country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_dynamic_dba_phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_dynamic_dba_email: Option<String>,

    // Level 2/3 and purchasing card fields - KEEP AS INTEGERS
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_customer_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_salestax: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_salestax_indicator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_level3_indicator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_shipping_amount: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_discount_amount: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_duty_amount: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_national_tax_indicator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_national_tax_amount: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_order_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_other_tax: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_summary_commodity_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_merchant_vat_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_customer_vat_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_freight_tax_amount: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_vat_invoice_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_tracking_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_shipping_company: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_other_fees: Option<i64>,

    // Line item fields (for Level 3) - KEEP AS INTEGERS
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_line_item_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_line_item_product_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_line_item_commodity_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_line_item_quantity: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_line_item_unit_of_measure: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_line_item_unit_cost: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_line_item_discount_indicator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_line_item_tax_indicator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_line_item_discount_amount: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_line_item_tax_rate: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_line_item_tax_amount: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_line_item_tax_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_line_item_extended_total: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_line_item_total: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_line_item_alternative_tax: Option<i64>,

    // Healthcare fields - KEEP AS INTEGERS
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_healthcare_amount: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_otc_amount: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_prescription_amount: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_clinic_other_amount: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_dental_amount: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_vision_amount: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_transit_amount: Option<i64>,

    // Travel data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_departure_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_completion_date: Option<String>,

    // MOTO indicator
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_terminal_type: Option<String>,

    // CIT/MIT indicators for recurring
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_cit_mit_cof_indicator: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MandatePaymentRequest {
    pub ssl_transaction_type: TransactionType,
    pub ssl_account_id: Secret<String>,
    pub ssl_user_id: Secret<String>,
    pub ssl_pin: Secret<String>,
    pub ssl_amount: StringMajorUnit,
    pub ssl_email: Email,
    pub ssl_token: Secret<String>,
    pub ssl_transaction_currency: Currency,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_invoice_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_merchant_txn_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_description: Option<String>,
    // CIT/MIT indicators for recurring
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_cit_mit_cof_indicator: Option<String>,
}

// Helper function to determine CIT/MIT indicator based on transaction context
fn get_cit_mit_indicator(item: &ElavonRouterData<&PaymentsAuthorizeRouterData>) -> Option<String> {
    // Check if this is a recurring/mandate payment
    if item.router_data.request.mandate_id.is_some() {
        // This is an MIT (Merchant Initiated Transaction)
        return Some("M01".to_string()); // Merchant Initiated Unscheduled
    }

    // Check setup_future_usage to determine CIT type
    match item.router_data.request.setup_future_usage {
        Some(common_enums::FutureUsage::OffSession) => {
            // Customer initiated but stored for future use
            Some("C01".to_string()) // Consumer Initiated Ad-hoc
        }
        Some(common_enums::FutureUsage::OnSession) => {
            // One-time payment
            Some("000".to_string()) // Not a CIT/MIT transaction
        }
        None => {
            // Regular one-time transaction
            Some("000".to_string())
        }
    }
}

// Helper function to determine terminal type based on channel
fn get_terminal_type(channel: Option<&str>) -> Option<String> {
    match channel {
        Some("moto") => Some("00".to_string()), // Attended Terminal for MOTO
        Some("ecommerce") => Some("04".to_string()), // No Terminal Used (ecommerce)
        _ => Some("04".to_string()),            // Default to ecommerce
    }
}

// Helper function to extract Level 2/3 data from metadata - KEEP AS INTEGERS
fn extract_level2_data(
    metadata: &Option<serde_json::Value>,
) -> (Option<String>, Option<String>, Option<i64>, Option<i64>) {
    if let Some(meta) = metadata {
        let ssl_salestax_indicator = meta
            .get("ssl_salestax_indicator")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let ssl_level3_indicator = meta
            .get("ssl_level3_indicator")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let ssl_shipping_amount = meta.get("ssl_shipping_amount").and_then(|v| v.as_i64());

        let ssl_discount_amount = meta.get("ssl_discount_amount").and_then(|v| v.as_i64());

        (
            ssl_salestax_indicator,
            ssl_level3_indicator,
            ssl_shipping_amount,
            ssl_discount_amount,
        )
    } else {
        (None, None, None, None)
    }
}

// Helper function to extract line item data from order_details - KEEP AS INTEGERS
fn extract_line_item_data(
    order_details: &Option<Vec<OrderDetailsWithAmount>>,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<u16>,
    Option<i64>,
    Option<String>,
    Option<String>,
) {
    if let Some(items) = order_details {
        if let Some(first_item) = items.first() {
            return (
                Some(first_item.product_name.clone()),
                first_item.product_id.clone(),
                first_item.category.clone(),
                Some(first_item.quantity),
                Some(first_item.amount.get_amount_as_i64()),
                Some("Y".to_string()), // Default tax indicator
                Some("N".to_string()), // Default discount indicator
            );
        }
    }
    (None, None, None, None, None, None, None)
}

impl TryFrom<&ElavonRouterData<&PaymentsAuthorizeRouterData>> for ElavonPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ElavonRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth = ElavonAuthType::try_from(&item.router_data.connector_auth_type)?;
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let billing = item.router_data.get_optional_billing();
                let shipping = item.router_data.get_optional_shipping();

                // Extract Level 2/3 data - KEEP AS INTEGERS
                let (
                    ssl_salestax_indicator,
                    ssl_level3_indicator,
                    ssl_shipping_amount,
                    ssl_discount_amount,
                ) = extract_level2_data(&item.router_data.request.metadata);

                // Extract line item data - KEEP AS INTEGERS
                let (
                    line_item_description,
                    line_item_product_code,
                    line_item_commodity_code,
                    line_item_quantity,
                    line_item_unit_cost,
                    line_item_tax_indicator,
                    line_item_discount_indicator,
                ) = extract_line_item_data(&item.router_data.request.order_details);

                // Determine CVV indicator
                let ssl_cvv2cvc2_indicator = if req_card.card_cvc.peek().is_empty() {
                    CvvIndicator::NotPresent
                } else {
                    CvvIndicator::Present
                };

                Ok(Self::Card(CardPaymentRequest {
                    ssl_transaction_type: match item.router_data.request.is_auto_capture()? {
                        true => TransactionType::CcSale,
                        false => TransactionType::CcAuthOnly,
                    },
                    ssl_account_id: auth.account_id.clone(),
                    ssl_user_id: auth.user_id.clone(),
                    ssl_pin: auth.pin.clone(),
                    ssl_transaction_source: Some("MOTO".to_string()),
                    ssl_amount: item.amount.clone(),
                    ssl_card_number: req_card.card_number.clone(),
                    ssl_exp_date: req_card.get_expiry_date_as_mmyy()?,
                    ssl_cvv2cvc2: req_card.card_cvc.clone(),
                    ssl_cvv2cvc2_indicator,
                    ssl_email: item.router_data.get_billing_email()?,
                    ssl_transaction_currency: item.router_data.request.currency,

                    // AVS fields
                    ssl_avs_zip: billing
                        .and_then(|b| b.address.as_ref())
                        .and_then(|addr| addr.zip.clone()),
                    ssl_avs_address: billing
                        .and_then(|b| b.address.as_ref())
                        .and_then(|addr| addr.line1.clone()),

                    // Token generation
                    ssl_add_token: match item.router_data.request.is_mandate_payment() {
                        true => Some("Y".to_string()),
                        false => None,
                    },
                    ssl_get_token: match item.router_data.request.is_mandate_payment() {
                        true => Some("Y".to_string()),
                        false => None,
                    },

                    // Transaction identification
                    ssl_invoice_number: Some(item.router_data.payment_id.clone()),
                    ssl_merchant_txn_id: Some(
                        item.router_data.connector_request_reference_id.clone(),
                    ),
                    ssl_description: None,

                    // Partial authorization (default to not supported)
                    ssl_partial_auth_indicator: Some(0),

                    // Customer information
                    ssl_first_name: billing
                        .and_then(|b| b.address.as_ref())
                        .and_then(|addr| addr.first_name.clone()),
                    ssl_last_name: billing
                        .and_then(|b| b.address.as_ref())
                        .and_then(|addr| addr.last_name.clone()),
                    ssl_company: None, // company field not available in AddressDetails

                    // Billing address
                    ssl_address2: billing
                        .and_then(|b| b.address.as_ref())
                        .and_then(|addr| addr.line2.clone()),
                    ssl_city: billing
                        .and_then(|b| b.address.as_ref())
                        .and_then(|addr| addr.city.clone())
                        .map(|city| Secret::new(city)),
                    ssl_state: billing
                        .and_then(|b| b.address.as_ref())
                        .and_then(|addr| addr.state.clone()),
                    ssl_country: billing
                        .and_then(|b| b.address.as_ref())
                        .and_then(|addr| addr.country.map(|c| Secret::new(c.to_string()))),
                    ssl_phone: billing
                        .and_then(|b| b.phone.as_ref())
                        .and_then(|phone| phone.number.clone()),

                    // Shipping address
                    ssl_ship_to_company: None, // company field not available in AddressDetails
                    ssl_ship_to_first_name: shipping
                        .and_then(|s| s.address.as_ref())
                        .and_then(|addr| addr.first_name.clone()),
                    ssl_ship_to_last_name: shipping
                        .and_then(|s| s.address.as_ref())
                        .and_then(|addr| addr.last_name.clone()),
                    ssl_ship_to_address1: shipping
                        .and_then(|s| s.address.as_ref())
                        .and_then(|addr| addr.line1.clone()),
                    ssl_ship_to_address2: shipping
                        .and_then(|s| s.address.as_ref())
                        .and_then(|addr| addr.line2.clone()),
                    ssl_ship_to_city: shipping
                        .and_then(|s| s.address.as_ref())
                        .and_then(|addr| addr.city.clone())
                        .map(|city| Secret::new(city)),
                    ssl_ship_to_state: shipping
                        .and_then(|s| s.address.as_ref())
                        .and_then(|addr| addr.state.clone()),
                    ssl_ship_to_zip: shipping
                        .and_then(|s| s.address.as_ref())
                        .and_then(|addr| addr.zip.clone()),
                    ssl_ship_to_country: shipping
                        .and_then(|s| s.address.as_ref())
                        .and_then(|addr| addr.country.map(|c| Secret::new(c.to_string()))),
                    ssl_ship_to_phone: shipping
                        .and_then(|s| s.phone.as_ref())
                        .and_then(|phone| phone.number.clone()),

                    // Dynamic DBA fields (can be set via metadata)
                    ssl_dynamic_dba: item
                        .router_data
                        .request
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ssl_dynamic_dba"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    ssl_dynamic_dba_address: None,
                    ssl_dynamic_dba_city: None,
                    ssl_dynamic_dba_state: None,
                    ssl_dynamic_dba_postal: None,
                    ssl_dynamic_dba_country: None,
                    ssl_dynamic_dba_phone: None,
                    ssl_dynamic_dba_email: None,

                    // Level 2/3 purchasing card fields - KEEP AS INTEGERS
                    ssl_customer_code: item
                        .router_data
                        .request
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ssl_customer_code"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    ssl_salestax: item
                        .router_data
                        .request
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ssl_salestax"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    ssl_salestax_indicator,
                    ssl_level3_indicator,
                    ssl_shipping_amount,
                    ssl_discount_amount,
                    ssl_duty_amount: item
                        .router_data
                        .request
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ssl_duty_amount"))
                        .and_then(|v| v.as_i64()),
                    ssl_national_tax_indicator: item
                        .router_data
                        .request
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ssl_national_tax_indicator"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    ssl_national_tax_amount: item
                        .router_data
                        .request
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ssl_national_tax_amount"))
                        .and_then(|v| v.as_i64()),
                    ssl_order_date: item
                        .router_data
                        .request
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ssl_order_date"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    ssl_other_tax: item
                        .router_data
                        .request
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ssl_other_tax"))
                        .and_then(|v| v.as_i64()),
                    ssl_summary_commodity_code: item
                        .router_data
                        .request
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ssl_summary_commodity_code"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    ssl_merchant_vat_number: item
                        .router_data
                        .request
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ssl_merchant_vat_number"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    ssl_customer_vat_number: item
                        .router_data
                        .request
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ssl_customer_vat_number"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    ssl_freight_tax_amount: item
                        .router_data
                        .request
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ssl_freight_tax_amount"))
                        .and_then(|v| v.as_i64()),
                    ssl_vat_invoice_number: item
                        .router_data
                        .request
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ssl_vat_invoice_number"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    ssl_tracking_number: item
                        .router_data
                        .request
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ssl_tracking_number"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    ssl_shipping_company: item
                        .router_data
                        .request
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ssl_shipping_company"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    ssl_other_fees: item
                        .router_data
                        .request
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ssl_other_fees"))
                        .and_then(|v| v.as_i64()),

                    // Line item fields (Level 3) - KEEP AS INTEGERS
                    ssl_line_item_description: line_item_description,
                    ssl_line_item_product_code: line_item_product_code,
                    ssl_line_item_commodity_code: line_item_commodity_code,
                    ssl_line_item_quantity: line_item_quantity,
                    ssl_line_item_unit_of_measure: Some("EA".to_string()), // Default to "Each"
                    ssl_line_item_unit_cost: line_item_unit_cost,
                    ssl_line_item_discount_indicator: line_item_discount_indicator,
                    ssl_line_item_tax_indicator: line_item_tax_indicator,
                    ssl_line_item_discount_amount: item
                        .router_data
                        .request
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ssl_line_item_discount_amount"))
                        .and_then(|v| v.as_i64()),
                    ssl_line_item_tax_rate: item
                        .router_data
                        .request
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ssl_line_item_tax_rate"))
                        .and_then(|v| v.as_i64())
                        .map(|rate| rate as u32),
                    ssl_line_item_tax_amount: item
                        .router_data
                        .request
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ssl_line_item_tax_amount"))
                        .and_then(|v| v.as_i64()),
                    ssl_line_item_tax_type: item
                        .router_data
                        .request
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ssl_line_item_tax_type"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    ssl_line_item_extended_total: line_item_unit_cost,
                    ssl_line_item_total: line_item_unit_cost,
                    ssl_line_item_alternative_tax: None,

                    // Healthcare fields - KEEP AS INTEGERS
                    ssl_healthcare_amount: item
                        .router_data
                        .request
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ssl_healthcare_amount"))
                        .and_then(|v| v.as_i64()),
                    ssl_otc_amount: item
                        .router_data
                        .request
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ssl_otc_amount"))
                        .and_then(|v| v.as_i64()),
                    ssl_prescription_amount: item
                        .router_data
                        .request
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ssl_prescription_amount"))
                        .and_then(|v| v.as_i64()),
                    ssl_clinic_other_amount: item
                        .router_data
                        .request
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ssl_clinic_other_amount"))
                        .and_then(|v| v.as_i64()),
                    ssl_dental_amount: item
                        .router_data
                        .request
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ssl_dental_amount"))
                        .and_then(|v| v.as_i64()),
                    ssl_vision_amount: item
                        .router_data
                        .request
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ssl_vision_amount"))
                        .and_then(|v| v.as_i64()),
                    ssl_transit_amount: item
                        .router_data
                        .request
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ssl_transit_amount"))
                        .and_then(|v| v.as_i64()),

                    // Travel data
                    ssl_departure_date: item
                        .router_data
                        .request
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ssl_departure_date"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    ssl_completion_date: item
                        .router_data
                        .request
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ssl_completion_date"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),

                    // MOTO/Terminal type
                    ssl_terminal_type: get_terminal_type(
                        item.router_data
                            .request
                            .metadata
                            .as_ref()
                            .and_then(|m| m.get("payment_channel"))
                            .and_then(|v| v.as_str()),
                    ),

                    // CIT/MIT indicator
                    ssl_cit_mit_cof_indicator: get_cit_mit_indicator(item),
                }))
            }
            PaymentMethodData::MandatePayment => Ok(Self::MandatePayment(MandatePaymentRequest {
                ssl_transaction_type: match item.router_data.request.is_auto_capture()? {
                    true => TransactionType::CcSale,
                    false => TransactionType::CcAuthOnly,
                },
                ssl_account_id: auth.account_id.clone(),
                ssl_user_id: auth.user_id.clone(),
                ssl_pin: auth.pin.clone(),
                ssl_amount: item.amount.clone(),
                ssl_email: item.router_data.get_billing_email()?,
                ssl_token: Secret::new(item.router_data.request.get_connector_mandate_id()?),
                ssl_transaction_currency: item.router_data.request.currency,
                ssl_invoice_number: Some(item.router_data.payment_id.clone()),
                ssl_merchant_txn_id: Some(item.router_data.connector_request_reference_id.clone()),
                ssl_description: None,
                ssl_cit_mit_cof_indicator: get_cit_mit_indicator(item),
            })),
            PaymentMethodData::Wallet(wallet_data) => match wallet_data {
                WalletData::ApplePay(ref _apple_pay_data) => {
                    let billing = item.router_data.get_optional_billing();
                    let _shipping = item.router_data.get_optional_shipping();

                    Ok(Self::ApplePay(ApplePayPaymentRequest {
                        ssl_transaction_type: TransactionType::CcSale,
                        ssl_account_id: auth.account_id.clone(),
                        ssl_user_id: auth.user_id.clone(),
                        ssl_pin: auth.pin.clone(),
                        ssl_amount: item.amount.clone(),
                        ssl_email: item.router_data.get_billing_email()?,
                        ssl_applepay_web: wallet_data
                            .get_wallet_token_as_json("Apple Pay".to_string())?,
                        ssl_transaction_currency: item.router_data.request.currency,
                        ssl_invoice_number: Some(item.router_data.payment_id.clone()),
                        ssl_avs_zip: billing
                            .and_then(|b| b.address.as_ref())
                            .and_then(|addr| addr.zip.clone()),
                        ssl_avs_address: billing
                            .and_then(|b| b.address.as_ref())
                            .and_then(|addr| addr.line1.clone()),
                        ssl_first_name: billing
                            .and_then(|b| b.address.as_ref())
                            .and_then(|addr| addr.first_name.clone()),
                        ssl_last_name: billing
                            .and_then(|b| b.address.as_ref())
                            .and_then(|addr| addr.last_name.clone()),
                        ssl_company: None, // company field not available in AddressDetails
                        ssl_city: billing
                            .and_then(|b| b.address.as_ref())
                            .and_then(|addr| addr.city.clone())
                            .map(|city| Secret::new(city)),
                        ssl_state: billing
                            .and_then(|b| b.address.as_ref())
                            .and_then(|addr| addr.state.clone()),
                        ssl_country: billing
                            .and_then(|b| b.address.as_ref())
                            .and_then(|addr| addr.country.map(|c| Secret::new(c.to_string()))),
                        ssl_phone: billing
                            .and_then(|b| b.phone.as_ref())
                            .and_then(|phone| phone.number.clone()),
                        ssl_description: None,
                        ssl_merchant_txn_id: Some(
                            item.router_data.connector_request_reference_id.clone(),
                        ),
                    }))
                }
                WalletData::GooglePay(ref _google_pay_data) => {
                    let billing = item.router_data.get_optional_billing();

                    Ok(Self::GooglePay(GooglePayPaymentRequest {
                        ssl_transaction_type: TransactionType::CcSale,
                        ssl_account_id: auth.account_id.clone(),
                        ssl_user_id: auth.user_id.clone(),
                        ssl_pin: auth.pin.clone(),
                        ssl_amount: item.amount.clone(),
                        ssl_email: item.router_data.get_billing_email()?,
                        ssl_google_pay: wallet_data
                            .get_wallet_token_as_json("Google Pay".to_string())?,
                        ssl_transaction_currency: item.router_data.request.currency,
                        ssl_transaction_source: Some("X_HPP".to_string()),
                        ssl_invoice_number: Some(item.router_data.payment_id.clone()),
                        ssl_avs_zip: billing
                            .and_then(|b| b.address.as_ref())
                            .and_then(|addr| addr.zip.clone()),
                        ssl_avs_address: billing
                            .and_then(|b| b.address.as_ref())
                            .and_then(|addr| addr.line1.clone()),
                        ssl_first_name: billing
                            .and_then(|b| b.address.as_ref())
                            .and_then(|addr| addr.first_name.clone()),
                        ssl_last_name: billing
                            .and_then(|b| b.address.as_ref())
                            .and_then(|addr| addr.last_name.clone()),
                        ssl_company: None, // company field not available in AddressDetails
                        ssl_city: billing
                            .and_then(|b| b.address.as_ref())
                            .and_then(|addr| addr.city.clone())
                            .map(|city| Secret::new(city)),
                        ssl_state: billing
                            .and_then(|b| b.address.as_ref())
                            .and_then(|addr| addr.state.clone()),
                        ssl_country: billing
                            .and_then(|b| b.address.as_ref())
                            .and_then(|addr| addr.country.map(|c| Secret::new(c.to_string()))),
                        ssl_phone: billing
                            .and_then(|b| b.phone.as_ref())
                            .and_then(|phone| phone.number.clone()),
                        ssl_description: None,
                        ssl_merchant_txn_id: Some(
                            item.router_data.connector_request_reference_id.clone(),
                        ),
                    }))
                }
                _ => Err(
                    errors::ConnectorError::NotImplemented("Payment methods".to_string()).into(),
                ),
            },
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

pub struct ElavonAuthType {
    pub(super) account_id: Secret<String>,
    pub(super) user_id: Secret<String>,
    pub(super) pin: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for ElavonAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                account_id: api_key.to_owned(),
                user_id: key1.to_owned(),
                pin: api_secret.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
enum SslResult {
    #[serde(rename = "0")]
    ImportedBatchFile,
    #[serde(other)]
    DeclineOrUnauthorized,
}

#[derive(Debug, Clone, Serialize)]
pub struct ElavonPaymentsResponse {
    pub result: ElavonResult,
}

#[derive(Debug, Clone, Serialize)]
pub enum ElavonResult {
    Success(PaymentResponse),
    Error(ElavonErrorResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElavonErrorResponse {
    error_code: Option<String>,
    error_message: String,
    error_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentResponse {
    ssl_result: SslResult,
    ssl_txn_id: String,
    ssl_result_message: String,
    ssl_token: Option<Secret<String>>,
    // Enhanced response fields for AVS and CVV
    ssl_avs_response: Option<String>,
    ssl_cvv2_response: Option<String>,
    ssl_approval_code: Option<String>,
    // Network response codes
    ssl_card_type: Option<String>,
    ssl_card_short_description: Option<String>,
    ssl_issuer_response: Option<String>,
    // Additional response fields
    ssl_network_response_code: Option<String>,
    ssl_network_advice_code: Option<String>,
}

impl<'de> Deserialize<'de> for ElavonPaymentsResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize, Debug)]
        #[serde(rename = "txn")]
        struct XmlResponse {
            // Error fields
            #[serde(rename = "errorCode", default)]
            error_code: Option<String>,
            #[serde(rename = "errorMessage", default)]
            error_message: Option<String>,
            #[serde(rename = "errorName", default)]
            error_name: Option<String>,

            // Success fields
            #[serde(rename = "ssl_result", default)]
            ssl_result: Option<SslResult>,
            #[serde(rename = "ssl_txn_id", default)]
            ssl_txn_id: Option<String>,
            #[serde(rename = "ssl_result_message", default)]
            ssl_result_message: Option<String>,
            #[serde(rename = "ssl_token", default)]
            ssl_token: Option<Secret<String>>,
            // Enhanced response fields
            #[serde(rename = "ssl_avs_response", default)]
            ssl_avs_response: Option<String>,
            #[serde(rename = "ssl_cvv2_response", default)]
            ssl_cvv2_response: Option<String>,
            #[serde(rename = "ssl_approval_code", default)]
            ssl_approval_code: Option<String>,
            #[serde(rename = "ssl_card_type", default)]
            ssl_card_type: Option<String>,
            #[serde(rename = "ssl_card_short_description", default)]
            ssl_card_short_description: Option<String>,
            #[serde(rename = "ssl_issuer_response", default)]
            ssl_issuer_response: Option<String>,
            #[serde(rename = "ssl_network_response_code", default)]
            ssl_network_response_code: Option<String>,
            #[serde(rename = "ssl_network_advice_code", default)]
            ssl_network_advice_code: Option<String>,
        }

        let xml_res = XmlResponse::deserialize(deserializer)?;

        let result = match (xml_res.error_message.clone(), xml_res.error_name.clone()) {
            (Some(error_message), Some(error_name)) => ElavonResult::Error(ElavonErrorResponse {
                error_code: xml_res.error_code.clone(),
                error_message,
                error_name,
            }),
            _ => {
                if let (Some(ssl_result), Some(ssl_txn_id), Some(ssl_result_message)) = (
                    xml_res.ssl_result.clone(),
                    xml_res.ssl_txn_id.clone(),
                    xml_res.ssl_result_message.clone(),
                ) {
                    ElavonResult::Success(PaymentResponse {
                        ssl_result,
                        ssl_txn_id,
                        ssl_result_message,
                        ssl_token: xml_res.ssl_token.clone(),
                        ssl_avs_response: xml_res.ssl_avs_response,
                        ssl_cvv2_response: xml_res.ssl_cvv2_response,
                        ssl_approval_code: xml_res.ssl_approval_code,
                        ssl_card_type: xml_res.ssl_card_type,
                        ssl_card_short_description: xml_res.ssl_card_short_description,
                        ssl_issuer_response: xml_res.ssl_issuer_response,
                        ssl_network_response_code: xml_res.ssl_network_response_code,
                        ssl_network_advice_code: xml_res.ssl_network_advice_code,
                    })
                } else {
                    return Err(serde::de::Error::custom(
                        "Invalid Response XML structure - neither error nor success",
                    ));
                }
            }
        };

        Ok(Self { result })
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<F, ElavonPaymentsResponse, PaymentsAuthorizeData, PaymentsResponseData>,
    > for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            ElavonPaymentsResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status =
            get_payment_status(&item.response.result, item.data.request.is_auto_capture()?);
        let connector_response = match &item.response.result {
            ElavonResult::Success(response) => {
                Some(ConnectorResponseData::with_additional_payment_method_data(
                    convert_to_additional_payment_method_connector_response(response),
                ))
            }
            ElavonResult::Error(_) => None,
        };
        let response = match &item.response.result {
            ElavonResult::Error(error) => Err(ErrorResponse {
                code: error
                    .error_code
                    .clone()
                    .unwrap_or(NO_ERROR_CODE.to_string()),
                message: error.error_message.clone(),
                reason: Some(error.error_message.clone()),
                attempt_status: None,
                connector_transaction_id: None,
                status_code: item.http_code,
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
            }),
            ElavonResult::Success(response) => {
                if status == enums::AttemptStatus::Failure {
                    Err(ErrorResponse {
                        code: response.ssl_result_message.clone(),
                        message: response.ssl_result_message.clone(),
                        reason: Some(response.ssl_result_message.clone()),
                        attempt_status: None,
                        connector_transaction_id: Some(response.ssl_txn_id.clone()),
                        status_code: item.http_code,
                        network_advice_code: response.ssl_network_advice_code.clone(),
                        network_decline_code: response.ssl_network_response_code.clone(),
                        network_error_message: response.ssl_issuer_response.clone(),
                    })
                } else {
                    Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(
                            response.ssl_txn_id.clone(),
                        ),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(Some(MandateReference {
                            connector_mandate_id: response
                                .ssl_token
                                .as_ref()
                                .map(|secret| secret.clone().expose()),
                            payment_method_id: None,
                            mandate_metadata: None,
                            connector_mandate_request_reference_id: None,
                        })),
                        connector_metadata: Some(serde_json::json!({
                            "ssl_avs_response": response.ssl_avs_response,
                            "ssl_cvv2_response": response.ssl_cvv2_response,
                            "ssl_approval_code": response.ssl_approval_code,
                            "ssl_card_type": response.ssl_card_type,
                            "ssl_card_short_description": response.ssl_card_short_description
                        })),
                        network_txn_id: None,
                        connector_response_reference_id: Some(response.ssl_txn_id.clone()),
                        incremental_authorization_allowed: None,
                        charges: None,
                    })
                }
            }
        };
        Ok(Self {
            status,
            response,
            connector_response,
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TransactionSyncStatus {
    PEN, // Pended
    OPN, // Unpended / release / open
    REV, // Review
    STL, // Settled
    PST, // Failed due to post-auth rule
    FPR, // Failed due to fraud prevention rules
    PRE, // Failed due to pre-auth rule
}

#[derive(Debug, Serialize)]
#[serde(rename = "txn")]
pub struct PaymentsCaptureRequest {
    pub ssl_transaction_type: TransactionType,
    pub ssl_account_id: Secret<String>,
    pub ssl_user_id: Secret<String>,
    pub ssl_pin: Secret<String>,
    pub ssl_amount: StringMajorUnit,
    pub ssl_txn_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_invoice_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_description: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename = "txn")]
pub struct PaymentsVoidRequest {
    pub ssl_transaction_type: TransactionType,
    pub ssl_account_id: Secret<String>,
    pub ssl_user_id: Secret<String>,
    pub ssl_pin: Secret<String>,
    pub ssl_txn_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_description: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename = "txn")]
pub struct ElavonRefundRequest {
    pub ssl_transaction_type: TransactionType,
    pub ssl_account_id: Secret<String>,
    pub ssl_user_id: Secret<String>,
    pub ssl_pin: Secret<String>,
    pub ssl_amount: StringMajorUnit,
    pub ssl_txn_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_invoice_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_description: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename = "txn")]
pub struct SyncRequest {
    pub ssl_transaction_type: TransactionType,
    pub ssl_account_id: Secret<String>,
    pub ssl_user_id: Secret<String>,
    pub ssl_pin: Secret<String>,
    pub ssl_txn_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "txn")]
pub struct ElavonSyncResponse {
    pub ssl_trans_status: TransactionSyncStatus,
    pub ssl_transaction_type: SyncTransactionType,
    pub ssl_txn_id: String,
    // Enhanced sync response fields
    pub ssl_result_message: Option<String>,
    pub ssl_approval_code: Option<String>,
    pub ssl_avs_response: Option<String>,
    pub ssl_cvv2_response: Option<String>,
    pub ssl_card_type: Option<String>,
    pub ssl_network_response_code: Option<String>,
    pub ssl_network_advice_code: Option<String>,
}

impl TryFrom<&RefundSyncRouterData> for SyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RefundSyncRouterData) -> Result<Self, Self::Error> {
        let auth = ElavonAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            ssl_txn_id: item.request.get_connector_refund_id()?,
            ssl_transaction_type: TransactionType::TxnQuery,
            ssl_account_id: auth.account_id.clone(),
            ssl_user_id: auth.user_id.clone(),
            ssl_pin: auth.pin.clone(),
        })
    }
}

impl TryFrom<&PaymentsSyncRouterData> for SyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let auth = ElavonAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            ssl_txn_id: item
                .request
                .connector_transaction_id
                .get_connector_transaction_id()
                .change_context(errors::ConnectorError::MissingConnectorTransactionID)?,
            ssl_transaction_type: TransactionType::TxnQuery,
            ssl_account_id: auth.account_id.clone(),
            ssl_user_id: auth.user_id.clone(),
            ssl_pin: auth.pin.clone(),
        })
    }
}

impl<F> TryFrom<&ElavonRouterData<&RefundsRouterData<F>>> for ElavonRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &ElavonRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let auth = ElavonAuthType::try_from(&item.router_data.connector_auth_type)?;
        Ok(Self {
            ssl_txn_id: item.router_data.request.connector_transaction_id.clone(),
            ssl_amount: item.amount.clone(),
            ssl_transaction_type: TransactionType::CcReturn,
            ssl_account_id: auth.account_id.clone(),
            ssl_user_id: auth.user_id.clone(),
            ssl_pin: auth.pin.clone(),
            ssl_invoice_number: Some(item.router_data.request.refund_id.clone()),
            ssl_description: None,
        })
    }
}

impl TryFrom<&ElavonRouterData<&PaymentsCaptureRouterData>> for PaymentsCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &ElavonRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
        let auth = ElavonAuthType::try_from(&item.router_data.connector_auth_type)?;
        Ok(Self {
            ssl_txn_id: item.router_data.request.connector_transaction_id.clone(),
            ssl_amount: item.amount.clone(),
            ssl_transaction_type: TransactionType::CcComplete,
            ssl_account_id: auth.account_id.clone(),
            ssl_user_id: auth.user_id.clone(),
            ssl_pin: auth.pin.clone(),
            ssl_invoice_number: Some(item.router_data.payment_id.clone()),
            ssl_description: None, // PaymentsCaptureData doesn't have statement_descriptor
        })
    }
}

impl TryFrom<PaymentsSyncResponseRouterData<ElavonSyncResponse>> for PaymentsSyncRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsSyncResponseRouterData<ElavonSyncResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: get_sync_status(item.data.status, &item.response),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.ssl_txn_id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: Some(serde_json::json!({
                    "ssl_result_message": item.response.ssl_result_message,
                    "ssl_approval_code": item.response.ssl_approval_code,
                    "ssl_avs_response": item.response.ssl_avs_response,
                    "ssl_cvv2_response": item.response.ssl_cvv2_response,
                    "ssl_card_type": item.response.ssl_card_type,
                    "ssl_network_response_code": item.response.ssl_network_response_code,
                    "ssl_network_advice_code": item.response.ssl_network_advice_code
                })),
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.ssl_txn_id),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, ElavonSyncResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, ElavonSyncResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.ssl_txn_id.clone(),
                refund_status: get_refund_status(item.data.request.refund_status, &item.response),
            }),
            ..item.data
        })
    }
}

impl TryFrom<PaymentsCaptureResponseRouterData<ElavonPaymentsResponse>>
    for PaymentsCaptureRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsCaptureResponseRouterData<ElavonPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        let status = map_payment_status(&item.response.result, enums::AttemptStatus::Charged);
        let response = match &item.response.result {
            ElavonResult::Error(error) => Err(ErrorResponse {
                code: error
                    .error_code
                    .clone()
                    .unwrap_or(NO_ERROR_CODE.to_string()),
                message: error.error_message.clone(),
                reason: Some(error.error_message.clone()),
                attempt_status: None,
                connector_transaction_id: None,
                status_code: item.http_code,
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
            }),
            ElavonResult::Success(response) => {
                if status == enums::AttemptStatus::Failure {
                    Err(ErrorResponse {
                        code: response.ssl_result_message.clone(),
                        message: response.ssl_result_message.clone(),
                        reason: Some(response.ssl_result_message.clone()),
                        attempt_status: None,
                        connector_transaction_id: None,
                        status_code: item.http_code,
                        network_advice_code: response.ssl_network_advice_code.clone(),
                        network_decline_code: response.ssl_network_response_code.clone(),
                        network_error_message: response.ssl_issuer_response.clone(),
                    })
                } else {
                    Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(
                            response.ssl_txn_id.clone(),
                        ),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata: Some(serde_json::json!({
                            "ssl_avs_response": response.ssl_avs_response,
                            "ssl_cvv2_response": response.ssl_cvv2_response,
                            "ssl_approval_code": response.ssl_approval_code,
                            "ssl_card_type": response.ssl_card_type,
                            "ssl_card_short_description": response.ssl_card_short_description
                        })),
                        network_txn_id: None,
                        connector_response_reference_id: Some(response.ssl_txn_id.clone()),
                        incremental_authorization_allowed: None,
                        charges: None,
                    })
                }
            }
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, ElavonPaymentsResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, ElavonPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        let status = enums::RefundStatus::from(&item.response.result);
        let response = match &item.response.result {
            ElavonResult::Error(error) => Err(ErrorResponse {
                code: error
                    .error_code
                    .clone()
                    .unwrap_or(NO_ERROR_CODE.to_string()),
                message: error.error_message.clone(),
                reason: Some(error.error_message.clone()),
                attempt_status: None,
                connector_transaction_id: None,
                status_code: item.http_code,
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
            }),
            ElavonResult::Success(response) => {
                if status == enums::RefundStatus::Failure {
                    Err(ErrorResponse {
                        code: response.ssl_result_message.clone(),
                        message: response.ssl_result_message.clone(),
                        reason: Some(response.ssl_result_message.clone()),
                        attempt_status: None,
                        connector_transaction_id: None,
                        status_code: item.http_code,
                        network_advice_code: response.ssl_network_advice_code.clone(),
                        network_decline_code: response.ssl_network_response_code.clone(),
                        network_error_message: response.ssl_issuer_response.clone(),
                    })
                } else {
                    Ok(RefundsResponseData {
                        connector_refund_id: response.ssl_txn_id.clone(),
                        refund_status: enums::RefundStatus::from(&item.response.result),
                    })
                }
            }
        };
        Ok(Self {
            response,
            ..item.data
        })
    }
}

trait ElavonResponseValidator {
    fn is_successful(&self) -> bool;
}

impl ElavonResponseValidator for ElavonResult {
    fn is_successful(&self) -> bool {
        matches!(self, Self::Success(response) if response.ssl_result == SslResult::ImportedBatchFile)
    }
}

fn map_payment_status(
    item: &ElavonResult,
    success_status: enums::AttemptStatus,
) -> enums::AttemptStatus {
    if item.is_successful() {
        success_status
    } else {
        enums::AttemptStatus::Failure
    }
}

impl From<&ElavonResult> for enums::RefundStatus {
    fn from(item: &ElavonResult) -> Self {
        if item.is_successful() {
            Self::Success
        } else {
            Self::Failure
        }
    }
}

fn get_refund_status(
    prev_status: enums::RefundStatus,
    item: &ElavonSyncResponse,
) -> enums::RefundStatus {
    match item.ssl_trans_status {
        TransactionSyncStatus::REV | TransactionSyncStatus::OPN | TransactionSyncStatus::PEN => {
            prev_status
        }
        TransactionSyncStatus::STL => enums::RefundStatus::Success,
        TransactionSyncStatus::PST | TransactionSyncStatus::FPR | TransactionSyncStatus::PRE => {
            enums::RefundStatus::Failure
        }
    }
}

impl From<&ElavonSyncResponse> for enums::AttemptStatus {
    fn from(item: &ElavonSyncResponse) -> Self {
        match item.ssl_trans_status {
            TransactionSyncStatus::REV
            | TransactionSyncStatus::OPN
            | TransactionSyncStatus::PEN => Self::Pending,
            TransactionSyncStatus::STL => match item.ssl_transaction_type {
                SyncTransactionType::Sale => Self::Charged,
                SyncTransactionType::AuthOnly => Self::Authorized,
                SyncTransactionType::Return => Self::Pending,
            },
            TransactionSyncStatus::PST
            | TransactionSyncStatus::FPR
            | TransactionSyncStatus::PRE => Self::Failure,
        }
    }
}

fn get_sync_status(
    prev_status: enums::AttemptStatus,
    item: &ElavonSyncResponse,
) -> enums::AttemptStatus {
    match item.ssl_trans_status {
        TransactionSyncStatus::REV | TransactionSyncStatus::OPN | TransactionSyncStatus::PEN => {
            prev_status
        }
        TransactionSyncStatus::STL => match item.ssl_transaction_type {
            SyncTransactionType::Sale => enums::AttemptStatus::Charged,
            SyncTransactionType::AuthOnly => enums::AttemptStatus::Authorized,
            SyncTransactionType::Return => enums::AttemptStatus::Pending,
        },
        TransactionSyncStatus::PST | TransactionSyncStatus::FPR | TransactionSyncStatus::PRE => {
            enums::AttemptStatus::Failure
        }
    }
}

fn get_payment_status(item: &ElavonResult, is_auto_capture: bool) -> enums::AttemptStatus {
    if item.is_successful() {
        if is_auto_capture {
            enums::AttemptStatus::Charged
        } else {
            enums::AttemptStatus::Authorized
        }
    } else {
        enums::AttemptStatus::Failure
    }
}

fn convert_to_additional_payment_method_connector_response(
    response: &PaymentResponse,
) -> AdditionalPaymentMethodConnectorResponse {
    let payment_checks = Some(serde_json::json!({
        "avs_response": {
            "code": response.ssl_avs_response,
            "code_raw": response.ssl_avs_response
        },
        "card_verification": {
            "result_code": response.ssl_cvv2_response,
            "result_code_raw": response.ssl_cvv2_response
        }
    }));

    AdditionalPaymentMethodConnectorResponse::Card {
        authentication_data: None,
        payment_checks,
        card_network: None,
        domestic_network: None,
    }
}
