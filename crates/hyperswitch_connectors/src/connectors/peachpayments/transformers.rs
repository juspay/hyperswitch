use std::str::FromStr;

use cards::{CardNumber, NetworkToken};
use common_enums::enums as storage_enums;
use common_utils::{errors::CustomResult, pii, types::MinorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{
        Card, CardDetailsForNetworkTransactionId, CardWithLimitedDetails, NetworkTokenData,
        NetworkTokenDetailsForNetworkTransactionId, PaymentMethodData,
    },
    payment_methods::storage_enums::MitCategory,
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{RefundsData, ResponseId},
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        RefundsRouterData,
    },
};
use hyperswitch_interfaces::{
    consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE},
    errors,
};
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{
    types::ResponseRouterData,
    utils::{
        self, CardData, CardWithLimitedData as _, ForeignTryFrom, NetworkTokenData as _,
        PaymentsAuthorizeRequestData, RouterData as OtherRouterData,
    },
};

const CHARGE_METHOD: &str = "ecommerce_card_payment_only";
const CONNECTOR: &str = "Peachpayments";

pub struct PeachpaymentsRouterData<T> {
    pub amount: MinorUnit,
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for PeachpaymentsRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

impl TryFrom<&Option<pii::SecretSerdeValue>> for PeachPaymentsConnectorMetadataObject {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: &Option<pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata = utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata",
            })?;
        Ok(metadata)
    }
}

