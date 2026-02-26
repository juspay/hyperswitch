use std::{collections::HashMap, str::FromStr};

use api_models::payments::{
    AmountInfo, ApplePayAddressParameters, ApplePayPaymentRequest, ApplePaySessionResponse,
    ApplepaySessionTokenResponse, GooglePaySessionResponse, GpayAllowedMethodsParameters,
    GpayAllowedPaymentMethods, GpayBillingAddressFormat, GpayBillingAddressParameters,
    GpayMerchantInfo, GpaySessionTokenResponse, GpayShippingAddressParameters, GpayTokenParameters,
    GpayTokenizationSpecification, GpayTransactionInfo, NextActionCall, PaypalFlow,
    PaypalSessionTokenResponse, PaypalTransactionInfo, SdkNextAction, SecretInfoToInitiateSdk,
    SessionToken, ThirdPartySdkSessionResponse,
};
use common_enums::{AttemptStatus, AuthenticationType, AuthorizationStatus, RefundStatus};
use common_utils::{
    ext_traits::Encode,
    types::{self, AmountConvertor, MinorUnit, StringMajorUnitForConnector},
};
use diesel_models::enums as storage_enums;
use error_stack::{report, ResultExt};
use external_services::grpc_client::unified_connector_service::UnifiedConnectorServiceError;
use hyperswitch_domain_models::{
    mandates::{MandateData, MandateDataType},
    router_data::{AccessToken, ErrorResponse, RouterData},
    router_flow_types::{
        payments::{Authorize, Capture, PSync, SetupMandate},
        refunds::{Execute, RSync},
        unified_authentication_service as uas_flows, ExternalVaultProxy, IncrementalAuthorization,
        Session,
    },
    router_request_types::{
        self, AuthenticationData, ExternalVaultProxyPaymentsData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsIncrementalAuthorizationData,
        PaymentsSessionData, PaymentsSyncData, RefundsData, SetupMandateRequestData,
        SyncRequestType,
    },
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
};
pub use hyperswitch_interfaces::{
    helpers::ForeignTryFrom,
    unified_connector_service::{
        transformers::convert_connector_service_status_code, WebhookTransformData,
        WebhookTransformationStatus,
    },
};
use masking::{ExposeInterface, PeekInterface, Secret};
use router_env::tracing;
use time::{Duration, OffsetDateTime};
use unified_connector_service_cards::{CardNumber, NetworkToken};
use unified_connector_service_client::payments::{
    self as payments_grpc, session_token, ConnectorState, Identifier,
    PaymentServiceTransformRequest, PaymentServiceTransformResponse,
};
use unified_connector_service_masking::ExposeInterface as UcsMaskingExposeInterface;

use crate::{
    core::{errors, mandate::MandateBehaviour, unified_connector_service},
    types::{
        api,
        transformers::{self, ForeignFrom},
    },
};

const UPI_WAIT_SCREEN_DISPLAY_DURATION_MINUTES: i64 = 5;
const UPI_POLL_DELAY_IN_SECS: u16 = 5;
const UPI_POLL_FREQUENCY: u16 = 60;

pub fn build_upi_wait_screen_data(
) -> Result<serde_json::Value, error_stack::Report<UnifiedConnectorServiceError>> {
    let current_time = OffsetDateTime::now_utc().unix_timestamp_nanos();

    let wait_screen_data = api_models::payments::WaitScreenInstructions {
        display_from_timestamp: current_time,
        display_to_timestamp: Some(
            current_time
                + Duration::minutes(UPI_WAIT_SCREEN_DISPLAY_DURATION_MINUTES).whole_nanoseconds(),
        ),
        poll_config: Some(api_models::payments::PollConfig {
            delay_in_secs: UPI_POLL_DELAY_IN_SECS,
            frequency: UPI_POLL_FREQUENCY,
        }),
    };

    serde_json::to_value(wait_screen_data)
        .change_context(UnifiedConnectorServiceError::ParsingFailed)
        .attach_printable("Failed to serialize WaitScreenInstructions to JSON value")
}

impl transformers::ForeignTryFrom<&payments_grpc::AccessToken> for AccessToken {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(grpc_token: &payments_grpc::AccessToken) -> Result<Self, Self::Error> {
        let token = grpc_token
            .token
            .clone()
            .ok_or(UnifiedConnectorServiceError::MissingRequiredField {
                field_name: "token",
            })
            .attach_printable("Missing token in AccessToken response")?;

        Ok(Self {
            token: token.expose().into(),
            expires: grpc_token.expires_in_seconds.unwrap_or_default(),
        })
    }
}

impl ForeignFrom<&AccessToken> for ConnectorState {
    fn foreign_from(access_token: &AccessToken) -> Self {
        Self {
            access_token: Some(payments_grpc::AccessToken {
                token: Some(access_token.token.clone().expose().into()),
                expires_in_seconds: Some(access_token.expires),
                token_type: None,
            }),
            connector_customer_id: None,
        }
    }
}

impl
    transformers::ForeignTryFrom<
        &RouterData<
            api::PaymentMethodToken,
            router_request_types::PaymentMethodTokenizationData,
            PaymentsResponseData,
        >,
    > for payments_grpc::PaymentServiceCreatePaymentMethodTokenRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;
    fn foreign_try_from(
        router_data: &RouterData<
            api::PaymentMethodToken,
            router_request_types::PaymentMethodTokenizationData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let connector_ref_id = Identifier {
            id_type: Some(payments_grpc::identifier::IdType::Id(
                router_data.connector_request_reference_id.clone(),
            )),
        };

        let merchant_account_metadata = router_data
            .connector_meta_data
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
            .map(|s| s.into());

        let currency = payments_grpc::Currency::foreign_try_from(router_data.request.currency)?;

        // Always build payment_method using payment_method_data (which is non-optional).
        // payment_method_type is passed as optional, but payment_method must always be present
        // for UCS to process the tokenization request.
        let payment_method =
            unified_connector_service::build_unified_connector_service_payment_method(
                router_data.request.payment_method_data.clone(),
                router_data.request.payment_method_type,
                router_data.payment_method_token.as_ref(),
            )?;

        let address = payments_grpc::PaymentAddress::foreign_try_from(router_data.address.clone())?;

        let amount = router_data.request.amount.ok_or(report!(
            UnifiedConnectorServiceError::MissingRequiredField {
                field_name: "amount"
            }
        ))?;

        Ok(Self {
            request_ref_id: Some(connector_ref_id),
            merchant_account_metadata,
            amount,
            currency: currency.into(),
            minor_amount: amount,
            payment_method: Some(payment_method),
            customer_name: None,
            email: None,
            customer_id: None,
            address: Some(address),
            metadata: None,
            connector_metadata: None,
            return_url: router_data.request.router_return_url.clone(),
            test_mode: router_data.test_mode,
        })
    }
}

impl
    transformers::ForeignTryFrom<(
        &RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
        common_enums::CallConnectorAction,
    )> for payments_grpc::PaymentServiceAuthorizeOnlyRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        (router_data, _call_connector_action): (
            &RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
            common_enums::CallConnectorAction,
        ),
    ) -> Result<Self, Self::Error> {
        let currency = payments_grpc::Currency::foreign_try_from(router_data.request.currency)?;

        let payment_method =
            unified_connector_service::build_unified_connector_service_payment_method(
                router_data.request.payment_method_data.clone(),
                router_data.request.payment_method_type,
                router_data.payment_method_token.as_ref(),
            )?;

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
        let merchant_account_metadata = router_data
            .connector_meta_data
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
            .map(|s| s.into());
        let metadata = router_data
            .request
            .metadata
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
            .map(|s| s.into());
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

        let state = router_data
            .access_token
            .as_ref()
            .map(ConnectorState::foreign_from);

        Ok(Self {
            connector_order_reference_id: router_data.request.order_id.clone(),
            amount: router_data.request.amount,
            currency: currency.into(),
            payment_method: Some(payment_method),
            return_url: router_data.request.router_return_url.clone(),
            address: Some(address),
            auth_type: auth_type.into(),
            enrolled_for_3ds: Some(router_data.request.enrolled_for_3ds),
            request_incremental_authorization: Some(
                router_data.request.request_incremental_authorization,
            ),
            minor_amount: router_data.request.amount,
            email: router_data
                .request
                .email
                .clone()
                .map(|e| e.expose().expose().into()),
            browser_info,

            session_token: router_data.session_token.clone(),
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
            setup_future_usage: setup_future_usage.map(|s| s.into()),
            off_session: router_data.request.off_session,
            customer_acceptance,
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
            customer_id: router_data
                .request
                .guest_customer
                .as_ref()
                .map(|guest| guest.customer_id.clone())
                .or_else(|| {
                    router_data
                        .request
                        .customer_id
                        .as_ref()
                        .map(|id| id.get_string_repr().to_string())
                }),
            metadata,
            test_mode: router_data.test_mode,
            connector_customer_id: router_data.connector_customer.clone(),
            state,
            payment_method_token: router_data
                .payment_method_token
                .as_ref()
                .and_then(|payment_method_token| payment_method_token.get_payment_method_token())
                .map(|payment_method_token| {
                    unified_connector_service_masking::Secret::new(payment_method_token.expose())
                }),
            merchant_account_metadata,
            description: router_data.description.clone(),
            setup_mandate_details: router_data
                .request
                .setup_mandate_details
                .as_ref()
                .map(payments_grpc::SetupMandateDetails::foreign_try_from)
                .transpose()?,
            statement_descriptor_name: router_data
                .request
                .billing_descriptor
                .as_ref()
                .and_then(|descriptor| descriptor.statement_descriptor.clone()),
            statement_descriptor_suffix: router_data
                .request
                .billing_descriptor
                .as_ref()
                .and_then(|descriptor| descriptor.statement_descriptor_suffix.clone()),
            order_details: vec![],
            enable_partial_authorization: router_data
                .request
                .enable_partial_authorization
                .map(|e| e.is_true()),
            billing_descriptor: router_data
                .request
                .billing_descriptor
                .as_ref()
                .map(payments_grpc::BillingDescriptor::foreign_from),
            payment_channel: router_data
                .request
                .payment_channel
                .as_ref()
                .map(payments_grpc::PaymentChannel::foreign_try_from)
                .transpose()?
                .map(|payment_channel| payment_channel.into()),
            connector_metadata: None,
            locale: router_data.request.locale.clone(),
            continue_redirection_url: router_data.request.complete_authorize_url.clone(),
            redirection_response: None,
            threeds_completion_indicator: None,
            tokenization_strategy: router_data
                .request
                .tokenization
                .map(payments_grpc::Tokenization::foreign_from)
                .map(Into::into),
        })
    }
}

impl
    transformers::ForeignTryFrom<(
        &RouterData<
            api::CompleteAuthorize,
            router_request_types::CompleteAuthorizeData,
            PaymentsResponseData,
        >,
        common_enums::CallConnectorAction,
    )> for payments_grpc::PaymentServiceAuthorizeOnlyRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        (router_data, _call_connector_action): (
            &RouterData<
                api::CompleteAuthorize,
                router_request_types::CompleteAuthorizeData,
                PaymentsResponseData,
            >,
            common_enums::CallConnectorAction,
        ),
    ) -> Result<Self, Self::Error> {
        let currency = payments_grpc::Currency::foreign_try_from(router_data.request.currency)?;

        let payment_method = router_data
            .request
            .payment_method_data
            .clone()
            .map(|payment_method_data| {
                unified_connector_service::build_unified_connector_service_payment_method(
                    payment_method_data,
                    router_data.request.payment_method_type,
                    router_data.payment_method_token.as_ref(),
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

        // First try to get authentication_data from the request
        let authentication_data = router_data
            .request
            .authentication_data
            .clone()
            .map(payments_grpc::AuthenticationData::foreign_try_from)
            .transpose()?;

        // If not in request, try to extract from connector_meta (stored during Authorize flow)
        let authentication_data = if authentication_data.is_none() {
            router_data
                .request
                .connector_meta
                .as_ref()
                .and_then(|metadata| metadata.get("authentication_data"))
                .and_then(|value| {
                    serde_json::from_value::<router_request_types::UcsAuthenticationData>(
                        value.clone(),
                    )
                    .ok()
                })
                .map(payments_grpc::AuthenticationData::foreign_try_from)
                .transpose()?
        } else {
            authentication_data
        };

        let merchant_account_metadata = router_data
            .connector_meta_data
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
            .map(|s| s.into());
        let metadata = router_data
            .request
            .metadata
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
            .map(|s| s.into());
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

        let state = router_data
            .access_token
            .as_ref()
            .map(ConnectorState::foreign_from);

        Ok(Self {
            connector_order_reference_id: None,
            amount: router_data.request.amount,
            currency: currency.into(),
            payment_method,
            return_url: router_data.request.router_return_url.clone(),
            address: Some(address),
            auth_type: auth_type.into(),
            enrolled_for_3ds: Some(true),
            request_incremental_authorization: Some(
                router_data.request.request_incremental_authorization,
            ),
            minor_amount: router_data.request.amount,
            email: router_data
                .request
                .email
                .clone()
                .map(|e| e.expose().expose().into()),
            browser_info,
            session_token: router_data.session_token.clone(),
            order_tax_amount: None,
            customer_name: None,
            capture_method: capture_method.map(|capture_method| capture_method.into()),
            webhook_url: None,
            complete_authorize_url: router_data.request.complete_authorize_url.clone(),
            setup_future_usage: setup_future_usage.map(|s| s.into()),
            off_session: router_data.request.off_session,
            customer_acceptance,
            order_category: None,
            payment_experience: None,
            authentication_data,
            request_extended_authorization: None,
            merchant_order_reference_id: None,
            shipping_cost: None,
            request_ref_id: Some(Identifier {
                id_type: Some(payments_grpc::identifier::IdType::Id(
                    router_data.connector_request_reference_id.clone(),
                )),
            }),
            customer_id: None,
            metadata,
            test_mode: router_data.test_mode,
            connector_customer_id: router_data.connector_customer.clone(),
            state,
            payment_method_token: router_data
                .payment_method_token
                .as_ref()
                .and_then(|payment_method_token| payment_method_token.get_payment_method_token())
                .map(|payment_method_token| {
                    unified_connector_service_masking::Secret::new(payment_method_token.expose())
                }),
            merchant_account_metadata,
            description: router_data.description.clone(),
            setup_mandate_details: router_data
                .request
                .setup_mandate_details
                .as_ref()
                .map(payments_grpc::SetupMandateDetails::foreign_try_from)
                .transpose()?,
            statement_descriptor_name: None,
            statement_descriptor_suffix: None,
            order_details: vec![],
            connector_metadata: None,
            enable_partial_authorization: None,
            payment_channel: None,
            billing_descriptor: None,
            locale: None,
            continue_redirection_url: router_data.request.complete_authorize_url.clone(),
            redirection_response: router_data
                .request
                .redirect_response
                .clone()
                .map(|redirection_response| {
                    payments_grpc::RedirectionResponse::foreign_try_from(redirection_response)
                })
                .transpose()?,
            threeds_completion_indicator: router_data
                .request
                .threeds_method_comp_ind
                .clone()
                .map(|ind| payments_grpc::ThreeDsCompletionIndicator::foreign_from(ind).into()),
            tokenization_strategy: router_data
                .request
                .tokenization
                .map(payments_grpc::Tokenization::foreign_from)
                .map(Into::into),
        })
    }
}

impl
    transformers::ForeignTryFrom<
        &RouterData<
            api::CreateOrder,
            router_request_types::CreateOrderRequestData,
            PaymentsResponseData,
        >,
    > for payments_grpc::PaymentServiceCreateOrderRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;
    fn foreign_try_from(
        router_data: &RouterData<
            api::CreateOrder,
            router_request_types::CreateOrderRequestData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let currency = payments_grpc::Currency::foreign_try_from(router_data.request.currency)?;

        // Populate state with access token if available (same pattern as Authorize flow)
        let state = router_data
            .access_token
            .as_ref()
            .map(ConnectorState::foreign_from);

        let merchant_account_metadata = router_data
            .connector_meta_data
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
            .map(|s| s.into());

        Ok(Self {
            request_ref_id: Some(Identifier {
                id_type: Some(payments_grpc::identifier::IdType::Id(
                    router_data.connector_request_reference_id.clone(),
                )),
            }),
            amount: router_data.request.minor_amount.get_amount_as_i64(),
            currency: currency.into(),
            metadata: None,
            webhook_url: None,
            connector_metadata: None,
            merchant_account_metadata,
            state,
            test_mode: router_data.test_mode,
            payment_method_type: router_data
                .payment_method_type
                .map(payments_grpc::PaymentMethodType::foreign_try_from)
                .transpose()?
                .map(|payment_method_type| payment_method_type.into()),
        })
    }
}

impl
    transformers::ForeignTryFrom<(
        &RouterData<
            hyperswitch_domain_models::router_flow_types::payments::CreateConnectorCustomer,
            router_request_types::ConnectorCustomerData,
            PaymentsResponseData,
        >,
        common_enums::CallConnectorAction,
    )> for payments_grpc::PaymentServiceCreateConnectorCustomerRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        (router_data, _call_connector_action): (
            &RouterData<
                hyperswitch_domain_models::router_flow_types::payments::CreateConnectorCustomer,
                router_request_types::ConnectorCustomerData,
                PaymentsResponseData,
            >,
            common_enums::CallConnectorAction,
        ),
    ) -> Result<Self, Self::Error> {
        let request_ref_id = router_data.connector_request_reference_id.clone();
        let address = payments_grpc::PaymentAddress::foreign_try_from(router_data.address.clone())?;

        let merchant_account_metadata = router_data
            .connector_meta_data
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
            .map(|s| s.into());

        Ok(Self {
            request_ref_id: Some(Identifier {
                id_type: Some(payments_grpc::identifier::IdType::Id(request_ref_id)),
            }),
            merchant_account_metadata,
            customer_name: router_data
                .request
                .name
                .clone()
                .map(ExposeInterface::expose),
            email: router_data
                .request
                .email
                .clone()
                .map(|e| e.expose().expose().into()),
            customer_id: router_data
                .customer_id
                .clone()
                .map(|id| id.get_string_repr().to_string()),
            phone_number: router_data
                .request
                .phone
                .as_ref()
                .map(|phone| phone.peek().to_string()),
            address: Some(address),
            metadata: None,
            connector_metadata: None,
            test_mode: router_data.test_mode,
        })
    }
}

impl
    transformers::ForeignTryFrom<(
        &RouterData<PSync, PaymentsSyncData, PaymentsResponseData>,
        common_enums::CallConnectorAction,
    )> for payments_grpc::PaymentServiceGetRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        (router_data, call_connector_action): (
            &RouterData<PSync, PaymentsSyncData, PaymentsResponseData>,
            common_enums::CallConnectorAction,
        ),
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
        let connector_order_reference_id = router_data.request.connector_reference_id.clone();

        let request_ref_id = Some(Identifier {
            id_type: Some(payments_grpc::identifier::IdType::Id(
                router_data.connector_request_reference_id.clone(),
            )),
        });

        let currency = payments_grpc::Currency::foreign_try_from(router_data.request.currency)?;

        let handle_response = match call_connector_action {
            common_enums::CallConnectorAction::UCSHandleResponse(res) => Some(res),
            common_enums::CallConnectorAction::Trigger => None,
            common_enums::CallConnectorAction::HandleResponse(_)
            | common_enums::CallConnectorAction::UCSConsumeResponse(_)
            | common_enums::CallConnectorAction::Avoid
            | common_enums::CallConnectorAction::StatusUpdate { .. } => Err(
                UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                    "Invalid CallConnectorAction for payment sync call via UCS Gateway system"
                        .to_string(),
                ),
            )?,
        };

        let capture_method = router_data
            .request
            .capture_method
            .map(payments_grpc::CaptureMethod::foreign_try_from)
            .transpose()?;

        let state = router_data
            .access_token
            .as_ref()
            .map(ConnectorState::foreign_from);

        let setup_future_usage = router_data
            .request
            .setup_future_usage
            .map(payments_grpc::FutureUsage::foreign_try_from)
            .transpose()?;

        let merchant_account_metadata = router_data
            .connector_meta_data
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
            .map(|s| s.into());

        Ok(Self {
            transaction_id: connector_transaction_id,
            encoded_data: router_data.request.encoded_data.clone(),
            request_ref_id,
            capture_method: capture_method.map(|capture_method| capture_method.into()),
            handle_response,
            setup_future_usage: setup_future_usage.map(|s| s.into()),
            connector_order_reference_id,
            amount: router_data.request.amount.get_amount_as_i64(),
            currency: currency.into(),
            state,
            connector_metadata: router_data
                .request
                .connector_meta
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
                .map(|s| s.into()),
            sync_type: Some(
                payments_grpc::SyncRequestType::foreign_from(&router_data.request.sync_type).into(),
            ),
            merchant_account_metadata,
            metadata: None,
            test_mode: router_data.test_mode,
            payment_experience: router_data
                .request
                .payment_experience
                .map(payments_grpc::PaymentExperience::foreign_from)
                .map(Into::into),
        })
    }
}

impl ForeignFrom<common_enums::PaymentExperience> for payments_grpc::PaymentExperience {
    fn foreign_from(tokenization: common_enums::PaymentExperience) -> Self {
        match tokenization {
            common_enums::PaymentExperience::RedirectToUrl => Self::RedirectToUrl,
            common_enums::PaymentExperience::InvokeSdkClient => Self::InvokeSdkClient,
            common_enums::PaymentExperience::DisplayQrCode => Self::DisplayQrCode,
            common_enums::PaymentExperience::OneClick => Self::OneClick,
            common_enums::PaymentExperience::LinkWallet => Self::LinkWallet,
            common_enums::PaymentExperience::InvokePaymentApp => Self::InvokePaymentApp,
            common_enums::PaymentExperience::DisplayWaitScreen => Self::DisplayWaitScreen,
            common_enums::PaymentExperience::CollectOtp => Self::CollectOtp,
        }
    }
}

