use std::str::FromStr;

use cards::{CardNumber, NetworkToken};
use common_enums::enums as storage_enums;
use common_utils::{errors::CustomResult, pii, types::MinorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{BankTransferData, Card, NetworkTokenData, PaymentMethodData},
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
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{
    types::ResponseRouterData,
    utils::{self, CardData, NetworkTokenData as _, RouterData as OtherRouterData},
};

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

const CHARGE_METHOD: &str = "ecommerce_card_payment_only";
const COF_DATA_TYPE: &str = "adhoc";
const COF_DATA_SOURCE: &str = "cit";
const COF_DATA_MODE: &str = "initial";

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
    pub _type: String,
    pub source: String,
    pub mode: String,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EcommerceCardPaymentOnlyTransactionData {
    pub merchant_information: MerchantInformation,
    pub routing_reference: RoutingReference,
    pub card: CardDetails,
    pub amount: AmountDetails,
    pub rrn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_auth_inc_ext_capture_flow: Option<PreAuthIncExtCaptureFlow>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EcommerceNetworkTokenPaymentOnlyTransactionData {
    pub merchant_information: MerchantInformation,
    pub routing_reference: RoutingReference,
    pub network_token_data: NetworkTokenDetails,
    pub amount: AmountDetails,
    pub cof_data: CardOnFileData,
    pub rrn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_auth_inc_ext_capture_flow: Option<PreAuthIncExtCaptureFlow>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cardholder_name: Option<Secret<String>>,
    pub expiry_year: Secret<String>,
    pub expiry_month: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cvv: Option<Secret<String>>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkTokenDetails {
    pub token: NetworkToken,
    pub expiry_year: Secret<String>,
    pub expiry_month: Secret<String>,
    pub cryptogram: Option<Secret<String>>,
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
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => Self::try_from((item, req_card)),
            PaymentMethodData::NetworkToken(token_data) => Self::try_from((item, token_data)),

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
        let amount_in_cents = item.amount;

        let connector_merchant_config =
            PeachPaymentsConnectorMetadataObject::try_from(&item.router_data.connector_meta_data)?;

        let merchant_information = MerchantInformation {
            client_merchant_reference_id: connector_merchant_config.client_merchant_reference_id,
        };

        let routing_reference = RoutingReference {
            merchant_payment_method_route_id: connector_merchant_config
                .merchant_payment_method_route_id,
        };

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

        let amount = AmountDetails {
            amount: amount_in_cents,
            currency_code: item.router_data.request.currency,
            display_amount: None,
        };

        let pre_auth_inc_ext_capture_flow = if matches!(
            item.router_data.request.capture_method,
            Some(common_enums::CaptureMethod::Manual)
        ) {
            Some(PreAuthIncExtCaptureFlow {
                dcc_mode: DccMode::NoDcc,
                txn_ref_nr: item.router_data.connector_request_reference_id.clone(),
            })
        } else {
            None
        };

        let ecommerce_data = EcommercePaymentOnlyTransactionData::NetworkToken(
            EcommerceNetworkTokenPaymentOnlyTransactionData {
                merchant_information,
                routing_reference,
                network_token_data,
                amount,
                cof_data: CardOnFileData {
                    _type: COF_DATA_TYPE.to_string(),
                    source: COF_DATA_SOURCE.to_string(),
                    mode: COF_DATA_MODE.to_string(),
                },
                rrn: item.router_data.request.merchant_order_reference_id.clone(),
                pre_auth_inc_ext_capture_flow,
            },
        );

        // Generate current timestamp for sendDateTime (ISO 8601 format: YYYY-MM-DDTHH:MM:SSZ)
        let send_date_time = OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Iso8601::DEFAULT)
            .map_err(|_| errors::ConnectorError::RequestEncodingFailed)?;

        Ok(Self::NetworkToken(PeachpaymentsPaymentsNTRequest {
            payment_method: "ecommerce_card_payment_only".to_string(),
            reference_id: item.router_data.connector_request_reference_id.clone(),
            ecommerce_card_payment_only_transaction_data: ecommerce_data,
            send_date_time: send_date_time.clone(),
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
        if item.router_data.is_three_ds() {
            return Err(errors::ConnectorError::NotSupported {
                message: "3DS flow".to_string(),
                connector: "Peachpayments",
            }
            .into());
        }
        let amount_in_cents = item.amount;

        let connector_merchant_config =
            PeachPaymentsConnectorMetadataObject::try_from(&item.router_data.connector_meta_data)?;

        let merchant_information = MerchantInformation {
            client_merchant_reference_id: connector_merchant_config.client_merchant_reference_id,
        };

        let routing_reference = RoutingReference {
            merchant_payment_method_route_id: connector_merchant_config
                .merchant_payment_method_route_id,
        };

        let card = CardDetails {
            pan: req_card.card_number.clone(),
            cardholder_name: req_card.card_holder_name.clone(),
            expiry_year: req_card.get_card_expiry_year_2_digit()?,
            expiry_month: req_card.card_exp_month.clone(),
            cvv: Some(req_card.card_cvc.clone()),
        };

        let amount = AmountDetails {
            amount: amount_in_cents,
            currency_code: item.router_data.request.currency,
            display_amount: None,
        };

        let pre_auth_inc_ext_capture_flow = if matches!(
            item.router_data.request.capture_method,
            Some(common_enums::CaptureMethod::Manual)
        ) {
            Some(PreAuthIncExtCaptureFlow {
                dcc_mode: DccMode::NoDcc,
                txn_ref_nr: item.router_data.connector_request_reference_id.clone(),
            })
        } else {
            None
        };

        let ecommerce_data =
            EcommercePaymentOnlyTransactionData::Card(EcommerceCardPaymentOnlyTransactionData {
                merchant_information,
                routing_reference,
                card,
                amount,
                rrn: item.router_data.request.merchant_order_reference_id.clone(),
                pre_auth_inc_ext_capture_flow,
            });

        // Generate current timestamp for sendDateTime (ISO 8601 format: YYYY-MM-DDTHH:MM:SSZ)
        let send_date_time = OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Iso8601::DEFAULT)
            .map_err(|_| errors::ConnectorError::RequestEncodingFailed)?;

        Ok(Self::Card(PeachpaymentsPaymentsCardRequest {
            charge_method: CHARGE_METHOD.to_string(),
            reference_id: item.router_data.connector_request_reference_id.clone(),
            ecommerce_card_payment_only_transaction_data: ecommerce_data,
            pos_data: None,
            send_date_time,
        }))
    }
}

/// Extract payment brand and bank from bank_code
/// Format: "PAYSHAP:NEDBANK" -> ("PAYSHAP", Some("NEDBANK"))
/// Format: "PEACHEFT" -> ("PEACHEFT", None)
fn extract_payment_brand_and_bank(bank_code: &str) -> (String, Option<String>) {
    if let Some((brand, bank)) = bank_code.split_once(':') {
        (brand.to_uppercase(), Some(bank.to_uppercase()))
    } else {
        (bank_code.to_uppercase(), None)
    }
}

// TryFrom for APM (Bank Transfer) requests
impl TryFrom<&PeachpaymentsRouterData<&PaymentsAuthorizeRouterData>>
    for PeachpaymentsApmPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PeachpaymentsRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth = PeachpaymentsApmAuthType::try_from(&item.router_data.connector_auth_type)?;

        // Convert amount from minor units to decimal string (e.g., 10000 -> "100.00")
        let amount_decimal = item.amount.get_amount_as_i64() as f64 / 100.0;
        let amount = format!("{:.2}", amount_decimal);

        let return_url = item.router_data.request.router_return_url.clone();

        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::BankTransfer(bank_transfer) => match *bank_transfer {
                BankTransferData::LocalBankTransfer { bank_code } => {
                    let bank_code_str = bank_code.unwrap_or_default();
                    let (payment_brand, bank) = extract_payment_brand_and_bank(&bank_code_str);

                    let virtual_account = build_virtual_account(
                        &payment_brand,
                        bank,
                        item.router_data,
                    );

                    let is_non_production_env = !matches!(
                        router_env::env::which(),
                        router_env::Env::Production
                    );
                    let is_test_mode = item.router_data.test_mode.unwrap_or(false);
                    let custom_parameters = if is_non_production_env || is_test_mode {
                        Some(PeachpaymentsCustomParameters {
                            enable_test_mode: Some("true".to_string()),
                        })
                    } else {
                        None
                    };

                    Ok(Self {
                        authentication: PeachpaymentsApmAuthentication {
                            user_id: auth.user_id,
                            password: auth.password,
                            entity_id: auth.entity_id,
                        },
                        merchant_transaction_id: item
                            .router_data
                            .connector_request_reference_id
                            .clone(),
                        amount,
                        currency: item.router_data.request.currency,
                        payment_brand,
                        payment_type: "DB".to_string(),
                        shopper_result_url: return_url,
                        virtual_account,
                        custom_parameters,
                    })
                }
                _ => Err(errors::ConnectorError::NotImplemented(
                    "Bank transfer type not supported".to_string(),
                )
                .into()),
            },
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment method not supported for APM".to_string(),
            )
            .into()),
        }
    }
}

