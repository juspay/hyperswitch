use std::collections::HashMap;

use api_models::payments::QrCodeInformation;
use common_enums::{AttemptStatus, AuthenticationType};
use common_utils::{ext_traits::Encode, request::Method};
use diesel_models::enums as storage_enums;
use error_stack::ResultExt;
use external_services::grpc_client::unified_connector_service::UnifiedConnectorServiceError;
use hyperswitch_connectors::utils::QrImage;
use hyperswitch_domain_models::{
    router_data::{ErrorResponse, RouterData},
    router_flow_types::{
        payments::{Authorize, PSync, SetupMandate},
        ExternalVaultProxy,
    },
    router_request_types::{
        AuthenticationData, ExternalVaultProxyPaymentsData, PaymentsAuthorizeData,
        PaymentsSyncData, SetupMandateRequestData,
    },
    router_response_types::{PaymentsResponseData, RedirectForm},
};
use masking::{ExposeInterface, PeekInterface};
use router_env::tracing;
use unified_connector_service_client::payments::{
    self as payments_grpc, Identifier, PaymentServiceTransformRequest,
    PaymentServiceTransformResponse,
};
use url::Url;

use crate::{
    core::{errors, unified_connector_service},
    types::transformers::ForeignTryFrom,
};
impl ForeignTryFrom<&RouterData<PSync, PaymentsSyncData, PaymentsResponseData>>
    for payments_grpc::PaymentServiceGetRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        router_data: &RouterData<PSync, PaymentsSyncData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let connector_transaction_id = router_data
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .map(|id| Identifier {
                id_type: Some(payments_grpc::identifier::IdType::Id(id)),
            })
            .map_err(|e| {
                tracing::debug!(
                    transaction_id_error=?e,
                    "Failed to extract connector transaction ID for UCS payment sync request"
                );
                e
            })
            .ok();

        let encoded_data = router_data
            .request
            .encoded_data
            .as_ref()
            .map(|data| Identifier {
                id_type: Some(payments_grpc::identifier::IdType::EncodedData(
                    data.to_string(),
                )),
            });

        let connector_ref_id = router_data
            .request
            .connector_reference_id
            .clone()
            .map(|id| Identifier {
                id_type: Some(payments_grpc::identifier::IdType::Id(id)),
            });

        Ok(Self {
            transaction_id: connector_transaction_id.or(encoded_data),
            request_ref_id: connector_ref_id,
        })
    }
}

impl ForeignTryFrom<&RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>>
    for payments_grpc::PaymentServiceAuthorizeRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        router_data: &RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let currency = payments_grpc::Currency::foreign_try_from(router_data.request.currency)?;

        let payment_method = router_data
            .request
            .payment_method_type
            .map(|payment_method_type| {
                unified_connector_service::build_unified_connector_service_payment_method(
                    router_data.request.payment_method_data.clone(),
                    payment_method_type,
                )
            })
            .transpose()?;

        let address = payments_grpc::PaymentAddress::foreign_try_from(router_data.address.clone())?;

        let auth_type = payments_grpc::AuthenticationType::foreign_try_from(router_data.auth_type)?;

        let browser_info = router_data
            .request
            .browser_info
            .clone()
            .map(payments_grpc::BrowserInformation::foreign_try_from)
            .transpose()?;

        let capture_method = router_data
            .request
            .capture_method
            .map(payments_grpc::CaptureMethod::foreign_try_from)
            .transpose()?;

        let authentication_data = router_data
            .request
            .authentication_data
            .clone()
            .map(payments_grpc::AuthenticationData::foreign_try_from)
            .transpose()?;

        Ok(Self {
            amount: router_data.request.amount,
            currency: currency.into(),
            payment_method,
            return_url: router_data.request.router_return_url.clone(),
            address: Some(address),
            auth_type: auth_type.into(),
            enrolled_for_3ds: router_data.request.enrolled_for_3ds,
            request_incremental_authorization: router_data
                .request
                .request_incremental_authorization,
            minor_amount: router_data.request.amount,
            email: router_data
                .request
                .email
                .clone()
                .map(|e| e.expose().expose().into()),
            browser_info,
            access_token: None,
            session_token: None,
            order_tax_amount: router_data
                .request
                .order_tax_amount
                .map(|order_tax_amount| order_tax_amount.get_amount_as_i64()),
            customer_name: router_data
                .request
                .customer_name
                .clone()
                .map(|customer_name| customer_name.peek().to_owned()),
            capture_method: capture_method.map(|capture_method| capture_method.into()),
            webhook_url: router_data.request.webhook_url.clone(),
            complete_authorize_url: router_data.request.complete_authorize_url.clone(),
            setup_future_usage: None,
            off_session: None,
            customer_acceptance: None,
            order_category: router_data.request.order_category.clone(),
            payment_experience: None,
            authentication_data,
            request_extended_authorization: router_data
                .request
                .request_extended_authorization
                .map(|request_extended_authorization| request_extended_authorization.is_true()),
            merchant_order_reference_id: router_data.request.merchant_order_reference_id.clone(),
            shipping_cost: router_data
                .request
                .shipping_cost
                .map(|shipping_cost| shipping_cost.get_amount_as_i64()),
            request_ref_id: Some(Identifier {
                id_type: Some(payments_grpc::identifier::IdType::Id(
                    router_data.connector_request_reference_id.clone(),
                )),
            }),
            connector_customer_id: router_data
                .request
                .customer_id
                .as_ref()
                .map(|id| id.get_string_repr().to_string()),
            metadata: router_data
                .request
                .metadata
                .as_ref()
                .and_then(|val| val.as_object())
                .map(|map| {
                    map.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect::<HashMap<String, String>>()
                })
                .unwrap_or_default(),
            test_mode: None,
        })
    }
}

