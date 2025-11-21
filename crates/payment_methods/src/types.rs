use crate::{core::errors, helpers::validate_payment_method_type_against_payment_method};
use api_models::payment_methods::PaymentMethodCreate;
use common_utils::id_type;
use error_stack::report;
use hyperswitch_domain_models::router_data::{self, ErrorResponse};
use hyperswitch_domain_models::router_data_v2::flow_common_types as common_types;
use hyperswitch_interfaces::connector_integration_interface::BoxedConnectorIntegrationInterface;
use serde::{Deserialize, Serialize};
use masking::Secret;

pub type BoxedPaymentConnectorIntegrationInterface<T, Req, Resp> =
    BoxedConnectorIntegrationInterface<T, common_types::PaymentFlowData, Req, Resp>;
pub type BoxedVaultConnectorIntegrationInterface<T, Req, Res> =
    BoxedConnectorIntegrationInterface<T, common_types::VaultConnectorFlowData, Req, Res>;

#[cfg(feature = "v1")]
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteCardToken {
    pub card_reference: String, //network token requestor ref id
    pub customer_id: id_type::CustomerId,
}

#[cfg(feature = "v2")]
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteCardToken {
    pub card_reference: String, //network token requestor ref id
    pub customer_id: id_type::GlobalCustomerId,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum DeleteNetworkTokenStatus {
    Success,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct NetworkTokenErrorInfo {
    pub code: String,
    pub developer_message: String,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct NetworkTokenErrorResponse {
    pub error_message: String,
    pub error_info: NetworkTokenErrorInfo,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct DeleteNetworkTokenResponse {
    pub status: DeleteNetworkTokenStatus,
}

pub(crate) trait PaymentMethodCreateExt {
    fn validate(&self) -> errors::PmResult<()>;
}

// convert self.payment_method_type to payment_method and compare it against self.payment_method
#[cfg(feature = "v1")]
impl PaymentMethodCreateExt for PaymentMethodCreate {
    fn validate(&self) -> errors::PmResult<()> {
        if let Some(pm) = self.payment_method {
            if let Some(payment_method_type) = self.payment_method_type {
                if !validate_payment_method_type_against_payment_method(pm, payment_method_type) {
                    return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                        message: "Invalid 'payment_method_type' provided".to_string()
                    })
                    .attach_printable("Invalid payment method type"));
                }
            }
        }
        Ok(())
    }
}

#[cfg(feature = "v2")]
impl PaymentMethodCreateExt for PaymentMethodCreate {
    fn validate(&self) -> RouterResult<()> {
        utils::when(
            !validate_payment_method_type_against_payment_method(
                self.payment_method_type,
                self.payment_method_subtype,
            ),
            || {
                Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                    message: "Invalid 'payment_method_type' provided".to_string()
                })
                .attach_printable("Invalid payment method type"))
            },
        )?;

        utils::when(
            !Self::validate_payment_method_data_against_payment_method(
                self.payment_method_type,
                self.payment_method_data.clone(),
            ),
            || {
                Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                    message: "Invalid 'payment_method_data' provided".to_string()
                })
                .attach_printable("Invalid payment method data"))
            },
        )?;
        Ok(())
    }
}

pub struct PaymentMethodTokenResult {
    pub payment_method_token_result: Result<Option<String>, ErrorResponse>,
    pub is_payment_method_tokenization_performed: bool,
    pub connector_response: Option<router_data::ConnectorResponseData>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct AddVaultResponse {
    #[cfg(feature = "v2")]
    pub entity_id: Option<id_type::GlobalCustomerId>,
    #[cfg(feature = "v1")]
    pub entity_id: Option<id_type::CustomerId>,
    #[cfg(feature = "v2")]
    pub vault_id: domain::VaultId,
    #[cfg(feature = "v1")]
    pub vault_id: hyperswitch_domain_models::router_response_types::VaultIdType,
    pub fingerprint_id: Option<String>,
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug)]
pub enum TokenizationAction {
    TokenizeInRouter,
    TokenizeInConnector,
    TokenizeInConnectorAndRouter,
    ConnectorToken(String),
    SkipConnectorTokenization,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug)]
pub enum TokenizationAction {
    TokenizeInConnector,
    SkipConnectorTokenization,
}

#[cfg(feature = "v1")]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardNetworkTokenResponsePayload {
    pub card_brand: api_enums::CardNetwork,
    pub card_fingerprint: Option<Secret<String>>,
    pub card_reference: String,
    pub correlation_id: String,
    pub customer_id: String,
    pub par: String,
    pub token: cards::CardNumber,
    pub token_expiry_month: Secret<String>,
    pub token_expiry_year: Secret<String>,
    pub token_isin: String,
    pub token_last_four: String,
    pub token_status: String,
}
#[cfg(feature = "v1")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardData {
    pub card_number: cards::CardNumber,
    pub exp_month: Secret<String>,
    pub exp_year: Secret<String>,
    pub card_security_code: Option<Secret<String>>,
}

#[cfg(feature = "v2")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardData {
    pub card_number: CardNumber,
    pub exp_month: Secret<String>,
    pub exp_year: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_security_code: Option<Secret<String>>,
}