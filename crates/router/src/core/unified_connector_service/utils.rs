use api_models::admin::ConnectorAuthType;
use common_enums::{enums::PaymentMethod, AttemptStatus, AuthenticationType};
use common_utils::{errors::CustomResult, ext_traits::ValueExt};
use error_stack::ResultExt;
use hyperswitch_connectors::utils::CardData;
use hyperswitch_domain_models::{
    errors::api_error_response::ApiErrorResponse,
    merchant_context::MerchantContext,
    router_data::{ErrorResponse, RouterData},
    router_flow_types::payments::Authorize,
    router_request_types::PaymentsAuthorizeData,
    router_response_types::PaymentsResponseData,
};
use hyperswitch_interfaces::errors::ConnectorError;
use masking::{ExposeInterface, PeekInterface};
use rand::Rng;
use rust_grpc_client::payments::{
    self as payments_grpc, payment_service_client::PaymentServiceClient,
};
use tonic::metadata::MetadataMap;

use crate::{
    consts::UCS_ROLLOUT_PERCENT_CONFIG_PREFIX,
    core::{
        errors::RouterResult, payments::helpers::MerchantConnectorAccountType, utils::get_flow_name,
    },
    routes::SessionState,
    types::transformers::ForeignTryFrom,
};

pub async fn should_call_unified_connector_service<F: Clone, T>(
    state: &SessionState,
    merchant_context: &MerchantContext,
    router_data: &RouterData<F, T, PaymentsResponseData>,
) -> RouterResult<Option<PaymentServiceClient<tonic::transport::Channel>>> {
    let merchant_id = merchant_context
        .get_merchant_account()
        .get_id()
        .get_string_repr();

    let connector_name = router_data.connector.clone();

    let payment_method = router_data.payment_method.to_string();
    let flow_name = get_flow_name::<F>()?;

    let config_key = format!(
        "{}_{}_{}_{}_{}",
        UCS_ROLLOUT_PERCENT_CONFIG_PREFIX, merchant_id, connector_name, payment_method, flow_name
    );

    let db = state.store.as_ref();

    match db.find_config_by_key(&config_key).await {
        Ok(rollout_config) => match rollout_config.config.parse() {
            Ok(rollout_percent) => {
                let random_value: f64 = rand::thread_rng().gen_range(0.0..=1.0);
                if random_value < rollout_percent {
                    match PaymentServiceClient::connect(
                        state.conf.unified_connector_service.base_url.clone(),
                    )
                    .await
                    {
                        Ok(client) => Ok(Some(client)),
                        Err(_) => Ok(None),
                    }
                } else {
                    Ok(None)
                }
            }
            Err(_) => Ok(None),
        },
        Err(_) => Ok(None),
    }
}