// Card Gateway API Transaction Request
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PeachpaymentsPaymentsCardRequest {
    pub charge_method: String,
    pub reference_id: String,
    pub ecommerce_card_payment_only_transaction_data: EcommercePaymentOnlyTransactionData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pos_data: Option<serde_json::Value>,
    pub send_date_time: String,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PeachpaymentsPaymentsNTRequest {
    pub payment_method: String,
    pub reference_id: String,
    pub ecommerce_card_payment_only_transaction_data: EcommercePaymentOnlyTransactionData,
    pub send_date_time: String,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(untagged)]
pub enum PeachpaymentsPaymentsRequest {
    Card(PeachpaymentsPaymentsCardRequest),
    NetworkToken(PeachpaymentsPaymentsNTRequest),
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CardOnFileData {
    #[serde(rename = "type")]
    pub cof_type: CofType,
    pub source: CofSource,
    pub mode: CofMode,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EcommerceCardPaymentOnlyTransactionData {
    pub merchant_information: MerchantInformation,
    pub routing_reference: RoutingReference,
    pub card: CardDetails,
    pub amount: AmountDetails,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rrn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_auth_inc_ext_capture_flow: Option<PreAuthIncExtCaptureFlow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cof_data: Option<CardOnFileData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_link_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub three_d_s_data: Option<PeachpaymentsThreeDSData>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CofType {
    Adhoc,
    Recurring,
    Instalment,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CofSource {
    Cit,
    Mit,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CofMode {
    Initial,
    Subsequent,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EcommerceNetworkTokenPaymentOnlyTransactionData {
    pub merchant_information: MerchantInformation,
    pub routing_reference: RoutingReference,
    pub network_token_data: NetworkTokenDetails,
    pub amount: AmountDetails,
    pub cof_data: CardOnFileData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rrn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_auth_inc_ext_capture_flow: Option<PreAuthIncExtCaptureFlow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_link_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub three_d_s_data: Option<PeachpaymentsThreeDSData>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PreAuthIncExtCaptureFlow {
    pub dcc_mode: DccMode,
    pub txn_ref_nr: String,
}

#[derive(Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DccMode {
    #[default]
    NoDcc,
    OptInDcc,
    OptOutDcc,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PeachpaymentsThreeDSData {
    #[serde(skip_serializing_if = "Option::is_none")]
    cavv: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ds_trans_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    three_d_s_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    eci: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    authentication_status: Option<common_enums::TransactionStatus>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(untagged)]
pub enum EcommercePaymentOnlyTransactionData {
    Card(EcommerceCardPaymentOnlyTransactionData),
    NetworkToken(EcommerceNetworkTokenPaymentOnlyTransactionData),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MerchantInformation {
    pub client_merchant_reference_id: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MerchantType {
    Standard,
    Sub,
    Iso,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RoutingReference {
    pub merchant_payment_method_route_id: Secret<String>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CardDetails {
    pub pan: CardNumber,
    pub cardholder_name: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry_year: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry_month: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cvv: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eci: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkTokenDetails {
    pub token: NetworkToken,
    pub expiry_year: Secret<String>,
    pub expiry_month: Secret<String>,
    pub cryptogram: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eci: Option<String>,
    pub scheme: Option<CardNetworkLowercase>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CardNetworkLowercase {
    Visa,
    Mastercard,
    Amex,
    Discover,
    Jcb,
    Diners,
    CartesBancaires,
    UnionPay,
    Interac,
    RuPay,
    Maestro,
    Star,
    Pulse,
    Accel,
    Nyce,
    Prop,
    PrivateLabel,
    Dinacard,
}

impl From<common_enums::CardNetwork> for CardNetworkLowercase {
    fn from(value: common_enums::CardNetwork) -> Self {
        match value {
            common_enums::CardNetwork::Visa => Self::Visa,
            common_enums::CardNetwork::Mastercard => Self::Mastercard,
            common_enums::CardNetwork::AmericanExpress => Self::Amex,
            common_enums::CardNetwork::Discover => Self::Discover,
            common_enums::CardNetwork::JCB => Self::Jcb,
            common_enums::CardNetwork::DinersClub => Self::Diners,
            common_enums::CardNetwork::CartesBancaires => Self::CartesBancaires,
            common_enums::CardNetwork::UnionPay => Self::UnionPay,
            common_enums::CardNetwork::Interac => Self::Interac,
            common_enums::CardNetwork::RuPay => Self::RuPay,
            common_enums::CardNetwork::Maestro => Self::Maestro,
            common_enums::CardNetwork::Star => Self::Star,
            common_enums::CardNetwork::Pulse => Self::Pulse,
            common_enums::CardNetwork::Accel => Self::Accel,
            common_enums::CardNetwork::Nyce => Self::Nyce,
            common_enums::CardNetwork::Prop => Self::Prop,
            common_enums::CardNetwork::PrivateLabel => Self::PrivateLabel,
            common_enums::CardNetwork::Dinacard => Self::Dinacard,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmountDetails {
    pub amount: MinorUnit,
    pub currency_code: common_enums::Currency,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_amount: Option<String>,
}

// Confirm Transaction Request (for capture)
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PeachpaymentsCaptureRequest {
    pub amount: AmountDetails,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeachpaymentsRefundRequest {
    pub reference_id: String,
    pub ecommerce_card_payment_only_transaction_data: PeachpaymentsRefundTransactionData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pos_data: Option<PosData>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PosData {
    pub referral: String,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PeachpaymentsRefundTransactionData {
    pub amount: AmountDetails,
}

impl TryFrom<&RefundsRouterData<Execute>> for PeachpaymentsRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RefundsRouterData<Execute>) -> Result<Self, Self::Error> {
        let amount = AmountDetails {
            amount: item.request.minor_refund_amount,
            currency_code: item.request.currency,
            display_amount: None,
        };
        let ecommerce_card_payment_only_transaction_data =
            PeachpaymentsRefundTransactionData { amount };
        Ok(Self {
            reference_id: item.request.refund_id.clone(),
            ecommerce_card_payment_only_transaction_data,
            pos_data: None,
        })
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct EcommerceCardPaymentOnlyConfirmationData {
    pub amount: AmountDetails,
}

// Void Transaction Request
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PeachpaymentsVoidRequest {
    pub amount: AmountDetails,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethod {
    EcommerceCardPaymentOnly,
}

impl TryFrom<&PeachpaymentsRouterData<&PaymentsCaptureRouterData>> for PeachpaymentsCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PeachpaymentsRouterData<&PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: AmountDetails {
                amount: item.amount,
                currency_code: item.router_data.request.currency,
                display_amount: None,
            },
        })
    }
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FailureReason {
    UnableToSend,
    #[default]
    Timeout,
    SecurityError,
    IssuerUnavailable,
    TooLateResponse,
    Malfunction,
    UnableToComplete,
    OnlineDeclined,
    SuspectedFraud,
    CardDeclined,
    Partial,
    OfflineDeclined,
    CustomerCancel,
}

impl FromStr for FailureReason {
    type Err = error_stack::Report<errors::ConnectorError>;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "unable_to_send" => Ok(Self::UnableToSend),
            "timeout" => Ok(Self::Timeout),
            "security_error" => Ok(Self::SecurityError),
            "issuer_unavailable" => Ok(Self::IssuerUnavailable),
            "too_late_response" => Ok(Self::TooLateResponse),
            "malfunction" => Ok(Self::Malfunction),
            "unable_to_complete" => Ok(Self::UnableToComplete),
            "online_declined" => Ok(Self::OnlineDeclined),
            "suspected_fraud" => Ok(Self::SuspectedFraud),
            "card_declined" => Ok(Self::CardDeclined),
            "partial" => Ok(Self::Partial),
            "offline_declined" => Ok(Self::OfflineDeclined),
            "customer_cancel" => Ok(Self::CustomerCancel),
            _ => Ok(Self::Timeout),
        }
    }
}

impl TryFrom<&PeachpaymentsRouterData<&PaymentsCancelRouterData>> for PeachpaymentsVoidRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PeachpaymentsRouterData<&PaymentsCancelRouterData>,
    ) -> Result<Self, Self::Error> {
        let amount = AmountDetails {
            amount: item.amount,
            currency_code: item.router_data.request.currency.ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "Currency",
                },
            )?,
            display_amount: None,
        };
        Ok(Self { amount })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PeachPaymentsConnectorMetadataObject {
    pub client_merchant_reference_id: Secret<String>,
    pub merchant_payment_method_route_id: Secret<String>,
}

impl TryFrom<&PeachpaymentsRouterData<&PaymentsAuthorizeRouterData>>
    for PeachpaymentsPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PeachpaymentsRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        if item.router_data.is_three_ds() && item.router_data.request.authentication_data.is_none()
        {
            return Err(errors::ConnectorError::NotSupported {
                message: "3DS flow".to_string(),
                connector: CONNECTOR,
            }
            .into());
        }
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => Self::try_from((item, req_card)),
            PaymentMethodData::NetworkToken(token_data) => Self::try_from((item, token_data)),
            PaymentMethodData::CardWithLimitedDetails(card_with_limited_details) => {
                Self::try_from((item, card_with_limited_details))
            }
            PaymentMethodData::CardDetailsForNetworkTransactionId(
                card_details_for_network_transaction_id,
            ) => Self::try_from((item, card_details_for_network_transaction_id)),
            PaymentMethodData::NetworkTokenDetailsForNetworkTransactionId(
                network_details_for_network_transaction_id,
            ) => Self::try_from((item, network_details_for_network_transaction_id)),
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

impl
    TryFrom<(
        &PeachpaymentsRouterData<&PaymentsAuthorizeRouterData>,
        NetworkTokenData,
    )> for PeachpaymentsPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, token_data): (
            &PeachpaymentsRouterData<&PaymentsAuthorizeRouterData>,
            NetworkTokenData,
        ),
    ) -> Result<Self, Self::Error> {
        let (merchant_information, routing_reference) =
            get_config_data(&item.router_data.connector_meta_data)?;

        let network_token_data = NetworkTokenDetails {
            token: token_data.get_network_token(),
            expiry_year: token_data.get_token_expiry_year_2_digit()?,
            expiry_month: token_data.get_network_token_expiry_month(),
            cryptogram: token_data.get_cryptogram(),
            eci: token_data.eci.clone(),
            scheme: Some(CardNetworkLowercase::from(
                token_data.card_network.clone().ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "card_network",
                    },
                )?,
            )),
        };

        let peachpayments_data = get_peachpayments_data(item);

        let ecommerce_data = EcommercePaymentOnlyTransactionData::NetworkToken(
            EcommerceNetworkTokenPaymentOnlyTransactionData {
                merchant_information,
                routing_reference,
                network_token_data,
                amount: get_amount_details(item),
                cof_data: CardOnFileData {
                    cof_type: get_cof_type(item),
                    source: CofSource::Cit,
                    mode: CofMode::Initial,
                },
                rrn: get_rrn(&peachpayments_data),
                pre_auth_inc_ext_capture_flow: get_transaction_operations(item),
                trace_id: None,
                transaction_link_id: None,
                three_d_s_data: get_three_ds_data(item),
            },
        );

        Ok(Self::NetworkToken(PeachpaymentsPaymentsNTRequest {
            payment_method: CHARGE_METHOD.to_string(),
            reference_id: item.router_data.connector_request_reference_id.clone(),
            ecommerce_card_payment_only_transaction_data: ecommerce_data,
            send_date_time: get_send_date_time()?,
        }))
    }
}

impl TryFrom<(&PeachpaymentsRouterData<&PaymentsAuthorizeRouterData>, Card)>
    for PeachpaymentsPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, req_card): (&PeachpaymentsRouterData<&PaymentsAuthorizeRouterData>, Card),
    ) -> Result<Self, Self::Error> {
        let (merchant_information, routing_reference) =
            get_config_data(&item.router_data.connector_meta_data)?;

        let card = CardDetails {
            pan: req_card.card_number.clone(),
            cardholder_name: req_card.card_holder_name.clone().ok_or_else(|| {
                errors::ConnectorError::MissingRequiredField {
                    field_name: "card_holder_name",
                }
            })?,
            expiry_year: Some(req_card.get_card_expiry_year_2_digit()?),
            expiry_month: Some(req_card.card_exp_month.clone()),
            cvv: Some(req_card.card_cvc.clone()),
            eci: None,
        };

        let cof_data = if item.router_data.request.is_cit_mandate_payment() {
            Some(CardOnFileData {
                cof_type: get_cof_type(item),
                source: CofSource::Cit,
                mode: CofMode::Initial,
            })
        } else if item.router_data.request.setup_future_usage
            == Some(storage_enums::FutureUsage::OffSession)
        {
            Some(CardOnFileData {
                cof_type: get_cof_type(item),
                source: CofSource::Mit,
                mode: CofMode::Initial,
            })
        } else {
            None
        };

        let peachpayments_data = get_peachpayments_data(item);

        let ecommerce_data =
            EcommercePaymentOnlyTransactionData::Card(EcommerceCardPaymentOnlyTransactionData {
                merchant_information,
                routing_reference,
                card,
                amount: get_amount_details(item),
                rrn: get_rrn(&peachpayments_data),
                pre_auth_inc_ext_capture_flow: get_transaction_operations(item),
                cof_data,
                trace_id: None,
                transaction_link_id: None,
                three_d_s_data: get_three_ds_data(item),
            });

        Ok(Self::Card(PeachpaymentsPaymentsCardRequest {
            charge_method: CHARGE_METHOD.to_string(),
            reference_id: item.router_data.connector_request_reference_id.clone(),
            ecommerce_card_payment_only_transaction_data: ecommerce_data,
            pos_data: None,
            send_date_time: get_send_date_time()?,
        }))
    }
}

impl
    TryFrom<(
        &PeachpaymentsRouterData<&PaymentsAuthorizeRouterData>,
        CardWithLimitedDetails,
    )> for PeachpaymentsPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, card_with_limited_details): (
            &PeachpaymentsRouterData<&PaymentsAuthorizeRouterData>,
            CardWithLimitedDetails,
        ),
    ) -> Result<Self, Self::Error> {
        let (merchant_information, routing_reference) =
            get_config_data(&item.router_data.connector_meta_data)?;

        let card = CardDetails {
            pan: card_with_limited_details.card_number.clone(),
            cardholder_name: card_with_limited_details
                .card_holder_name
                .clone()
                .ok_or_else(|| errors::ConnectorError::MissingRequiredField {
                    field_name: "card_holder_name",
                })?,
            expiry_year: card_with_limited_details.get_card_expiry_year_2_digit()?,
            expiry_month: card_with_limited_details.card_exp_month.clone(),
            cvv: None,
            eci: card_with_limited_details.eci.clone(),
        };

        let peachpayments_data = get_peachpayments_data(item);

        let card_on_file_transaction_type = peachpayments_data
            .as_ref()
            .and_then(|peachpayments| peachpayments.card_on_file_transaction_type.clone());

        let cof_data = match card_on_file_transaction_type {
            Some(api_models::payments::PeachpaymentsCardOnFileTransactionType::OneOff)
            | None => None,
            Some(api_models::payments::PeachpaymentsCardOnFileTransactionType::CustomerInitiatedTransaction) => {
                Some(CardOnFileData {
                    cof_type: get_cof_type(item),
                    source: CofSource::Cit,
                    mode: CofMode::Initial,
                })
            },
            Some(api_models::payments::PeachpaymentsCardOnFileTransactionType::MerchantInitiatedMandate) => {
                Some(CardOnFileData {
                    cof_type: get_cof_type(item),
                    source: CofSource::Mit,
                    mode: CofMode::Initial,
                })
            },
            Some(api_models::payments::PeachpaymentsCardOnFileTransactionType::MerchantInitiatedTransaction) => {
                Some(CardOnFileData {
                    cof_type: get_cof_type(item),
                    source: CofSource::Mit,
                    mode: CofMode::Subsequent,
                })
            },
        };

        let trace_id = item
            .router_data
            .request
            .get_optional_network_transaction_id();

        let transaction_link_id = item.router_data.request.get_optional_transaction_link_id();

        let ecommerce_data =
            EcommercePaymentOnlyTransactionData::Card(EcommerceCardPaymentOnlyTransactionData {
                merchant_information,
                routing_reference,
                card,
                amount: get_amount_details(item),
                rrn: get_rrn(&peachpayments_data),
                pre_auth_inc_ext_capture_flow: get_transaction_operations(item),
                cof_data,
                trace_id,
                transaction_link_id,
                three_d_s_data: get_three_ds_data(item),
            });

        Ok(Self::Card(PeachpaymentsPaymentsCardRequest {
            charge_method: CHARGE_METHOD.to_string(),
            reference_id: item.router_data.connector_request_reference_id.clone(),
            ecommerce_card_payment_only_transaction_data: ecommerce_data,
            pos_data: None,
            send_date_time: get_send_date_time()?,
        }))
    }
}

impl
    TryFrom<(
        &PeachpaymentsRouterData<&PaymentsAuthorizeRouterData>,
        CardDetailsForNetworkTransactionId,
    )> for PeachpaymentsPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, card_details): (
            &PeachpaymentsRouterData<&PaymentsAuthorizeRouterData>,
            CardDetailsForNetworkTransactionId,
        ),
    ) -> Result<Self, Self::Error> {
        let (merchant_information, routing_reference) =
            get_config_data(&item.router_data.connector_meta_data)?;

        let card = CardDetails {
            pan: card_details.card_number.clone(),
            cardholder_name: card_details.card_holder_name.clone().ok_or_else(|| {
                errors::ConnectorError::MissingRequiredField {
                    field_name: "card_holder_name",
                }
            })?,
            expiry_year: Some(card_details.get_card_expiry_year_2_digit()?),
            expiry_month: Some(card_details.card_exp_month.clone()),
            cvv: None,
            eci: None,
        };

        let cof_data = Some(CardOnFileData {
            cof_type: get_cof_type(item),
            source: CofSource::Mit,
            mode: CofMode::Subsequent,
        });

        let trace_id = item
            .router_data
            .request
            .get_optional_network_transaction_id();

        let transaction_link_id = item.router_data.request.get_optional_transaction_link_id();

        let peachpayments_data = get_peachpayments_data(item);

        let ecommerce_data =
            EcommercePaymentOnlyTransactionData::Card(EcommerceCardPaymentOnlyTransactionData {
                merchant_information,
                routing_reference,
                card,
                amount: get_amount_details(item),
                rrn: get_rrn(&peachpayments_data),
                pre_auth_inc_ext_capture_flow: get_transaction_operations(item),
                cof_data,
                trace_id,
                transaction_link_id,
                three_d_s_data: get_three_ds_data(item),
            });

        Ok(Self::Card(PeachpaymentsPaymentsCardRequest {
            charge_method: CHARGE_METHOD.to_string(),
            reference_id: item.router_data.connector_request_reference_id.clone(),
            ecommerce_card_payment_only_transaction_data: ecommerce_data,
            pos_data: None,
            send_date_time: get_send_date_time()?,
        }))
    }
}

