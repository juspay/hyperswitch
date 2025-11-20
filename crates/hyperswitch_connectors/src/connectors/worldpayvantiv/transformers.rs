use common_utils::{
    ext_traits::Encode,
    pii,
    types::{MinorUnit, StringMajorUnit, StringMinorUnitForConnector},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{
        AdditionalPaymentMethodConnectorResponse, ConnectorAuthType, ConnectorResponseData,
        ErrorResponse, RouterData,
    },
    router_flow_types::{
        refunds::{Execute, RSync},
        Dsync, Fetch, Retrieve, Upload,
    },
    router_request_types::{
        DisputeSyncData, FetchDisputesRequestData, PaymentsAuthorizeData,
        PaymentsCancelPostCaptureData, PaymentsSyncData, ResponseId, RetrieveFileRequestData,
        SetupMandateRequestData, UploadFileRequestData,
    },
    router_response_types::{
        DisputeSyncResponse, FetchDisputesResponse, MandateReference, PaymentsResponseData,
        RefundsResponseData, RetrieveFileResponse, UploadFileResponse,
    },
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelPostCaptureRouterData, PaymentsCancelRouterData,
        PaymentsCaptureRouterData, RefundsRouterData, SetupMandateRouterData,
    },
};
use hyperswitch_interfaces::{consts, errors};
use masking::{ExposeInterface, PeekInterface, Secret};
use router_env::logger;
use serde::{Deserialize, Serialize};

use crate::{
    types::{
        AcceptDisputeRouterData, DisputeSyncRouterData, FetchDisputeRouterData,
        PaymentsCancelResponseRouterData, PaymentsCaptureResponseRouterData,
        RefundsResponseRouterData, ResponseRouterData, RetrieveFileRouterData,
        SubmitEvidenceRouterData,
    },
    utils::{
        self as connector_utils, CardData, PaymentsAuthorizeRequestData,
        PaymentsSetupMandateRequestData, RouterData as UtilsRouterData,
    },
};

pub mod worldpayvantiv_constants {
    pub const WORLDPAYVANTIV_VERSION: &str = "12.23";
    pub const XML_VERSION: &str = "1.0";
    pub const XML_ENCODING: &str = "UTF-8";
    pub const XMLNS: &str = "http://www.vantivcnp.com/schema";
    pub const MAX_PAYMENT_REFERENCE_ID_LENGTH: usize = 28;
    pub const XML_STANDALONE: &str = "yes";
    pub const XML_CHARGEBACK: &str = "http://www.vantivcnp.com/chargebacks";
    pub const MAC_FIELD_NUMBER: &str = "39";
    pub const CUSTOMER_ID_MAX_LENGTH: usize = 50;
    pub const CUSTOMER_REFERENCE_MAX_LENGTH: usize = 17;
}

pub struct WorldpayvantivRouterData<T> {
    pub amount: MinorUnit,
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for WorldpayvantivRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

pub struct WorldpayvantivAuthType {
    pub(super) user: Secret<String>,
    pub(super) password: Secret<String>,
    pub(super) merchant_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for WorldpayvantivAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                api_secret,
                key1,
            } => Ok(Self {
                user: api_key.to_owned(),
                password: api_secret.to_owned(),
                merchant_id: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, strum::Display, Serialize, Deserialize, PartialEq, Clone, Copy)]
#[strum(serialize_all = "lowercase")]
pub enum OperationId {
    Sale,
    Auth,
    Capture,
    Void,
    // VoidPostCapture
    VoidPC,
    Refund,
}

// Represents the payment metadata for Worldpay Vantiv.
// The `report_group` field is an Option<String> to account for cases where the report group might not be provided in the metadata.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct WorldpayvantivPaymentMetadata {
    pub report_group: Option<String>,
    pub fraud_filter_override: Option<bool>,
}

// Represents the merchant connector account metadata for Worldpay Vantiv
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WorldpayvantivMetadataObject {
    pub report_group: String,
    pub merchant_config_currency: common_enums::Currency,
    pub fraud_filter_override: Option<bool>,
}

impl TryFrom<&Option<pii::SecretSerdeValue>> for WorldpayvantivMetadataObject {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: &Option<pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata = connector_utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata",
            })?;
        Ok(metadata)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename = "cnpOnlineRequest", rename_all = "camelCase")]
pub struct CnpOnlineRequest {
    #[serde(rename = "@version")]
    pub version: String,
    #[serde(rename = "@xmlns")]
    pub xmlns: String,
    #[serde(rename = "@merchantId")]
    pub merchant_id: Secret<String>,
    pub authentication: Authentication,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization: Option<Authorization>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sale: Option<Sale>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capture: Option<Capture>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_reversal: Option<AuthReversal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub void: Option<Void>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credit: Option<RefundRequest>,
}

