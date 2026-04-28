use hyperswitch_domain_models::{
    router_request_types::authentication::{
        ConnectorAuthenticationRequestData, ConnectorPostAuthenticationRequestData,
        PreAuthNRequestData,
    },
    router_response_types::AuthenticationResponseData,
};
use hyperswitch_interfaces::errors;
use api_models::authentication as api_authentication;

use common_utils::{id_type, types::MinorUnit};
use crate::types::{self, ResponseRouterData};

pub struct ModularAuthenticationRouterData<T> {
    pub amount: MinorUnit,
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for ModularAuthenticationRouterData<T> {
    fn from((amount, router_data): (MinorUnit, T)) -> Self {
        Self {
            amount,
            router_data,
        }
    }
}

pub struct ModularAuthenticationAuthType {
    pub(super) api_key: hyperswitch_masking::Secret<String>,
}

impl TryFrom<&hyperswitch_domain_models::router_data::ConnectorAuthType> for ModularAuthenticationAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(auth_type: &hyperswitch_domain_models::router_data::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            hyperswitch_domain_models::router_data::ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

// ----------------------------------------
// Authentication Create
// ----------------------------------------

#[derive(Debug, serde::Serialize)]
pub struct ModularAuthenticationCreateRequest {
    pub amount: MinorUnit,
    pub currency: common_enums::Currency,
    pub return_url: Option<String>,
    pub authentication_connector: Option<common_enums::AuthenticationConnectors>,
    pub force_3ds_challenge: Option<bool>,
    pub psd2_sca_exemption_type: Option<common_enums::ScaExemptionType>,
    pub profile_acquirer_id: Option<common_utils::id_type::ProfileAcquirerId>,
    pub acquirer_details: Option<api_models::authentication::AcquirerDetails>,
}

impl TryFrom<&types::RouterData<hyperswitch_domain_models::router_flow_types::authentication::AuthenticationCreate, hyperswitch_domain_models::router_request_types::authentication::ConnectorAuthenticationCreateRequestData, AuthenticationResponseData>> for ModularAuthenticationCreateRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &types::RouterData<hyperswitch_domain_models::router_flow_types::authentication::AuthenticationCreate, hyperswitch_domain_models::router_request_types::authentication::ConnectorAuthenticationCreateRequestData, AuthenticationResponseData>) -> Result<Self, Self::Error> {
        let amount = item.request.amount.ok_or(errors::ConnectorError::MissingRequiredField { field_name: "amount" })?;
        let currency = item.request.currency.ok_or(errors::ConnectorError::MissingRequiredField { field_name: "currency" })?;
        let minor_amount = MinorUnit::new(amount);
        let acquirer_details = if item.request.acquirer_bin.is_some() || item.request.acquirer_merchant_id.is_some() {
            Some(api_models::authentication::AcquirerDetails {
                acquirer_bin: item.request.acquirer_bin.clone(),
                acquirer_merchant_id: item.request.acquirer_merchant_id.clone(),
                merchant_country_code: item.request.merchant_country_code.clone(),
            })
        } else {
            None
        };
        Ok(Self {
            amount: minor_amount,
            currency,
            return_url: item.request.return_url.clone(),
            authentication_connector: item.request.authentication_connector,
            force_3ds_challenge: item.request.force_3ds_challenge,
            psd2_sca_exemption_type: item.request.psd2_sca_exemption_type,
            profile_acquirer_id: item.request.profile_acquirer_id.clone(),
            acquirer_details,
        })
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModularAuthenticationCreateResponse {
    pub authentication_id: String,
    pub error_message: Option<String>,
}

impl<F, T> TryFrom<ResponseRouterData<F, ModularAuthenticationCreateResponse, T, AuthenticationResponseData>>
    for types::RouterData<F, T, AuthenticationResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, ModularAuthenticationCreateResponse, T, AuthenticationResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(AuthenticationResponseData::PreAuthNResponse {
                threeds_server_transaction_id: item.response.authentication_id.clone(),
                maximum_supported_3ds_version: common_utils::types::SemanticVersion::new(2, 2, 0),
                connector_authentication_id: item.response.authentication_id,
                three_ds_method_data: None,
                three_ds_method_url: None,
                message_version: common_utils::types::SemanticVersion::new(2, 2, 0),
                connector_metadata: None,
                directory_server_id: None,
                scheme_id: None,
            }),
            ..item.data
        })
    }
}

