use std::str::FromStr;

use api_models::authentication as api_authentication;
use hyperswitch_domain_models::router_request_types::{
    authentication::{
        ConnectorAuthenticationRequestData, ConnectorPostAuthenticationRequestData,
        PreAuthNRequestData,
    },
    unified_authentication_service::UasAuthenticationResponseData,
};
use hyperswitch_interfaces::errors;

use crate::types::{self, ResponseRouterData};
use common_utils::types::MinorUnit;

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

impl TryFrom<&hyperswitch_domain_models::router_data::ConnectorAuthType>
    for ModularAuthenticationAuthType
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        auth_type: &hyperswitch_domain_models::router_data::ConnectorAuthType,
    ) -> Result<Self, Self::Error> {
        match auth_type {
            hyperswitch_domain_models::router_data::ConnectorAuthType::HeaderKey { api_key } => {
                Ok(Self {
                    api_key: api_key.to_owned(),
                })
            }
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

// ----------------------------------------
// Authentication Create
// ----------------------------------------

pub fn construct_authentication_create_request(
    item: &types::RouterData<hyperswitch_domain_models::router_flow_types::authentication::AuthenticationCreate, hyperswitch_domain_models::router_request_types::unified_authentication_service::AuthenticationCreateRequestData, UasAuthenticationResponseData>,
) -> Result<
    api_models::authentication::AuthenticationCreateRequest,
    error_stack::Report<errors::ConnectorError>,
> {
    let amount = item.request.amount;
    let currency = item.request.currency;
    let acquirer_details = item.request.acquirer_details.clone();
    let connector = common_enums::AuthenticationConnectors::from_str(
        item.request.authentication_connector.as_str(),
    )
    .map_err(|_| errors::ConnectorError::InvalidConnectorName)?;
    Ok(api_models::authentication::AuthenticationCreateRequest {
        authentication_id: None,
        profile_id: item.request.profile_id.clone(),
        amount,
        currency,
        return_url: item.request.return_url.clone(),
        authentication_connector: Some(connector),
        force_3ds_challenge: item.request.force_3ds_challenge,
        psd2_sca_exemption_type: item.request.psd2_sca_exemption_type,
        profile_acquirer_id: item.request.profile_acquirer_id.clone(),
        acquirer_details,
        customer_details: item.request.customer_details.clone(),
    })
}

impl<F, T>
    TryFrom<
        ResponseRouterData<
            F,
            api_authentication::AuthenticationResponse,
            T,
            UasAuthenticationResponseData,
        >,
    > for types::RouterData<F, T, UasAuthenticationResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            api_authentication::AuthenticationResponse,
            T,
            UasAuthenticationResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(
                UasAuthenticationResponseData::AuthenticationCreateResponse {
                    connector_authentication_id: item
                        .response
                        .authentication_id
                        .get_string_repr()
                        .to_string(),
                },
            ),
            ..item.data
        })
    }
}

// ----------------------------------------
// PreAuthentication
// ----------------------------------------

pub fn construct_pre_auth_request(
    item: &types::PreAuthNRouterData,
) -> Result<
    api_authentication::AuthenticationEligibilityRequest,
    error_stack::Report<errors::ConnectorError>,