pub fn construct_ucs_request_metadata(
    metadata: &mut MetadataMap,
    merchant_connector_account: MerchantConnectorAccountType,
) -> RouterResult<()> {
    let auth_type: ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while parsing value for ConnectorAuthType")?;

    let connector_name = {
        #[cfg(feature = "v1")]
        {
            merchant_connector_account
                .get_connector_name()
                .ok_or_else(|| ApiErrorResponse::InternalServerError)
                .attach_printable("Missing connector name")?
        }

        #[cfg(feature = "v2")]
        {
            merchant_connector_account
                .get_connector_name()
                .map(|connector| connector.to_string())
                .ok_or_else(|| ApiErrorResponse::InternalServerError)
                .attach_printable("Missing connector name")?
        }
    };

    let parsed_connector_name = connector_name
        .parse()
        .map_err(|_| ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| format!("Failed to parse connector name: {connector_name}"))?;

    metadata.append("x-connector", parsed_connector_name);

    match &auth_type {
        ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } => {
            metadata.append(
                "x-auth",
                "signature-key"
                    .parse()
                    .map_err(|_| ApiErrorResponse::InternalServerError)?,
            );
            metadata.append(
                "x-api-key",
                api_key
                    .peek()
                    .parse()
                    .map_err(|_| ApiErrorResponse::InternalServerError)?,
            );
            metadata.append(
                "x-key1",
                key1.peek()
                    .parse()
                    .map_err(|_| ApiErrorResponse::InternalServerError)?,
            );
            metadata.append(
                "x-api-secret",
                api_secret
                    .peek()
                    .parse()
                    .map_err(|_| ApiErrorResponse::InternalServerError)?,
            );
        }
        ConnectorAuthType::BodyKey { api_key, key1 } => {
            metadata.append(
                "x-auth",
                "body-key"
                    .parse()
                    .map_err(|_| ApiErrorResponse::InternalServerError)?,
            );
            metadata.append(
                "x-api-key",
                api_key
                    .peek()
                    .parse()
                    .map_err(|_| ApiErrorResponse::InternalServerError)?,
            );
            metadata.append(
                "x-key1",
                key1.peek()
                    .parse()
                    .map_err(|_| ApiErrorResponse::InternalServerError)?,
            );
        }
        ConnectorAuthType::HeaderKey { api_key } => {
            metadata.append(
                "x-auth",
                "header-key"
                    .parse()
                    .map_err(|_| ApiErrorResponse::InternalServerError)?,
            );
            metadata.append(
                "x-api-key",
                api_key
                    .peek()
                    .parse()
                    .map_err(|_| ApiErrorResponse::InternalServerError)?,
            );
        }
        _ => {
            return Err(ApiErrorResponse::InternalServerError)
                .attach_printable("Unsupported ConnectorAuthType for header injection")?;
        }
    }

    Ok(())
}

impl ForeignTryFrom<&RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>>
    for payments_grpc::PaymentsAuthorizeRequest
{
    type Error = error_stack::Report<ConnectorError>;

    fn foreign_try_from(
        router_data: &RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let currency = payments_grpc::Currency::foreign_try_from(router_data.request.currency)?;

        let payment_method =
            payments_grpc::PaymentMethod::foreign_try_from(router_data.payment_method)?;

        let payment_method_data = payments_grpc::PaymentMethodData::foreign_try_from(
            router_data.request.payment_method_data.clone(),
        )?;

        let address = payments_grpc::PaymentAddress::foreign_try_from(router_data.address.clone())?;

        let auth_type = payments_grpc::AuthenticationType::foreign_try_from(router_data.auth_type)?;

        let browser_info = router_data
            .request
            .browser_info
            .clone()
            .map(payments_grpc::BrowserInformation::foreign_try_from)
            .transpose()?;

        Ok(Self {
            amount: router_data.request.amount,
            currency: currency.into(),
            payment_method: payment_method.into(),
            payment_method_data: Some(payment_method_data),
            connector_customer: router_data
                .request
                .customer_id
                .as_ref()
                .map(|id| id.get_string_repr().to_string()),
            return_url: router_data.request.router_return_url.clone(),
            address: Some(address),
            auth_type: auth_type.into(),
            connector_request_reference_id: router_data.connector_request_reference_id.clone(),
            enrolled_for_3ds: router_data.request.enrolled_for_3ds,
            request_incremental_authorization: router_data
                .request
                .request_incremental_authorization,
            minor_amount: router_data.request.amount,
            email: router_data
                .request
                .email
                .clone()
                .map(|e| e.expose().peek().clone()),
            browser_info,
            ..Default::default()
        })
    }
}

impl ForeignTryFrom<common_enums::Currency> for payments_grpc::Currency {
    type Error = error_stack::Report<ConnectorError>;

    fn foreign_try_from(currency: common_enums::Currency) -> Result<Self, Self::Error> {
        Self::from_str_name(&currency.to_string()).ok_or_else(|| {
            ConnectorError::RequestEncodingFailedWithReason("Failed to parse currency".to_string())
                .into()
        })
    }
}

impl ForeignTryFrom<PaymentMethod> for payments_grpc::PaymentMethod {
    type Error = error_stack::Report<ConnectorError>;

    fn foreign_try_from(payment_method: PaymentMethod) -> Result<Self, Self::Error> {
        match payment_method {
            PaymentMethod::Card => Ok(Self::Card),
            _ => Err(ConnectorError::NotImplemented(format!(
                "Unimplemented payment method: {:?}",
                payment_method
            ))
            .into()),
        }
    }
}

