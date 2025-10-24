use common_enums::{enums, Currency};
use common_utils::{id_type::CustomerId, pii::Email, types::MinorUnit};
use hyperswitch_domain_models::{
    address::Address as DomainAddress,
    payment_method_data::PaymentMethodData,
    router_data::{
        AdditionalPaymentMethodConnectorResponse, ConnectorAuthType, ConnectorResponseData,
        RouterData,
    },
    router_flow_types::{
        payments::Capture,
        refunds::{Execute, RSync},
    },
    router_request_types::{PaymentsCaptureData, ResponseId},
    router_response_types::{MandateReference, PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, PaymentsSyncRouterData,
        RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{
    consts,
    errors::{self},
};
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{
        get_unimplemented_payment_method_error_message, AddressDetailsData,
        PaymentsAuthorizeRequestData, RefundsRequestData, RouterData as _,
    },
};

//TODO: Fill the struct with respective fields
pub struct CeleroRouterData<T> {
    pub amount: MinorUnit, // CeleroCommerce expects integer cents
    pub router_data: T,
}

impl<T> TryFrom<(MinorUnit, T)> for CeleroRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from((amount, item): (MinorUnit, T)) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}
// CeleroCommerce Search Request for sync operations - POST /api/transaction/search
#[derive(Debug, Serialize, PartialEq)]
pub struct CeleroSearchRequest {
    transaction_id: String,
}

impl TryFrom<&PaymentsSyncRouterData> for CeleroSearchRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let transaction_id = match &item.request.connector_transaction_id {
            ResponseId::ConnectorTransactionId(id) => id.clone(),
            _ => {
                return Err(errors::ConnectorError::MissingConnectorTransactionID.into());
            }
        };
        Ok(Self { transaction_id })
    }
}

impl TryFrom<&RefundSyncRouterData> for CeleroSearchRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RefundSyncRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction_id: item.request.get_connector_refund_id()?,
        })
    }
}

