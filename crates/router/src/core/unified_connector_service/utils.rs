use api_models::admin::ConnectorAuthType;
use common_enums::{enums::PaymentMethod, AttemptStatus, AuthenticationType};
use common_utils::ext_traits::ValueExt;
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
use masking::{ExposeInterface, PeekInterface};
use rand::Rng;
use rust_grpc_client::payments::{
    self as payments_grpc, payment_service_client::PaymentServiceClient,
};
use tonic::metadata::MetadataMap;

use crate::{
    core::{
        errors::RouterResult, payments::helpers::MerchantConnectorAccountType, utils::get_flow_name,
    },
    routes::SessionState,
};

pub async fn should_call_unified_connector_service<F: Clone, T>(
    state: &SessionState,
    merchant_context: &MerchantContext,
    merchant_connector_account: MerchantConnectorAccountType,
    router_data: &RouterData<F, T, PaymentsResponseData>,
) -> RouterResult<Option<PaymentServiceClient<tonic::transport::Channel>>> {
    let merchant_id = merchant_context
        .get_merchant_account()
        .get_id()
        .get_string_repr();

    let connector_name = match merchant_connector_account.get_connector_name() {
        Some(name) => name,
        None => return Ok(None),
    };

    let payment_method = router_data.payment_method.to_string();
    let flow_name = get_flow_name::<F>()?;

    let config_key = format!(
        "{}_{}_{}_{}",
        merchant_id, connector_name, payment_method, flow_name
    );

    let db = state.store.as_ref();

    let rollout_config = match db.find_config_by_key(&config_key).await {
        Ok(config) => config,
        Err(_) => return Ok(None),
    };

    let rollout_percent: f64 = match rollout_config.config.parse() {
        Ok(percent) => percent,
        Err(_) => return Ok(None),
    };
    let random_value: f64 = rand::thread_rng().gen_range(0.0..=1.0);

    if random_value < rollout_percent {
        match PaymentServiceClient::connect(state.conf.unified_connector_service.url.clone()).await
        {
            Ok(client) => Ok(Some(client)),
            Err(_) => Ok(None),
        }
    } else {
        Ok(None)
    }
}