/// Build virtual account based on payment brand
fn build_virtual_account(
    payment_brand: &str,
    bank: Option<String>,
    router_data: &PaymentsAuthorizeRouterData,
) -> Option<PeachpaymentsVirtualAccount> {
    // Get phone number as-is from billing - let PeachPayments validate format
    let phone = router_data
        .get_billing_phone_number()
        .ok()
        .map(|p| p.expose().to_string());

    // Get email from billing
    let email = router_data
        .get_billing_email()
        .ok()
        .map(|e| e.peek().to_string());

    match payment_brand {
        // Phone-based payments
        "PAYSHAP" => Some(PeachpaymentsVirtualAccount {
            bank,
            account_type: Some("CELLPHONE".to_string()),
            account_id: phone,
            password: None,
        }),
        "CAPITECPAY" => Some(PeachpaymentsVirtualAccount {
            bank: None,
            account_type: Some("CELLPHONE".to_string()),
            account_id: phone,
            password: None,
        }),
        "MPESA" | "BLINKBYEMTEL" | "MCBJUICE" => Some(PeachpaymentsVirtualAccount {
            bank: None,
            account_type: None,
            account_id: phone,
            password: None,
        }),
        // Email-based payments (vouchers, store credit)
        "MOBICRED" | "1VOUCHER" | "1FORYOU" | "RCS" | "APLUS" => {
            Some(PeachpaymentsVirtualAccount {
                bank: None,
                account_type: None,
                account_id: email,
                password: None, // Voucher PIN would come from payment_method_data if needed
            })
        }
        // PEACHEFT, PAYFLEX, ZEROPAY, FLOAT, HAPPYPAY, MASTERPASS, MONEYBADGER don't require virtual account
        _ => None,
    }
}

