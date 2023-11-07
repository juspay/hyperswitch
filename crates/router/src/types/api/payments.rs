pub use api_models::payments::{
    AcceptanceType, Address, AddressDetails, Amount, AuthenticationForStartResponse, Card,
    CryptoData, CustomerAcceptance, HeaderPayload, MandateAmountData, MandateData,
    MandateTransactionType, MandateType, MandateValidationFields, NextActionType, OnlineMandate,
    PayLaterData, PaymentIdType, PaymentListConstraints, PaymentListFilterConstraints,
    PaymentListFilters, PaymentListResponse, PaymentListResponseV2, PaymentMethodData,
    PaymentMethodDataResponse, PaymentOp, PaymentRetrieveBody, PaymentRetrieveBodyWithCredentials,
    PaymentsApproveRequest, PaymentsCancelRequest, PaymentsCaptureRequest, PaymentsRedirectRequest,
    PaymentsRedirectionResponse, PaymentsRejectRequest, PaymentsRequest, PaymentsResponse,
    PaymentsResponseForm, PaymentsRetrieveRequest, PaymentsSessionRequest, PaymentsSessionResponse,
    PaymentsStartRequest, PgRedirectResponse, PhoneDetails, RedirectionResponse, SessionToken,
    TimeRange, UrlDetails, VerifyRequest, VerifyResponse, WalletData,
};
use error_stack::{IntoReport, ResultExt};

use crate::{
    core::errors,
    services::api,
    types::{self, api as api_types},
};

pub(crate) trait PaymentsRequestExt {
    fn is_mandate(&self) -> Option<MandateTransactionType>;
}

impl PaymentsRequestExt for PaymentsRequest {
    fn is_mandate(&self) -> Option<MandateTransactionType> {
        match (&self.mandate_data, &self.mandate_id) {
            (None, None) => None,
            (_, Some(_)) => Some(MandateTransactionType::RecurringMandateTransaction),
            (Some(_), _) => Some(MandateTransactionType::NewMandateTransaction),
        }
    }
}

impl super::Router for PaymentsRequest {}

// Core related api layer.
#[derive(Debug, Clone)]
pub struct Authorize;

#[derive(Debug, Clone)]
pub struct AuthorizeSessionToken;

#[derive(Debug, Clone)]
pub struct CompleteAuthorize;

#[derive(Debug, Clone)]
pub struct Approve;

// Used in gift cards balance check
#[derive(Debug, Clone)]
pub struct Balance;

#[derive(Debug, Clone)]
pub struct InitPayment;

#[derive(Debug, Clone)]
pub struct Capture;

#[derive(Debug, Clone)]
pub struct PSync;
#[derive(Debug, Clone)]
pub struct Void;

#[derive(Debug, Clone)]
pub struct Reject;

#[derive(Debug, Clone)]
pub struct Session;

#[derive(Debug, Clone)]
pub struct PaymentMethodToken;

#[derive(Debug, Clone)]
pub struct CreateConnectorCustomer;

#[derive(Debug, Clone)]
pub struct SetupMandate;

#[derive(Debug, Clone)]
pub struct PreProcessing;

pub trait PaymentIdTypeExt {
    fn get_payment_intent_id(&self) -> errors::CustomResult<String, errors::ValidationError>;
}

impl PaymentIdTypeExt for PaymentIdType {
    fn get_payment_intent_id(&self) -> errors::CustomResult<String, errors::ValidationError> {
        match self {
            Self::PaymentIntentId(id) => Ok(id.clone()),
            Self::ConnectorTransactionId(_)
            | Self::PaymentAttemptId(_)
            | Self::PreprocessingId(_) => Err(errors::ValidationError::IncorrectValueProvided {
                field_name: "payment_id",
            })
            .into_report()
            .attach_printable("Expected payment intent ID but got connector transaction ID"),
        }
    }
}

pub(crate) trait MandateValidationFieldsExt {
    fn validate_and_get_mandate_type(
        &self,
    ) -> errors::CustomResult<Option<MandateTransactionType>, errors::ValidationError>;
}

impl MandateValidationFieldsExt for MandateValidationFields {
    fn validate_and_get_mandate_type(
        &self,
    ) -> errors::CustomResult<Option<MandateTransactionType>, errors::ValidationError> {
        match (&self.mandate_data, &self.mandate_id) {
            (None, None) => Ok(None),
            (Some(_), Some(_)) => Err(errors::ValidationError::InvalidValue {
                message: "Expected one out of mandate_id and mandate_data but got both".to_string(),
            })
            .into_report(),
            (_, Some(_)) => Ok(Some(MandateTransactionType::RecurringMandateTransaction)),
            (Some(_), _) => Ok(Some(MandateTransactionType::NewMandateTransaction)),
        }
    }
}