impl
    transformers::ForeignTryFrom<
        &RouterData<
            api::AuthorizeSessionToken,
            router_request_types::AuthorizeSessionTokenData,
            PaymentsResponseData,
        >,
    > for payments_grpc::PaymentServiceCreateSessionTokenRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;
    fn foreign_try_from(
        router_data: &RouterData<
            api::AuthorizeSessionToken,
            router_request_types::AuthorizeSessionTokenData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let currency = payments_grpc::Currency::foreign_try_from(router_data.request.currency)?;

        let merchant_account_metadata = router_data
            .connector_meta_data
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
            .map(|s| s.into());

        Ok(Self {
            request_ref_id: Some(Identifier {
                id_type: Some(payments_grpc::identifier::IdType::Id(
                    router_data.connector_request_reference_id.clone(),
                )),
            }),
            amount: router_data
                .request
                .amount
                .ok_or(report!(UnifiedConnectorServiceError::RequestEncodingFailed))?,
            currency: currency.into(),
            minor_amount: router_data
                .request
                .amount
                .ok_or(report!(UnifiedConnectorServiceError::RequestEncodingFailed))?,
            metadata: None,
            state: None,
            browser_info: None,
            connector_metadata: None,
            merchant_account_metadata,
            test_mode: router_data.test_mode,
        })
    }
}

impl
    transformers::ForeignTryFrom<
        &RouterData<
            uas_flows::Authenticate,
            router_request_types::PaymentsAuthenticateData,
            PaymentsResponseData,
        >,
    > for payments_grpc::PaymentServiceAuthenticateRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;
    fn foreign_try_from(
        router_data: &RouterData<
            uas_flows::Authenticate,
            router_request_types::PaymentsAuthenticateData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let currency = payments_grpc::Currency::foreign_try_from(
            router_data.request.currency.unwrap_or_default(),
        )?;

        let payment_method = router_data
            .request
            .payment_method_data
            .clone()
            .map(|payment_method_data| {
                unified_connector_service::build_unified_connector_service_payment_method(
                    payment_method_data,
                    router_data.request.payment_method_type,
                    router_data.payment_method_token.as_ref(),
                )
            })
            .transpose()?;

        let capture_method = router_data
            .request
            .capture_method
            .map(payments_grpc::CaptureMethod::foreign_try_from)
            .transpose()?;
        let address = payments_grpc::PaymentAddress::foreign_try_from(router_data.address.clone())?;
        let merchant_account_metadata = router_data
            .connector_meta_data
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
            .map(|s| s.into());

        // Convert ucs_authentication_data from PreAuthenticate response to gRPC format
        let authentication_data = router_data
            .request
            .authentication_data
            .clone()
            .map(payments_grpc::AuthenticationData::foreign_try_from)
            .transpose()?;

        Ok(Self {
            request_ref_id: Some(Identifier {
                id_type: Some(payments_grpc::identifier::IdType::Id(
                    router_data.connector_request_reference_id.clone(),
                )),
            }),
            amount: router_data.request.amount.unwrap_or(0),
            currency: currency.into(),
            minor_amount: router_data
                .request
                .minor_amount
                .map(|amount| amount.get_amount_as_i64())
                .unwrap_or(0),
            payment_method,
            email: router_data
                .request
                .email
                .clone()
                .map(|e| e.expose().expose().into()),
            customer_name: None,
            address: Some(address),
            authentication_data,
            metadata: None,
            return_url: None,
            continue_redirection_url: router_data.request.complete_authorize_url.clone(),
            state: None,
            redirection_response: router_data
                .request
                .redirect_response
                .clone()
                .map(|redirection_response| {
                    payments_grpc::RedirectionResponse::foreign_try_from(redirection_response)
                })
                .transpose()?,
            merchant_account_metadata,
            browser_info: router_data
                .request
                .browser_info
                .clone()
                .map(payments_grpc::BrowserInformation::foreign_try_from)
                .transpose()?,
            connector_metadata: None,
            capture_method: capture_method.map(|capture_method| capture_method.into()),
        })
    }
}

impl
    transformers::ForeignTryFrom<
        &RouterData<
            uas_flows::PostAuthenticate,
            router_request_types::PaymentsPostAuthenticateData,
            PaymentsResponseData,
        >,
    > for payments_grpc::PaymentServicePostAuthenticateRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;
    fn foreign_try_from(
        router_data: &RouterData<
            uas_flows::PostAuthenticate,
            router_request_types::PaymentsPostAuthenticateData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let currency = payments_grpc::Currency::foreign_try_from(
            router_data.request.currency.unwrap_or_default(),
        )?;

        let address = payments_grpc::PaymentAddress::foreign_try_from(router_data.address.clone())?;

        let payment_method = router_data
            .request
            .payment_method_data
            .clone()
            .map(|payment_method_data| {
                unified_connector_service::build_unified_connector_service_payment_method(
                    payment_method_data,
                    router_data.request.payment_method_type,
                    router_data.payment_method_token.as_ref(),
                )
            })
            .transpose()?;
        let merchant_account_metadata = router_data
            .connector_meta_data
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
            .map(|s| s.into());
        Ok(Self {
            request_ref_id: Some(Identifier {
                id_type: Some(payments_grpc::identifier::IdType::Id(
                    router_data.connector_request_reference_id.clone(),
                )),
            }),
            amount: router_data.request.amount.unwrap_or(0),
            currency: currency.into(),
            minor_amount: router_data
                .request
                .minor_amount
                .map(|amount| amount.get_amount_as_i64())
                .unwrap_or(0),
            payment_method,
            email: router_data
                .request
                .email
                .clone()
                .map(|e| e.expose().expose().into()),
            customer_name: None,
            address: Some(address),
            authentication_data: None,
            metadata: None,
            return_url: None,
            continue_redirection_url: None,
            state: None,
            merchant_account_metadata,
            redirection_response: router_data
                .request
                .redirect_response
                .clone()
                .map(|redirection_response| {
                    payments_grpc::RedirectionResponse::foreign_try_from(redirection_response)
                })
                .transpose()?,
            browser_info: router_data
                .request
                .browser_info
                .clone()
                .map(payments_grpc::BrowserInformation::foreign_try_from)
                .transpose()?,
            connector_metadata: None,
            connector_order_reference_id: None,
        })
    }
}

impl
    transformers::ForeignTryFrom<
        &RouterData<
            uas_flows::PreAuthenticate,
            router_request_types::PaymentsPreAuthenticateData,
            PaymentsResponseData,
        >,
    > for payments_grpc::PaymentServicePreAuthenticateRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;
    fn foreign_try_from(
        router_data: &RouterData<
            uas_flows::PreAuthenticate,
            router_request_types::PaymentsPreAuthenticateData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let currency = payments_grpc::Currency::foreign_try_from(
            router_data.request.currency.unwrap_or_default(),
        )?;

        let payment_method =
            unified_connector_service::build_unified_connector_service_payment_method(
                router_data.request.payment_method_data.clone(),
                router_data.request.payment_method_type,
                router_data.payment_method_token.as_ref(),
            )?;

        let capture_method = router_data
            .request
            .capture_method
            .map(payments_grpc::CaptureMethod::foreign_try_from)
            .transpose()?;

        let address = payments_grpc::PaymentAddress::foreign_try_from(router_data.address.clone())?;
        let merchant_account_metadata = router_data
            .connector_meta_data
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
            .map(|s| s.into());
        let metadata = router_data
            .request
            .metadata
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
            .map(|s| s.into());
        let amount = router_data.request.amount;
        let minor_amount = router_data.request.minor_amount;

        Ok(Self {
            request_ref_id: Some(Identifier {
                id_type: Some(payments_grpc::identifier::IdType::Id(
                    router_data.connector_request_reference_id.clone(),
                )),
            }),
            amount,
            currency: currency.into(),
            minor_amount: minor_amount.get_amount_as_i64(),
            payment_method: Some(payment_method),
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
            address: Some(address),
            enrolled_for_3ds: router_data.request.enrolled_for_3ds,
            metadata,
            return_url: router_data.request.router_return_url.clone(),
            continue_redirection_url: router_data.request.complete_authorize_url.clone(),
            state: None,
            merchant_account_metadata,
            browser_info: router_data
                .request
                .browser_info
                .clone()
                .map(payments_grpc::BrowserInformation::foreign_try_from)
                .transpose()?,
            connector_metadata: None,
            capture_method: capture_method.map(|capture_method| capture_method.into()),
            description: router_data.description.clone(),
        })
    }
}

impl transformers::ForeignTryFrom<&RouterData<Capture, PaymentsCaptureData, PaymentsResponseData>>
    for payments_grpc::PaymentServiceCaptureRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        router_data: &RouterData<Capture, PaymentsCaptureData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let connector_transaction_id = router_data.request.connector_transaction_id.clone();

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

        let state = router_data
            .access_token
            .as_ref()
            .map(ConnectorState::foreign_from);

        let merchant_account_metadata = router_data
            .connector_meta_data
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
            .map(|s| s.into());

        Ok(Self {
            transaction_id: Some(Identifier {
                id_type: Some(payments_grpc::identifier::IdType::Id(
                    connector_transaction_id,
                )),
            }),
            request_ref_id: Some(Identifier {
                id_type: Some(payments_grpc::identifier::IdType::Id(
                    router_data.connector_request_reference_id.clone(),
                )),
            }),
            amount_to_capture: router_data
                .request
                .minor_amount_to_capture
                .get_amount_as_i64(),
            currency: currency.into(),
            capture_method: capture_method.map(|capture_method| capture_method.into()),
            metadata: router_data
                .request
                .metadata
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
                .map(|s| s.into()),
            browser_info,
            multiple_capture_data: router_data.request.multiple_capture_data.as_ref().map(
                |multiple_capture_request_data| payments_grpc::MultipleCaptureRequestData {
                    capture_sequence: multiple_capture_request_data.capture_sequence.into(),
                    capture_reference: multiple_capture_request_data.capture_reference.clone(),
                },
            ),
            state,
            connector_metadata: router_data
                .request
                .connector_meta
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
                .map(|s| s.into()),
            merchant_account_metadata,
            test_mode: router_data.test_mode,
            merchant_order_reference_id: router_data.request.merchant_order_reference_id.clone(),
        })
    }
}

impl
    transformers::ForeignTryFrom<
        &RouterData<
            api::CompleteAuthorize,
            crate::types::CompleteAuthorizeData,
            PaymentsResponseData,
        >,
    > for payments_grpc::PaymentServiceAuthorizeRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        router_data: &RouterData<
            api::CompleteAuthorize,
            crate::types::CompleteAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let currency = payments_grpc::Currency::foreign_try_from(router_data.request.currency)?;

        let payment_method = router_data
            .request
            .payment_method_data
            .clone()
            .map(|payment_method_data| {
                unified_connector_service::build_unified_connector_service_payment_method(
                    payment_method_data,
                    router_data.request.payment_method_type,
                    router_data.payment_method_token.as_ref(),
                )
            })
            .transpose()?;

        let address = payments_grpc::PaymentAddress::foreign_try_from(router_data.address.clone())?;

        let auth_type =
            payments_grpc::AuthenticationType::foreign_try_from(AuthenticationType::NoThreeDs)?;

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

        let merchant_account_metadata = router_data
            .connector_meta_data
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
            .map(|s| s.into());
        let metadata = router_data
            .request
            .metadata
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
            .map(|s| s.into());
        let authentication_data = router_data
            .request
            .authentication_data
            .clone()
            .map(payments_grpc::AuthenticationData::foreign_try_from)
            .transpose()?;

        Ok(Self {
            amount: router_data.request.amount,
            currency: currency.into(),
            billing_descriptor: None,
            payment_method,
            return_url: router_data.request.complete_authorize_url.clone(),
            address: Some(address),
            auth_type: auth_type.into(),
            enrolled_for_3ds: Some(false),
            request_incremental_authorization: Some(false),
            minor_amount: router_data.request.minor_amount.get_amount_as_i64(),
            email: router_data
                .request
                .email
                .clone()
                .map(|e| e.expose().expose().into()),
            browser_info,
            locale: None,
            session_token: None,
            order_tax_amount: None,
            customer_name: None,
            capture_method: capture_method.map(|capture_method| capture_method.into()),
            webhook_url: None, // CompleteAuthorize doesn't have webhook_url
            complete_authorize_url: router_data.request.complete_authorize_url.clone(),
            setup_future_usage: router_data
                .request
                .setup_future_usage
                .map(payments_grpc::FutureUsage::foreign_try_from)
                .transpose()?
                .map(|s| s.into()),
            off_session: router_data.request.off_session,
            customer_acceptance: router_data
                .request
                .customer_acceptance
                .clone()
                .map(payments_grpc::CustomerAcceptance::foreign_try_from)
                .transpose()?,
            order_category: None,
            payment_experience: None,
            authentication_data,
            request_extended_authorization: None,
            merchant_order_reference_id: None,
            shipping_cost: None,
            request_ref_id: Some(Identifier {
                id_type: Some(payments_grpc::identifier::IdType::Id(
                    router_data.connector_request_reference_id.clone(),
                )),
            }),
            customer_id: None,
            metadata,
            test_mode: router_data.test_mode,
            connector_customer_id: router_data.connector_customer.clone(),
            merchant_account_metadata,
            state: None,
            description: None,
            setup_mandate_details: None,
            statement_descriptor_name: None,
            statement_descriptor_suffix: None,
            order_details: vec![],
            connector_metadata: None,
            connector_order_reference_id: Some(router_data.connector_request_reference_id.clone()),
            enable_partial_authorization: None,
            payment_channel: None,
            tokenization_strategy: router_data
                .request
                .tokenization
                .map(payments_grpc::Tokenization::foreign_from)
                .map(Into::into),
        })
    }
}

impl
    transformers::ForeignTryFrom<
        &RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
    > for payments_grpc::PaymentServiceAuthorizeRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        router_data: &RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let currency = payments_grpc::Currency::foreign_try_from(router_data.request.currency)?;

        let payment_method =
            unified_connector_service::build_unified_connector_service_payment_method(
                router_data.request.payment_method_data.clone(),
                router_data.request.payment_method_type,
                router_data.payment_method_token.as_ref(),
            )?;

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
        let merchant_account_metadata = router_data
            .connector_meta_data
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
            .map(|s| s.into());
        let metadata = router_data
            .request
            .metadata
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
            .map(|s| s.into());
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

        let state = router_data
            .access_token
            .as_ref()
            .map(ConnectorState::foreign_from);

        Ok(Self {
            amount: router_data.request.amount,
            currency: currency.into(),
            payment_method: Some(payment_method),
            return_url: router_data.request.router_return_url.clone(),
            address: Some(address),
            auth_type: auth_type.into(),
            enrolled_for_3ds: Some(router_data.request.enrolled_for_3ds),
            request_incremental_authorization: Some(
                router_data.request.request_incremental_authorization,
            ),
            minor_amount: router_data.request.amount,
            email: router_data
                .request
                .email
                .clone()
                .map(|e| e.expose().expose().into()),
            browser_info,
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
            setup_future_usage: setup_future_usage.map(|s| s.into()),
            off_session: router_data.request.off_session,
            customer_acceptance,
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
            customer_id: router_data
                .request
                .guest_customer
                .as_ref()
                .map(|guest| guest.customer_id.clone())
                .or_else(|| {
                    router_data
                        .request
                        .customer_id
                        .as_ref()
                        .map(|id| id.get_string_repr().to_string())
                }),
            metadata,
            test_mode: router_data.test_mode,
            connector_customer_id: router_data.connector_customer.clone(),
            state,
            merchant_account_metadata,
            description: router_data.description.clone(),
            setup_mandate_details: router_data
                .request
                .setup_mandate_details
                .as_ref()
                .map(payments_grpc::SetupMandateDetails::foreign_try_from)
                .transpose()?,
            statement_descriptor_name: router_data
                .request
                .billing_descriptor
                .as_ref()
                .and_then(|descriptor| descriptor.statement_descriptor.clone()),
            statement_descriptor_suffix: router_data
                .request
                .billing_descriptor
                .as_ref()
                .and_then(|descriptor| descriptor.statement_descriptor_suffix.clone()),
            order_details: vec![],
            connector_metadata: None,
            connector_order_reference_id: router_data.request.order_id.clone(),
            enable_partial_authorization: router_data
                .request
                .enable_partial_authorization
                .map(|e| e.is_true()),
            billing_descriptor: router_data
                .request
                .billing_descriptor
                .as_ref()
                .map(payments_grpc::BillingDescriptor::foreign_from),
            payment_channel: router_data
                .request
                .payment_channel
                .as_ref()
                .map(payments_grpc::PaymentChannel::foreign_try_from)
                .transpose()?
                .map(|payment_channel| payment_channel.into()),
            locale: router_data.request.locale.clone(),
            tokenization_strategy: router_data
                .request
                .tokenization
                .map(payments_grpc::Tokenization::foreign_from)
                .map(Into::into),
        })
    }
}

impl
    transformers::ForeignTryFrom<
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

        let payment_method = Some(
            unified_connector_service::build_unified_connector_service_payment_method_for_external_proxy(
                router_data.request.payment_method_data.clone(),
                router_data.request.payment_method_type,
            )?
        );

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
            amount: router_data.request.amount,
            currency: currency.into(),
            billing_descriptor: None,
            payment_method,
            return_url: router_data.request.router_return_url.clone(),
            address: Some(address),
            auth_type: auth_type.into(),
            enrolled_for_3ds: Some(router_data.request.enrolled_for_3ds),
            request_incremental_authorization: Some(
                router_data.request.request_incremental_authorization,
            ),
            minor_amount: router_data.request.amount,
            email: router_data
                .request
                .email
                .clone()
                .map(|e| e.expose().expose().into()),
            browser_info,
            locale: None,
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
            setup_future_usage: setup_future_usage.map(|s| s.into()),
            off_session: router_data.request.off_session,
            customer_acceptance,
            order_category: router_data.request.order_category.clone(),
            payment_experience: None,
            authentication_data,
            request_extended_authorization: router_data
                .request
                .request_extended_authorization
                .map(|request_extended_authorization| request_extended_authorization.is_true()),
            merchant_order_reference_id: router_data
                .request
                .merchant_order_reference_id
                .as_ref()
                .map(|merchant_order_reference_id| {
                    merchant_order_reference_id.get_string_repr().to_string()
                }),
            shipping_cost: router_data
                .request
                .shipping_cost
                .map(|shipping_cost| shipping_cost.get_amount_as_i64()),
            request_ref_id: Some(Identifier {
                id_type: Some(payments_grpc::identifier::IdType::Id(
                    router_data.connector_request_reference_id.clone(),
                )),
            }),
            customer_id: router_data
                .request
                .customer_id
                .as_ref()
                .map(|id| id.get_string_repr().to_string()),
            metadata: router_data
                .request
                .metadata
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
                .map(|s| s.into()),
            merchant_account_metadata: router_data
                .connector_meta_data
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
                .map(|s| s.into()),
            test_mode: router_data.test_mode,
            connector_customer_id: router_data.connector_customer.clone(),
            state: None,
            description: router_data.description.clone(),
            setup_mandate_details: router_data
                .request
                .setup_mandate_details
                .as_ref()
                .map(payments_grpc::SetupMandateDetails::foreign_try_from)
                .transpose()?,
            statement_descriptor_name: router_data.request.statement_descriptor.clone(),
            statement_descriptor_suffix: router_data.request.statement_descriptor_suffix.clone(),
            order_details: vec![],
            connector_metadata: None,
            connector_order_reference_id: router_data.request.order_id.clone(),
            enable_partial_authorization: None,
            payment_channel: None,
            tokenization_strategy: None,
        })
    }
}

impl
    transformers::ForeignTryFrom<
        &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
    > for payments_grpc::PaymentServiceRegisterRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        router_data: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let currency = payments_grpc::Currency::foreign_try_from(router_data.request.currency)?;
        let payment_method =
            unified_connector_service::build_unified_connector_service_payment_method(
                router_data.request.payment_method_data.clone(),
                router_data.request.payment_method_type,
                router_data.payment_method_token.as_ref(),
            )?;
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

        // If this is the deprecated mandate_id flow, take customer_acceptance
        // from setup_mandate_details. Otherwise, take it from request.customer_acceptance.
        let customer_acceptance = match router_data.request.get_setup_mandate_details() {
            Some(mandate_data) => mandate_data
                .customer_acceptance
                .clone()
                .map(payments_grpc::CustomerAcceptance::foreign_try_from)
                .transpose(),
            None => router_data
                .request
                .customer_acceptance
                .clone()
                .map(payments_grpc::CustomerAcceptance::foreign_try_from)
                .transpose(),
        }?;

        let state = router_data
            .access_token
            .as_ref()
            .map(ConnectorState::foreign_from);

        Ok(Self {
            payment_method_token: router_data
                .payment_method_token
                .as_ref()
                .and_then(|payment_method_token| payment_method_token.get_payment_method_token())
                .map(|payment_method_token| {
                    unified_connector_service_masking::Secret::new(payment_method_token.expose())
                }),
            request_ref_id: Some(Identifier {
                id_type: Some(payments_grpc::identifier::IdType::Id(
                    router_data.connector_request_reference_id.clone(),
                )),
            }),
            currency: currency.into(),
            payment_method: Some(payment_method),
            minor_amount: Some(router_data.request.amount),
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
            customer_id: router_data
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
                .map(serde_json::to_string)
                .transpose()
                .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
                .map(|s| s.into()),
            return_url: router_data.request.router_return_url.clone(),
            webhook_url: router_data.request.webhook_url.clone(),
            complete_authorize_url: router_data.request.complete_authorize_url.clone(),

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
            merchant_account_metadata: router_data
                .connector_meta_data
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
                .map(|s| s.into()),
            connector_customer_id: router_data.connector_customer.clone(),
            state,
            order_id: None,
            connector_metadata: None,
            enable_partial_authorization: router_data
                .request
                .enable_partial_authorization
                .map(|e| e.is_true()),
            billing_descriptor: router_data
                .request
                .billing_descriptor
                .as_ref()
                .map(payments_grpc::BillingDescriptor::foreign_from),
            payment_channel: router_data
                .request
                .payment_channel
                .as_ref()
                .map(payments_grpc::PaymentChannel::foreign_try_from)
                .transpose()?
                .map(|payment_channel| payment_channel.into()),
            locale: None,
            connector_testing_data: router_data.request.connector_testing_data.as_ref().map(
                |data| unified_connector_service_masking::Secret::new(data.peek().to_string()),
            ),
        })
    }
}

