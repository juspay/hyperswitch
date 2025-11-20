use api_models::payouts;
use common_enums::enums;
use common_utils::{
    pii,
    types::{MinorUnit, StringMinorUnit},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::{
        payouts::{PoFulfill, PoSync},
        refunds::{Execute, RSync},
    },
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, PayoutsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, PayoutsRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::{api, errors};
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{PayoutsResponseRouterData, RefundsResponseRouterData, ResponseRouterData},
    utils::{PayoutFulfillRequestData, PayoutsData as _, RouterData as _},
};

//TODO: Fill the struct with respective fields
pub struct EnvoyRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for EnvoyRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct EnvoyPaymentsRequest {
    amount: StringMinorUnit,
    card: EnvoyCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct EnvoyCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&EnvoyRouterData<&PaymentsAuthorizeRouterData>> for EnvoyPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &EnvoyRouterData<&PaymentsAuthorizeRouterData>) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(_) => Err(errors::ConnectorError::NotImplemented(
                "Card payment method not implemented".to_string(),
            )
            .into()),
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct EnvoyAuthType {
    pub(super) username: Secret<String>,
    pub(super) password: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for EnvoyAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                username: api_key.to_owned(),
                password: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EnvoyPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<EnvoyPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: EnvoyPaymentStatus) -> Self {
        match item {
            EnvoyPaymentStatus::Succeeded => Self::Charged,
            EnvoyPaymentStatus::Failed => Self::Failure,
            EnvoyPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnvoyPaymentsResponse {
    status: EnvoyPaymentStatus,
    id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, EnvoyPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, EnvoyPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct EnvoyRefundRequest {
    pub amount: StringMinorUnit,
}

impl<F> TryFrom<&EnvoyRouterData<&RefundsRouterData<F>>> for EnvoyRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &EnvoyRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Copy, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing => Self::Pending,
            //TODO: Review mapping
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct EnvoyErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
    pub network_advice_code: Option<String>,
    pub network_decline_code: Option<String>,
    pub network_error_message: Option<String>,
}

// PAYOUT IMPLEMENTATION

// Payout Router Data
pub struct EnvoyPayoutRouterData<T> {
    pub amount: MinorUnit,
    pub router_data: T,
}

impl<T> TryFrom<(&api::CurrencyUnit, enums::Currency, MinorUnit, T)> for EnvoyPayoutRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (_currency_unit, _currency, minor_amount, item): (
            &api::CurrencyUnit,
            enums::Currency,
            MinorUnit,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: minor_amount,
            router_data: item,
        })
    }
}

// Payout connector metadata
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct EnvoyPayoutConnectorMetadataObject {
    pub merchant_id: Option<Secret<String>>,
    pub endpoint_url: Option<String>,
}

impl TryFrom<Option<&pii::SecretSerdeValue>> for EnvoyPayoutConnectorMetadataObject {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: Option<&pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self =
            crate::utils::to_connector_meta_from_secret::<Self>(meta_data.cloned())
                .change_context(errors::ConnectorError::InvalidConnectorConfig {
                    config: "metadata",
                })?;
        Ok(metadata)
    }
}