impl
    ForeignTryFrom<
        &RouterData<ExternalVaultProxy, ExternalVaultProxyPaymentsData, PaymentsResponseData>,
    > for payments_grpc::PaymentServiceAuthorizeRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        router_data: &RouterData<
            ExternalVaultProxy,
            ExternalVaultProxyPaymentsData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let currency = payments_grpc::Currency::foreign_try_from(router_data.request.currency)?;

        let payment_method = router_data
            .request
            .payment_method_type
            .map(|payment_method_type| {
                unified_connector_service::build_unified_connector_service_payment_method_for_external_proxy(
                    router_data.request.payment_method_data.clone(),
                    payment_method_type,
                )
            })
            .transpose()?;

        let address = payments_grpc::PaymentAddress::foreign_try_from(router_data.address.clone())?;

        let auth_type = payments_grpc::AuthenticationType::foreign_try_from(router_data.auth_type)?;

        let browser_info = router_data
            .request
            .browser_info
            .clone()
            .map(payments_grpc::BrowserInformation::foreign_try_from)
            .transpose()?;

        let capture_method = router_data
            .request
            .capture_method
            .map(payments_grpc::CaptureMethod::foreign_try_from)
            .transpose()?;

        let authentication_data = router_data
            .request
            .authentication_data
            .clone()
            .map(payments_grpc::AuthenticationData::foreign_try_from)
            .transpose()?;

        Ok(Self {
            amount: router_data.request.amount,
            currency: currency.into(),
            payment_method,
            return_url: router_data.request.router_return_url.clone(),
            address: Some(address),
            auth_type: auth_type.into(),
            enrolled_for_3ds: router_data.request.enrolled_for_3ds,
            request_incremental_authorization: router_data
                .request
                .request_incremental_authorization,
            minor_amount: router_data.request.amount,
            email: router_data
                .request
                .email
                .clone()
                .map(|e| e.expose().expose().into()),
            browser_info,
            access_token: None,
            session_token: None,
            order_tax_amount: router_data
                .request
                .order_tax_amount
                .map(|order_tax_amount| order_tax_amount.get_amount_as_i64()),
            customer_name: router_data
                .request
                .customer_name
                .clone()
                .map(|customer_name| customer_name.peek().to_owned()),
            capture_method: capture_method.map(|capture_method| capture_method.into()),
            webhook_url: router_data.request.webhook_url.clone(),
            complete_authorize_url: router_data.request.complete_authorize_url.clone(),
            setup_future_usage: None,
            off_session: None,
            customer_acceptance: None,
            order_category: router_data.request.order_category.clone(),
            payment_experience: None,
            authentication_data,
            request_extended_authorization: router_data
                .request
                .request_extended_authorization
                .map(|request_extended_authorization| request_extended_authorization.is_true()),
            merchant_order_reference_id: router_data.request.merchant_order_reference_id.clone(),
            shipping_cost: router_data
                .request
                .shipping_cost
                .map(|shipping_cost| shipping_cost.get_amount_as_i64()),
            request_ref_id: Some(Identifier {
                id_type: Some(payments_grpc::identifier::IdType::Id(
                    router_data.connector_request_reference_id.clone(),
                )),
            }),
            connector_customer_id: router_data
                .request
                .customer_id
                .as_ref()
                .map(|id| id.get_string_repr().to_string()),
            metadata: router_data
                .request
                .metadata
                .as_ref()
                .and_then(|val| val.as_object())
                .map(|map| {
                    map.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect::<HashMap<String, String>>()
                })
                .unwrap_or_default(),
            test_mode: None,
        })
    }
}