impl
    transformers::ForeignTryFrom<
        &RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
    > for payments_grpc::PaymentServiceRepeatEverythingRequest
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

        let payment_method_type = router_data
            .request
            .payment_method_type
            .map(payments_grpc::PaymentMethodType::foreign_try_from)
            .transpose()?
            .map(|pm_type| pm_type.into());

        let address = payments_grpc::PaymentAddress::foreign_try_from(router_data.address.clone())?;
        let mandate_reference_id = match &router_data.request.mandate_id {
            Some(mandate) => match &mandate.mandate_reference_id {
                Some(api_models::payments::MandateReferenceId::ConnectorMandateId(
                    connector_mandate_id,
                )) => Some(payments_grpc::MandateReferenceId {
                    mandate_id_type: Some(
                        payments_grpc::mandate_reference_id::MandateIdType::ConnectorMandateId(
                            payments_grpc::ConnectorMandateReferenceId {
                                connector_mandate_id: connector_mandate_id
                                    .get_connector_mandate_id(),
                                payment_method_id: connector_mandate_id.get_payment_method_id(),
                                connector_mandate_request_reference_id: connector_mandate_id
                                    .get_connector_mandate_request_reference_id(),
                            },
                        ),
                    ),
                }),
                Some(api_models::payments::MandateReferenceId::NetworkMandateId(
                    network_mandate_id,
                )) => Some(payments_grpc::MandateReferenceId {
                    mandate_id_type: Some(
                        payments_grpc::mandate_reference_id::MandateIdType::NetworkMandateId(
                            network_mandate_id.clone(),
                        ),
                    ),
                }),
                Some(api_models::payments::MandateReferenceId::NetworkTokenWithNTI(
                    network_token_with_nti,
                )) => Some(payments_grpc::MandateReferenceId {
                    mandate_id_type: Some(
                        payments_grpc::mandate_reference_id::MandateIdType::NetworkTokenWithNti(
                            payments_grpc::NetworkTokenWithNti {
                                network_transaction_id: network_token_with_nti
                                    .network_transaction_id
                                    .clone(),
                                token_exp_month: network_token_with_nti
                                    .token_exp_month
                                    .clone()
                                    .map(|exp| exp.expose().into()),
                                token_exp_year: network_token_with_nti
                                    .token_exp_year
                                    .clone()
                                    .map(|exp| exp.expose().into()),
                            },
                        ),
                    ),
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

        let state = router_data
            .access_token
            .as_ref()
            .map(ConnectorState::foreign_from);

        let payment_method_data = match router_data.request.payment_method_data.clone() {
            hyperswitch_domain_models::payment_method_data::PaymentMethodData::MandatePayment => {
                None
            }
            payment_method_data => Some(payment_method_data),
        };
        let payment_method = payment_method_data
            .map(|payment_method_data| {
                unified_connector_service::build_unified_connector_service_payment_method(
                    payment_method_data,
                    router_data.request.payment_method_type,
                    router_data.payment_method_token.as_ref(),
                )
            })
            .transpose()?;

        let authentication_data = router_data
            .request
            .authentication_data
            .clone()
            .map(payments_grpc::AuthenticationData::foreign_try_from)
            .transpose()?;

        let recurring_mandate_payment_data = router_data
            .recurring_mandate_payment_data
            .as_ref()
            .map(payments_grpc::RecurringMandatePaymentData::foreign_try_from)
            .transpose()?;

        Ok(Self {
            request_ref_id: Some(Identifier {
                id_type: Some(payments_grpc::identifier::IdType::Id(
                    router_data.connector_request_reference_id.clone(),
                )),
            }),
            payment_method,
            mandate_reference_id,
            amount: router_data.request.amount,
            currency: currency.into(),
            minor_amount: router_data.request.amount,
            merchant_order_reference_id: router_data.request.merchant_order_reference_id.clone(),
            metadata: router_data
                .request
                .metadata
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
                .map(|s| s.into()),
            merchant_account_metadata: router_data
                .connector_meta_data
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
                .map(|s| s.into()),
            webhook_url: router_data.request.webhook_url.clone(),
            capture_method: capture_method.map(|capture_method| capture_method.into()),
            email: router_data
                .request
                .email
                .clone()
                .map(|e| e.expose().expose().into()),
            browser_info,
            test_mode: router_data.test_mode,
            payment_method_type,
            state,
            return_url: router_data.request.router_return_url.clone(),
            description: router_data.description.clone(),
            connector_customer_id: router_data.connector_customer.clone(),
            address: Some(address),
            off_session: router_data.request.off_session,
            recurring_mandate_payment_data,
            enable_partial_authorization: router_data
                .request
                .enable_partial_authorization
                .map(|e| e.is_true()),
            billing_descriptor: router_data
                .request
                .billing_descriptor
                .as_ref()
                .map(payments_grpc::BillingDescriptor::foreign_from),
            mit_category: router_data
                .request
                .mit_category
                .map(payments_grpc::MitCategory::foreign_from)
                .map(|mit_category| mit_category.into()),
            shipping_cost: router_data
                .request
                .shipping_cost
                .map(|shipping_cost| shipping_cost.get_amount_as_i64()),
            authentication_data,
            connector_metadata: None,
            locale: router_data.request.locale.clone(),
            connector_testing_data: router_data.request.connector_testing_data.as_ref().map(
                |data| unified_connector_service_masking::Secret::new(data.peek().to_string()),
            ),
            merchant_account_id: router_data.request.merchant_account_id.as_ref().map(
                |merchant_account_id| {
                    unified_connector_service_masking::Secret::new(
                        merchant_account_id.clone().expose(),
                    )
                },
            ),
            merchant_configured_currency: router_data
                .request
                .merchant_config_currency
                .map(payments_grpc::Currency::foreign_try_from)
                .transpose()?
                .map(|currency| currency.into()),
        })
    }
}

impl
    transformers::ForeignTryFrom<
        &hyperswitch_domain_models::router_data::RecurringMandatePaymentData,
    > for payments_grpc::RecurringMandatePaymentData
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        data: &hyperswitch_domain_models::router_data::RecurringMandatePaymentData,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            original_payment_authorized_amount: data.original_payment_authorized_amount,
            original_payment_authorized_currency: data
                .original_payment_authorized_currency
                .map(payments_grpc::Currency::foreign_try_from)
                .transpose()?
                .map(|currency| currency.into()),
        })
    }
}

impl transformers::ForeignTryFrom<&RouterData<Session, PaymentsSessionData, PaymentsResponseData>>
    for payments_grpc::PaymentServiceSdkSessionTokenRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        router_data: &RouterData<Session, PaymentsSessionData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let currency = payments_grpc::Currency::foreign_try_from(router_data.request.currency)?;

        let country = router_data
            .request
            .country
            .as_ref()
            .and_then(|c| payments_grpc::CountryAlpha2::from_str_name(&c.to_string()))
            .map(|country| country.into());

        let merchant_account_metadata = serde_json::to_string(&router_data.connector_meta_data)
            .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?;

        Ok(Self {
            request_ref_id: Some(Identifier {
                id_type: Some(payments_grpc::identifier::IdType::Id(
                    router_data.connector_request_reference_id.clone(),
                )),
            }),
            amount: router_data.request.amount,
            currency: currency.into(),
            minor_amount: router_data.request.minor_amount.get_amount_as_i64(),
            email: router_data
                .request
                .email
                .clone()
                .map(|e| e.expose().expose().into()),
            merchant_account_metadata: Some(merchant_account_metadata.into()),
            order_tax_amount: router_data
                .request
                .order_tax_amount
                .map(|order_tax_amount| order_tax_amount.get_amount_as_i64()),
            customer_name: router_data
                .request
                .customer_name
                .clone()
                .map(|customer_name| customer_name.expose().into()),
            shipping_cost: router_data
                .request
                .shipping_cost
                .map(|shipping_cost| shipping_cost.get_amount_as_i64()),
            country_alpha2_code: country,
            payment_method_type: router_data
                .payment_method_type
                .map(payments_grpc::PaymentMethodType::foreign_try_from)
                .transpose()?
                .map(|payment_method_type| payment_method_type.into()),
            metadata: None,
            connector_metadata: None,
        })
    }
}

impl
    transformers::ForeignTryFrom<
        &RouterData<
            IncrementalAuthorization,
            PaymentsIncrementalAuthorizationData,
            PaymentsResponseData,
        >,
    > for payments_grpc::PaymentServiceIncrementalAuthorizationRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        router_data: &RouterData<
            IncrementalAuthorization,
            PaymentsIncrementalAuthorizationData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let currency = payments_grpc::Currency::foreign_try_from(router_data.request.currency)?;

        let state = router_data
            .access_token
            .as_ref()
            .map(ConnectorState::foreign_from);

        Ok(Self {
            minor_amount: router_data.request.total_amount,
            transaction_id: Some(Identifier {
                id_type: Some(payments_grpc::identifier::IdType::Id(
                    router_data.request.connector_transaction_id.clone(),
                )),
            }),
            request_ref_id: Some(Identifier {
                id_type: Some(payments_grpc::identifier::IdType::Id(
                    router_data.connector_request_reference_id.clone(),
                )),
            }),
            currency: currency.into(),
            reason: router_data.request.reason.clone(),
            connector_metadata: router_data
                .request
                .connector_meta
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
                .map(|s| s.into()),
            state,
        })
    }
}

impl
    transformers::ForeignTryFrom<(
        payments_grpc::PaymentServicePreAuthenticateResponse,
        AttemptStatus,
    )> for Result<(PaymentsResponseData, AttemptStatus), ErrorResponse>
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;
    fn foreign_try_from(
        (response, prev_status): (
            payments_grpc::PaymentServicePreAuthenticateResponse,
            AttemptStatus,
        ),
    ) -> Result<Self, Self::Error> {
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

        let resource_id: router_request_types::ResponseId = match response
            .transaction_id
            .as_ref()
            .and_then(|id| id.id_type.clone())
        {
            Some(payments_grpc::identifier::IdType::Id(id)) => {
                router_request_types::ResponseId::ConnectorTransactionId(id)
            }
            Some(payments_grpc::identifier::IdType::EncodedData(encoded_data)) => {
                router_request_types::ResponseId::EncodedData(encoded_data)
            }
            Some(payments_grpc::identifier::IdType::NoResponseIdMarker(_)) | None => {
                router_request_types::ResponseId::NoResponseId
            }
        };

        let redirection_data = response
            .redirection_data
            .clone()
            .map(RedirectForm::foreign_try_from)
            .transpose()?;

        let status_code = convert_connector_service_status_code(response.status_code)?;

        let response = if response.error_code.is_some() {
            let attempt_status = match response.status() {
                payments_grpc::PaymentStatus::AttemptStatusUnspecified => None,
                _ => Some(AttemptStatus::foreign_try_from((
                    response.status(),
                    prev_status,
                ))?),
            };

            Err(ErrorResponse {
                code: response.error_code().to_owned(),
                message: response.error_message().to_owned(),
                reason: Some(response.error_reason().to_owned()),
                status_code,
                attempt_status,
                connector_transaction_id: resource_id.get_optional_response_id(),
                connector_response_reference_id,
                network_decline_code: response.network_decline_code.clone(),
                network_advice_code: response.network_advice_code.clone(),
                network_error_message: response.network_error_message.clone(),
                connector_metadata: None,
            })
        } else {
            let status = AttemptStatus::foreign_try_from((response.status(), prev_status))?;

            let authentication_data = response.authentication_data.and_then(|auth_data| {
                router_request_types::UcsAuthenticationData::foreign_try_from(auth_data)
                    .ok()
                    .map(Box::new)
            });

            Ok((
                PaymentsResponseData::TransactionResponse {
                    resource_id,
                    redirection_data: Box::new(redirection_data),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: response.network_txn_id.clone(),
                    connector_response_reference_id,
                    incremental_authorization_allowed: None,
                    authentication_data,
                    charges: None,
                },
                status,
            ))
        };

        Ok(response)
    }
}