impl ForeignTryFrom<hyperswitch_domain_models::payment_method_data::PaymentMethodData>
    for payments_grpc::PaymentMethodData
{
    type Error = error_stack::Report<ConnectorError>;

    fn foreign_try_from(
        payment_method_data: hyperswitch_domain_models::payment_method_data::PaymentMethodData,
    ) -> Result<Self, Self::Error> {
        match payment_method_data {
            hyperswitch_domain_models::payment_method_data::PaymentMethodData::Card(card) => {
                Ok(Self {
                    data: Some(payments_grpc::payment_method_data::Data::Card(
                        payments_grpc::Card {
                            card_number: card.card_number.get_card_no(),
                            card_exp_month: card
                                .get_card_expiry_month_2_digit()
                                .attach_printable(
                                    "Failed to extract 2-digit expiry month from card",
                                )?
                                .peek()
                                .to_string(),
                            card_exp_year: card.get_expiry_year_4_digit().peek().to_string(),
                            card_cvc: card.card_cvc.peek().to_string(),
                            ..Default::default()
                        },
                    )),
                })
            }
            _ => Err(ConnectorError::NotImplemented(format!(
                "Unimplemented payment method: {:?}",
                payment_method_data
            ))
            .into()),
        }
    }
}

impl ForeignTryFrom<hyperswitch_domain_models::payment_address::PaymentAddress>
    for payments_grpc::PaymentAddress
{
    type Error = error_stack::Report<ConnectorError>;

    fn foreign_try_from(
        payment_address: hyperswitch_domain_models::payment_address::PaymentAddress,
    ) -> Result<Self, Self::Error> {
        let shipping =
            if let Some(address) = payment_address.get_shipping() {
                let country = address
                    .address
                    .as_ref()
                    .and_then(|details| {
                        details.country.as_ref().and_then(|c| {
                            payments_grpc::CountryAlpha2::from_str_name(&c.to_string())
                        })
                    })
                    .ok_or_else(|| {
                        ConnectorError::RequestEncodingFailedWithReason(
                            "Invalid country code".to_string(),
                        )
                    })
                    .attach_printable("Invalid country code")?
                    .into();

                Some(payments_grpc::Address {
                    address: address.address.as_ref().map(|details| {
                        payments_grpc::AddressDetails {
                            city: details.city.clone(),
                            country: Some(country),
                            line1: details.line1.as_ref().map(|l| l.peek().to_string()),
                            line2: details.line2.as_ref().map(|l| l.peek().to_string()),
                            line3: details.line3.as_ref().map(|l| l.peek().to_string()),
                            zip: details.zip.as_ref().map(|z| z.peek().to_string()),
                            state: details.state.as_ref().map(|s| s.peek().to_string()),
                            first_name: details.first_name.as_ref().map(|f| f.peek().to_string()),
                            last_name: details.last_name.as_ref().map(|l| l.peek().to_string()),
                        }
                    }),
                    phone: address
                        .phone
                        .as_ref()
                        .map(|phone| payments_grpc::PhoneDetails {
                            number: phone.number.as_ref().map(|n| n.peek().to_string()),
                            country_code: phone.country_code.clone(),
                        }),
                    email: address.email.as_ref().map(|e| e.peek().to_string()),
                })
            } else {
                None
            };

        let billing =
            if let Some(address) = payment_address.get_payment_billing() {
                let country = address
                    .address
                    .as_ref()
                    .and_then(|details| {
                        details.country.as_ref().and_then(|c| {
                            payments_grpc::CountryAlpha2::from_str_name(&c.to_string())
                        })
                    })
                    .ok_or_else(|| {
                        ConnectorError::RequestEncodingFailedWithReason(
                            "Invalid country code".to_string(),
                        )
                    })
                    .attach_printable("Invalid country code")?
                    .into();

                Some(payments_grpc::Address {
                    address: address.address.as_ref().map(|details| {
                        payments_grpc::AddressDetails {
                            city: details.city.clone(),
                            country: Some(country),
                            line1: details.line1.as_ref().map(|l| l.peek().to_string()),
                            line2: details.line2.as_ref().map(|l| l.peek().to_string()),
                            line3: details.line3.as_ref().map(|l| l.peek().to_string()),
                            zip: details.zip.as_ref().map(|z| z.peek().to_string()),
                            state: details.state.as_ref().map(|s| s.peek().to_string()),
                            first_name: details.first_name.as_ref().map(|f| f.peek().to_string()),
                            last_name: details.last_name.as_ref().map(|l| l.peek().to_string()),
                        }
                    }),
                    phone: address
                        .phone
                        .as_ref()
                        .map(|phone| payments_grpc::PhoneDetails {
                            number: phone.number.as_ref().map(|n| n.peek().to_string()),
                            country_code: phone.country_code.clone(),
                        }),
                    email: address.email.as_ref().map(|e| e.peek().to_string()),
                })
            } else {
                None
            };

        let unified_payment_method_billing =
            if let Some(address) = payment_address.get_payment_method_billing() {
                let country = address
                    .address
                    .as_ref()
                    .and_then(|details| {
                        details.country.as_ref().and_then(|c| {
                            payments_grpc::CountryAlpha2::from_str_name(&c.to_string())
                        })
                    })
                    .ok_or_else(|| {
                        ConnectorError::RequestEncodingFailedWithReason(
                            "Invalid country code".to_string(),
                        )
                    })
                    .attach_printable("Invalid country code")?
                    .into();

                Some(payments_grpc::Address {
                    address: address.address.as_ref().map(|details| {
                        payments_grpc::AddressDetails {
                            city: details.city.clone(),
                            country: Some(country),
                            line1: details.line1.as_ref().map(|l| l.peek().to_string()),
                            line2: details.line2.as_ref().map(|l| l.peek().to_string()),
                            line3: details.line3.as_ref().map(|l| l.peek().to_string()),
                            zip: details.zip.as_ref().map(|z| z.peek().to_string()),
                            state: details.state.as_ref().map(|s| s.peek().to_string()),
                            first_name: details.first_name.as_ref().map(|f| f.peek().to_string()),
                            last_name: details.last_name.as_ref().map(|l| l.peek().to_string()),
                        }
                    }),
                    phone: address
                        .phone
                        .as_ref()
                        .map(|phone| payments_grpc::PhoneDetails {
                            number: phone.number.as_ref().map(|n| n.peek().to_string()),
                            country_code: phone.country_code.clone(),
                        }),
                    email: address.email.as_ref().map(|e| e.peek().to_string()),
                })
            } else {
                None
            };

        Ok(Self {
            shipping: shipping.clone(),
            billing: billing.clone(),
            unified_payment_method_billing: unified_payment_method_billing.clone(),
            payment_method_billing: unified_payment_method_billing.clone(),
        })
    }
}