impl ForeignTryFrom<&RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>>
    for payments_grpc::PaymentServiceRegisterRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        router_data: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let currency = payments_grpc::Currency::foreign_try_from(router_data.request.currency)?;
        let payment_method = router_data
            .request
            .payment_method_type
            .map(|payment_method_type| {
                unified_connector_service::build_unified_connector_service_payment_method(
                    router_data.request.payment_method_data.clone(),
                    payment_method_type,
                )
            })
            .transpose()?;
        let address = payments_grpc::PaymentAddress::foreign_try_from(router_data.address.clone())?;
        let auth_type = payments_grpc::AuthenticationType::foreign_try_from(router_data.auth_type)?;
        let browser_info = router_data
            .request
            .browser_info
            .clone()
            .map(payments_grpc::BrowserInformation::foreign_try_from)
            .transpose()?;
        let setup_future_usage = router_data
            .request
            .setup_future_usage
            .map(payments_grpc::FutureUsage::foreign_try_from)
            .transpose()?;
        let customer_acceptance = router_data
            .request
            .customer_acceptance
            .clone()
            .map(payments_grpc::CustomerAcceptance::foreign_try_from)
            .transpose()?;

        Ok(Self {
            request_ref_id: Some(Identifier {
                id_type: Some(payments_grpc::identifier::IdType::Id(
                    router_data.connector_request_reference_id.clone(),
                )),
            }),
            currency: currency.into(),
            payment_method,
            minor_amount: router_data.request.amount,
            email: router_data
                .request
                .email
                .clone()
                .map(|e| e.expose().expose().into()),
            customer_name: router_data
                .request
                .customer_name
                .clone()
                .map(|customer_name| customer_name.peek().to_owned()),
            connector_customer_id: router_data
                .request
                .customer_id
                .as_ref()
                .map(|id| id.get_string_repr().to_string()),
            address: Some(address),
            auth_type: auth_type.into(),
            enrolled_for_3ds: false,
            authentication_data: None,
            metadata: router_data
                .request
                .metadata
                .as_ref()
                .map(|secret| secret.peek())
                .and_then(|val| val.as_object()) //secret
                .map(|map| {
                    map.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect::<HashMap<String, String>>()
                })
                .unwrap_or_default(),
            return_url: router_data.request.router_return_url.clone(),
            webhook_url: router_data.request.webhook_url.clone(),
            complete_authorize_url: router_data.request.complete_authorize_url.clone(),
            access_token: None,
            session_token: None,
            order_tax_amount: None,
            order_category: None,
            merchant_order_reference_id: None,
            shipping_cost: router_data
                .request
                .shipping_cost
                .map(|cost| cost.get_amount_as_i64()),
            setup_future_usage: setup_future_usage.map(|s| s.into()),
            off_session: router_data.request.off_session,
            request_incremental_authorization: router_data
                .request
                .request_incremental_authorization,
            request_extended_authorization: None,
            customer_acceptance,
            browser_info,
            payment_experience: None,
        })
    }
}

impl ForeignTryFrom<&RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>>
    for payments_grpc::PaymentServiceRepeatEverythingRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        router_data: &RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let currency = payments_grpc::Currency::foreign_try_from(router_data.request.currency)?;
        let browser_info = router_data
            .request
            .browser_info
            .clone()
            .map(payments_grpc::BrowserInformation::foreign_try_from)
            .transpose()?;
        let capture_method = router_data
            .request
            .capture_method
            .map(payments_grpc::CaptureMethod::foreign_try_from)
            .transpose()?;

        let mandate_reference = match &router_data.request.mandate_id {
            Some(mandate) => match &mandate.mandate_reference_id {
                Some(api_models::payments::MandateReferenceId::ConnectorMandateId(
                    connector_mandate_id,
                )) => Some(payments_grpc::MandateReference {
                    mandate_id: connector_mandate_id.get_connector_mandate_id(),
                }),
                _ => {
                    return Err(UnifiedConnectorServiceError::MissingRequiredField {
                        field_name: "connector_mandate_id",
                    }
                    .into())
                }
            },
            None => {
                return Err(UnifiedConnectorServiceError::MissingRequiredField {
                    field_name: "connector_mandate_id",
                }
                .into())
            }
        };

        Ok(Self {
            request_ref_id: Some(Identifier {
                id_type: Some(payments_grpc::identifier::IdType::Id(
                    router_data.connector_request_reference_id.clone(),
                )),
            }),
            mandate_reference,
            amount: router_data.request.amount,
            currency: currency.into(),
            minor_amount: router_data.request.amount,
            merchant_order_reference_id: router_data.request.merchant_order_reference_id.clone(),
            metadata: router_data
                .request
                .metadata
                .as_ref()
                .and_then(|val| val.as_object())
                .map(|map| {
                    map.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect::<HashMap<String, String>>()
                })
                .unwrap_or_default(),
            webhook_url: router_data.request.webhook_url.clone(),
            capture_method: capture_method.map(|capture_method| capture_method.into()),
            email: router_data
                .request
                .email
                .clone()
                .map(|e| e.expose().expose().into()),
            browser_info,
            test_mode: None,
            payment_method_type: None,
        })
    }
}