impl
    transformers::ForeignTryFrom<(
        payments_grpc::PaymentServiceAuthorizeResponse,
        AttemptStatus,
    )> for Result<(PaymentsResponseData, AttemptStatus), ErrorResponse>
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        (response, prev_status): (
            payments_grpc::PaymentServiceAuthorizeResponse,
            AttemptStatus,
        ),
    ) -> Result<Self, Self::Error> {
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

        let resource_id: router_request_types::ResponseId = match response
            .transaction_id
            .as_ref()
            .and_then(|id| id.id_type.clone())
        {
            Some(payments_grpc::identifier::IdType::Id(id)) => {
                router_request_types::ResponseId::ConnectorTransactionId(id)
            }
            Some(payments_grpc::identifier::IdType::EncodedData(encoded_data)) => {
                router_request_types::ResponseId::EncodedData(encoded_data)
            }
            Some(payments_grpc::identifier::IdType::NoResponseIdMarker(_)) | None => {
                router_request_types::ResponseId::NoResponseId
            }
        };

        let (mut connector_metadata, redirection_data) = match response.redirection_data.clone() {
            Some(redirection_data) => match redirection_data.form_type {
                Some(ref form_type) => match form_type {
                    payments_grpc::redirect_form::FormType::Uri(uri) => {
                        let sdk_uri_info = api_models::payments::SdkUpiUriInformation {
                            sdk_uri: uri.uri.clone(),
                        };
                        (
                            Some(
                                sdk_uri_info
                                    .encode_to_value()
                                    .change_context(UnifiedConnectorServiceError::ParsingFailed)
                                    .attach_printable(
                                        "Failed to serialize SdkUpiUriInformation to JSON value",
                                    )?,
                            ),
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

        // Parse connector_metadata from Secret<String> to serde_json::Value
        let parsed_connector_metadata: Option<serde_json::Value> =
            response.connector_metadata.clone().and_then(|secret| {
                let exposed = secret.expose();
                serde_json::from_str(&exposed)
                    .map_err(|e| {
                        tracing::warn!(
                            serialization_error = ?e,
                            metadata = ?response.connector_metadata,
                            "Failed to parse connector_metadata as JSON"
                        );
                        e
                    })
                    .ok()
            });

        connector_metadata = if parsed_connector_metadata
            .as_ref()
            .and_then(|meta| meta.get("nextActionData"))
            .and_then(|next_action_data| next_action_data.as_str())
            .filter(|&next_action_data| next_action_data == "WaitScreenInstructions")
            .is_some()
        {
            let wait_screen_metadata = build_upi_wait_screen_data()?;

            let mut metadata_map = connector_metadata
                .as_ref()
                .and_then(|meta| meta.as_object())
                .cloned()
                .unwrap_or_else(serde_json::Map::new);

            metadata_map.insert("WaitScreenInstructions".to_string(), wait_screen_metadata);

            // For UPI Intent/QR, also preserve URI information from redirection data
            if let Some(redirection_data) = response.redirection_data.as_ref() {
                if let Some(payments_grpc::redirect_form::FormType::Uri(uri)) =
                    &redirection_data.form_type
                {
                    let sdk_uri_info = api_models::payments::SdkUpiUriInformation {
                        sdk_uri: uri.uri.clone(),
                    };
                    let uri_data = sdk_uri_info
                        .encode_to_value()
                        .change_context(UnifiedConnectorServiceError::ParsingFailed)
                        .attach_printable(
                            "Failed to serialize SdkUpiUriInformation to JSON value",
                        )?;
                    metadata_map.insert("SdkUpiUriInformation".to_string(), uri_data);
                }
            }

            Some(serde_json::Value::Object(metadata_map))
        } else {
            connector_metadata
        };

        // Extract connector_metadata from response if present and not already set
        connector_metadata = match (connector_metadata, response.connector_metadata.clone()) {
            (None, Some(_secret)) => parsed_connector_metadata,
            (existing, _) => existing, // keep the existing value if already Some or response is None
        };

        let status_code = convert_connector_service_status_code(response.status_code)?;

        let response = if response.error_code.is_some() {
            let attempt_status = match response.status() {
                payments_grpc::PaymentStatus::AttemptStatusUnspecified => None,
                _ => Some(AttemptStatus::foreign_try_from((
                    response.status(),
                    prev_status,
                ))?),
            };

            Err(ErrorResponse {
                code: response.error_code().to_owned(),
                message: response.error_message().to_owned(),
                reason: Some(response.error_reason().to_owned()),
                status_code,
                attempt_status,
                connector_transaction_id: resource_id.get_optional_response_id(),
                connector_response_reference_id,
                network_decline_code: response.network_decline_code.clone(),
                network_advice_code: response.network_advice_code.clone(),
                network_error_message: response.network_error_message.clone(),
                connector_metadata: None,
            })
        } else {
            let status = AttemptStatus::foreign_try_from((response.status(), prev_status))?;

            Ok((
                PaymentsResponseData::TransactionResponse {
                    resource_id,
                    redirection_data: Box::new(redirection_data),
                    mandate_reference: Box::new(response.mandate_reference.map(|grpc_mandate| {
                        hyperswitch_domain_models::router_response_types::MandateReference {
                            connector_mandate_id: grpc_mandate.mandate_id,
                            payment_method_id: grpc_mandate.payment_method_id,
                            mandate_metadata: None,
                            connector_mandate_request_reference_id: grpc_mandate
                                .connector_mandate_request_reference_id,
                        }
                    })),
                    connector_metadata,
                    network_txn_id: response.network_txn_id.clone(),
                    connector_response_reference_id,
                    incremental_authorization_allowed: response.incremental_authorization_allowed,
                    authentication_data: None,
                    charges: None,
                },
                status,
            ))
        };

        Ok(response)
    }
}

impl transformers::ForeignTryFrom<(payments_grpc::PaymentServiceCaptureResponse, AttemptStatus)>
    for Result<(PaymentsResponseData, AttemptStatus), ErrorResponse>
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        (response, prev_status): (payments_grpc::PaymentServiceCaptureResponse, AttemptStatus),
    ) -> Result<Self, Self::Error> {
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

        let resource_id: router_request_types::ResponseId = match response
            .transaction_id
            .as_ref()
            .and_then(|id| id.id_type.clone())
        {
            Some(payments_grpc::identifier::IdType::Id(id)) => {
                router_request_types::ResponseId::ConnectorTransactionId(id)
            }
            Some(payments_grpc::identifier::IdType::EncodedData(encoded_data)) => {
                router_request_types::ResponseId::EncodedData(encoded_data)
            }
            Some(payments_grpc::identifier::IdType::NoResponseIdMarker(_)) | None => {
                router_request_types::ResponseId::NoResponseId
            }
        };

        // Extract connector_metadata from response if present
        let connector_metadata = response.connector_metadata.clone().and_then(|secret| {
            let connector_metadata = secret.expose();
            serde_json::from_str(&connector_metadata)
                .map_err(|e| {
                    tracing::warn!(
                        serialization_error = ?e,
                        metadata = ?response.connector_metadata,
                        "Failed to serialize connector_metadata from UCS capture response"
                    );
                    e
                })
                .ok()
        });

        let response = if response.error_code.is_some() {
            let attempt_status = match response.status() {
                payments_grpc::PaymentStatus::AttemptStatusUnspecified => None,
                _ => Some(AttemptStatus::foreign_try_from((
                    response.status(),
                    prev_status,
                ))?),
            };
            Err(ErrorResponse {
                code: response.error_code().to_owned(),
                message: response.error_message().to_owned(),
                reason: Some(response.error_reason().to_owned()),
                status_code,
                attempt_status,
                connector_transaction_id: resource_id.get_optional_response_id(),
                connector_response_reference_id,
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            let status = AttemptStatus::foreign_try_from((response.status(), prev_status))?;
            Ok((
                PaymentsResponseData::TransactionResponse {
                    resource_id,
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(response.mandate_reference.map(|grpc_mandate| {
                        hyperswitch_domain_models::router_response_types::MandateReference {
                            connector_mandate_id: grpc_mandate.mandate_id,
                            payment_method_id: grpc_mandate.payment_method_id,
                            mandate_metadata: None,
                            connector_mandate_request_reference_id: grpc_mandate
                                .connector_mandate_request_reference_id,
                        }
                    })),
                    connector_metadata,
                    network_txn_id: None,
                    connector_response_reference_id,
                    incremental_authorization_allowed: response.incremental_authorization_allowed,
                    authentication_data: None,
                    charges: None,
                },
                status,
            ))
        };
        Ok(response)
    }
}

impl transformers::ForeignTryFrom<payments_grpc::PaymentServiceCreateConnectorCustomerResponse>
    for Result<PaymentsResponseData, ErrorResponse>
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        response: payments_grpc::PaymentServiceCreateConnectorCustomerResponse,
    ) -> Result<Self, Self::Error> {
        let status_code = convert_connector_service_status_code(response.status_code)?;

        let response = if response.error_code.is_some() {
            router_env::logger::error!(
                error_message = ?response.error_message,
                error_code = ?response.error_code,
                status_code,
                "UCS create connector customer failed"
            );

            Err(ErrorResponse {
                code: response.error_code().to_owned(),
                message: response.error_message().to_owned(),
                reason: Some(response.error_message().to_owned()),
                status_code,
                attempt_status: None,
                connector_transaction_id: None,
                connector_response_reference_id: None,
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            use hyperswitch_domain_models::router_response_types::ConnectorCustomerResponseData;
            Ok(PaymentsResponseData::ConnectorCustomerResponse(
                ConnectorCustomerResponseData::new_with_customer_id(response.connector_customer_id),
            ))
        };

        Ok(response)
    }
}

impl transformers::ForeignTryFrom<(payments_grpc::PaymentServiceRegisterResponse, AttemptStatus)>
    for Result<(PaymentsResponseData, AttemptStatus), ErrorResponse>
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        (response, prev_status): (payments_grpc::PaymentServiceRegisterResponse, AttemptStatus),
    ) -> Result<Self, Self::Error> {
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

        let resource_id: router_request_types::ResponseId = match response
            .registration_id
            .as_ref()
            .and_then(|id| id.id_type.clone())
        {
            Some(payments_grpc::identifier::IdType::Id(id)) => {
                router_request_types::ResponseId::ConnectorTransactionId(id)
            }
            Some(payments_grpc::identifier::IdType::EncodedData(encoded_data)) => {
                router_request_types::ResponseId::EncodedData(encoded_data)
            }
            Some(payments_grpc::identifier::IdType::NoResponseIdMarker(_)) | None => {
                router_request_types::ResponseId::NoResponseId
            }
        };

        let status_code = convert_connector_service_status_code(response.status_code)?;

        let response = if response.error_code.is_some() {
            let attempt_status = match response.status() {
                payments_grpc::PaymentStatus::AttemptStatusUnspecified => None,
                _ => Some(AttemptStatus::foreign_try_from((
                    response.status(),
                    prev_status,
                ))?),
            };
            Err(ErrorResponse {
                code: response.error_code().to_owned(),
                message: response.error_message().to_owned(),
                reason: Some(response.error_reason().to_owned()),
                status_code,
                attempt_status,
                connector_transaction_id: resource_id.get_optional_response_id(),
                connector_response_reference_id,
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            let status = AttemptStatus::foreign_try_from((response.status(), prev_status))?;

            // Extract connector_metadata from response if present
            let connector_metadata = response.connector_metadata.clone().and_then(|secret| {
                let connector_metadata = secret.expose();
                serde_json::from_str(&connector_metadata)
                    .map_err(|e| {
                        tracing::warn!(
                            serialization_error=?e,
                            metadata=?response.connector_metadata,
                            "Failed to serialize connector_metadata from UCS register response"
                        );
                        e
                    })
                    .ok()
            });

            Ok((
                PaymentsResponseData::TransactionResponse {
                    resource_id,
                    redirection_data: Box::new(
                        response
                            .redirection_data
                            .clone()
                            .map(RedirectForm::foreign_try_from)
                            .transpose()?,
                    ),
                    mandate_reference: Box::new(response.mandate_reference.map(|grpc_mandate| {
                        hyperswitch_domain_models::router_response_types::MandateReference {
                            connector_mandate_id: grpc_mandate.mandate_id,
                            payment_method_id: grpc_mandate.payment_method_id,
                            mandate_metadata: None,
                            connector_mandate_request_reference_id: grpc_mandate
                                .connector_mandate_request_reference_id,
                        }
                    })),
                    connector_metadata,
                    network_txn_id: response.network_txn_id,
                    connector_response_reference_id,
                    incremental_authorization_allowed: response.incremental_authorization_allowed,
                    authentication_data: None,
                    charges: None,
                },
                status,
            ))
        };

        Ok(response)
    }
}

impl
    transformers::ForeignTryFrom<(
        payments_grpc::PaymentServiceRepeatEverythingResponse,
        AttemptStatus,
    )> for Result<(PaymentsResponseData, AttemptStatus), ErrorResponse>
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        (response, prev_status): (
            payments_grpc::PaymentServiceRepeatEverythingResponse,
            AttemptStatus,
        ),
    ) -> Result<Self, Self::Error> {
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

        let resource_id: router_request_types::ResponseId = match response
            .transaction_id
            .as_ref()
            .and_then(|id| id.id_type.clone())
        {
            Some(payments_grpc::identifier::IdType::Id(id)) => {
                router_request_types::ResponseId::ConnectorTransactionId(id)
            }
            Some(payments_grpc::identifier::IdType::EncodedData(encoded_data)) => {
                router_request_types::ResponseId::EncodedData(encoded_data)
            }
            Some(payments_grpc::identifier::IdType::NoResponseIdMarker(_)) | None => {
                router_request_types::ResponseId::NoResponseId
            }
        };

        let status_code = convert_connector_service_status_code(response.status_code)?;

        // Extract connector_metadata from response if present
        let connector_metadata = response.connector_metadata.clone().and_then(|secret| {
            let connector_metadata = secret.expose();
            serde_json::from_str(&connector_metadata)
                .map_err(|e| {
                    tracing::warn!(
                        serialization_error=?e,
                        metadata=?response.connector_metadata,
                        "Failed to serialize connector_metadata from UCS repeat payment response"
                    );
                    e
                })
                .ok()
        });

        let response = if response.error_code.is_some() {
            let attempt_status = match response.status() {
                payments_grpc::PaymentStatus::AttemptStatusUnspecified => None,
                _ => Some(AttemptStatus::foreign_try_from((
                    response.status(),
                    prev_status,
                ))?),
            };
            Err(ErrorResponse {
                code: response.error_code().to_owned(),
                message: response.error_message().to_owned(),
                reason: Some(response.error_reason().to_owned()),
                status_code,
                attempt_status,
                connector_transaction_id: resource_id.get_optional_response_id(),
                connector_response_reference_id,
                network_decline_code: response.network_decline_code.clone(),
                network_advice_code: response.network_advice_code.clone(),
                network_error_message: response.network_error_message.clone(),
                connector_metadata: None,
            })
        } else {
            let status = AttemptStatus::foreign_try_from((response.status(), prev_status))?;

            Ok((
                PaymentsResponseData::TransactionResponse {
                    resource_id,
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(response.mandate_reference.map(|grpc_mandate| {
                        hyperswitch_domain_models::router_response_types::MandateReference {
                            connector_mandate_id: grpc_mandate.mandate_id,
                            payment_method_id: grpc_mandate.payment_method_id,
                            mandate_metadata: None,
                            connector_mandate_request_reference_id: grpc_mandate
                                .connector_mandate_request_reference_id,
                        }
                    })),
                    connector_metadata,
                    network_txn_id: response.network_txn_id.clone(),
                    connector_response_reference_id,
                    incremental_authorization_allowed: response.incremental_authorization_allowed,
                    authentication_data: None,
                    charges: None,
                },
                status,
            ))
        };

        Ok(response)
    }
}

impl transformers::ForeignTryFrom<payments_grpc::PaymentServiceCreateAccessTokenResponse>
    for Result<AccessToken, ErrorResponse>
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        response: payments_grpc::PaymentServiceCreateAccessTokenResponse,
    ) -> Result<Self, Self::Error> {
        let status_code = convert_connector_service_status_code(response.status_code)?;

        let response = if response.error_code.is_some() {
            router_env::logger::error!(
                error_message = ?response.error_message,
                error_code = ?response.error_code,
                status_code,
                "UCS create access token failed"
            );

            Err(ErrorResponse {
                code: response.error_code().to_owned(),
                message: response.error_message().to_owned(),
                reason: Some(response.error_message().to_owned()),
                status_code,
                attempt_status: None,
                connector_transaction_id: None,
                connector_response_reference_id: None,
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            let token = response
                .access_token
                .ok_or(UnifiedConnectorServiceError::MissingRequiredField {
                    field_name: "access_token",
                })
                .attach_printable("Missing access_token in CreateAccessToken response")?;

            Ok(AccessToken {
                token: token.expose().into(),
                expires: response.expires_in_seconds.unwrap_or_default(),
            })
        };

        Ok(response)
    }
}

impl transformers::ForeignTryFrom<common_enums::Currency> for payments_grpc::Currency {
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

impl transformers::ForeignTryFrom<payments_grpc::Currency> for common_enums::Currency {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(currency: payments_grpc::Currency) -> Result<Self, Self::Error> {
        let currency_str = currency.as_str_name();
        Self::from_str(currency_str).change_context(UnifiedConnectorServiceError::ParsingFailed)
    }
}

impl transformers::ForeignTryFrom<payments_grpc::CountryAlpha2> for common_enums::CountryAlpha2 {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(country: payments_grpc::CountryAlpha2) -> Result<Self, Self::Error> {
        let country_str = country.as_str_name();
        Self::from_str(country_str).change_context(UnifiedConnectorServiceError::ParsingFailed)
    }
}

impl transformers::ForeignTryFrom<common_enums::PaymentMethodType>
    for payments_grpc::PaymentMethodType
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(value: common_enums::PaymentMethodType) -> Result<Self, Self::Error> {
        match value {
            common_enums::PaymentMethodType::Ach => Ok(Self::Ach),
            common_enums::PaymentMethodType::Affirm => Ok(Self::Affirm),
            common_enums::PaymentMethodType::AfterpayClearpay => Ok(Self::AfterpayClearpay),
            common_enums::PaymentMethodType::Alfamart => Ok(Self::Alfamart),
            common_enums::PaymentMethodType::AliPay => Ok(Self::AliPay),
            common_enums::PaymentMethodType::AliPayHk => Ok(Self::AliPayHk),
            common_enums::PaymentMethodType::Alma => Ok(Self::Alma),
            common_enums::PaymentMethodType::AmazonPay => Ok(Self::AmazonPay),
            common_enums::PaymentMethodType::ApplePay => Ok(Self::ApplePay),
            common_enums::PaymentMethodType::Atome => Ok(Self::Atome),
            common_enums::PaymentMethodType::Bacs => Ok(Self::Bacs),
            common_enums::PaymentMethodType::BancontactCard => Ok(Self::BancontactCard),
            common_enums::PaymentMethodType::Becs => Ok(Self::Becs),
            common_enums::PaymentMethodType::Benefit => Ok(Self::Benefit),
            common_enums::PaymentMethodType::Bizum => Ok(Self::Bizum),
            common_enums::PaymentMethodType::Blik => Ok(Self::Blik),
            common_enums::PaymentMethodType::Boleto => Ok(Self::Boleto),
            common_enums::PaymentMethodType::BcaBankTransfer => Ok(Self::BcaBankTransfer),
            common_enums::PaymentMethodType::BniVa => Ok(Self::BniVa),
            common_enums::PaymentMethodType::BriVa => Ok(Self::BriVa),
            common_enums::PaymentMethodType::CardRedirect => Ok(Self::CardRedirect),
            common_enums::PaymentMethodType::CimbVa => Ok(Self::CimbVa),
            common_enums::PaymentMethodType::ClassicReward => Ok(Self::ClassicReward),
            common_enums::PaymentMethodType::Credit => Ok(Self::Credit),
            common_enums::PaymentMethodType::CryptoCurrency => Ok(Self::CryptoCurrency),
            common_enums::PaymentMethodType::Cashapp => Ok(Self::Cashapp),
            common_enums::PaymentMethodType::Dana => Ok(Self::Dana),
            common_enums::PaymentMethodType::DanamonVa => Ok(Self::DanamonVa),
            common_enums::PaymentMethodType::Debit => Ok(Self::Debit),
            common_enums::PaymentMethodType::DuitNow => Ok(Self::DuitNow),
            common_enums::PaymentMethodType::Efecty => Ok(Self::Efecty),
            common_enums::PaymentMethodType::Eft => Ok(Self::Eft),
            common_enums::PaymentMethodType::Eps => Ok(Self::Eps),
            common_enums::PaymentMethodType::Fps => Ok(Self::Fps),
            common_enums::PaymentMethodType::Evoucher => Ok(Self::Evoucher),
            common_enums::PaymentMethodType::Giropay => Ok(Self::Giropay),
            common_enums::PaymentMethodType::Givex => Ok(Self::Givex),
            common_enums::PaymentMethodType::GooglePay => Ok(Self::GooglePay),
            common_enums::PaymentMethodType::GoPay => Ok(Self::GoPay),
            common_enums::PaymentMethodType::Gcash => Ok(Self::Gcash),
            common_enums::PaymentMethodType::Ideal => Ok(Self::Ideal),
            common_enums::PaymentMethodType::Interac => Ok(Self::Interac),
            common_enums::PaymentMethodType::Indomaret => Ok(Self::Indomaret),
            common_enums::PaymentMethodType::KakaoPay => Ok(Self::KakaoPay),
            common_enums::PaymentMethodType::LocalBankRedirect => Ok(Self::LocalBankRedirect),
            common_enums::PaymentMethodType::MandiriVa => Ok(Self::MandiriVa),
            common_enums::PaymentMethodType::Knet => Ok(Self::Knet),
            common_enums::PaymentMethodType::MbWay => Ok(Self::MbWay),
            common_enums::PaymentMethodType::MobilePay => Ok(Self::MobilePay),
            common_enums::PaymentMethodType::Momo => Ok(Self::Momo),
            common_enums::PaymentMethodType::MomoAtm => Ok(Self::MomoAtm),
            common_enums::PaymentMethodType::Multibanco => Ok(Self::Multibanco),
            common_enums::PaymentMethodType::OnlineBankingThailand => {
                Ok(Self::OnlineBankingThailand)
            }
            common_enums::PaymentMethodType::OnlineBankingCzechRepublic => {
                Ok(Self::OnlineBankingCzechRepublic)
            }
            common_enums::PaymentMethodType::OnlineBankingFinland => Ok(Self::OnlineBankingFinland),
            common_enums::PaymentMethodType::OnlineBankingFpx => Ok(Self::OnlineBankingFpx),
            common_enums::PaymentMethodType::OnlineBankingPoland => Ok(Self::OnlineBankingPoland),
            common_enums::PaymentMethodType::OnlineBankingSlovakia => {
                Ok(Self::OnlineBankingSlovakia)
            }
            common_enums::PaymentMethodType::Oxxo => Ok(Self::Oxxo),
            common_enums::PaymentMethodType::PagoEfectivo => Ok(Self::PagoEfectivo),
            common_enums::PaymentMethodType::PermataBankTransfer => Ok(Self::PermataBankTransfer),
            common_enums::PaymentMethodType::OpenBankingUk => Ok(Self::OpenBankingUk),
            common_enums::PaymentMethodType::PayBright => Ok(Self::PayBright),
            common_enums::PaymentMethodType::Paze => Ok(Self::Paze),
            common_enums::PaymentMethodType::Pix => Ok(Self::Pix),
            common_enums::PaymentMethodType::PaySafeCard => Ok(Self::PaySafeCard),
            common_enums::PaymentMethodType::Przelewy24 => Ok(Self::Przelewy24),
            common_enums::PaymentMethodType::PromptPay => Ok(Self::PromptPay),
            common_enums::PaymentMethodType::Pse => Ok(Self::Pse),
            common_enums::PaymentMethodType::RedCompra => Ok(Self::RedCompra),
            common_enums::PaymentMethodType::RedPagos => Ok(Self::RedPagos),
            common_enums::PaymentMethodType::SamsungPay => Ok(Self::SamsungPay),
            common_enums::PaymentMethodType::Sepa => Ok(Self::Sepa),
            common_enums::PaymentMethodType::SepaBankTransfer => Ok(Self::SepaBankTransfer),
            common_enums::PaymentMethodType::Sofort => Ok(Self::Sofort),
            common_enums::PaymentMethodType::Swish => Ok(Self::Swish),
            common_enums::PaymentMethodType::TouchNGo => Ok(Self::TouchNGo),
            common_enums::PaymentMethodType::Trustly => Ok(Self::Trustly),
            common_enums::PaymentMethodType::Twint => Ok(Self::Twint),
            common_enums::PaymentMethodType::UpiCollect => Ok(Self::UpiCollect),
            common_enums::PaymentMethodType::UpiIntent => Ok(Self::UpiIntent),
            common_enums::PaymentMethodType::UpiQr => Ok(Self::UpiQr),
            common_enums::PaymentMethodType::Vipps => Ok(Self::Vipps),
            common_enums::PaymentMethodType::VietQr => Ok(Self::VietQr),
            common_enums::PaymentMethodType::Venmo => Ok(Self::Venmo),
            common_enums::PaymentMethodType::Walley => Ok(Self::Walley),
            common_enums::PaymentMethodType::WeChatPay => Ok(Self::WeChatPay),
            common_enums::PaymentMethodType::SevenEleven => Ok(Self::SevenEleven),
            common_enums::PaymentMethodType::Lawson => Ok(Self::Lawson),
            common_enums::PaymentMethodType::MiniStop => Ok(Self::MiniStop),
            common_enums::PaymentMethodType::FamilyMart => Ok(Self::FamilyMart),
            common_enums::PaymentMethodType::Seicomart => Ok(Self::Seicomart),
            common_enums::PaymentMethodType::PayEasy => Ok(Self::PayEasy),
            common_enums::PaymentMethodType::LocalBankTransfer => Ok(Self::LocalBankTransfer),
            common_enums::PaymentMethodType::OpenBankingPIS => Ok(Self::OpenBankingPis),
            common_enums::PaymentMethodType::DirectCarrierBilling => Ok(Self::DirectCarrierBilling),
            common_enums::PaymentMethodType::InstantBankTransfer => Ok(Self::InstantBankTransfer),
            common_enums::PaymentMethodType::Paypal => Ok(Self::PayPal),
            common_enums::PaymentMethodType::RevolutPay => Ok(Self::RevolutPay),
            _ => Err(
                UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                    "Payment Method Type not yet supported".to_string(),
                ),
            )?,
        }
    }
}

impl ForeignFrom<common_enums::CardNetwork> for payments_grpc::CardNetwork {
    fn foreign_from(card_network: common_enums::CardNetwork) -> Self {
        match card_network {
            common_enums::CardNetwork::Visa => Self::Visa,
            common_enums::CardNetwork::Mastercard => Self::Mastercard,
            common_enums::CardNetwork::JCB => Self::Jcb,
            common_enums::CardNetwork::DinersClub => Self::Diners,
            common_enums::CardNetwork::Discover => Self::Discover,
            common_enums::CardNetwork::CartesBancaires => Self::CartesBancaires,
            common_enums::CardNetwork::UnionPay => Self::Unionpay,
            common_enums::CardNetwork::RuPay => Self::Rupay,
            common_enums::CardNetwork::Maestro => Self::Maestro,
            common_enums::CardNetwork::AmericanExpress => Self::Amex,
            common_enums::CardNetwork::Interac => Self::InteracCard,
            common_enums::CardNetwork::Star => Self::Star,
            common_enums::CardNetwork::Pulse => Self::Pulse,
            common_enums::CardNetwork::Accel => Self::Accel,
            common_enums::CardNetwork::Nyce => Self::Nyce,
        }
    }
}

impl transformers::ForeignTryFrom<hyperswitch_domain_models::payment_method_data::UpiSource>
    for payments_grpc::UpiSource
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        upi_source: hyperswitch_domain_models::payment_method_data::UpiSource,
    ) -> Result<Self, Self::Error> {
        match upi_source {
            hyperswitch_domain_models::payment_method_data::UpiSource::UpiCc => Ok(Self::UpiCc),
            hyperswitch_domain_models::payment_method_data::UpiSource::UpiCl => Ok(Self::UpiCl),
            hyperswitch_domain_models::payment_method_data::UpiSource::UpiAccount => {
                Ok(Self::UpiAccount)
            }
            hyperswitch_domain_models::payment_method_data::UpiSource::UpiCcCl => Ok(Self::UpiCcCl),
            hyperswitch_domain_models::payment_method_data::UpiSource::UpiPpi => Ok(Self::UpiPpi),
            hyperswitch_domain_models::payment_method_data::UpiSource::UpiVoucher => {
                Ok(Self::UpiVoucher)
            }
        }
    }
}

impl transformers::ForeignTryFrom<hyperswitch_domain_models::payment_method_data::NetworkTokenData>
    for payments_grpc::NetworkTokenData
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        network_token_data: hyperswitch_domain_models::payment_method_data::NetworkTokenData,
    ) -> Result<Self, Self::Error> {
        let card_network = network_token_data
            .card_network
            .clone()
            .map(payments_grpc::CardNetwork::foreign_from);

        #[cfg(feature = "v1")]
        let network_token = Self {
            token_number: Some(
                NetworkToken::from_str(&network_token_data.token_number.get_card_no())
                    .change_context(
                        UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                            "Failed to parse token number".to_string(),
                        ),
                    )?,
            ),
            token_exp_month: Some(network_token_data.token_exp_month.expose().into()),
            token_exp_year: Some(network_token_data.token_exp_year.expose().into()),
            token_cryptogram: network_token_data
                .token_cryptogram
                .map(|cryptogram| cryptogram.expose().into()),
            card_issuer: network_token_data.card_issuer.clone(),
            card_network: card_network.map(|card_network| card_network.into()),
            card_type: network_token_data.card_type.clone(),
            card_issuing_country: network_token_data.card_issuing_country.clone(),
            bank_code: network_token_data.bank_code.clone(),
            nick_name: network_token_data.nick_name.map(|n| n.expose().into()),
            eci: network_token_data.eci,
        };

        #[cfg(feature = "v2")]
        let network_token = Self {
            token_number: Some(
                NetworkToken::from_str(&network_token_data.network_token.get_card_no())
                    .change_context(
                        UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                            "Failed to parse network token number".to_string(),
                        ),
                    )?,
            ),
            token_exp_month: Some(network_token_data.network_token_exp_month.expose().into()),
            token_exp_year: Some(network_token_data.network_token_exp_year.expose().into()),
            token_cryptogram: network_token_data
                .cryptogram
                .map(|cryptogram| cryptogram.expose().into()),
            card_issuer: network_token_data.card_issuer.clone(),
            card_network: card_network.map(|card_network| card_network.into()),
            card_type: network_token_data
                .card_type
                .clone()
                .map(|ct| ct.to_string()),
            card_issuing_country: network_token_data
                .card_issuing_country
                .map(|cic| cic.to_string()),
            bank_code: network_token_data.bank_code.clone(),
            nick_name: network_token_data.nick_name.map(|n| n.expose().into()),
            eci: network_token_data.eci,
        };

        Ok(network_token)
    }
}

impl
    transformers::ForeignTryFrom<
        hyperswitch_domain_models::payment_method_data::CardDetailsForNetworkTransactionId,
    > for payments_grpc::CardDetailsForNetworkTransactionId
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        card_nti_data: hyperswitch_domain_models::payment_method_data::CardDetailsForNetworkTransactionId,
    ) -> Result<Self, Self::Error> {
        let card_network = card_nti_data
            .card_network
            .clone()
            .map(payments_grpc::CardNetwork::foreign_from);

        let card_details_for_nti = Self {
            card_number: Some(
                CardNumber::from_str(&card_nti_data.card_number.get_card_no()).change_context(
                    UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                        "Failed to parse card number".to_string(),
                    ),
                )?,
            ),
            card_exp_month: Some(card_nti_data.card_exp_month.expose().into()),
            card_exp_year: Some(card_nti_data.card_exp_year.expose().into()),
            card_issuer: card_nti_data.card_issuer.clone(),
            card_network: card_network.map(|card_network| card_network.into()),
            card_type: card_nti_data.card_type.clone(),
            card_issuing_country: card_nti_data.card_issuing_country.clone(),
            bank_code: card_nti_data.bank_code.clone(),
            nick_name: card_nti_data.nick_name.map(|n| n.expose().into()),
            card_holder_name: card_nti_data
                .card_holder_name
                .map(|name| name.expose().into()),
        };

        Ok(card_details_for_nti)
    }
}