// Payout Request Structures
#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct EnvoyPayoutRequest {
    pub request_reference: String,
    pub amount: MinorUnit,
    pub currency: String,
    pub merchant: EnvoyMerchant,
    pub payout_details: EnvoyPayoutDetails,
    pub destination: EnvoyPayoutDestination,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct EnvoyMerchant {
    pub merchant_id: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[serde(untagged)]
pub enum EnvoyPayoutDetails {
    Bank(EnvoyBankPayoutDetails),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct EnvoyBankPayoutDetails {
    pub account_number: Secret<String>,
    pub bank_code: Secret<String>,
    pub bank_name: Option<String>,
    pub account_type: Option<EnvoyAccountType>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub enum EnvoyAccountType {
    Checking,
    Savings,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct EnvoyPayoutDestination {
    pub name: Secret<String>,
    pub address: Option<EnvoyAddress>,
    pub email: Option<pii::Email>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct EnvoyAddress {
    pub street1: Secret<String>,
    pub street2: Option<Secret<String>>,
    pub city: String,
    pub state: Option<Secret<String>>,
    pub postal_code: Option<Secret<String>>,
    pub country: enums::CountryAlpha2,
}

// Payout Response Structures
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct EnvoyPayoutResponse {
    pub transaction_id: String,
    pub status: EnvoyPayoutStatus,
    pub status_message: Option<String>,
    pub amount: MinorUnit,
    pub currency: String,
    pub reference: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum EnvoyPayoutStatus {
    Pending,
    Approved,
    Sent,
    Completed,
    Failed,
    Cancelled,
}

impl From<EnvoyPayoutStatus> for enums::PayoutStatus {
    fn from(item: EnvoyPayoutStatus) -> Self {
        match item {
            EnvoyPayoutStatus::Pending => Self::Initiated,
            EnvoyPayoutStatus::Approved => Self::Initiated,
            EnvoyPayoutStatus::Sent => Self::Initiated,
            EnvoyPayoutStatus::Completed => Self::Success,
            EnvoyPayoutStatus::Failed => Self::Failed,
            EnvoyPayoutStatus::Cancelled => Self::Cancelled,
        }
    }
}

// Implementation for POFulfill Request
impl<F>
    TryFrom<(
        &EnvoyPayoutRouterData<&PayoutsRouterData<F>>,
        &EnvoyAuthType,
    )> for EnvoyPayoutRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        req: (
            &EnvoyPayoutRouterData<&PayoutsRouterData<F>>,
            &EnvoyAuthType,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, _auth) = req;

        let envoy_connector_metadata_object = EnvoyPayoutConnectorMetadataObject::try_from(
            item.router_data.connector_meta_data.as_ref(),
        )?;

        let merchant_id = envoy_connector_metadata_object.merchant_id.ok_or(
            errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata.merchant_id",
            },
        )?;

        let payout_method_data = item.router_data.get_payout_method_data()?;
        let payout_details = EnvoyPayoutDetails::try_from(payout_method_data)?;

        let billing = item.router_data.get_billing()?;
        let destination_name = billing
            .address
            .as_ref()
            .and_then(|addr| addr.get_optional_full_name())
            .or_else(|| {
                // Try to get name from phone.contact_first_name and phone.contact_last_name or other fields
                if let Some(addr) = &billing.address {
                    addr.get_optional_full_name()
                } else {
                    None
                }
            })
            .unwrap_or_else(|| Secret::new("Default Name".to_string()));

        let address = billing.address.as_ref().map(|addr| EnvoyAddress {
            street1: addr.line1.clone().unwrap_or(Secret::new("".to_string())),
            street2: addr.line2.clone(),
            city: addr.city.clone().unwrap_or("".to_string()),
            state: addr.state.clone(),
            postal_code: addr.zip.clone(),
            country: addr.country.clone().unwrap_or(enums::CountryAlpha2::US),
        });

        Ok(Self {
            request_reference: item.router_data.connector_request_reference_id.clone(),
            amount: item.amount,
            currency: item.router_data.request.destination_currency.to_string(),
            merchant: EnvoyMerchant { merchant_id },
            payout_details,
            destination: EnvoyPayoutDestination {
                name: destination_name,
                address,
                email: billing.email.clone(),
            },
        })
    }
}

impl TryFrom<payouts::PayoutMethodData> for EnvoyPayoutDetails {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(payout_method_data: payouts::PayoutMethodData) -> Result<Self, Self::Error> {
        match payout_method_data {
            payouts::PayoutMethodData::Bank(bank_data) => match bank_data {
                payouts::Bank::PayToBank(pay_to_bank_data) => {
                    Ok(Self::Bank(EnvoyBankPayoutDetails {
                        account_number: pay_to_bank_data.bank_account_number,
                        bank_code: pay_to_bank_data.bank_code,
                        bank_name: pay_to_bank_data.bank_name,
                        account_type: None, // ACH doesn't have account_type field in the struct
                    }))
                }
                payouts::Bank::Ach(_)
                | payouts::Bank::Sepa(_)
                | payouts::Bank::Bacs(_)
                | payouts::Bank::Pix(_) => Err(errors::ConnectorError::NotSupported {
                    message: "Bank transfer type not supported".to_string(),
                    connector: "Envoy",
                }
                .into()),
            },
            payouts::PayoutMethodData::Card(_)
            | payouts::PayoutMethodData::Wallet(_)
            | payouts::PayoutMethodData::BankRedirect(_)
            | payouts::PayoutMethodData::Passthrough(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    "Selected payout method not implemented for Envoy".to_string(),
                )
                .into())
            }
        }
    }
}

// Remove the AchBankAccountType implementation as it doesn't exist in the API

// Implementation for POFulfill Response
impl TryFrom<PayoutsResponseRouterData<PoFulfill, EnvoyPayoutResponse>>
    for PayoutsRouterData<PoFulfill>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PayoutsResponseRouterData<PoFulfill, EnvoyPayoutResponse>,
    ) -> Result<Self, Self::Error> {
        let response = item.response;
        let status = enums::PayoutStatus::from(response.status);

        Ok(Self {
            response: Ok(PayoutsResponseData {
                status: Some(status),
                connector_payout_id: Some(response.transaction_id),
                payout_eligible: None,
                should_add_next_step_to_process_tracker: false,
                error_code: None,
                error_message: response.status_message,
                payout_connector_metadata: None,
            }),
            ..item.data
        })
    }
}