impl
    TryFrom<(
        &PeachpaymentsRouterData<&PaymentsAuthorizeRouterData>,
        NetworkTokenDetailsForNetworkTransactionId,
    )> for PeachpaymentsPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, token_details): (
            &PeachpaymentsRouterData<&PaymentsAuthorizeRouterData>,
            NetworkTokenDetailsForNetworkTransactionId,
        ),
    ) -> Result<Self, Self::Error> {
        let (merchant_information, routing_reference) =
            get_config_data(&item.router_data.connector_meta_data)?;

        let network_token_data = NetworkTokenDetails {
            token: token_details.get_network_token(),
            expiry_year: token_details.get_token_expiry_year_2_digit()?,
            expiry_month: token_details.get_network_token_expiry_month(),
            cryptogram: token_details.get_cryptogram(),
            eci: token_details.eci.clone(),
            scheme: Some(CardNetworkLowercase::from(
                token_details.card_network.clone().ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "card_network",
                    },
                )?,
            )),
        };

        let trace_id = item
            .router_data
            .request
            .get_optional_network_transaction_id();

        let transaction_link_id = item.router_data.request.get_optional_transaction_link_id();

        let peachpayments_data = get_peachpayments_data(item);

        let ecommerce_data = EcommercePaymentOnlyTransactionData::NetworkToken(
            EcommerceNetworkTokenPaymentOnlyTransactionData {
                merchant_information,
                routing_reference,
                network_token_data,
                amount: get_amount_details(item),
                cof_data: CardOnFileData {
                    cof_type: get_cof_type(item),
                    source: CofSource::Mit,
                    mode: CofMode::Subsequent,
                },
                rrn: get_rrn(&peachpayments_data),
                pre_auth_inc_ext_capture_flow: get_transaction_operations(item),
                trace_id,
                transaction_link_id,
                three_d_s_data: get_three_ds_data(item),
            },
        );

        Ok(Self::NetworkToken(PeachpaymentsPaymentsNTRequest {
            payment_method: CHARGE_METHOD.to_string(),
            reference_id: item.router_data.connector_request_reference_id.clone(),
            ecommerce_card_payment_only_transaction_data: ecommerce_data,
            send_date_time: get_send_date_time()?,
        }))
    }
}

