use common_utils::{ext_traits::Encode, types::MinorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, PaymentsSyncData,
        ResponseId,
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        RefundsRouterData,
    },
};
use hyperswitch_interfaces::{consts, errors};
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self as connector_utils, CardData, PaymentsAuthorizeRequestData, RouterData as _},
};

pub mod worldpayvantiv_constants {
    pub const WORLDPAYVANTIV_VERSION: &str = "12.23";
    pub const XML_VERSION: &str = "1.0";
    pub const XML_ENCODING: &str = "UTF-8";
    pub const XMLNS: &str = "http://www.vantivcnp.com/schema";
    pub const MAX_ID_LENGTH: usize = 26;
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

// Represents the payment metadata for Worldpay Vantiv.
// The `report_group` field is an Option<String> to account for cases where the report group might not be provided in the metadata.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WorldpayvantivPaymentMetadata {
    pub report_group: Option<String>,
}

// Represents the merchant connector account metadata for Worldpay Vantiv
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WorldpayvantivMetadataObject {
    pub report_group: String,
    pub merchant_config_currency: common_enums::Currency,
}

impl TryFrom<&Option<common_utils::pii::SecretSerdeValue>> for WorldpayvantivMetadataObject {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        meta_data: &Option<common_utils::pii::SecretSerdeValue>,
    ) -> Result<Self, Self::Error> {
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
pub struct BillToAddressData {
    pub name: Option<Secret<String>>,
    pub address_line1: Option<Secret<String>>,
    pub city: Option<String>,
    pub state: Option<Secret<String>>,
    pub zip: Option<Secret<String>>,
    pub email: Option<common_utils::pii::Email>,
    pub country: Option<common_enums::CountryAlpha2>,
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
    #[serde(rename = "@customerId")]
    pub customer_id: Option<String>,
    pub order_id: String,
    pub amount: MinorUnit,
    pub order_source: OrderSource,
    pub bill_to_address: Option<BillToAddressData>,
    pub card: Option<WorldpayvantivCardData>,
    pub token: Option<TokenizationData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing_type: Option<VantivProcessingType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_network_transaction_id: Option<Secret<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sale {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@reportGroup")]
    pub report_group: String,
    #[serde(rename = "@customerId")]
    pub customer_id: Option<String>,
    pub order_id: String,
    pub amount: MinorUnit,
    pub order_source: OrderSource,
    pub bill_to_address: Option<BillToAddressData>,
    pub card: Option<WorldpayvantivCardData>,
    pub token: Option<TokenizationData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing_type: Option<VantivProcessingType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_network_transaction_id: Option<Secret<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundRequest {
    #[serde(rename = "@reportGroup")]
    pub report_group: String,
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@customerId")]
    pub customer_id: Option<String>,
    pub cnp_txn_id: String,
    pub amount: MinorUnit,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderSource {
    Ecommerce,
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

#[derive(Debug, Clone, Serialize)]

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
    #[serde(rename = "JCB")]
    JCB,
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

    fn get_vantiv_card_data(payment_method_data: &PaymentMethodData) -> Result<Option<WorldpayvantivCardData>, error_stack::Report<errors::ConnectorError>> {
        match payment_method_data {
            PaymentMethodData::Card(card) => {
                let card_type = match card.card_network.clone() {
                    Some(card_type) => WorldpayvativCardType::try_from(card_type)?,
                    None => WorldpayvativCardType::try_from(&card.get_card_issuer()?)?,
                };

                let exp_date = card.get_expiry_date_as_mmyy()?;

                Ok(Some(WorldpayvantivCardData {
                    card_type,
                    number: card.card_number.clone(),
                    exp_date,
                    card_validation_num: Some(card.card_cvc.clone()),
                }))
            }
            PaymentMethodData::CardDetailsForNetworkTransactionId(card_data) => {
                let card_type = match card_data.card_network.clone() {
                    Some(card_type) => WorldpayvativCardType::try_from(card_type)?,
                    None => WorldpayvativCardType::try_from(&card_data.get_card_issuer()?)?,
                };

                let exp_date = card_data.get_expiry_date_as_mmyy()?;

                Ok(Some(WorldpayvantivCardData {
                    card_type,
                    number: card_data.card_number.clone(),
                    exp_date,
                    card_validation_num: None,
                }))
            },
            PaymentMethodData::MandatePayment => {
               Ok(None)
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }

impl<F> TryFrom<ResponseRouterData<F, VantivSyncResponse, PaymentsSyncData, PaymentsResponseData>>
    for RouterData<F, PaymentsSyncData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, VantivSyncResponse, PaymentsSyncData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = get_attempt_status_for_psync(item.response.payment_status, item.data.status)?;

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
                .unwrap_or(consts::NO_ERROR_MESSAGE.to_string());

            Ok(Self {
                status,
                response: Err(ErrorResponse {
                    code: error_code.clone(),
                    message: error_message.clone(),
                    reason: Some(error_message.clone()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                }),
                ..item.data
            })
        } else {
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
                ..item.data
            })
        }
    }
}

fn get_bill_to_address(item: &PaymentsAuthorizeRouterData) -> Option<BillToAddressData> {
    let billing_address = item.get_optional_billing();
    billing_address.and_then(|billing_address| {
        billing_address
            .address
            .clone()
            .map(|address| BillToAddressData {
                name: address.get_optional_full_name(),
                address_line1: item.get_optional_billing_line1(),
                city: item.get_optional_billing_city(),
                state: item.get_optional_billing_state(),
                zip: item.get_optional_billing_zip(),
                email: item.get_optional_billing_email(),
                country: item.get_optional_billing_country(),
                phone: item.get_optional_billing_phone_number(),
            })
    })
}

impl TryFrom<&WorldpayvantivRouterData<&PaymentsAuthorizeRouterData>> for CnpOnlineRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &WorldpayvantivRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        if item.router_data.is_three_ds() {
            Err(errors::ConnectorError::NotSupported {
                message: "Card 3DS".to_string(),
                connector: "Worldpayvantiv",
            })?
        };
        let worldpayvantiv_metadata =
            WorldpayvantivMetadataObject::try_from(&item.router_data.connector_meta_data)?;

        if worldpayvantiv_metadata.merchant_config_currency != item.router_data.request.currency {
            Err(errors::ConnectorError::CurrencyNotSupported {
                message: item.router_data.request.currency.to_string(),
                connector: "Worldpayvantiv",
            })?
        };

        let card = get_vantiv_card_data(
            &item.router_data.request.payment_method_data.clone(),
        )?;
        let report_group = item
            .router_data
            .request
            .metadata
            .clone()
            .map(|payment_metadata| {
                connector_utils::to_connector_meta::<WorldpayvantivPaymentMetadata>(Some(
                    payment_metadata,
                ))
            })
            .transpose()?
            .and_then(|worldpayvantiv_metadata| worldpayvantiv_metadata.report_group)
            .unwrap_or(worldpayvantiv_metadata.report_group);

        let worldpayvantiv_auth_type =
            WorldpayvantivAuthType::try_from(&item.router_data.connector_auth_type)?;
        let authentication = Authentication {
            user: worldpayvantiv_auth_type.user,
            password: worldpayvantiv_auth_type.password,
        };

        let api_call_id =
            if item.router_data.attempt_id.len() < worldpayvantiv_constants::MAX_ID_LENGTH {
                item.router_data.attempt_id.clone()
            } else {
                format!("auth_{:?}", connector_utils::generate_12_digit_number())
            };

        let customer_id = item
            .router_data
            .customer_id
            .clone()
            .map(|customer_id| customer_id.get_string_repr().to_string());
        let bill_to_address = get_bill_to_address(item.router_data);

        let processing_info =
            get_processing_info(&item.router_data.request);

        let (authorization, sale) = if item.router_data.request.is_auto_capture()? {
            (
                None,
                Some(Sale {
                    id: api_call_id.clone(),
                    report_group: report_group.clone(),
                    customer_id,
                    order_id: item.router_data.payment_id.clone(),
                    amount: item.amount,
                    order_source: OrderSource::Ecommerce,
                    bill_to_address,
                    card,
                    token: processing_info.token,
                    processing_type: processing_info.processing_type,
                    original_network_transaction_id: processing_info.network_transaction_id,
                }),
            )
        } else {
            (
                Some(Authorization {
                    id: api_call_id.clone(),
                    report_group: report_group.clone(),
                    customer_id,
                    order_id: item.router_data.payment_id.clone(),
                    amount: item.amount,
                    order_source: OrderSource::Ecommerce,
                    bill_to_address,
                    card,
                    token: processing_info.token,
                    processing_type: processing_info.processing_type,
                    original_network_transaction_id: processing_info.network_transaction_id,
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
            void: None,
            credit: None,
        })
    }
}

#[derive(Debug)]
struct VantivMandateDetail {
    processing_type: Option<VantivProcessingType>,
    network_transaction_id: Option<Secret<String>>,
    token: Option<TokenizationData>,
}

#[derive(Debug, Serialize)]
pub struct TokenizationData {
    cnp_token: Secret<String>,
    exp_date: Secret<String>,
}

fn get_processing_info(
    request: &PaymentsAuthorizeData,
) -> VantivMandateDetail {
    if request.is_customer_initiated_mandate_payment() {
        VantivMandateDetail {
            processing_type: Some(VantivProcessingType::InitialCOF),
            network_transaction_id: None,
            token: None,
        }
    } else {
        match request
            .mandate_id
            .as_ref()
            .and_then(|mandate| mandate.mandate_reference_id.clone())
        {
            Some(api_models::payments::MandateReferenceId::NetworkMandateId(
                network_transaction_id,
            )) => VantivMandateDetail {
                processing_type: Some(VantivProcessingType::MerchantInitiatedCOF),
                network_transaction_id: Some(network_transaction_id.into()),
                token: None,
            },
            Some(api_models::payments::MandateReferenceId::ConnectorMandateId(
                _,
            )) => {
                let token = Some(TokenizationData {
                    cnp_token: Secret::new("Some token".to_string()),
                    exp_date: Secret::new("12/34".to_string()),
                });
                VantivMandateDetail {
                    processing_type: None,
                    network_transaction_id: None,
                    token,
                }
            }
            _ => VantivMandateDetail {
                processing_type: None,
                network_transaction_id: None,
                token: None,
            },
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
        let api_call_id = format!("capture_{:?}", connector_utils::generate_12_digit_number());
        let capture = Some(Capture {
            id: api_call_id,
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
            void: None,
            credit: None,
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

        let customer_id = item
            .router_data
            .customer_id
            .clone()
            .map(|customer_id| customer_id.get_string_repr().to_string());

        let api_call_id = format!("ref_{:?}", connector_utils::generate_12_digit_number());
        let credit = Some(RefundRequest {
            id: api_call_id,
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
            void: None,
            credit,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VantivSyncErrorResponse {
    pub error_messages: Vec<String>,
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
    pub authorization_response: Option<AuthorizationResponse>,
    pub sale_response: Option<SaleResponse>,
    pub capture_response: Option<CaptureResponse>,
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
    pub amount: Option<String>,
    pub purchase_currency: Option<String>,
    pub post_day: Option<String>,
    pub reported_timestamp: Option<String>,
    pub payment_type: Option<String>,
    pub merchant_order_number: Option<String>,
    pub merchant_txn_id: Option<String>,
    pub parent_id: Option<u64>,
    pub reporting_group: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
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
    #[serde(rename = "@customerId")]
    pub customer_id: Option<String>,
    #[serde(rename = "cnpTxnId")]
    pub cnp_txn_id: String,
    pub response: String,
    pub response_time: String,
    pub message: String,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FraudResult {
    pub avs_result: Option<String>,
    pub card_validation_result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationResponse {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@reportGroup")]
    pub report_group: String,
    #[serde(rename = "@customerId")]
    pub customer_id: Option<String>,
    pub cnp_txn_id: String,
    pub order_id: String,
    #[serde(rename = "response")]
    pub response_code: String,
    pub message: String,
    pub response_time: String,
    pub auth_code: Option<Secret<String>>,
    pub fraud_result: Option<FraudResult>,
    pub network_transaction_id: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SaleResponse {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@reportGroup")]
    pub report_group: String,
    #[serde(rename = "@customerId")]
    pub customer_id: Option<String>,
    pub cnp_txn_id: String,
    pub order_id: String,
    #[serde(rename = "response")]
    pub response_code: String,
    pub message: String,
    pub response_time: String,
    pub auth_code: Option<Secret<String>>,
    pub fraud_result: Option<FraudResult>,
    pub network_transaction_id: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoidResponse {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@reportGroup")]
    pub report_group: String,
    #[serde(rename = "@customerId")]
    pub customer_id: Option<String>,
    pub cnp_txn_id: String,
    pub response: String,
    pub response_time: String,
    pub post_date: String,
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
    #[serde(rename = "@customerId")]
    pub customer_id: Option<String>,
    pub cnp_txn_id: String,
    pub response: String,
    pub response_time: String,
    pub message: String,
    pub location: Option<String>,
}

impl<F> TryFrom<ResponseRouterData<F, CnpOnlineResponse, PaymentsCaptureData, PaymentsResponseData>>
    for RouterData<F, PaymentsCaptureData, PaymentsResponseData>
where
    F: Send,
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, CnpOnlineResponse, PaymentsCaptureData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response.capture_response {
            Some(capture_response) => Ok(Self {
                status: common_enums::AttemptStatus::CaptureInitiated,
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(capture_response.cnp_txn_id),
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
            None => Ok(Self {
                status: common_enums::AttemptStatus::CaptureFailed,
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
                }),
                ..item.data
            }),
        }
    }
}

impl<F> TryFrom<ResponseRouterData<F, CnpOnlineResponse, PaymentsCancelData, PaymentsResponseData>>
    for RouterData<F, PaymentsCancelData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, CnpOnlineResponse, PaymentsCancelData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response.void_response {
            Some(void_response) => Ok(Self {
                status: common_enums::AttemptStatus::VoidInitiated,
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(void_response.cnp_txn_id),
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
            Some(credit_response) => Ok(Self {
                response: Ok(RefundsResponseData {
                    connector_refund_id: credit_response.cnp_txn_id,

                    refund_status: common_enums::RefundStatus::Pending,
                }),
                ..item.data
            }),
            None => Ok(Self {
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
                }),
                ..item.data
            }),
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
        let api_call_id = format!("void_{:?}", connector_utils::generate_12_digit_number());
        let void = Some(Void {
            id: api_call_id,
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
        match (item.response.sale_response, item.response.authorization_response) {
            (Some(sale_response), None) => {
                    let report_group = WorldpayvantivPaymentMetadata {
                        report_group: Some(sale_response.report_group.clone()),
                    };
                    let connector_metadata =   Some(report_group.encode_to_value()
                    .change_context(errors::ConnectorError::ResponseHandlingFailed)?);

                let network_txn_id = sale_response.network_transaction_id.map(|network_transaction_id| network_transaction_id.expose());

                    Ok(Self {
                        status: common_enums::AttemptStatus::Pending,
                        response: Ok(PaymentsResponseData::TransactionResponse {
                            resource_id: ResponseId::ConnectorTransactionId(sale_response.cnp_txn_id),
                            redirection_data: Box::new(None),
                            mandate_reference: Box::new(None),
                            connector_metadata,
                            network_txn_id,
                            connector_response_reference_id: Some(sale_response.order_id),
                            incremental_authorization_allowed: None,
                            charges: None,
                        }),
                        ..item.data
                    })
            },
            (None, Some(auth_response)) => {
                    let report_group = WorldpayvantivPaymentMetadata {
                        report_group: Some(auth_response.report_group.clone()),
                    };
                    let connector_metadata =   Some(report_group.encode_to_value()
                    .change_context(errors::ConnectorError::ResponseHandlingFailed)?);

                let network_txn_id = auth_response.network_transaction_id.map(|network_transaction_id| network_transaction_id.expose());

                    Ok(Self {
                        status: common_enums::AttemptStatus::Authorizing,
                        response: Ok(PaymentsResponseData::TransactionResponse {
                            resource_id: ResponseId::ConnectorTransactionId(auth_response.cnp_txn_id),
                            redirection_data: Box::new(None),
                            mandate_reference: Box::new(None),
                            connector_metadata,
                            network_txn_id,
                            connector_response_reference_id: Some(auth_response.order_id),
                            incremental_authorization_allowed: None,
                            charges: None,
                        }),
                        ..item.data
                    })
            },
            (None, None) => { // Incase of API failure
                Ok(Self {status: common_enums::AttemptStatus::Failure,
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

            Ok(Self {
                response: Err(ErrorResponse {
                    code: error_code.clone(),
                    message: error_message.clone(),
                    reason: Some(error_message.clone()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
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

fn get_attempt_status_for_psync(
    vantiv_status: PaymentStatus,
    previous_status: common_enums::AttemptStatus,
) -> Result<common_enums::AttemptStatus, errors::ConnectorError> {
    match vantiv_status {
        PaymentStatus::ProcessedSuccessfully => {
            if previous_status == common_enums::AttemptStatus::Authorizing {
                Ok(common_enums::AttemptStatus::Authorized)
            } else if previous_status == common_enums::AttemptStatus::VoidInitiated {
                Ok(common_enums::AttemptStatus::Voided)
            } else {
                Ok(common_enums::AttemptStatus::Charged)
            }
        }
        PaymentStatus::TransactionDeclined => {
            if previous_status == common_enums::AttemptStatus::Authorizing {
                Ok(common_enums::AttemptStatus::AuthorizationFailed)
            } else if previous_status == common_enums::AttemptStatus::VoidInitiated {
                Ok(common_enums::AttemptStatus::VoidFailed)
            } else {
                Ok(common_enums::AttemptStatus::Failure)
            }
        }
        PaymentStatus::PaymentStatusNotFound => Ok(common_enums::AttemptStatus::Unresolved),
        PaymentStatus::NotYetProcessed | PaymentStatus::StatusUnavailable => Ok(previous_status),
    }
}

fn get_refund_status_for_rsync(
    vantiv_status: PaymentStatus,
) -> Result<common_enums::RefundStatus, errors::ConnectorError> {
    match vantiv_status {
        PaymentStatus::ProcessedSuccessfully => Ok(common_enums::RefundStatus::Success),
        PaymentStatus::TransactionDeclined => Ok(common_enums::RefundStatus::Failure),
        PaymentStatus::PaymentStatusNotFound => Ok(common_enums::RefundStatus::ManualReview),
        PaymentStatus::NotYetProcessed | PaymentStatus::StatusUnavailable => {
            Ok(common_enums::RefundStatus::Pending)
        }
    }
}