// Extract only the last 4 digits of card

pub trait PaymentAuthorize:
    api::ConnectorIntegration<Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
{
}

pub trait PaymentSync:
    api::ConnectorIntegration<PSync, types::PaymentsSyncData, types::PaymentsResponseData>
{
}

pub trait PaymentVoid:
    api::ConnectorIntegration<Void, types::PaymentsCancelData, types::PaymentsResponseData>
{
}

pub trait PaymentApprove:
    api::ConnectorIntegration<Approve, types::PaymentsApproveData, types::PaymentsResponseData>
{
}

pub trait PaymentReject:
    api::ConnectorIntegration<Reject, types::PaymentsRejectData, types::PaymentsResponseData>
{
}

pub trait PaymentCapture:
    api::ConnectorIntegration<Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
{
}

pub trait PaymentSession:
    api::ConnectorIntegration<Session, types::PaymentsSessionData, types::PaymentsResponseData>
{
}

pub trait MandateSetup:
    api::ConnectorIntegration<SetupMandate, types::SetupMandateRequestData, types::PaymentsResponseData>
{
}

pub trait PaymentsCompleteAuthorize:
    api::ConnectorIntegration<
    CompleteAuthorize,
    types::CompleteAuthorizeData,
    types::PaymentsResponseData,
>
{
}

pub trait PaymentToken:
    api::ConnectorIntegration<
    PaymentMethodToken,
    types::PaymentMethodTokenizationData,
    types::PaymentsResponseData,
>
{
}

pub trait ConnectorCustomer:
    api::ConnectorIntegration<
    CreateConnectorCustomer,
    types::ConnectorCustomerData,
    types::PaymentsResponseData,
>
{
}

pub trait PaymentsPreProcessing:
    api::ConnectorIntegration<
    PreProcessing,
    types::PaymentsPreProcessingData,
    types::PaymentsResponseData,
>
{
}

pub trait Payment:
    api_types::ConnectorCommon
    + api_types::ConnectorValidation
    + PaymentAuthorize
    + PaymentsCompleteAuthorize
    + PaymentSync
    + PaymentCapture
    + PaymentVoid
    + PaymentApprove
    + PaymentReject
    + MandateSetup
    + PaymentSession
    + PaymentToken
    + PaymentsPreProcessing
    + ConnectorCustomer
{
}

#[cfg(test)]
mod payments_test {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[allow(dead_code)]
    fn card() -> Card {
        Card {
            card_number: "1234432112344321".to_string().try_into().unwrap(),
            card_exp_month: "12".to_string().into(),
            card_exp_year: "99".to_string().into(),
            card_holder_name: "JohnDoe".to_string().into(),
            card_cvc: "123".to_string().into(),
            card_issuer: Some("HDFC".to_string()),
            card_network: Some(api_models::enums::CardNetwork::Visa),
            bank_code: None,
            card_issuing_country: None,
            card_type: None,
            nick_name: Some(masking::Secret::new("nick_name".into())),
        }
    }

    #[allow(dead_code)]
    fn payments_request() -> PaymentsRequest {
        PaymentsRequest {
            amount: Some(Amount::from(200)),
            payment_method_data: Some(PaymentMethodData::Card(card())),
            ..PaymentsRequest::default()
        }
    }

    //#[test] // FIXME: Fix test
    #[allow(dead_code)]
    fn verify_payments_request() {
        let pay_req = payments_request();
        let serialized =
            serde_json::to_string(&pay_req).expect("error serializing payments request");
        let _deserialized_pay_req: PaymentsRequest =
            serde_json::from_str(&serialized).expect("error de-serializing payments response");
        //assert_eq!(pay_req, deserialized_pay_req)
    }

    // Intended to test the serialization and deserialization of the enum PaymentIdType
    #[test]
    fn test_connector_id_type() {
        let sample_1 = PaymentIdType::PaymentIntentId("test_234565430uolsjdnf48i0".to_string());
        let s_sample_1 = serde_json::to_string(&sample_1).unwrap();
        let ds_sample_1 = serde_json::from_str::<PaymentIdType>(&s_sample_1).unwrap();
        assert_eq!(ds_sample_1, sample_1)
    }
}