fn get_config_data(
    metadata: &Option<pii::SecretSerdeValue>,
) -> Result<(MerchantInformation, RoutingReference), error_stack::Report<errors::ConnectorError>> {
    let connector_merchant_config = PeachPaymentsConnectorMetadataObject::try_from(metadata)?;

    let merchant_information = MerchantInformation {
        client_merchant_reference_id: connector_merchant_config.client_merchant_reference_id,
    };

    let routing_reference = RoutingReference {
        merchant_payment_method_route_id: connector_merchant_config
            .merchant_payment_method_route_id,
    };

    Ok((merchant_information, routing_reference))
}

fn get_amount_details(
    item: &PeachpaymentsRouterData<&PaymentsAuthorizeRouterData>,
) -> AmountDetails {
    AmountDetails {
        amount: item.amount,
        currency_code: item.router_data.request.currency,
        display_amount: None,
    }
}

fn get_peachpayments_data(
    item: &PeachpaymentsRouterData<&PaymentsAuthorizeRouterData>,
) -> Option<api_models::payments::PeachpaymentsData> {
    item.router_data
        .request
        .connector_intent_metadata
        .as_ref()
        .and_then(|metadata| metadata.peachpayments.clone())
}

fn get_rrn(peachpayments_data: &Option<api_models::payments::PeachpaymentsData>) -> Option<String> {
    peachpayments_data
        .as_ref()
        .and_then(|peachpayments| peachpayments.rrn.clone())
}