pub fn construct_ucs_authorize_request(
    router_data: &RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
) -> RouterResult<payments_grpc::PaymentsAuthorizeRequest> {
    let currency =
        payments_grpc::Currency::from_str_name(&router_data.request.currency.to_string())
            .ok_or_else(|| ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to parse currency")?;

    let payment_method = construct_ucs_payment_method(router_data.payment_method)?;

    let payment_method_data =
        construct_ucs_payment_method_data(router_data.request.payment_method_data.clone())?;

    let address = construct_ucs_payment_address(router_data.address.clone())?;

    let auth_type = construct_ucs_auth_type(router_data.auth_type)?;

    let browser_info = router_data
        .request
        .browser_info
        .clone()
        .map(construct_ucs_browser_info)
        .transpose()?;

    Ok(payments_grpc::PaymentsAuthorizeRequest {
        amount: router_data.request.amount,
        currency: currency.into(),
        payment_method: payment_method.into(),
        payment_method_data: Some(payment_method_data),
        connector_customer: Some("abcd".to_string()),
        return_url: router_data.request.router_return_url.clone(),
        address: Some(address),
        auth_type: auth_type.into(),
        connector_request_reference_id: router_data.connector_request_reference_id.clone(),
        enrolled_for_3ds: router_data.request.enrolled_for_3ds,
        request_incremental_authorization: router_data.request.request_incremental_authorization,
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

pub fn construct_ucs_payment_method(
    payment_method: PaymentMethod,
) -> RouterResult<payments_grpc::PaymentMethod> {
    match payment_method {
        PaymentMethod::Card => Ok(payments_grpc::PaymentMethod::Card),
        _ => Err(ApiErrorResponse::InternalServerError.into()),
    }
}

pub fn construct_ucs_payment_method_data(
    payment_method_data: hyperswitch_domain_models::payment_method_data::PaymentMethodData,
) -> RouterResult<payments_grpc::PaymentMethodData> {
    match payment_method_data {
        hyperswitch_domain_models::payment_method_data::PaymentMethodData::Card(card) => {
            Ok(payments_grpc::PaymentMethodData {
                data: Some(payments_grpc::payment_method_data::Data::Card(
                    payments_grpc::Card {
                        card_number: card.card_number.get_card_no(),
                        card_exp_month: card
                            .get_card_expiry_month_2_digit()
                            .map_err(|_| ApiErrorResponse::InternalServerError)?
                            .peek()
                            .to_string(),
                        card_exp_year: card.get_expiry_year_4_digit().peek().to_string(),
                        card_cvc: card.card_cvc.peek().to_string(),
                        ..Default::default()
                    },
                )),
            })
        }
        _ => Err(ApiErrorResponse::InternalServerError.into()),
    }
}

pub fn construct_ucs_payment_address(
    payment_address: hyperswitch_domain_models::payment_address::PaymentAddress,
) -> RouterResult<payments_grpc::PaymentAddress> {
    let shipping = if let Some(address) = payment_address.get_shipping() {
        let country = address
            .address
            .as_ref()
            .and_then(|details| {
                details
                    .country
                    .as_ref()
                    .and_then(|c| payments_grpc::CountryAlpha2::from_str_name(&c.to_string()))
            })
            .ok_or_else(|| ApiErrorResponse::InternalServerError)
            .attach_printable("Invalid country code")?
            .into();

        Some(payments_grpc::Address {
            address: address
                .address
                .as_ref()
                .map(|details| payments_grpc::AddressDetails {
                    city: details.city.clone(),
                    country: Some(country),
                    line1: details.line1.as_ref().map(|l| l.peek().to_string()),
                    line2: details.line2.as_ref().map(|l| l.peek().to_string()),
                    line3: details.line3.as_ref().map(|l| l.peek().to_string()),
                    zip: details.zip.as_ref().map(|z| z.peek().to_string()),
                    state: details.state.as_ref().map(|s| s.peek().to_string()),
                    first_name: details.first_name.as_ref().map(|f| f.peek().to_string()),
                    last_name: details.last_name.as_ref().map(|l| l.peek().to_string()),
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

    let billing = if let Some(address) = payment_address.get_payment_billing() {
        let country = address
            .address
            .as_ref()
            .and_then(|details| {
                details
                    .country
                    .as_ref()
                    .and_then(|c| payments_grpc::CountryAlpha2::from_str_name(&c.to_string()))
            })
            .ok_or_else(|| ApiErrorResponse::InternalServerError)
            .attach_printable("Invalid country code")?
            .into();

        Some(payments_grpc::Address {
            address: address
                .address
                .as_ref()
                .map(|details| payments_grpc::AddressDetails {
                    city: details.city.clone(),
                    country: Some(country),
                    line1: details.line1.as_ref().map(|l| l.peek().to_string()),
                    line2: details.line2.as_ref().map(|l| l.peek().to_string()),
                    line3: details.line3.as_ref().map(|l| l.peek().to_string()),
                    zip: details.zip.as_ref().map(|z| z.peek().to_string()),
                    state: details.state.as_ref().map(|s| s.peek().to_string()),
                    first_name: details.first_name.as_ref().map(|f| f.peek().to_string()),
                    last_name: details.last_name.as_ref().map(|l| l.peek().to_string()),
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
                    details
                        .country
                        .as_ref()
                        .and_then(|c| payments_grpc::CountryAlpha2::from_str_name(&c.to_string()))
                })
                .ok_or_else(|| ApiErrorResponse::InternalServerError)
                .attach_printable("Invalid country code")?
                .into();

            Some(payments_grpc::Address {
                address: address
                    .address
                    .as_ref()
                    .map(|details| payments_grpc::AddressDetails {
                        city: details.city.clone(),
                        country: Some(country),
                        line1: details.line1.as_ref().map(|l| l.peek().to_string()),
                        line2: details.line2.as_ref().map(|l| l.peek().to_string()),
                        line3: details.line3.as_ref().map(|l| l.peek().to_string()),
                        zip: details.zip.as_ref().map(|z| z.peek().to_string()),
                        state: details.state.as_ref().map(|s| s.peek().to_string()),
                        first_name: details.first_name.as_ref().map(|f| f.peek().to_string()),
                        last_name: details.last_name.as_ref().map(|l| l.peek().to_string()),
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

    Ok(payments_grpc::PaymentAddress {
        shipping: shipping.clone(),
        billing: billing.clone(),
        unified_payment_method_billing: unified_payment_method_billing.clone(),
        payment_method_billing: unified_payment_method_billing.clone(),
    })
}

pub fn construct_ucs_auth_type(
    auth_type: AuthenticationType,
) -> RouterResult<payments_grpc::AuthenticationType> {
    match auth_type {
        AuthenticationType::ThreeDs => Ok(payments_grpc::AuthenticationType::ThreeDs),
        AuthenticationType::NoThreeDs => Ok(payments_grpc::AuthenticationType::NoThreeDs),
    }
}

pub fn construct_ucs_browser_info(
    browser_info: hyperswitch_domain_models::router_request_types::BrowserInformation,
) -> RouterResult<payments_grpc::BrowserInformation> {
    Ok(payments_grpc::BrowserInformation {
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

pub fn construct_ucs_request_metadata(
    metadata: &mut MetadataMap,
    merchant_connector_account: MerchantConnectorAccountType,
) -> RouterResult<()> {
    let auth_type: ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while parsing value for ConnectorAuthType")?;

    let connector_name = merchant_connector_account
        .get_connector_name()
        .ok_or_else(|| ApiErrorResponse::InternalServerError)
        .attach_printable("Missing connector name")?;

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

pub fn convert_ucs_attempt_status(
    grpc_status: payments_grpc::AttemptStatus,
) -> RouterResult<AttemptStatus> {
    match grpc_status {
        payments_grpc::AttemptStatus::Started => Ok(AttemptStatus::Started),
        payments_grpc::AttemptStatus::AuthenticationFailed => {
            Ok(AttemptStatus::AuthenticationFailed)
        }
        payments_grpc::AttemptStatus::RouterDeclined => Ok(AttemptStatus::RouterDeclined),
        payments_grpc::AttemptStatus::AuthenticationPending => {
            Ok(AttemptStatus::AuthenticationPending)
        }
        payments_grpc::AttemptStatus::AuthenticationSuccessful => {
            Ok(AttemptStatus::AuthenticationSuccessful)
        }
        payments_grpc::AttemptStatus::Authorized => Ok(AttemptStatus::Authorized),
        payments_grpc::AttemptStatus::AuthorizationFailed => {
            Ok(AttemptStatus::AuthenticationFailed)
        }
        payments_grpc::AttemptStatus::Charged => Ok(AttemptStatus::Charged),
        payments_grpc::AttemptStatus::Authorizing => Ok(AttemptStatus::Authorizing),
        payments_grpc::AttemptStatus::CodInitiated => Ok(AttemptStatus::CodInitiated),
        payments_grpc::AttemptStatus::Voided => Ok(AttemptStatus::Voided),
        payments_grpc::AttemptStatus::VoidInitiated => Ok(AttemptStatus::VoidInitiated),
        payments_grpc::AttemptStatus::CaptureInitiated => Ok(AttemptStatus::CaptureInitiated),
        payments_grpc::AttemptStatus::CaptureFailed => Ok(AttemptStatus::CaptureFailed),
        payments_grpc::AttemptStatus::VoidFailed => Ok(AttemptStatus::VoidFailed),
        payments_grpc::AttemptStatus::AutoRefunded => Ok(AttemptStatus::AutoRefunded),
        payments_grpc::AttemptStatus::PartialCharged => Ok(AttemptStatus::PartialCharged),
        payments_grpc::AttemptStatus::PartialChargedAndChargeable => {
            Ok(AttemptStatus::PartialChargedAndChargeable)
        }
        payments_grpc::AttemptStatus::Unresolved => Ok(AttemptStatus::Unresolved),
        payments_grpc::AttemptStatus::Pending => Ok(AttemptStatus::Pending),
        payments_grpc::AttemptStatus::Failure => Ok(AttemptStatus::Failure),
        payments_grpc::AttemptStatus::PaymentMethodAwaited => {
            Ok(AttemptStatus::PaymentMethodAwaited)
        }
        payments_grpc::AttemptStatus::ConfirmationAwaited => Ok(AttemptStatus::ConfirmationAwaited),
        payments_grpc::AttemptStatus::DeviceDataCollectionPending => {
            Ok(AttemptStatus::DeviceDataCollectionPending)
        }
    }
}

pub fn construct_router_data_from_ucs_authorize_response(
    response: payments_grpc::PaymentsAuthorizeResponse,
    router_data: &mut RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
) -> RouterResult<()> {
    let status = convert_ucs_attempt_status(response.status())?;

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
            code: hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string(),
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