// Implementation for POSync Response
impl TryFrom<PayoutsResponseRouterData<PoSync, EnvoyPayoutResponse>> for PayoutsRouterData<PoSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PayoutsResponseRouterData<PoSync, EnvoyPayoutResponse>,
    ) -> Result<Self, Self::Error> {
        let response = item.response;
        let status = enums::PayoutStatus::from(response.status);

        Ok(Self {
            response: Ok(PayoutsResponseData {
                status: Some(status),
                connector_payout_id: Some(response.transaction_id),
                payout_eligible: None,
                should_add_next_step_to_process_tracker: false,
                error_code: None,
                error_message: response.status_message,
                payout_connector_metadata: None,
            }),
            ..item.data
        })
    }
}

// Enhanced error response for payouts
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct EnvoyPayoutErrorResponse {
    pub error_code: String,
    pub error_message: String,
    pub transaction_reference: Option<String>,
    pub details: Option<String>,
}

// Utility functions for better error handling
impl EnvoyPayoutErrorResponse {
    pub fn get_error_message(&self) -> String {
        if let Some(details) = &self.details {
            format!("{}: {}", self.error_message, details)
        } else {
            self.error_message.clone()
        }
    }

    pub fn get_error_code(&self) -> String {
        self.error_code.clone()
    }
}

// Error handling for POFulfill
impl From<EnvoyPayoutErrorResponse> for hyperswitch_domain_models::router_data::ErrorResponse {
    fn from(error: EnvoyPayoutErrorResponse) -> Self {
        Self {
            code: error.get_error_code(),
            message: error.get_error_message(),
            reason: error.details,
            status_code: 400, // Default to 400, will be overridden by actual HTTP status
            attempt_status: None,
            connector_transaction_id: error.transaction_reference,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        }
    }
}