fn get_cof_type(item: &PeachpaymentsRouterData<&PaymentsAuthorizeRouterData>) -> CofType {
    match item.router_data.request.mit_category.as_ref() {
        Some(MitCategory::Recurring) => CofType::Recurring,
        _ => CofType::Adhoc,
    }
}

fn get_transaction_operations(
    item: &PeachpaymentsRouterData<&PaymentsAuthorizeRouterData>,
) -> Option<PreAuthIncExtCaptureFlow> {
    if matches!(
        item.router_data.request.capture_method,
        Some(common_enums::CaptureMethod::Manual)
    ) {
        Some(PreAuthIncExtCaptureFlow {
            dcc_mode: DccMode::NoDcc,
            txn_ref_nr: item.router_data.connector_request_reference_id.clone(),
        })
    } else {
        None
    }
}

fn get_send_date_time() -> Result<String, errors::ConnectorError> {
    OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Iso8601::DEFAULT)
        .map_err(|_| errors::ConnectorError::RequestEncodingFailed)
}

fn get_three_ds_data(
    item: &PeachpaymentsRouterData<&PaymentsAuthorizeRouterData>,
) -> Option<PeachpaymentsThreeDSData> {
    item.router_data
        .request
        .authentication_data
        .as_ref()
        .map(|authentication_data| PeachpaymentsThreeDSData {
            cavv: Some(authentication_data.cavv.clone()),
            ds_trans_id: authentication_data.ds_trans_id.clone(),
            three_d_s_version: authentication_data
                .message_version
                .clone()
                .map(|version| format!("{}.{}", version.get_major(), version.get_minor(),)),
            eci: authentication_data.eci.clone(),
            authentication_status: authentication_data.transaction_status.clone(),
        })
}