impl transformers::ForeignTryFrom<&common_types::payments::ApplePayPaymentData>
    for payments_grpc::apple_wallet::payment_data::PaymentData
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        payment_data: &common_types::payments::ApplePayPaymentData,
    ) -> Result<Self, Self::Error> {
        match payment_data {
            common_types::payments::ApplePayPaymentData::Encrypted(encrypted_data) => {
                Ok(Self::EncryptedData(encrypted_data.clone()))
            }
            common_types::payments::ApplePayPaymentData::Decrypted(decrypted_data) => {
                let application_primary_account_number = CardNumber::from_str(
                    &decrypted_data
                        .application_primary_account_number
                        .get_card_no(),
                )
                .change_context(
                    UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                        "Failed to parse card number".to_string(),
                    ),
                )?;
                Ok(Self::DecryptedData(payments_grpc::ApplePayDecryptedData {
                    application_primary_account_number: Some(application_primary_account_number),
                    application_expiration_month: Some(
                        decrypted_data
                            .application_expiration_month
                            .clone()
                            .expose()
                            .into(),
                    ),
                    application_expiration_year: Some(
                        decrypted_data
                            .application_expiration_year
                            .clone()
                            .expose()
                            .into(),
                    ),
                    payment_data: Some(payments_grpc::ApplePayCryptogramData {
                        online_payment_cryptogram: Some(
                            decrypted_data
                                .payment_data
                                .online_payment_cryptogram
                                .clone()
                                .expose()
                                .into(),
                        ),
                        eci_indicator: decrypted_data.payment_data.eci_indicator.clone(),
                    }),
                }))
            }
        }
    }
}

impl transformers::ForeignTryFrom<&hyperswitch_domain_models::router_data::PazeToken>
    for payments_grpc::PazeToken
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        value: &hyperswitch_domain_models::router_data::PazeToken,
    ) -> Result<Self, Self::Error> {
        let payment_token = NetworkToken::from_str(&value.payment_token.get_card_no())
            .change_context(
                UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                    "Failed to parse network token".to_string(),
                ),
            )?;
        Ok(Self {
            payment_token: Some(payment_token),
            token_expiration_month: Some(value.token_expiration_month.clone().expose().into()),
            token_expiration_year: Some(value.token_expiration_year.clone().expose().into()),
            payment_account_reference: Some(
                value.payment_account_reference.clone().expose().into(),
            ),
        })
    }
}

impl transformers::ForeignTryFrom<&hyperswitch_domain_models::router_data::PazeDynamicData>
    for payments_grpc::PazeDynamicData
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        value: &hyperswitch_domain_models::router_data::PazeDynamicData,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            dynamic_data_value: value
                .dynamic_data_value
                .clone()
                .map(|dynamic_data| dynamic_data.expose().into()),
            dynamic_data_type: value.dynamic_data_type.clone(),
            dynamic_data_expiration: value.dynamic_data_expiration.clone(),
        })
    }
}

impl transformers::ForeignTryFrom<&hyperswitch_domain_models::router_data::PazePhoneNumber>
    for payments_grpc::PazePhoneNumber
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        value: &hyperswitch_domain_models::router_data::PazePhoneNumber,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            country_code: Some(value.country_code.clone().expose().into()),
            phone_number: Some(value.phone_number.clone().expose().into()),
        })
    }
}

impl transformers::ForeignTryFrom<&hyperswitch_domain_models::router_data::PazeConsumer>
    for payments_grpc::PazeConsumer
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        value: &hyperswitch_domain_models::router_data::PazeConsumer,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            first_name: value
                .first_name
                .clone()
                .map(|first_name| first_name.expose().into()),
            last_name: value
                .last_name
                .clone()
                .map(|last_name| last_name.expose().into()),
            full_name: Some(value.full_name.clone().expose().into()),
            email_address: Some(value.email_address.clone().expose().expose().into()),
            mobile_number: value
                .mobile_number
                .as_ref()
                .map(payments_grpc::PazePhoneNumber::foreign_try_from)
                .transpose()?,
            country_code: value.country_code.and_then(|country_code| {
                payments_grpc::CountryAlpha2::from_str_name(&country_code.to_string())
                    .map(|country| country.into())
            }),
            language_code: value.language_code.clone(),
        })
    }
}

impl transformers::ForeignTryFrom<&hyperswitch_domain_models::router_data::PazeAddress>
    for payments_grpc::PazeAddress
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        value: &hyperswitch_domain_models::router_data::PazeAddress,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name.clone().map(|name| name.expose().into()),
            line1: value.line1.clone().map(|line1| line1.expose().into()),
            line2: value.line2.clone().map(|line2| line2.expose().into()),
            line3: value.line3.clone().map(|line3| line3.expose().into()),
            city: value.city.clone().map(|city| city.expose().into()),
            state: value.state.clone().map(|state| state.expose().into()),
            zip: value.zip.clone().map(|zip| zip.expose().into()),
            country_code: value.country_code.and_then(|country_code| {
                payments_grpc::CountryAlpha2::from_str_name(&country_code.to_string())
                    .map(|country| country.into())
            }),
        })
    }
}

impl transformers::ForeignTryFrom<&hyperswitch_domain_models::router_data::PazeDecryptedData>
    for payments_grpc::PazeDecryptedData
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        value: &hyperswitch_domain_models::router_data::PazeDecryptedData,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            client_id: Some(value.client_id.clone().expose().into()),
            profile_id: value.profile_id.clone(),
            token: Some(payments_grpc::PazeToken::foreign_try_from(&value.token)?),
            payment_card_network: payments_grpc::CardNetwork::foreign_from(
                value.payment_card_network.clone(),
            )
            .into(),
            dynamic_data: value
                .dynamic_data
                .iter()
                .map(payments_grpc::PazeDynamicData::foreign_try_from)
                .collect::<Result<Vec<_>, _>>()?,
            billing_address: Some(payments_grpc::PazeAddress::foreign_try_from(
                &value.billing_address,
            )?),
            consumer: Some(payments_grpc::PazeConsumer::foreign_try_from(
                &value.consumer,
            )?),
            eci: value.eci.clone(),
        })
    }
}

impl transformers::ForeignTryFrom<&hyperswitch_domain_models::payment_method_data::PazeWalletData>
    for payments_grpc::paze_wallet::PazeData
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        value: &hyperswitch_domain_models::payment_method_data::PazeWalletData,
    ) -> Result<Self, Self::Error> {
        Ok(Self::CompleteResponse(
            value.complete_response.clone().expose().into(),
        ))
    }
}

impl transformers::ForeignTryFrom<&common_types::payments::GpayTokenizationData>
    for payments_grpc::google_wallet::tokenization_data::TokenizationData
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        tokenization_data: &common_types::payments::GpayTokenizationData,
    ) -> Result<Self, Self::Error> {
        match tokenization_data {
            common_types::payments::GpayTokenizationData::Encrypted(encrypted_data) => Ok(
                Self::EncryptedData(payments_grpc::GooglePayEncryptedTokenizationData {
                    token_type: encrypted_data.token_type.clone(),
                    token: encrypted_data.token.clone(),
                }),
            ),
            common_types::payments::GpayTokenizationData::Decrypted(decrypted_data) => {
                let application_primary_account_number = CardNumber::from_str(
                    &decrypted_data
                        .application_primary_account_number
                        .get_card_no(),
                )
                .change_context(
                    UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                        "Failed to parse card number".to_string(),
                    ),
                )?;
                Ok(Self::DecryptedData(payments_grpc::GooglePayDecryptedData {
                    card_exp_month: Some(decrypted_data.card_exp_month.clone().expose().into()),
                    card_exp_year: Some(decrypted_data.card_exp_year.clone().expose().into()),
                    application_primary_account_number: Some(application_primary_account_number),
                    cryptogram: decrypted_data
                        .cryptogram
                        .clone()
                        .map(|cryptogram| cryptogram.expose().into()),
                    eci_indicator: decrypted_data.eci_indicator.clone(),
                }))
            }
        }
    }
}

impl transformers::ForeignTryFrom<hyperswitch_domain_models::payment_address::PaymentAddress>
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
                first_name: details
                    .and_then(|d| d.first_name.as_ref().map(|s| s.clone().expose().into())),
                last_name: details
                    .and_then(|d| d.last_name.as_ref().map(|s| s.clone().expose().into())),
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
                    .and_then(|d| d.first_name.as_ref().map(|s| s.peek().to_string().into())),
                last_name: details
                    .and_then(|d| d.last_name.as_ref().map(|s| s.peek().to_string().into())),
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
                        .and_then(|d| d.first_name.as_ref().map(|s| s.peek().to_string().into())),
                    last_name: details
                        .and_then(|d| d.last_name.as_ref().map(|s| s.peek().to_string().into())),
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
            shipping_address: shipping,
            billing_address: unified_payment_method_billing.or(billing),
        })
    }
}

impl transformers::ForeignTryFrom<AuthenticationType> for payments_grpc::AuthenticationType {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(auth_type: AuthenticationType) -> Result<Self, Self::Error> {
        match auth_type {
            AuthenticationType::ThreeDs => Ok(Self::ThreeDs),
            AuthenticationType::NoThreeDs => Ok(Self::NoThreeDs),
        }
    }
}

impl transformers::ForeignTryFrom<router_request_types::CompleteAuthorizeRedirectResponse>
    for payments_grpc::RedirectionResponse
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        domain_redirect_response: router_request_types::CompleteAuthorizeRedirectResponse,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            params: domain_redirect_response
                .params
                .map(|params| params.expose()),
            payload: domain_redirect_response
                .payload
                .as_ref()
                .and_then(|val| {
                    let exposed_val = val.clone().expose();
                    exposed_val.as_object().cloned()
                })
                .map(|map| {
                    map.into_iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_string())))
                        .collect::<HashMap<String, String>>()
                })
                .unwrap_or_default(),
        })
    }
}

impl transformers::ForeignTryFrom<router_request_types::BrowserInformation>
    for payments_grpc::BrowserInformation
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        browser_info: router_request_types::BrowserInformation,
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
            referer: browser_info.referer,
        })
    }
}

impl transformers::ForeignTryFrom<storage_enums::CaptureMethod> for payments_grpc::CaptureMethod {
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

impl transformers::ForeignTryFrom<common_enums::BankNames> for payments_grpc::BankNames {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(bank_name: common_enums::BankNames) -> Result<Self, Self::Error> {
        match bank_name {
            common_enums::BankNames::AmericanExpress => Ok(Self::AmericanExpress),
            common_enums::BankNames::AffinBank => Ok(Self::AffinBank),
            common_enums::BankNames::AgroBank => Ok(Self::AgroBank),
            common_enums::BankNames::AllianceBank => Ok(Self::AllianceBank),
            common_enums::BankNames::AmBank => Ok(Self::AmBank),
            common_enums::BankNames::BankOfAmerica => Ok(Self::BankOfAmerica),
            common_enums::BankNames::BankOfChina => Ok(Self::BankOfChina),
            common_enums::BankNames::BankIslam => Ok(Self::BankIslam),
            common_enums::BankNames::BankMuamalat => Ok(Self::BankMuamalat),
            common_enums::BankNames::BankRakyat => Ok(Self::BankRakyat),
            common_enums::BankNames::BankSimpananNasional => Ok(Self::BankSimpananNasional),
            common_enums::BankNames::Barclays => Ok(Self::Barclays),
            common_enums::BankNames::BlikPSP => Ok(Self::BlikPsp),
            common_enums::BankNames::CapitalOne => Ok(Self::CapitalOne),
            common_enums::BankNames::Chase => Ok(Self::Chase),
            common_enums::BankNames::Citi => Ok(Self::Citi),
            common_enums::BankNames::CimbBank => Ok(Self::CimbBank),
            common_enums::BankNames::Discover => Ok(Self::Discover),
            common_enums::BankNames::NavyFederalCreditUnion => Ok(Self::NavyFederalCreditUnion),
            common_enums::BankNames::PentagonFederalCreditUnion => {
                Ok(Self::PentagonFederalCreditUnion)
            }
            common_enums::BankNames::SynchronyBank => Ok(Self::SynchronyBank),
            common_enums::BankNames::WellsFargo => Ok(Self::WellsFargo),
            common_enums::BankNames::AbnAmro => Ok(Self::AbnAmro),
            common_enums::BankNames::AsnBank => Ok(Self::AsnBank),
            common_enums::BankNames::Bunq => Ok(Self::Bunq),
            common_enums::BankNames::Handelsbanken => Ok(Self::Handelsbanken),
            common_enums::BankNames::HongLeongBank => Ok(Self::HongLeongBank),
            common_enums::BankNames::HsbcBank => Ok(Self::HsbcBank),
            common_enums::BankNames::Ing => Ok(Self::Ing),
            common_enums::BankNames::Knab => Ok(Self::Knab),
            common_enums::BankNames::KuwaitFinanceHouse => Ok(Self::KuwaitFinanceHouse),
            common_enums::BankNames::Moneyou => Ok(Self::Moneyou),
            common_enums::BankNames::Rabobank => Ok(Self::Rabobank),
            common_enums::BankNames::Regiobank => Ok(Self::Regiobank),
            common_enums::BankNames::Revolut => Ok(Self::Revolut),
            common_enums::BankNames::SnsBank => Ok(Self::SnsBank),
            common_enums::BankNames::TriodosBank => Ok(Self::TriodosBank),
            common_enums::BankNames::VanLanschot => Ok(Self::VanLanschot),
            common_enums::BankNames::ArzteUndApothekerBank => Ok(Self::ArzteUndApothekerBank),
            common_enums::BankNames::AustrianAnadiBankAg => Ok(Self::AustrianAnadiBankAg),
            common_enums::BankNames::BankAustria => Ok(Self::BankAustria),
            common_enums::BankNames::Bank99Ag => Ok(Self::Bank99Ag),
            common_enums::BankNames::BankhausCarlSpangler => Ok(Self::BankhausCarlSpangler),
            common_enums::BankNames::BankhausSchelhammerUndSchatteraAg => {
                Ok(Self::BankhausSchelhammerUndSchatteraAg)
            }
            common_enums::BankNames::BankMillennium => Ok(Self::BankMillennium),
            common_enums::BankNames::BankPEKAOSA => Ok(Self::BankPekaoSa),
            common_enums::BankNames::BawagPskAg => Ok(Self::BawagPskAg),
            common_enums::BankNames::BksBankAg => Ok(Self::BksBankAg),
            common_enums::BankNames::BrullKallmusBankAg => Ok(Self::BrullKallmusBankAg),
            common_enums::BankNames::BtvVierLanderBank => Ok(Self::BtvVierLanderBank),
            common_enums::BankNames::CapitalBankGraweGruppeAg => Ok(Self::CapitalBankGraweGruppeAg),
            common_enums::BankNames::CeskaSporitelna => Ok(Self::CeskaSporitelna),
            common_enums::BankNames::Dolomitenbank => Ok(Self::Dolomitenbank),
            common_enums::BankNames::EasybankAg => Ok(Self::EasybankAg),
            common_enums::BankNames::EPlatbyVUB => Ok(Self::EPlatbyVub),
            common_enums::BankNames::ErsteBankUndSparkassen => Ok(Self::ErsteBankUndSparkassen),
            common_enums::BankNames::FrieslandBank => Ok(Self::FrieslandBank),
            common_enums::BankNames::HypoAlpeadriabankInternationalAg => {
                Ok(Self::HypoAlpeadriabankInternationalAg)
            }
            common_enums::BankNames::HypoNoeLbFurNiederosterreichUWien => {
                Ok(Self::HypoNoeLbFurNiederosterreichUWien)
            }
            common_enums::BankNames::HypoOberosterreichSalzburgSteiermark => {
                Ok(Self::HypoOberosterreichSalzburgSteiermark)
            }
            common_enums::BankNames::HypoTirolBankAg => Ok(Self::HypoTirolBankAg),
            common_enums::BankNames::HypoVorarlbergBankAg => Ok(Self::HypoVorarlbergBankAg),
            common_enums::BankNames::HypoBankBurgenlandAktiengesellschaft => {
                Ok(Self::HypoBankBurgenlandAktiengesellschaft)
            }
            common_enums::BankNames::KomercniBanka => Ok(Self::KomercniBanka),
            common_enums::BankNames::MBank => Ok(Self::MBank),
            common_enums::BankNames::MarchfelderBank => Ok(Self::MarchfelderBank),
            common_enums::BankNames::Maybank => Ok(Self::Maybank),
            common_enums::BankNames::OberbankAg => Ok(Self::OberbankAg),
            common_enums::BankNames::OsterreichischeArzteUndApothekerbank => {
                Ok(Self::OsterreichischeArzteUndApothekerbank)
            }
            common_enums::BankNames::OcbcBank => Ok(Self::OcbcBank),
            common_enums::BankNames::PayWithING => Ok(Self::PayWithIng),
            common_enums::BankNames::PlaceZIPKO => Ok(Self::PlaceZipko),
            common_enums::BankNames::PlatnoscOnlineKartaPlatnicza => {
                Ok(Self::PlatnoscOnlineKartaPlatnicza)
            }
            common_enums::BankNames::PosojilnicaBankEGen => Ok(Self::PosojilnicaBankEGen),
            common_enums::BankNames::PostovaBanka => Ok(Self::PostovaBanka),
            common_enums::BankNames::PublicBank => Ok(Self::PublicBank),
            common_enums::BankNames::RaiffeisenBankengruppeOsterreich => {
                Ok(Self::RaiffeisenBankengruppeOsterreich)
            }
            common_enums::BankNames::RhbBank => Ok(Self::RhbBank),
            common_enums::BankNames::SchelhammerCapitalBankAg => Ok(Self::SchelhammerCapitalBankAg),
            common_enums::BankNames::StandardCharteredBank => Ok(Self::StandardCharteredBank),
            common_enums::BankNames::SchoellerbankAg => Ok(Self::SchoellerbankAg),
            common_enums::BankNames::SpardaBankWien => Ok(Self::SpardaBankWien),
            common_enums::BankNames::SporoPay => Ok(Self::SporoPay),
            common_enums::BankNames::SantanderPrzelew24 => Ok(Self::SantanderPrzelew24),
            common_enums::BankNames::TatraPay => Ok(Self::TatraPay),
            common_enums::BankNames::Viamo => Ok(Self::Viamo),
            common_enums::BankNames::VolksbankGruppe => Ok(Self::VolksbankGruppe),
            common_enums::BankNames::VolkskreditbankAg => Ok(Self::VolkskreditbankAg),
            common_enums::BankNames::VrBankBraunau => Ok(Self::VrBankBraunau),
            common_enums::BankNames::UobBank => Ok(Self::UobBank),
            common_enums::BankNames::PayWithAliorBank => Ok(Self::PayWithAliorBank),
            common_enums::BankNames::BankiSpoldzielcze => Ok(Self::BankiSpoldzielcze),
            common_enums::BankNames::PayWithInteligo => Ok(Self::PayWithInteligo),
            common_enums::BankNames::BNPParibasPoland => Ok(Self::BnpParibasPoland),
            common_enums::BankNames::BankNowySA => Ok(Self::BankNowySa),
            common_enums::BankNames::CreditAgricole => Ok(Self::CreditAgricole),
            common_enums::BankNames::PayWithBOS => Ok(Self::PayWithBos),
            common_enums::BankNames::PayWithCitiHandlowy => Ok(Self::PayWithCitiHandlowy),
            common_enums::BankNames::PayWithPlusBank => Ok(Self::PayWithPlusBank),
            common_enums::BankNames::ToyotaBank => Ok(Self::ToyotaBank),
            common_enums::BankNames::VeloBank => Ok(Self::VeloBank),
            common_enums::BankNames::ETransferPocztowy24 => Ok(Self::ETransferPocztowy24),
            common_enums::BankNames::PlusBank => Ok(Self::PlusBank),
            common_enums::BankNames::EtransferPocztowy24 => Ok(Self::ETransferPocztowy24),
            common_enums::BankNames::BankiSpbdzielcze => Ok(Self::BankiSpbdzielcze),
            common_enums::BankNames::BankNowyBfgSa => Ok(Self::BankNowyBfgSa),
            common_enums::BankNames::GetinBank => Ok(Self::GetinBank),
            common_enums::BankNames::Blik => Ok(Self::BlikPoland),
            common_enums::BankNames::NoblePay => Ok(Self::NoblePay),
            common_enums::BankNames::IdeaBank => Ok(Self::IdeaBank),
            common_enums::BankNames::EnveloBank => Ok(Self::EnveloBank),
            common_enums::BankNames::NestPrzelew => Ok(Self::NestPrzelew),
            common_enums::BankNames::MbankMtransfer => Ok(Self::MbankMtransfer),
            common_enums::BankNames::Inteligo => Ok(Self::Inteligo),
            common_enums::BankNames::PbacZIpko => Ok(Self::PbacZIpko),
            common_enums::BankNames::BnpParibas => Ok(Self::BnpParibas),
            common_enums::BankNames::BankPekaoSa => Ok(Self::BankPekaoSa),
            common_enums::BankNames::VolkswagenBank => Ok(Self::VolkswagenBank),
            common_enums::BankNames::AliorBank => Ok(Self::AliorBank),
            common_enums::BankNames::Boz => Ok(Self::Boz),
            common_enums::BankNames::BangkokBank => Ok(Self::BangkokBank),
            common_enums::BankNames::KrungsriBank => Ok(Self::KrungsriBank),
            common_enums::BankNames::KrungThaiBank => Ok(Self::KrungThaiBank),
            common_enums::BankNames::TheSiamCommercialBank => Ok(Self::TheSiamCommercialBank),
            common_enums::BankNames::KasikornBank => Ok(Self::KasikornBank),
            common_enums::BankNames::OpenBankSuccess => Ok(Self::OpenBankSuccess),
            common_enums::BankNames::OpenBankFailure => Ok(Self::OpenBankFailure),
            common_enums::BankNames::OpenBankCancelled => Ok(Self::OpenBankCancelled),
            common_enums::BankNames::Aib => Ok(Self::Aib),
            common_enums::BankNames::BankOfScotland => Ok(Self::BankOfScotland),
            common_enums::BankNames::DanskeBank => Ok(Self::DanskeBank),
            common_enums::BankNames::FirstDirect => Ok(Self::FirstDirect),
            common_enums::BankNames::FirstTrust => Ok(Self::FirstTrust),
            common_enums::BankNames::Halifax => Ok(Self::Halifax),
            common_enums::BankNames::Lloyds => Ok(Self::Lloyds),
            common_enums::BankNames::Monzo => Ok(Self::Monzo),
            common_enums::BankNames::NatWest => Ok(Self::NatWest),
            common_enums::BankNames::NationwideBank => Ok(Self::NationwideBank),
            common_enums::BankNames::RoyalBankOfScotland => Ok(Self::RoyalBankOfScotland),
            common_enums::BankNames::Starling => Ok(Self::Starling),
            common_enums::BankNames::TsbBank => Ok(Self::TsbBank),
            common_enums::BankNames::TescoBank => Ok(Self::TescoBank),
            common_enums::BankNames::UlsterBank => Ok(Self::UlsterBank),
            common_enums::BankNames::Yoursafe => Ok(Self::Yoursafe),
            common_enums::BankNames::N26 => Ok(Self::N26),
            common_enums::BankNames::NationaleNederlanden => Ok(Self::NationaleNederlanden),
        }
    }
}

impl transformers::ForeignTryFrom<common_enums::BankType> for payments_grpc::BankType {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(bank_type: common_enums::BankType) -> Result<Self, Self::Error> {
        match bank_type {
            common_enums::BankType::Checking => Ok(Self::Checking),
            common_enums::BankType::Savings => Ok(Self::Savings),
        }
    }
}

impl transformers::ForeignTryFrom<common_enums::BankHolderType> for payments_grpc::BankHolderType {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        bank_holder_type: common_enums::BankHolderType,
    ) -> Result<Self, Self::Error> {
        match bank_holder_type {
            common_enums::BankHolderType::Personal => Ok(Self::Personal),
            common_enums::BankHolderType::Business => Ok(Self::Business),
        }
    }
}