// Validation utilities
impl EnvoyPayoutRequest {
    pub fn validate(&self) -> Result<(), errors::ConnectorError> {
        if self.request_reference.is_empty() {
            return Err(errors::ConnectorError::MissingRequiredField {
                field_name: "request_reference",
            });
        }

        if self.currency.is_empty() {
            return Err(errors::ConnectorError::MissingRequiredField {
                field_name: "currency",
            });
        }

        if self.destination.name.clone().expose().is_empty() {
            return Err(errors::ConnectorError::MissingRequiredField {
                field_name: "destination.name",
            });
        }

        // Validate amount is positive
        if self.amount.get_amount_as_i64() <= 0 {
            return Err(errors::ConnectorError::RequestEncodingFailed);
        }

        Ok(())
    }
}

// Additional status mapping for edge cases
impl From<String> for EnvoyPayoutStatus {
    fn from(status: String) -> Self {
        match status.to_uppercase().as_str() {
            "PENDING" => Self::Pending,
            "APPROVED" => Self::Approved,
            "SENT" => Self::Sent,
            "COMPLETED" | "SUCCESS" | "SUCCESSFUL" => Self::Completed,
            "FAILED" | "FAILURE" | "ERROR" => Self::Failed,
            "CANCELLED" | "CANCELED" => Self::Cancelled,
            _ => {
                // Default to Failed for unknown statuses for safety
                Self::Failed
            }
        }
    }
}

// Helper function to construct SOAP envelope for XML-based endpoints
pub fn build_soap_envelope(body: &str, action: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/" xmlns:tns="http://envoy.com/payout/service">
    <soap:Header>
        <Action xmlns="http://schemas.microsoft.com/ws/2005/05/addressing/none">{}</Action>
    </soap:Header>
    <soap:Body>
        {}
    </soap:Body>
</soap:Envelope>"#,
        action, body
    )
}

// Convert request to XML if needed
impl EnvoyPayoutRequest {
    pub fn to_xml(&self) -> Result<String, errors::ConnectorError> {
        let body = format!(
            r#"<tns:ProcessPayout>
                <tns:RequestReference>{}</tns:RequestReference>
                <tns:Amount>{}</tns:Amount>
                <tns:Currency>{}</tns:Currency>
                <tns:MerchantId>{}</tns:MerchantId>
                <tns:Destination>
                    <tns:Name>{}</tns:Name>
                    {}
                </tns:Destination>
                {}
            </tns:ProcessPayout>"#,
            self.request_reference,
            self.amount.get_amount_as_i64(),
            self.currency,
            self.merchant.merchant_id.clone().expose(),
            self.destination.name.clone().expose(),
            self.destination
                .address
                .as_ref()
                .map_or(String::new(), |addr| {
                    format!(
                        "<tns:Address>
                        <tns:Street1>{}</tns:Street1>
                        <tns:City>{}</tns:City>
                        <tns:Country>{}</tns:Country>
                    </tns:Address>",
                        addr.street1.clone().expose(),
                        addr.city,
                        addr.country
                    )
                }),
            match &self.payout_details {
                EnvoyPayoutDetails::Bank(bank) => format!(
                    "<tns:BankDetails>
                        <tns:AccountNumber>{}</tns:AccountNumber>
                        <tns:RoutingNumber>{}</tns:RoutingNumber>
                    </tns:BankDetails>",
                    bank.account_number.clone().expose(),
                    bank.bank_code.clone().expose()
                ),
            }
        );

        Ok(build_soap_envelope(&body, "ProcessPayout"))
    }
}

// Payout sync response type
pub type EnvoyPayoutSyncResponse = EnvoyPayoutResponse;

// Function to build payout sync request
pub fn build_payout_sync_request<F>(
    req: &PayoutsRouterData<F>,
) -> Result<Vec<u8>, errors::ConnectorError> {
    let body = format!(
        r#"<tns:GetPayoutStatus>
            <tns:RequestReference>{}</tns:RequestReference>
        </tns:GetPayoutStatus>"#,
        req.request
            .connector_payout_id
            .clone()
            .unwrap_or(req.connector_request_reference_id.clone())
    );

    Ok(build_soap_envelope(&body, "GetPayoutStatus").into_bytes())
}