impl ForeignTryFrom<payments_grpc::PaymentServiceAuthorizeResponse>
    for Result<PaymentsResponseData, ErrorResponse>
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        response: payments_grpc::PaymentServiceAuthorizeResponse,
    ) -> Result<Self, Self::Error> {
        let status = AttemptStatus::foreign_try_from(response.status())?;

        let connector_response_reference_id =
            response.response_ref_id.as_ref().and_then(|identifier| {
                identifier
                    .id_type
                    .clone()
                    .and_then(|id_type| match id_type {
                        payments_grpc::identifier::IdType::Id(id) => Some(id),
                        payments_grpc::identifier::IdType::EncodedData(encoded_data) => {
                            Some(encoded_data)
                        }
                        payments_grpc::identifier::IdType::NoResponseIdMarker(_) => None,
                    })
            });

        let resource_id: hyperswitch_domain_models::router_request_types::ResponseId = match response.transaction_id.as_ref().and_then(|id| id.id_type.clone()) {
            Some(payments_grpc::identifier::IdType::Id(id)) => hyperswitch_domain_models::router_request_types::ResponseId::ConnectorTransactionId(id),
            Some(payments_grpc::identifier::IdType::EncodedData(encoded_data)) => hyperswitch_domain_models::router_request_types::ResponseId::EncodedData(encoded_data),
            Some(payments_grpc::identifier::IdType::NoResponseIdMarker(_)) | None => hyperswitch_domain_models::router_request_types::ResponseId::NoResponseId,
        };

        let (connector_metadata, redirection_data) = match response.redirection_data.clone() {
            Some(redirection_data) => match redirection_data.form_type {
                Some(ref form_type) => match form_type {
                    payments_grpc::redirect_form::FormType::Uri(uri) => {
                        let image_data = QrImage::new_from_data(uri.uri.clone())
                            .change_context(UnifiedConnectorServiceError::ParsingFailed)?;
                        let image_data_url = Url::parse(image_data.data.clone().as_str())
                            .change_context(UnifiedConnectorServiceError::ParsingFailed)?;
                        let qr_code_info = QrCodeInformation::QrDataUrl {
                            image_data_url,
                            display_to_timestamp: None,
                        };
                        (
                            Some(qr_code_info.encode_to_value())
                                .transpose()
                                .change_context(UnifiedConnectorServiceError::ParsingFailed)?,
                            None,
                        )
                    }
                    _ => (
                        None,
                        Some(RedirectForm::foreign_try_from(redirection_data)).transpose()?,
                    ),
                },
                None => (None, None),
            },
            None => (None, None),
        };

        let status_code = convert_connector_service_status_code(response.status_code)?;

        let response = if response.error_code.is_some() {
            Err(ErrorResponse {
                code: response.error_code().to_owned(),
                message: response.error_message().to_owned(),
                reason: Some(response.error_message().to_owned()),
                status_code,
                attempt_status: Some(status),
                connector_transaction_id: connector_response_reference_id,
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id,
                redirection_data: Box::new(redirection_data),
                mandate_reference: Box::new(None),
                connector_metadata,
                network_txn_id: response.network_txn_id.clone(),
                connector_response_reference_id,
                incremental_authorization_allowed: response.incremental_authorization_allowed,
                charges: None,
            })
        };

        Ok(response)
    }
}

impl ForeignTryFrom<payments_grpc::PaymentServiceGetResponse>
    for Result<PaymentsResponseData, ErrorResponse>
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        response: payments_grpc::PaymentServiceGetResponse,
    ) -> Result<Self, Self::Error> {
        let status = AttemptStatus::foreign_try_from(response.status())?;

        let connector_response_reference_id =
            response.response_ref_id.as_ref().and_then(|identifier| {
                identifier
                    .id_type
                    .clone()
                    .and_then(|id_type| match id_type {
                        payments_grpc::identifier::IdType::Id(id) => Some(id),
                        payments_grpc::identifier::IdType::EncodedData(encoded_data) => {
                            Some(encoded_data)
                        }
                        payments_grpc::identifier::IdType::NoResponseIdMarker(_) => None,
                    })
            });

        let status_code = convert_connector_service_status_code(response.status_code)?;

        let resource_id: hyperswitch_domain_models::router_request_types::ResponseId = match response.transaction_id.as_ref().and_then(|id| id.id_type.clone()) {
            Some(payments_grpc::identifier::IdType::Id(id)) => hyperswitch_domain_models::router_request_types::ResponseId::ConnectorTransactionId(id),
            Some(payments_grpc::identifier::IdType::EncodedData(encoded_data)) => hyperswitch_domain_models::router_request_types::ResponseId::EncodedData(encoded_data),
            Some(payments_grpc::identifier::IdType::NoResponseIdMarker(_)) | None => hyperswitch_domain_models::router_request_types::ResponseId::NoResponseId,
        };

        let response = if response.error_code.is_some() {
            Err(ErrorResponse {
                code: response.error_code().to_owned(),
                message: response.error_message().to_owned(),
                reason: Some(response.error_message().to_owned()),
                status_code,
                attempt_status: Some(status),
                connector_transaction_id: connector_response_reference_id,
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id,
                redirection_data: Box::new(None),
                mandate_reference: Box::new(response.mandate_reference.map(|grpc_mandate| {
                    hyperswitch_domain_models::router_response_types::MandateReference {
                        connector_mandate_id: grpc_mandate.mandate_id,
                        payment_method_id: None,
                        mandate_metadata: None,
                        connector_mandate_request_reference_id: None,
                    }
                })),
                connector_metadata: None,
                network_txn_id: response.network_txn_id.clone(),
                connector_response_reference_id,
                incremental_authorization_allowed: None,
                charges: None,
            })
        };

        Ok(response)
    }
}