impl ForeignTryFrom<AuthenticationType> for payments_grpc::AuthenticationType {
    type Error = error_stack::Report<ConnectorError>;

    fn foreign_try_from(auth_type: AuthenticationType) -> Result<Self, Self::Error> {
        match auth_type {
            AuthenticationType::ThreeDs => Ok(Self::ThreeDs),
            AuthenticationType::NoThreeDs => Ok(Self::NoThreeDs),
        }
    }
}

impl ForeignTryFrom<hyperswitch_domain_models::router_request_types::BrowserInformation>
    for payments_grpc::BrowserInformation
{
    type Error = error_stack::Report<ConnectorError>;

    fn foreign_try_from(
        browser_info: hyperswitch_domain_models::router_request_types::BrowserInformation,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            color_depth: browser_info.color_depth.map(|v| v.into()),
            java_enabled: browser_info.java_enabled,
            java_script_enabled: browser_info.java_script_enabled,
            language: browser_info.language,
            screen_height: browser_info.screen_height,
            screen_width: browser_info.screen_width,
            time_zone: browser_info.time_zone,
            ip_address: browser_info.ip_address.map(|ip| ip.to_string()),
            accept_header: browser_info.accept_header,
            user_agent: browser_info.user_agent,
            os_type: browser_info.os_type,
            os_version: browser_info.os_version,
            device_model: browser_info.device_model,
            accept_language: browser_info.accept_language,
        })
    }
}