// ----------------------------------------
// PreAuthentication
// ----------------------------------------

#[derive(Debug, serde::Serialize)]
pub struct ModularAuthenticationPreAuthRequest {
    pub payment_method_data: api_models::payments::PaymentMethodData,
    pub payment_method: common_enums::PaymentMethod,
    pub payment_method_type: Option<common_enums::PaymentMethodType>,
    pub client_secret: Option<hyperswitch_masking::Secret<String>>,
    pub billing: Option<api_models::payments::Address>,
    pub shipping: Option<api_models::payments::Address>,
    pub browser_information: Option<api_models::payments::BrowserInformation>,
    pub email: Option<common_utils::pii::Email>,
}

impl TryFrom<&types::PreAuthNRouterData> for ModularAuthenticationPreAuthRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &types::PreAuthNRouterData) -> Result<Self, Self::Error> {
        let payment_method_data = api_models::payments::PaymentMethodData::Card(
            api_models::payments::Card {
                card_number: item.request.card.card_number.clone(),
                card_exp_month: item.request.card.card_exp_month.clone(),
                card_exp_year: item.request.card.card_exp_year.clone(),
                card_cvc: item.request.card.card_cvc.clone(),
                card_holder_name: None,
                card_issuer: None,
                card_network: None,
                card_type: None,
                card_issuing_country: None,
                card_issuing_country_code: None,
                bank_code: None,
                nick_name: None,
            }
        );

        Ok(Self {
            payment_method_data,
            payment_method: item.payment_method,
            payment_method_type: None, // Will be filled by router if needed
            client_secret: None,
            billing: item.address.get_payment_method_billing().cloned().map(Into::into),
            shipping: item.address.get_shipping().cloned().map(Into::into),
            browser_information: item.request.browser_info.clone().map(Into::into),
            email: item.request.email.clone(),
        })
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ModularAuthenticationPreAuthResponse {
    pub authentication_id: id_type::AuthenticationId,
    pub next_action: api_authentication::NextAction,
    pub status: common_enums::AuthenticationStatus,
    pub eligibility_response_params: Option<api_authentication::EligibilityResponseParams>,
    pub connector_metadata: Option<serde_json::Value>,
    pub profile_id: id_type::ProfileId,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub authentication_connector: Option<common_enums::AuthenticationConnectors>,
    pub billing: Option<api_models::payments::Address>,
    pub shipping: Option<api_models::payments::Address>,
    pub browser_information: Option<api_models::payments::BrowserInformation>,
    pub email: Option<common_utils::pii::Email>,
    pub acquirer_details: Option<api_authentication::AcquirerDetails>,
}

impl<F> TryFrom<ResponseRouterData<F, ModularAuthenticationPreAuthResponse, PreAuthNRequestData, AuthenticationResponseData>>
    for types::RouterData<F, PreAuthNRequestData, AuthenticationResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<F, ModularAuthenticationPreAuthResponse, PreAuthNRequestData, AuthenticationResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(item.data)
    }
}

// ----------------------------------------
// PreAuthenticationVersionCall
// ----------------------------------------

#[derive(Debug, serde::Serialize)]
pub struct ModularAuthenticationPreAuthVersionCallRequest {
    pub client_secret: Option<hyperswitch_masking::Secret<String>>,
    pub eligibility_check_data: api_authentication::AuthenticationEligibilityCheckData,
}