impl ForeignTryFrom<payments_grpc::PaymentServiceRegisterResponse>
    for Result<PaymentsResponseData, ErrorResponse>
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        response: payments_grpc::PaymentServiceRegisterResponse,
    ) -> Result<Self, Self::Error> {
        let status = AttemptStatus::foreign_try_from(response.status())?;

        let connector_response_reference_id =
            response.response_ref_id.as_ref().and_then(|identifier| {
                identifier
                    .id_type
                    .clone()
                    .and_then(|id_type| match id_type {
                        payments_grpc::identifier::IdType::Id(id) => Some(id),
                        payments_grpc::identifier::IdType::EncodedData(encoded_data) => {
                            Some(encoded_data)
                        }
                        payments_grpc::identifier::IdType::NoResponseIdMarker(_) => None,
                    })
            });

        let status_code = convert_connector_service_status_code(response.status_code)?;

        let response = if response.error_code.is_some() {
            Err(ErrorResponse {
                code: response.error_code().to_owned(),
                message: response.error_message().to_owned(),
                reason: Some(response.error_message().to_owned()),
                status_code,
                attempt_status: Some(status),
                connector_transaction_id: connector_response_reference_id,
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: response.registration_id.as_ref().and_then(|identifier| {
                    identifier
                        .id_type
                        .clone()
                        .and_then(|id_type| match id_type {
                            payments_grpc::identifier::IdType::Id(id) => Some(
                                hyperswitch_domain_models::router_request_types::ResponseId::ConnectorTransactionId(id),
                            ),
                            payments_grpc::identifier::IdType::EncodedData(encoded_data) => Some(
                                hyperswitch_domain_models::router_request_types::ResponseId::ConnectorTransactionId(encoded_data),
                            ),
                            payments_grpc::identifier::IdType::NoResponseIdMarker(_) => None,
                        })
                }).unwrap_or(hyperswitch_domain_models::router_request_types::ResponseId::NoResponseId),
                redirection_data: Box::new(
                    response
                        .redirection_data
                        .clone()
                        .map(RedirectForm::foreign_try_from)
                        .transpose()?
                ),
                mandate_reference: Box::new(
                    response.mandate_reference.map(|grpc_mandate| {
                        hyperswitch_domain_models::router_response_types::MandateReference {
                            connector_mandate_id: grpc_mandate.mandate_id,
                            payment_method_id: None,
                            mandate_metadata: None,
                            connector_mandate_request_reference_id: None,
                        }
                    })
                ),
                connector_metadata: None,
                network_txn_id: response.network_txn_id,
                connector_response_reference_id,
                incremental_authorization_allowed: response.incremental_authorization_allowed,
                charges: None,
            })
        };

        Ok(response)
    }
}

impl ForeignTryFrom<payments_grpc::PaymentServiceRepeatEverythingResponse>
    for Result<PaymentsResponseData, ErrorResponse>
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        response: payments_grpc::PaymentServiceRepeatEverythingResponse,
    ) -> Result<Self, Self::Error> {
        let status = AttemptStatus::foreign_try_from(response.status())?;

        let connector_response_reference_id =
            response.response_ref_id.as_ref().and_then(|identifier| {
                identifier
                    .id_type
                    .clone()
                    .and_then(|id_type| match id_type {
                        payments_grpc::identifier::IdType::Id(id) => Some(id),
                        payments_grpc::identifier::IdType::EncodedData(encoded_data) => {
                            Some(encoded_data)
                        }
                        payments_grpc::identifier::IdType::NoResponseIdMarker(_) => None,
                    })
            });

        let transaction_id = response.transaction_id.as_ref().and_then(|id| {
            id.id_type.clone().and_then(|id_type| match id_type {
                payments_grpc::identifier::IdType::Id(id) => Some(id),
                payments_grpc::identifier::IdType::EncodedData(encoded_data) => Some(encoded_data),
                payments_grpc::identifier::IdType::NoResponseIdMarker(_) => None,
            })
        });

        let status_code = convert_connector_service_status_code(response.status_code)?;

        let response = if response.error_code.is_some() {
            Err(ErrorResponse {
                code: response.error_code().to_owned(),
                message: response.error_message().to_owned(),
                reason: Some(response.error_message().to_owned()),
                status_code,
                attempt_status: Some(status),
                connector_transaction_id: transaction_id,
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: match transaction_id.as_ref() {
                    Some(transaction_id) => hyperswitch_domain_models::router_request_types::ResponseId::ConnectorTransactionId(transaction_id.clone()),
                    None => hyperswitch_domain_models::router_request_types::ResponseId::NoResponseId,
                },
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: response.network_txn_id.clone(),
                connector_response_reference_id,
                incremental_authorization_allowed: None,
                charges: None,
            })
        };

        Ok(response)
    }
}

impl ForeignTryFrom<common_enums::Currency> for payments_grpc::Currency {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(currency: common_enums::Currency) -> Result<Self, Self::Error> {
        Self::from_str_name(&currency.to_string()).ok_or_else(|| {
            UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                "Failed to parse currency".to_string(),
            )
            .into()
        })
    }
}

impl ForeignTryFrom<common_enums::CardNetwork> for payments_grpc::CardNetwork {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(card_network: common_enums::CardNetwork) -> Result<Self, Self::Error> {
        match card_network {
            common_enums::CardNetwork::Visa => Ok(Self::Visa),
            common_enums::CardNetwork::Mastercard => Ok(Self::Mastercard),
            common_enums::CardNetwork::JCB => Ok(Self::Jcb),
            common_enums::CardNetwork::DinersClub => Ok(Self::Diners),
            common_enums::CardNetwork::Discover => Ok(Self::Discover),
            common_enums::CardNetwork::CartesBancaires => Ok(Self::CartesBancaires),
            common_enums::CardNetwork::UnionPay => Ok(Self::Unionpay),
            common_enums::CardNetwork::RuPay => Ok(Self::Rupay),
            common_enums::CardNetwork::Maestro => Ok(Self::Maestro),
            common_enums::CardNetwork::AmericanExpress => Ok(Self::Amex),
            _ => Err(
                UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                    "Card Network not supported".to_string(),
                )
                .into(),
            ),
        }
    }
}