impl ForeignTryFrom<payments_grpc::AttemptStatus> for AttemptStatus {
    type Error = error_stack::Report<ConnectorError>;

    fn foreign_try_from(grpc_status: payments_grpc::AttemptStatus) -> Result<Self, Self::Error> {
        match grpc_status {
            payments_grpc::AttemptStatus::Started => Ok(Self::Started),
            payments_grpc::AttemptStatus::AuthenticationFailed => Ok(Self::AuthenticationFailed),
            payments_grpc::AttemptStatus::RouterDeclined => Ok(Self::RouterDeclined),
            payments_grpc::AttemptStatus::AuthenticationPending => Ok(Self::AuthenticationPending),
            payments_grpc::AttemptStatus::AuthenticationSuccessful => {
                Ok(Self::AuthenticationSuccessful)
            }
            payments_grpc::AttemptStatus::Authorized => Ok(Self::Authorized),
            payments_grpc::AttemptStatus::AuthorizationFailed => Ok(Self::AuthorizationFailed),
            payments_grpc::AttemptStatus::Charged => Ok(Self::Charged),
            payments_grpc::AttemptStatus::Authorizing => Ok(Self::Authorizing),
            payments_grpc::AttemptStatus::CodInitiated => Ok(Self::CodInitiated),
            payments_grpc::AttemptStatus::Voided => Ok(Self::Voided),
            payments_grpc::AttemptStatus::VoidInitiated => Ok(Self::VoidInitiated),
            payments_grpc::AttemptStatus::CaptureInitiated => Ok(Self::CaptureInitiated),
            payments_grpc::AttemptStatus::CaptureFailed => Ok(Self::CaptureFailed),
            payments_grpc::AttemptStatus::VoidFailed => Ok(Self::VoidFailed),
            payments_grpc::AttemptStatus::AutoRefunded => Ok(Self::AutoRefunded),
            payments_grpc::AttemptStatus::PartialCharged => Ok(Self::PartialCharged),
            payments_grpc::AttemptStatus::PartialChargedAndChargeable => {
                Ok(Self::PartialChargedAndChargeable)
            }
            payments_grpc::AttemptStatus::Unresolved => Ok(Self::Unresolved),
            payments_grpc::AttemptStatus::Pending => Ok(Self::Pending),
            payments_grpc::AttemptStatus::Failure => Ok(Self::Failure),
            payments_grpc::AttemptStatus::PaymentMethodAwaited => Ok(Self::PaymentMethodAwaited),
            payments_grpc::AttemptStatus::ConfirmationAwaited => Ok(Self::ConfirmationAwaited),
            payments_grpc::AttemptStatus::DeviceDataCollectionPending => {
                Ok(Self::DeviceDataCollectionPending)
            }
        }
    }
}

pub fn handle_unified_connector_service_response(
    response: payments_grpc::PaymentsAuthorizeResponse,
    router_data: &mut RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
) -> CustomResult<(), ConnectorError> {
    let status = AttemptStatus::foreign_try_from(response.status())?;

    let router_data_response = match status {
        AttemptStatus::Charged |
        AttemptStatus::Authorized |
        AttemptStatus::AuthenticationPending |
        AttemptStatus::DeviceDataCollectionPending => Ok(PaymentsResponseData::TransactionResponse {
            resource_id: hyperswitch_domain_models::router_request_types::ResponseId::ConnectorTransactionId(response.connector_response_reference_id().to_owned()),
            redirection_data: Box::new(None),
            mandate_reference: Box::new(None),
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: Some(response.connector_response_reference_id().to_owned()),
            incremental_authorization_allowed: None,
            charges: None,
        }),
        _ => Err(ErrorResponse {
            code: response.error_code().to_owned(),
            message: response.error_message().to_owned(),
            reason: Some(response.error_message().to_owned()),
            status_code: 500,
            attempt_status: Some(status),
            connector_transaction_id: Some(response.connector_response_reference_id().to_owned()),
            network_decline_code: None,
            network_advice_code: None,
            network_error_message: None,
        })
    };
    router_data.status = status;
    router_data.response = router_data_response;

    Ok(())
}