impl transformers::ForeignTryFrom<AuthenticationData> for payments_grpc::AuthenticationData {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(authentication_data: AuthenticationData) -> Result<Self, Self::Error> {
        Ok(Self {
            eci: authentication_data.eci,
            cavv: Some(authentication_data.cavv.expose()),
            threeds_server_transaction_id: authentication_data.threeds_server_transaction_id.map(
                |id| Identifier {
                    id_type: Some(payments_grpc::identifier::IdType::Id(id)),
                },
            ),
            message_version: authentication_data
                .message_version
                .map(|message_version| message_version.to_string()),
            ds_transaction_id: authentication_data.ds_trans_id,
            trans_status: None,
            acs_transaction_id: authentication_data.acs_trans_id,
            transaction_id: None,
            ucaf_collection_indicator: None,
            exemption_indicator: authentication_data
                .exemption_indicator
                .map(payments_grpc::ExemptionIndicator::foreign_from)
                .map(i32::from),
            network_params: authentication_data
                .cb_network_params
                .map(payments_grpc::NetworkParams::foreign_try_from)
                .transpose()?,
        })
    }
}

impl transformers::ForeignTryFrom<router_request_types::UcsAuthenticationData>
    for payments_grpc::AuthenticationData
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        authentication_data: router_request_types::UcsAuthenticationData,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            eci: authentication_data.eci,
            cavv: authentication_data.cavv.map(ExposeInterface::expose),
            threeds_server_transaction_id: authentication_data.threeds_server_transaction_id.map(
                |id| Identifier {
                    id_type: Some(payments_grpc::identifier::IdType::Id(id)),
                },
            ),
            message_version: authentication_data
                .message_version
                .map(|message_version| message_version.to_string()),
            ds_transaction_id: authentication_data.ds_trans_id,
            trans_status: authentication_data
                .trans_status
                .map(payments_grpc::TransactionStatus::foreign_from)
                .map(i32::from),
            acs_transaction_id: authentication_data.acs_trans_id,
            transaction_id: authentication_data.transaction_id,
            ucaf_collection_indicator: authentication_data.ucaf_collection_indicator,
            exemption_indicator: None,
            network_params: None,
        })
    }
}

impl transformers::ForeignTryFrom<storage_enums::FutureUsage> for payments_grpc::FutureUsage {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(future_usage: storage_enums::FutureUsage) -> Result<Self, Self::Error> {
        match future_usage {
            storage_enums::FutureUsage::OnSession => Ok(Self::OnSession),
            storage_enums::FutureUsage::OffSession => Ok(Self::OffSession),
        }
    }
}

impl
    transformers::ForeignTryFrom<(
        payments_grpc::PaymentServicePostAuthenticateResponse,
        AttemptStatus,
    )> for Result<(PaymentsResponseData, AttemptStatus), ErrorResponse>
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;
    fn foreign_try_from(
        (response, prev_status): (
            payments_grpc::PaymentServicePostAuthenticateResponse,
            AttemptStatus,
        ),
    ) -> Result<Self, Self::Error> {
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

        let resource_id: router_request_types::ResponseId = match response
            .transaction_id
            .as_ref()
            .and_then(|id| id.id_type.clone())
        {
            Some(payments_grpc::identifier::IdType::Id(id)) => {
                router_request_types::ResponseId::ConnectorTransactionId(id)
            }
            Some(payments_grpc::identifier::IdType::EncodedData(encoded_data)) => {
                router_request_types::ResponseId::EncodedData(encoded_data)
            }
            Some(payments_grpc::identifier::IdType::NoResponseIdMarker(_)) | None => {
                router_request_types::ResponseId::NoResponseId
            }
        };

        let (connector_metadata, redirection_data) = match response.redirection_data.clone() {
            Some(redirection_data) => match redirection_data.form_type {
                Some(ref form_type) => match form_type {
                    payments_grpc::redirect_form::FormType::Uri(uri) => {
                        let sdk_uri_info = api_models::payments::SdkUpiUriInformation {
                            sdk_uri: uri.uri.clone(),
                        };
                        (
                            Some(
                                sdk_uri_info
                                    .encode_to_value()
                                    .change_context(UnifiedConnectorServiceError::ParsingFailed)
                                    .attach_printable(
                                        "Failed to serialize SdkUpiUriInformation to JSON value",
                                    )?,
                            ),
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

        let authentication_data = response
            .authentication_data
            .clone()
            .map(router_request_types::UcsAuthenticationData::foreign_try_from)
            .transpose()?
            .map(Box::new);

        let status_code = convert_connector_service_status_code(response.status_code)?;

        let response = if response.error_code.is_some() {
            let attempt_status = match response.status() {
                payments_grpc::PaymentStatus::AttemptStatusUnspecified => None,
                _ => Some(AttemptStatus::foreign_try_from((
                    response.status(),
                    prev_status,
                ))?),
            };

            Err(ErrorResponse {
                code: response.error_code().to_owned(),
                message: response.error_message().to_owned(),
                reason: Some(response.error_reason().to_owned()),
                status_code,
                attempt_status,
                connector_transaction_id: resource_id.get_optional_response_id(),
                connector_response_reference_id,
                network_decline_code: response.network_decline_code.clone(),
                network_advice_code: response.network_advice_code.clone(),
                network_error_message: response.network_error_message.clone(),
                connector_metadata: None,
            })
        } else {
            let status = AttemptStatus::foreign_try_from((response.status(), prev_status))?;

            Ok((
                PaymentsResponseData::TransactionResponse {
                    resource_id,
                    redirection_data: Box::new(redirection_data),
                    mandate_reference: Box::new(None),
                    connector_metadata,
                    network_txn_id: response.network_txn_id.clone(),
                    connector_response_reference_id,
                    incremental_authorization_allowed: None,
                    authentication_data,
                    charges: None,
                },
                status,
            ))
        };

        Ok(response)
    }
}

impl transformers::ForeignTryFrom<payments_grpc::AuthenticationData>
    for router_request_types::UcsAuthenticationData
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;
    fn foreign_try_from(response: payments_grpc::AuthenticationData) -> Result<Self, Self::Error> {
        let payments_grpc::AuthenticationData {
            eci,
            cavv,
            threeds_server_transaction_id,
            message_version,
            ds_transaction_id,
            trans_status,
            acs_transaction_id,
            transaction_id,
            ucaf_collection_indicator,
            exemption_indicator: _,
            network_params: _,
        } = response;
        let trans_status = trans_status
            .map(payments_grpc::TransactionStatus::try_from)
            .transpose()
            .change_context(UnifiedConnectorServiceError::ResponseDeserializationFailed)
            .attach_printable("Failed to convert TransactionStatus from grpc to domain")?
            .map(ForeignFrom::foreign_from);
        Ok(Self {
            trans_status,
            eci,
            cavv: cavv.map(Secret::new),
            threeds_server_transaction_id: threeds_server_transaction_id
                .and_then(|id| id.id_type)
                .and_then(|id_type| match id_type {
                    payments_grpc::identifier::IdType::Id(id) => Some(id),
                    payments_grpc::identifier::IdType::EncodedData(encoded_data) => {
                        Some(encoded_data)
                    }
                    payments_grpc::identifier::IdType::NoResponseIdMarker(_) => None,
                }),
            message_version: message_version
                .map(|message_version_str| {
                    types::SemanticVersion::from_str(message_version_str.as_ref())
                        .change_context(UnifiedConnectorServiceError::ResponseDeserializationFailed)
                        .attach_printable("Failed to Deserialize message_version")
                })
                .transpose()?,
            ds_trans_id: ds_transaction_id,
            acs_trans_id: acs_transaction_id,
            transaction_id,
            ucaf_collection_indicator,
        })
    }
}

impl ForeignFrom<payments_grpc::TransactionStatus> for common_enums::TransactionStatus {
    fn foreign_from(value: payments_grpc::TransactionStatus) -> Self {
        match value {
            payments_grpc::TransactionStatus::Success => Self::Success,
            payments_grpc::TransactionStatus::Failure => Self::Failure,
            payments_grpc::TransactionStatus::VerificationNotPerformed => {
                Self::VerificationNotPerformed
            }
            payments_grpc::TransactionStatus::NotVerified => Self::NotVerified,
            payments_grpc::TransactionStatus::Rejected => Self::Rejected,
            payments_grpc::TransactionStatus::ChallengeRequired => Self::ChallengeRequired,
            payments_grpc::TransactionStatus::ChallengeRequiredDecoupledAuthentication => {
                Self::ChallengeRequiredDecoupledAuthentication
            }
            payments_grpc::TransactionStatus::InformationOnly => Self::InformationOnly,
        }
    }
}

impl ForeignFrom<common_enums::TransactionStatus> for payments_grpc::TransactionStatus {
    fn foreign_from(value: common_enums::TransactionStatus) -> Self {
        match value {
            common_enums::TransactionStatus::Success => Self::Success,
            common_enums::TransactionStatus::Failure => Self::Failure,
            common_enums::TransactionStatus::VerificationNotPerformed => {
                Self::VerificationNotPerformed
            }
            common_enums::TransactionStatus::NotVerified => Self::NotVerified,
            common_enums::TransactionStatus::Rejected => Self::Rejected,
            common_enums::TransactionStatus::ChallengeRequired => Self::ChallengeRequired,
            common_enums::TransactionStatus::ChallengeRequiredDecoupledAuthentication => {
                Self::ChallengeRequiredDecoupledAuthentication
            }
            common_enums::TransactionStatus::InformationOnly => Self::InformationOnly,
        }
    }
}

impl ForeignFrom<api_models::payments::ThreeDsCompletionIndicator>
    for payments_grpc::ThreeDsCompletionIndicator
{
    fn foreign_from(value: api_models::payments::ThreeDsCompletionIndicator) -> Self {
        match value {
            api_models::payments::ThreeDsCompletionIndicator::Success => Self::Success,
            api_models::payments::ThreeDsCompletionIndicator::Failure => Self::Failure,
            api_models::payments::ThreeDsCompletionIndicator::NotAvailable => Self::NotAvailable,
        }
    }
}

impl ForeignFrom<common_enums::ExemptionIndicator> for payments_grpc::ExemptionIndicator {
    fn foreign_from(value: common_enums::ExemptionIndicator) -> Self {
        match value {
            common_enums::ExemptionIndicator::LowValue => Self::LowValue,
            common_enums::ExemptionIndicator::SecureCorporatePayment => {
                Self::SecureCorporatePayment
            }
            common_enums::ExemptionIndicator::TrustedListing => Self::TrustedListing,
            common_enums::ExemptionIndicator::TransactionRiskAssessment => {
                Self::TransactionRiskAssessment
            }
            common_enums::ExemptionIndicator::ThreeDsOutage => Self::ThreeDsOutage,
            common_enums::ExemptionIndicator::ScaDelegation => Self::ScaDelegation,
            common_enums::ExemptionIndicator::OutOfScaScope => Self::OutOfScaScope,
            common_enums::ExemptionIndicator::Other => Self::Other,
            common_enums::ExemptionIndicator::LowRiskProgram => Self::LowRiskProgram,
            common_enums::ExemptionIndicator::RecurringOperation => Self::RecurringOperation,
        }
    }
}

impl ForeignFrom<common_enums::CavvAlgorithm> for payments_grpc::CavvAlgorithm {
    fn foreign_from(value: common_enums::CavvAlgorithm) -> Self {
        match value {
            common_enums::CavvAlgorithm::Zero => Self::Zero,
            common_enums::CavvAlgorithm::One => Self::One,
            common_enums::CavvAlgorithm::Two => Self::Two,
            common_enums::CavvAlgorithm::Three => Self::Three,
            common_enums::CavvAlgorithm::Four => Self::Four,
            common_enums::CavvAlgorithm::A => Self::A,
        }
    }
}

impl transformers::ForeignTryFrom<api_models::payments::CartesBancairesParams>
    for payments_grpc::CartesBancairesParams
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        value: api_models::payments::CartesBancairesParams,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            cavv_algorithm: payments_grpc::CavvAlgorithm::foreign_from(value.cavv_algorithm).into(),
            cb_exemption: value.cb_exemption,
            cb_score: value.cb_score,
        })
    }
}

impl transformers::ForeignTryFrom<api_models::payments::NetworkParams>
    for payments_grpc::NetworkParams
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(value: api_models::payments::NetworkParams) -> Result<Self, Self::Error> {
        Ok(Self {
            cartes_bancaires: value
                .cartes_bancaires
                .map(payments_grpc::CartesBancairesParams::foreign_try_from)
                .transpose()?,
        })
    }
}

impl ForeignFrom<common_enums::PaymentMethodType> for payments_grpc::PaymentMethodType {
    fn foreign_from(value: common_enums::PaymentMethodType) -> Self {
        match value {
            common_enums::PaymentMethodType::Ach => Self::Ach,
            common_enums::PaymentMethodType::Affirm => Self::Affirm,
            common_enums::PaymentMethodType::AfterpayClearpay => Self::AfterpayClearpay,
            common_enums::PaymentMethodType::Alfamart => Self::Alfamart,
            common_enums::PaymentMethodType::AliPay => Self::AliPay,
            common_enums::PaymentMethodType::AliPayHk => Self::AliPayHk,
            common_enums::PaymentMethodType::Alma => Self::Alma,
            common_enums::PaymentMethodType::AmazonPay => Self::AmazonPay,
            common_enums::PaymentMethodType::ApplePay => Self::ApplePay,
            common_enums::PaymentMethodType::Atome => Self::Atome,
            common_enums::PaymentMethodType::Bacs => Self::Bacs,
            common_enums::PaymentMethodType::BancontactCard => Self::BancontactCard,
            common_enums::PaymentMethodType::Becs => Self::Becs,
            common_enums::PaymentMethodType::Benefit => Self::Benefit,
            common_enums::PaymentMethodType::Bizum => Self::Bizum,
            common_enums::PaymentMethodType::Blik => Self::Blik,
            common_enums::PaymentMethodType::Boleto => Self::Boleto,
            common_enums::PaymentMethodType::BcaBankTransfer => Self::BcaBankTransfer,
            common_enums::PaymentMethodType::BniVa => Self::BniVa,
            common_enums::PaymentMethodType::BriVa => Self::BriVa,
            #[cfg(feature = "v2")]
            common_enums::PaymentMethodType::Card => Self::Credit, // Card maps to CREDIT in proto
            common_enums::PaymentMethodType::CardRedirect => Self::CardRedirect,
            common_enums::PaymentMethodType::CimbVa => Self::CimbVa,
            common_enums::PaymentMethodType::ClassicReward => Self::ClassicReward,
            common_enums::PaymentMethodType::Credit => Self::Credit,
            common_enums::PaymentMethodType::CryptoCurrency => Self::CryptoCurrency,
            common_enums::PaymentMethodType::Cashapp => Self::Cashapp,
            common_enums::PaymentMethodType::Dana => Self::Dana,
            common_enums::PaymentMethodType::DanamonVa => Self::DanamonVa,
            common_enums::PaymentMethodType::Debit => Self::Debit,
            common_enums::PaymentMethodType::DuitNow => Self::DuitNow,
            common_enums::PaymentMethodType::Efecty => Self::Efecty,
            common_enums::PaymentMethodType::Eft => Self::Eft,
            common_enums::PaymentMethodType::Eps => Self::Eps,
            common_enums::PaymentMethodType::Fps => Self::Fps,
            common_enums::PaymentMethodType::Evoucher => Self::Evoucher,
            common_enums::PaymentMethodType::Giropay => Self::Giropay,
            common_enums::PaymentMethodType::Givex => Self::Givex,
            common_enums::PaymentMethodType::GooglePay => Self::GooglePay,
            common_enums::PaymentMethodType::GoPay => Self::GoPay,
            common_enums::PaymentMethodType::Gcash => Self::Gcash,
            common_enums::PaymentMethodType::Ideal => Self::Ideal,
            common_enums::PaymentMethodType::Interac => Self::Interac,
            common_enums::PaymentMethodType::Indomaret => Self::Indomaret,
            common_enums::PaymentMethodType::KakaoPay => Self::KakaoPay,
            common_enums::PaymentMethodType::LocalBankRedirect => Self::LocalBankRedirect,
            common_enums::PaymentMethodType::MandiriVa => Self::MandiriVa,
            common_enums::PaymentMethodType::Knet => Self::Knet,
            common_enums::PaymentMethodType::MbWay => Self::MbWay,
            common_enums::PaymentMethodType::MobilePay => Self::MobilePay,
            common_enums::PaymentMethodType::Momo => Self::Momo,
            common_enums::PaymentMethodType::MomoAtm => Self::MomoAtm,
            common_enums::PaymentMethodType::Multibanco => Self::Multibanco,
            common_enums::PaymentMethodType::OnlineBankingThailand => Self::OnlineBankingThailand,
            common_enums::PaymentMethodType::OnlineBankingCzechRepublic => {
                Self::OnlineBankingCzechRepublic
            }
            common_enums::PaymentMethodType::OnlineBankingFinland => Self::OnlineBankingFinland,
            common_enums::PaymentMethodType::OnlineBankingFpx => Self::OnlineBankingFpx,
            common_enums::PaymentMethodType::OnlineBankingPoland => Self::OnlineBankingPoland,
            common_enums::PaymentMethodType::OnlineBankingSlovakia => Self::OnlineBankingSlovakia,
            common_enums::PaymentMethodType::Oxxo => Self::Oxxo,
            common_enums::PaymentMethodType::PagoEfectivo => Self::PagoEfectivo,
            common_enums::PaymentMethodType::PermataBankTransfer => Self::PermataBankTransfer,
            common_enums::PaymentMethodType::OpenBankingUk => Self::OpenBankingUk,
            common_enums::PaymentMethodType::PayBright => Self::PayBright,
            common_enums::PaymentMethodType::Paypal => Self::PayPal,
            common_enums::PaymentMethodType::Paze => Self::Paze,
            common_enums::PaymentMethodType::Pix => Self::Pix,
            common_enums::PaymentMethodType::PaySafeCard => Self::PaySafeCard,
            common_enums::PaymentMethodType::Przelewy24 => Self::Przelewy24,
            common_enums::PaymentMethodType::PromptPay => Self::PromptPay,
            common_enums::PaymentMethodType::Pse => Self::Pse,
            common_enums::PaymentMethodType::RedCompra => Self::RedCompra,
            common_enums::PaymentMethodType::RedPagos => Self::RedPagos,
            common_enums::PaymentMethodType::SamsungPay => Self::SamsungPay,
            common_enums::PaymentMethodType::Sepa => Self::Sepa,
            common_enums::PaymentMethodType::SepaBankTransfer => Self::SepaBankTransfer,
            common_enums::PaymentMethodType::Sofort => Self::Sofort,
            common_enums::PaymentMethodType::Swish => Self::Swish,
            common_enums::PaymentMethodType::TouchNGo => Self::TouchNGo,
            common_enums::PaymentMethodType::Trustly => Self::Trustly,
            common_enums::PaymentMethodType::Twint => Self::Twint,
            common_enums::PaymentMethodType::UpiCollect => Self::UpiCollect,
            common_enums::PaymentMethodType::UpiIntent => Self::UpiIntent,
            common_enums::PaymentMethodType::UpiQr => Self::UpiQr,
            common_enums::PaymentMethodType::Vipps => Self::Vipps,
            common_enums::PaymentMethodType::VietQr => Self::VietQr,
            common_enums::PaymentMethodType::Venmo => Self::Venmo,
            common_enums::PaymentMethodType::Walley => Self::Walley,
            common_enums::PaymentMethodType::WeChatPay => Self::WeChatPay,
            common_enums::PaymentMethodType::SevenEleven => Self::SevenEleven,
            common_enums::PaymentMethodType::Lawson => Self::Lawson,
            common_enums::PaymentMethodType::MiniStop => Self::MiniStop,
            common_enums::PaymentMethodType::FamilyMart => Self::FamilyMart,
            common_enums::PaymentMethodType::Seicomart => Self::Seicomart,
            common_enums::PaymentMethodType::PayEasy => Self::PayEasy,
            common_enums::PaymentMethodType::LocalBankTransfer => Self::LocalBankTransfer,
            common_enums::PaymentMethodType::OpenBankingPIS => Self::OpenBankingPis,
            common_enums::PaymentMethodType::DirectCarrierBilling => Self::DirectCarrierBilling,
            common_enums::PaymentMethodType::InstantBankTransfer => Self::InstantBankTransfer,
            common_enums::PaymentMethodType::RevolutPay => Self::RevolutPay,
            // Variants that don't have direct proto equivalents
            _ => Self::Unspecified,
        }
    }
}