impl ForeignTryFrom<hyperswitch_domain_models::payment_address::PaymentAddress>
    for payments_grpc::PaymentAddress
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        payment_address: hyperswitch_domain_models::payment_address::PaymentAddress,
    ) -> Result<Self, Self::Error> {
        let shipping = payment_address.get_shipping().map(|address| {
            let details = address.address.as_ref();

            let country = details.and_then(|details| {
                details
                    .country
                    .as_ref()
                    .and_then(|c| payments_grpc::CountryAlpha2::from_str_name(&c.to_string()))
                    .map(|country| country.into())
            });

            payments_grpc::Address {
                first_name: details.and_then(|d| d.first_name.as_ref().map(|s| s.clone().expose())),
                last_name: details.and_then(|d| d.last_name.as_ref().map(|s| s.clone().expose())),
                line1: details.and_then(|d| d.line1.as_ref().map(|s| s.clone().expose().into())),
                line2: details.and_then(|d| d.line2.as_ref().map(|s| s.clone().expose().into())),
                line3: details.and_then(|d| d.line3.as_ref().map(|s| s.clone().expose().into())),
                city: details.and_then(|d| d.city.as_ref().map(|s| s.clone().into())),
                state: details.and_then(|d| d.state.as_ref().map(|s| s.clone().expose().into())),
                zip_code: details.and_then(|d| d.zip.as_ref().map(|s| s.clone().expose().into())),
                country_alpha2_code: country,
                email: address
                    .email
                    .as_ref()
                    .map(|e| e.clone().expose().expose().into()),
                phone_number: address
                    .phone
                    .as_ref()
                    .and_then(|phone| phone.number.as_ref().map(|n| n.clone().expose().into())),
                phone_country_code: address.phone.as_ref().and_then(|p| p.country_code.clone()),
            }
        });

        let billing = payment_address.get_payment_billing().map(|address| {
            let details = address.address.as_ref();

            let country = details.and_then(|details| {
                details
                    .country
                    .as_ref()
                    .and_then(|c| payments_grpc::CountryAlpha2::from_str_name(&c.to_string()))
                    .map(|country| country.into())
            });

            payments_grpc::Address {
                first_name: details
                    .and_then(|d| d.first_name.as_ref().map(|s| s.peek().to_string())),
                last_name: details.and_then(|d| d.last_name.as_ref().map(|s| s.peek().to_string())),
                line1: details.and_then(|d| d.line1.as_ref().map(|s| s.peek().to_string().into())),
                line2: details.and_then(|d| d.line2.as_ref().map(|s| s.peek().to_string().into())),
                line3: details.and_then(|d| d.line3.as_ref().map(|s| s.peek().to_string().into())),
                city: details.and_then(|d| d.city.as_ref().map(|s| s.clone().into())),
                state: details.and_then(|d| d.state.as_ref().map(|s| s.peek().to_string().into())),
                zip_code: details.and_then(|d| d.zip.as_ref().map(|s| s.peek().to_string().into())),
                country_alpha2_code: country,
                email: address.email.as_ref().map(|e| e.peek().to_string().into()),
                phone_number: address
                    .phone
                    .as_ref()
                    .and_then(|phone| phone.number.as_ref().map(|n| n.peek().to_string().into())),
                phone_country_code: address.phone.as_ref().and_then(|p| p.country_code.clone()),
            }
        });

        let unified_payment_method_billing =
            payment_address.get_payment_method_billing().map(|address| {
                let details = address.address.as_ref();

                let country = details.and_then(|details| {
                    details
                        .country
                        .as_ref()
                        .and_then(|c| payments_grpc::CountryAlpha2::from_str_name(&c.to_string()))
                        .map(|country| country.into())
                });

                payments_grpc::Address {
                    first_name: details
                        .and_then(|d| d.first_name.as_ref().map(|s| s.peek().to_string())),
                    last_name: details
                        .and_then(|d| d.last_name.as_ref().map(|s| s.peek().to_string())),
                    line1: details
                        .and_then(|d| d.line1.as_ref().map(|s| s.peek().to_string().into())),
                    line2: details
                        .and_then(|d| d.line2.as_ref().map(|s| s.peek().to_string().into())),
                    line3: details
                        .and_then(|d| d.line3.as_ref().map(|s| s.peek().to_string().into())),
                    city: details.and_then(|d| d.city.as_ref().map(|s| s.clone().into())),
                    state: details
                        .and_then(|d| d.state.as_ref().map(|s| s.peek().to_string().into())),
                    zip_code: details
                        .and_then(|d| d.zip.as_ref().map(|s| s.peek().to_string().into())),
                    country_alpha2_code: country,
                    email: address
                        .email
                        .as_ref()
                        .map(|e| e.clone().expose().expose().into()),
                    phone_number: address
                        .phone
                        .as_ref()
                        .and_then(|phone| phone.number.as_ref().map(|n| n.clone().expose().into())),
                    phone_country_code: address.phone.as_ref().and_then(|p| p.country_code.clone()),
                }
            });
        Ok(Self {
            shipping_address: shipping.or(unified_payment_method_billing.clone()),
            billing_address: billing.or(unified_payment_method_billing),
        })
    }
}

impl ForeignTryFrom<AuthenticationType> for payments_grpc::AuthenticationType {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

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
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

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
            ip_address: browser_info.ip_address.map(|ip| ip.to_string()),
            accept_header: browser_info.accept_header,
            user_agent: browser_info.user_agent,
            os_type: browser_info.os_type,
            os_version: browser_info.os_version,
            device_model: browser_info.device_model,
            accept_language: browser_info.accept_language,
            time_zone_offset_minutes: browser_info.time_zone,
        })
    }
}