impl TryFrom<&types::PreAuthNVersionCallRouterData> for ModularAuthenticationPreAuthVersionCallRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(_item: &types::PreAuthNVersionCallRouterData) -> Result<Self, Self::Error> {
        Err(errors::ConnectorError::NotImplemented("PreAuthNVersionCall".to_string()).into())
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ModularAuthenticationPreAuthVersionCallResponse {
    pub authentication_id: id_type::AuthenticationId,
    pub sdk_next_action: api_authentication::AuthenticationSdkNextAction,
}

impl<F> TryFrom<ResponseRouterData<F, ModularAuthenticationPreAuthVersionCallResponse, PreAuthNRequestData, AuthenticationResponseData>>
    for types::RouterData<F, PreAuthNRequestData, AuthenticationResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<F, ModularAuthenticationPreAuthVersionCallResponse, PreAuthNRequestData, AuthenticationResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(item.data)
    }
}

// ----------------------------------------
// Authentication
// ----------------------------------------

#[derive(Debug, serde::Serialize)]
pub struct ModularAuthenticationAuthenticationRequest {
    pub authentication_id: id_type::AuthenticationId,
    pub client_secret: Option<hyperswitch_masking::Secret<String>>,
    pub sdk_information: Option<api_models::payments::SdkInformation>,
    pub device_channel: api_models::payments::DeviceChannel,
    pub threeds_method_comp_ind: api_models::payments::ThreeDsCompletionIndicator,
    pub email: Option<common_utils::pii::Email>,
    pub billing: Option<api_models::payments::Address>,
    pub shipping: Option<api_models::payments::Address>,
    pub amount: Option<common_utils::types::MinorUnit>,
    pub currency: Option<common_enums::Currency>,
    pub browser_information: Option<api_models::payments::BrowserInformation>,
}

impl TryFrom<&types::ConnectorAuthenticationRouterData> for ModularAuthenticationAuthenticationRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &types::ConnectorAuthenticationRouterData) -> Result<Self, Self::Error> {
        let authentication_id = item.authentication_id.clone().ok_or(errors::ConnectorError::MissingConnectorAuthenticationID)?;
        Ok(Self {
            authentication_id,
            client_secret: None,
            sdk_information: item.request.sdk_information.clone(),
            device_channel: item.request.device_channel.clone(),
            threeds_method_comp_ind: item.request.threeds_method_comp_ind.clone(),
            email: item.request.email.clone(),
            billing: Some(item.request.billing_address.clone().into()),
            shipping: item.request.shipping_address.clone().map(Into::into),
            amount: item.request.amount.map(common_utils::types::MinorUnit::new),
            currency: item.request.currency,
            browser_information: item.request.browser_details.clone().map(Into::into),
        })
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ModularAuthenticationAuthenticationResponse {
    pub transaction_status: Option<common_enums::TransactionStatus>,
    pub acs_url: Option<String>,
    pub challenge_request: Option<String>,
    pub acs_reference_number: Option<String>,
    pub acs_trans_id: Option<String>,
    pub three_ds_server_transaction_id: Option<String>,
    pub acs_signed_content: Option<String>,
    pub three_ds_requestor_url: String,
    pub three_ds_requestor_app_url: Option<String>,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub authentication_value: Option<hyperswitch_masking::Secret<String>>,
    pub status: common_enums::AuthenticationStatus,
    pub authentication_connector: Option<common_enums::AuthenticationConnectors>,
    pub authentication_id: id_type::AuthenticationId,
    pub eci: Option<String>,
    pub acquirer_details: Option<api_authentication::AcquirerDetails>,
}

impl<F> TryFrom<ResponseRouterData<F, ModularAuthenticationAuthenticationResponse, ConnectorAuthenticationRequestData, AuthenticationResponseData>>
    for types::RouterData<F, ConnectorAuthenticationRequestData, AuthenticationResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<F, ModularAuthenticationAuthenticationResponse, ConnectorAuthenticationRequestData, AuthenticationResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(item.data)
    }
}