// Auth Struct for Card Gateway API
pub struct PeachpaymentsAuthType {
    pub(crate) api_key: Secret<String>,
    pub(crate) tenant_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for PeachpaymentsAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let ConnectorAuthType::BodyKey { api_key, key1 } = auth_type {
            Ok(Self {
                api_key: api_key.clone(),
                tenant_id: key1.clone(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}
// Card Gateway API Response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PeachpaymentsPaymentStatus {
    Successful,
    Pending,
    Authorized,
    Approved,
    ApprovedConfirmed,
    Declined,
    Failed,
    Reversed,
    ThreedsRequired,
    Voided,
    FailedRetry,
}

fn get_attempt_status(
    item: PeachpaymentsPaymentStatus,
    is_account_service_inquiry_flow: bool,
) -> Result<common_enums::AttemptStatus, errors::ConnectorError> {
    match item {
        // PENDING means authorized but not yet captured - requires confirmation
        PeachpaymentsPaymentStatus::Pending
        | PeachpaymentsPaymentStatus::Authorized
        | PeachpaymentsPaymentStatus::Approved => {
            if is_account_service_inquiry_flow {
                Ok(common_enums::AttemptStatus::Charged)
            } else {
                Ok(common_enums::AttemptStatus::Authorized)
            }
        }
        PeachpaymentsPaymentStatus::Declined | PeachpaymentsPaymentStatus::Failed => {
            Ok(common_enums::AttemptStatus::Failure)
        }
        PeachpaymentsPaymentStatus::Voided | PeachpaymentsPaymentStatus::Reversed => {
            Ok(common_enums::AttemptStatus::Voided)
        }
        PeachpaymentsPaymentStatus::ThreedsRequired => {
            Ok(common_enums::AttemptStatus::AuthenticationPending)
        }
        PeachpaymentsPaymentStatus::ApprovedConfirmed | PeachpaymentsPaymentStatus::Successful => {
            Ok(common_enums::AttemptStatus::Charged)
        }
        PeachpaymentsPaymentStatus::FailedRetry => Err(
            errors::ConnectorError::UnexpectedResponseError(bytes::Bytes::from(
                "Received FailedRetry status from PeachPayments in 2xx response",
            )),
        ),
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PeachpaymentsPaymentsResponse {
    Response(Box<PeachpaymentsPaymentsData>),
    WebhookResponse(Box<PeachpaymentsIncomingWebhook>),
}

impl From<PeachpaymentsRefundStatus> for common_enums::RefundStatus {
    fn from(item: PeachpaymentsRefundStatus) -> Self {
        match item {
            PeachpaymentsRefundStatus::ApprovedConfirmed => Self::Success,
            PeachpaymentsRefundStatus::Failed | PeachpaymentsRefundStatus::Declined => {
                Self::Failure
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PeachpaymentsPaymentsData {
    pub transaction_id: String,
    pub reference_id: String,
    pub response_code: Option<ResponseCode>,
    pub transaction_result: PeachpaymentsPaymentStatus,
    pub ecommerce_card_payment_only_transaction_data: Option<EcommerceCardPaymentOnlyResponseData>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeachpaymentsRsyncResponse {
    pub transaction_id: String,
    pub reference_id: String,
    pub transaction_result: PeachpaymentsRefundStatus,
    pub response_code: Option<ResponseCode>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeachpaymentsRefundResponse {
    pub transaction_id: String,
    pub original_transaction_id: Option<String>,
    pub reference_id: String,
    pub transaction_result: PeachpaymentsRefundStatus,
    pub response_code: Option<ResponseCode>,
    pub refund_balance_data: Option<RefundBalanceData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PeachpaymentsRefundStatus {
    ApprovedConfirmed,
    Declined,
    Failed,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RefundBalanceData {
    pub amount: AmountDetails,
    pub balance: AmountDetails,
    pub refund_history: Vec<RefundHistory>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RefundHistory {
    pub transaction_id: String,
    pub reference_id: String,
    pub amount: AmountDetails,
}

impl<F>
    TryFrom<ResponseRouterData<F, PeachpaymentsRefundResponse, RefundsData, RefundsResponseData>>
    for RouterData<F, RefundsData, RefundsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PeachpaymentsRefundResponse, RefundsData, RefundsResponseData>,
    ) -> Result<Self, Self::Error> {
        let refund_status = common_enums::RefundStatus::from(item.response.transaction_result);
        let response = if refund_status == storage_enums::RefundStatus::Failure {
            Err(ErrorResponse {
                code: get_error_code(item.response.response_code.as_ref()),
                message: get_error_message(item.response.response_code.as_ref()),
                reason: None,
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: Some(item.response.transaction_id),
                connector_response_reference_id: Some(item.response.reference_id),
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            Ok(RefundsResponseData {
                connector_refund_id: item.response.transaction_id,
                refund_status,
            })
        };
        Ok(Self {
            response,
            ..item.data
        })
    }
}

impl
    TryFrom<ResponseRouterData<RSync, PeachpaymentsRsyncResponse, RefundsData, RefundsResponseData>>
    for RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            RSync,
            PeachpaymentsRsyncResponse,
            RefundsData,
            RefundsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let refund_status = item.response.transaction_result.into();
        let response = if refund_status == storage_enums::RefundStatus::Failure {
            Err(ErrorResponse {
                code: get_error_code(item.response.response_code.as_ref()),
                message: get_error_message(item.response.response_code.as_ref()),
                reason: None,
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: None,
                connector_response_reference_id: Some(item.response.reference_id),
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            Ok(RefundsResponseData {
                connector_refund_id: item.response.transaction_id,
                refund_status,
            })
        };

        Ok(Self {
            response,
            ..item.data
        })
    }
}

// Confirm Transaction Response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PeachpaymentsCaptureResponse {
    pub transaction_id: String,
    pub reference_id: String,
    pub response_code: Option<ResponseCode>,
    pub transaction_result: PeachpaymentsPaymentStatus,
    pub authorization_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum ResponseCode {
    Text(String),
    Structured {
        value: String,
        description: String,
        terminal_outcome_string: Option<String>,
        receipt_string: Option<String>,
        iso_code_description: Option<String>,
        explanation: Option<String>,
    },
}

impl ResponseCode {
    pub fn value(&self) -> Option<&String> {
        match self {
            Self::Structured { value, .. } => Some(value),
            _ => None,
        }
    }

    pub fn description(&self) -> Option<&String> {
        match self {
            Self::Structured { description, .. } => Some(description),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&String> {
        match self {
            Self::Text(s) => Some(s),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EcommerceCardPaymentOnlyResponseData {
    pub amount: Option<AmountDetails>,
    pub stan: Option<Secret<String>>,
    pub rrn: Option<String>,
    pub approval_code: Option<String>,
    pub merchant_advice_code: Option<String>,
    pub description: Option<String>,
    pub trace_id: Option<String>,
    pub transaction_link_id: Option<String>,
}

fn get_error_code(response_code: Option<&ResponseCode>) -> String {
    response_code
        .and_then(|code| code.value())
        .map(|val| val.to_string())
        .unwrap_or(
            response_code
                .and_then(|code| code.as_text())
                .map(|text| text.to_string())
                .unwrap_or(NO_ERROR_CODE.to_string()),
        )
}

fn get_error_message(response_code: Option<&ResponseCode>) -> String {
    response_code
        .and_then(|code| code.description())
        .map(|desc| desc.to_string())
        .unwrap_or(
            response_code
                .and_then(|code| code.as_text())
                .map(|text| text.to_string())
                .unwrap_or(NO_ERROR_MESSAGE.to_string()),
        )
}

pub fn get_peachpayments_response(
    response: PeachpaymentsPaymentsData,
    status_code: u16,
    is_account_service_inquiry_flow: bool,
) -> CustomResult<
    (
        storage_enums::AttemptStatus,
        Result<PaymentsResponseData, ErrorResponse>,
    ),
    errors::ConnectorError,
> {
    let status = get_attempt_status(response.transaction_result, is_account_service_inquiry_flow)?;
    let payments_response = if utils::is_payment_failure(status) {
        Err(ErrorResponse {
            code: get_error_code(response.response_code.as_ref()),
            message: get_error_message(response.response_code.as_ref()),
            reason: response
                .ecommerce_card_payment_only_transaction_data
                .and_then(|data| data.description),
            status_code,
            attempt_status: Some(status),
            connector_transaction_id: Some(response.transaction_id),
            connector_response_reference_id: Some(response.reference_id),
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    } else {
        let network_txn_link_id = response
            .ecommerce_card_payment_only_transaction_data
            .as_ref()
            .and_then(|data| data.transaction_link_id.clone());
        Ok(PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(response.transaction_id.clone()),
            redirection_data: Box::new(None),
            mandate_reference: Box::new(None),
            connector_metadata: None,
            network_txn_id: response
                .ecommerce_card_payment_only_transaction_data
                .and_then(|data| data.trace_id),
            network_txn_link_id,
            connector_response_reference_id: Some(response.reference_id),
            incremental_authorization_allowed: None,
            authentication_data: None,
            charges: None,
        })
    };
    Ok((status, payments_response))
}

pub fn get_webhook_response(
    response: PeachpaymentsIncomingWebhook,
    status_code: u16,
    is_account_service_inquiry_flow: bool,
) -> CustomResult<
    (
        storage_enums::AttemptStatus,
        Result<PaymentsResponseData, ErrorResponse>,
    ),
    errors::ConnectorError,
> {
    let transaction = response
        .transaction
        .ok_or(errors::ConnectorError::WebhookResourceObjectNotFound)?;
    let status = get_attempt_status(
        transaction.transaction_result,
        is_account_service_inquiry_flow,
    )?;
    let webhook_response = if utils::is_payment_failure(status) {
        Err(ErrorResponse {
            code: get_error_code(transaction.response_code.as_ref()),
            message: get_error_message(transaction.response_code.as_ref()),
            reason: transaction
                .ecommerce_card_payment_only_transaction_data
                .and_then(|data| data.description),
            status_code,
            attempt_status: Some(status),
            connector_transaction_id: Some(transaction.transaction_id.clone()),
            connector_response_reference_id: Some(transaction.reference_id),
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    } else {
        let network_txn_link_id = transaction
            .ecommerce_card_payment_only_transaction_data
            .as_ref()
            .and_then(|data| data.transaction_link_id.clone());
        Ok(PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(transaction.transaction_id),
            redirection_data: Box::new(None),
            mandate_reference: Box::new(None),
            connector_metadata: None,
            network_txn_id: transaction
                .ecommerce_card_payment_only_transaction_data
                .and_then(|data| data.trace_id),
            network_txn_link_id,
            connector_response_reference_id: Some(transaction.reference_id.clone()),
            incremental_authorization_allowed: None,
            authentication_data: None,
            charges: None,
        })
    };
    Ok((status, webhook_response))
}

impl<F, T>
    ForeignTryFrom<(
        ResponseRouterData<F, PeachpaymentsPaymentsResponse, T, PaymentsResponseData>,
        bool,
    )> for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(
        (item, is_account_service_inquiry_flow): (
            ResponseRouterData<F, PeachpaymentsPaymentsResponse, T, PaymentsResponseData>,
            bool,
        ),
    ) -> Result<Self, Self::Error> {
        let (status, response) = match item.response {
            PeachpaymentsPaymentsResponse::Response(response) => get_peachpayments_response(
                *response,
                item.http_code,
                is_account_service_inquiry_flow,
            )?,
            PeachpaymentsPaymentsResponse::WebhookResponse(response) => {
                get_webhook_response(*response, item.http_code, is_account_service_inquiry_flow)?
            }
        };

        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

// TryFrom implementation for confirm response
impl<F, T> TryFrom<ResponseRouterData<F, PeachpaymentsCaptureResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PeachpaymentsCaptureResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = get_attempt_status(item.response.transaction_result, false)?;

        // Check if it's an error response
        let response = if utils::is_payment_failure(status) {
            Err(ErrorResponse {
                code: get_error_code(item.response.response_code.as_ref()),
                message: get_error_message(item.response.response_code.as_ref()),
                reason: None,
                status_code: item.http_code,
                attempt_status: Some(status),
                connector_transaction_id: Some(item.response.transaction_id.clone()),
                connector_response_reference_id: Some(item.response.reference_id),
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.transaction_id.clone(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: item.response.authorization_code.map(|auth_code| {
                    serde_json::json!({
                        "authorization_code": auth_code
                    })
                }),
                network_txn_id: None,
                network_txn_link_id: None,
                connector_response_reference_id: Some(item.response.reference_id),
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
            })
        };

        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PeachpaymentsIncomingWebhook {
    pub webhook_id: String,
    pub webhook_type: String,
    pub reversal_failure_reason: Option<String>,
    pub transaction: Option<WebhookTransaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WebhookTransaction {
    pub transaction_id: String,
    pub original_transaction_id: Option<String>,
    pub reference_id: String,
    pub transaction_result: PeachpaymentsPaymentStatus,
    pub error_message: Option<String>,
    pub transaction_type: TransactionType,
    pub response_code: Option<ResponseCode>,
    pub ecommerce_card_payment_only_transaction_data: Option<EcommerceCardPaymentOnlyResponseData>,
    pub refund_balance_data: Option<RefundBalanceData>,
    pub payment_method: Secret<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransactionType {
    pub value: i32,
    pub description: String,
}

// Error Response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeachpaymentsErrorResponse {
    pub error_ref: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Peachpayments5xxErrorResponse {
    Standard(PeachpaymentsErrorResponse),
    Detailed(PeachpaymentsDetailedDeclineResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeachpaymentsDetailedDeclineResponse {
    #[serde(flatten)]
    pub response: PeachpaymentsCaptureResponse,
}