impl ForeignTryFrom<storage_enums::CaptureMethod> for payments_grpc::CaptureMethod {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(capture_method: storage_enums::CaptureMethod) -> Result<Self, Self::Error> {
        match capture_method {
            common_enums::CaptureMethod::Automatic => Ok(Self::Automatic),
            common_enums::CaptureMethod::Manual => Ok(Self::Manual),
            common_enums::CaptureMethod::ManualMultiple => Ok(Self::ManualMultiple),
            common_enums::CaptureMethod::Scheduled => Ok(Self::Scheduled),
            common_enums::CaptureMethod::SequentialAutomatic => Ok(Self::SequentialAutomatic),
        }
    }
}

impl ForeignTryFrom<AuthenticationData> for payments_grpc::AuthenticationData {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(authentication_data: AuthenticationData) -> Result<Self, Self::Error> {
        Ok(Self {
            eci: authentication_data.eci,
            cavv: authentication_data.cavv.peek().to_string(),
            threeds_server_transaction_id: authentication_data.threeds_server_transaction_id.map(
                |id| Identifier {
                    id_type: Some(payments_grpc::identifier::IdType::Id(id)),
                },
            ),
            message_version: None,
            ds_transaction_id: authentication_data.ds_trans_id,
        })
    }
}

impl ForeignTryFrom<payments_grpc::PaymentStatus> for AttemptStatus {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(grpc_status: payments_grpc::PaymentStatus) -> Result<Self, Self::Error> {
        match grpc_status {
            payments_grpc::PaymentStatus::Started => Ok(Self::Started),
            payments_grpc::PaymentStatus::AuthenticationFailed => Ok(Self::AuthenticationFailed),
            payments_grpc::PaymentStatus::RouterDeclined => Ok(Self::RouterDeclined),
            payments_grpc::PaymentStatus::AuthenticationPending => Ok(Self::AuthenticationPending),
            payments_grpc::PaymentStatus::AuthenticationSuccessful => {
                Ok(Self::AuthenticationSuccessful)
            }
            payments_grpc::PaymentStatus::Authorized => Ok(Self::Authorized),
            payments_grpc::PaymentStatus::AuthorizationFailed => Ok(Self::AuthorizationFailed),
            payments_grpc::PaymentStatus::Charged => Ok(Self::Charged),
            payments_grpc::PaymentStatus::Authorizing => Ok(Self::Authorizing),
            payments_grpc::PaymentStatus::CodInitiated => Ok(Self::CodInitiated),
            payments_grpc::PaymentStatus::Voided => Ok(Self::Voided),
            payments_grpc::PaymentStatus::VoidInitiated => Ok(Self::VoidInitiated),
            payments_grpc::PaymentStatus::CaptureInitiated => Ok(Self::CaptureInitiated),
            payments_grpc::PaymentStatus::CaptureFailed => Ok(Self::CaptureFailed),
            payments_grpc::PaymentStatus::VoidFailed => Ok(Self::VoidFailed),
            payments_grpc::PaymentStatus::AutoRefunded => Ok(Self::AutoRefunded),
            payments_grpc::PaymentStatus::PartialCharged => Ok(Self::PartialCharged),
            payments_grpc::PaymentStatus::PartialChargedAndChargeable => {
                Ok(Self::PartialChargedAndChargeable)
            }
            payments_grpc::PaymentStatus::Unresolved => Ok(Self::Unresolved),
            payments_grpc::PaymentStatus::Pending => Ok(Self::Pending),
            payments_grpc::PaymentStatus::Failure => Ok(Self::Failure),
            payments_grpc::PaymentStatus::PaymentMethodAwaited => Ok(Self::PaymentMethodAwaited),
            payments_grpc::PaymentStatus::ConfirmationAwaited => Ok(Self::ConfirmationAwaited),
            payments_grpc::PaymentStatus::DeviceDataCollectionPending => {
                Ok(Self::DeviceDataCollectionPending)
            }
            payments_grpc::PaymentStatus::AttemptStatusUnspecified => Ok(Self::Unresolved),
        }
    }
}

impl ForeignTryFrom<payments_grpc::RedirectForm> for RedirectForm {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(value: payments_grpc::RedirectForm) -> Result<Self, Self::Error> {
        match value.form_type {
            Some(payments_grpc::redirect_form::FormType::Form(form)) => Ok(Self::Form {
                endpoint: form.clone().endpoint,
                method: Method::foreign_try_from(form.clone().method())?,
                form_fields: form.clone().form_fields,
            }),
            Some(payments_grpc::redirect_form::FormType::Html(html)) => Ok(Self::Html {
                html_data: html.html_data,
            }),
            Some(payments_grpc::redirect_form::FormType::Uri(_)) => Err(
                UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                    "URI form type is not implemented".to_string(),
                )
                .into(),
            ),
            None => Err(
                UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                    "Missing form type".to_string(),
                )
                .into(),
            ),
        }
    }
}

impl ForeignTryFrom<payments_grpc::HttpMethod> for Method {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(value: payments_grpc::HttpMethod) -> Result<Self, Self::Error> {
        tracing::debug!("Converting gRPC HttpMethod: {:?}", value);
        match value {
            payments_grpc::HttpMethod::Get => Ok(Self::Get),
            payments_grpc::HttpMethod::Post => Ok(Self::Post),
            payments_grpc::HttpMethod::Put => Ok(Self::Put),
            payments_grpc::HttpMethod::Delete => Ok(Self::Delete),
            payments_grpc::HttpMethod::Unspecified => {
                Err(UnifiedConnectorServiceError::ResponseDeserializationFailed)
                    .attach_printable("Invalid Http Method")
            }
        }
    }
}