// ----------------------------------------
// PostAuthentication
// ----------------------------------------

#[derive(Debug, serde::Serialize)]
pub struct ModularAuthenticationPostAuthRequest {
    pub client_secret: Option<hyperswitch_masking::Secret<String>>,
    pub payment_method_details: Option<api_authentication::PostAuthenticationRequestPaymentMethodData>,
    pub authentication_id: id_type::AuthenticationId,
    pub threeds_server_transaction_id: Option<String>,
}

impl TryFrom<&types::ConnectorPostAuthenticationRouterData> for ModularAuthenticationPostAuthRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &types::ConnectorPostAuthenticationRouterData) -> Result<Self, Self::Error> {
        let authentication_id = item.authentication_id.clone().ok_or(errors::ConnectorError::MissingConnectorAuthenticationID)?;
        Ok(Self {
            client_secret: None,
            payment_method_details: None,
            authentication_id,
            threeds_server_transaction_id: Some(item.request.threeds_server_transaction_id.clone()),
        })
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ModularAuthenticationPostAuthResponse {
    pub authentication_id: id_type::AuthenticationId,
    pub merchant_id: id_type::MerchantId,
    pub status: common_enums::AuthenticationStatus,
    pub client_secret: Option<hyperswitch_masking::Secret<String>>,
    pub amount: common_utils::types::MinorUnit,
    pub currency: common_enums::Currency,
    pub authentication_connector: Option<common_enums::AuthenticationConnectors>,
    pub force_3ds_challenge: Option<bool>,
    pub return_url: Option<String>,
    pub created_at: time::PrimitiveDateTime,
    pub profile_id: id_type::ProfileId,
    pub psd2_sca_exemption_type: Option<common_enums::ScaExemptionType>,
    pub acquirer_details: Option<api_authentication::AcquirerDetails>,
    pub threeds_server_transaction_id: Option<String>,
    pub maximum_supported_3ds_version: Option<common_utils::types::SemanticVersion>,
    pub connector_authentication_id: Option<String>,
    pub three_ds_method_data: Option<String>,
    pub three_ds_method_url: Option<String>,
    pub message_version: Option<common_utils::types::SemanticVersion>,
    pub connector_metadata: Option<serde_json::Value>,
    pub directory_server_id: Option<String>,
    pub payment_method_data: Option<serde_json::Value>,
    pub vault_token_data: Option<serde_json::Value>,
    pub authentication_details: Option<serde_json::Value>,
    pub billing: Option<api_models::payments::Address>,
    pub shipping: Option<api_models::payments::Address>,
    pub browser_information: Option<api_models::payments::BrowserInformation>,
    pub email: Option<common_utils::pii::Email>,
    pub transaction_status: Option<common_enums::TransactionStatus>,
    pub acs_url: Option<String>,
    pub challenge_request: Option<String>,
    pub acs_reference_number: Option<String>,
    pub acs_trans_id: Option<String>,
    pub acs_signed_content: Option<String>,
    pub three_ds_requestor_url: Option<String>,
    pub three_ds_requestor_app_url: Option<String>,
    pub eci: Option<String>,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub profile_acquirer_id: Option<id_type::ProfileAcquirerId>,
}

impl<F> TryFrom<ResponseRouterData<F, ModularAuthenticationPostAuthResponse, ConnectorPostAuthenticationRequestData, AuthenticationResponseData>>
    for types::RouterData<F, ConnectorPostAuthenticationRequestData, AuthenticationResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<F, ModularAuthenticationPostAuthResponse, ConnectorPostAuthenticationRequestData, AuthenticationResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(item.data)
    }
}

// ----------------------------------------
// Error
// ----------------------------------------

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ModularAuthenticationErrorResponse {
    pub error_code: String,
    pub error_message: String,
    pub reason: Option<String>,
}