#[derive(Debug, Serialize)]
pub struct Authentication {
    pub user: Secret<String>,
    pub password: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Void {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@reportGroup")]
    pub report_group: String,
    pub cnp_txn_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthReversal {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@reportGroup")]
    pub report_group: String,
    pub cnp_txn_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capture {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@reportGroup")]
    pub report_group: String,
    pub cnp_txn_id: String,
    pub amount: MinorUnit,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VantivAddressData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_line1: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_line2: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_line3: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zip: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<pii::Email>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<common_enums::CountryAlpha2>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<Secret<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum VantivProcessingType {
    InitialCOF,
    MerchantInitiatedCOF,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Authorization {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@reportGroup")]
    pub report_group: String,
    #[serde(rename = "@customerId", skip_serializing_if = "Option::is_none")]
    pub customer_id: Option<String>,
    pub order_id: String,
    pub amount: MinorUnit,
    pub order_source: OrderSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bill_to_address: Option<VantivAddressData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ship_to_address: Option<VantivAddressData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card: Option<WorldpayvantivCardData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<TokenizationData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enhanced_data: Option<EnhancedData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing_type: Option<VantivProcessingType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_network_transaction_id: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_partial_auth: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fraud_filter_override: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cardholder_authentication: Option<CardholderAuthentication>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardholderAuthentication {
    authentication_value: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sale {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@reportGroup")]
    pub report_group: String,
    #[serde(rename = "@customerId", skip_serializing_if = "Option::is_none")]
    pub customer_id: Option<String>,
    pub order_id: String,
    pub amount: MinorUnit,
    pub order_source: OrderSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bill_to_address: Option<VantivAddressData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ship_to_address: Option<VantivAddressData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card: Option<WorldpayvantivCardData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<TokenizationData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enhanced_data: Option<EnhancedData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing_type: Option<VantivProcessingType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_network_transaction_id: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_partial_auth: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fraud_filter_override: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnhancedData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sales_tax: Option<MinorUnit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_exempt: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount_amount: Option<MinorUnit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shipping_amount: Option<MinorUnit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duty_amount: Option<MinorUnit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ship_from_postal_code: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_postal_code: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_country_code: Option<common_enums::CountryAlpha2>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail_tax: Option<DetailTax>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_item_data: Option<Vec<LineItemData>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailTax {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_included_in_total: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_amount: Option<MinorUnit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_acceptor_tax_id: Option<Secret<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LineItemData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_sequence_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_of_measure: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_amount: Option<MinorUnit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_item_total: Option<MinorUnit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_item_total_with_tax: Option<MinorUnit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_discount_amount: Option<MinorUnit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commodity_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_cost: Option<MinorUnit>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundRequest {
    #[serde(rename = "@reportGroup")]
    pub report_group: String,
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@customerId", skip_serializing_if = "Option::is_none")]
    pub customer_id: Option<String>,
    pub cnp_txn_id: String,
    pub amount: MinorUnit,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderSource {
    Ecommerce,
    ApplePay,
    MailOrder,
    Telephone,
    AndroidPay,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenizationData {
    cnp_token: Secret<String>,
    exp_date: Secret<String>,
}

#[derive(Debug)]
struct VantivMandateDetail {
    processing_type: Option<VantivProcessingType>,
    network_transaction_id: Option<Secret<String>>,
    token: Option<TokenizationData>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldpayvantivCardData {
    #[serde(rename = "type")]
    pub card_type: WorldpayvativCardType,
    pub number: cards::CardNumber,
    pub exp_date: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_validation_num: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]

pub enum WorldpayvativCardType {
    #[serde(rename = "VI")]
    Visa,
    #[serde(rename = "MC")]
    MasterCard,
    #[serde(rename = "AX")]
    AmericanExpress,
    #[serde(rename = "DI")]
    Discover,
    #[serde(rename = "DC")]
    DinersClub,
    #[serde(rename = "JC")]
    JCB,
    #[serde(rename = "")]
    UnionPay,
}

#[derive(Debug, Clone, Serialize, strum::EnumString)]
pub enum WorldPayVativApplePayNetwork {
    Visa,
    MasterCard,
    AmEx,
    Discover,
    DinersClub,
    JCB,
    UnionPay,
}

impl From<WorldPayVativApplePayNetwork> for WorldpayvativCardType {
    fn from(network: WorldPayVativApplePayNetwork) -> Self {
        match network {
            WorldPayVativApplePayNetwork::Visa => Self::Visa,
            WorldPayVativApplePayNetwork::MasterCard => Self::MasterCard,
            WorldPayVativApplePayNetwork::AmEx => Self::AmericanExpress,
            WorldPayVativApplePayNetwork::Discover => Self::Discover,
            WorldPayVativApplePayNetwork::DinersClub => Self::DinersClub,
            WorldPayVativApplePayNetwork::JCB => Self::JCB,
            WorldPayVativApplePayNetwork::UnionPay => Self::UnionPay,
        }
    }
}

#[derive(Debug, Clone, Serialize, strum::EnumString)]
#[serde(rename_all = "UPPERCASE")]
#[strum(ascii_case_insensitive)]
pub enum WorldPayVativGooglePayNetwork {
    Visa,
    Mastercard,
    Amex,
    Discover,
    Dinersclub,
    Jcb,
    Unionpay,
}

impl From<WorldPayVativGooglePayNetwork> for WorldpayvativCardType {
    fn from(network: WorldPayVativGooglePayNetwork) -> Self {
        match network {
            WorldPayVativGooglePayNetwork::Visa => Self::Visa,
            WorldPayVativGooglePayNetwork::Mastercard => Self::MasterCard,
            WorldPayVativGooglePayNetwork::Amex => Self::AmericanExpress,
            WorldPayVativGooglePayNetwork::Discover => Self::Discover,
            WorldPayVativGooglePayNetwork::Dinersclub => Self::DinersClub,
            WorldPayVativGooglePayNetwork::Jcb => Self::JCB,
            WorldPayVativGooglePayNetwork::Unionpay => Self::UnionPay,
        }
    }
}
impl TryFrom<common_enums::CardNetwork> for WorldpayvativCardType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(card_network: common_enums::CardNetwork) -> Result<Self, Self::Error> {
        match card_network {
            common_enums::CardNetwork::Visa => Ok(Self::Visa),
            common_enums::CardNetwork::Mastercard => Ok(Self::MasterCard),
            common_enums::CardNetwork::AmericanExpress => Ok(Self::AmericanExpress),
            common_enums::CardNetwork::Discover => Ok(Self::Discover),
            common_enums::CardNetwork::DinersClub => Ok(Self::DinersClub),
            common_enums::CardNetwork::JCB => Ok(Self::JCB),
            common_enums::CardNetwork::UnionPay => Ok(Self::UnionPay),
            _ => Err(errors::ConnectorError::NotSupported {
                message: "Card network".to_string(),
                connector: "worldpayvantiv",
            }
            .into()),
        }
    }
}

impl TryFrom<&connector_utils::CardIssuer> for WorldpayvativCardType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(card_issuer: &connector_utils::CardIssuer) -> Result<Self, Self::Error> {
        match card_issuer {
            connector_utils::CardIssuer::Visa => Ok(Self::Visa),
            connector_utils::CardIssuer::Master => Ok(Self::MasterCard),
            connector_utils::CardIssuer::AmericanExpress => Ok(Self::AmericanExpress),
            connector_utils::CardIssuer::Discover => Ok(Self::Discover),
            connector_utils::CardIssuer::DinersClub => Ok(Self::DinersClub),
            connector_utils::CardIssuer::JCB => Ok(Self::JCB),
            _ => Err(errors::ConnectorError::NotSupported {
                message: "Card network".to_string(),
                connector: "worldpayvantiv",
            }
            .into()),
        }
    }
}

impl<F> TryFrom<ResponseRouterData<F, VantivSyncResponse, PaymentsSyncData, PaymentsResponseData>>
    for RouterData<F, PaymentsSyncData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, VantivSyncResponse, PaymentsSyncData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = determine_attempt_status(&item)?;

        if connector_utils::is_payment_failure(status) {
            let error_code = item
                .response
                .payment_detail
                .as_ref()
                .and_then(|detail| detail.response_reason_code.clone())
                .unwrap_or(consts::NO_ERROR_CODE.to_string());
            let error_message = item
                .response
                .payment_detail
                .as_ref()
                .and_then(|detail| detail.response_reason_message.clone())
                .unwrap_or(item.response.payment_status.to_string());

            let connector_transaction_id = item
                .response
                .payment_detail
                .as_ref()
                .and_then(|detail| detail.payment_id.map(|id| id.to_string()));

            Ok(Self {
                status,
                response: Err(ErrorResponse {
                    code: error_code.clone(),
                    message: error_message.clone(),
                    reason: Some(error_message.clone()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..item.data
            })
        } else {
            let required_conversion_type = common_utils::types::StringMajorUnitForConnector;
            let minor_amount_captured = item
                .response
                .payment_detail
                .and_then(|details| {
                    details.amount.map(|amount| {
                        common_utils::types::AmountConvertor::convert_back(
                            &required_conversion_type,
                            amount,
                            item.data.request.currency,
                        )
                    })
                })
                .transpose()
                .change_context(errors::ConnectorError::ResponseHandlingFailed)?;
            Ok(Self {
                status,
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(
                        item.response.payment_id.to_string(),
                    ),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                minor_amount_captured,
                ..item.data
            })
        }
    }
}

fn get_bill_to_address<T>(item: &T) -> Option<VantivAddressData>
where
    T: UtilsRouterData,
{
    let billing_address = item.get_optional_billing();
    billing_address.and_then(|billing_address| {
        billing_address
            .address
            .clone()
            .map(|address| VantivAddressData {
                name: address.get_optional_full_name(),
                address_line1: item.get_optional_billing_line1(),
                address_line2: item.get_optional_billing_line2(),
                address_line3: item.get_optional_billing_line3(),
                city: item.get_optional_billing_city(),
                state: item.get_optional_billing_state(),
                zip: item.get_optional_billing_zip(),
                email: item.get_optional_billing_email(),
                country: item.get_optional_billing_country(),
                phone: item.get_optional_billing_phone_number(),
            })
    })
}

fn extract_customer_id<T>(item: &T) -> Option<String>
where
    T: UtilsRouterData,
{
    item.get_optional_customer_id().and_then(|customer_id| {
        let customer_id_str = customer_id.get_string_repr().to_string();
        if customer_id_str.len() <= worldpayvantiv_constants::CUSTOMER_ID_MAX_LENGTH {
            Some(customer_id_str)
        } else {
            None
        }
    })
}

fn get_valid_transaction_id(
    id: String,
    error_field_name: &str,
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    if id.len() <= worldpayvantiv_constants::MAX_PAYMENT_REFERENCE_ID_LENGTH {
        Ok(id.clone())
    } else {
        Err(errors::ConnectorError::MaxFieldLengthViolated {
            connector: "Worldpayvantiv".to_string(),
            field_name: error_field_name.to_string(),
            max_length: worldpayvantiv_constants::MAX_PAYMENT_REFERENCE_ID_LENGTH,
            received_length: id.len(),
        }
        .into())
    }
}

fn get_ship_to_address<T>(item: &T) -> Option<VantivAddressData>
where
    T: UtilsRouterData,
{
    let shipping_address = item.get_optional_shipping();
    shipping_address.and_then(|shipping_address| {
        shipping_address
            .address
            .clone()
            .map(|address| VantivAddressData {
                name: address.get_optional_full_name(),
                address_line1: item.get_optional_shipping_line1(),
                address_line2: item.get_optional_shipping_line2(),
                address_line3: item.get_optional_shipping_line3(),
                city: item.get_optional_shipping_city(),
                state: item.get_optional_shipping_state(),
                zip: item.get_optional_shipping_zip(),
                email: item.get_optional_shipping_email(),
                country: item.get_optional_shipping_country(),
                phone: item.get_optional_shipping_phone_number(),
            })
    })
}

impl TryFrom<&WorldpayvantivRouterData<&PaymentsAuthorizeRouterData>> for CnpOnlineRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &WorldpayvantivRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        if item.router_data.is_three_ds()
            && matches!(
                item.router_data.request.payment_method_data,
                PaymentMethodData::Card(_)
            )
        {
            Err(errors::ConnectorError::NotSupported {
                message: "Card 3DS".to_string(),
                connector: "Worldpayvantiv",
            })?
        }
        let worldpayvantiv_metadata =
            WorldpayvantivMetadataObject::try_from(&item.router_data.connector_meta_data)?;
        if worldpayvantiv_metadata.merchant_config_currency != item.router_data.request.currency {
            Err(errors::ConnectorError::CurrencyNotSupported {
                message: item.router_data.request.currency.to_string(),
                connector: "Worldpayvantiv",
            })?
        };

        let (card, cardholder_authentication) = get_vantiv_card_data(
            &item.router_data.request.payment_method_data,
            item.router_data.payment_method_token.clone(),
        )?;

        let worldpayvantivmetadata = item
            .router_data
            .request
            .metadata
            .clone()
            .map(|payment_metadata| {
                connector_utils::to_connector_meta::<WorldpayvantivPaymentMetadata>(Some(
                    payment_metadata,
                ))
            })
            .transpose()?;

        let report_group = worldpayvantivmetadata
            .clone()
            .and_then(|worldpayvantiv_metadata| worldpayvantiv_metadata.report_group)
            .unwrap_or(worldpayvantiv_metadata.report_group);

        let fraud_filter_override = worldpayvantivmetadata
            .clone()
            .and_then(|worldpayvantiv_metadata| worldpayvantiv_metadata.fraud_filter_override)
            .or(worldpayvantiv_metadata.fraud_filter_override);

        let worldpayvantiv_auth_type =
            WorldpayvantivAuthType::try_from(&item.router_data.connector_auth_type)?;
        let authentication = Authentication {
            user: worldpayvantiv_auth_type.user,
            password: worldpayvantiv_auth_type.password,
        };
        let customer_id = extract_customer_id(item.router_data);

        let bill_to_address = get_bill_to_address(item.router_data);
        let ship_to_address = get_ship_to_address(item.router_data);
        let processing_info = get_processing_info(&item.router_data.request)?;
        let enhanced_data = get_enhanced_data(item.router_data)?;
        let order_source = OrderSource::from((
            item.router_data.request.payment_method_data.clone(),
            item.router_data.request.payment_channel.clone(),
        ));

        let (authorization, sale) =
            if item.router_data.request.is_auto_capture()? && item.amount != MinorUnit::zero() {
                let merchant_txn_id = get_valid_transaction_id(
                    item.router_data.connector_request_reference_id.clone(),
                    "sale.id",
                )?;
                (
                    None,
                    Some(Sale {
                        id: format!("{}_{merchant_txn_id}", OperationId::Sale),
                        report_group: report_group.clone(),
                        fraud_filter_override,
                        customer_id,
                        order_id: item.router_data.connector_request_reference_id.clone(),
                        amount: item.amount,
                        order_source,
                        bill_to_address,
                        ship_to_address,
                        card: card.clone(),
                        token: processing_info.token,
                        processing_type: processing_info.processing_type,
                        original_network_transaction_id: processing_info.network_transaction_id,
                        enhanced_data,
                        allow_partial_auth: item
                            .router_data
                            .request
                            .enable_partial_authorization
                            .and_then(|enable_partial_authorization| {
                                enable_partial_authorization.then_some(true)
                            }),
                    }),
                )
            } else {
                let operation_id = if item.router_data.request.is_auto_capture()? {
                    OperationId::Sale
                } else {
                    OperationId::Auth
                };
                let merchant_txn_id = get_valid_transaction_id(
                    item.router_data.connector_request_reference_id.clone(),
                    "authorization.id",
                )?;
                (
                    Some(Authorization {
                        id: format!("{operation_id}_{merchant_txn_id}"),
                        report_group: report_group.clone(),
                        fraud_filter_override,
                        customer_id,
                        order_id: item.router_data.connector_request_reference_id.clone(),
                        amount: item.amount,
                        order_source,
                        bill_to_address,
                        ship_to_address,
                        card: card.clone(),
                        token: processing_info.token,
                        processing_type: processing_info.processing_type,
                        original_network_transaction_id: processing_info.network_transaction_id,
                        cardholder_authentication,
                        enhanced_data,
                        allow_partial_auth: item
                            .router_data
                            .request
                            .enable_partial_authorization
                            .and_then(|enable_partial_authorization| {
                                enable_partial_authorization.then_some(true)
                            }),
                    }),
                    None,
                )
            };

        Ok(Self {
            version: worldpayvantiv_constants::WORLDPAYVANTIV_VERSION.to_string(),
            xmlns: worldpayvantiv_constants::XMLNS.to_string(),
            merchant_id: worldpayvantiv_auth_type.merchant_id,
            authentication,
            authorization,
            sale,
            capture: None,
            auth_reversal: None,
            credit: None,
            void: None,
        })
    }
}

impl TryFrom<&SetupMandateRouterData> for CnpOnlineRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &SetupMandateRouterData) -> Result<Self, Self::Error> {
        if item.is_three_ds()
            && matches!(item.request.payment_method_data, PaymentMethodData::Card(_))
        {
            Err(errors::ConnectorError::NotSupported {
                message: "Card 3DS".to_string(),
                connector: "Worldpayvantiv",
            })?
        }

        let worldpayvantiv_metadata =
            WorldpayvantivMetadataObject::try_from(&item.connector_meta_data)?;
        if worldpayvantiv_metadata.merchant_config_currency != item.request.currency {
            Err(errors::ConnectorError::CurrencyNotSupported {
                message: item.request.currency.to_string(),
                connector: "Worldpayvantiv",
            })?
        };

        let (card, cardholder_authentication) = get_vantiv_card_data(
            &item.request.payment_method_data,
            item.payment_method_token.clone(),
        )?;

        let worldpayvativmetadata = item
            .request
            .metadata
            .clone()
            .map(|payment_metadata| {
                connector_utils::to_connector_meta::<WorldpayvantivPaymentMetadata>(Some(
                    payment_metadata.expose(),
                ))
            })
            .transpose()?;
        let report_group = worldpayvativmetadata
            .clone()
            .and_then(|worldpayvantiv_metadata| worldpayvantiv_metadata.report_group)
            .unwrap_or(worldpayvantiv_metadata.report_group);
        let fraud_filter_override = worldpayvativmetadata
            .clone()
            .and_then(|worldpayvantiv_metadata| worldpayvantiv_metadata.fraud_filter_override)
            .or(worldpayvantiv_metadata.fraud_filter_override);
        let worldpayvantiv_auth_type = WorldpayvantivAuthType::try_from(&item.connector_auth_type)?;
        let authentication = Authentication {
            user: worldpayvantiv_auth_type.user,
            password: worldpayvantiv_auth_type.password,
        };
        let customer_id = extract_customer_id(item);

        let bill_to_address = get_bill_to_address(item);
        let ship_to_address = get_ship_to_address(item);
        let processing_type = if item.request.is_customer_initiated_mandate_payment() {
            Some(VantivProcessingType::InitialCOF)
        } else {
            None
        };

        let enhanced_data = get_enhanced_data(item)?;
        let order_source = OrderSource::from((
            item.request.payment_method_data.clone(),
            item.request.payment_channel.clone(),
        ));
        let merchant_txn_id = get_valid_transaction_id(
            item.connector_request_reference_id.clone(),
            "authorization.id",
        )?;
        let authorization_data = Authorization {
            id: format!("{}_{merchant_txn_id}", OperationId::Sale),
            report_group: report_group.clone(),
            fraud_filter_override,
            customer_id,
            order_id: item.connector_request_reference_id.clone(),
            amount: MinorUnit::zero(),
            order_source,
            bill_to_address,
            ship_to_address,
            card: card.clone(),
            token: None,
            processing_type,
            original_network_transaction_id: None,
            cardholder_authentication,
            enhanced_data,
            allow_partial_auth: item.request.enable_partial_authorization.and_then(
                |enable_partial_authorization| enable_partial_authorization.then_some(true),
            ),
        };

        Ok(Self {
            version: worldpayvantiv_constants::WORLDPAYVANTIV_VERSION.to_string(),
            xmlns: worldpayvantiv_constants::XMLNS.to_string(),
            merchant_id: worldpayvantiv_auth_type.merchant_id,
            authentication,
            authorization: Some(authorization_data),
            sale: None,
            capture: None,
            auth_reversal: None,
            credit: None,
            void: None,
        })
    }
}

impl From<(PaymentMethodData, Option<common_enums::PaymentChannel>)> for OrderSource {
    fn from(
        (payment_method_data, payment_channel): (
            PaymentMethodData,
            Option<common_enums::PaymentChannel>,
        ),
    ) -> Self {
        if let PaymentMethodData::Wallet(
            hyperswitch_domain_models::payment_method_data::WalletData::ApplePay(_),
        ) = &payment_method_data
        {
            return Self::ApplePay;
        }
        if let PaymentMethodData::Wallet(
            hyperswitch_domain_models::payment_method_data::WalletData::GooglePay(_),
        ) = &payment_method_data
        {
            return Self::AndroidPay;
        }

        match payment_channel {
            Some(common_enums::PaymentChannel::Ecommerce)
            | Some(common_enums::PaymentChannel::Other(_))
            | None => Self::Ecommerce,
            Some(common_enums::PaymentChannel::MailOrder) => Self::MailOrder,
            Some(common_enums::PaymentChannel::TelephoneOrder) => Self::Telephone,
        }
    }
}

fn get_enhanced_data<T>(
    item: &T,
) -> Result<Option<EnhancedData>, error_stack::Report<errors::ConnectorError>>
where
    T: UtilsRouterData,
{
    let l2_l3_data = item.get_optional_l2_l3_data();
    if let Some(l2_l3_data) = l2_l3_data {
        let line_item_data = l2_l3_data.get_order_details().map(|order_details| {
            order_details
                .iter()
                .enumerate()
                .map(|(i, order)| LineItemData {
                    item_sequence_number: Some((i + 1).to_string()),
                    item_description: order
                        .description
                        .as_ref()
                        .map(|desc| desc.chars().take(19).collect::<String>()),
                    product_code: order.product_id.clone(),
                    quantity: Some(order.quantity.to_string().clone()),
                    unit_of_measure: order.unit_of_measure.clone(),
                    tax_amount: order.total_tax_amount,
                    line_item_total: Some(order.amount),
                    line_item_total_with_tax: order.total_tax_amount.map(|tax| tax + order.amount),
                    item_discount_amount: order.unit_discount_amount,
                    commodity_code: order.commodity_code.clone(),
                    unit_cost: Some(order.amount),
                })
                .collect()
        });

        let tax_exempt = match l2_l3_data.get_tax_status() {
            Some(common_enums::TaxStatus::Exempt) => Some(true),
            Some(common_enums::TaxStatus::Taxable) => Some(false),
            None => None,
        };
        let customer_reference =
            get_vantiv_customer_reference(&l2_l3_data.get_merchant_order_reference_id());

        let detail_tax: Option<DetailTax> =
            if l2_l3_data.get_merchant_tax_registration_id().is_some()
                && l2_l3_data.get_order_details().is_some()
            {
                Some(DetailTax {
                    tax_included_in_total: match tax_exempt {
                        Some(false) => Some(true),
                        Some(true) | None => Some(false),
                    },
                    card_acceptor_tax_id: l2_l3_data.get_merchant_tax_registration_id(),
                    tax_amount: l2_l3_data.get_order_details().map(|orders| {
                        orders
                            .iter()
                            .filter_map(|order| order.total_tax_amount)
                            .fold(MinorUnit::zero(), |acc, tax| acc + tax)
                    }),
                })
            } else {
                None
            };
        let enhanced_data = EnhancedData {
            customer_reference,
            sales_tax: l2_l3_data.get_order_tax_amount(),
            tax_exempt,
            discount_amount: l2_l3_data.get_discount_amount(),
            shipping_amount: l2_l3_data.get_shipping_cost(),
            duty_amount: l2_l3_data.get_duty_amount(),
            ship_from_postal_code: l2_l3_data.get_shipping_origin_zip(),
            destination_postal_code: l2_l3_data.get_shipping_zip(),
            destination_country_code: l2_l3_data.get_shipping_country(),
            detail_tax,
            line_item_data,
        };
        Ok(Some(enhanced_data))
    } else {
        Ok(None)
    }
}

fn get_processing_info(
    request: &PaymentsAuthorizeData,
) -> Result<VantivMandateDetail, error_stack::Report<errors::ConnectorError>> {
    if request.is_customer_initiated_mandate_payment() {
        Ok(VantivMandateDetail {
            processing_type: Some(VantivProcessingType::InitialCOF),
            network_transaction_id: None,
            token: None,
        })
    } else {
        match request
            .mandate_id
            .as_ref()
            .and_then(|mandate| mandate.mandate_reference_id.clone())
        {
            Some(api_models::payments::MandateReferenceId::NetworkMandateId(
                network_transaction_id,
            )) => Ok(VantivMandateDetail {
                processing_type: Some(VantivProcessingType::MerchantInitiatedCOF),
                network_transaction_id: Some(network_transaction_id.into()),
                token: None,
            }),
            Some(api_models::payments::MandateReferenceId::ConnectorMandateId(mandate_data)) => {
                let card_mandate_data = request.get_card_mandate_info()?;
                Ok(VantivMandateDetail {
                    processing_type: None,
                    network_transaction_id: None,
                    token: Some(TokenizationData {
                        cnp_token: mandate_data
                            .get_connector_mandate_id()
                            .ok_or(errors::ConnectorError::MissingConnectorMandateID)?
                            .into(),
                        exp_date: format!(
                            "{}{}",
                            card_mandate_data.card_exp_month.peek(),
                            card_mandate_data.card_exp_year.peek()
                        )
                        .into(),
                    }),
                })
            }
            _ => Ok(VantivMandateDetail {
                processing_type: None,
                network_transaction_id: None,
                token: None,
            }),
        }
    }
}

impl TryFrom<&WorldpayvantivRouterData<&PaymentsCaptureRouterData>> for CnpOnlineRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &WorldpayvantivRouterData<&PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let report_group_metadata: WorldpayvantivPaymentMetadata =
            connector_utils::to_connector_meta(item.router_data.request.connector_meta.clone())?;
        let report_group = report_group_metadata.report_group.clone().ok_or(
            errors::ConnectorError::RequestEncodingFailedWithReason(
                "Failed to obtain report_group from metadata".to_string(),
            ),
        )?;
        let merchant_txn_id = get_valid_transaction_id(
            item.router_data.connector_request_reference_id.clone(),
            "capture.id",
        )?;
        let capture = Some(Capture {
            id: format!("{}_{}", OperationId::Capture, merchant_txn_id),
            report_group,
            cnp_txn_id: item.router_data.request.connector_transaction_id.clone(),
            amount: item.amount,
        });

        let worldpayvantiv_auth_type =
            WorldpayvantivAuthType::try_from(&item.router_data.connector_auth_type)?;
        let authentication = Authentication {
            user: worldpayvantiv_auth_type.user,
            password: worldpayvantiv_auth_type.password,
        };

        Ok(Self {
            version: worldpayvantiv_constants::WORLDPAYVANTIV_VERSION.to_string(),
            xmlns: worldpayvantiv_constants::XMLNS.to_string(),
            merchant_id: worldpayvantiv_auth_type.merchant_id,
            authentication,
            authorization: None,
            sale: None,
            capture,
            auth_reversal: None,
            credit: None,
            void: None,
        })
    }
}

impl<F> TryFrom<&WorldpayvantivRouterData<&RefundsRouterData<F>>> for CnpOnlineRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &WorldpayvantivRouterData<&RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        let report_group_metadata: WorldpayvantivPaymentMetadata =
            connector_utils::to_connector_meta(
                item.router_data.request.connector_metadata.clone(),
            )?;

        let report_group = report_group_metadata.report_group.clone().ok_or(
            errors::ConnectorError::RequestEncodingFailedWithReason(
                "Failed to obtain report_group from metadata".to_string(),
            ),
        )?;
        let customer_id = extract_customer_id(item.router_data);
        let merchant_txn_id = get_valid_transaction_id(
            item.router_data.connector_request_reference_id.clone(),
            "credit.id",
        )?;

        let credit = Some(RefundRequest {
            id: format!("{}_{merchant_txn_id}", OperationId::Refund),
            report_group,
            customer_id,
            cnp_txn_id: item.router_data.request.connector_transaction_id.clone(),
            amount: item.amount,
        });

        let worldpayvantiv_auth_type =
            WorldpayvantivAuthType::try_from(&item.router_data.connector_auth_type)?;
        let authentication = Authentication {
            user: worldpayvantiv_auth_type.user,
            password: worldpayvantiv_auth_type.password,
        };

        Ok(Self {
            version: worldpayvantiv_constants::WORLDPAYVANTIV_VERSION.to_string(),
            xmlns: worldpayvantiv_constants::XMLNS.to_string(),
            merchant_id: worldpayvantiv_auth_type.merchant_id,
            authentication,
            authorization: None,
            sale: None,
            capture: None,
            auth_reversal: None,
            credit,
            void: None,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VantivSyncErrorResponse {
    pub error_messages: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "errorResponse")]
pub struct VantivDisputeErrorResponse {
    #[serde(rename = "@xmlns")]
    pub xmlns: String,
    pub errors: Vec<VantivDisputeErrors>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VantivDisputeErrors {
    pub error: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "cnpOnlineResponse", rename_all = "camelCase")]
pub struct CnpOnlineResponse {
    #[serde(rename = "@version")]
    pub version: String,
    #[serde(rename = "@response")]
    pub response_code: String,
    #[serde(rename = "@message")]
    pub message: String,
    pub authorization_response: Option<PaymentResponse>,
    pub sale_response: Option<PaymentResponse>,
    pub capture_response: Option<CaptureResponse>,
    pub auth_reversal_response: Option<AuthReversalResponse>,
    pub void_response: Option<VoidResponse>,
    pub credit_response: Option<CreditResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VantivSyncResponse {
    pub payment_id: u64,
    pub request_uuid: String,
    pub payment_status: PaymentStatus,
    pub payment_detail: Option<PaymentDetail>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentDetail {
    pub payment_id: Option<u64>,
    pub batch_id: Option<u64>,
    pub session_id: Option<u64>,
    pub response_reason_code: Option<String>,
    pub response_reason_message: Option<String>,
    pub reject_type: Option<String>,
    pub dupe_txn_id: Option<u64>,
    pub amount: Option<StringMajorUnit>,
    pub purchase_currency: Option<String>,
    pub post_day: Option<String>,
    pub reported_timestamp: Option<String>,
    pub payment_type: Option<String>,
    pub merchant_order_number: Option<String>,
    pub merchant_txn_id: Option<String>,
    pub parent_id: Option<u64>,
    pub reporting_group: Option<String>,
}

#[derive(Debug, strum::Display, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentStatus {
    NotYetProcessed,
    ProcessedSuccessfully,
    TransactionDeclined,
    StatusUnavailable,
    PaymentStatusNotFound,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureResponse {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@reportGroup")]
    pub report_group: String,
    #[serde(rename = "@customerId", skip_serializing_if = "Option::is_none")]
    pub customer_id: Option<String>,
    #[serde(rename = "cnpTxnId")]
    pub cnp_txn_id: String,
    pub response: WorldpayvantivResponseCode,
    pub response_time: String,
    pub message: String,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FraudResult {
    pub avs_result: Option<String>,
    pub card_validation_result: Option<String>,
    pub authentication_result: Option<String>,
    pub advanced_a_v_s_result: Option<String>,
    pub advanced_fraud_results: Option<AdvancedFraudResults>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdvancedFraudResults {
    pub device_review_status: Option<String>,
    pub device_reputation_score: Option<String>,
    pub triggered_rule: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentResponse {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@reportGroup")]
    pub report_group: String,
    #[serde(rename = "@customerId", skip_serializing_if = "Option::is_none")]
    pub customer_id: Option<String>,
    pub cnp_txn_id: String,
    pub order_id: String,
    pub response: WorldpayvantivResponseCode,
    pub message: String,
    pub response_time: String,
    pub auth_code: Option<Secret<String>>,
    pub fraud_result: Option<FraudResult>,
    pub token_response: Option<TokenResponse>,
    pub network_transaction_id: Option<Secret<String>>,
    pub approved_amount: Option<MinorUnit>,
    pub enhanced_auth_response: Option<EnhancedAuthResponse>,
    pub account_updater: Option<AccountUpdater>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AccountUpdater {
    pub original_card_token_info: Option<AccountUpdaterCardTokenInfo>,
    pub new_card_token_info: Option<AccountUpdaterCardTokenInfo>,
    pub extended_card_response: ExtendedCardResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExtendedCardResponse {
    pub message: Option<String>,
    pub code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AccountUpdaterCardTokenInfo {
    pub cnp_token: Secret<String>,
    pub exp_date: Option<Secret<String>>,
    #[serde(rename = "type")]
    pub card_type: Option<WorldpayvativCardType>,
    pub bin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AccountUpdaterCardTokenInfoMetadata {
    pub exp_date: Option<Secret<String>>,
    #[serde(rename = "type")]
    pub card_type: Option<String>,
    pub bin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EnhancedAuthResponse {
    pub network_response: Option<NetworkResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkResponse {
    pub endpoint: Option<String>,
    #[serde(default)]
    #[serde(rename = "networkField")]
    pub network_fields: Vec<NetworkField>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkField {
    #[serde(rename = "@fieldNumber")]
    pub field_number: String,
    #[serde(rename = "@fieldName", skip_serializing_if = "Option::is_none")]
    pub field_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_value: Option<String>,
    #[serde(default)]
    #[serde(rename = "networkSubField")]
    pub network_sub_fields: Vec<NetworkSubField>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkSubField {
    #[serde(rename = "@fieldNumber")]
    pub field_number: String,
    pub field_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TokenResponse {
    pub cnp_token: Secret<String>,
    pub token_response_code: String,
    pub token_message: String,
    #[serde(rename = "type")]
    pub card_type: Option<String>,
    pub bin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthReversalResponse {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@reportGroup")]
    pub report_group: String,
    #[serde(rename = "@customerId", skip_serializing_if = "Option::is_none")]
    pub customer_id: Option<String>,
    pub cnp_txn_id: String,
    pub response: WorldpayvantivResponseCode,
    pub response_time: String,
    pub post_date: Option<String>,
    pub message: String,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoidResponse {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@reportGroup")]
    pub report_group: String,
    pub cnp_txn_id: String,
    pub response: WorldpayvantivResponseCode,
    pub response_time: String,
    pub post_date: Option<String>,
    pub message: String,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditResponse {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@reportGroup")]
    pub report_group: String,
    #[serde(rename = "@customerId", skip_serializing_if = "Option::is_none")]
    pub customer_id: Option<String>,
    pub cnp_txn_id: String,
    pub response: WorldpayvantivResponseCode,
    pub response_time: String,
    pub message: String,
    pub location: Option<String>,
}

impl TryFrom<PaymentsCaptureResponseRouterData<CnpOnlineResponse>> for PaymentsCaptureRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsCaptureResponseRouterData<CnpOnlineResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response.capture_response {
            Some(capture_response) => {
                let status = get_attempt_status(
                    WorldpayvantivPaymentFlow::Capture,
                    capture_response.response,
                )?;
                if connector_utils::is_payment_failure(status) {
                    let network_decline_code = item
                        .response
                        .sale_response
                        .as_ref()
                        .and_then(|pr| pr.enhanced_auth_response.as_ref())
                        .and_then(|ea| ea.network_response.as_ref())
                        .and_then(|nr| {
                            nr.network_fields
                                .iter()
                                .find(|f| {
                                    f.field_number == *worldpayvantiv_constants::MAC_FIELD_NUMBER
                                })
                                .and_then(|f| f.field_value.clone())
                        });

                    let network_error_message = network_decline_code
                        .as_ref()
                        .map(|_| capture_response.message.clone());
                    Ok(Self {
                        status,
                        response: Err(ErrorResponse {
                            code: capture_response.response.to_string(),
                            message: capture_response.message.clone(),
                            reason: Some(capture_response.message.clone()),
                            status_code: item.http_code,
                            attempt_status: None,
                            connector_transaction_id: Some(capture_response.cnp_txn_id),
                            network_advice_code: None,
                            network_decline_code,
                            network_error_message,
                            connector_metadata: None,
                        }),
                        ..item.data
                    })
                } else {
                    Ok(Self {
                        status,
                        response: Ok(PaymentsResponseData::TransactionResponse {
                            resource_id: ResponseId::ConnectorTransactionId(
                                capture_response.cnp_txn_id,
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
                    })
                }
            }
            None => {
                let network_decline_code = item
                    .response
                    .sale_response
                    .as_ref()
                    .and_then(|pr| pr.enhanced_auth_response.as_ref())
                    .and_then(|ea| ea.network_response.as_ref())
                    .and_then(|nr| {
                        nr.network_fields
                            .iter()
                            .find(|f| f.field_number == *worldpayvantiv_constants::MAC_FIELD_NUMBER)
                            .and_then(|f| f.field_value.clone())
                    });

                let network_error_message = network_decline_code
                    .as_ref()
                    .map(|_| item.response.message.clone());
                Ok(Self {
                    status: common_enums::AttemptStatus::CaptureFailed,
                    response: Err(ErrorResponse {
                        code: item.response.response_code,
                        message: item.response.message.clone(),
                        reason: Some(item.response.message.clone()),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: None,
                        network_advice_code: None,
                        network_decline_code,
                        network_error_message,
                        connector_metadata: None,
                    }),
                    ..item.data
                })
            }
        }
    }
}

fn get_vantiv_customer_reference(customer_id: &Option<String>) -> Option<String> {
    customer_id.clone().and_then(|id| {
        if id.len() <= worldpayvantiv_constants::CUSTOMER_REFERENCE_MAX_LENGTH {
            Some(id)
        } else {
            None
        }
    })
}

impl TryFrom<PaymentsCancelResponseRouterData<CnpOnlineResponse>> for PaymentsCancelRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsCancelResponseRouterData<CnpOnlineResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response.auth_reversal_response {
            Some(auth_reversal_response) => {
                let status = get_attempt_status(
                    WorldpayvantivPaymentFlow::Void,
                    auth_reversal_response.response,
                )?;
                if connector_utils::is_payment_failure(status) {
                    let network_decline_code = item
                        .response
                        .sale_response
                        .as_ref()
                        .and_then(|pr| pr.enhanced_auth_response.as_ref())
                        .and_then(|ea| ea.network_response.as_ref())
                        .and_then(|nr| {
                            nr.network_fields
                                .iter()
                                .find(|f| {
                                    f.field_number == *worldpayvantiv_constants::MAC_FIELD_NUMBER
                                })
                                .and_then(|f| f.field_value.clone())
                        });

                    let network_error_message = network_decline_code
                        .as_ref()
                        .map(|_| auth_reversal_response.message.clone());
                    Ok(Self {
                        status,
                        response: Err(ErrorResponse {
                            code: auth_reversal_response.response.to_string(),
                            message: auth_reversal_response.message.clone(),
                            reason: Some(auth_reversal_response.message.clone()),
                            status_code: item.http_code,
                            attempt_status: None,
                            connector_transaction_id: Some(auth_reversal_response.cnp_txn_id),
                            network_advice_code: None,
                            network_decline_code,
                            network_error_message,
                            connector_metadata: None,
                        }),
                        ..item.data
                    })
                } else {
                    Ok(Self {
                        status,
                        response: Ok(PaymentsResponseData::TransactionResponse {
                            resource_id: ResponseId::ConnectorTransactionId(
                                auth_reversal_response.cnp_txn_id,
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
                    })
                }
            }
            None => {
                let network_decline_code = item
                    .response
                    .sale_response
                    .as_ref()
                    .and_then(|pr| pr.enhanced_auth_response.as_ref())
                    .and_then(|ea| ea.network_response.as_ref())
                    .and_then(|nr| {
                        nr.network_fields
                            .iter()
                            .find(|f| f.field_number == *worldpayvantiv_constants::MAC_FIELD_NUMBER)
                            .and_then(|f| f.field_value.clone())
                    });

                let network_error_message = network_decline_code
                    .as_ref()
                    .map(|_| item.response.message.clone());
                Ok(Self {
                    // Incase of API failure
                    status: common_enums::AttemptStatus::VoidFailed,
                    response: Err(ErrorResponse {
                        code: item.response.response_code,
                        message: item.response.message.clone(),
                        reason: Some(item.response.message.clone()),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: None, // Transaction id not created at connector
                        network_advice_code: None,
                        network_decline_code,
                        network_error_message,
                        connector_metadata: None,
                    }),
                    ..item.data
                })
            }
        }
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            CnpOnlineResponse,
            PaymentsCancelPostCaptureData,
            PaymentsResponseData,
        >,
    > for RouterData<F, PaymentsCancelPostCaptureData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            CnpOnlineResponse,
            PaymentsCancelPostCaptureData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response.void_response {
            Some(void_response) => {
                let status =
                    get_attempt_status(WorldpayvantivPaymentFlow::VoidPC, void_response.response)?;
                if connector_utils::is_payment_failure(status) {
                    Ok(Self {
                        status,
                        response: Err(ErrorResponse {
                            code: void_response.response.to_string(),
                            message: void_response.message.clone(),
                            reason: Some(void_response.message.clone()),
                            status_code: item.http_code,
                            attempt_status: None,
                            connector_transaction_id: Some(void_response.cnp_txn_id),
                            network_advice_code: None,
                            network_decline_code: None,
                            network_error_message: None,
                            connector_metadata: None,
                        }),
                        ..item.data
                    })
                } else {
                    Ok(Self {
                        status,
                        response: Ok(PaymentsResponseData::TransactionResponse {
                            resource_id: ResponseId::ConnectorTransactionId(
                                void_response.cnp_txn_id,
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
                    })
                }
            }
            None => Ok(Self {
                // Incase of API failure
                status: common_enums::AttemptStatus::VoidFailed,
                response: Err(ErrorResponse {
                    code: item.response.response_code,
                    message: item.response.message.clone(),
                    reason: Some(item.response.message.clone()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..item.data
            }),
        }
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, CnpOnlineResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, CnpOnlineResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response.credit_response {
            Some(credit_response) => {
                let refund_status = get_refund_status(credit_response.response)?;
                if connector_utils::is_refund_failure(refund_status) {
                    let network_decline_code = item
                        .response
                        .sale_response
                        .as_ref()
                        .and_then(|pr| pr.enhanced_auth_response.as_ref())
                        .and_then(|ea| ea.network_response.as_ref())
                        .and_then(|nr| {
                            nr.network_fields
                                .iter()
                                .find(|f| {
                                    f.field_number == *worldpayvantiv_constants::MAC_FIELD_NUMBER
                                })
                                .and_then(|f| f.field_value.clone())
                        });

                    let network_error_message = network_decline_code
                        .as_ref()
                        .map(|_| credit_response.message.clone());
                    Ok(Self {
                        response: Err(ErrorResponse {
                            code: credit_response.response.to_string(),
                            message: credit_response.message.clone(),
                            reason: Some(credit_response.message.clone()),
                            status_code: item.http_code,
                            attempt_status: None,
                            connector_transaction_id: None,
                            network_advice_code: None,
                            network_decline_code,
                            network_error_message,
                            connector_metadata: None,
                        }),
                        ..item.data
                    })
                } else {
                    Ok(Self {
                        response: Ok(RefundsResponseData {
                            connector_refund_id: credit_response.cnp_txn_id,
                            refund_status,
                        }),
                        ..item.data
                    })
                }
            }
            None => {
                let network_decline_code = item
                    .response
                    .sale_response
                    .as_ref()
                    .and_then(|pr| pr.enhanced_auth_response.as_ref())
                    .and_then(|ea| ea.network_response.as_ref())
                    .and_then(|nr| {
                        nr.network_fields
                            .iter()
                            .find(|f| f.field_number == *worldpayvantiv_constants::MAC_FIELD_NUMBER)
                            .and_then(|f| f.field_value.clone())
                    });

                let network_error_message = network_decline_code
                    .as_ref()
                    .map(|_| item.response.message.clone());
                Ok(Self {
                    response: Err(ErrorResponse {
                        code: item.response.response_code,
                        message: item.response.message.clone(),
                        reason: Some(item.response.message.clone()),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: None,
                        network_advice_code: None,
                        network_decline_code,
                        network_error_message,
                        connector_metadata: None,
                    }),
                    ..item.data
                })
            }
        }
    }
}

impl TryFrom<&PaymentsCancelRouterData> for CnpOnlineRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let report_group_metadata: WorldpayvantivPaymentMetadata =
            connector_utils::to_connector_meta(item.request.connector_meta.clone())?;
        let report_group = report_group_metadata.report_group.clone().ok_or(
            errors::ConnectorError::RequestEncodingFailedWithReason(
                "Failed to obtain report_group from metadata".to_string(),
            ),
        )?;
        let merchant_txn_id = get_valid_transaction_id(
            item.connector_request_reference_id.clone(),
            "authReversal.id",
        )?;
        let auth_reversal = Some(AuthReversal {
            id: format!("{}_{merchant_txn_id}", OperationId::Void),
            report_group,
            cnp_txn_id: item.request.connector_transaction_id.clone(),
        });

        let worldpayvantiv_auth_type = WorldpayvantivAuthType::try_from(&item.connector_auth_type)?;
        let authentication = Authentication {
            user: worldpayvantiv_auth_type.user,
            password: worldpayvantiv_auth_type.password,
        };

        Ok(Self {
            version: worldpayvantiv_constants::WORLDPAYVANTIV_VERSION.to_string(),
            xmlns: worldpayvantiv_constants::XMLNS.to_string(),
            merchant_id: worldpayvantiv_auth_type.merchant_id,
            authentication,
            authorization: None,
            sale: None,
            capture: None,
            auth_reversal,
            credit: None,
            void: None,
        })
    }
}

impl TryFrom<&PaymentsCancelPostCaptureRouterData> for CnpOnlineRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsCancelPostCaptureRouterData) -> Result<Self, Self::Error> {
        let report_group_metadata: WorldpayvantivPaymentMetadata =
            connector_utils::to_connector_meta(item.request.connector_meta.clone())?;
        let report_group = report_group_metadata.report_group.clone().ok_or(
            errors::ConnectorError::RequestEncodingFailedWithReason(
                "Failed to obtain report_group from metadata".to_string(),
            ),
        )?;
        let merchant_txn_id =
            get_valid_transaction_id(item.connector_request_reference_id.clone(), "void.id")?;
        let void = Some(Void {
            id: format!("{}_{merchant_txn_id}", OperationId::VoidPC,),
            report_group,
            cnp_txn_id: item.request.connector_transaction_id.clone(),
        });

        let worldpayvantiv_auth_type = WorldpayvantivAuthType::try_from(&item.connector_auth_type)?;
        let authentication = Authentication {
            user: worldpayvantiv_auth_type.user,
            password: worldpayvantiv_auth_type.password,
        };

        Ok(Self {
            version: worldpayvantiv_constants::WORLDPAYVANTIV_VERSION.to_string(),
            xmlns: worldpayvantiv_constants::XMLNS.to_string(),
            merchant_id: worldpayvantiv_auth_type.merchant_id,
            authentication,
            authorization: None,
            sale: None,
            capture: None,
            void,
            auth_reversal: None,
            credit: None,
        })
    }
}

impl<F>
    TryFrom<ResponseRouterData<F, CnpOnlineResponse, PaymentsAuthorizeData, PaymentsResponseData>>
    for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, CnpOnlineResponse, PaymentsAuthorizeData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match (item.response.sale_response.as_ref(), item.response.authorization_response.as_ref()) {
            (Some(sale_response), None) => {
                let status = get_attempt_status(WorldpayvantivPaymentFlow::Sale, sale_response.response)?;

                // While making an authorize flow call to WorldpayVantiv, if Account Updater is enabled then we well get new card token info in response.
                // We are extracting that new card token info here to be sent back in mandate_id in router_data.
                let mandate_reference_data = match item.data.request.is_mandate_payment() {
                    true => {
                        if let Some(account_updater) = sale_response.account_updater.as_ref() {
                            account_updater
                                .new_card_token_info
                                .clone()
                                .map(MandateReference::from)
                        } else {
                            sale_response
                                .token_response
                                .clone()
                                .map(MandateReference::from)
                        }
                    }
                    false => sale_response
                        .token_response
                        .clone()
                        .map(MandateReference::from)
                };


                if connector_utils::is_payment_failure(status) {
                    let network_decline_code = item
                    .response
                    .sale_response
                    .as_ref()
                    .and_then(|pr| pr.enhanced_auth_response.as_ref())
                    .and_then(|ea| ea.network_response.as_ref())
                    .and_then(|nr| {
                        nr.network_fields
                            .iter()
                            .find(|f| f.field_number == *worldpayvantiv_constants::MAC_FIELD_NUMBER)
                            .and_then(|f| f.field_value.clone())
                    });

                let network_error_message = network_decline_code
                    .as_ref()
                    .map(|_| sale_response.message.clone());


                    Ok(Self {
                        connector_response: Some(ConnectorResponseData::new(None, None,None, mandate_reference_data.clone())),
                        status,
                        response: Err(ErrorResponse {
                            code: sale_response.response.to_string(),
                            message: sale_response.message.clone(),
                            reason: Some(sale_response.message.clone()),
                            status_code: item.http_code,
                            attempt_status: None,
                            connector_transaction_id: Some(sale_response.cnp_txn_id.clone()),
                            network_advice_code: None,
                            network_decline_code,
                            network_error_message,
                            connector_metadata: None,
                        }),
                        ..item.data
                    })
                } else {
                    let report_group = WorldpayvantivPaymentMetadata {
                        report_group: Some(sale_response.report_group.clone()),
                        fraud_filter_override: item
                        .data
                        .request
                        .metadata
                        .clone()
                        .map(|payment_metadata| {
                            connector_utils::to_connector_meta::<WorldpayvantivPaymentMetadata>(Some(
                                payment_metadata,
                            ))
                        })
                        .transpose()?
                        .and_then(|worldpayvantiv_metadata| worldpayvantiv_metadata.fraud_filter_override)
                    };
                    let connector_metadata =   Some(report_group.encode_to_value()
                    .change_context(errors::ConnectorError::ResponseHandlingFailed)?);
                    let additional_payment_method_connector_response = sale_response.fraud_result.as_ref().map(get_additional_payment_method_connector_response);
                    let connector_response = Some(ConnectorResponseData::new(
                        additional_payment_method_connector_response,
                        None,
                        None,
                        mandate_reference_data.clone(),
                    ));

                    Ok(Self {
                        status,
                        response: Ok(PaymentsResponseData::TransactionResponse {
                            resource_id: ResponseId::ConnectorTransactionId(sale_response.cnp_txn_id.clone()),
                            redirection_data: Box::new(None),
                            mandate_reference: Box::new(mandate_reference_data),
                            connector_metadata,
                            network_txn_id: sale_response.network_transaction_id.clone().map(|network_transaction_id| network_transaction_id.expose()),
                            connector_response_reference_id: Some(sale_response.order_id.clone()),
                            incremental_authorization_allowed: None,
                            charges: None,
                        }),
                        connector_response,
                        amount_captured: sale_response.approved_amount.map(MinorUnit::get_amount_as_i64),
                        minor_amount_captured: sale_response.approved_amount,
                        ..item.data
                    })
                }
            },
            (None, Some(auth_response)) => {
                let payment_flow_type = if item.data.request.is_auto_capture()? {
                    WorldpayvantivPaymentFlow::Sale
                } else {
                    WorldpayvantivPaymentFlow::Auth
                };

                let status = get_attempt_status(payment_flow_type, auth_response.response)?;
                if connector_utils::is_payment_failure(status) {
                    let network_decline_code = item
                    .response
                    .authorization_response
                    .as_ref()
                    .and_then(|pr| pr.enhanced_auth_response.as_ref())
                    .and_then(|ea| ea.network_response.as_ref())
                    .and_then(|nr| {
                        nr.network_fields
                            .iter()
                            .find(|f| f.field_number == *worldpayvantiv_constants::MAC_FIELD_NUMBER)
                            .and_then(|f| f.field_value.clone())
                    });

                let network_error_message = network_decline_code
                    .as_ref()
                    .map(|_| auth_response.message.clone());
                    Ok(Self {
                        status,
                        response: Err(ErrorResponse {
                            code: auth_response.response.to_string(),
                            message: auth_response.message.clone(),
                            reason: Some(auth_response.message.clone()),
                            status_code: item.http_code,
                            attempt_status: None,
                            connector_transaction_id: Some(auth_response.cnp_txn_id.clone()),
                            network_advice_code: None,
                            network_decline_code,
                            network_error_message,
                            connector_metadata: None,
                        }),
                        ..item.data
                    })
                } else {
                    let report_group = WorldpayvantivPaymentMetadata {
                        report_group: Some(auth_response.report_group.clone()),
                        fraud_filter_override: item
                        .data
                        .request
                        .metadata
                        .clone()
                        .map(|payment_metadata| {
                            connector_utils::to_connector_meta::<WorldpayvantivPaymentMetadata>(Some(
                                payment_metadata,
                            ))
                        })
                        .transpose()?
                        .and_then(|worldpayvantiv_metadata| worldpayvantiv_metadata.fraud_filter_override)
                    };
                    let connector_metadata =   Some(report_group.encode_to_value()
                    .change_context(errors::ConnectorError::ResponseHandlingFailed)?);
                    let mandate_reference_data = auth_response.token_response.clone().map(MandateReference::from);
                    let connector_response = auth_response.fraud_result.as_ref().map(get_connector_response);

                    Ok(Self {
                        status,
                        response: Ok(PaymentsResponseData::TransactionResponse {
                            resource_id: ResponseId::ConnectorTransactionId(auth_response.cnp_txn_id.clone()),
                            redirection_data: Box::new(None),
                            mandate_reference: Box::new(mandate_reference_data),
                            connector_metadata,
                            network_txn_id: auth_response.network_transaction_id.clone().map(|network_transaction_id| network_transaction_id.expose()),
                            connector_response_reference_id: Some(auth_response.order_id.clone()),
                            incremental_authorization_allowed: None,
                            charges: None,
                        }),
                        connector_response,
                        amount_captured: if payment_flow_type == WorldpayvantivPaymentFlow::Sale {
                            auth_response.approved_amount.map(MinorUnit::get_amount_as_i64)
                        } else {
                            None
                        },
                        minor_amount_capturable: if payment_flow_type == WorldpayvantivPaymentFlow::Auth {
                            auth_response.approved_amount
                        } else {
                            None
                        },
                        minor_amount_captured: if payment_flow_type == WorldpayvantivPaymentFlow::Sale {
                            auth_response.approved_amount
                        } else {
                            None
                        },
                        ..item.data
                    })
                }
            },
            (None, None) => {
                Ok(Self {
                status: common_enums::AttemptStatus::Failure,
                response: Err(ErrorResponse {
                    code: item.response.response_code.clone(),
                    message: item.response.message.clone(),
                    reason: Some(item.response.message.clone()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None, // Transaction id not created at connector
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..item.data
            })},
            (_, _) => {  Err(errors::ConnectorError::UnexpectedResponseError(
                bytes::Bytes::from("Only one of 'sale_response' or 'authorisation_response' is expected, but both were received".to_string()),           
             ))?
            },
    }
    }
}

impl<F>
    TryFrom<ResponseRouterData<F, CnpOnlineResponse, SetupMandateRequestData, PaymentsResponseData>>
    for RouterData<F, SetupMandateRequestData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            CnpOnlineResponse,
            SetupMandateRequestData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response.authorization_response.as_ref() {
            Some(auth_response) => {
                let status =
                    get_attempt_status(WorldpayvantivPaymentFlow::Sale, auth_response.response)?;
                if connector_utils::is_payment_failure(status) {
                    let network_decline_code = item
                        .response
                        .authorization_response
                        .as_ref()
                        .and_then(|pr| pr.enhanced_auth_response.as_ref())
                        .and_then(|ea| ea.network_response.as_ref())
                        .and_then(|nr| {
                            nr.network_fields
                                .iter()
                                .find(|f| {
                                    f.field_number == *worldpayvantiv_constants::MAC_FIELD_NUMBER
                                })
                                .and_then(|f| f.field_value.clone())
                        });

                    let network_error_message = network_decline_code
                        .as_ref()
                        .map(|_| auth_response.message.clone());
                    Ok(Self {
                        status,
                        response: Err(ErrorResponse {
                            code: auth_response.response.to_string(),
                            message: auth_response.message.clone(),
                            reason: Some(auth_response.message.clone()),
                            status_code: item.http_code,
                            attempt_status: None,
                            connector_transaction_id: Some(auth_response.order_id.clone()),
                            network_advice_code: None,
                            network_decline_code,
                            network_error_message,
                            connector_metadata: None,
                        }),
                        ..item.data
                    })
                } else {
                    let report_group = WorldpayvantivPaymentMetadata {
                        report_group: Some(auth_response.report_group.clone()),
                        fraud_filter_override: item
                            .data
                            .request
                            .metadata
                            .clone()
                            .map(|payment_metadata| {
                                connector_utils::to_connector_meta::<WorldpayvantivPaymentMetadata>(
                                    Some(payment_metadata.expose()),
                                )
                            })
                            .transpose()?
                            .and_then(|worldpayvantiv_metadata| {
                                worldpayvantiv_metadata.fraud_filter_override
                            }),
                    };
                    let connector_metadata = Some(
                        report_group
                            .encode_to_value()
                            .change_context(errors::ConnectorError::ResponseHandlingFailed)?,
                    );
                    let mandate_reference_data = auth_response
                        .token_response
                        .clone()
                        .map(MandateReference::from);
                    let connector_response = auth_response
                        .fraud_result
                        .as_ref()
                        .map(get_connector_response);

                    Ok(Self {
                        status,
                        response: Ok(PaymentsResponseData::TransactionResponse {
                            resource_id: ResponseId::ConnectorTransactionId(
                                auth_response.cnp_txn_id.clone(),
                            ),
                            redirection_data: Box::new(None),
                            mandate_reference: Box::new(mandate_reference_data),
                            connector_metadata,
                            network_txn_id: auth_response
                                .network_transaction_id
                                .clone()
                                .map(|network_transaction_id| network_transaction_id.expose()),
                            connector_response_reference_id: Some(auth_response.order_id.clone()),
                            incremental_authorization_allowed: None,
                            charges: None,
                        }),
                        connector_response,
                        ..item.data
                    })
                }
            }
            None => Ok(Self {
                status: common_enums::AttemptStatus::Failure,
                response: Err(ErrorResponse {
                    code: item.response.response_code.clone(),
                    message: item.response.message.clone(),
                    reason: Some(item.response.message.clone()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..item.data
            }),
        }
    }
}

impl From<TokenResponse> for MandateReference {
    fn from(token_data: TokenResponse) -> Self {
        Self {
            connector_mandate_id: Some(token_data.cnp_token.expose()),
            payment_method_id: None,
            mandate_metadata: None,
            connector_mandate_request_reference_id: None,
        }
    }
}

impl From<&AccountUpdaterCardTokenInfo> for api_models::payments::UpdatedMandateDetails {
    fn from(token_data: &AccountUpdaterCardTokenInfo) -> Self {
        let card_exp_month = token_data
            .exp_date
            .as_ref()
            .and_then(|exp_date| exp_date.peek().as_str().get(0..2).map(|s| s.to_string()))
            .map(Secret::new);

        let card_exp_year = token_data
            .exp_date
            .as_ref()
            .and_then(|exp_date| exp_date.peek().as_str().get(2..4).map(|s| s.to_string()))
            .map(Secret::new);

        Self {
            card_network: token_data
                .card_type
                .clone()
                .map(common_enums::CardNetwork::from),
            card_isin: token_data.bin.clone(),
            card_exp_month,
            card_exp_year,
        }
    }
}

impl From<WorldpayvativCardType> for common_enums::CardNetwork {
    fn from(card_type: WorldpayvativCardType) -> Self {
        match card_type {
            WorldpayvativCardType::Visa => Self::Visa,
            WorldpayvativCardType::MasterCard => Self::Mastercard,
            WorldpayvativCardType::AmericanExpress => Self::AmericanExpress,
            WorldpayvativCardType::Discover => Self::Discover,
            WorldpayvativCardType::JCB => Self::JCB,
            WorldpayvativCardType::DinersClub => Self::DinersClub,
            WorldpayvativCardType::UnionPay => Self::UnionPay,
        }
    }
}

impl From<AccountUpdaterCardTokenInfo> for MandateReference {
    fn from(token_data: AccountUpdaterCardTokenInfo) -> Self {
        let mandate_metadata = api_models::payments::UpdatedMandateDetails::from(&token_data);

        let mandate_metadata_json = serde_json::to_value(&mandate_metadata)
            .inspect_err(|_| {
                logger::error!(
                    "Failed to construct Mandate Reference from the UpdatedMandateDetails"
                )
            })
            .ok();

        let mandate_metadata_secret_json = mandate_metadata_json.map(pii::SecretSerdeValue::new);

        Self {
            connector_mandate_id: Some(token_data.cnp_token.expose()),
            payment_method_id: None,
            mandate_metadata: mandate_metadata_secret_json,
            connector_mandate_request_reference_id: None,
        }
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, VantivSyncResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, VantivSyncResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = get_refund_status_for_rsync(item.response.payment_status)?;
        if connector_utils::is_refund_failure(refund_status) {
            let error_code = item
                .response
                .payment_detail
                .as_ref()
                .and_then(|detail| detail.response_reason_code.clone())
                .unwrap_or(consts::NO_ERROR_CODE.to_string());
            let error_message = item
                .response
                .payment_detail
                .as_ref()
                .and_then(|detail| detail.response_reason_message.clone())
                .unwrap_or(consts::NO_ERROR_MESSAGE.to_string());
            let connector_transaction_id = item
                .response
                .payment_detail
                .as_ref()
                .and_then(|detail| detail.payment_id.map(|id| id.to_string()));

            Ok(Self {
                response: Err(ErrorResponse {
                    code: error_code.clone(),
                    message: error_message.clone(),
                    reason: Some(error_message.clone()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..item.data
            })
        } else {
            Ok(Self {
                response: Ok(RefundsResponseData {
                    connector_refund_id: item.response.payment_id.to_string(),
                    refund_status,
                }),
                ..item.data
            })
        }
    }
}

fn determine_attempt_status<F>(
    item: &ResponseRouterData<F, VantivSyncResponse, PaymentsSyncData, PaymentsResponseData>,
) -> Result<common_enums::AttemptStatus, errors::ConnectorError> {
    if let Some(merchant_txn_id) = item
        .response
        .payment_detail
        .as_ref()
        .and_then(|payment_detail| payment_detail.merchant_txn_id.clone())
    {
        let flow_type = get_payment_flow_type(&merchant_txn_id)?;
        match item.response.payment_status {
            PaymentStatus::ProcessedSuccessfully => match flow_type {
                WorldpayvantivPaymentFlow::Sale | WorldpayvantivPaymentFlow::Capture => {
                    Ok(common_enums::AttemptStatus::Charged)
                }
                WorldpayvantivPaymentFlow::Auth => Ok(common_enums::AttemptStatus::Authorized),
                WorldpayvantivPaymentFlow::Void => Ok(common_enums::AttemptStatus::Voided),
                WorldpayvantivPaymentFlow::VoidPC => {
                    Ok(common_enums::AttemptStatus::VoidedPostCharge)
                }
            },
            PaymentStatus::TransactionDeclined => match flow_type {
                WorldpayvantivPaymentFlow::Sale | WorldpayvantivPaymentFlow::Capture => {
                    Ok(common_enums::AttemptStatus::Failure)
                }
                WorldpayvantivPaymentFlow::Auth => {
                    Ok(common_enums::AttemptStatus::AuthorizationFailed)
                }
                WorldpayvantivPaymentFlow::Void => Ok(common_enums::AttemptStatus::VoidFailed),
                WorldpayvantivPaymentFlow::VoidPC => Ok(common_enums::AttemptStatus::VoidFailed),
            },
            PaymentStatus::PaymentStatusNotFound
            | PaymentStatus::NotYetProcessed
            | PaymentStatus::StatusUnavailable => Ok(item.data.status),
        }
    } else {
        Ok(item.data.status)
    }
}

fn get_refund_status_for_rsync(
    vantiv_status: PaymentStatus,
) -> Result<common_enums::RefundStatus, errors::ConnectorError> {
    match vantiv_status {
        PaymentStatus::ProcessedSuccessfully => Ok(common_enums::RefundStatus::Success),
        PaymentStatus::TransactionDeclined => Ok(common_enums::RefundStatus::Failure),
        PaymentStatus::NotYetProcessed
        | PaymentStatus::StatusUnavailable
        | PaymentStatus::PaymentStatusNotFound => Ok(common_enums::RefundStatus::Pending),
    }
}

#[derive(Debug, strum::Display, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum WorldpayvantivResponseCode {
    #[serde(rename = "001")]
    #[strum(serialize = "001")]
    TransactionReceived,
    #[serde(rename = "000")]
    #[strum(serialize = "000")]
    Approved,
    #[serde(rename = "010")]
    #[strum(serialize = "010")]
    PartiallyApproved,
    #[serde(rename = "011")]
    #[strum(serialize = "011")]
    OfflineApproval,
    #[serde(rename = "013")]
    #[strum(serialize = "013")]
    OfflineApprovalUnableToGoOnline,
    #[serde(rename = "015")]
    #[strum(serialize = "015")]
    PendingShopperCheckoutCompletion,
    #[serde(rename = "016")]
    #[strum(serialize = "016")]
    ShopperCheckoutExpired,
    #[serde(rename = "100")]
    #[strum(serialize = "100")]
    ProcessingNetworkUnavailable,
    #[serde(rename = "101")]
    #[strum(serialize = "101")]
    IssuerUnavailable,
    #[serde(rename = "102")]
    #[strum(serialize = "102")]
    ReSubmitTransaction,
    #[serde(rename = "103")]
    #[strum(serialize = "103")]
    MerchantNotConfiguredForProcessingAtThisSite,
    #[serde(rename = "108")]
    #[strum(serialize = "108")]
    TryAgainLater,
    #[serde(rename = "110")]
    #[strum(serialize = "110")]
    InsufficientFunds,
    #[serde(rename = "111")]
    #[strum(serialize = "111")]
    AuthorizationAmountHasAlreadyBeenDepleted,
    #[serde(rename = "112")]
    #[strum(serialize = "112")]
    InsufficientFundsRetryAfter1Hour,
    #[serde(rename = "113")]
    #[strum(serialize = "113")]
    InsufficientFundsRetryAfter24Hour,
    #[serde(rename = "114")]
    #[strum(serialize = "114")]
    InsufficientFundsRetryAfter2Days,
    #[serde(rename = "115")]
    #[strum(serialize = "115")]
    InsufficientFundsRetryAfter4Days,
    #[serde(rename = "116")]
    #[strum(serialize = "116")]
    InsufficientFundsRetryAfter6Days,
    #[serde(rename = "117")]
    #[strum(serialize = "117")]
    InsufficientFundsRetryAfter8Days,
    #[serde(rename = "118")]
    #[strum(serialize = "118")]
    InsufficientFundsRetryAfter10Days,
    #[serde(rename = "120")]
    #[strum(serialize = "120")]
    CallIssuer,
    #[serde(rename = "121")]
    #[strum(serialize = "121")]
    CallAmex,
    #[serde(rename = "122")]
    #[strum(serialize = "122")]
    CallDinersClub,
    #[serde(rename = "123")]
    #[strum(serialize = "123")]
    CallDiscover,
    #[serde(rename = "124")]
    #[strum(serialize = "124")]
    CallJbs,
    #[serde(rename = "125")]
    #[strum(serialize = "125")]
    CallVisaMastercard,
    #[serde(rename = "126")]
    #[strum(serialize = "126")]
    CallIssuerUpdateCardholderData,
    #[serde(rename = "127")]
    #[strum(serialize = "127")]
    ExceedsApprovalAmountLimit,
    #[serde(rename = "130")]
    #[strum(serialize = "130")]
    CallIndicatedNumber,
    #[serde(rename = "131")]
    #[strum(serialize = "131")]
    UnacceptablePinTransactionDeclinedRetry,
    #[serde(rename = "132")]
    #[strum(serialize = "132")]
    PinNotChanged,
    #[serde(rename = "137")]
    #[strum(serialize = "137")]
    ConsumerMultiUseVirtualCardNumberSoftDecline,
    #[serde(rename = "138")]
    #[strum(serialize = "138")]
    ConsumerNonReloadablePrepaidCardSoftDecline,
    #[serde(rename = "139")]
    #[strum(serialize = "139")]
    ConsumerSingleUseVirtualCardNumberSoftDecline,
    #[serde(rename = "140")]
    #[strum(serialize = "140")]
    UpdateCardholderData,
    #[serde(rename = "141")]
    #[strum(serialize = "141")]
    ConsumerNonReloadablePrepaidCardApproved,
    #[serde(rename = "142")]
    #[strum(serialize = "142")]
    ConsumerSingleUseVirtualCardNumberApproved,
    #[serde(rename = "143")]
    #[strum(serialize = "143")]
    MerchantDoesntQualifyForProductCode,
    #[serde(rename = "145")]
    #[strum(serialize = "145")]
    Lifecycle,
    #[serde(rename = "146")]
    #[strum(serialize = "146")]
    Policy,
    #[serde(rename = "147")]
    #[strum(serialize = "147")]
    FraudSecurity,
    #[serde(rename = "148")]
    #[strum(serialize = "148")]
    InvalidOrExpiredCardContactCardholderToUpdate,
    #[serde(rename = "149")]
    #[strum(serialize = "149")]
    InvalidTransactionOrCardRestrictionVerifyInformationAndResubmit,
    #[serde(rename = "154")]
    #[strum(serialize = "154")]
    AtLeastOneOfOrigIdOrOrigCnpTxnIdIsRequired,
    #[serde(rename = "155")]
    #[strum(serialize = "155")]
    OrigCnpTxnIdIsRequiredWhenShowStatusOnlyIsUsed,
    #[serde(rename = "156")]
    #[strum(serialize = "156")]
    IncrementalAuthNotSupported,
    #[serde(rename = "157")]
    #[strum(serialize = "157")]
    SetAuthIndicatorToIncremental,
    #[serde(rename = "158")]
    #[strum(serialize = "158")]
    IncrementalValueForAuthIndicatorNotAllowedInThisAuthStructure,
    #[serde(rename = "159")]
    #[strum(serialize = "159")]
    CannotRequestAnIncrementalAuthIfOriginalAuthNotSetToEstimated,
    #[serde(rename = "161")]
    #[strum(serialize = "161")]
    TransactionMustReferenceTheEstimatedAuth,
    #[serde(rename = "162")]
    #[strum(serialize = "162")]
    IncrementedAuthExceedsMaxTransactionAmount,
    #[serde(rename = "170")]
    #[strum(serialize = "170")]
    SubmittedMccNotAllowed,
    #[serde(rename = "192")]
    #[strum(serialize = "192")]
    MerchantNotCertifiedEnabledForIias,
    #[serde(rename = "206")]
    #[strum(serialize = "206")]
    IssuerGeneratedError,
    #[serde(rename = "207")]
    #[strum(serialize = "207")]
    PickupCardOtherThanLostStolen,
    #[serde(rename = "209")]
    #[strum(serialize = "209")]
    InvalidAmountHardDecline,
    #[serde(rename = "211")]
    #[strum(serialize = "211")]
    ReversalUnsuccessful,
    #[serde(rename = "212")]
    #[strum(serialize = "212")]
    MissingData,
    #[serde(rename = "213")]
    #[strum(serialize = "213")]
    PickupCardLostCard,
    #[serde(rename = "214")]
    #[strum(serialize = "214")]
    PickupCardStolenCard,
    #[serde(rename = "215")]
    #[strum(serialize = "215")]
    RestrictedCard,
    #[serde(rename = "216")]
    #[strum(serialize = "216")]
    InvalidDeactivate,
    #[serde(rename = "217")]
    #[strum(serialize = "217")]
    CardAlreadyActive,
    #[serde(rename = "218")]
    #[strum(serialize = "218")]
    CardNotActive,
    #[serde(rename = "219")]
    #[strum(serialize = "219")]
    CardAlreadyDeactivate,
    #[serde(rename = "221")]
    #[strum(serialize = "221")]
    OverMaxBalance,
    #[serde(rename = "222")]
    #[strum(serialize = "222")]
    InvalidActivate,
    #[serde(rename = "223")]
    #[strum(serialize = "223")]
    NoTransactionFoundForReversal,
    #[serde(rename = "226")]
    #[strum(serialize = "226")]
    IncorrectCvv,
    #[serde(rename = "229")]
    #[strum(serialize = "229")]
    IllegalTransaction,
    #[serde(rename = "251")]
    #[strum(serialize = "251")]
    DuplicateTransaction,
    #[serde(rename = "252")]
    #[strum(serialize = "252")]
    SystemError,
    #[serde(rename = "253")]
    #[strum(serialize = "253")]
    DeconvertedBin,
    #[serde(rename = "254")]
    #[strum(serialize = "254")]
    MerchantDepleted,
    #[serde(rename = "255")]
    #[strum(serialize = "255")]
    GiftCardEscheated,
    #[serde(rename = "256")]
    #[strum(serialize = "256")]
    InvalidReversalTypeForCreditCardTransaction,
    #[serde(rename = "257")]
    #[strum(serialize = "257")]
    SystemErrorMessageFormatError,
    #[serde(rename = "258")]
    #[strum(serialize = "258")]
    SystemErrorCannotProcess,
    #[serde(rename = "271")]
    #[strum(serialize = "271")]
    RefundRejectedDueToPendingDepositStatus,
    #[serde(rename = "272")]
    #[strum(serialize = "272")]
    RefundRejectedDueToDeclinedDepositStatus,
    #[serde(rename = "273")]
    #[strum(serialize = "273")]
    RefundRejectedByTheProcessingNetwork,
    #[serde(rename = "284")]
    #[strum(serialize = "284")]
    CaptureCreditAndAuthReversalTagsCannotBeUsedForGiftCardTransactions,
    #[serde(rename = "301")]
    #[strum(serialize = "301")]
    InvalidAccountNumber,
    #[serde(rename = "302")]
    #[strum(serialize = "302")]
    AccountNumberDoesNotMatchPaymentType,
    #[serde(rename = "303")]
    #[strum(serialize = "303")]
    PickUpCard,
    #[serde(rename = "304")]
    #[strum(serialize = "304")]
    LostStolenCard,
    #[serde(rename = "305")]
    #[strum(serialize = "305")]
    ExpiredCard,
    #[serde(rename = "306")]
    #[strum(serialize = "306")]
    AuthorizationHasExpiredNoNeedToReverse,
    #[serde(rename = "307")]
    #[strum(serialize = "307")]
    RestrictedCardSoftDecline,
    #[serde(rename = "308")]
    #[strum(serialize = "308")]
    RestrictedCardChargeback,
    #[serde(rename = "309")]
    #[strum(serialize = "309")]
    RestrictedCardPrepaidCardFilteringService,
    #[serde(rename = "310")]
    #[strum(serialize = "310")]
    InvalidTrackData,
    #[serde(rename = "311")]
    #[strum(serialize = "311")]
    DepositIsAlreadyReferencedByAChargeback,
    #[serde(rename = "312")]
    #[strum(serialize = "312")]
    RestrictedCardInternationalCardFilteringService,
    #[serde(rename = "313")]
    #[strum(serialize = "313")]
    InternationalFilteringForIssuingCardCountry,
    #[serde(rename = "315")]
    #[strum(serialize = "315")]
    RestrictedCardAuthFraudVelocityFilteringService,
    #[serde(rename = "316")]
    #[strum(serialize = "316")]
    AutomaticRefundAlreadyIssued,
    #[serde(rename = "317")]
    #[strum(serialize = "317")]
    RestrictedCardCardUnderSanction,
    #[serde(rename = "318")]
    #[strum(serialize = "318")]
    RestrictedCardAuthFraudAdviceFilteringService,
    #[serde(rename = "319")]
    #[strum(serialize = "319")]
    RestrictedCardFraudAvsFilteringService,
    #[serde(rename = "320")]
    #[strum(serialize = "320")]
    InvalidExpirationDate,
    #[serde(rename = "321")]
    #[strum(serialize = "321")]
    InvalidMerchant,
    #[serde(rename = "322")]
    #[strum(serialize = "322")]
    InvalidTransaction,
    #[serde(rename = "323")]
    #[strum(serialize = "323")]
    NoSuchIssuer,
    #[serde(rename = "324")]
    #[strum(serialize = "324")]
    InvalidPin,
    #[serde(rename = "325")]
    #[strum(serialize = "325")]
    TransactionNotAllowedAtTerminal,
    #[serde(rename = "326")]
    #[strum(serialize = "326")]
    ExceedsNumberOfPinEntries,
    #[serde(rename = "327")]
    #[strum(serialize = "327")]
    CardholderTransactionNotPermitted,
    #[serde(rename = "328")]
    #[strum(serialize = "328")]
    CardholderRequestedThatRecurringOrInstallmentPaymentBeStopped,
    #[serde(rename = "330")]
    #[strum(serialize = "330")]
    InvalidPaymentType,
    #[serde(rename = "331")]
    #[strum(serialize = "331")]
    InvalidPosCapabilityForCardholderAuthorizedTerminalTransaction,
    #[serde(rename = "332")]
    #[strum(serialize = "332")]
    InvalidPosCardholderIdForCardholderAuthorizedTerminalTransaction,
    #[serde(rename = "335")]
    #[strum(serialize = "335")]
    ThisMethodOfPaymentDoesNotSupportAuthorizationReversals,
    #[serde(rename = "336")]
    #[strum(serialize = "336")]
    ReversalAmountDoesNotMatchAuthorizationAmount,
    #[serde(rename = "337")]
    #[strum(serialize = "337")]
    TransactionDidNotConvertToPinless,
    #[serde(rename = "340")]
    #[strum(serialize = "340")]
    InvalidAmountSoftDecline,
    #[serde(rename = "341")]
    #[strum(serialize = "341")]
    InvalidHealthcareAmounts,
    #[serde(rename = "346")]
    #[strum(serialize = "346")]
    InvalidBillingDescriptorPrefix,
    #[serde(rename = "347")]
    #[strum(serialize = "347")]
    InvalidBillingDescriptor,
    #[serde(rename = "348")]
    #[strum(serialize = "348")]
    InvalidReportGroup,
    #[serde(rename = "349")]
    #[strum(serialize = "349")]
    DoNotHonor,
    #[serde(rename = "350")]
    #[strum(serialize = "350")]
    GenericDecline, // Soft or Hard Decline
    #[serde(rename = "351")]
    #[strum(serialize = "351")]
    DeclineRequestPositiveId,
    #[serde(rename = "352")]
    #[strum(serialize = "352")]
    DeclineCvv2CidFail,
    #[serde(rename = "354")]
    #[strum(serialize = "354")]
    ThreeDSecureTransactionNotSupportedByMerchant,
    #[serde(rename = "356")]
    #[strum(serialize = "356")]
    InvalidPurchaseLevelIiiTheTransactionContainedBadOrMissingData,
    #[serde(rename = "357")]
    #[strum(serialize = "357")]
    MissingHealthcareIiasTagForAnFsaTransaction,
    #[serde(rename = "358")]
    #[strum(serialize = "358")]
    RestrictedByVantivDueToSecurityCodeMismatch,
    #[serde(rename = "360")]
    #[strum(serialize = "360")]
    NoTransactionFoundWithSpecifiedTransactionId,
    #[serde(rename = "361")]
    #[strum(serialize = "361")]
    AuthorizationNoLongerAvailable,
    #[serde(rename = "362")]
    #[strum(serialize = "362")]
    TransactionNotVoidedAlreadySettled,
    #[serde(rename = "363")]
    #[strum(serialize = "363")]
    AutoVoidOnRefund,
    #[serde(rename = "364")]
    #[strum(serialize = "364")]
    InvalidAccountNumberOriginalOrNocUpdatedECheckAccountRequired,
    #[serde(rename = "365")]
    #[strum(serialize = "365")]
    TotalCreditAmountExceedsCaptureAmount,
    #[serde(rename = "366")]
    #[strum(serialize = "366")]
    ExceedTheThresholdForSendingRedeposits,
    #[serde(rename = "367")]
    #[strum(serialize = "367")]
    DepositHasNotBeenReturnedForInsufficientNonSufficientFunds,
    #[serde(rename = "368")]
    #[strum(serialize = "368")]
    InvalidCheckNumber,
    #[serde(rename = "369")]
    #[strum(serialize = "369")]
    RedepositAgainstInvalidTransactionType,
    #[serde(rename = "370")]
    #[strum(serialize = "370")]
    InternalSystemErrorCallVantiv,
    #[serde(rename = "371")]
    #[strum(serialize = "371")]
    OriginalTransactionHasBeenProcessedFutureRedepositsCanceled,
    #[serde(rename = "372")]
    #[strum(serialize = "372")]
    SoftDeclineAutoRecyclingInProgress,
    #[serde(rename = "373")]
    #[strum(serialize = "373")]
    HardDeclineAutoRecyclingComplete,
    #[serde(rename = "375")]
    #[strum(serialize = "375")]
    MerchantIsNotEnabledForSurcharging,
    #[serde(rename = "376")]
    #[strum(serialize = "376")]
    ThisMethodOfPaymentDoesNotSupportSurcharging,
    #[serde(rename = "377")]
    #[strum(serialize = "377")]
    SurchargeIsNotValidForDebitOrPrepaidCards,
    #[serde(rename = "378")]
    #[strum(serialize = "378")]
    SurchargeCannotExceedsTheMaximumAllowedLimit,
    #[serde(rename = "379")]
    #[strum(serialize = "379")]
    TransactionDeclinedByTheProcessingNetwork,
    #[serde(rename = "380")]
    #[strum(serialize = "380")]
    SecondaryAmountCannotExceedTheSaleAmount,
    #[serde(rename = "381")]
    #[strum(serialize = "381")]
    ThisMethodOfPaymentDoesNotSupportSecondaryAmount,
    #[serde(rename = "382")]
    #[strum(serialize = "382")]
    SecondaryAmountCannotBeLessThanZero,
    #[serde(rename = "383")]
    #[strum(serialize = "383")]
    PartialTransactionIsNotSupportedWhenIncludingASecondaryAmount,
    #[serde(rename = "384")]
    #[strum(serialize = "384")]
    SecondaryAmountRequiredOnPartialRefundWhenUsedOnDeposit,
    #[serde(rename = "385")]
    #[strum(serialize = "385")]
    SecondaryAmountNotAllowedOnRefundIfNotIncludedOnDeposit,
    #[serde(rename = "386")]
    #[strum(serialize = "386")]
    ProcessingNetworkError,
    #[serde(rename = "401")]
    #[strum(serialize = "401")]
    InvalidEMail,
    #[serde(rename = "466")]
    #[strum(serialize = "466")]
    InvalidCombinationOfAccountFundingTransactionTypeAndMcc,
    #[serde(rename = "467")]
    #[strum(serialize = "467")]
    InvalidAccountFundingTransactionTypeForThisMethodOfPayment,
    #[serde(rename = "468")]
    #[strum(serialize = "468")]
    MissingOneOrMoreReceiverFieldsForAccountFundingTransaction,
    #[serde(rename = "469")]
    #[strum(serialize = "469")]
    InvalidRecurringRequestSeeRecurringResponseForDetails,
    #[serde(rename = "470")]
    #[strum(serialize = "470")]
    ApprovedRecurringSubscriptionCreated,
    #[serde(rename = "471")]
    #[strum(serialize = "471")]
    ParentTransactionDeclinedRecurringSubscriptionNotCreated,
    #[serde(rename = "472")]
    #[strum(serialize = "472")]
    InvalidPlanCode,
    #[serde(rename = "473")]
    #[strum(serialize = "473")]
    ScheduledRecurringPaymentProcessed,
    #[serde(rename = "475")]
    #[strum(serialize = "475")]
    InvalidSubscriptionId,
    #[serde(rename = "476")]
    #[strum(serialize = "476")]
    AddOnCodeAlreadyExists,
    #[serde(rename = "477")]
    #[strum(serialize = "477")]
    DuplicateAddOnCodesInRequests,
    #[serde(rename = "478")]
    #[strum(serialize = "478")]
    NoMatchingAddOnCodeForTheSubscription,
    #[serde(rename = "480")]
    #[strum(serialize = "480")]
    NoMatchingDiscountCodeForTheSubscription,
    #[serde(rename = "481")]
    #[strum(serialize = "481")]
    DuplicateDiscountCodesInRequest,
    #[serde(rename = "482")]
    #[strum(serialize = "482")]
    InvalidStartDate,
    #[serde(rename = "483")]
    #[strum(serialize = "483")]
    MerchantNotRegisteredForRecurringEngine,
    #[serde(rename = "484")]
    #[strum(serialize = "484")]
    InsufficientDataToUpdateSubscription,
    #[serde(rename = "485")]
    #[strum(serialize = "485")]
    InvalidBillingDate,
    #[serde(rename = "486")]
    #[strum(serialize = "486")]
    DiscountCodeAlreadyExists,
    #[serde(rename = "487")]
    #[strum(serialize = "487")]
    PlanCodeAlreadyExists,
    #[serde(rename = "500")]
    #[strum(serialize = "500")]
    TheAccountNumberWasChanged,
    #[serde(rename = "501")]
    #[strum(serialize = "501")]
    TheAccountWasClosed,
    #[serde(rename = "502")]
    #[strum(serialize = "502")]
    TheExpirationDateWasChanged,
    #[serde(rename = "503")]
    #[strum(serialize = "503")]
    TheIssuingBankDoesNotParticipateInTheUpdateProgram,
    #[serde(rename = "504")]
    #[strum(serialize = "504")]
    ContactTheCardholderForUpdatedInformation,
    #[serde(rename = "507")]
    #[strum(serialize = "507")]
    TheCardholderHasOptedOutOfTheUpdateProgram,
    #[serde(rename = "521")]
    #[strum(serialize = "521")]
    SoftDeclineCardReaderDecryptionServiceIsNotAvailable,
    #[serde(rename = "523")]
    #[strum(serialize = "523")]
    SoftDeclineDecryptionFailed,
    #[serde(rename = "524")]
    #[strum(serialize = "524")]
    HardDeclineInputDataIsInvalid,
    #[serde(rename = "530")]
    #[strum(serialize = "530")]
    ApplePayKeyMismatch,
    #[serde(rename = "531")]
    #[strum(serialize = "531")]
    ApplePayDecryptionFailed,
    #[serde(rename = "540")]
    #[strum(serialize = "540")]
    HardDeclineDecryptionFailed,
    #[serde(rename = "550")]
    #[strum(serialize = "550")]
    AdvancedFraudFilterScoreBelowThreshold,
    #[serde(rename = "555")]
    #[strum(serialize = "555")]
    SuspectedFraud,
    #[serde(rename = "560")]
    #[strum(serialize = "560")]
    SystemErrorContactWorldpayRepresentative,
    #[serde(rename = "561")]
    #[strum(serialize = "561")]
    AmazonPayAmazonUnavailable,
    #[serde(rename = "562")]
    #[strum(serialize = "562")]
    AmazonPayAmazonDeclined,
    #[serde(rename = "563")]
    #[strum(serialize = "563")]
    AmazonPayInvalidToken,
    #[serde(rename = "564")]
    #[strum(serialize = "564")]
    MerchantNotEnabledForAmazonPay,
    #[serde(rename = "565")]
    #[strum(serialize = "565")]
    TransactionNotSupportedBlockedByIssuer,
    #[serde(rename = "566")]
    #[strum(serialize = "566")]
    BlockedByCardholderContactCardholder,
    #[serde(rename = "601")]
    #[strum(serialize = "601")]
    SoftDeclinePrimaryFundingSourceFailed,
    #[serde(rename = "602")]
    #[strum(serialize = "602")]
    SoftDeclineBuyerHasAlternateFundingSource,
    #[serde(rename = "610")]
    #[strum(serialize = "610")]
    HardDeclineInvalidBillingAgreementId,
    #[serde(rename = "611")]
    #[strum(serialize = "611")]
    HardDeclinePrimaryFundingSourceFailed,
    #[serde(rename = "612")]
    #[strum(serialize = "612")]
    HardDeclineIssueWithPaypalAccount,
    #[serde(rename = "613")]
    #[strum(serialize = "613")]
    HardDeclinePayPalAuthorizationIdMissing,
    #[serde(rename = "614")]
    #[strum(serialize = "614")]
    HardDeclineConfirmedEmailAddressIsNotAvailable,
    #[serde(rename = "615")]
    #[strum(serialize = "615")]
    HardDeclinePayPalBuyerAccountDenied,
    #[serde(rename = "616")]
    #[strum(serialize = "616")]
    HardDeclinePayPalBuyerAccountRestricted,
    #[serde(rename = "617")]
    #[strum(serialize = "617")]
    HardDeclinePayPalOrderHasBeenVoidedExpiredOrCompleted,
    #[serde(rename = "618")]
    #[strum(serialize = "618")]
    HardDeclineIssueWithPayPalRefund,
    #[serde(rename = "619")]
    #[strum(serialize = "619")]
    HardDeclinePayPalCredentialsIssue,
    #[serde(rename = "620")]
    #[strum(serialize = "620")]
    HardDeclinePayPalAuthorizationVoidedOrExpired,
    #[serde(rename = "621")]
    #[strum(serialize = "621")]
    HardDeclineRequiredPayPalParameterMissing,
    #[serde(rename = "622")]
    #[strum(serialize = "622")]
    HardDeclinePayPalTransactionIdOrAuthIdIsInvalid,
    #[serde(rename = "623")]
    #[strum(serialize = "623")]
    HardDeclineExceededMaximumNumberOfPayPalAuthorizationAttempts,
    #[serde(rename = "624")]
    #[strum(serialize = "624")]
    HardDeclineTransactionAmountExceedsMerchantsPayPalAccountLimit,
    #[serde(rename = "625")]
    #[strum(serialize = "625")]
    HardDeclinePayPalFundingSourcesUnavailable,
    #[serde(rename = "626")]
    #[strum(serialize = "626")]
    HardDeclineIssueWithPayPalPrimaryFundingSource,
    #[serde(rename = "627")]
    #[strum(serialize = "627")]
    HardDeclinePayPalProfileDoesNotAllowThisTransactionType,
    #[serde(rename = "628")]
    #[strum(serialize = "628")]
    InternalSystemErrorWithPayPalContactVantiv,
    #[serde(rename = "629")]
    #[strum(serialize = "629")]
    HardDeclineContactPayPalConsumerForAnotherPaymentMethod,
    #[serde(rename = "637")]
    #[strum(serialize = "637")]
    InvalidTerminalId,
    #[serde(rename = "640")]
    #[strum(serialize = "640")]
    PinlessDebitProcessingNotSupportedForNonRecurringTransactions,
    #[serde(rename = "641")]
    #[strum(serialize = "641")]
    PinlessDebitProcessingNotSupportedForPartialAuths,
    #[serde(rename = "642")]
    #[strum(serialize = "642")]
    MerchantNotConfiguredForPinlessDebitProcessing,
    #[serde(rename = "651")]
    #[strum(serialize = "651")]
    DeclineCustomerCancellation,
    #[serde(rename = "652")]
    #[strum(serialize = "652")]
    DeclineReTryTransaction,
    #[serde(rename = "653")]
    #[strum(serialize = "653")]
    DeclineUnableToLocateRecordOnFile,
    #[serde(rename = "654")]
    #[strum(serialize = "654")]
    DeclineFileUpdateFieldEditError,
    #[serde(rename = "655")]
    #[strum(serialize = "655")]
    RemoteFunctionUnknown,
    #[serde(rename = "656")]
    #[strum(serialize = "656")]
    DeclinedExceedsWithdrawalFrequencyLimit,
    #[serde(rename = "657")]
    #[strum(serialize = "657")]
    DeclineCardRecordNotAvailable,
    #[serde(rename = "658")]
    #[strum(serialize = "658")]
    InvalidAuthorizationCode,
    #[serde(rename = "659")]
    #[strum(serialize = "659")]
    ReconciliationError,
    #[serde(rename = "660")]
    #[strum(serialize = "660")]
    PreferredDebitRoutingDenialCreditTransactionCanBeDebit,
    #[serde(rename = "661")]
    #[strum(serialize = "661")]
    DeclinedCurrencyConversionCompleteNoAuthPerformed,
    #[serde(rename = "662")]
    #[strum(serialize = "662")]
    DeclinedMultiCurrencyDccFail,
    #[serde(rename = "663")]
    #[strum(serialize = "663")]
    DeclinedMultiCurrencyInvertFail,
    #[serde(rename = "664")]
    #[strum(serialize = "664")]
    Invalid3DSecurePassword,
    #[serde(rename = "665")]
    #[strum(serialize = "665")]
    InvalidSocialSecurityNumber,
    #[serde(rename = "666")]
    #[strum(serialize = "666")]
    InvalidMothersMaidenName,
    #[serde(rename = "667")]
    #[strum(serialize = "667")]
    EnrollmentInquiryDeclined,
    #[serde(rename = "668")]
    #[strum(serialize = "668")]
    SocialSecurityNumberNotAvailable,
    #[serde(rename = "669")]
    #[strum(serialize = "669")]
    MothersMaidenNameNotAvailable,
    #[serde(rename = "670")]
    #[strum(serialize = "670")]
    PinAlreadyExistsOnDatabase,
    #[serde(rename = "701")]
    #[strum(serialize = "701")]
    Under18YearsOld,
    #[serde(rename = "702")]
    #[strum(serialize = "702")]
    BillToOutsideUsa,
    #[serde(rename = "703")]
    #[strum(serialize = "703")]
    BillToAddressIsNotEqualToShipToAddress,
    #[serde(rename = "704")]
    #[strum(serialize = "704")]
    DeclinedForeignCurrencyMustBeUsd,
    #[serde(rename = "705")]
    #[strum(serialize = "705")]
    OnNegativeFile,
    #[serde(rename = "706")]
    #[strum(serialize = "706")]
    BlockedAgreement,
    #[serde(rename = "707")]
    #[strum(serialize = "707")]
    InsufficientBuyingPower,
    #[serde(rename = "708")]
    #[strum(serialize = "708")]
    InvalidData,
    #[serde(rename = "709")]
    #[strum(serialize = "709")]
    InvalidDataDataElementsMissing,
    #[serde(rename = "710")]
    #[strum(serialize = "710")]
    InvalidDataDataFormatError,
    #[serde(rename = "711")]
    #[strum(serialize = "711")]
    InvalidDataInvalidTCVersion,
    #[serde(rename = "712")]
    #[strum(serialize = "712")]
    DuplicateTransactionPaypalCredit,
    #[serde(rename = "713")]
    #[strum(serialize = "713")]
    VerifyBillingAddress,
    #[serde(rename = "714")]
    #[strum(serialize = "714")]
    InactiveAccount,
    #[serde(rename = "716")]
    #[strum(serialize = "716")]
    InvalidAuth,
    #[serde(rename = "717")]
    #[strum(serialize = "717")]
    AuthorizationAlreadyExistsForTheOrder,
    #[serde(rename = "730")]
    #[strum(serialize = "730")]
    LodgingTransactionsAreNotAllowedForThisMcc,
    #[serde(rename = "731")]
    #[strum(serialize = "731")]
    DurationCannotBeNegative,
    #[serde(rename = "732")]
    #[strum(serialize = "732")]
    HotelFolioNumberCannotBeBlank,
    #[serde(rename = "733")]
    #[strum(serialize = "733")]
    InvalidCheckInDate,
    #[serde(rename = "734")]
    #[strum(serialize = "734")]
    InvalidCheckOutDate,
    #[serde(rename = "735")]
    #[strum(serialize = "735")]
    InvalidCheckInOrCheckOutDate,
    #[serde(rename = "736")]
    #[strum(serialize = "736")]
    CheckOutDateCannotBeBeforeCheckInDate,
    #[serde(rename = "737")]
    #[strum(serialize = "737")]
    NumberOfAdultsCannotBeNegative,
    #[serde(rename = "738")]
    #[strum(serialize = "738")]
    RoomRateCannotBeNegative,
    #[serde(rename = "739")]
    #[strum(serialize = "739")]
    RoomTaxCannotBeNegative,
    #[serde(rename = "740")]
    #[strum(serialize = "740")]
    DurationCanOnlyBeFrom0To99ForVisa,
    #[serde(rename = "801")]
    #[strum(serialize = "801")]
    AccountNumberWasSuccessfullyRegistered,
    #[serde(rename = "802")]
    #[strum(serialize = "802")]
    AccountNumberWasPreviouslyRegistered,
    #[serde(rename = "803")]
    #[strum(serialize = "803")]
    ValidToken,
    #[serde(rename = "820")]
    #[strum(serialize = "820")]
    CreditCardNumberWasInvalid,
    #[serde(rename = "821")]
    #[strum(serialize = "821")]
    MerchantIsNotAuthorizedForTokens,
    #[serde(rename = "822")]
    #[strum(serialize = "822")]
    TokenWasNotFound,
    #[serde(rename = "823")]
    #[strum(serialize = "823")]
    TokenInvalid,
    #[serde(rename = "825")]
    #[strum(serialize = "825")]
    MerchantNotAuthorizedForECheckTokens,
    #[serde(rename = "826")]
    #[strum(serialize = "826")]
    CheckoutIdWasInvalid,
    #[serde(rename = "827")]
    #[strum(serialize = "827")]
    CheckoutIdWasNotFound,
    #[serde(rename = "828")]
    #[strum(serialize = "828")]
    GenericCheckoutIdError,
    #[serde(rename = "835")]
    #[strum(serialize = "835")]
    CaptureAmountCanNotBeMoreThanAuthorizedAmount,
    #[serde(rename = "850")]
    #[strum(serialize = "850")]
    TaxBillingOnlyAllowedForMcc9311,
    #[serde(rename = "851")]
    #[strum(serialize = "851")]
    Mcc9311RequiresTaxTypeElement,
    #[serde(rename = "852")]
    #[strum(serialize = "852")]
    DebtRepaymentOnlyAllowedForViTransactionsOnMccs6012And6051,
    #[serde(rename = "861")]
    #[strum(serialize = "861")]
    RoutingNumberDidNotMatchOneOnFileForToken,
    #[serde(rename = "877")]
    #[strum(serialize = "877")]
    InvalidPayPageRegistrationId,
    #[serde(rename = "878")]
    #[strum(serialize = "878")]
    ExpiredPayPageRegistrationId,
    #[serde(rename = "879")]
    #[strum(serialize = "879")]
    MerchantIsNotAuthorizedForPayPage,
    #[serde(rename = "890")]
    #[strum(serialize = "890")]
    MaximumNumberOfUpdatesForThisTokenExceeded,
    #[serde(rename = "891")]
    #[strum(serialize = "891")]
    TooManyTokensCreatedForExistingNamespace,
    #[serde(rename = "895")]
    #[strum(serialize = "895")]
    PinValidationNotPossible,
    #[serde(rename = "898")]
    #[strum(serialize = "898")]
    GenericTokenRegistrationError,
    #[serde(rename = "899")]
    #[strum(serialize = "899")]
    GenericTokenUseError,
    #[serde(rename = "900")]
    #[strum(serialize = "900")]
    InvalidBankRoutingNumber,
    #[serde(rename = "901")]
    #[strum(serialize = "901")]
    MissingName,
    #[serde(rename = "902")]
    #[strum(serialize = "902")]
    InvalidName,
    #[serde(rename = "903")]
    #[strum(serialize = "903")]
    MissingBillingCountryCode,
    #[serde(rename = "904")]
    #[strum(serialize = "904")]
    InvalidIban,
    #[serde(rename = "905")]
    #[strum(serialize = "905")]
    MissingEmailAddress,
    #[serde(rename = "906")]
    #[strum(serialize = "906")]
    MissingMandateReference,
    #[serde(rename = "907")]
    #[strum(serialize = "907")]
    InvalidMandateReference,
    #[serde(rename = "908")]
    #[strum(serialize = "908")]
    MissingMandateUrl,
    #[serde(rename = "909")]
    #[strum(serialize = "909")]
    InvalidMandateUrl,
    #[serde(rename = "911")]
    #[strum(serialize = "911")]
    MissingMandateSignatureDate,
    #[serde(rename = "912")]
    #[strum(serialize = "912")]
    InvalidMandateSignatureDate,
    #[serde(rename = "913")]
    #[strum(serialize = "913")]
    RecurringMandateAlreadyExists,
    #[serde(rename = "914")]
    #[strum(serialize = "914")]
    RecurringMandateWasNotFound,
    #[serde(rename = "915")]
    #[strum(serialize = "915")]
    FinalRecurringWasAlreadyReceivedUsingThisMandate,
    #[serde(rename = "916")]
    #[strum(serialize = "916")]
    IbanDidNotMatchOneOnFileForMandate,
    #[serde(rename = "917")]
    #[strum(serialize = "917")]
    InvalidBillingCountry,
    #[serde(rename = "922")]
    #[strum(serialize = "922")]
    ExpirationDateRequiredForInteracTransaction,
    #[serde(rename = "923")]
    #[strum(serialize = "923")]
    TransactionTypeIsNotSupportedWithThisMethodOfPayment,
    #[serde(rename = "924")]
    #[strum(serialize = "924")]
    UnreferencedOrphanRefundsAreNotAllowed,
    #[serde(rename = "939")]
    #[strum(serialize = "939")]
    UnableToVoidATransactionWithAHeldState,
    #[serde(rename = "940")]
    #[strum(serialize = "940")]
    ThisFundingInstructionResultsInANegativeAccountBalance,
    #[serde(rename = "941")]
    #[strum(serialize = "941")]
    AccountBalanceInformationUnavailableAtThisTime,
    #[serde(rename = "942")]
    #[strum(serialize = "942")]
    TheSubmittedCardIsNotEligibleForFastAccessFunding,
    #[serde(rename = "943")]
    #[strum(serialize = "943")]
    TransactionCannotUseBothCcdPaymentInformationAndCtxPaymentInformation,
    #[serde(rename = "944")]
    #[strum(serialize = "944")]
    ProcessingError,
    #[serde(rename = "945")]
    #[strum(serialize = "945")]
    ThisFundingInstructionTypeIsInvalidForCanadianMerchants,
    #[serde(rename = "946")]
    #[strum(serialize = "946")]
    CtxAndCcdRecordsAreNotAllowedForCanadianMerchants,
    #[serde(rename = "947")]
    #[strum(serialize = "947")]
    CanadianAccountNumberCannotExceed12Digits,
    #[serde(rename = "948")]
    #[strum(serialize = "948")]
    ThisFundingInstructionTypeIsInvalid,
    #[serde(rename = "950")]
    #[strum(serialize = "950")]
    DeclineNegativeInformationOnFile,
    #[serde(rename = "951")]
    #[strum(serialize = "951")]
    AbsoluteDecline,
    #[serde(rename = "952")]
    #[strum(serialize = "952")]
    TheMerchantProfileDoesNotAllowTheRequestedOperation,
    #[serde(rename = "953")]
    #[strum(serialize = "953")]
    TheAccountCannotAcceptAchTransactions,
    #[serde(rename = "954")]
    #[strum(serialize = "954")]
    TheAccountCannotAcceptAchTransactionsOrSiteDrafts,
    #[serde(rename = "955")]
    #[strum(serialize = "955")]
    AmountGreaterThanLimitSpecifiedInTheMerchantProfile,
    #[serde(rename = "956")]
    #[strum(serialize = "956")]
    MerchantIsNotAuthorizedToPerformECheckVerificationTransactions,
    #[serde(rename = "957")]
    #[strum(serialize = "957")]
    FirstNameAndLastNameRequiredForECheckVerifications,
    #[serde(rename = "958")]
    #[strum(serialize = "958")]
    CompanyNameRequiredForCorporateAccountForECheckVerifications,
    #[serde(rename = "959")]
    #[strum(serialize = "959")]
    PhoneNumberRequiredForECheckVerifications,
    #[serde(rename = "961")]
    #[strum(serialize = "961")]
    CardBrandTokenNotSupported,
    #[serde(rename = "962")]
    #[strum(serialize = "962")]
    PrivateLabelCardNotSupported,
    #[serde(rename = "965")]
    #[strum(serialize = "965")]
    AllowedDailyDirectDebitCaptureECheckSaleLimitExceeded,
    #[serde(rename = "966")]
    #[strum(serialize = "966")]
    AllowedDailyDirectDebitCreditECheckCreditLimitExceeded,
    #[serde(rename = "973")]
    #[strum(serialize = "973")]
    AccountNotEligibleForRtp,
    #[serde(rename = "980")]
    #[strum(serialize = "980")]
    SoftDeclineCustomerAuthenticationRequired,
    #[serde(rename = "981")]
    #[strum(serialize = "981")]
    TransactionNotReversedVoidWorkflowNeedToBeInvoked,
    #[serde(rename = "982")]
    #[strum(serialize = "982")]
    TransactionReversalNotSupportedForTheCoreMerchants,
    #[serde(rename = "983")]
    #[strum(serialize = "983")]
    NoValidParentDepositOrParentRefundFound,
    #[serde(rename = "984")]
    #[strum(serialize = "984")]
    TransactionReversalNotEnabledForVisa,
    #[serde(rename = "985")]
    #[strum(serialize = "985")]
    TransactionReversalNotEnabledForMastercard,
    #[serde(rename = "986")]
    #[strum(serialize = "986")]
    TransactionReversalNotEnabledForAmEx,
    #[serde(rename = "987")]
    #[strum(serialize = "987")]
    TransactionReversalNotEnabledForDiscover,
    #[serde(rename = "988")]
    #[strum(serialize = "988")]
    TransactionReversalNotSupported,
    #[serde(rename = "990")]
    #[strum(serialize = "990")]
    FundingInstructionHeldPleaseContactYourRelationshipManager,
    #[serde(rename = "991")]
    #[strum(serialize = "991")]
    MissingAddressInformation,
    #[serde(rename = "992")]
    #[strum(serialize = "992")]
    CryptographicFailure,
    #[serde(rename = "993")]
    #[strum(serialize = "993")]
    InvalidRegionCode,
    #[serde(rename = "994")]
    #[strum(serialize = "994")]
    InvalidCountryCode,
    #[serde(rename = "995")]
    #[strum(serialize = "995")]
    InvalidCreditAccount,
    #[serde(rename = "996")]
    #[strum(serialize = "996")]
    InvalidCheckingAccount,
    #[serde(rename = "997")]
    #[strum(serialize = "997")]
    InvalidSavingsAccount,
    #[serde(rename = "998")]
    #[strum(serialize = "998")]
    InvalidUseOfMccCorrectAndReattempt,
    #[serde(rename = "999")]
    #[strum(serialize = "999")]
    ExceedsRtpTransactionLimit,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum WorldpayvantivPaymentFlow {
    Sale,
    Auth,
    Capture,
    Void,
    //VoidPostCapture
    VoidPC,
}

fn get_payment_flow_type(input: &str) -> Result<WorldpayvantivPaymentFlow, errors::ConnectorError> {
    if input.contains("auth") {
        Ok(WorldpayvantivPaymentFlow::Auth)
    } else if input.contains("sale") {
        Ok(WorldpayvantivPaymentFlow::Sale)
    } else if input.contains("voidpc") {
        Ok(WorldpayvantivPaymentFlow::VoidPC)
    } else if input.contains("void") {
        Ok(WorldpayvantivPaymentFlow::Void)
    } else if input.contains("capture") {
        Ok(WorldpayvantivPaymentFlow::Capture)
    } else {
        Err(errors::ConnectorError::UnexpectedResponseError(
            bytes::Bytes::from("Invalid merchantTxnId".to_string()),
        ))
    }
}

fn get_attempt_status(
    flow: WorldpayvantivPaymentFlow,
    response: WorldpayvantivResponseCode,
) -> Result<common_enums::AttemptStatus, errors::ConnectorError> {
    match response {
        WorldpayvantivResponseCode::Approved
            | WorldpayvantivResponseCode::PartiallyApproved
            | WorldpayvantivResponseCode::OfflineApproval
            | WorldpayvantivResponseCode::OfflineApprovalUnableToGoOnline
            | WorldpayvantivResponseCode::ConsumerNonReloadablePrepaidCardApproved
            | WorldpayvantivResponseCode::ConsumerSingleUseVirtualCardNumberApproved
            | WorldpayvantivResponseCode::ScheduledRecurringPaymentProcessed
            | WorldpayvantivResponseCode::ApprovedRecurringSubscriptionCreated
            | WorldpayvantivResponseCode::PendingShopperCheckoutCompletion
            | WorldpayvantivResponseCode::TransactionReceived
            | WorldpayvantivResponseCode::AccountNumberWasSuccessfullyRegistered
            | WorldpayvantivResponseCode::AccountNumberWasPreviouslyRegistered
            | WorldpayvantivResponseCode::ValidToken
             => match flow {
                WorldpayvantivPaymentFlow::Sale => Ok(common_enums::AttemptStatus::Pending),
                WorldpayvantivPaymentFlow::Auth => Ok(common_enums::AttemptStatus::Authorizing),
                WorldpayvantivPaymentFlow::Capture => Ok(common_enums::AttemptStatus::CaptureInitiated),
                WorldpayvantivPaymentFlow::Void => Ok(common_enums::AttemptStatus::VoidInitiated),
                WorldpayvantivPaymentFlow::VoidPC => {
                    Ok(common_enums::AttemptStatus::VoidInitiated)
                }
            },
        WorldpayvantivResponseCode::ShopperCheckoutExpired
            | WorldpayvantivResponseCode::ProcessingNetworkUnavailable
            | WorldpayvantivResponseCode::IssuerUnavailable
            | WorldpayvantivResponseCode::ReSubmitTransaction
            | WorldpayvantivResponseCode::TryAgainLater
            | WorldpayvantivResponseCode::InsufficientFunds
            | WorldpayvantivResponseCode::AuthorizationAmountHasAlreadyBeenDepleted
            | WorldpayvantivResponseCode::InsufficientFundsRetryAfter1Hour
            | WorldpayvantivResponseCode::InsufficientFundsRetryAfter24Hour
            | WorldpayvantivResponseCode::InsufficientFundsRetryAfter2Days
            | WorldpayvantivResponseCode::InsufficientFundsRetryAfter4Days
            | WorldpayvantivResponseCode::InsufficientFundsRetryAfter6Days
            | WorldpayvantivResponseCode::InsufficientFundsRetryAfter8Days
            | WorldpayvantivResponseCode::InsufficientFundsRetryAfter10Days
            | WorldpayvantivResponseCode::CallIssuer
            | WorldpayvantivResponseCode::CallAmex
            | WorldpayvantivResponseCode::CallDinersClub
            | WorldpayvantivResponseCode::CallDiscover
            | WorldpayvantivResponseCode::CallJbs
            | WorldpayvantivResponseCode::CallVisaMastercard
            | WorldpayvantivResponseCode::CallIssuerUpdateCardholderData
            | WorldpayvantivResponseCode::ExceedsApprovalAmountLimit
            | WorldpayvantivResponseCode::CallIndicatedNumber
            | WorldpayvantivResponseCode::UnacceptablePinTransactionDeclinedRetry
            | WorldpayvantivResponseCode::PinNotChanged
            | WorldpayvantivResponseCode::ConsumerMultiUseVirtualCardNumberSoftDecline
            | WorldpayvantivResponseCode::ConsumerNonReloadablePrepaidCardSoftDecline
            | WorldpayvantivResponseCode::ConsumerSingleUseVirtualCardNumberSoftDecline
            | WorldpayvantivResponseCode::UpdateCardholderData
            | WorldpayvantivResponseCode::MerchantDoesntQualifyForProductCode
            | WorldpayvantivResponseCode::Lifecycle
            | WorldpayvantivResponseCode::Policy
            | WorldpayvantivResponseCode::FraudSecurity
            | WorldpayvantivResponseCode::InvalidOrExpiredCardContactCardholderToUpdate
            | WorldpayvantivResponseCode::InvalidTransactionOrCardRestrictionVerifyInformationAndResubmit
            | WorldpayvantivResponseCode::AtLeastOneOfOrigIdOrOrigCnpTxnIdIsRequired
            | WorldpayvantivResponseCode::OrigCnpTxnIdIsRequiredWhenShowStatusOnlyIsUsed
            | WorldpayvantivResponseCode::IncrementalAuthNotSupported
            | WorldpayvantivResponseCode::SetAuthIndicatorToIncremental
            | WorldpayvantivResponseCode::IncrementalValueForAuthIndicatorNotAllowedInThisAuthStructure
            | WorldpayvantivResponseCode::CannotRequestAnIncrementalAuthIfOriginalAuthNotSetToEstimated
            | WorldpayvantivResponseCode::TransactionMustReferenceTheEstimatedAuth
            | WorldpayvantivResponseCode::IncrementedAuthExceedsMaxTransactionAmount
            | WorldpayvantivResponseCode::SubmittedMccNotAllowed
            | WorldpayvantivResponseCode::MerchantNotCertifiedEnabledForIias
            | WorldpayvantivResponseCode::IssuerGeneratedError
            | WorldpayvantivResponseCode::PickupCardOtherThanLostStolen
            | WorldpayvantivResponseCode::InvalidAmountHardDecline
            | WorldpayvantivResponseCode::ReversalUnsuccessful
            | WorldpayvantivResponseCode::MissingData
            | WorldpayvantivResponseCode::PickupCardLostCard
            | WorldpayvantivResponseCode::PickupCardStolenCard
            | WorldpayvantivResponseCode::RestrictedCard
            | WorldpayvantivResponseCode::InvalidDeactivate
            | WorldpayvantivResponseCode::CardAlreadyActive
            | WorldpayvantivResponseCode::CardNotActive
            | WorldpayvantivResponseCode::CardAlreadyDeactivate
            | WorldpayvantivResponseCode::OverMaxBalance
            | WorldpayvantivResponseCode::InvalidActivate
            | WorldpayvantivResponseCode::NoTransactionFoundForReversal
            | WorldpayvantivResponseCode::IncorrectCvv
            | WorldpayvantivResponseCode::IllegalTransaction
            | WorldpayvantivResponseCode::DuplicateTransaction
            | WorldpayvantivResponseCode::SystemError
            | WorldpayvantivResponseCode::DeconvertedBin
            | WorldpayvantivResponseCode::MerchantDepleted
            | WorldpayvantivResponseCode::GiftCardEscheated
            | WorldpayvantivResponseCode::InvalidReversalTypeForCreditCardTransaction
            | WorldpayvantivResponseCode::SystemErrorMessageFormatError
            | WorldpayvantivResponseCode::SystemErrorCannotProcess
            | WorldpayvantivResponseCode::RefundRejectedDueToPendingDepositStatus
            | WorldpayvantivResponseCode::RefundRejectedDueToDeclinedDepositStatus
            | WorldpayvantivResponseCode::RefundRejectedByTheProcessingNetwork
            | WorldpayvantivResponseCode::CaptureCreditAndAuthReversalTagsCannotBeUsedForGiftCardTransactions
            | WorldpayvantivResponseCode::InvalidAccountNumber
            | WorldpayvantivResponseCode::AccountNumberDoesNotMatchPaymentType
            | WorldpayvantivResponseCode::PickUpCard
            | WorldpayvantivResponseCode::LostStolenCard
            | WorldpayvantivResponseCode::ExpiredCard
            | WorldpayvantivResponseCode::AuthorizationHasExpiredNoNeedToReverse
            | WorldpayvantivResponseCode::RestrictedCardSoftDecline
            | WorldpayvantivResponseCode::RestrictedCardChargeback
            | WorldpayvantivResponseCode::RestrictedCardPrepaidCardFilteringService
            | WorldpayvantivResponseCode::InvalidTrackData
            | WorldpayvantivResponseCode::DepositIsAlreadyReferencedByAChargeback
            | WorldpayvantivResponseCode::RestrictedCardInternationalCardFilteringService
            | WorldpayvantivResponseCode::InternationalFilteringForIssuingCardCountry
            | WorldpayvantivResponseCode::RestrictedCardAuthFraudVelocityFilteringService
            | WorldpayvantivResponseCode::AutomaticRefundAlreadyIssued
            | WorldpayvantivResponseCode::RestrictedCardAuthFraudAdviceFilteringService
            | WorldpayvantivResponseCode::RestrictedCardFraudAvsFilteringService
            |  WorldpayvantivResponseCode::InvalidExpirationDate
            | WorldpayvantivResponseCode::InvalidMerchant
            | WorldpayvantivResponseCode::InvalidTransaction
            | WorldpayvantivResponseCode::NoSuchIssuer
            | WorldpayvantivResponseCode::InvalidPin
            | WorldpayvantivResponseCode::TransactionNotAllowedAtTerminal
            | WorldpayvantivResponseCode::ExceedsNumberOfPinEntries
            | WorldpayvantivResponseCode::CardholderTransactionNotPermitted
            | WorldpayvantivResponseCode::CardholderRequestedThatRecurringOrInstallmentPaymentBeStopped
            | WorldpayvantivResponseCode::InvalidPaymentType
            | WorldpayvantivResponseCode::InvalidPosCapabilityForCardholderAuthorizedTerminalTransaction
            | WorldpayvantivResponseCode::InvalidPosCardholderIdForCardholderAuthorizedTerminalTransaction
            | WorldpayvantivResponseCode::ThisMethodOfPaymentDoesNotSupportAuthorizationReversals
            | WorldpayvantivResponseCode::ReversalAmountDoesNotMatchAuthorizationAmount
            | WorldpayvantivResponseCode::TransactionDidNotConvertToPinless
            | WorldpayvantivResponseCode::InvalidAmountSoftDecline
            | WorldpayvantivResponseCode::InvalidHealthcareAmounts
            | WorldpayvantivResponseCode::InvalidBillingDescriptorPrefix
            | WorldpayvantivResponseCode::InvalidBillingDescriptor
            | WorldpayvantivResponseCode::InvalidReportGroup
            | WorldpayvantivResponseCode::DoNotHonor
            | WorldpayvantivResponseCode::GenericDecline
            | WorldpayvantivResponseCode::DeclineRequestPositiveId
            | WorldpayvantivResponseCode::DeclineCvv2CidFail
            | WorldpayvantivResponseCode::ThreeDSecureTransactionNotSupportedByMerchant
            | WorldpayvantivResponseCode::InvalidPurchaseLevelIiiTheTransactionContainedBadOrMissingData
            | WorldpayvantivResponseCode::MissingHealthcareIiasTagForAnFsaTransaction
            | WorldpayvantivResponseCode::RestrictedByVantivDueToSecurityCodeMismatch
            | WorldpayvantivResponseCode::NoTransactionFoundWithSpecifiedTransactionId
            | WorldpayvantivResponseCode::AuthorizationNoLongerAvailable
            | WorldpayvantivResponseCode::TransactionNotVoidedAlreadySettled
            | WorldpayvantivResponseCode::AutoVoidOnRefund
            | WorldpayvantivResponseCode::InvalidAccountNumberOriginalOrNocUpdatedECheckAccountRequired
            | WorldpayvantivResponseCode::TotalCreditAmountExceedsCaptureAmount
            | WorldpayvantivResponseCode::ExceedTheThresholdForSendingRedeposits
            | WorldpayvantivResponseCode::DepositHasNotBeenReturnedForInsufficientNonSufficientFunds
            | WorldpayvantivResponseCode::InvalidCheckNumber
            | WorldpayvantivResponseCode::RedepositAgainstInvalidTransactionType
            | WorldpayvantivResponseCode::InternalSystemErrorCallVantiv
            | WorldpayvantivResponseCode::OriginalTransactionHasBeenProcessedFutureRedepositsCanceled
            | WorldpayvantivResponseCode::SoftDeclineAutoRecyclingInProgress
            | WorldpayvantivResponseCode::HardDeclineAutoRecyclingComplete
            | WorldpayvantivResponseCode::RestrictedCardCardUnderSanction
            | WorldpayvantivResponseCode::MerchantIsNotEnabledForSurcharging
            | WorldpayvantivResponseCode::ThisMethodOfPaymentDoesNotSupportSurcharging
            | WorldpayvantivResponseCode::SurchargeIsNotValidForDebitOrPrepaidCards
            | WorldpayvantivResponseCode::SurchargeCannotExceedsTheMaximumAllowedLimit
            | WorldpayvantivResponseCode::TransactionDeclinedByTheProcessingNetwork
            | WorldpayvantivResponseCode::SecondaryAmountCannotExceedTheSaleAmount
            | WorldpayvantivResponseCode::ThisMethodOfPaymentDoesNotSupportSecondaryAmount
            | WorldpayvantivResponseCode::SecondaryAmountCannotBeLessThanZero
            | WorldpayvantivResponseCode::PartialTransactionIsNotSupportedWhenIncludingASecondaryAmount
            | WorldpayvantivResponseCode::SecondaryAmountRequiredOnPartialRefundWhenUsedOnDeposit
            | WorldpayvantivResponseCode::SecondaryAmountNotAllowedOnRefundIfNotIncludedOnDeposit
            | WorldpayvantivResponseCode::ProcessingNetworkError
            | WorldpayvantivResponseCode::InvalidEMail
            | WorldpayvantivResponseCode::InvalidCombinationOfAccountFundingTransactionTypeAndMcc
            | WorldpayvantivResponseCode::InvalidAccountFundingTransactionTypeForThisMethodOfPayment
            | WorldpayvantivResponseCode::MissingOneOrMoreReceiverFieldsForAccountFundingTransaction
            | WorldpayvantivResponseCode::InvalidRecurringRequestSeeRecurringResponseForDetails
            | WorldpayvantivResponseCode::ParentTransactionDeclinedRecurringSubscriptionNotCreated
            | WorldpayvantivResponseCode::InvalidPlanCode
            | WorldpayvantivResponseCode::InvalidSubscriptionId
            | WorldpayvantivResponseCode::AddOnCodeAlreadyExists
            | WorldpayvantivResponseCode::DuplicateAddOnCodesInRequests
            | WorldpayvantivResponseCode::NoMatchingAddOnCodeForTheSubscription
            | WorldpayvantivResponseCode::NoMatchingDiscountCodeForTheSubscription
            | WorldpayvantivResponseCode::DuplicateDiscountCodesInRequest
            | WorldpayvantivResponseCode::InvalidStartDate
            | WorldpayvantivResponseCode::MerchantNotRegisteredForRecurringEngine
            | WorldpayvantivResponseCode::InsufficientDataToUpdateSubscription
            | WorldpayvantivResponseCode::InvalidBillingDate
            | WorldpayvantivResponseCode::DiscountCodeAlreadyExists
            | WorldpayvantivResponseCode::PlanCodeAlreadyExists
            | WorldpayvantivResponseCode::TheAccountNumberWasChanged
            | WorldpayvantivResponseCode::TheAccountWasClosed
            | WorldpayvantivResponseCode::TheExpirationDateWasChanged
            | WorldpayvantivResponseCode::TheIssuingBankDoesNotParticipateInTheUpdateProgram
            | WorldpayvantivResponseCode::ContactTheCardholderForUpdatedInformation
            | WorldpayvantivResponseCode::TheCardholderHasOptedOutOfTheUpdateProgram
            | WorldpayvantivResponseCode::SoftDeclineCardReaderDecryptionServiceIsNotAvailable
            | WorldpayvantivResponseCode::SoftDeclineDecryptionFailed
            | WorldpayvantivResponseCode::HardDeclineInputDataIsInvalid
            | WorldpayvantivResponseCode::ApplePayKeyMismatch
            | WorldpayvantivResponseCode::ApplePayDecryptionFailed
            | WorldpayvantivResponseCode::HardDeclineDecryptionFailed
            | WorldpayvantivResponseCode::MerchantNotConfiguredForProcessingAtThisSite
            | WorldpayvantivResponseCode::AdvancedFraudFilterScoreBelowThreshold
            | WorldpayvantivResponseCode::SuspectedFraud
            | WorldpayvantivResponseCode::SystemErrorContactWorldpayRepresentative
            | WorldpayvantivResponseCode::AmazonPayAmazonUnavailable
            | WorldpayvantivResponseCode::AmazonPayAmazonDeclined
            | WorldpayvantivResponseCode::AmazonPayInvalidToken
            | WorldpayvantivResponseCode::MerchantNotEnabledForAmazonPay
            | WorldpayvantivResponseCode::TransactionNotSupportedBlockedByIssuer
            | WorldpayvantivResponseCode::BlockedByCardholderContactCardholder
            | WorldpayvantivResponseCode::SoftDeclinePrimaryFundingSourceFailed
            | WorldpayvantivResponseCode::SoftDeclineBuyerHasAlternateFundingSource
            | WorldpayvantivResponseCode::HardDeclineInvalidBillingAgreementId
            | WorldpayvantivResponseCode::HardDeclinePrimaryFundingSourceFailed
            | WorldpayvantivResponseCode::HardDeclineIssueWithPaypalAccount
            | WorldpayvantivResponseCode::HardDeclinePayPalAuthorizationIdMissing
            | WorldpayvantivResponseCode::HardDeclineConfirmedEmailAddressIsNotAvailable
            | WorldpayvantivResponseCode::HardDeclinePayPalBuyerAccountDenied
            | WorldpayvantivResponseCode::HardDeclinePayPalBuyerAccountRestricted
            | WorldpayvantivResponseCode::HardDeclinePayPalOrderHasBeenVoidedExpiredOrCompleted
            | WorldpayvantivResponseCode::HardDeclineIssueWithPayPalRefund
            | WorldpayvantivResponseCode::HardDeclinePayPalCredentialsIssue
            | WorldpayvantivResponseCode::HardDeclinePayPalAuthorizationVoidedOrExpired
            | WorldpayvantivResponseCode::HardDeclineRequiredPayPalParameterMissing
            | WorldpayvantivResponseCode::HardDeclinePayPalTransactionIdOrAuthIdIsInvalid
            | WorldpayvantivResponseCode::HardDeclineExceededMaximumNumberOfPayPalAuthorizationAttempts
            | WorldpayvantivResponseCode::HardDeclineTransactionAmountExceedsMerchantsPayPalAccountLimit
            | WorldpayvantivResponseCode::HardDeclinePayPalFundingSourcesUnavailable
            | WorldpayvantivResponseCode::HardDeclineIssueWithPayPalPrimaryFundingSource
            | WorldpayvantivResponseCode::HardDeclinePayPalProfileDoesNotAllowThisTransactionType
            | WorldpayvantivResponseCode::InternalSystemErrorWithPayPalContactVantiv
            | WorldpayvantivResponseCode::HardDeclineContactPayPalConsumerForAnotherPaymentMethod
            | WorldpayvantivResponseCode::InvalidTerminalId
            | WorldpayvantivResponseCode::PinlessDebitProcessingNotSupportedForNonRecurringTransactions
            | WorldpayvantivResponseCode::PinlessDebitProcessingNotSupportedForPartialAuths
            | WorldpayvantivResponseCode::MerchantNotConfiguredForPinlessDebitProcessing
            | WorldpayvantivResponseCode::DeclineCustomerCancellation
            | WorldpayvantivResponseCode::DeclineReTryTransaction
            | WorldpayvantivResponseCode::DeclineUnableToLocateRecordOnFile
            | WorldpayvantivResponseCode::DeclineFileUpdateFieldEditError
            | WorldpayvantivResponseCode::RemoteFunctionUnknown
            | WorldpayvantivResponseCode::DeclinedExceedsWithdrawalFrequencyLimit
            | WorldpayvantivResponseCode::DeclineCardRecordNotAvailable
            | WorldpayvantivResponseCode::InvalidAuthorizationCode
            | WorldpayvantivResponseCode::ReconciliationError
            | WorldpayvantivResponseCode::PreferredDebitRoutingDenialCreditTransactionCanBeDebit
            | WorldpayvantivResponseCode::DeclinedCurrencyConversionCompleteNoAuthPerformed
            | WorldpayvantivResponseCode::DeclinedMultiCurrencyDccFail
            | WorldpayvantivResponseCode::DeclinedMultiCurrencyInvertFail
            | WorldpayvantivResponseCode::Invalid3DSecurePassword
            | WorldpayvantivResponseCode::InvalidSocialSecurityNumber
            | WorldpayvantivResponseCode::InvalidMothersMaidenName
            | WorldpayvantivResponseCode::EnrollmentInquiryDeclined
            | WorldpayvantivResponseCode::SocialSecurityNumberNotAvailable
            | WorldpayvantivResponseCode::MothersMaidenNameNotAvailable
            | WorldpayvantivResponseCode::PinAlreadyExistsOnDatabase
            | WorldpayvantivResponseCode::Under18YearsOld
            | WorldpayvantivResponseCode::BillToOutsideUsa
            | WorldpayvantivResponseCode::BillToAddressIsNotEqualToShipToAddress
            | WorldpayvantivResponseCode::DeclinedForeignCurrencyMustBeUsd
            | WorldpayvantivResponseCode::OnNegativeFile
            | WorldpayvantivResponseCode::BlockedAgreement
            | WorldpayvantivResponseCode::InsufficientBuyingPower
            | WorldpayvantivResponseCode::InvalidData
            | WorldpayvantivResponseCode::InvalidDataDataElementsMissing
            | WorldpayvantivResponseCode::InvalidDataDataFormatError
            | WorldpayvantivResponseCode::InvalidDataInvalidTCVersion
            | WorldpayvantivResponseCode::DuplicateTransactionPaypalCredit
            | WorldpayvantivResponseCode::VerifyBillingAddress
            | WorldpayvantivResponseCode::InactiveAccount
            | WorldpayvantivResponseCode::InvalidAuth
            | WorldpayvantivResponseCode::AuthorizationAlreadyExistsForTheOrder
            | WorldpayvantivResponseCode::LodgingTransactionsAreNotAllowedForThisMcc
            | WorldpayvantivResponseCode::DurationCannotBeNegative
            | WorldpayvantivResponseCode::HotelFolioNumberCannotBeBlank
            | WorldpayvantivResponseCode::InvalidCheckInDate
            | WorldpayvantivResponseCode::InvalidCheckOutDate
            | WorldpayvantivResponseCode::InvalidCheckInOrCheckOutDate
            | WorldpayvantivResponseCode::CheckOutDateCannotBeBeforeCheckInDate
            | WorldpayvantivResponseCode::NumberOfAdultsCannotBeNegative
            | WorldpayvantivResponseCode::RoomRateCannotBeNegative
            | WorldpayvantivResponseCode::RoomTaxCannotBeNegative
            | WorldpayvantivResponseCode::DurationCanOnlyBeFrom0To99ForVisa
            | WorldpayvantivResponseCode::MerchantIsNotAuthorizedForTokens
            | WorldpayvantivResponseCode::CreditCardNumberWasInvalid
        | WorldpayvantivResponseCode::TokenWasNotFound
        | WorldpayvantivResponseCode::TokenInvalid
        | WorldpayvantivResponseCode::MerchantNotAuthorizedForECheckTokens
        | WorldpayvantivResponseCode::CheckoutIdWasInvalid
        | WorldpayvantivResponseCode::CheckoutIdWasNotFound
        | WorldpayvantivResponseCode::GenericCheckoutIdError
        | WorldpayvantivResponseCode::CaptureAmountCanNotBeMoreThanAuthorizedAmount
        | WorldpayvantivResponseCode::TaxBillingOnlyAllowedForMcc9311
        | WorldpayvantivResponseCode::Mcc9311RequiresTaxTypeElement
        | WorldpayvantivResponseCode::DebtRepaymentOnlyAllowedForViTransactionsOnMccs6012And6051
        | WorldpayvantivResponseCode::RoutingNumberDidNotMatchOneOnFileForToken
        | WorldpayvantivResponseCode::InvalidPayPageRegistrationId
        | WorldpayvantivResponseCode::ExpiredPayPageRegistrationId
        | WorldpayvantivResponseCode::MerchantIsNotAuthorizedForPayPage
        | WorldpayvantivResponseCode::MaximumNumberOfUpdatesForThisTokenExceeded
        | WorldpayvantivResponseCode::TooManyTokensCreatedForExistingNamespace
        | WorldpayvantivResponseCode::PinValidationNotPossible
        | WorldpayvantivResponseCode::GenericTokenRegistrationError
        | WorldpayvantivResponseCode::GenericTokenUseError
        | WorldpayvantivResponseCode::InvalidBankRoutingNumber
        | WorldpayvantivResponseCode::MissingName
        | WorldpayvantivResponseCode::InvalidName
        | WorldpayvantivResponseCode::MissingBillingCountryCode
        | WorldpayvantivResponseCode::InvalidIban
        | WorldpayvantivResponseCode::MissingEmailAddress
        | WorldpayvantivResponseCode::MissingMandateReference
        | WorldpayvantivResponseCode::InvalidMandateReference
        | WorldpayvantivResponseCode::MissingMandateUrl
        | WorldpayvantivResponseCode::InvalidMandateUrl
        | WorldpayvantivResponseCode::MissingMandateSignatureDate
        | WorldpayvantivResponseCode::InvalidMandateSignatureDate
        | WorldpayvantivResponseCode::RecurringMandateAlreadyExists
        | WorldpayvantivResponseCode::RecurringMandateWasNotFound
        | WorldpayvantivResponseCode::FinalRecurringWasAlreadyReceivedUsingThisMandate
        | WorldpayvantivResponseCode::IbanDidNotMatchOneOnFileForMandate
        | WorldpayvantivResponseCode::InvalidBillingCountry
        | WorldpayvantivResponseCode::ExpirationDateRequiredForInteracTransaction
        | WorldpayvantivResponseCode::TransactionTypeIsNotSupportedWithThisMethodOfPayment
        | WorldpayvantivResponseCode::UnreferencedOrphanRefundsAreNotAllowed
        | WorldpayvantivResponseCode::UnableToVoidATransactionWithAHeldState
        | WorldpayvantivResponseCode::ThisFundingInstructionResultsInANegativeAccountBalance
        | WorldpayvantivResponseCode::AccountBalanceInformationUnavailableAtThisTime
        | WorldpayvantivResponseCode::TheSubmittedCardIsNotEligibleForFastAccessFunding
        | WorldpayvantivResponseCode::TransactionCannotUseBothCcdPaymentInformationAndCtxPaymentInformation
        | WorldpayvantivResponseCode::ProcessingError
        | WorldpayvantivResponseCode::ThisFundingInstructionTypeIsInvalidForCanadianMerchants
        | WorldpayvantivResponseCode::CtxAndCcdRecordsAreNotAllowedForCanadianMerchants
        | WorldpayvantivResponseCode::CanadianAccountNumberCannotExceed12Digits
        | WorldpayvantivResponseCode::ThisFundingInstructionTypeIsInvalid
        | WorldpayvantivResponseCode::DeclineNegativeInformationOnFile
        | WorldpayvantivResponseCode::AbsoluteDecline
        | WorldpayvantivResponseCode::TheMerchantProfileDoesNotAllowTheRequestedOperation
        | WorldpayvantivResponseCode::TheAccountCannotAcceptAchTransactions
        | WorldpayvantivResponseCode::TheAccountCannotAcceptAchTransactionsOrSiteDrafts
        | WorldpayvantivResponseCode::AmountGreaterThanLimitSpecifiedInTheMerchantProfile
        | WorldpayvantivResponseCode::MerchantIsNotAuthorizedToPerformECheckVerificationTransactions
        | WorldpayvantivResponseCode::FirstNameAndLastNameRequiredForECheckVerifications
        | WorldpayvantivResponseCode::CompanyNameRequiredForCorporateAccountForECheckVerifications
        | WorldpayvantivResponseCode::PhoneNumberRequiredForECheckVerifications
        | WorldpayvantivResponseCode::CardBrandTokenNotSupported
        | WorldpayvantivResponseCode::PrivateLabelCardNotSupported
        | WorldpayvantivResponseCode::AllowedDailyDirectDebitCaptureECheckSaleLimitExceeded
        | WorldpayvantivResponseCode::AllowedDailyDirectDebitCreditECheckCreditLimitExceeded
        | WorldpayvantivResponseCode::AccountNotEligibleForRtp
        | WorldpayvantivResponseCode::SoftDeclineCustomerAuthenticationRequired
        | WorldpayvantivResponseCode::TransactionNotReversedVoidWorkflowNeedToBeInvoked
        | WorldpayvantivResponseCode::TransactionReversalNotSupportedForTheCoreMerchants
        | WorldpayvantivResponseCode::NoValidParentDepositOrParentRefundFound
        | WorldpayvantivResponseCode::TransactionReversalNotEnabledForVisa
        | WorldpayvantivResponseCode::TransactionReversalNotEnabledForMastercard
        | WorldpayvantivResponseCode::TransactionReversalNotEnabledForAmEx
        | WorldpayvantivResponseCode::TransactionReversalNotEnabledForDiscover
        | WorldpayvantivResponseCode::TransactionReversalNotSupported
        | WorldpayvantivResponseCode::FundingInstructionHeldPleaseContactYourRelationshipManager
        | WorldpayvantivResponseCode::MissingAddressInformation
        | WorldpayvantivResponseCode::CryptographicFailure
        | WorldpayvantivResponseCode::InvalidRegionCode
        | WorldpayvantivResponseCode::InvalidCountryCode
        | WorldpayvantivResponseCode::InvalidCreditAccount
        | WorldpayvantivResponseCode::InvalidCheckingAccount
        | WorldpayvantivResponseCode::InvalidSavingsAccount
        | WorldpayvantivResponseCode::InvalidUseOfMccCorrectAndReattempt
        | WorldpayvantivResponseCode::ExceedsRtpTransactionLimit
             => match flow {
                WorldpayvantivPaymentFlow::Sale => Ok(common_enums::AttemptStatus::Failure),
                WorldpayvantivPaymentFlow::Auth => Ok(common_enums::AttemptStatus::AuthorizationFailed),
                WorldpayvantivPaymentFlow::Capture => Ok(common_enums::AttemptStatus::CaptureFailed),
                WorldpayvantivPaymentFlow::Void => Ok(common_enums::AttemptStatus::VoidFailed),
                WorldpayvantivPaymentFlow::VoidPC => Ok(common_enums::AttemptStatus::VoidFailed)
            }
    }
}

fn get_refund_status(
    response: WorldpayvantivResponseCode,
) -> Result<common_enums::RefundStatus, errors::ConnectorError> {
    match response {
        WorldpayvantivResponseCode::Approved
            | WorldpayvantivResponseCode::PartiallyApproved
            | WorldpayvantivResponseCode::OfflineApproval
            | WorldpayvantivResponseCode::OfflineApprovalUnableToGoOnline => {
                Ok(common_enums::RefundStatus::Pending)
            },
        WorldpayvantivResponseCode::TransactionReceived => Ok(common_enums::RefundStatus::Pending),
        WorldpayvantivResponseCode::ProcessingNetworkUnavailable
        | WorldpayvantivResponseCode::IssuerUnavailable
        | WorldpayvantivResponseCode::ReSubmitTransaction
        | WorldpayvantivResponseCode::MerchantNotConfiguredForProcessingAtThisSite
        | WorldpayvantivResponseCode::TryAgainLater
        | WorldpayvantivResponseCode::InsufficientFunds
        | WorldpayvantivResponseCode::AuthorizationAmountHasAlreadyBeenDepleted
        | WorldpayvantivResponseCode::InsufficientFundsRetryAfter1Hour
        | WorldpayvantivResponseCode::InsufficientFundsRetryAfter24Hour
        | WorldpayvantivResponseCode::InsufficientFundsRetryAfter2Days
        | WorldpayvantivResponseCode::InsufficientFundsRetryAfter4Days
        | WorldpayvantivResponseCode::InsufficientFundsRetryAfter6Days
        | WorldpayvantivResponseCode::InsufficientFundsRetryAfter8Days
        | WorldpayvantivResponseCode::InsufficientFundsRetryAfter10Days
        | WorldpayvantivResponseCode::CallIssuer
        | WorldpayvantivResponseCode::CallAmex
        | WorldpayvantivResponseCode::CallDinersClub
        | WorldpayvantivResponseCode::CallDiscover
        | WorldpayvantivResponseCode::CallJbs
        | WorldpayvantivResponseCode::CallVisaMastercard
        | WorldpayvantivResponseCode::ExceedsApprovalAmountLimit
        | WorldpayvantivResponseCode::CallIndicatedNumber
        | WorldpayvantivResponseCode::ConsumerMultiUseVirtualCardNumberSoftDecline
        | WorldpayvantivResponseCode::ConsumerNonReloadablePrepaidCardSoftDecline
        | WorldpayvantivResponseCode::ConsumerSingleUseVirtualCardNumberSoftDecline
        | WorldpayvantivResponseCode::MerchantDoesntQualifyForProductCode
        | WorldpayvantivResponseCode::Lifecycle
        | WorldpayvantivResponseCode::Policy
        | WorldpayvantivResponseCode::InvalidOrExpiredCardContactCardholderToUpdate
        | WorldpayvantivResponseCode::InvalidTransactionOrCardRestrictionVerifyInformationAndResubmit
        | WorldpayvantivResponseCode::AtLeastOneOfOrigIdOrOrigCnpTxnIdIsRequired
        | WorldpayvantivResponseCode::OrigCnpTxnIdIsRequiredWhenShowStatusOnlyIsUsed
        | WorldpayvantivResponseCode::TransactionMustReferenceTheEstimatedAuth
        | WorldpayvantivResponseCode::IncrementedAuthExceedsMaxTransactionAmount
        | WorldpayvantivResponseCode::SubmittedMccNotAllowed
        | WorldpayvantivResponseCode::MerchantNotCertifiedEnabledForIias
        | WorldpayvantivResponseCode::IssuerGeneratedError
        | WorldpayvantivResponseCode::InvalidAmountHardDecline
        | WorldpayvantivResponseCode::ReversalUnsuccessful
        | WorldpayvantivResponseCode::MissingData
        | WorldpayvantivResponseCode::InvalidDeactivate
        | WorldpayvantivResponseCode::OverMaxBalance
        | WorldpayvantivResponseCode::InvalidActivate
        | WorldpayvantivResponseCode::NoTransactionFoundForReversal
        | WorldpayvantivResponseCode::IllegalTransaction
        | WorldpayvantivResponseCode::DuplicateTransaction
        | WorldpayvantivResponseCode::SystemError
        | WorldpayvantivResponseCode::MerchantDepleted
        | WorldpayvantivResponseCode::InvalidReversalTypeForCreditCardTransaction
        | WorldpayvantivResponseCode::SystemErrorMessageFormatError
        | WorldpayvantivResponseCode::SystemErrorCannotProcess
        | WorldpayvantivResponseCode::RefundRejectedDueToPendingDepositStatus
        | WorldpayvantivResponseCode::RefundRejectedDueToDeclinedDepositStatus
        | WorldpayvantivResponseCode::RefundRejectedByTheProcessingNetwork
        | WorldpayvantivResponseCode::CaptureCreditAndAuthReversalTagsCannotBeUsedForGiftCardTransactions
        | WorldpayvantivResponseCode::InvalidAccountNumber
        | WorldpayvantivResponseCode::AuthorizationHasExpiredNoNeedToReverse
        | WorldpayvantivResponseCode::InvalidTrackData
        | WorldpayvantivResponseCode::DepositIsAlreadyReferencedByAChargeback
        | WorldpayvantivResponseCode::AutomaticRefundAlreadyIssued
        | WorldpayvantivResponseCode::InvalidMerchant
        | WorldpayvantivResponseCode::InvalidTransaction
        | WorldpayvantivResponseCode::TransactionNotAllowedAtTerminal
        | WorldpayvantivResponseCode::ThisMethodOfPaymentDoesNotSupportAuthorizationReversals
        | WorldpayvantivResponseCode::ReversalAmountDoesNotMatchAuthorizationAmount
        | WorldpayvantivResponseCode::InvalidAmountSoftDecline
        | WorldpayvantivResponseCode::InvalidReportGroup
        | WorldpayvantivResponseCode::DoNotHonor
        | WorldpayvantivResponseCode::GenericDecline
        | WorldpayvantivResponseCode::DeclineRequestPositiveId
        | WorldpayvantivResponseCode::ThreeDSecureTransactionNotSupportedByMerchant
        | WorldpayvantivResponseCode::RestrictedByVantivDueToSecurityCodeMismatch
        | WorldpayvantivResponseCode::NoTransactionFoundWithSpecifiedTransactionId
        | WorldpayvantivResponseCode::AuthorizationNoLongerAvailable
        | WorldpayvantivResponseCode::TransactionNotVoidedAlreadySettled
        | WorldpayvantivResponseCode::AutoVoidOnRefund
        | WorldpayvantivResponseCode::InvalidAccountNumberOriginalOrNocUpdatedECheckAccountRequired
        | WorldpayvantivResponseCode::TotalCreditAmountExceedsCaptureAmount
        | WorldpayvantivResponseCode::ExceedTheThresholdForSendingRedeposits
        | WorldpayvantivResponseCode::DepositHasNotBeenReturnedForInsufficientNonSufficientFunds
        | WorldpayvantivResponseCode::RedepositAgainstInvalidTransactionType
        | WorldpayvantivResponseCode::InternalSystemErrorCallVantiv
        | WorldpayvantivResponseCode::OriginalTransactionHasBeenProcessedFutureRedepositsCanceled
        | WorldpayvantivResponseCode::SoftDeclineAutoRecyclingInProgress
        | WorldpayvantivResponseCode::HardDeclineAutoRecyclingComplete
        | WorldpayvantivResponseCode::TransactionDeclinedByTheProcessingNetwork
        | WorldpayvantivResponseCode::SecondaryAmountCannotExceedTheSaleAmount
        | WorldpayvantivResponseCode::ThisMethodOfPaymentDoesNotSupportSecondaryAmount
        | WorldpayvantivResponseCode::SecondaryAmountCannotBeLessThanZero
        | WorldpayvantivResponseCode::PartialTransactionIsNotSupportedWhenIncludingASecondaryAmount
        | WorldpayvantivResponseCode::SecondaryAmountRequiredOnPartialRefundWhenUsedOnDeposit
        | WorldpayvantivResponseCode::SecondaryAmountNotAllowedOnRefundIfNotIncludedOnDeposit
        | WorldpayvantivResponseCode::ProcessingNetworkError
        | WorldpayvantivResponseCode::InvalidEMail
        | WorldpayvantivResponseCode::InvalidCombinationOfAccountFundingTransactionTypeAndMcc
        | WorldpayvantivResponseCode::InvalidAccountFundingTransactionTypeForThisMethodOfPayment
        | WorldpayvantivResponseCode::MissingOneOrMoreReceiverFieldsForAccountFundingTransaction
        | WorldpayvantivResponseCode::SoftDeclineDecryptionFailed
        | WorldpayvantivResponseCode::HardDeclineInputDataIsInvalid
        | WorldpayvantivResponseCode::HardDeclineDecryptionFailed
        | WorldpayvantivResponseCode::SuspectedFraud
        | WorldpayvantivResponseCode::SystemErrorContactWorldpayRepresentative
        | WorldpayvantivResponseCode::InvalidTerminalId
        | WorldpayvantivResponseCode::DeclineReTryTransaction
        | WorldpayvantivResponseCode::RemoteFunctionUnknown
        | WorldpayvantivResponseCode::InvalidData
        | WorldpayvantivResponseCode::InvalidDataDataElementsMissing
        | WorldpayvantivResponseCode::InvalidDataDataFormatError
        | WorldpayvantivResponseCode::VerifyBillingAddress
        | WorldpayvantivResponseCode::InactiveAccount
        | WorldpayvantivResponseCode::InvalidAuth
        | WorldpayvantivResponseCode::CheckoutIdWasInvalid
        | WorldpayvantivResponseCode::CheckoutIdWasNotFound
        | WorldpayvantivResponseCode::TransactionTypeIsNotSupportedWithThisMethodOfPayment
        | WorldpayvantivResponseCode::UnreferencedOrphanRefundsAreNotAllowed
        | WorldpayvantivResponseCode::ThisFundingInstructionResultsInANegativeAccountBalance
        | WorldpayvantivResponseCode::ProcessingError
        | WorldpayvantivResponseCode::ThisFundingInstructionTypeIsInvalidForCanadianMerchants
        | WorldpayvantivResponseCode::ThisFundingInstructionTypeIsInvalid
        | WorldpayvantivResponseCode::AbsoluteDecline
        | WorldpayvantivResponseCode::TheMerchantProfileDoesNotAllowTheRequestedOperation
        | WorldpayvantivResponseCode::AmountGreaterThanLimitSpecifiedInTheMerchantProfile
        | WorldpayvantivResponseCode::AccountNotEligibleForRtp
        | WorldpayvantivResponseCode::NoValidParentDepositOrParentRefundFound
        | WorldpayvantivResponseCode::FundingInstructionHeldPleaseContactYourRelationshipManager
        | WorldpayvantivResponseCode::InvalidCreditAccount => Ok(common_enums::RefundStatus::Failure),
         _ => {
            Err(errors::ConnectorError::UnexpectedResponseError(
                bytes::Bytes::from("Invalid response code for refund".to_string()),
            ))
        }
            }
}

fn get_vantiv_card_data(
    payment_method_data: &PaymentMethodData,
    payment_method_token: Option<hyperswitch_domain_models::router_data::PaymentMethodToken>,
) -> Result<
    (
        Option<WorldpayvantivCardData>,
        Option<CardholderAuthentication>,
    ),
    error_stack::Report<errors::ConnectorError>,
> {
    match payment_method_data {
        PaymentMethodData::Card(card) => {
            let card_type = match card.card_network.clone() {
                Some(card_type) => WorldpayvativCardType::try_from(card_type)?,
                None => WorldpayvativCardType::try_from(&card.get_card_issuer()?)?,
            };

            let exp_date = card.get_expiry_date_as_mmyy()?;

            Ok((
                Some(WorldpayvantivCardData {
                    card_type,
                    number: card.card_number.clone(),
                    exp_date,
                    card_validation_num: Some(card.card_cvc.clone()),
                }),
                None,
            ))
        }
        PaymentMethodData::CardDetailsForNetworkTransactionId(card_data) => {
            let card_type = match card_data.card_network.clone() {
                Some(card_type) => WorldpayvativCardType::try_from(card_type)?,
                None => WorldpayvativCardType::try_from(&card_data.get_card_issuer()?)?,
            };

            let exp_date = card_data.get_expiry_date_as_mmyy()?;

            Ok((
                Some(WorldpayvantivCardData {
                    card_type,
                    number: card_data.card_number.clone(),
                    exp_date,
                    card_validation_num: None,
                }),
                None,
            ))
        }
        PaymentMethodData::MandatePayment => Ok((None, None)),
        PaymentMethodData::Wallet(wallet_data) => match wallet_data {
            hyperswitch_domain_models::payment_method_data::WalletData::ApplePay(
                apple_pay_data,
            ) => match payment_method_token.clone() {
                Some(
                    hyperswitch_domain_models::router_data::PaymentMethodToken::ApplePayDecrypt(
                        apple_pay_decrypted_data,
                    ),
                ) => {
                    let number = apple_pay_decrypted_data
                        .application_primary_account_number
                        .clone();
                    let exp_date = apple_pay_decrypted_data
                        .get_expiry_date_as_mmyy()
                        .change_context(errors::ConnectorError::InvalidDataFormat {
                            field_name: "payment_method_data.card.card_exp_month",
                        })?;

                    let cardholder_authentication = CardholderAuthentication {
                        authentication_value: apple_pay_decrypted_data
                            .payment_data
                            .online_payment_cryptogram
                            .clone(),
                    };

                    let apple_pay_network = apple_pay_data
                    .payment_method
                    .network
                    .parse::<WorldPayVativApplePayNetwork>()
                    .map_err(|_| {
                        error_stack::Report::new(errors::ConnectorError::NotSupported {
                            message: format!(
                                "Invalid Apple Pay network '{}'. Supported networks: Visa,MasterCard,AmEx,Discover,DinersClub,JCB,UnionPay",
                                apple_pay_data.payment_method.network
                            ),
                            connector: "worldpay_vativ"
                        })
                    })?;

                    Ok((
                        (Some(WorldpayvantivCardData {
                            card_type: apple_pay_network.into(),
                            number,
                            exp_date,
                            card_validation_num: None,
                        })),
                        Some(cardholder_authentication),
                    ))
                }
                _ => Err(
                    errors::ConnectorError::NotImplemented("Payment method type".to_string())
                        .into(),
                ),
            },
            hyperswitch_domain_models::payment_method_data::WalletData::GooglePay(
                google_pay_data,
            ) => match payment_method_token.clone() {
                Some(
                    hyperswitch_domain_models::router_data::PaymentMethodToken::GooglePayDecrypt(
                        google_pay_decrypted_data,
                    ),
                ) => {
                    let number = google_pay_decrypted_data
                        .application_primary_account_number
                        .clone();
                    let exp_date = google_pay_decrypted_data
                        .get_expiry_date_as_mmyy()
                        .change_context(errors::ConnectorError::InvalidDataFormat {
                            field_name: "payment_method_data.card.card_exp_month",
                        })?;

                    let cardholder_authentication = CardholderAuthentication {
                        authentication_value: google_pay_decrypted_data
                            .cryptogram
                            .clone()
                            .ok_or_else(|| errors::ConnectorError::MissingRequiredField {
                                field_name: "cryptogram",
                            })?,
                    };
                    let google_pay_network = google_pay_data
                        .info
                        .card_network
                        .parse::<WorldPayVativGooglePayNetwork>()
                        .map_err(|_| {
                            error_stack::Report::new(errors::ConnectorError::NotSupported {
                                message: format!(
                                    "Invalid Google Pay card network '{}'. Supported networks: VISA, MASTERCARD, AMEX, DISCOVER, JCB, UNIONPAY",
                                    google_pay_data.info.card_network
                                ),
                                connector: "worldpay_vativ"
                            })
                        })?;

                    Ok((
                        (Some(WorldpayvantivCardData {
                            card_type: google_pay_network.into(),
                            number,
                            exp_date,
                            card_validation_num: None,
                        })),
                        Some(cardholder_authentication),
                    ))
                }
                _ => {
                    Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into())
                }
            },
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        },
        _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
    }
}

fn get_connector_response(payment_response: &FraudResult) -> ConnectorResponseData {
    let payment_checks = Some(serde_json::json!({
        "avs_result": payment_response.avs_result,
        "card_validation_result": payment_response.card_validation_result,
        "authentication_result": payment_response.authentication_result,
        "advanced_a_v_s_result": payment_response.advanced_a_v_s_result,
    }));

    ConnectorResponseData::with_additional_payment_method_data(
        AdditionalPaymentMethodConnectorResponse::Card {
            authentication_data: None,
            payment_checks,
            card_network: None,
            domestic_network: None,
        },
    )
}

fn get_additional_payment_method_connector_response(
    payment_response: &FraudResult,
) -> AdditionalPaymentMethodConnectorResponse {
    let payment_checks = Some(serde_json::json!({
        "avs_result": payment_response.avs_result,
        "card_validation_result": payment_response.card_validation_result,
        "authentication_result": payment_response.authentication_result,
        "advanced_a_v_s_result": payment_response.advanced_a_v_s_result,
    }));

    AdditionalPaymentMethodConnectorResponse::Card {
        authentication_data: None,
        payment_checks,
        card_network: None,
        domestic_network: None,
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename = "chargebackRetrievalResponse", rename_all = "camelCase")]
pub struct ChargebackRetrievalResponse {
    #[serde(rename = "@xmlns")]
    pub xmlns: String,
    pub transaction_id: String,
    pub chargeback_case: Option<Vec<ChargebackCase>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChargebackCase {
    pub case_id: String,
    pub merchant_id: String,
    pub day_issued_by_bank: Option<String>,
    pub date_received_by_vantiv_cnp: Option<String>,
    pub vantiv_cnp_txn_id: String,
    pub cycle: String,
    pub order_id: String,
    pub card_number_last4: Option<String>,
    pub card_type: Option<String>,
    pub chargeback_amount: MinorUnit,
    pub chargeback_currency_type: common_enums::enums::Currency,
    pub original_txn_day: Option<String>,
    pub chargeback_type: Option<String>,
    pub reason_code: Option<String>,
    pub reason_code_description: Option<String>,
    pub current_queue: Option<String>,
    pub acquirer_reference_number: Option<String>,
    pub chargeback_reference_number: Option<String>,
    pub bin: Option<String>,
    pub payment_amount: Option<MinorUnit>,
    pub reply_by_day: Option<String>,
    pub pre_arbitration_amount: Option<MinorUnit>,
    pub pre_arbitration_currency_type: Option<common_enums::enums::Currency>,
    pub activity: Vec<Activity>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub activity_date: Option<String>,
    pub activity_type: Option<String>,
    pub from_queue: Option<String>,
    pub to_queue: Option<String>,
    pub notes: Option<String>,
}

impl
    TryFrom<
        ResponseRouterData<
            Fetch,
            ChargebackRetrievalResponse,
            FetchDisputesRequestData,
            FetchDisputesResponse,
        >,
    > for RouterData<Fetch, FetchDisputesRequestData, FetchDisputesResponse>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            Fetch,
            ChargebackRetrievalResponse,
            FetchDisputesRequestData,
            FetchDisputesResponse,
        >,
    ) -> Result<Self, Self::Error> {
        let dispute_list = item
            .response
            .chargeback_case
            .unwrap_or_default()
            .into_iter()
            .map(DisputeSyncResponse::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(FetchDisputeRouterData {
            response: Ok(dispute_list),
            ..item.data
        })
    }
}

impl
    TryFrom<
        ResponseRouterData<
            Dsync,
            ChargebackRetrievalResponse,
            DisputeSyncData,
            DisputeSyncResponse,
        >,
    > for RouterData<Dsync, DisputeSyncData, DisputeSyncResponse>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            Dsync,
            ChargebackRetrievalResponse,
            DisputeSyncData,
            DisputeSyncResponse,
        >,
    ) -> Result<Self, Self::Error> {
        let dispute_response = item
            .response
            .chargeback_case
            .and_then(|chargeback_case| chargeback_case.first().cloned())
            .ok_or(errors::ConnectorError::RequestEncodingFailedWithReason(
                "Could not find chargeback case".to_string(),
            ))?;

        let dispute_sync_response = DisputeSyncResponse::try_from(dispute_response.clone())?;
        Ok(DisputeSyncRouterData {
            response: Ok(dispute_sync_response),
            ..item.data
        })
    }
}

fn get_dispute_stage(
    dispute_cycle: String,
) -> Result<common_enums::enums::DisputeStage, error_stack::Report<errors::ConnectorError>> {
    match connector_utils::normalize_string(dispute_cycle.clone())
        .change_context(errors::ConnectorError::RequestEncodingFailed)?
        .as_str()
    {
        "arbitration"
        | "arbitrationmastercard"
        | "arbitrationchargeback"
        | "arbitrationlost"
        | "arbitrationsplit"
        | "arbitrationwon" => Ok(common_enums::enums::DisputeStage::Arbitration),
        "chargebackreversal" | "firstchargeback" | "rapiddisputeresolution" | "representment" => {
            Ok(common_enums::enums::DisputeStage::Dispute)
        }

        "issueracceptedprearbitration"
        | "issuerarbitration"
        | "issuerdeclinedprearbitration"
        | "prearbitration"
        | "responsetoissuerarbitration" => Ok(common_enums::enums::DisputeStage::PreArbitration),

        "retrievalrequest" => Ok(common_enums::enums::DisputeStage::PreDispute),
        _ => Err(errors::ConnectorError::NotSupported {
            message: format!("Dispute stage {dispute_cycle}",),
            connector: "worldpayvantiv",
        }
        .into()),
    }
}

pub fn get_dispute_status(
    dispute_cycle: String,
    dispute_activities: Vec<Activity>,
) -> Result<common_enums::DisputeStatus, error_stack::Report<errors::ConnectorError>> {
    if let Some(activity) = get_last_non_auxiliary_activity_type(dispute_activities) {
        match activity.as_ref() {
            "Merchant Accept"
            | "Issuer Accepted Pre-Arbitration"
            | "Vantiv Accept"
            | "Sent Credit" => Ok(common_enums::DisputeStatus::DisputeAccepted),

            "Merchant Represent"
            | "Respond to Dispute"
            | "Respond to PreArb"
            | "Request Arbitration"
            | "Request Pre-Arbitration"
            | "Create Arbitration"
            | "Record Arbitration"
            | "Create Pre-Arbitration"
            | "File Arbitration"
            | "File Pre-Arbitration"
            | "File Visa Pre-Arbitration"
            | "Send Representment"
            | "Send Response"
            | "Arbitration"
            | "Arbitration (Mastercard)"
            | "Arbitration Chargeback"
            | "Issuer Declined Pre-Arbitration"
            | "Issuer Arbitration"
            | "Request Response to Pre-Arbitration"
            | "Vantiv Represent"
            | "Vantiv Respond"
            | "Auto Represent"
            | "Arbitration Ruling" => Ok(common_enums::DisputeStatus::DisputeChallenged),

            "Arbitration Lost" | "Unsuccessful Arbitration" | "Unsuccessful Pre-Arbitration" => {
                Ok(common_enums::DisputeStatus::DisputeLost)
            }

            "Arbitration Won"
            | "Arbitration Split"
            | "Successful Arbitration"
            | "Successful Pre-Arbitration" => Ok(common_enums::DisputeStatus::DisputeWon),

            "Chargeback Reversal" => Ok(common_enums::DisputeStatus::DisputeCancelled),

            "Receive Network Transaction" => Ok(common_enums::DisputeStatus::DisputeOpened),

            "Unaccept" | "Unrepresent" => Ok(common_enums::DisputeStatus::DisputeOpened),

            unexpected_activity => Err(errors::ConnectorError::UnexpectedResponseError(
                bytes::Bytes::from(format!("Dispute Activity: {unexpected_activity})")),
            )
            .into()),
        }
    } else {
        match connector_utils::normalize_string(dispute_cycle.clone())
            .change_context(errors::ConnectorError::RequestEncodingFailed)?
            .as_str()
        {
            "arbitration"
            | "arbitrationmastercard"
            | "arbitrationsplit"
            | "representment"
            | "issuerarbitration"
            | "prearbitration"
            | "responsetoissuerarbitration"
            | "arbitrationchargeback" => Ok(api_models::enums::DisputeStatus::DisputeChallenged),
            "chargebackreversal" | "issueracceptedprearbitration" | "arbitrationwon" => {
                Ok(api_models::enums::DisputeStatus::DisputeWon)
            }
            "arbitrationlost" | "issuerdeclinedprearbitration" => {
                Ok(api_models::enums::DisputeStatus::DisputeLost)
            }
            "firstchargeback" | "retrievalrequest" | "rapiddisputeresolution" => {
                Ok(api_models::enums::DisputeStatus::DisputeOpened)
            }
            dispute_cycle => Err(errors::ConnectorError::UnexpectedResponseError(
                bytes::Bytes::from(format!("Dispute Stage: {dispute_cycle}")),
            )
            .into()),
        }
    }
}

fn convert_string_to_primitive_date(
    item: Option<String>,
) -> Result<Option<time::PrimitiveDateTime>, error_stack::Report<errors::ConnectorError>> {
    item.map(|day| {
        let full_datetime_str = format!("{day}T00:00:00");
        let format =
            time::macros::format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]");
        time::PrimitiveDateTime::parse(&full_datetime_str, &format)
    })
    .transpose()
    .change_context(errors::ConnectorError::ParsingFailed)
}

impl TryFrom<ChargebackCase> for DisputeSyncResponse {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: ChargebackCase) -> Result<Self, Self::Error> {
        let amount = connector_utils::convert_amount(
            &StringMinorUnitForConnector,
            item.chargeback_amount,
            item.chargeback_currency_type,
        )?;
        Ok(Self {
            object_reference_id: api_models::webhooks::ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::ConnectorTransactionId(item.vantiv_cnp_txn_id),
            ),
            amount,
            currency: item.chargeback_currency_type,
            dispute_stage: get_dispute_stage(item.cycle.clone())?,
            dispute_status: get_dispute_status(item.cycle.clone(), item.activity)?,
            connector_status: item.cycle.clone(),
            connector_dispute_id: item.case_id.clone(),
            connector_reason: item.reason_code_description.clone(),
            connector_reason_code: item.reason_code.clone(),
            challenge_required_by: convert_string_to_primitive_date(item.reply_by_day.clone())?,
            created_at: convert_string_to_primitive_date(item.date_received_by_vantiv_cnp.clone())?,
            updated_at: None,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActivityType {
    MerchantAcceptsLiability,
    MerchantRepresent,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "chargebackUpdateRequest", rename_all = "camelCase")]
pub struct ChargebackUpdateRequest {
    #[serde(rename = "@xmlns")]
    xmlns: String,
    activity_type: ActivityType,
}

impl From<&SubmitEvidenceRouterData> for ChargebackUpdateRequest {
    fn from(_item: &SubmitEvidenceRouterData) -> Self {
        Self {
            xmlns: worldpayvantiv_constants::XML_CHARGEBACK.to_string(),
            activity_type: ActivityType::MerchantRepresent,
        }
    }
}

impl From<&AcceptDisputeRouterData> for ChargebackUpdateRequest {
    fn from(_item: &AcceptDisputeRouterData) -> Self {
        Self {
            xmlns: worldpayvantiv_constants::XML_CHARGEBACK.to_string(),
            activity_type: ActivityType::MerchantAcceptsLiability,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "chargebackDocumentUploadResponse", rename_all = "camelCase")]
pub struct ChargebackDocumentUploadResponse {
    #[serde(rename = "@xmlns")]
    pub xmlns: String,
    pub merchant_id: String,
    pub case_id: String,
    pub document_id: Option<String>,
    pub response_code: WorldpayvantivFileUploadResponseCode,
    pub response_message: String,
}

#[derive(Debug, strum::Display, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum WorldpayvantivFileUploadResponseCode {
    #[serde(rename = "000")]
    #[strum(serialize = "000")]
    Success,

    #[serde(rename = "001")]
    #[strum(serialize = "001")]
    InvalidMerchant,

    #[serde(rename = "002")]
    #[strum(serialize = "002")]
    FutureUse002,

    #[serde(rename = "003")]
    #[strum(serialize = "003")]
    CaseNotFound,

    #[serde(rename = "004")]
    #[strum(serialize = "004")]
    CaseNotInMerchantQueue,

    #[serde(rename = "005")]
    #[strum(serialize = "005")]
    DocumentAlreadyExists,

    #[serde(rename = "006")]
    #[strum(serialize = "006")]
    InternalError,

    #[serde(rename = "007")]
    #[strum(serialize = "007")]
    FutureUse007,

    #[serde(rename = "008")]
    #[strum(serialize = "008")]
    MaxDocumentLimitReached,

    #[serde(rename = "009")]
    #[strum(serialize = "009")]
    DocumentNotFound,

    #[serde(rename = "010")]
    #[strum(serialize = "010")]
    CaseNotInValidCycle,

    #[serde(rename = "011")]
    #[strum(serialize = "011")]
    ServerBusy,

    #[serde(rename = "012")]
    #[strum(serialize = "012")]
    FileSizeExceedsLimit,

    #[serde(rename = "013")]
    #[strum(serialize = "013")]
    InvalidFileContent,

    #[serde(rename = "014")]
    #[strum(serialize = "014")]
    UnableToConvert,

    #[serde(rename = "015")]
    #[strum(serialize = "015")]
    InvalidImageSize,

    #[serde(rename = "016")]
    #[strum(serialize = "016")]
    MaxDocumentPageCountReached,
}

fn is_file_uploaded(vantiv_file_upload_status: WorldpayvantivFileUploadResponseCode) -> bool {
    match vantiv_file_upload_status {
        WorldpayvantivFileUploadResponseCode::Success
        | WorldpayvantivFileUploadResponseCode::FutureUse002
        | WorldpayvantivFileUploadResponseCode::FutureUse007 => true,
        WorldpayvantivFileUploadResponseCode::InvalidMerchant
        | WorldpayvantivFileUploadResponseCode::CaseNotFound
        | WorldpayvantivFileUploadResponseCode::CaseNotInMerchantQueue
        | WorldpayvantivFileUploadResponseCode::DocumentAlreadyExists
        | WorldpayvantivFileUploadResponseCode::InternalError
        | WorldpayvantivFileUploadResponseCode::MaxDocumentLimitReached
        | WorldpayvantivFileUploadResponseCode::DocumentNotFound
        | WorldpayvantivFileUploadResponseCode::CaseNotInValidCycle
        | WorldpayvantivFileUploadResponseCode::ServerBusy
        | WorldpayvantivFileUploadResponseCode::FileSizeExceedsLimit
        | WorldpayvantivFileUploadResponseCode::InvalidFileContent
        | WorldpayvantivFileUploadResponseCode::UnableToConvert
        | WorldpayvantivFileUploadResponseCode::InvalidImageSize
        | WorldpayvantivFileUploadResponseCode::MaxDocumentPageCountReached => false,
    }
}

impl
    TryFrom<
        ResponseRouterData<
            Upload,
            ChargebackDocumentUploadResponse,
            UploadFileRequestData,
            UploadFileResponse,
        >,
    > for RouterData<Upload, UploadFileRequestData, UploadFileResponse>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<
            Upload,
            ChargebackDocumentUploadResponse,
            UploadFileRequestData,
            UploadFileResponse,
        >,
    ) -> Result<Self, Self::Error> {
        let response = if is_file_uploaded(item.response.response_code) {
            let provider_file_id = item
                .response
                .document_id
                .ok_or(errors::ConnectorError::MissingConnectorTransactionID)?;

            Ok(UploadFileResponse { provider_file_id })
        } else {
            Err(ErrorResponse {
                code: item.response.response_code.to_string(),
                message: item.response.response_message.clone(),
                reason: Some(item.response.response_message.clone()),
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: None,
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        };

        Ok(Self {
            response,
            ..item.data
        })
    }
}

impl
    TryFrom<
        ResponseRouterData<
            Retrieve,
            ChargebackDocumentUploadResponse,
            RetrieveFileRequestData,
            RetrieveFileResponse,
        >,
    > for RouterData<Retrieve, RetrieveFileRequestData, RetrieveFileResponse>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<
            Retrieve,
            ChargebackDocumentUploadResponse,
            RetrieveFileRequestData,
            RetrieveFileResponse,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(RetrieveFileRouterData {
            response: Err(ErrorResponse {
                code: item.response.response_code.to_string(),
                message: item.response.response_message.clone(),
                reason: Some(item.response.response_message.clone()),
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: None,
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            }),
            ..item.data.clone()
        })
    }
}

fn get_last_non_auxiliary_activity_type(activities: Vec<Activity>) -> Option<String> {
    let auxiliary_activities: std::collections::HashSet<&'static str> = [
        "Add Note",
        "Attach Document",
        "Attempted Attach Document",
        "Delete Document",
        "Update Document",
        "Move To Error Queue",
        "Assign to Vantiv",
        "Assign To Merchant",
        "Merchant Auto Assign",
        "Issuer Recalled",
        "Network Decision",
        "Request Declined",
        "Sent Gift",
        "Successful PayPal",
    ]
    .iter()
    .copied()
    .collect();

    let mut last_non_auxiliary_activity = None;

    for activity in activities {
        let auxiliary_activity = activity
            .activity_type
            .as_deref()
            .map(|activity_type| auxiliary_activities.contains(activity_type))
            .unwrap_or(false);

        if !auxiliary_activity {
            last_non_auxiliary_activity = activity.activity_type.clone()
        }
    }
    last_non_auxiliary_activity
}