// TryFrom for APM refund requests
impl TryFrom<&RefundsRouterData<Execute>> for PeachpaymentsApmRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RefundsRouterData<Execute>) -> Result<Self, Self::Error> {
        let auth = PeachpaymentsApmAuthType::try_from(&item.connector_auth_type)?;

        // Convert amount from minor units to decimal string
        let amount_decimal = item.request.minor_refund_amount.get_amount_as_i64() as f64 / 100.0;
        let amount = format!("{:.2}", amount_decimal);

        Ok(Self {
            authentication: PeachpaymentsApmAuthentication {
                user_id: auth.user_id,
                password: auth.password,
                entity_id: auth.entity_id,
            },
            amount,
            currency: item.request.currency,
            payment_type: "RF".to_string(),
        })
    }
}

// Auth Struct for Card Gateway API
// Note: Can accept either BodyKey (card-only) or SignatureKey (card + APM)
// When SignatureKey is used: api_key=x-api-key header, key1=x-tenant-id header
// The api_secret is only used for APM and ignored for card payments
pub struct PeachpaymentsAuthType {
    pub(crate) api_key: Secret<String>,
    pub(crate) tenant_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for PeachpaymentsAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.clone(),
                tenant_id: key1.clone(),
            }),
            // Also accept SignatureKey for combined card + APM setup
            ConnectorAuthType::SignatureKey { api_key, key1, .. } => Ok(Self {
                api_key: api_key.clone(),
                tenant_id: key1.clone(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType)?,
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
}

impl From<PeachpaymentsPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: PeachpaymentsPaymentStatus) -> Self {
        match item {
            // PENDING means authorized but not yet captured - requires confirmation
            PeachpaymentsPaymentStatus::Pending
            | PeachpaymentsPaymentStatus::Authorized
            | PeachpaymentsPaymentStatus::Approved => Self::Authorized,
            PeachpaymentsPaymentStatus::Declined | PeachpaymentsPaymentStatus::Failed => {
                Self::Failure
            }
            PeachpaymentsPaymentStatus::Voided | PeachpaymentsPaymentStatus::Reversed => {
                Self::Voided
            }
            PeachpaymentsPaymentStatus::ThreedsRequired => Self::AuthenticationPending,
            PeachpaymentsPaymentStatus::ApprovedConfirmed
            | PeachpaymentsPaymentStatus::Successful => Self::Charged,
        }
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
    pub response_code: Option<ResponseCode>,
    pub transaction_result: PeachpaymentsPaymentStatus,
    pub ecommerce_card_payment_only_transaction_data: Option<EcommerceCardPaymentOnlyResponseData>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeachpaymentsRsyncResponse {
    pub transaction_id: String,
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

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundBalanceData {
    pub amount: AmountDetails,
    pub balance: AmountDetails,
    pub refund_history: Vec<RefundHistory>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
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
                connector_response_reference_id: None,
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
                connector_transaction_id: Some(item.response.transaction_id),
                connector_response_reference_id: None,
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
    pub rrn: Option<Secret<String>>,
    pub approval_code: Option<String>,
    pub merchant_advice_code: Option<String>,
    pub description: Option<String>,
    pub trace_id: Option<String>,
}

fn is_payment_success(value: Option<&String>) -> bool {
    if let Some(val) = value {
        val == "00" || val == "08" || val == "X94"
    } else {
        false
    }
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
) -> CustomResult<
    (
        storage_enums::AttemptStatus,
        Result<PaymentsResponseData, ErrorResponse>,
    ),
    errors::ConnectorError,
> {
    let status = common_enums::AttemptStatus::from(response.transaction_result);
    let payments_response = if !is_payment_success(
        response
            .response_code
            .as_ref()
            .and_then(|code| code.value()),
    ) {
        Err(ErrorResponse {
            code: get_error_code(response.response_code.as_ref()),
            message: get_error_message(response.response_code.as_ref()),
            reason: response
                .ecommerce_card_payment_only_transaction_data
                .and_then(|data| data.description),
            status_code,
            attempt_status: Some(status),
            connector_transaction_id: Some(response.transaction_id.clone()),
            connector_response_reference_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    } else {
        Ok(PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(response.transaction_id.clone()),
            redirection_data: Box::new(None),
            mandate_reference: Box::new(None),
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: Some(response.transaction_id),
            incremental_authorization_allowed: None,
            charges: None,
        })
    };
    Ok((status, payments_response))
}

pub fn get_webhook_response(
    response: PeachpaymentsIncomingWebhook,
    status_code: u16,
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
    let status = common_enums::AttemptStatus::from(transaction.transaction_result);
    let webhook_response = if !is_payment_success(
        transaction
            .response_code
            .as_ref()
            .and_then(|code| code.value()),
    ) {
        Err(ErrorResponse {
            code: get_error_code(transaction.response_code.as_ref()),
            message: get_error_message(transaction.response_code.as_ref()),
            reason: transaction
                .ecommerce_card_payment_only_transaction_data
                .and_then(|data| data.description),
            status_code,
            attempt_status: Some(status),
            connector_transaction_id: Some(transaction.transaction_id.clone()),
            connector_response_reference_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    } else {
        Ok(PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(
                transaction
                    .original_transaction_id
                    .unwrap_or(transaction.transaction_id.clone()),
            ),
            redirection_data: Box::new(None),
            mandate_reference: Box::new(None),
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: Some(transaction.transaction_id.clone()),
            incremental_authorization_allowed: None,
            charges: None,
        })
    };
    Ok((status, webhook_response))
}

impl<F, T> TryFrom<ResponseRouterData<F, PeachpaymentsPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PeachpaymentsPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let (status, response) = match item.response {
            PeachpaymentsPaymentsResponse::Response(response) => {
                get_peachpayments_response(*response, item.http_code)?
            }
            PeachpaymentsPaymentsResponse::WebhookResponse(response) => {
                get_webhook_response(*response, item.http_code)?
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
        let status = common_enums::AttemptStatus::from(item.response.transaction_result);

        // Check if it's an error response
        let response = if !is_payment_success(
            item.response
                .response_code
                .as_ref()
                .and_then(|code| code.value()),
        ) {
            Err(ErrorResponse {
                code: get_error_code(item.response.response_code.as_ref()),
                message: get_error_message(item.response.response_code.as_ref()),
                reason: None,
                status_code: item.http_code,
                attempt_status: Some(status),
                connector_transaction_id: Some(item.response.transaction_id.clone()),
                connector_response_reference_id: None,
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
                connector_response_reference_id: Some(item.response.transaction_id),
                incremental_authorization_allowed: None,
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
    pub response_code: Option<ResponseCode>,
    pub ecommerce_card_payment_only_transaction_data: Option<EcommerceCardPaymentOnlyResponseData>,
    pub payment_method: Secret<String>,
}

// Error Response for Card Gateway API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeachpaymentsErrorResponse {
    pub error_ref: String,
    pub message: String,
}

impl TryFrom<ErrorResponse> for PeachpaymentsErrorResponse {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(error_response: ErrorResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            error_ref: error_response.code,
            message: error_response.message,
        })
    }
}

/// Error Response for APM API - simpler format with just message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeachpaymentsApmErrorResponse {
    pub message: String,
    #[serde(default)]
    pub result: Option<ApmResultCode>,
}

impl From<PeachpaymentsApmErrorResponse> for ErrorResponse {
    fn from(error: PeachpaymentsApmErrorResponse) -> Self {
        let (code, message) = if let Some(ref result) = error.result {
            (result.code.clone(), result.description.clone())
        } else {
            ("APM_ERROR".to_string(), error.message.clone())
        };

        // Determine attempt_status based on result code
        // Error codes (100.xxx, 200.xxx, 800.xxx, 900.xxx) should mark payment as failed
        let attempt_status = error.result.as_ref().map(|r| {
            if r.code.starts_with("100.")
                || r.code.starts_with("200.")
                || r.code.starts_with("800.")
                || r.code.starts_with("900.")
            {
                common_enums::AttemptStatus::Failure
            } else {
                // For other error codes, keep pending to allow retry
                common_enums::AttemptStatus::Pending
            }
        });

        ErrorResponse {
            status_code: 400,
            code,
            message: message.clone(),
            reason: Some(message),
            attempt_status,
            connector_transaction_id: None,
            connector_response_reference_id: None,
            network_decline_code: None,
            network_advice_code: None,
            network_error_message: None,
            connector_metadata: None,
        }
    }
}

// ============================================================================
// APM (Payments API v2) Types - Form-encoded requests with different auth
// ============================================================================

/// Auth for APM Payments API - uses SignatureKey
/// api_key = entityId
/// key1 = userId
/// api_secret = password
pub struct PeachpaymentsApmAuthType {
    pub entity_id: Secret<String>,
    pub user_id: Secret<String>,
    pub password: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for PeachpaymentsApmAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } = auth_type
        {
            Ok(Self {
                entity_id: api_key.clone(),
                user_id: key1.clone(),
                password: api_secret.clone(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

/// APM authentication object for JSON request
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeachpaymentsApmAuthentication {
    pub user_id: Secret<String>,
    pub password: Secret<String>,
    pub entity_id: Secret<String>,
}

/// Virtual account details for APM payments
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeachpaymentsVirtualAccount {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub account_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<Secret<String>>,
}

/// Custom parameters for APM requests (e.g., test mode)
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeachpaymentsCustomParameters {
    /// Enable test mode for sandbox testing - required for many APMs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_test_mode: Option<String>,
}

/// APM payment request - JSON format
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeachpaymentsApmPaymentsRequest {
    pub authentication: PeachpaymentsApmAuthentication,
    pub merchant_transaction_id: String,
    pub amount: String,
    pub currency: common_enums::Currency,
    pub payment_brand: String,
    pub payment_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shopper_result_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtual_account: Option<PeachpaymentsVirtualAccount>,
    /// Custom parameters - includes enableTestMode for sandbox
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_parameters: Option<PeachpaymentsCustomParameters>,
}

/// APM refund request - JSON format
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeachpaymentsApmRefundRequest {
    pub authentication: PeachpaymentsApmAuthentication,
    pub amount: String,
    pub currency: common_enums::Currency,
    pub payment_type: String,
}

/// APM response with result code and optional redirect
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PeachpaymentsApmPaymentsResponse {
    pub id: String,
    pub result: ApmResultCode,
    #[serde(default)]
    pub redirect: Option<ApmRedirect>,
    #[serde(rename = "merchantTransactionId")]
    pub merchant_transaction_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApmResultCode {
    pub code: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApmRedirect {
    pub url: String,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub parameters: Vec<ApmRedirectParameter>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApmRedirectParameter {
    pub name: String,
    pub value: String,
}

/// APM Sync response
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeachpaymentsApmSyncResponse {
    pub id: String,
    pub result: ApmResultCode,
    pub payment_type: Option<String>,
    pub payment_brand: Option<String>,
    /// Amount comes back as a float from the API (e.g., 100.00)
    pub amount: Option<f64>,
    pub currency: Option<common_enums::Currency>,
    pub merchant_transaction_id: Option<String>,
}

/// APM webhook body
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PeachpaymentsApmWebhookBody {
    pub id: String,
    pub result: ApmResultCode,
    #[serde(rename = "paymentType")]
    pub payment_type: Option<String>,
    #[serde(rename = "merchantTransactionId")]
    pub merchant_transaction_id: Option<String>,
}

/// Helper function to map APM result codes to AttemptStatus
pub fn map_apm_result_code_to_status(code: &str) -> common_enums::AttemptStatus {
    match code {
        // Success codes
        c if c.starts_with("000.000") => common_enums::AttemptStatus::Charged,
        c if c.starts_with("000.100") => common_enums::AttemptStatus::Charged,
        c if c.starts_with("000.300") => common_enums::AttemptStatus::Charged,
        // Pending/redirect codes
        c if c.starts_with("000.200") => common_enums::AttemptStatus::AuthenticationPending,
        // 3DS success
        c if c.starts_with("000.400.1") => common_enums::AttemptStatus::Charged,
        // Failure codes
        c if c.starts_with("100.") => common_enums::AttemptStatus::Failure,
        c if c.starts_with("200.") => common_enums::AttemptStatus::Failure,
        c if c.starts_with("800.") => common_enums::AttemptStatus::Failure,
        c if c.starts_with("900.") => common_enums::AttemptStatus::Failure,
        // Default to pending for unknown codes
        _ => common_enums::AttemptStatus::Pending,
    }
}

/// Helper function to check if result code indicates success
pub fn is_apm_success(code: &str) -> bool {
    code.starts_with("000.000")
        || code.starts_with("000.100")
        || code.starts_with("000.300")
        || code.starts_with("000.400.1")
}

/// Helper function to check if result code indicates pending (redirect)
pub fn is_apm_pending(code: &str) -> bool {
    code.starts_with("000.200")
}

/// Helper function to check if payment method data is APM (bank transfer)
pub fn is_apm_payment_method(payment_method_data: &PaymentMethodData) -> bool {
    matches!(
        payment_method_data,
        PaymentMethodData::BankTransfer(bank_transfer) if matches!(
            **bank_transfer,
            BankTransferData::LocalBankTransfer { .. }
        )
    )
}

// TryFrom implementation for APM authorize response
impl<F, T> TryFrom<ResponseRouterData<F, PeachpaymentsApmPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PeachpaymentsApmPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = map_apm_result_code_to_status(&item.response.result.code);

        let response = if is_apm_success(&item.response.result.code) {
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: item.response.merchant_transaction_id,
                incremental_authorization_allowed: None,
                charges: None,
            })
        } else if is_apm_pending(&item.response.result.code) {
            // Redirect flow - return redirect URL with parameters
            let redirect_data = item.response.redirect.map(|r| {
                // Convert parameters to form fields
                let form_fields: std::collections::HashMap<String, String> = r
                    .parameters
                    .into_iter()
                    .map(|p| (p.name, p.value))
                    .collect();

                // Use the method from the response (POST or GET)
                let method = match r.method.as_deref() {
                    Some("POST") => common_utils::request::Method::Post,
                    _ => common_utils::request::Method::Get,
                };

                hyperswitch_domain_models::router_response_types::RedirectForm::Form {
                    endpoint: r.url,
                    method,
                    form_fields,
                }
            });
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: Box::new(redirect_data),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: item.response.merchant_transaction_id,
                incremental_authorization_allowed: None,
                charges: None,
            })
        } else {
            Err(ErrorResponse {
                code: item.response.result.code.clone(),
                message: item.response.result.description.clone(),
                reason: Some(item.response.result.description),
                status_code: item.http_code,
                attempt_status: Some(status),
                connector_transaction_id: Some(item.response.id),
                connector_response_reference_id: None,
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        };

        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

// TryFrom implementation for APM sync response
impl<F, T> TryFrom<ResponseRouterData<F, PeachpaymentsApmSyncResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PeachpaymentsApmSyncResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = map_apm_result_code_to_status(&item.response.result.code);

        let response = if is_apm_success(&item.response.result.code) {
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.id),
                incremental_authorization_allowed: None,
                charges: None,
            })
        } else if is_apm_pending(&item.response.result.code) {
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.id),
                incremental_authorization_allowed: None,
                charges: None,
            })
        } else {
            Err(ErrorResponse {
                code: item.response.result.code.clone(),
                message: item.response.result.description.clone(),
                reason: Some(item.response.result.description),
                status_code: item.http_code,
                attempt_status: Some(status),
                connector_transaction_id: Some(item.response.id),
                connector_response_reference_id: None,
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        };

        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

// ============================================================================
// APM Refund Response Types
// ============================================================================

/// APM refund response - same format as payment response
pub type PeachpaymentsApmRefundResponse = PeachpaymentsApmPaymentsResponse;

/// APM refund sync response - same format as payment sync response
pub type PeachpaymentsApmRefundSyncResponse = PeachpaymentsApmSyncResponse;

/// Helper function to map APM result codes to RefundStatus
pub fn map_apm_result_code_to_refund_status(code: &str) -> storage_enums::RefundStatus {
    if is_apm_success(code) {
        storage_enums::RefundStatus::Success
    } else if is_apm_pending(code) {
        storage_enums::RefundStatus::Pending
    } else {
        storage_enums::RefundStatus::Failure
    }
}

// TryFrom implementation for APM refund response
impl<F> TryFrom<ResponseRouterData<F, PeachpaymentsApmRefundResponse, RefundsData, RefundsResponseData>>
    for RouterData<F, RefundsData, RefundsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PeachpaymentsApmRefundResponse, RefundsData, RefundsResponseData>,
    ) -> Result<Self, Self::Error> {
        let refund_status = map_apm_result_code_to_refund_status(&item.response.result.code);

        let response = if refund_status == storage_enums::RefundStatus::Failure {
            Err(ErrorResponse {
                code: item.response.result.code.clone(),
                message: item.response.result.description.clone(),
                reason: Some(item.response.result.description),
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: Some(item.response.id),
                connector_response_reference_id: None,
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            Ok(RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status,
            })
        };

        Ok(Self {
            response,
            ..item.data
        })
    }
}

// TryFrom implementation for APM refund sync response
impl<F> TryFrom<ResponseRouterData<F, PeachpaymentsApmRefundSyncResponse, RefundsData, RefundsResponseData>>
    for RouterData<F, RefundsData, RefundsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PeachpaymentsApmRefundSyncResponse, RefundsData, RefundsResponseData>,
    ) -> Result<Self, Self::Error> {
        let refund_status = map_apm_result_code_to_refund_status(&item.response.result.code);

        let response = if refund_status == storage_enums::RefundStatus::Failure {
            Err(ErrorResponse {
                code: item.response.result.code.clone(),
                message: item.response.result.description.clone(),
                reason: Some(item.response.result.description),
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: Some(item.response.id),
                connector_response_reference_id: None,
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            Ok(RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status,
            })
        };

        Ok(Self {
            response,
            ..item.data
        })
    }
}