impl ForeignTryFrom<storage_enums::FutureUsage> for payments_grpc::FutureUsage {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(future_usage: storage_enums::FutureUsage) -> Result<Self, Self::Error> {
        match future_usage {
            storage_enums::FutureUsage::OnSession => Ok(Self::OnSession),
            storage_enums::FutureUsage::OffSession => Ok(Self::OffSession),
        }
    }
}

impl ForeignTryFrom<common_types::payments::CustomerAcceptance>
    for payments_grpc::CustomerAcceptance
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        customer_acceptance: common_types::payments::CustomerAcceptance,
    ) -> Result<Self, Self::Error> {
        let acceptance_type = match customer_acceptance.acceptance_type {
            common_types::payments::AcceptanceType::Online => payments_grpc::AcceptanceType::Online,
            common_types::payments::AcceptanceType::Offline => {
                payments_grpc::AcceptanceType::Offline
            }
        };

        let online_mandate_details =
            customer_acceptance
                .online
                .map(|online| payments_grpc::OnlineMandate {
                    ip_address: online.ip_address.map(|ip| ip.peek().to_string()),
                    user_agent: online.user_agent,
                });

        Ok(Self {
            acceptance_type: acceptance_type.into(),
            accepted_at: customer_acceptance
                .accepted_at
                .map(|dt| dt.assume_utc().unix_timestamp())
                .unwrap_or_default(),
            online_mandate_details,
        })
    }
}

impl ForeignTryFrom<&hyperswitch_interfaces::webhooks::IncomingWebhookRequestDetails<'_>>
    for payments_grpc::RequestDetails
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        request_details: &hyperswitch_interfaces::webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> Result<Self, Self::Error> {
        let headers_map = request_details
            .headers
            .iter()
            .map(|(key, value)| {
                let value_string = value.to_str().unwrap_or_default().to_string();
                (key.as_str().to_string(), value_string)
            })
            .collect();

        Ok(Self {
            method: 1, // POST method for webhooks
            uri: Some({
                let uri_result = request_details
                    .headers
                    .get("x-forwarded-path")
                    .and_then(|h| h.to_str().map_err(|e| {
                        tracing::warn!(
                            header_conversion_error=?e,
                            header_value=?h,
                            "Failed to convert x-forwarded-path header to string for webhook processing"
                        );
                        e
                    }).ok());

                uri_result.unwrap_or_else(|| {
                    tracing::debug!("x-forwarded-path header not found or invalid, using default '/Unknown'");
                    "/Unknown"
                }).to_string()
            }),
            body: request_details.body.to_vec(),
            headers: headers_map,
            query_params: Some(request_details.query_params.clone()),
        })
    }
}

/// Webhook transform data structure containing UCS response information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WebhookTransformData {
    pub event_type: api_models::webhooks::IncomingWebhookEvent,
    pub source_verified: bool,
    pub webhook_content: Option<payments_grpc::WebhookResponseContent>,
    pub response_ref_id: Option<String>,
}

/// Transform UCS webhook response into webhook event data
pub fn transform_ucs_webhook_response(
    response: PaymentServiceTransformResponse,
) -> Result<WebhookTransformData, error_stack::Report<errors::ApiErrorResponse>> {
    let event_type =
        api_models::webhooks::IncomingWebhookEvent::from_ucs_event_type(response.event_type);

    Ok(WebhookTransformData {
        event_type,
        source_verified: response.source_verified,
        webhook_content: response.content,
        response_ref_id: response.response_ref_id.and_then(|identifier| {
            identifier.id_type.and_then(|id_type| match id_type {
                payments_grpc::identifier::IdType::Id(id) => Some(id),
                payments_grpc::identifier::IdType::EncodedData(encoded_data) => Some(encoded_data),
                payments_grpc::identifier::IdType::NoResponseIdMarker(_) => None,
            })
        }),
    })
}

/// Build UCS webhook transform request from webhook components
pub fn build_webhook_transform_request(
    _webhook_body: &[u8],
    request_details: &hyperswitch_interfaces::webhooks::IncomingWebhookRequestDetails<'_>,
    webhook_secrets: Option<payments_grpc::WebhookSecrets>,
    merchant_id: &str,
    connector_id: &str,
) -> Result<PaymentServiceTransformRequest, error_stack::Report<errors::ApiErrorResponse>> {
    let request_details_grpc = payments_grpc::RequestDetails::foreign_try_from(request_details)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to transform webhook request details to gRPC format")?;

    Ok(PaymentServiceTransformRequest {
        request_ref_id: Some(Identifier {
            id_type: Some(payments_grpc::identifier::IdType::Id(format!(
                "{}_{}_{}",
                merchant_id,
                connector_id,
                time::OffsetDateTime::now_utc().unix_timestamp()
            ))),
        }),
        request_details: Some(request_details_grpc),
        webhook_secrets,
    })
}

pub fn convert_connector_service_status_code(
    status_code: u32,
) -> Result<u16, error_stack::Report<UnifiedConnectorServiceError>> {
    u16::try_from(status_code).map_err(|err| {
        UnifiedConnectorServiceError::RequestEncodingFailedWithReason(format!(
            "Failed to convert connector service status code to u16: {err}"
        ))
        .into()
    })
}
