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
        PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self as connector_utils, CardData, PaymentsAuthorizeRequestData, RefundsRequestData},
};

pub mod worldpayvantiv_constants {
    pub const WORLDPAYVANTIV_VERSION: &str = "12.23";
    pub const XML_VERSION: &str = "1.0";
    pub const XML_ENCODING: &str = "UTF-8";
    pub const XMLNS: &str = "http://www.vantivcnp.com/schema";
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

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WorldpayvantivMetadataObject {
    pub report_group: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_transaction: Option<TransactionQuery>,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Authorization {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@reportGroup")]
    pub report_group: String,
    pub order_id: String,
    pub amount: MinorUnit,
    pub order_source: OrderSource,
    pub card: WorldpayvantivCardData,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sale {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@reportGroup")]
    pub report_group: String,
    pub order_id: String,
    pub amount: MinorUnit,
    pub order_source: OrderSource,
    pub card: WorldpayvantivCardData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundRequest {
    #[serde(rename = "@reportGroup")]
    pub report_group: String,
    #[serde(rename = "@id")]
    pub id: String,
    pub cnp_txn_id: String,
    pub amount: MinorUnit,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionQuery {
    #[serde(rename = "@reportGroup")]
    pub report_group: String,
    #[serde(rename = "@id")]
    pub id: String,
    pub orig_cnp_txn_id: String,
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
    pub card_validation_num: Secret<String>,
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

impl TryFrom<&PaymentMethodData> for WorldpayvantivCardData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(payment_method_data: &PaymentMethodData) -> Result<Self, Self::Error> {
        match payment_method_data {
            PaymentMethodData::Card(card) => {
                let card_type = match card.card_network.clone() {
                    Some(card_type) => WorldpayvativCardType::try_from(card_type)?,
                    None => WorldpayvativCardType::try_from(&card.get_card_issuer()?)?,
                };

                let exp_date = card.get_expiry_date_as_mmyy()?;

                Ok(Self {
                    card_type,
                    number: card.card_number.clone(),
                    exp_date,
                    card_validation_num: card.card_cvc.clone(),
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

impl TryFrom<&PaymentsSyncRouterData> for CnpOnlineRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let report_group_metadata: WorldpayvantivMetadataObject =
            connector_utils::to_connector_meta(item.request.connector_meta.clone())?;
        let api_call_id = format!("psync_{:?}", connector_utils::generate_12_digit_number());
        let query_transaction = Some(TransactionQuery {
            id: api_call_id,
            report_group: report_group_metadata.report_group.clone(),
            orig_cnp_txn_id: item
                .request
                .connector_transaction_id
                .get_connector_transaction_id()
                .change_context(errors::ConnectorError::MissingConnectorTransactionID)?,
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
            void: None,
            credit: None,
            query_transaction,
        })
    }
}

impl<F> TryFrom<ResponseRouterData<F, CnpOnlineResponse, PaymentsSyncData, PaymentsResponseData>>
    for RouterData<F, PaymentsSyncData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, CnpOnlineResponse, PaymentsSyncData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let connector_transaction_id = item
            .data
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;

        match item.response.query_transaction_response {
            Some(query_transaction_response) => {
                query_transaction_response
                    .results
                    .first()
                    .map(|result| {
                        if let Some(void_response) = &result.void_response {
                            let status = get_attempt_status(
                                WorldpayvantivPaymentFlow::Void,
                                void_response.response,
                            )?;
                            if connector_utils::is_payment_failure(status) {
                                Ok(Self {
                                    status,
                                    response: Err(ErrorResponse {
                                        code: void_response.response.to_string(),
                                        message: void_response.message.clone(),
                                        reason: Some(void_response.message.clone()),
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
                                            connector_transaction_id,
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
                        } else if let Some(capture_response) = &result.capture_response {
                            let status = get_attempt_status(
                                WorldpayvantivPaymentFlow::Capture,
                                capture_response.response,
                            )?;
                            if connector_utils::is_payment_failure(status) {
                                Ok(Self {
                                    status,
                                    response: Err(ErrorResponse {
                                        code: capture_response.response.to_string(),
                                        message: capture_response.message.clone(),
                                        reason: Some(capture_response.message.clone()),
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
                                            connector_transaction_id,
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
                        } else if let Some(sale_response) = &result.sale_response {
                            let status = get_attempt_status(
                                WorldpayvantivPaymentFlow::Sale,
                                sale_response.response_code.clone(),
                            )?;
                            if connector_utils::is_payment_failure(status) {
                                Ok(Self {
                                    status,
                                    response: Err(ErrorResponse {
                                        code: sale_response.response_code.to_string(),
                                        message: sale_response.message.clone(),
                                        reason: Some(sale_response.message.clone()),
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
                                            connector_transaction_id,
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
                        } else if let Some(authorization_response) = &result.authorization_response
                        {
                            let status = get_attempt_status(
                                WorldpayvantivPaymentFlow::Auth,
                                authorization_response.response_code.clone(),
                            )?;
                            if connector_utils::is_payment_failure(status) {
                                Ok(Self {
                                    status,
                                    response: Err(ErrorResponse {
                                        code: authorization_response.response_code.to_string(),
                                        message: authorization_response.message.clone(),
                                        reason: Some(authorization_response.message.clone()),
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
                                            connector_transaction_id,
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
                        } else {
                            // In case of Psync failure
                            Ok(Self {
                                status: item.data.status,
                                response: Ok(PaymentsResponseData::TransactionResponse {
                                    resource_id: ResponseId::ConnectorTransactionId(
                                        connector_transaction_id,
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
                    })
                    .ok_or(errors::ConnectorError::ResponseHandlingFailed)?
            }
            None => {
                // In case of 2xx Psync failure
                Ok(Self {
                    status: item.data.status,
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(connector_transaction_id),
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
}

impl TryFrom<&WorldpayvantivRouterData<&PaymentsAuthorizeRouterData>> for CnpOnlineRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &WorldpayvantivRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let card = WorldpayvantivCardData::try_from(
            &item.router_data.request.payment_method_data.clone(),
        )?;
        let default_report_group =
            WorldpayvantivMetadataObject::try_from(&item.router_data.connector_meta_data)?;
        let report_group = item
            .router_data
            .request
            .metadata
            .clone()
            .map(|payment_metadata| {
                connector_utils::to_connector_meta::<WorldpayvantivMetadataObject>(Some(
                    payment_metadata,
                ))
            })
            .transpose()?
            .map(|worldpayvantiv_metadata| worldpayvantiv_metadata.report_group)
            .unwrap_or(default_report_group.report_group);

        let worldpayvantiv_auth_type =
            WorldpayvantivAuthType::try_from(&item.router_data.connector_auth_type)?;
        let authentication = Authentication {
            user: worldpayvantiv_auth_type.user,
            password: worldpayvantiv_auth_type.password,
        };

        let (authorization, sale) = if item.router_data.request.is_auto_capture()? {
            (
                None,
                Some(Sale {
                    id: item.router_data.attempt_id.clone(),
                    report_group: report_group.clone(),
                    order_id: item.router_data.payment_id.clone(),
                    amount: item.amount,
                    order_source: OrderSource::Ecommerce,
                    card: card.clone(),
                }),
            )
        } else {
            (
                Some(Authorization {
                    id: item.router_data.attempt_id.clone(),
                    report_group: report_group.clone(),
                    order_id: item.router_data.payment_id.clone(),
                    amount: item.amount,
                    order_source: OrderSource::Ecommerce,
                    card: card.clone(),
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
            query_transaction: None,
        })
    }
}

impl TryFrom<&WorldpayvantivRouterData<&PaymentsCaptureRouterData>> for CnpOnlineRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &WorldpayvantivRouterData<&PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let report_group_metadata: WorldpayvantivMetadataObject =
            connector_utils::to_connector_meta(item.router_data.request.connector_meta.clone())?;
        let api_call_id = format!(
            "capture_{:?}",
            connector_utils::generate_12_digit_number()
        );
        let capture = Some(Capture {
            id: api_call_id,
            report_group: report_group_metadata.report_group.clone(),
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
            query_transaction: None,
        })
    }
}

impl<F> TryFrom<&WorldpayvantivRouterData<&RefundsRouterData<F>>> for CnpOnlineRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &WorldpayvantivRouterData<&RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        let report_group_metadata: WorldpayvantivMetadataObject =
            connector_utils::to_connector_meta(
                item.router_data.request.connector_metadata.clone(),
            )?;
        let api_call_id = format!("ref_{:?}", connector_utils::generate_12_digit_number());
        let credit = Some(RefundRequest {
            id: api_call_id,
            report_group: report_group_metadata.report_group.clone(),
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
            query_transaction: None,
        })
    }
}

impl TryFrom<&RefundSyncRouterData> for CnpOnlineRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RefundSyncRouterData) -> Result<Self, Self::Error> {
        let report_group_metadata: WorldpayvantivMetadataObject =
            connector_utils::to_connector_meta(item.request.connector_metadata.clone())?;
        let api_call_id = format!("rsync_{:?}", connector_utils::generate_12_digit_number());
        let query_transaction = Some(TransactionQuery {
            id: api_call_id,
            report_group: report_group_metadata.report_group.clone(),
            orig_cnp_txn_id: item.request.get_connector_refund_id()?,
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
            void: None,
            credit: None,
            query_transaction,
        })
    }
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
    pub query_transaction_response: Option<QueryTransactionResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureResponse {
    #[serde(rename = "@id")]
    pub id: String,

    #[serde(rename = "@reportGroup")]
    pub report_group: String,

    #[serde(rename = "cnpTxnId")]
    pub cnp_txn_id: String,

    pub response: WorldpayvantivResponseCode,

    pub response_time: String,

    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationResponse {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@reportGroup")]
    pub report_group: String,
    pub cnp_txn_id: String,
    pub order_id: String,
    #[serde(rename = "response")]
    pub response_code: WorldpayvantivResponseCode,
    pub message: String,
    pub response_time: String,
    pub auth_code: Secret<String>,
    pub network_transaction_id: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SaleResponse {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@reportGroup")]
    pub report_group: String,
    pub cnp_txn_id: String,
    pub order_id: String,
    #[serde(rename = "response")]
    pub response_code: WorldpayvantivResponseCode,
    pub message: String,
    pub response_time: String,
    pub auth_code: Secret<String>,
    pub network_transaction_id: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoidResponse {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@reportGroup")]
    pub report_group: String,
    pub cnp_txn_id: String,
    pub response: WorldpayvantivResponseCode,
    pub response_time: String,
    pub post_date: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryTransactionResponse {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@reportGroup")]
    pub report_group: String,
    pub response: WorldpayvantivResponseCode,
    pub response_time: String,
    pub message: String,
    pub match_count: u32,
    #[serde(rename = "results_max10")]
    pub results: Vec<ResultsMax10>,
    pub location: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResultsMax10 {
    pub authorization_response: Option<AuthorizationResponse>,
    pub sale_response: Option<SaleResponse>,
    pub capture_response: Option<CaptureResponse>,
    pub void_response: Option<VoidResponse>,
    pub credit_response: Option<CreditResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditResponse {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@reportGroup")]
    pub report_group: String,
    pub cnp_txn_id: String,
    pub response: WorldpayvantivResponseCode,
    pub response_time: String,
    pub message: String,
}

impl<F> TryFrom<ResponseRouterData<F, CnpOnlineResponse, PaymentsCaptureData, PaymentsResponseData>>
    for RouterData<F, PaymentsCaptureData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, CnpOnlineResponse, PaymentsCaptureData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response.capture_response {
            Some(capture_response) => {
                let status = get_attempt_status(
                    WorldpayvantivPaymentFlow::Capture,
                    capture_response.response.clone(),
                )?;
                if connector_utils::is_payment_failure(status) {
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
            Some(void_response) => {
                let status = get_attempt_status(
                    WorldpayvantivPaymentFlow::Void,
                    void_response.response.clone(),
                )?;
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
            Some(credit_response) => {
                let refund_status = get_refund_status(credit_response.response.clone())?;
                if connector_utils::is_refund_failure(refund_status) {
                    Ok(Self {
                        response: Err(ErrorResponse {
                            code: credit_response.response.to_string(),
                            message: credit_response.message.clone(),
                            reason: Some(credit_response.message.clone()),
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
                            connector_refund_id: credit_response.cnp_txn_id,
                            refund_status,
                        }),
                        ..item.data
                    })
                }
            }
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
        let report_group_metadata: WorldpayvantivMetadataObject =
            connector_utils::to_connector_meta(item.request.connector_meta.clone())?;
        let api_call_id = format!("void_{:?}", connector_utils::generate_12_digit_number());
        let void = Some(Void {
            id: api_call_id,
            report_group: report_group_metadata.report_group.clone(),
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
            query_transaction: None,
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
                let status = get_attempt_status(WorldpayvantivPaymentFlow::Sale, sale_response.response_code.clone())?;
                if connector_utils::is_payment_failure(status) {
                    Ok(Self {
                        status,
                        response: Err(ErrorResponse {
                            code: sale_response.response_code.to_string(),
                            message: sale_response.message.clone(),
                            reason: Some(sale_response.message.clone()),
                            status_code: item.http_code,
                            attempt_status: None,
                            connector_transaction_id: Some(sale_response.order_id),
                            network_advice_code: None,
                            network_decline_code: None,
                            network_error_message: None,
                        }),
                        ..item.data
                    })
                } else {
                    let report_group = WorldpayvantivMetadataObject {
                        report_group: sale_response.report_group.clone(),
                    };
                    let connector_metadata =   Some(report_group.encode_to_value()
                    .change_context(errors::ConnectorError::ResponseHandlingFailed)?);

                    Ok(Self {
                        status,
                        response: Ok(PaymentsResponseData::TransactionResponse {
                            resource_id: ResponseId::ConnectorTransactionId(sale_response.cnp_txn_id),
                            redirection_data: Box::new(None),
                            mandate_reference: Box::new(None),
                            connector_metadata,
                            network_txn_id: None,
                            connector_response_reference_id: Some(sale_response.order_id),
                            incremental_authorization_allowed: None,
                            charges: None,
                        }),
                        ..item.data
                    })
                }
            },
            (None, Some(auth_response)) => {
                let status = get_attempt_status(WorldpayvantivPaymentFlow::Auth, auth_response.response_code.clone())?;
                if connector_utils::is_payment_failure(status) {
                    Ok(Self {
                        status,
                        response: Err(ErrorResponse {
                            code: auth_response.response_code.to_string(),
                            message: auth_response.message.clone(),
                            reason: Some(auth_response.message.clone()),
                            status_code: item.http_code,
                            attempt_status: None,
                            connector_transaction_id: Some(auth_response.order_id),
                            network_advice_code: None,
                            network_decline_code: None,
                            network_error_message: None,
                        }),
                        ..item.data
                    })
                } else {
                    let report_group = WorldpayvantivMetadataObject {
                        report_group: auth_response.report_group.clone(),
                    };
                    let connector_metadata =   Some(report_group.encode_to_value()
                    .change_context(errors::ConnectorError::ResponseHandlingFailed)?);

                    Ok(Self {
                        status,
                        response: Ok(PaymentsResponseData::TransactionResponse {
                            resource_id: ResponseId::ConnectorTransactionId(auth_response.cnp_txn_id),
                            redirection_data: Box::new(None),
                            mandate_reference: Box::new(None),
                            connector_metadata,
                            network_txn_id: None,
                            connector_response_reference_id: Some(auth_response.order_id),
                            incremental_authorization_allowed: None,
                            charges: None,
                        }),
                        ..item.data
                    })
                }
            },
            (None, None) => { Ok(Self {
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
                }),
                ..item.data
            })},
            (_, _) => {  Err(errors::ConnectorError::UnexpectedResponseError(
                bytes::Bytes::from("Only one of 'sale_response' or 'authorisation_response' is expected, but both were recieved".to_string()),           
             ))?
            },
    }
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, CnpOnlineResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, CnpOnlineResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response.query_transaction_response {
            Some(query_transaction_response) => {
                let results = query_transaction_response
                    .results
                    .first()
                    .ok_or(errors::ConnectorError::ResponseHandlingFailed)?;
                if let Some(refund_response) = &results.credit_response {
                    let refund_status = get_refund_status(refund_response.response.clone())?;
                    if connector_utils::is_refund_failure(refund_status) {
                        Ok(Self {
                            response: Err(ErrorResponse {
                                code: refund_response.response.to_string(),
                                message: refund_response.message.clone(),
                                reason: Some(refund_response.message.clone()),
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
                                connector_refund_id: item
                                    .data
                                    .request
                                    .connector_refund_id
                                    .clone()
                                    .ok_or(errors::ConnectorError::MissingConnectorRefundID)?,
                                refund_status,
                            }),
                            ..item.data
                        })
                    }
                } else {
                    Ok(Self {
                        response: Ok(RefundsResponseData {
                            connector_refund_id: item
                                .data
                                .request
                                .connector_refund_id
                                .clone()
                                .ok_or(errors::ConnectorError::MissingConnectorRefundID)?,
                            refund_status: common_enums::RefundStatus::Pending,
                        }),
                        ..item.data
                    })
                }
            }
            None => Ok(Self {
                response: Ok(RefundsResponseData {
                    connector_refund_id: item
                        .data
                        .request
                        .connector_refund_id
                        .clone()
                        .ok_or(errors::ConnectorError::MissingConnectorRefundID)?,
                    refund_status: common_enums::RefundStatus::Pending,
                }),
                ..item.data
            }),
        }
    }
}

#[derive(Debug, strum::Display, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum WorldpayvantivResponseCode {
    #[serde(rename = "001")]
    TransactionReceived,
    #[serde(rename = "000")]
    Approved,
    #[serde(rename = "010")]
    PartiallyApproved,
    #[serde(rename = "011")]
    OfflineApproval,
    #[serde(rename = "013")]
    OfflineApprovalUnableToGoOnline,
    #[serde(rename = "014")]
    InquirySuccessful,
    #[serde(rename = "015")]
    PendingShopperCheckoutCompletion,
    #[serde(rename = "016")]
    ShopperCheckoutExpired, 
    #[serde(rename = "100")]
    ProcessingNetworkUnavailable, 
    #[serde(rename = "101")]
    IssuerUnavailable, 
    #[serde(rename = "102")]
    ReSubmitTransaction, 
    #[serde(rename = "103")]
    MerchantNotConfiguredForProcessingAtThisSite, 
    #[serde(rename = "108")]
    TryAgainLater,
    #[serde(rename = "110")]
    InsufficientFunds, 
    #[serde(rename = "111")]
    AuthorizationAmountHasAlreadyBeenDepleted, 
    #[serde(rename = "112")]
    InsufficientFundsRetryAfter1Hour, 
    #[serde(rename = "113")]
    InsufficientFundsRetryAfter24Hour, 
    #[serde(rename = "114")]
    InsufficientFundsRetryAfter2Days, 
    #[serde(rename = "115")]
    InsufficientFundsRetryAfter4Days, 
    #[serde(rename = "116")]
    InsufficientFundsRetryAfter6Days, 
    #[serde(rename = "117")]
    InsufficientFundsRetryAfter8Days, 
    #[serde(rename = "118")]
    InsufficientFundsRetryAfter10Days, 
    #[serde(rename = "120")]
    CallIssuer, 
    #[serde(rename = "121")]
    CallAmex, 
    #[serde(rename = "122")]
    CallDinersClub, 
    #[serde(rename = "123")]
    CallDiscover, 
    #[serde(rename = "124")]
    CallJbs, 
    #[serde(rename = "125")]
    CallVisaMastercard, 
    #[serde(rename = "126")]
    CallIssuerUpdateCardholderData, 
    #[serde(rename = "127")]
    ExceedsApprovalAmountLimit, 
    #[serde(rename = "130")]
    CallIndicatedNumber, 
    #[serde(rename = "131")]
    UnacceptablePinTransactionDeclinedRetry, 
    #[serde(rename = "132")]
    PinNotChanged, 
    #[serde(rename = "137")]
    ConsumerMultiUseVirtualCardNumberSoftDecline, 
    #[serde(rename = "138")]
    ConsumerNonReloadablePrepaidCardSoftDecline, 
    #[serde(rename = "139")]
    ConsumerSingleUseVirtualCardNumberSoftDecline, 
    #[serde(rename = "140")]
    UpdateCardholderData, 
    #[serde(rename = "141")]
    ConsumerNonReloadablePrepaidCardApproved, 
    #[serde(rename = "142")]
    ConsumerSingleUseVirtualCardNumberApproved, 
    #[serde(rename = "143")]
    MerchantDoesntQualifyForProductCode, 
    #[serde(rename = "145")]
    Lifecycle, 
    #[serde(rename = "146")]
    Policy, 
    #[serde(rename = "147")]
    FraudSecurity, 
    #[serde(rename = "148")]
    InvalidOrExpiredCardContactCardholderToUpdate, 
    #[serde(rename = "149")]
    InvalidTransactionOrCardRestrictionVerifyInformationAndResubmit, 
    #[serde(rename = "150")]
    OriginalTransactionFound, 
    #[serde(rename = "151")]
    OriginalTransactionNotFound, 
    #[serde(rename = "152")]
    OriginalTransactionFoundButResponseNotYetAvailable, 
    #[serde(rename = "153")]
    QueryTransactionNotEnabled, 
    #[serde(rename = "154")]
    AtLeastOneOfOrigIdOrOrigCnpTxnIdIsRequired, 
    #[serde(rename = "155")]
    OrigCnpTxnIdIsRequiredWhenShowStatusOnlyIsUsed, 
    #[serde(rename = "156")]
    IncrementalAuthNotSupported, 
    #[serde(rename = "157")]
    SetAuthIndicatorToIncremental, 
    #[serde(rename = "158")]
    IncrementalValueForAuthIndicatorNotAllowedInThisAuthStructure, 
    #[serde(rename = "159")]
    CannotRequestAnIncrementalAuthIfOriginalAuthNotSetToEstimated, 
    #[serde(rename = "161")]
    TransactionMustReferenceTheEstimatedAuth, 
    #[serde(rename = "162")]
    IncrementedAuthExceedsMaxTransactionAmount, 
    #[serde(rename = "170")]
    SubmittedMccNotAllowed, 
    #[serde(rename = "191")]
    TheMerchantIsNotRegisteredInTheUpdateProgram, 
    #[serde(rename = "192")]
    MerchantNotCertifiedEnabledForIias, 
    #[serde(rename = "206")]
    IssuerGeneratedError, 
    #[serde(rename = "207")]
    PickupCardOtherThanLostStolen, 
    #[serde(rename = "209")]
    InvalidAmountHardDecline, 
    #[serde(rename = "211")]
    ReversalUnsuccessful, 
    #[serde(rename = "212")]
    MissingData, 
    #[serde(rename = "213")]
    PickupCardLostCard, 
    #[serde(rename = "214")]
    PickupCardStolenCard, 
    #[serde(rename = "215")]
    RestrictedCard, 
    #[serde(rename = "216")]
    InvalidDeactivate, 
    #[serde(rename = "217")]
    CardAlreadyActive, 
    #[serde(rename = "218")]
    CardNotActive, 
    #[serde(rename = "219")]
    CardAlreadyDeactivate, 
    #[serde(rename = "221")]
    OverMaxBalance, 
    #[serde(rename = "222")]
    InvalidActivate, 
    #[serde(rename = "223")]
    NoTransactionFoundForReversal, 
    #[serde(rename = "226")]
    IncorrectCvv, 
    #[serde(rename = "229")]
    IllegalTransaction, 
    #[serde(rename = "251")]
    DuplicateTransaction, 
    #[serde(rename = "252")]
    SystemError, 
    #[serde(rename = "253")]
    DeconvertedBin, 
    #[serde(rename = "254")]
    MerchantDepleted, 
    #[serde(rename = "255")]
    GiftCardEscheated, 
    #[serde(rename = "256")]
    InvalidReversalTypeForCreditCardTransaction, 
    #[serde(rename = "257")]
    SystemErrorMessageFormatError, 
    #[serde(rename = "258")]
    SystemErrorCannotProcess, 
    #[serde(rename = "271")]
    RefundRejectedDueToPendingDepositStatus, 
    #[serde(rename = "272")]
    RefundRejectedDueToDeclinedDepositStatus, 
    #[serde(rename = "273")]
    RefundRejectedByTheProcessingNetwork, 
    #[serde(rename = "284")]
    CaptureCreditAndAuthReversalTagsCannotBeUsedForGiftCardTransactions, 
    #[serde(rename = "301")]
    InvalidAccountNumber, 
    #[serde(rename = "302")]
    AccountNumberDoesNotMatchPaymentType, 
    #[serde(rename = "303")]
    PickUpCard, 
    #[serde(rename = "304")]
    LostStolenCard, 
    #[serde(rename = "305")]
    ExpiredCard, 
    #[serde(rename = "306")]
    AuthorizationHasExpiredNoNeedToReverse, 
    #[serde(rename = "307")]
    RestrictedCardSoftDecline, 
    #[serde(rename = "308")]
    RestrictedCardChargeback, 
    #[serde(rename = "309")]
    RestrictedCardPrepaidCardFilteringService, 
    #[serde(rename = "310")]
    InvalidTrackData, 
    #[serde(rename = "311")]
    DepositIsAlreadyReferencedByAChargeback, 
    #[serde(rename = "312")]
    RestrictedCardInternationalCardFilteringService, 
    #[serde(rename = "313")]
    InternationalFilteringForIssuingCardCountry, 
    #[serde(rename = "315")]
    RestrictedCardAuthFraudVelocityFilteringService, 
    #[serde(rename = "316")]
    AutomaticRefundAlreadyIssued, 
    #[serde(rename = "317")]
    RestrictedCardCardUnderSanction, 
    #[serde(rename = "318")]
    RestrictedCardAuthFraudAdviceFilteringService, 
    #[serde(rename = "319")]
    RestrictedCardFraudAvsFilteringService, 
    #[serde(rename = "320")]
    InvalidExpirationDate, 
    #[serde(rename = "321")]
    InvalidMerchant, 
    #[serde(rename = "322")]
    InvalidTransaction, 
    #[serde(rename = "323")]
    NoSuchIssuer, 
    #[serde(rename = "324")]
    InvalidPin, 
    #[serde(rename = "325")]
    TransactionNotAllowedAtTerminal, 
    #[serde(rename = "326")]
    ExceedsNumberOfPinEntries, 
    #[serde(rename = "327")]
    CardholderTransactionNotPermitted, 
    #[serde(rename = "328")]
    CardholderRequestedThatRecurringOrInstallmentPaymentBeStopped, 
    #[serde(rename = "330")]
    InvalidPaymentType, 
    #[serde(rename = "331")]
    InvalidPosCapabilityForCardholderAuthorizedTerminalTransaction, 
    #[serde(rename = "332")]
    InvalidPosCardholderIdForCardholderAuthorizedTerminalTransaction, 
    #[serde(rename = "335")]
    ThisMethodOfPaymentDoesNotSupportAuthorizationReversals, 
    #[serde(rename = "336")]
    ReversalAmountDoesNotMatchAuthorizationAmount, 
    #[serde(rename = "337")]
    TransactionDidNotConvertToPinless, 
    #[serde(rename = "340")]
    InvalidAmountSoftDecline, 
    #[serde(rename = "341")]
    InvalidHealthcareAmounts, 
    #[serde(rename = "346")]
    InvalidBillingDescriptorPrefix, 
    #[serde(rename = "347")]
    InvalidBillingDescriptor, 
    #[serde(rename = "348")]
    InvalidReportGroup, 
    #[serde(rename = "349")]
    DoNotHonor, 
    #[serde(rename = "350")]
    GenericDecline, // Soft or Hard Decline
    #[serde(rename = "351")]
    DeclineRequestPositiveId, 
    #[serde(rename = "352")]
    DeclineCvv2CidFail, 
    #[serde(rename = "354")]
    ThreeDSecureTransactionNotSupportedByMerchant,
    #[serde(rename = "356")]
    InvalidPurchaseLevelIiiTheTransactionContainedBadOrMissingData, 
    #[serde(rename = "357")]
    MissingHealthcareIiasTagForAnFsaTransaction, 
    #[serde(rename = "358")]
    RestrictedByVantivDueToSecurityCodeMismatch, 
    #[serde(rename = "360")]
    NoTransactionFoundWithSpecifiedTransactionId, 
    #[serde(rename = "361")]
    AuthorizationNoLongerAvailable, 
    #[serde(rename = "362")]
    TransactionNotVoidedAlreadySettled, 
    #[serde(rename = "363")]
    AutoVoidOnRefund, 
    #[serde(rename = "364")]
    InvalidAccountNumberOriginalOrNocUpdatedECheckAccountRequired, 
    #[serde(rename = "365")]
    TotalCreditAmountExceedsCaptureAmount, 
    #[serde(rename = "366")]
    ExceedTheThresholdForSendingRedeposits, 
    #[serde(rename = "367")]
    DepositHasNotBeenReturnedForInsufficientNonSufficientFunds, 
    #[serde(rename = "368")]
    InvalidCheckNumber, 
    #[serde(rename = "369")]
    RedepositAgainstInvalidTransactionType, 
    #[serde(rename = "370")]
    InternalSystemErrorCallVantiv, 
    #[serde(rename = "371")]
    OriginalTransactionHasBeenProcessedFutureRedepositsCanceled, 
    #[serde(rename = "372")]
    SoftDeclineAutoRecyclingInProgress, 
    #[serde(rename = "373")]
    HardDeclineAutoRecyclingComplete, 
    #[serde(rename = "375")]
    MerchantIsNotEnabledForSurcharging, 
    #[serde(rename = "376")]
    ThisMethodOfPaymentDoesNotSupportSurcharging, 
    #[serde(rename = "377")]
    SurchargeIsNotValidForDebitOrPrepaidCards, 
    #[serde(rename = "378")]
    SurchargeCannotExceedsTheMaximumAllowedLimit, 
    #[serde(rename = "379")]
    TransactionDeclinedByTheProcessingNetwork, 
    #[serde(rename = "380")]
    SecondaryAmountCannotExceedTheSaleAmount, 
    #[serde(rename = "381")]
    ThisMethodOfPaymentDoesNotSupportSecondaryAmount, 
    #[serde(rename = "382")]
    SecondaryAmountCannotBeLessThanZero, 
    #[serde(rename = "383")]
    PartialTransactionIsNotSupportedWhenIncludingASecondaryAmount, 
    #[serde(rename = "384")]
    SecondaryAmountRequiredOnPartialRefundWhenUsedOnDeposit, 
    #[serde(rename = "385")]
    SecondaryAmountNotAllowedOnRefundIfNotIncludedOnDeposit, 
    #[serde(rename = "386")]
    ProcessingNetworkError, 
    #[serde(rename = "401")]
    InvalidEMail, 
    #[serde(rename = "466")]
    InvalidCombinationOfAccountFundingTransactionTypeAndMcc, 
    #[serde(rename = "467")]
    InvalidAccountFundingTransactionTypeForThisMethodOfPayment, 
    #[serde(rename = "468")]
    MissingOneOrMoreReceiverFieldsForAccountFundingTransaction, 
    #[serde(rename = "469")]
    InvalidRecurringRequestSeeRecurringResponseForDetails, 
    #[serde(rename = "470")]
    ApprovedRecurringSubscriptionCreated, 
    #[serde(rename = "471")]
    ParentTransactionDeclinedRecurringSubscriptionNotCreated, 
    #[serde(rename = "472")]
    InvalidPlanCode, 
    #[serde(rename = "473")]
    ScheduledRecurringPaymentProcessed, 
    #[serde(rename = "475")]
    InvalidSubscriptionId, 
    #[serde(rename = "476")]
    AddOnCodeAlreadyExists, 
    #[serde(rename = "477")]
    DuplicateAddOnCodesInRequests, 
    #[serde(rename = "478")]
    NoMatchingAddOnCodeForTheSubscription, 
    #[serde(rename = "480")]
    NoMatchingDiscountCodeForTheSubscription, 
    #[serde(rename = "481")]
    DuplicateDiscountCodesInRequest, 
    #[serde(rename = "482")]
    InvalidStartDate, 
    #[serde(rename = "483")]
    MerchantNotRegisteredForRecurringEngine, 
    #[serde(rename = "484")]
    InsufficientDataToUpdateSubscription, 
    #[serde(rename = "485")]
    InvalidBillingDate, 
    #[serde(rename = "486")]
    DiscountCodeAlreadyExists, 
    #[serde(rename = "487")]
    PlanCodeAlreadyExists, 
    #[serde(rename = "500")]
    TheAccountNumberWasChanged, 
    #[serde(rename = "501")]
    TheAccountWasClosed, 
    #[serde(rename = "502")]
    TheExpirationDateWasChanged, 
    #[serde(rename = "503")]
    TheIssuingBankDoesNotParticipateInTheUpdateProgram, 
    #[serde(rename = "504")]
    ContactTheCardholderForUpdatedInformation, 
    #[serde(rename = "505")]
    NoMatchFound, 
    #[serde(rename = "506")]
    NoChangesFound, 
    #[serde(rename = "507")]
    TheCardholderHasOptedOutOfTheUpdateProgram, 
    #[serde(rename = "521")]
    SoftDeclineCardReaderDecryptionServiceIsNotAvailable, 
    #[serde(rename = "523")]
    SoftDeclineDecryptionFailed, 
    #[serde(rename = "524")]
    HardDeclineInputDataIsInvalid, 
    #[serde(rename = "530")]
    ApplePayKeyMismatch, 
    #[serde(rename = "531")]
    ApplePayDecryptionFailed, 
    #[serde(rename = "540")]
    HardDeclineDecryptionFailed, 
    #[serde(rename = "550")]
    AdvancedFraudFilterScoreBelowThreshold, 
    #[serde(rename = "555")]
    SuspectedFraud, 
    #[serde(rename = "560")]
    SystemErrorContactWorldpayRepresentative, 
    #[serde(rename = "561")]
    AmazonPayAmazonUnavailable, 
    #[serde(rename = "562")]
    AmazonPayAmazonDeclined, 
    #[serde(rename = "563")]
    AmazonPayInvalidToken, 
    #[serde(rename = "564")]
    MerchantNotEnabledForAmazonPay, 
    #[serde(rename = "565")]
    TransactionNotSupportedBlockedByIssuer, 
    #[serde(rename = "566")]
    BlockedByCardholderContactCardholder, 
    #[serde(rename = "601")]
    SoftDeclinePrimaryFundingSourceFailed, 
    #[serde(rename = "602")]
    SoftDeclineBuyerHasAlternateFundingSource, 
    #[serde(rename = "610")]
    HardDeclineInvalidBillingAgreementId, 
    #[serde(rename = "611")]
    HardDeclinePrimaryFundingSourceFailed, 
    #[serde(rename = "612")]
    HardDeclineIssueWithPaypalAccount, 
    #[serde(rename = "613")]
    HardDeclinePayPalAuthorizationIdMissing, 
    #[serde(rename = "614")]
    HardDeclineConfirmedEmailAddressIsNotAvailable, 
    #[serde(rename = "615")]
    HardDeclinePayPalBuyerAccountDenied, 
    #[serde(rename = "616")]
    HardDeclinePayPalBuyerAccountRestricted, 
    #[serde(rename = "617")]
    HardDeclinePayPalOrderHasBeenVoidedExpiredOrCompleted, 
    #[serde(rename = "618")]
    HardDeclineIssueWithPayPalRefund, 
    #[serde(rename = "619")]
    HardDeclinePayPalCredentialsIssue, 
    #[serde(rename = "620")]
    HardDeclinePayPalAuthorizationVoidedOrExpired, 
    #[serde(rename = "621")]
    HardDeclineRequiredPayPalParameterMissing, 
    #[serde(rename = "622")]
    HardDeclinePayPalTransactionIdOrAuthIdIsInvalid, 
    #[serde(rename = "623")]
    HardDeclineExceededMaximumNumberOfPayPalAuthorizationAttempts, 
    #[serde(rename = "624")]
    HardDeclineTransactionAmountExceedsMerchantsPayPalAccountLimit, 
    #[serde(rename = "625")]
    HardDeclinePayPalFundingSourcesUnavailable, 
    #[serde(rename = "626")]
    HardDeclineIssueWithPayPalPrimaryFundingSource, 
    #[serde(rename = "627")]
    HardDeclinePayPalProfileDoesNotAllowThisTransactionType, 
    #[serde(rename = "628")]
    InternalSystemErrorWithPayPalContactVantiv, 
    #[serde(rename = "629")]
    HardDeclineContactPayPalConsumerForAnotherPaymentMethod, 
    #[serde(rename = "637")]
    InvalidTerminalId, 
    #[serde(rename = "640")]
    PinlessDebitProcessingNotSupportedForNonRecurringTransactions, 
    #[serde(rename = "641")]
    PinlessDebitProcessingNotSupportedForPartialAuths, 
    #[serde(rename = "642")]
    MerchantNotConfiguredForPinlessDebitProcessing, 
    #[serde(rename = "651")]
    DeclineCustomerCancellation, 
    #[serde(rename = "652")]
    DeclineReTryTransaction, 
    #[serde(rename = "653")]
    DeclineUnableToLocateRecordOnFile, 
    #[serde(rename = "654")]
    DeclineFileUpdateFieldEditError, 
    #[serde(rename = "655")]
    RemoteFunctionUnknown, 
    #[serde(rename = "656")]
    DeclinedExceedsWithdrawalFrequencyLimit, 
    #[serde(rename = "657")]
    DeclineCardRecordNotAvailable, 
    #[serde(rename = "658")]
    InvalidAuthorizationCode, 
    #[serde(rename = "659")]
    ReconciliationError, 
    #[serde(rename = "660")]
    PreferredDebitRoutingDenialCreditTransactionCanBeDebit, 
    #[serde(rename = "661")]
    DeclinedCurrencyConversionCompleteNoAuthPerformed, 
    #[serde(rename = "662")]
    DeclinedMultiCurrencyDccFail, 
    #[serde(rename = "663")]
    DeclinedMultiCurrencyInvertFail, 
    #[serde(rename = "664")]
    Invalid3DSecurePassword, 
    #[serde(rename = "665")]
    InvalidSocialSecurityNumber, 
    #[serde(rename = "666")]
    InvalidMothersMaidenName, 
    #[serde(rename = "667")]
    EnrollmentInquiryDeclined, 
    #[serde(rename = "668")]
    SocialSecurityNumberNotAvailable, 
    #[serde(rename = "669")]
    MothersMaidenNameNotAvailable, 
    #[serde(rename = "670")]
    PinAlreadyExistsOnDatabase, 
    #[serde(rename = "701")]
    Under18YearsOld, 
    #[serde(rename = "702")]
    BillToOutsideUsa, 
    #[serde(rename = "703")]
    BillToAddressIsNotEqualToShipToAddress, 
    #[serde(rename = "704")]
    DeclinedForeignCurrencyMustBeUsd, 
    #[serde(rename = "705")]
    OnNegativeFile, 
    #[serde(rename = "706")]
    BlockedAgreement, 
    #[serde(rename = "707")]
    InsufficientBuyingPower, // Other
    #[serde(rename = "708")]
    InvalidData, 
    #[serde(rename = "709")]
    InvalidDataDataElementsMissing, 
    #[serde(rename = "710")]
    InvalidDataDataFormatError, 
    #[serde(rename = "711")]
    InvalidDataInvalidTCVersion, 
    #[serde(rename = "712")]
    DuplicateTransactionPaypalCredit, 
    #[serde(rename = "713")]
    VerifyBillingAddress, 
    #[serde(rename = "714")]
    InactiveAccount, 
    #[serde(rename = "716")]
    InvalidAuth, 
    #[serde(rename = "717")]
    AuthorizationAlreadyExistsForTheOrder, 
    #[serde(rename = "730")]
    LodgingTransactionsAreNotAllowedForThisMcc, 
    #[serde(rename = "731")]
    DurationCannotBeNegative, 
    #[serde(rename = "732")]
    HotelFolioNumberCannotBeBlank, 
    #[serde(rename = "733")]
    InvalidCheckInDate, 
    #[serde(rename = "734")]
    InvalidCheckOutDate, 
    #[serde(rename = "735")]
    InvalidCheckInOrCheckOutDate, 
    #[serde(rename = "736")]
    CheckOutDateCannotBeBeforeCheckInDate, 
    #[serde(rename = "737")]
    NumberOfAdultsCannotBeNegative, 
    #[serde(rename = "738")]
    RoomRateCannotBeNegative, 
    #[serde(rename = "739")]
    RoomTaxCannotBeNegative, 
    #[serde(rename = "740")]
    DurationCanOnlyBeFrom0To99ForVisa, 
    #[serde(rename = "801")]
    AccountNumberWasSuccessfullyRegistered, 
    #[serde(rename = "802")]
    AccountNumberWasPreviouslyRegistered, 
    #[serde(rename = "803")]
    ValidToken, 
    #[serde(rename = "805")]
    CardValidationNumberUpdated, 
    #[serde(rename = "820")]
    CreditCardNumberWasInvalid, 
    #[serde(rename = "821")]
    MerchantIsNotAuthorizedForTokens, 
    #[serde(rename = "822")]
    TokenWasNotFound, 
    #[serde(rename = "823")]
    TokenInvalid, 
    #[serde(rename = "825")]
    MerchantNotAuthorizedForECheckTokens, 
    #[serde(rename = "826")]
    CheckoutIdWasInvalid, 
    #[serde(rename = "827")]
    CheckoutIdWasNotFound, 
    #[serde(rename = "828")]
    GenericCheckoutIdError, 
    #[serde(rename = "835")]
    CaptureAmountCanNotBeMoreThanAuthorizedAmount, 
    #[serde(rename = "850")]
    TaxBillingOnlyAllowedForMcc9311, 
    #[serde(rename = "851")]
    Mcc9311RequiresTaxTypeElement, 
    #[serde(rename = "852")]
    DebtRepaymentOnlyAllowedForViTransactionsOnMccs6012And6051, 
    #[serde(rename = "861")]
    RoutingNumberDidNotMatchOneOnFileForToken, 
    #[serde(rename = "877")]
    InvalidPayPageRegistrationId, 
    #[serde(rename = "878")]
    ExpiredPayPageRegistrationId, 
    #[serde(rename = "879")]
    MerchantIsNotAuthorizedForPayPage, 
    #[serde(rename = "890")]
    MaximumNumberOfUpdatesForThisTokenExceeded, 
    #[serde(rename = "891")]
    TooManyTokensCreatedForExistingNamespace, 
    #[serde(rename = "895")]
    PinValidationNotPossible, 
    #[serde(rename = "898")]
    GenericTokenRegistrationError, 
    #[serde(rename = "899")]
    GenericTokenUseError, 
    #[serde(rename = "900")]
    InvalidBankRoutingNumber, 
    #[serde(rename = "901")]
    MissingName, 
    #[serde(rename = "902")]
    InvalidName, 
    #[serde(rename = "903")]
    MissingBillingCountryCode, 
    #[serde(rename = "904")]
    InvalidIban, 
    #[serde(rename = "905")]
    MissingEmailAddress, 
    #[serde(rename = "906")]
    MissingMandateReference, 
    #[serde(rename = "907")]
    InvalidMandateReference, 
    #[serde(rename = "908")]
    MissingMandateUrl, 
    #[serde(rename = "909")]
    InvalidMandateUrl, 
    #[serde(rename = "911")]
    MissingMandateSignatureDate, 
    #[serde(rename = "912")]
    InvalidMandateSignatureDate, 
    #[serde(rename = "913")]
    RecurringMandateAlreadyExists, 
    #[serde(rename = "914")]
    RecurringMandateWasNotFound, 
    #[serde(rename = "915")]
    FinalRecurringWasAlreadyReceivedUsingThisMandate, 
    #[serde(rename = "916")]
    IbanDidNotMatchOneOnFileForMandate, 
    #[serde(rename = "917")]
    InvalidBillingCountry, 
    #[serde(rename = "922")]
    ExpirationDateRequiredForInteracTransaction, 
    #[serde(rename = "923")]
    TransactionTypeIsNotSupportedWithThisMethodOfPayment, 
    #[serde(rename = "924")]
    UnreferencedOrphanRefundsAreNotAllowed, 
    #[serde(rename = "939")]
    UnableToVoidATransactionWithAHeldState, 
    #[serde(rename = "940")]
    ThisFundingInstructionResultsInANegativeAccountBalance, 
    #[serde(rename = "941")]
    AccountBalanceInformationUnavailableAtThisTime, 
    #[serde(rename = "942")]
    TheSubmittedCardIsNotEligibleForFastAccessFunding, 
    #[serde(rename = "943")]
    TransactionCannotUseBothCcdPaymentInformationAndCtxPaymentInformation, 
    #[serde(rename = "944")]
    ProcessingError, 
    #[serde(rename = "945")]
    ThisFundingInstructionTypeIsInvalidForCanadianMerchants, 
    #[serde(rename = "946")]
    CtxAndCcdRecordsAreNotAllowedForCanadianMerchants, 
    #[serde(rename = "947")]
    CanadianAccountNumberCannotExceed12Digits, 
    #[serde(rename = "948")]
    ThisFundingInstructionTypeIsInvalid, 
    #[serde(rename = "950")]
    DeclineNegativeInformationOnFile, 
    #[serde(rename = "951")]
    AbsoluteDecline, 
    #[serde(rename = "952")]
    TheMerchantProfileDoesNotAllowTheRequestedOperation, 
    #[serde(rename = "953")]
    TheAccountCannotAcceptAchTransactions, 
    #[serde(rename = "954")]
    TheAccountCannotAcceptAchTransactionsOrSiteDrafts, 
    #[serde(rename = "955")]
    AmountGreaterThanLimitSpecifiedInTheMerchantProfile, 
    #[serde(rename = "956")]
    MerchantIsNotAuthorizedToPerformECheckVerificationTransactions, 
    #[serde(rename = "957")]
    FirstNameAndLastNameRequiredForECheckVerifications, 
    #[serde(rename = "958")]
    CompanyNameRequiredForCorporateAccountForECheckVerifications, 
    #[serde(rename = "959")]
    PhoneNumberRequiredForECheckVerifications, 
    #[serde(rename = "961")]
    CardBrandTokenNotSupported, 
    #[serde(rename = "962")]
    PrivateLabelCardNotSupported, 
    #[serde(rename = "965")]
    AllowedDailyDirectDebitCaptureECheckSaleLimitExceeded, 
    #[serde(rename = "966")]
    AllowedDailyDirectDebitCreditECheckCreditLimitExceeded, 
    #[serde(rename = "973")]
    AccountNotEligibleForRtp, 
    #[serde(rename = "980")]
    SoftDeclineCustomerAuthenticationRequired, 
    #[serde(rename = "981")]
    TransactionNotReversedVoidWorkflowNeedToBeInvoked, 
    #[serde(rename = "982")]
    TransactionReversalNotSupportedForTheCoreMerchants, 
    #[serde(rename = "983")]
    NoValidParentDepositOrParentRefundFound, 
    #[serde(rename = "984")]
    TransactionReversalNotEnabledForVisa, 
    #[serde(rename = "985")]
    TransactionReversalNotEnabledForMastercard, 
    #[serde(rename = "986")]
    TransactionReversalNotEnabledForAmEx, 
    #[serde(rename = "987")]
    TransactionReversalNotEnabledForDiscover, 
    #[serde(rename = "988")]
    TransactionReversalNotSupported, 
    #[serde(rename = "990")]
    FundingInstructionHeldPleaseContactYourRelationshipManager, 
    #[serde(rename = "991")]
    MissingAddressInformation, 
    #[serde(rename = "992")]
    CryptographicFailure, 
    #[serde(rename = "993")]
    InvalidRegionCode, 
    #[serde(rename = "994")]
    InvalidCountryCode, 
    #[serde(rename = "995")]
    InvalidCreditAccount, 
    #[serde(rename = "996")]
    InvalidCheckingAccount, 
    #[serde(rename = "997")]
    InvalidSavingsAccount, 
    #[serde(rename = "998")]
    InvalidUseOfMccCorrectAndReattempt, 
    #[serde(rename = "999")]
    ExceedsRtpTransactionLimit, 
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum WorldpayvantivPaymentFlow {
    Sale,
    Auth,
    Capture,
    Void,
}

fn get_attempt_status(
    flow: WorldpayvantivPaymentFlow,
    response: WorldpayvantivResponseCode,
) -> Result<common_enums::AttemptStatus, errors::ConnectorError> {
    match response {
        WorldpayvantivResponseCode::Approved
        | WorldpayvantivResponseCode::PartiallyApproved
        | WorldpayvantivResponseCode::OfflineApproval
        | WorldpayvantivResponseCode::OfflineApprovalUnableToGoOnline => match flow {
            WorldpayvantivPaymentFlow::Sale => Ok(common_enums::AttemptStatus::Charged),
            WorldpayvantivPaymentFlow::Auth => Ok(common_enums::AttemptStatus::Authorized),
            WorldpayvantivPaymentFlow::Capture => Ok(common_enums::AttemptStatus::Charged),
            WorldpayvantivPaymentFlow::Void => Ok(common_enums::AttemptStatus::Voided),
        }
        WorldpayvantivResponseCode::PendingShopperCheckoutCompletion
        | WorldpayvantivResponseCode::TransactionReceived => {
            Ok(common_enums::AttemptStatus::Pending)
        }

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
        | WorldpayvantivResponseCode::ConsumerNonReloadablePrepaidCardApproved
        | WorldpayvantivResponseCode::ConsumerSingleUseVirtualCardNumberApproved
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
        | WorldpayvantivResponseCode::ApprovedRecurringSubscriptionCreated
        | WorldpayvantivResponseCode::ParentTransactionDeclinedRecurringSubscriptionNotCreated
        | WorldpayvantivResponseCode::InvalidPlanCode
        | WorldpayvantivResponseCode::ScheduledRecurringPaymentProcessed
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
        |    WorldpayvantivResponseCode::CreditCardNumberWasInvalid
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
            WorldpayvantivPaymentFlow::Sale
            | WorldpayvantivPaymentFlow::Auth => Ok(common_enums::AttemptStatus::Failure),
            WorldpayvantivPaymentFlow::Capture => Ok(common_enums::AttemptStatus::CaptureFailed),
            WorldpayvantivPaymentFlow::Void => Ok(common_enums::AttemptStatus::VoidFailed)
        }
        _  => {
            Err(errors::ConnectorError::UnexpectedResponseError(
                bytes::Bytes::from("Invalid response code ".to_string()),
            ))
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
                Ok(common_enums::RefundStatus::Success)
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
        | WorldpayvantivResponseCode::ConsumerNonReloadablePrepaidCardApproved
        | WorldpayvantivResponseCode::ConsumerSingleUseVirtualCardNumberApproved
        | WorldpayvantivResponseCode::MerchantDoesntQualifyForProductCode
        | WorldpayvantivResponseCode::Lifecycle
        | WorldpayvantivResponseCode::Policy
        | WorldpayvantivResponseCode::InvalidOrExpiredCardContactCardholderToUpdate
        | WorldpayvantivResponseCode::InvalidTransactionOrCardRestrictionVerifyInformationAndResubmit
        | WorldpayvantivResponseCode::OriginalTransactionNotFound
        | WorldpayvantivResponseCode::QueryTransactionNotEnabled
        | WorldpayvantivResponseCode::AtLeastOneOfOrigIdOrOrigCnpTxnIdIsRequired
        | WorldpayvantivResponseCode::OrigCnpTxnIdIsRequiredWhenShowStatusOnlyIsUsed
        | WorldpayvantivResponseCode::TransactionMustReferenceTheEstimatedAuth
        | WorldpayvantivResponseCode::IncrementedAuthExceedsMaxTransactionAmount
        | WorldpayvantivResponseCode::SubmittedMccNotAllowed
        | WorldpayvantivResponseCode::TheMerchantIsNotRegisteredInTheUpdateProgram
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