impl
    transformers::ForeignTryFrom<(
        payments_grpc::PaymentServiceCreateOrderResponse,
        AttemptStatus,
    )> for Result<(PaymentsResponseData, AttemptStatus), ErrorResponse>
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        (response, prev_status): (
            payments_grpc::PaymentServiceCreateOrderResponse,
            AttemptStatus,
        ),
    ) -> Result<Self, Self::Error> {
        let status_code = convert_connector_service_status_code(response.status_code)?;

        let response = if response.error_code.is_some() {
            let attempt_status = match response.status() {
                payments_grpc::PaymentStatus::AttemptStatusUnspecified => None,
                _ => Some(AttemptStatus::foreign_try_from((
                    response.status(),
                    prev_status,
                ))?),
            };

            Err(ErrorResponse {
                code: response.error_code().to_owned(),
                message: response.error_message().to_owned(),
                reason: Some(response.error_message().to_owned()),
                status_code,
                attempt_status,
                connector_transaction_id: None,
                connector_response_reference_id: None,
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            let order_id = response
                .order_id
                .clone()
                .and_then(|id| id.id_type)
                .and_then(|id_type| match id_type {
                    payments_grpc::identifier::IdType::Id(id) => Some(id),
                    payments_grpc::identifier::IdType::EncodedData(encoded_data) => {
                        Some(encoded_data)
                    }
                    payments_grpc::identifier::IdType::NoResponseIdMarker(_) => None,
                })
                .ok_or(UnifiedConnectorServiceError::ResponseDeserializationFailed)?;

            let status = AttemptStatus::foreign_try_from((response.status(), prev_status))?;

            let session_token = response
                .session_token
                .map(SessionToken::foreign_try_from)
                .transpose()?;

            // For order creation, we typically return a successful response with the order_id
            // Since this is not a standard payment response, we'll create a simple success response
            Ok((
                PaymentsResponseData::PaymentsCreateOrderResponse {
                    order_id,
                    session_token,
                },
                status,
            ))
        };

        Ok(response)
    }
}

impl transformers::ForeignTryFrom<payments_grpc::PaymentServiceCreatePaymentMethodTokenResponse>
    for Result<PaymentsResponseData, ErrorResponse>
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        response: payments_grpc::PaymentServiceCreatePaymentMethodTokenResponse,
    ) -> Result<Self, Self::Error> {
        let status_code = convert_connector_service_status_code(response.status_code)?;

        let response = if response.error_code.is_some() {
            Err(ErrorResponse {
                code: response.error_code().to_owned(),
                message: response.error_message().to_owned(),
                reason: Some(response.error_message().to_owned()),
                status_code,
                attempt_status: None,
                connector_transaction_id: None,
                connector_response_reference_id: None,
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            Ok(PaymentsResponseData::TokenizationResponse {
                token: response.payment_method_token,
            })
        };

        Ok(response)
    }
}

impl transformers::ForeignTryFrom<payments_grpc::PaymentServiceIncrementalAuthorizationResponse>
    for Result<PaymentsResponseData, ErrorResponse>
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        response: payments_grpc::PaymentServiceIncrementalAuthorizationResponse,
    ) -> Result<Self, Self::Error> {
        let status_code = convert_connector_service_status_code(response.status_code)?;

        let response = if response.error_code.is_some() {
            Err(ErrorResponse {
                code: response.error_code().to_owned(),
                message: response.error_message().to_owned(),
                reason: response.error_reason,
                status_code,
                attempt_status: None,
                connector_transaction_id: None,
                connector_response_reference_id: None,
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            Ok(PaymentsResponseData::IncrementalAuthorizationResponse {
                status: AuthorizationStatus::foreign_from(response.status()),
                connector_authorization_id: response.connector_authorization_id,
                error_code: response.error_code,
                error_message: response.error_message,
            })
        };

        Ok(response)
    }
}

impl transformers::ForeignTryFrom<payments_grpc::PaymentServiceSdkSessionTokenResponse>
    for Result<PaymentsResponseData, ErrorResponse>
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        response: payments_grpc::PaymentServiceSdkSessionTokenResponse,
    ) -> Result<Self, Self::Error> {
        let status_code = convert_connector_service_status_code(response.status_code)?;

        let response = if response.error_code.is_some() {
            Err(ErrorResponse {
                code: response.error_code().to_owned(),
                message: response.error_message().to_owned(),
                reason: response.error_reason,
                status_code,
                attempt_status: None,
                connector_transaction_id: None,
                connector_response_reference_id: None,
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            let session_token = match response.session_token {
                Some(session_token) => SessionToken::foreign_try_from(session_token),
                None => {
                    router_env::logger::info!(
                        "Missing session_token in UCS Sdk Session Token Response"
                    );
                    Ok(SessionToken::NoSessionTokenReceived)
                }
            }?;

            Ok(PaymentsResponseData::SessionResponse { session_token })
        };

        Ok(response)
    }
}

impl transformers::ForeignTryFrom<payments_grpc::SdkNextAction> for SdkNextAction {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;
    fn foreign_try_from(value: payments_grpc::SdkNextAction) -> Result<Self, Self::Error> {
        let next_action = match value {
            payments_grpc::SdkNextAction::Confirm
            | payments_grpc::SdkNextAction::NextActionUnspecified => NextActionCall::Confirm,
            payments_grpc::SdkNextAction::PostSessionTokens => NextActionCall::PostSessionTokens,
        };

        Ok(Self { next_action })
    }
}

impl transformers::ForeignTryFrom<payments_grpc::PaypalTransactionInfo> for PaypalTransactionInfo {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;
    fn foreign_try_from(value: payments_grpc::PaypalTransactionInfo) -> Result<Self, Self::Error> {
        let flow = match value.flow() {
            payments_grpc::PaypalFlow::Unspecified => PaypalFlow::Checkout,
            payments_grpc::PaypalFlow::Checkout => PaypalFlow::Checkout,
        };

        let currency_code = common_enums::Currency::foreign_try_from(value.currency_code())?;
        let minor_total_price = MinorUnit::new(value.total_price);
        let required_amount_type = StringMajorUnitForConnector;

        let total_price = required_amount_type
            .convert(minor_total_price, currency_code)
            .change_context(UnifiedConnectorServiceError::SdkSessionTokenFailure)?;

        Ok(Self {
            total_price,
            currency_code,
            flow,
        })
    }
}

impl transformers::ForeignTryFrom<payments_grpc::SessionToken> for SessionToken {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(value: payments_grpc::SessionToken) -> Result<Self, Self::Error> {
        match value.wallet_name {
            Some(session_token::WalletName::GooglePay(gpay_session_token_response)) => {
                let gpay_session = gpay_session_token_response
                    .google_pay_session
                    .ok_or(UnifiedConnectorServiceError::SdkSessionTokenFailure)
                    .attach_printable(
                        "Missing Google Pay Session Token Response in UCS SdkSessionToken Response",
                    )?;

                let gpay_response = GooglePaySessionResponse::foreign_try_from(gpay_session)?;

                Ok(Self::GooglePay(Box::new(
                    GpaySessionTokenResponse::GooglePaySession(gpay_response),
                )))
            }
            Some(session_token::WalletName::ApplePay(apay_session_token_response)) => {
                let apay_response = ApplepaySessionTokenResponse {
                    session_token_data: apay_session_token_response
                        .session_token_data
                        .as_ref()
                        .map(ApplePaySessionResponse::foreign_try_from)
                        .transpose()?,
                    payment_request_data: apay_session_token_response
                        .payment_request_data
                        .as_ref()
                        .map(ApplePayPaymentRequest::foreign_try_from)
                        .transpose()?,
                    connector: apay_session_token_response.connector.clone(),
                    sdk_next_action: SdkNextAction::foreign_try_from(
                        apay_session_token_response.sdk_next_action(),
                    )?,
                    delayed_session_token: apay_session_token_response.delayed_session_token,
                    connector_merchant_id: apay_session_token_response.connector_merchant_id,
                    connector_reference_id: apay_session_token_response.connector_reference_id,
                    connector_sdk_public_key: apay_session_token_response.connector_sdk_public_key,
                };

                Ok(Self::ApplePay(Box::new(apay_response)))
            }
            Some(session_token::WalletName::Paypal(paypal_session_token_response)) => {
                let paypal_session_token_response = PaypalSessionTokenResponse {
                    session_token: paypal_session_token_response.session_token.clone(),
                    connector: paypal_session_token_response.connector.clone(),
                    sdk_next_action: SdkNextAction::foreign_try_from(
                        paypal_session_token_response.sdk_next_action(),
                    )?,
                    client_token: paypal_session_token_response.client_token,
                    transaction_info: paypal_session_token_response
                        .transaction_info
                        .map(PaypalTransactionInfo::foreign_try_from)
                        .transpose()?,
                };

                Ok(Self::Paypal(Box::new(paypal_session_token_response)))
            }
            _ => Err(UnifiedConnectorServiceError::SdkSessionTokenFailure)
                .attach_printable("Missing session_token in UCS Sdk Session Token Response")?,
        }
    }
}

impl transformers::ForeignTryFrom<payments_grpc::ApplePayAddressParameters>
    for ApplePayAddressParameters
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;
    fn foreign_try_from(
        value: payments_grpc::ApplePayAddressParameters,
    ) -> Result<Self, Self::Error> {
        match value {
            payments_grpc::ApplePayAddressParameters::PostalAddress => Ok(Self::PostalAddress),
            payments_grpc::ApplePayAddressParameters::Phone => Ok(Self::Phone),
            payments_grpc::ApplePayAddressParameters::Email => Ok(Self::Email),
            payments_grpc::ApplePayAddressParameters::Unspecified => {
                Err(UnifiedConnectorServiceError::SdkSessionTokenFailure)
                    .attach_printable("Unspecified ApplePayAddressParameters")?
            }
        }
    }
}

impl transformers::ForeignTryFrom<(&payments_grpc::AmountInfo, common_enums::Currency)>
    for AmountInfo
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;
    fn foreign_try_from(
        (value, currency_code): (&payments_grpc::AmountInfo, common_enums::Currency),
    ) -> Result<Self, Self::Error> {
        let minor_amount = MinorUnit::new(value.amount);
        let required_amount_type = StringMajorUnitForConnector;

        let amount = required_amount_type
            .convert(minor_amount, currency_code)
            .change_context(UnifiedConnectorServiceError::SdkSessionTokenFailure)
            .attach_printable("Response amount conversion failed")?;

        Ok(Self {
            label: value.label.clone(),
            total_type: value.total_type.clone(),
            amount,
        })
    }
}

impl transformers::ForeignTryFrom<&payments_grpc::ThirdPartySdkSessionResponse>
    for ThirdPartySdkSessionResponse
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;
    fn foreign_try_from(
        value: &payments_grpc::ThirdPartySdkSessionResponse,
    ) -> Result<Self, Self::Error> {
        let secrets = value
            .secrets
            .as_ref()
            .ok_or(UnifiedConnectorServiceError::ResponseDeserializationFailed)?;

        Ok(Self {
            secrets: SecretInfoToInitiateSdk::foreign_try_from(secrets)?,
        })
    }
}

impl transformers::ForeignTryFrom<&payments_grpc::ApplePaySessionResponse>
    for ApplePaySessionResponse
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;
    fn foreign_try_from(
        value: &payments_grpc::ApplePaySessionResponse,
    ) -> Result<Self, Self::Error> {
        let third_party_sdk = value
            .third_party_sdk
            .as_ref()
            .ok_or(UnifiedConnectorServiceError::ResponseDeserializationFailed)
            .attach_printable("Missing third_party_sdk in ApplePaySessionResponse")?;

        let session_token_data = ThirdPartySdkSessionResponse::foreign_try_from(third_party_sdk)?;

        Ok(Self::ThirdPartySdk(session_token_data))
    }
}

impl transformers::ForeignTryFrom<&payments_grpc::ApplePayPaymentRequest>
    for ApplePayPaymentRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;
    fn foreign_try_from(
        value: &payments_grpc::ApplePayPaymentRequest,
    ) -> Result<Self, Self::Error> {
        let total = value
            .total
            .as_ref()
            .ok_or(UnifiedConnectorServiceError::ResponseDeserializationFailed)
            .attach_printable("Missing total in ApplePayPaymentRequest")?;

        let currency_code = common_enums::Currency::foreign_try_from(value.currency_code())?;
        let country_code = common_enums::CountryAlpha2::foreign_try_from(value.country_code())?;

        Ok(Self {
            country_code,
            currency_code,
            total: AmountInfo::foreign_try_from((total, currency_code))?,
            merchant_capabilities: if value.merchant_capabilities.is_empty() {
                None
            } else {
                Some(value.merchant_capabilities.clone())
            },
            supported_networks: if value.supported_networks.is_empty() {
                None
            } else {
                Some(value.supported_networks.clone())
            },
            merchant_identifier: value.merchant_identifier.clone(),
            required_billing_contact_fields: None,
            required_shipping_contact_fields: None,
            recurring_payment_request: None,
        })
    }
}

impl transformers::ForeignTryFrom<payments_grpc::GooglePaySessionResponse>
    for GooglePaySessionResponse
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;
    fn foreign_try_from(
        value: payments_grpc::GooglePaySessionResponse,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            merchant_info: value
                .merchant_info
                .clone()
                .map(GpayMerchantInfo::foreign_try_from)
                .transpose()?
                .ok_or(UnifiedConnectorServiceError::SdkSessionTokenFailure)
                .attach_printable("Missing merchant_info in GooglePaySessionResponse")?,
            shipping_address_required: value.shipping_address_required,
            email_required: value.email_required,
            shipping_address_parameters: value
                .shipping_address_parameters
                .map(GpayShippingAddressParameters::foreign_try_from)
                .transpose()?
                .ok_or(UnifiedConnectorServiceError::SdkSessionTokenFailure)
                .attach_printable(
                    "Missing shipping_address_parameters in GooglePaySessionResponse",
                )?,
            allowed_payment_methods: value
                .allowed_payment_methods
                .clone()
                .into_iter()
                .map(GpayAllowedPaymentMethods::foreign_try_from)
                .collect::<Result<Vec<GpayAllowedPaymentMethods>, _>>()?,
            transaction_info: value
                .transaction_info
                .clone()
                .map(GpayTransactionInfo::foreign_try_from)
                .transpose()?
                .ok_or(UnifiedConnectorServiceError::SdkSessionTokenFailure)
                .attach_printable("Missing transaction_info in GooglePaySessionResponse")?,
            delayed_session_token: value.delayed_session_token,
            connector: value.connector.clone(),
            sdk_next_action: SdkNextAction::foreign_try_from(value.sdk_next_action())?,
            secrets: value
                .secrets
                .as_ref()
                .map(SecretInfoToInitiateSdk::foreign_try_from)
                .transpose()?,
        })
    }
}

impl transformers::ForeignTryFrom<&payments_grpc::SecretInfoToInitiateSdk>
    for SecretInfoToInitiateSdk
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;
    fn foreign_try_from(
        value: &payments_grpc::SecretInfoToInitiateSdk,
    ) -> Result<Self, Self::Error> {
        let display = value
            .display
            .clone()
            .map(|display| display.expose().into())
            .ok_or(UnifiedConnectorServiceError::SdkSessionTokenFailure)
            .attach_printable("Missing display in SecretInfoToInitiateSdk")?;
        let payment = value.payment.clone().map(|payment| payment.expose().into());

        Ok(Self { display, payment })
    }
}

impl transformers::ForeignTryFrom<payments_grpc::GpayBillingAddressFormat>
    for GpayBillingAddressFormat
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;
    fn foreign_try_from(
        value: payments_grpc::GpayBillingAddressFormat,
    ) -> Result<Self, Self::Error> {
        match value {
            payments_grpc::GpayBillingAddressFormat::Min => Ok(Self::MIN),
            payments_grpc::GpayBillingAddressFormat::Full => Ok(Self::FULL),
            payments_grpc::GpayBillingAddressFormat::BillingAddressFormatUnspecified => {
                Err(UnifiedConnectorServiceError::SdkSessionTokenFailure)
                    .attach_printable("Unspecified GpayBillingAddressFormat")?
            }
        }
    }
}

impl transformers::ForeignTryFrom<payments_grpc::GpayMerchantInfo> for GpayMerchantInfo {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;
    fn foreign_try_from(value: payments_grpc::GpayMerchantInfo) -> Result<Self, Self::Error> {
        Ok(Self {
            merchant_id: value.merchant_id,
            merchant_name: value.merchant_name,
        })
    }
}

impl transformers::ForeignTryFrom<payments_grpc::GpayShippingAddressParameters>
    for GpayShippingAddressParameters
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;
    fn foreign_try_from(
        value: payments_grpc::GpayShippingAddressParameters,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            phone_number_required: value.phone_number_required,
        })
    }
}

impl transformers::ForeignTryFrom<payments_grpc::GpayAllowedPaymentMethods>
    for GpayAllowedPaymentMethods
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;
    fn foreign_try_from(
        value: payments_grpc::GpayAllowedPaymentMethods,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_type: value.payment_method_type,
            parameters: value
                .parameters
                .map(GpayAllowedMethodsParameters::foreign_try_from)
                .transpose()?
                .ok_or(UnifiedConnectorServiceError::SdkSessionTokenFailure)
                .attach_printable("Missing GpayAllowedPaymentMethods parameters")?,
            tokenization_specification: value
                .tokenization_specification
                .map(GpayTokenizationSpecification::foreign_try_from)
                .transpose()?
                .ok_or(UnifiedConnectorServiceError::SdkSessionTokenFailure)
                .attach_printable("Missing GpayAllowedPaymentMethods tokenization_specification")?,
        })
    }
}

impl transformers::ForeignTryFrom<payments_grpc::GpayAllowedMethodsParameters>
    for GpayAllowedMethodsParameters
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;
    fn foreign_try_from(
        value: payments_grpc::GpayAllowedMethodsParameters,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            allowed_auth_methods: value.allowed_auth_methods,
            allowed_card_networks: value.allowed_card_networks,
            billing_address_required: value.billing_address_required,
            billing_address_parameters: value
                .billing_address_parameters
                .map(GpayBillingAddressParameters::foreign_try_from)
                .transpose()?,
            assurance_details_required: value.assurance_details_required,
        })
    }
}

impl transformers::ForeignTryFrom<payments_grpc::GpayBillingAddressParameters>
    for GpayBillingAddressParameters
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;
    fn foreign_try_from(
        value: payments_grpc::GpayBillingAddressParameters,
    ) -> Result<Self, Self::Error> {
        let format = GpayBillingAddressFormat::foreign_try_from(value.format())?;
        Ok(Self {
            phone_number_required: value.phone_number_required,
            format,
        })
    }
}

impl transformers::ForeignTryFrom<payments_grpc::GpayTokenizationSpecification>
    for GpayTokenizationSpecification
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;
    fn foreign_try_from(
        value: payments_grpc::GpayTokenizationSpecification,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            token_specification_type: value.token_specification_type,
            parameters: value
                .parameters
                .map(GpayTokenParameters::foreign_try_from)
                .transpose()?
                .ok_or(UnifiedConnectorServiceError::SdkSessionTokenFailure)
                .attach_printable("Missing GpayTokenizationSpecification parameters")?,
        })
    }
}

impl transformers::ForeignTryFrom<payments_grpc::GpayTokenParameters> for GpayTokenParameters {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;
    fn foreign_try_from(value: payments_grpc::GpayTokenParameters) -> Result<Self, Self::Error> {
        Ok(Self {
            gateway: value.gateway,
            gateway_merchant_id: value.gateway_merchant_id,
            stripe_version: None,
            stripe_publishable_key: None,
            protocol_version: value.protocol_version,
            public_key: value.public_key.map(|pk| pk.expose().into()),
        })
    }
}

impl transformers::ForeignTryFrom<payments_grpc::GpayTransactionInfo> for GpayTransactionInfo {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;
    fn foreign_try_from(value: payments_grpc::GpayTransactionInfo) -> Result<Self, Self::Error> {
        let currency_code = common_enums::Currency::foreign_try_from(value.currency_code())?;
        let country_code = common_enums::CountryAlpha2::foreign_try_from(value.country_code())?;
        let minor_total_price = MinorUnit::new(value.total_price);
        let required_amount_type = StringMajorUnitForConnector;

        let total_price = required_amount_type
            .convert(minor_total_price, currency_code)
            .change_context(UnifiedConnectorServiceError::SdkSessionTokenFailure)
            .attach_printable("Response amount conversion failed")?;

        Ok(Self {
            country_code,
            currency_code,
            total_price_status: value.total_price_status,
            total_price,
        })
    }
}