// CeleroCommerce Payment Request according to API specs
#[derive(Debug, Serialize, PartialEq)]
pub struct CeleroPaymentsRequest {
    idempotency_key: String,
    #[serde(rename = "type")]
    transaction_type: TransactionType,
    amount: MinorUnit, // CeleroCommerce expects integer cents
    currency: Currency,
    payment_method: CeleroPaymentMethod,
    #[serde(skip_serializing_if = "Option::is_none")]
    billing_address: Option<CeleroAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shipping_address: Option<CeleroAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    create_vault_record: Option<bool>,
    // CIT/MIT fields
    #[serde(skip_serializing_if = "Option::is_none")]
    card_on_file_indicator: Option<CardOnFileIndicator>,
    #[serde(skip_serializing_if = "Option::is_none")]
    initiated_by: Option<InitiatedBy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    initial_transaction_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stored_credential_indicator: Option<StoredCredentialIndicator>,
    #[serde(skip_serializing_if = "Option::is_none")]
    billing_method: Option<BillingMethod>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CeleroAddress {
    first_name: Option<Secret<String>>,
    last_name: Option<Secret<String>>,
    address_line_1: Option<Secret<String>>,
    address_line_2: Option<Secret<String>>,
    city: Option<String>,
    state: Option<Secret<String>>,
    postal_code: Option<Secret<String>>,
    country: Option<common_enums::CountryAlpha2>,
    phone: Option<Secret<String>>,
    email: Option<Email>,
}

impl TryFrom<&DomainAddress> for CeleroAddress {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(address: &DomainAddress) -> Result<Self, Self::Error> {
        let address_details = address.address.as_ref();
        match address_details {
            Some(address_details) => Ok(Self {
                first_name: address_details.get_optional_first_name(),
                last_name: address_details.get_optional_last_name(),
                address_line_1: address_details.get_optional_line1(),
                address_line_2: address_details.get_optional_line2(),
                city: address_details.get_optional_city(),
                state: address_details.get_optional_state(),
                postal_code: address_details.get_optional_zip(),
                country: address_details.get_optional_country(),
                phone: address
                    .phone
                    .as_ref()
                    .and_then(|phone| phone.number.clone()),
                email: address.email.clone(),
            }),
            None => Err(errors::ConnectorError::MissingRequiredField {
                field_name: "address_details",
            }
            .into()),
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CeleroPaymentMethod {
    Card(CeleroCard),
    Customer(CeleroCustomer),
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CeleroCustomer {
    id: Option<CustomerId>,
    payment_method_id: Option<String>,
}
#[derive(Debug, Serialize, PartialEq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum CeleroEntryType {
    Keyed,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CeleroCard {
    entry_type: CeleroEntryType,
    number: cards::CardNumber,
    expiration_date: Secret<String>,
    cvc: Secret<String>,
}

impl TryFrom<&PaymentMethodData> for CeleroPaymentMethod {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentMethodData) -> Result<Self, Self::Error> {
        match item {
            PaymentMethodData::Card(req_card) => {
                let card = CeleroCard {
                    entry_type: CeleroEntryType::Keyed,
                    number: req_card.card_number.clone(),
                    expiration_date: Secret::new(format!(
                        "{}/{}",
                        req_card.card_exp_month.peek(),
                        req_card.card_exp_year.peek()
                    )),
                    cvc: req_card.card_cvc.clone(),
                };
                Ok(Self::Card(card))
            }
            PaymentMethodData::CardDetailsForNetworkTransactionId(_)
            | PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::Wallet(_)
            | PaymentMethodData::PayLater(_)
            | PaymentMethodData::BankRedirect(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::BankTransfer(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::MandatePayment
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::MobilePayment(_) => Err(errors::ConnectorError::NotImplemented(
                "Selected payment method through celero".to_string(),
            )
            .into()),
        }
    }
}

// Implementation for handling 3DS specifically
impl TryFrom<(&PaymentMethodData, bool)> for CeleroPaymentMethod {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from((item, is_three_ds): (&PaymentMethodData, bool)) -> Result<Self, Self::Error> {
        // If 3DS is requested, return an error
        if is_three_ds {
            return Err(errors::ConnectorError::NotSupported {
                message: "Cards 3DS".to_string(),
                connector: "celero",
            }
            .into());
        }

        // Otherwise, delegate to the standard implementation
        Self::try_from(item)
    }
}

impl TryFrom<&CeleroRouterData<&PaymentsAuthorizeRouterData>> for CeleroPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CeleroRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let is_auto_capture = item.router_data.request.is_auto_capture()?;
        let transaction_type = if is_auto_capture {
            TransactionType::Sale
        } else {
            TransactionType::Authorize
        };

        let billing_address: Option<CeleroAddress> = item
            .router_data
            .get_optional_billing()
            .and_then(|address| address.try_into().ok());

        let shipping_address: Option<CeleroAddress> = item
            .router_data
            .get_optional_shipping()
            .and_then(|address| address.try_into().ok());

        // Determine CIT/MIT fields based on mandate data
        let (mandate_fields, payment_method) = determine_cit_mit_fields(item.router_data)?;

        let request = Self {
            idempotency_key: item.router_data.connector_request_reference_id.clone(),
            transaction_type,
            amount: item.amount,
            currency: item.router_data.request.currency,
            payment_method,
            billing_address,
            shipping_address,
            create_vault_record: Some(false),
            card_on_file_indicator: mandate_fields.card_on_file_indicator,
            initiated_by: mandate_fields.initiated_by,
            initial_transaction_id: mandate_fields.initial_transaction_id,
            stored_credential_indicator: mandate_fields.stored_credential_indicator,
            billing_method: mandate_fields.billing_method,
        };

        Ok(request)
    }
}

// Define a struct to hold CIT/MIT fields to avoid complex tuple return type
#[derive(Debug, Default)]
pub struct CeleroMandateFields {
    pub card_on_file_indicator: Option<CardOnFileIndicator>,
    pub initiated_by: Option<InitiatedBy>,
    pub initial_transaction_id: Option<String>,
    pub stored_credential_indicator: Option<StoredCredentialIndicator>,
    pub billing_method: Option<BillingMethod>,
}

// Helper function to determine CIT/MIT fields based on mandate data
fn determine_cit_mit_fields(
    router_data: &PaymentsAuthorizeRouterData,
) -> Result<(CeleroMandateFields, CeleroPaymentMethod), error_stack::Report<errors::ConnectorError>>
{
    // Default null values
    let mut mandate_fields = CeleroMandateFields::default();

    // First check if there's a mandate_id in the request
    match router_data
        .request
        .mandate_id
        .clone()
        .and_then(|mandate_ids| mandate_ids.mandate_reference_id)
    {
        // If there's a connector mandate ID, this is a MIT (Merchant Initiated Transaction)
        Some(api_models::payments::MandateReferenceId::ConnectorMandateId(
            connector_mandate_id,
        )) => {
            mandate_fields.card_on_file_indicator = Some(CardOnFileIndicator::RecurringPayment);
            mandate_fields.initiated_by = Some(InitiatedBy::Merchant); // This is a MIT
            mandate_fields.stored_credential_indicator = Some(StoredCredentialIndicator::Used);
            mandate_fields.billing_method = Some(BillingMethod::Recurring);
            mandate_fields.initial_transaction_id =
                connector_mandate_id.get_connector_mandate_request_reference_id();
            Ok((
                mandate_fields,
                CeleroPaymentMethod::Customer(CeleroCustomer {
                    id: Some(router_data.get_customer_id()?),
                    payment_method_id: connector_mandate_id.get_payment_method_id(),
                }),
            ))
        }
        // For other mandate types that might not be supported
        Some(api_models::payments::MandateReferenceId::NetworkMandateId(_))
        | Some(api_models::payments::MandateReferenceId::NetworkTokenWithNTI(_)) => {
            // These might need different handling or return an error
            Err(errors::ConnectorError::NotImplemented(
                get_unimplemented_payment_method_error_message("Celero"),
            )
            .into())
        }
        // If no mandate ID is present, check if it's a mandate payment
        None => {
            if router_data.request.is_mandate_payment() {
                // This is a customer-initiated transaction for a recurring payment
                mandate_fields.initiated_by = Some(InitiatedBy::Customer);
                mandate_fields.card_on_file_indicator = Some(CardOnFileIndicator::RecurringPayment);
                mandate_fields.billing_method = Some(BillingMethod::Recurring);
                mandate_fields.stored_credential_indicator = Some(StoredCredentialIndicator::Used);
            }
            let is_three_ds = router_data.is_three_ds();
            Ok((
                mandate_fields,
                CeleroPaymentMethod::try_from((
                    &router_data.request.payment_method_data,
                    is_three_ds,
                ))?,
            ))
        }
    }
}

// Auth Struct for CeleroCommerce API key authentication
pub struct CeleroAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for CeleroAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.clone(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// CeleroCommerce API Response Structures
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CeleroResponseStatus {
    #[serde(alias = "success", alias = "Success", alias = "SUCCESS")]
    Success,
    #[serde(alias = "error", alias = "Error", alias = "ERROR")]
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CeleroTransactionStatus {
    Approved,
    Declined,
    Error,
    Pending,
    PendingSettlement,
    Settled,
    Voided,
    Reversed,
}

impl From<CeleroTransactionStatus> for common_enums::AttemptStatus {
    fn from(item: CeleroTransactionStatus) -> Self {
        match item {
            CeleroTransactionStatus::Approved => Self::Authorized,
            CeleroTransactionStatus::Settled => Self::Charged,
            CeleroTransactionStatus::Declined | CeleroTransactionStatus::Error => Self::Failure,
            CeleroTransactionStatus::Pending | CeleroTransactionStatus::PendingSettlement => {
                Self::Pending
            }
            CeleroTransactionStatus::Voided | CeleroTransactionStatus::Reversed => Self::Voided,
        }
    }
}
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CeleroCardResponse {
    pub status: CeleroTransactionStatus,
    pub auth_code: Option<String>,
    pub processor_response_code: Option<String>,
    pub avs_response_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CeleroPaymentMethodResponse {
    Card(CeleroCardResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Sale,
    Authorize,
}

// CIT/MIT related enums
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CardOnFileIndicator {
    #[serde(rename = "C")]
    GeneralPurposeStorage,
    #[serde(rename = "R")]
    RecurringPayment,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum InitiatedBy {
    Customer,
    Merchant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StoredCredentialIndicator {
    Used,
    Stored,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BillingMethod {
    Straight,
    #[serde(rename = "initial_recurring")]
    InitialRecurring,
    Recurring,
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde_with::skip_serializing_none]
pub struct CeleroTransactionResponseData {
    pub id: String,
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    pub amount: i64,
    pub currency: String,
    pub response: CeleroPaymentMethodResponse,
    pub billing_address: Option<CeleroAddressResponse>,
    pub shipping_address: Option<CeleroAddressResponse>,
    // Additional fields from the sample response
    pub status: Option<String>,
    pub response_code: Option<i32>,
    pub customer_id: Option<String>,
    pub payment_method_id: Option<String>,
}

impl CeleroTransactionResponseData {
    pub fn get_mandate_reference(&self) -> Box<Option<MandateReference>> {
        if self.payment_method_id.is_some() {
            Box::new(Some(MandateReference {
                connector_mandate_id: None,
                payment_method_id: self.payment_method_id.clone(),
                mandate_metadata: None,
                connector_mandate_request_reference_id: Some(self.id.clone()),
            }))
        } else {
            Box::new(None)
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct CeleroAddressResponse {
    first_name: Option<Secret<String>>,
    last_name: Option<Secret<String>>,
    address_line_1: Option<Secret<String>>,
    address_line_2: Option<Secret<String>>,
    city: Option<String>,
    state: Option<Secret<String>>,
    postal_code: Option<Secret<String>>,
    country: Option<common_enums::CountryAlpha2>,
    phone: Option<Secret<String>>,
    email: Option<Secret<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct CeleroPaymentsResponse {
    pub status: CeleroResponseStatus,
    pub msg: String,
    pub data: Option<CeleroTransactionResponseData>,
}

impl<F, T> TryFrom<ResponseRouterData<F, CeleroPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, CeleroPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response.status {
            CeleroResponseStatus::Success => {
                if let Some(data) = item.response.data {
                    let CeleroPaymentMethodResponse::Card(response) = &data.response;
                    // Check if transaction itself failed despite successful API call
                    match response.status {
                        CeleroTransactionStatus::Declined | CeleroTransactionStatus::Error => {
                            // Transaction failed - create error response with transaction details
                            let error_details = CeleroErrorDetails::from_transaction_response(
                                response,
                                item.response.msg,
                            );

                            Ok(Self {
                                status: common_enums::AttemptStatus::Failure,
                                response: Err(
                                    hyperswitch_domain_models::router_data::ErrorResponse {
                                        code: error_details
                                            .error_code
                                            .unwrap_or_else(|| "TRANSACTION_FAILED".to_string()),
                                        message: error_details.error_message,
                                        reason: error_details.decline_reason,
                                        status_code: item.http_code,
                                        attempt_status: None,
                                        connector_transaction_id: Some(data.id),
                                        network_decline_code: None,
                                        network_advice_code: None,
                                        network_error_message: None,
                                        connector_metadata: None,
                                    },
                                ),
                                ..item.data
                            })
                        }
                        _ => {
                            let connector_response_data =
                                convert_to_additional_payment_method_connector_response(
                                    response.avs_response_code.clone(),
                                )
                                .map(ConnectorResponseData::with_additional_payment_method_data);
                            let final_status: enums::AttemptStatus = response.status.into();
                            Ok(Self {
                                status: final_status,
                                response: Ok(PaymentsResponseData::TransactionResponse {
                                    resource_id: ResponseId::ConnectorTransactionId(
                                        data.id.clone(),
                                    ),
                                    redirection_data: Box::new(None),
                                    mandate_reference: data.get_mandate_reference(),
                                    connector_metadata: None,
                                    network_txn_id: None,
                                    connector_response_reference_id: response.auth_code.clone(),
                                    incremental_authorization_allowed: None,
                                    charges: None,
                                }),
                                connector_response: connector_response_data,
                                ..item.data
                            })
                        }
                    }
                } else {
                    // No transaction data in successful response
                    // We don't have a transaction ID in this case
                    Ok(Self {
                        status: common_enums::AttemptStatus::Failure,
                        response: Err(hyperswitch_domain_models::router_data::ErrorResponse {
                            code: "MISSING_DATA".to_string(),
                            message: "No transaction data in response".to_string(),
                            reason: Some(item.response.msg),
                            status_code: item.http_code,
                            attempt_status: None,
                            connector_transaction_id: None,
                            network_decline_code: None,
                            network_advice_code: None,
                            network_error_message: None,
                            connector_metadata: None,
                        }),
                        ..item.data
                    })
                }
            }
            CeleroResponseStatus::Error => {
                // Top-level API error
                let error_details =
                    CeleroErrorDetails::from_top_level_error(item.response.msg.clone());

                // Extract transaction ID from the top-level data if available
                let connector_transaction_id =
                    item.response.data.as_ref().map(|data| data.id.clone());

                Ok(Self {
                    status: common_enums::AttemptStatus::Failure,
                    response: Err(hyperswitch_domain_models::router_data::ErrorResponse {
                        code: error_details
                            .error_code
                            .unwrap_or_else(|| "API_ERROR".to_string()),
                        message: error_details.error_message,
                        reason: error_details.decline_reason,
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id,
                        network_decline_code: None,
                        network_advice_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    }),
                    ..item.data
                })
            }
        }
    }
}

// CAPTURE:
// Type definition for CaptureRequest
#[derive(Default, Debug, Serialize)]
pub struct CeleroCaptureRequest {
    pub amount: MinorUnit,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
}

impl TryFrom<&CeleroRouterData<&PaymentsCaptureRouterData>> for CeleroCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &CeleroRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount,
            order_id: Some(item.router_data.payment_id.clone()),
        })
    }
}

// CeleroCommerce Capture Response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CeleroCaptureResponse {
    pub status: CeleroResponseStatus,
    pub msg: Option<String>,
    pub data: Option<serde_json::Value>, // Usually null for capture responses
}

impl
    TryFrom<
        ResponseRouterData<
            Capture,
            CeleroCaptureResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    > for RouterData<Capture, PaymentsCaptureData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            Capture,
            CeleroCaptureResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response.status {
            CeleroResponseStatus::Success => Ok(Self {
                status: common_enums::AttemptStatus::Charged,
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(
                        item.data.request.connector_transaction_id.clone(),
                    ),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            }),
            CeleroResponseStatus::Error => Ok(Self {
                status: common_enums::AttemptStatus::Failure,
                response: Err(hyperswitch_domain_models::router_data::ErrorResponse {
                    code: "CAPTURE_FAILED".to_string(),
                    message: item
                        .response
                        .msg
                        .clone()
                        .unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
                    reason: None,
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: Some(
                        item.data.request.connector_transaction_id.clone(),
                    ),
                    network_decline_code: None,
                    network_advice_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..item.data
            }),
        }
    }
}

// CeleroCommerce Void Response - matches API spec format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CeleroVoidResponse {
    pub status: CeleroResponseStatus,
    pub msg: String,
    pub data: Option<serde_json::Value>, // Usually null for void responses
}

impl
    TryFrom<
        ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::payments::Void,
            CeleroVoidResponse,
            hyperswitch_domain_models::router_request_types::PaymentsCancelData,
            PaymentsResponseData,
        >,
    >
    for RouterData<
        hyperswitch_domain_models::router_flow_types::payments::Void,
        hyperswitch_domain_models::router_request_types::PaymentsCancelData,
        PaymentsResponseData,
    >
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::payments::Void,
            CeleroVoidResponse,
            hyperswitch_domain_models::router_request_types::PaymentsCancelData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response.status {
            CeleroResponseStatus::Success => Ok(Self {
                status: common_enums::AttemptStatus::Voided,
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(
                        item.data.request.connector_transaction_id.clone(),
                    ),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            }),
            CeleroResponseStatus::Error => Ok(Self {
                status: common_enums::AttemptStatus::Failure,
                response: Err(hyperswitch_domain_models::router_data::ErrorResponse {
                    code: "VOID_FAILED".to_string(),
                    message: item.response.msg.clone(),
                    reason: Some(item.response.msg),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: Some(
                        item.data.request.connector_transaction_id.clone(),
                    ),
                    network_decline_code: None,
                    network_advice_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..item.data
            }),
        }
    }
}
#[derive(Default, Debug, Serialize)]
pub struct CeleroRefundRequest {
    pub amount: MinorUnit,
    pub surcharge: MinorUnit, // Required field as per API specification
}

impl<F> TryFrom<&CeleroRouterData<&RefundsRouterData<F>>> for CeleroRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &CeleroRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount,
            surcharge: MinorUnit::zero(), // Default to 0 as per API specification
        })
    }
}

// CeleroCommerce Refund Response - matches API spec format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CeleroRefundResponse {
    pub status: CeleroResponseStatus,
    pub msg: String,
    pub data: Option<serde_json::Value>, // Usually null for refund responses
}

impl TryFrom<RefundsResponseRouterData<Execute, CeleroRefundResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, CeleroRefundResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response.status {
            CeleroResponseStatus::Success => Ok(Self {
                response: Ok(RefundsResponseData {
                    connector_refund_id: item.data.request.refund_id.clone(),
                    refund_status: enums::RefundStatus::Success,
                }),
                ..item.data
            }),
            CeleroResponseStatus::Error => Ok(Self {
                response: Err(hyperswitch_domain_models::router_data::ErrorResponse {
                    code: "REFUND_FAILED".to_string(),
                    message: item.response.msg.clone(),
                    reason: Some(item.response.msg),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: Some(
                        item.data.request.connector_transaction_id.clone(),
                    ),
                    network_decline_code: None,
                    network_advice_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..item.data
            }),
        }
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, CeleroRefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, CeleroRefundResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response.status {
            CeleroResponseStatus::Success => Ok(Self {
                response: Ok(RefundsResponseData {
                    connector_refund_id: item.data.request.refund_id.clone(),
                    refund_status: enums::RefundStatus::Success,
                }),
                ..item.data
            }),
            CeleroResponseStatus::Error => Ok(Self {
                response: Err(hyperswitch_domain_models::router_data::ErrorResponse {
                    code: "REFUND_SYNC_FAILED".to_string(),
                    message: item.response.msg.clone(),
                    reason: Some(item.response.msg),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: Some(
                        item.data.request.connector_transaction_id.clone(),
                    ),
                    network_decline_code: None,
                    network_advice_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..item.data
            }),
        }
    }
}

// CeleroCommerce Error Response Structures

// Main error response structure - matches API spec format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CeleroErrorResponse {
    pub status: CeleroResponseStatus,
    pub msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

// Error details that can be extracted from various response fields
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CeleroErrorDetails {
    pub error_code: Option<String>,
    pub error_message: String,
    pub processor_response_code: Option<String>,
    pub decline_reason: Option<String>,
}

impl From<CeleroErrorResponse> for CeleroErrorDetails {
    fn from(error_response: CeleroErrorResponse) -> Self {
        Self {
            error_code: Some("API_ERROR".to_string()),
            error_message: error_response.msg,
            processor_response_code: None,
            decline_reason: None,
        }
    }
}

// Function to extract error details from transaction response data
impl CeleroErrorDetails {
    pub fn from_transaction_response(response: &CeleroCardResponse, msg: String) -> Self {
        // Map specific error codes based on common response patterns
        let decline_reason = Self::map_processor_error(&response.processor_response_code, &msg);

        Self {
            error_code: None,
            error_message: msg,
            processor_response_code: response.processor_response_code.clone(),
            decline_reason,
        }
    }

    pub fn from_top_level_error(msg: String) -> Self {
        // Map specific error codes from top-level API errors

        Self {
            error_code: None,
            error_message: msg,
            processor_response_code: None,
            decline_reason: None,
        }
    }

    /// Map processor response codes and messages to specific Hyperswitch error codes
    fn map_processor_error(processor_code: &Option<String>, message: &str) -> Option<String> {
        let message_lower = message.to_lowercase();
        // Check processor response codes if available
        if let Some(code) = processor_code {
            match code.as_str() {
                "05" => Some("TRANSACTION_DECLINED".to_string()),
                "14" => Some("INVALID_CARD_DATA".to_string()),
                "51" => Some("INSUFFICIENT_FUNDS".to_string()),
                "54" => Some("EXPIRED_CARD".to_string()),
                "55" => Some("INCORRECT_CVC".to_string()),
                "61" => Some("Exceeds withdrawal amount limit".to_string()),
                "62" => Some("TRANSACTION_DECLINED".to_string()),
                "65" => Some("Exceeds withdrawal frequency limit".to_string()),
                "78" => Some("INVALID_CARD_DATA".to_string()),
                "91" => Some("PROCESSING_ERROR".to_string()),
                "96" => Some("PROCESSING_ERROR".to_string()),
                _ => {
                    router_env::logger::info!(
                        "Celero response error code ({:?}) is not mapped to any error state ",
                        code
                    );
                    Some("Transaction failed".to_string())
                }
            }
        } else {
            Some(message_lower)
        }
    }
}

pub fn get_avs_definition(code: &str) -> Option<&'static str> {
    match code {
        "0" => Some("AVS Not Available"),
        "A" => Some("Address match only"),
        "B" => Some("Address matches, ZIP not verified"),
        "C" => Some("Incompatible format"),
        "D" => Some("Exact match"),
        "F" => Some("Exact match, UK-issued cards"),
        "G" => Some("Non-U.S. Issuer does not participate"),
        "I" => Some("Not verified"),
        "M" => Some("Exact match"),
        "N" => Some("No address or ZIP match"),
        "P" => Some("Postal Code match"),
        "R" => Some("Issuer system unavailable"),
        "S" => Some("Service not supported"),
        "U" => Some("Address unavailable"),
        "W" => Some("9-character numeric ZIP match only"),
        "X" => Some("Exact match, 9-character numeric ZIP"),
        "Y" => Some("Exact match, 5-character numeric ZIP"),
        "Z" => Some("5-character ZIP match only"),
        "L" => Some("Partial match, Name and billing postal code match"),
        "1" => Some("Cardholder name and ZIP match"),
        "2" => Some("Cardholder name, address and ZIP match"),
        "3" => Some("Cardholder name and address match"),
        "4" => Some("Cardholder name matches"),
        "5" => Some("Cardholder name incorrect, ZIP matches"),
        "6" => Some("Cardholder name incorrect, address and zip match"),
        "7" => Some("Cardholder name incorrect, address matches"),
        "8" => Some("Cardholder name, address, and ZIP do not match"),
        _ => {
            router_env::logger::info!(
                "Celero avs response code ({:?}) is not mapped to any definition.",
                code
            );

            None
        } // No definition found for the given code
    }
}
fn convert_to_additional_payment_method_connector_response(
    response_code: Option<String>,
) -> Option<AdditionalPaymentMethodConnectorResponse> {
    match response_code {
        None => None,
        Some(code) => {
            let description = get_avs_definition(&code);
            let payment_checks = serde_json::json!({
                "avs_result_code": code,
                "description": description
            });
            Some(AdditionalPaymentMethodConnectorResponse::Card {
                authentication_data: None,
                payment_checks: Some(payment_checks),
                card_network: None,
                domestic_network: None,
            })
        }
    }
}