> {
    let payment_method_data =
        api_models::payments::PaymentMethodData::Card(api_models::payments::Card {
            card_number: item.request.card.card_number.clone(),
            card_exp_month: item.request.card.card_exp_month.clone(),
            card_exp_year: item.request.card.card_exp_year.clone(),
            card_cvc: item.request.card.card_cvc.clone(),
            card_holder_name: item.request.card.card_holder_name.clone(),
            card_issuer: item.request.card.card_issuer.clone(),
            card_network: item.request.card.card_network.clone(),
            card_type: item.request.card.card_type.clone(),
            card_issuing_country: item.request.card.card_issuing_country.clone(),
            card_issuing_country_code: item.request.card.card_issuing_country_code.clone(),
            bank_code: item.request.card.bank_code.clone(),
            nick_name: None,
        });

    Ok(api_authentication::AuthenticationEligibilityRequest {
        payment_method_data,
        payment_method: item.payment_method,
        payment_method_type: None, // Will be filled by router if needed
        client_secret: None,
        profile_id: None,
        billing: item
            .address
            .get_payment_method_billing()
            .cloned()
            .map(Into::into),
        shipping: item.address.get_shipping().cloned().map(Into::into),
        browser_information: item.request.browser_info.clone().map(Into::into),
        email: item.request.email.clone(),
    })
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            api_authentication::AuthenticationEligibilityResponse,
            PreAuthNRequestData,
            UasAuthenticationResponseData,
        >,
    > for types::RouterData<F, PreAuthNRequestData, UasAuthenticationResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<
            F,
            api_authentication::AuthenticationEligibilityResponse,
            PreAuthNRequestData,
            UasAuthenticationResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let eligibility_params =
            item.response
                .eligibility_response_params
                .map(|params| match params {
                    api_authentication::EligibilityResponseParams::ThreeDsData(three_ds) => {
                        api_authentication::ThreeDsData {
                            maximum_supported_3ds_version: three_ds.maximum_supported_3ds_version,
                            three_ds_method_data: three_ds.three_ds_method_data,
                            three_ds_method_url: three_ds.three_ds_method_url,
                            message_version: three_ds.message_version,
                            directory_server_id: three_ds.directory_server_id,
                            three_ds_server_transaction_id: three_ds.three_ds_server_transaction_id,
                            connector_authentication_id: Some(
                                item.response
                                    .authentication_id
                                    .get_string_repr()
                                    .to_string(),
                            ),
                        }
                    }
                });
        Ok(Self {
            response: Ok(UasAuthenticationResponseData::PreAuthNResponse {
                threeds_server_transaction_id: item
                    .response
                    .authentication_id
                    .get_string_repr()
                    .to_string(),
                maximum_supported_3ds_version: eligibility_params
                    .as_ref()
                    .and_then(|params| params.maximum_supported_3ds_version.clone())
                    .ok_or(errors::ConnectorError::MissingRequiredField {
                        field_name: "maximum_supported_3ds_version",
                    })?,
                connector_authentication_id: eligibility_params
                    .as_ref()
                    .and_then(|params| params.connector_authentication_id.clone())
                    .ok_or(errors::ConnectorError::MissingRequiredField {
                        field_name: "connector_authentication_id",
                    })?,
                three_ds_method_data: eligibility_params
                    .as_ref()
                    .and_then(|params| params.three_ds_method_data.clone()),
                three_ds_method_url: eligibility_params
                    .as_ref()
                    .and_then(|params| params.three_ds_method_url.as_ref().map(|u| u.to_string())),
                message_version: eligibility_params
                    .as_ref()
                    .and_then(|params| params.message_version.clone())
                    .ok_or(errors::ConnectorError::MissingRequiredField {
                        field_name: "message_version",
                    })?,
                connector_metadata: item.response.connector_metadata,
                directory_server_id: eligibility_params
                    .as_ref()
                    .and_then(|params| params.directory_server_id.clone()),
                scheme_id: None,
            }),
            ..item.data
        })
    }
}

// ----------------------------------------
// Authentication
// ----------------------------------------

pub fn construct_authentication_request(
    item: &types::ConnectorAuthenticationRouterData,
) -> Result<
    api_authentication::AuthenticationAuthenticateRequest,
    error_stack::Report<errors::ConnectorError>,
> {
    let authentication_id = item
        .authentication_id
        .clone()
        .ok_or(errors::ConnectorError::MissingConnectorAuthenticationID)?;
    Ok(api_authentication::AuthenticationAuthenticateRequest {
        authentication_id,
        client_secret: None,
        sdk_information: item.request.sdk_information.clone(),
        device_channel: item.request.device_channel.clone(),
        threeds_method_comp_ind: item.request.threeds_method_comp_ind.clone(),
    })
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            api_authentication::AuthenticationAuthenticateResponse,
            ConnectorAuthenticationRequestData,
            UasAuthenticationResponseData,
        >,
    > for types::RouterData<F, ConnectorAuthenticationRequestData, UasAuthenticationResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<
            F,
            api_authentication::AuthenticationAuthenticateResponse,
            ConnectorAuthenticationRequestData,
            UasAuthenticationResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(item.data)
    }
}

// ----------------------------------------
// PostAuthentication
// ----------------------------------------

pub fn construct_post_auth_request(
    item: &types::ConnectorPostAuthenticationRouterData,
) -> Result<
    api_authentication::AuthenticationSyncRequest,
    error_stack::Report<errors::ConnectorError>,
> {
    let authentication_id = item
        .authentication_id
        .clone()
        .ok_or(errors::ConnectorError::MissingConnectorAuthenticationID)?;
    Ok(api_authentication::AuthenticationSyncRequest {
        client_secret: None,
        payment_method_details: None,
        authentication_id,
    })
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            api_authentication::AuthenticationSyncResponse,
            ConnectorPostAuthenticationRequestData,
            UasAuthenticationResponseData,
        >,
    >
    for types::RouterData<F, ConnectorPostAuthenticationRequestData, UasAuthenticationResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<
            F,
            api_authentication::AuthenticationSyncResponse,
            ConnectorPostAuthenticationRequestData,
            UasAuthenticationResponseData,
        >,
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