impl
    transformers::ForeignTryFrom<(
        payments_grpc::PaymentServiceAuthenticateResponse,
        AttemptStatus,
    )> for Result<(PaymentsResponseData, AttemptStatus), ErrorResponse>
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;
    fn foreign_try_from(
        (response, prev_status): (
            payments_grpc::PaymentServiceAuthenticateResponse,
            AttemptStatus,
        ),
    ) -> Result<Self, Self::Error> {
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
        let authentication_data = response
            .authentication_data
            .clone()
            .map(router_request_types::UcsAuthenticationData::foreign_try_from)
            .transpose()?
            .map(Box::new);

        let resource_id: router_request_types::ResponseId = match response
            .transaction_id
            .as_ref()
            .and_then(|id| id.id_type.clone())
        {
            Some(payments_grpc::identifier::IdType::Id(id)) => {
                router_request_types::ResponseId::ConnectorTransactionId(id)
            }
            Some(payments_grpc::identifier::IdType::EncodedData(encoded_data)) => {
                router_request_types::ResponseId::EncodedData(encoded_data)
            }
            Some(payments_grpc::identifier::IdType::NoResponseIdMarker(_)) | None => {
                router_request_types::ResponseId::NoResponseId
            }
        };

        let (connector_metadata, redirection_data) = match response.redirection_data.clone() {
            Some(redirection_data) => match redirection_data.form_type {
                Some(ref form_type) => match form_type {
                    payments_grpc::redirect_form::FormType::Uri(uri) => {
                        let sdk_uri_info = api_models::payments::SdkUpiUriInformation {
                            sdk_uri: uri.uri.clone(),
                        };
                        (
                            Some(
                                sdk_uri_info
                                    .encode_to_value()
                                    .change_context(UnifiedConnectorServiceError::ParsingFailed)
                                    .attach_printable(
                                        "Failed to serialize SdkUpiUriInformation to JSON value",
                                    )?,
                            ),
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
            let attempt_status = match response.status() {
                payments_grpc::PaymentStatus::AttemptStatusUnspecified => None,
                _ => Some(AttemptStatus::foreign_try_from((
                    response.status(),
                    prev_status,
                ))?),
            };

            Err(ErrorResponse {
                code: response.error_code().to_owned(),
                message: response.error_message().to_owned(),
                reason: Some(response.error_reason().to_owned()),
                status_code,
                attempt_status,
                connector_transaction_id: resource_id.get_optional_response_id(),
                connector_response_reference_id,
                network_decline_code: response.network_decline_code.clone(),
                network_advice_code: response.network_advice_code.clone(),
                network_error_message: response.network_error_message.clone(),
                connector_metadata: None,
            })
        } else {
            let status = AttemptStatus::foreign_try_from((response.status(), prev_status))?;

            Ok((
                PaymentsResponseData::TransactionResponse {
                    resource_id,
                    redirection_data: Box::new(redirection_data),
                    mandate_reference: Box::new(None),
                    connector_metadata,
                    network_txn_id: response.network_txn_id.clone(),
                    connector_response_reference_id,
                    incremental_authorization_allowed: None,
                    authentication_data,
                    charges: None,
                },
                status,
            ))
        };

        Ok(response)
    }
}

impl transformers::ForeignTryFrom<common_types::payments::CustomerAcceptance>
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

impl transformers::ForeignTryFrom<&MandateData> for payments_grpc::SetupMandateDetails {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(mandate_data: &MandateData) -> Result<Self, Self::Error> {
        let customer_acceptance = mandate_data
            .customer_acceptance
            .clone()
            .map(payments_grpc::CustomerAcceptance::foreign_try_from)
            .transpose()?;

        // Map the mandate_type from domain type to grpc type
        let mandate_type = mandate_data
            .mandate_type
            .as_ref()
            .map(|domain_mandate_type| match domain_mandate_type {
                MandateDataType::SingleUse(amount_data) => payments_grpc::MandateType {
                    mandate_type: Some(payments_grpc::mandate_type::MandateType::SingleUse(
                        payments_grpc::MandateAmountData {
                            amount: amount_data.amount.get_amount_as_i64(),
                            currency: payments_grpc::Currency::foreign_try_from(
                                amount_data.currency,
                            )
                            .unwrap_or(payments_grpc::Currency::Unspecified)
                            .into(),
                            start_date: amount_data.start_date.map(
                                |dt: time::PrimitiveDateTime| dt.assume_utc().unix_timestamp(),
                            ),
                            end_date: amount_data.end_date.map(|dt: time::PrimitiveDateTime| {
                                dt.assume_utc().unix_timestamp()
                            }),
                        },
                    )),
                },
                MandateDataType::MultiUse(amount_data_opt) => payments_grpc::MandateType {
                    mandate_type: amount_data_opt.as_ref().map(|amount_data| {
                        payments_grpc::mandate_type::MandateType::MultiUse(
                            payments_grpc::MandateAmountData {
                                amount: amount_data.amount.get_amount_as_i64(),
                                currency: payments_grpc::Currency::foreign_try_from(
                                    amount_data.currency,
                                )
                                .unwrap_or(payments_grpc::Currency::Unspecified)
                                .into(),
                                start_date: amount_data.start_date.map(
                                    |dt: time::PrimitiveDateTime| dt.assume_utc().unix_timestamp(),
                                ),
                                end_date: amount_data.end_date.map(
                                    |dt: time::PrimitiveDateTime| dt.assume_utc().unix_timestamp(),
                                ),
                            },
                        )
                    }),
                },
            });

        Ok(Self {
            update_mandate_id: mandate_data.update_mandate_id.clone(),
            customer_acceptance,
            mandate_type,
        })
    }
}

impl ForeignFrom<common_enums::MitCategory> for payments_grpc::MitCategory {
    fn foreign_from(mit_category: common_enums::MitCategory) -> Self {
        match mit_category {
            common_enums::MitCategory::Installment => Self::InstallmentMit,
            common_enums::MitCategory::Recurring => Self::RecurringMit,
            common_enums::MitCategory::Resubmission => Self::ResubmissionMit,
            common_enums::MitCategory::Unscheduled => Self::UnscheduledMit,
        }
    }
}

impl ForeignFrom<&SyncRequestType> for payments_grpc::SyncRequestType {
    fn foreign_from(sync_type: &SyncRequestType) -> Self {
        match sync_type {
            SyncRequestType::MultipleCaptureSync(_) => Self::MultipleCaptureSync,
            SyncRequestType::SinglePaymentSync => Self::SinglePaymentSync,
        }
    }
}

impl ForeignFrom<common_enums::Tokenization> for payments_grpc::Tokenization {
    fn foreign_from(tokenization: common_enums::Tokenization) -> Self {
        match tokenization {
            common_enums::Tokenization::TokenizeAtPsp => Self::TokenizeAtPsp,
            common_enums::Tokenization::SkipPsp => Self::SkipPsp,
        }
    }
}

impl ForeignFrom<payments_grpc::AuthorizationStatus> for AuthorizationStatus {
    fn foreign_from(grpc_status: payments_grpc::AuthorizationStatus) -> Self {
        match grpc_status {
            payments_grpc::AuthorizationStatus::AuthorizationSuccess => Self::Success,
            payments_grpc::AuthorizationStatus::AuthorizationFailure => Self::Failure,
            payments_grpc::AuthorizationStatus::AuthorizationProcessing => Self::Processing,
            payments_grpc::AuthorizationStatus::AuthorizationUnresolved => Self::Unresolved,
            payments_grpc::AuthorizationStatus::Unspecified => Self::Processing,
        }
    }
}

impl ForeignFrom<&common_types::payments::BillingDescriptor> for payments_grpc::BillingDescriptor {
    fn foreign_from(billing_descriptor: &common_types::payments::BillingDescriptor) -> Self {
        Self {
            name: billing_descriptor
                .name
                .clone()
                .map(|name| name.expose().into()),
            city: billing_descriptor
                .city
                .clone()
                .map(|city| city.expose().into()),
            phone: billing_descriptor
                .phone
                .clone()
                .map(|phone| phone.expose().into()),
            statement_descriptor: billing_descriptor.statement_descriptor.clone(),
            statement_descriptor_suffix: billing_descriptor.statement_descriptor_suffix.clone(),
            reference: billing_descriptor.reference.clone(),
        }
    }
}

impl transformers::ForeignTryFrom<&common_enums::PaymentChannel> for payments_grpc::PaymentChannel {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        payment_channel: &common_enums::PaymentChannel,
    ) -> Result<Self, Self::Error> {
        match payment_channel {
            common_enums::PaymentChannel::MailOrder => Ok(Self::MailOrder),
            common_enums::PaymentChannel::Ecommerce => Ok(Self::Ecommerce),
            common_enums::PaymentChannel::TelephoneOrder => Ok(Self::TelephoneOrder),
            common_enums::PaymentChannel::Other(_) => Err(
                UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                    "This payment channel variant is not yet supported".to_string(),
                ),
            )?,
        }
    }
}

impl transformers::ForeignTryFrom<payments_grpc::PaymentServiceCreateSessionTokenResponse>
    for Result<PaymentsResponseData, ErrorResponse>
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        response: payments_grpc::PaymentServiceCreateSessionTokenResponse,
    ) -> Result<Self, Self::Error> {
        let status_code = convert_connector_service_status_code(response.status_code)?;

        let response = if response.error_code.is_some() {
            Err(ErrorResponse {
                code: response.error_code().to_owned(),
                message: response.error_message().to_owned(),
                reason: Some(response.error_message().to_owned()),
                status_code,
                attempt_status: None,
                connector_transaction_id: None,
                connector_response_reference_id: None,
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            Ok(PaymentsResponseData::SessionTokenResponse {
                session_token: response.session_token.clone(),
            })
        };

        Ok(response)
    }
}

impl
    transformers::ForeignTryFrom<
        &hyperswitch_interfaces::webhooks::IncomingWebhookRequestDetails<'_>,
    > for payments_grpc::RequestDetails
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

/// Transform UCS webhook response into webhook event data
pub fn transform_ucs_webhook_response(
    response: PaymentServiceTransformResponse,
) -> Result<WebhookTransformData, error_stack::Report<errors::ApiErrorResponse>> {
    let event_type =
        api_models::webhooks::IncomingWebhookEvent::from_ucs_event_type(response.event_type);

    let webhook_transformation_status = if matches!(
        response.transformation_status(),
        payments_grpc::WebhookTransformationStatus::Incomplete
    ) {
        WebhookTransformationStatus::Incomplete
    } else {
        WebhookTransformationStatus::Complete
    };

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
        webhook_transformation_status,
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
    let request_details_grpc =
        <payments_grpc::RequestDetails as transformers::ForeignTryFrom<_>>::foreign_try_from(
            request_details,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to transform webhook request details to gRPC format")?;

    Ok(PaymentServiceTransformRequest {
        request_ref_id: Some(Identifier {
            id_type: Some(payments_grpc::identifier::IdType::Id(format!(
                "{}_{}_{}",
                merchant_id,
                connector_id,
                OffsetDateTime::now_utc().unix_timestamp()
            ))),
        }),
        request_details: Some(request_details_grpc),
        webhook_secrets,
        state: None,
    })
}

// ============================================================================
// REFUND TRANSFORMERS
// ============================================================================

/// Transform RouterData for Execute refund into UCS PaymentServiceRefundRequest
impl transformers::ForeignTryFrom<&RouterData<Execute, RefundsData, RefundsResponseData>>
    for payments_grpc::PaymentServiceRefundRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        router_data: &RouterData<Execute, RefundsData, RefundsResponseData>,
    ) -> Result<Self, Self::Error> {
        let currency = payments_grpc::Currency::foreign_try_from(router_data.request.currency)?;

        let transaction_id = Identifier {
            id_type: Some(payments_grpc::identifier::IdType::Id(
                router_data.request.connector_transaction_id.clone(),
            )),
        };

        let request_ref_id = Some(Identifier {
            id_type: Some(payments_grpc::identifier::IdType::Id(
                router_data.connector_request_reference_id.clone(),
            )),
        });

        // Convert connector_metadata to gRPC format
        let connector_metadata = router_data
            .request
            .connector_metadata
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
            .map(|s| s.into());

        // Convert refund_connector_metadata to gRPC format
        let refund_metadata = router_data
            .request
            .refund_connector_metadata
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
            .map(|s| s.into());

        let state = router_data
            .access_token
            .as_ref()
            .map(ConnectorState::foreign_from);

        let merchant_account_metadata = router_data
            .connector_meta_data
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
            .map(|s| s.into());

        let payment_method_type = router_data
            .payment_method_type
            .map(payments_grpc::PaymentMethodType::foreign_try_from)
            .transpose()?
            .map(|payment_method_type| payment_method_type.into());

        Ok(Self {
            request_ref_id,
            refund_id: router_data.request.refund_id.clone(),
            transaction_id: Some(transaction_id),
            payment_amount: router_data.request.payment_amount,
            currency: currency.into(),
            minor_payment_amount: router_data.request.minor_payment_amount.get_amount_as_i64(),
            refund_amount: router_data.request.refund_amount,
            minor_refund_amount: router_data.request.minor_refund_amount.get_amount_as_i64(),
            reason: router_data.request.reason.clone(),
            webhook_url: router_data.request.webhook_url.clone(),
            merchant_account_id: router_data
                .request
                .merchant_account_id
                .as_ref()
                .map(|id| id.clone().expose().clone()),
            capture_method: router_data
                .request
                .capture_method
                .map(payments_grpc::CaptureMethod::foreign_try_from)
                .transpose()
                .map_err(|_| {
                    UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                        "Failed to convert capture method".to_string(),
                    )
                })?
                .map(i32::from),
            connector_metadata,
            refund_metadata,
            browser_info: router_data
                .request
                .browser_info
                .clone()
                .map(payments_grpc::BrowserInformation::foreign_try_from)
                .transpose()
                .map_err(|_| {
                    UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                        "Failed to convert browser info".to_string(),
                    )
                })?,
            state,
            merchant_account_metadata,
            metadata: None,
            test_mode: router_data.test_mode,
            payment_method_type,
            customer_id: router_data
                .customer_id
                .as_ref()
                .map(|id| id.get_string_repr().to_string()),
        })
    }
}

/// Transform RouterData for RSync refund into UCS RefundServiceGetRequest
impl transformers::ForeignTryFrom<&RouterData<RSync, RefundsData, RefundsResponseData>>
    for payments_grpc::RefundServiceGetRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        router_data: &RouterData<RSync, RefundsData, RefundsResponseData>,
    ) -> Result<Self, Self::Error> {
        let transaction_id = Identifier {
            id_type: Some(payments_grpc::identifier::IdType::Id(
                router_data.request.connector_transaction_id.clone(),
            )),
        };

        let request_ref_id = Some(Identifier {
            id_type: Some(payments_grpc::identifier::IdType::Id(
                router_data.connector_request_reference_id.clone(),
            )),
        });

        let state = router_data
            .access_token
            .as_ref()
            .map(ConnectorState::foreign_from);

        let merchant_account_metadata = router_data
            .connector_meta_data
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
            .map(|s| s.into());

        let payment_method_type = router_data
            .payment_method_type
            .map(payments_grpc::PaymentMethodType::foreign_try_from)
            .transpose()?
            .map(|payment_method_type| payment_method_type.into());

        Ok(Self {
            request_ref_id,
            transaction_id: Some(transaction_id),
            refund_id: router_data.request.connector_refund_id.clone().ok_or(
                UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                    "Missing connector_refund_id for refund sync operation".to_string(),
                ),
            )?,
            refund_reason: router_data.request.reason.clone(),
            browser_info: router_data
                .request
                .browser_info
                .clone()
                .map(payments_grpc::BrowserInformation::foreign_try_from)
                .transpose()
                .map_err(|_| {
                    UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                        "Failed to convert browser info".to_string(),
                    )
                })?,
            state,
            merchant_account_metadata,
            refund_metadata: router_data
                .request
                .refund_connector_metadata
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
                .map(|s| s.into()),
            test_mode: router_data.test_mode,
            payment_method_type,
        })
    }
}

/// Transform UCS RefundResponse into Result<RefundsResponseData, ErrorResponse>
impl transformers::ForeignTryFrom<payments_grpc::RefundResponse>
    for Result<RefundsResponseData, ErrorResponse>
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(response: payments_grpc::RefundResponse) -> Result<Self, Self::Error> {
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
                reason: Some(response.error_reason().to_owned()),
                status_code,
                attempt_status: None,
                connector_transaction_id: None,
                connector_response_reference_id,
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            let refund_status = RefundStatus::foreign_try_from(response.status())?;

            Ok(RefundsResponseData {
                connector_refund_id: response.refund_id,
                refund_status,
            })
        };

        Ok(response)
    }
}

impl transformers::ForeignTryFrom<&RouterData<api::Void, PaymentsCancelData, PaymentsResponseData>>
    for payments_grpc::PaymentServiceVoidRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        router_data: &RouterData<api::Void, PaymentsCancelData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let browser_info = router_data
            .request
            .browser_info
            .clone()
            .map(payments_grpc::BrowserInformation::foreign_try_from)
            .transpose()?;

        let currency = router_data
            .request
            .currency
            .map(payments_grpc::Currency::foreign_try_from)
            .transpose()?;
        let state = router_data
            .access_token
            .as_ref()
            .map(ConnectorState::foreign_from);

        let merchant_account_metadata = router_data
            .connector_meta_data
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
            .map(|s| s.into());

        Ok(Self {
            request_ref_id: Some(Identifier {
                id_type: Some(payments_grpc::identifier::IdType::Id(
                    router_data.connector_request_reference_id.clone(),
                )),
            }),
            transaction_id: if router_data.request.connector_transaction_id.is_empty() {
                None
            } else {
                Some(Identifier {
                    id_type: Some(payments_grpc::identifier::IdType::Id(
                        router_data.request.connector_transaction_id.clone(),
                    )),
                })
            },
            cancellation_reason: router_data.request.cancellation_reason.clone(),
            all_keys_required: None,
            browser_info,
            amount: router_data.request.amount,
            currency: currency.map(|c| c.into()),
            metadata: router_data
                .request
                .metadata
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
                .map(|s| s.into()),
            state,
            connector_metadata: router_data
                .request
                .connector_meta
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
                .map(|s| s.into()),
            merchant_account_metadata,
            test_mode: router_data.test_mode,
            merchant_order_reference_id: router_data.request.merchant_order_reference_id.clone(),
        })
    }
}

impl transformers::ForeignTryFrom<payments_grpc::RefundStatus> for RefundStatus {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(grpc_status: payments_grpc::RefundStatus) -> Result<Self, Self::Error> {
        match grpc_status {
            payments_grpc::RefundStatus::Unspecified => Ok(Self::Pending),
            payments_grpc::RefundStatus::RefundFailure => Ok(Self::Failure),
            payments_grpc::RefundStatus::RefundManualReview => Ok(Self::ManualReview),
            payments_grpc::RefundStatus::RefundPending => Ok(Self::Pending),
            payments_grpc::RefundStatus::RefundSuccess => Ok(Self::Success),
            payments_grpc::RefundStatus::RefundTransactionFailure => Ok(Self::TransactionFailure),
        }
    }
}

impl transformers::ForeignTryFrom<(payments_grpc::PaymentServiceVoidResponse, AttemptStatus)>
    for Result<(PaymentsResponseData, AttemptStatus), ErrorResponse>
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        (response, prev_status): (payments_grpc::PaymentServiceVoidResponse, AttemptStatus),
    ) -> Result<Self, Self::Error> {
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

        let resource_id: router_request_types::ResponseId = match response
            .transaction_id
            .as_ref()
            .and_then(|id| id.id_type.clone())
        {
            Some(payments_grpc::identifier::IdType::Id(id)) => {
                router_request_types::ResponseId::ConnectorTransactionId(id)
            }
            Some(payments_grpc::identifier::IdType::EncodedData(encoded_data)) => {
                router_request_types::ResponseId::EncodedData(encoded_data)
            }
            Some(payments_grpc::identifier::IdType::NoResponseIdMarker(_)) | None => {
                router_request_types::ResponseId::NoResponseId
            }
        };

        let status_code = convert_connector_service_status_code(response.status_code)?;

        // Extract connector_metadata from response if present
        let connector_metadata = response.connector_metadata.clone().and_then(|secret| {
            let connector_metadata = secret.expose();
            serde_json::from_str(&connector_metadata)
                .map_err(|e| {
                    tracing::warn!(
                        serialization_error=?e,
                        metadata=?response.connector_metadata,
                        "Failed to serialize connector_metadata from UCS void response"
                    );
                    e
                })
                .ok()
        });

        let response = if response.error_code.is_some() {
            let attempt_status = match response.status() {
                payments_grpc::PaymentStatus::AttemptStatusUnspecified => None,
                _ => Some(AttemptStatus::foreign_try_from((
                    response.status(),
                    prev_status,
                ))?),
            };

            Err(ErrorResponse {
                code: response.error_code().to_owned(),
                message: response.error_message().to_owned(),
                reason: Some(response.error_reason().to_owned()),
                status_code,
                attempt_status,
                connector_transaction_id: resource_id.get_optional_response_id(),
                connector_response_reference_id,
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            let status = AttemptStatus::foreign_try_from((response.status(), prev_status))?;

            Ok((
                PaymentsResponseData::TransactionResponse {
                    resource_id,
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(response.mandate_reference.map(|grpc_mandate| {
                        hyperswitch_domain_models::router_response_types::MandateReference {
                            connector_mandate_id: grpc_mandate.mandate_id,
                            payment_method_id: grpc_mandate.payment_method_id,
                            mandate_metadata: None,
                            connector_mandate_request_reference_id: grpc_mandate
                                .connector_mandate_request_reference_id,
                        }
                    })),
                    connector_metadata,
                    network_txn_id: None,
                    connector_response_reference_id,
                    incremental_authorization_allowed: response.incremental_authorization_allowed,
                    authentication_data: None,
                    charges: None,
                },
                status,
            ))
        };

        Ok(response)
    }
}

impl
    transformers::ForeignTryFrom<(
        &RouterData<
            hyperswitch_domain_models::router_flow_types::access_token_auth::AccessTokenAuth,
            router_request_types::AccessTokenRequestData,
            AccessToken,
        >,
        common_enums::CallConnectorAction,
    )> for payments_grpc::PaymentServiceCreateAccessTokenRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        (router_data, _call_connector_action): (
            &RouterData<
                hyperswitch_domain_models::router_flow_types::access_token_auth::AccessTokenAuth,
                router_request_types::AccessTokenRequestData,
                AccessToken,
            >,
            common_enums::CallConnectorAction,
        ),
    ) -> Result<Self, Self::Error> {
        let request_ref_id = router_data.connector_request_reference_id.clone();

        let merchant_account_metadata = router_data
            .connector_meta_data
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .change_context(UnifiedConnectorServiceError::RequestEncodingFailed)?
            .map(|s| s.into());

        Ok(Self {
            request_ref_id: Some(Identifier {
                id_type: Some(payments_grpc::identifier::IdType::Id(request_ref_id)),
            }),
            merchant_account_metadata,
            // depricated field we have to remove this/ Default to unspecified connector
            connector: 0_i32,
            metadata: None,
            connector_metadata: None,
            test_mode: router_data.test_mode,
        })
    }
}
