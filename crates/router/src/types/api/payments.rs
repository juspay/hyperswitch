pub use api_models::payments::{
    AcceptanceType, Address, AddressDetails, Amount, AuthenticationForStartResponse, CCard,
    CustomerAcceptance, MandateData, MandateTxnType, MandateType, MandateValidationFields,
    NextAction, NextActionType, OnlineMandate, PayLaterData, PaymentIdType, PaymentListConstraints,
    PaymentListResponse, PaymentMethod, PaymentMethodDataResponse, PaymentOp, PaymentRetrieveBody,
    PaymentsCancelRequest, PaymentsCaptureRequest, PaymentsRedirectRequest,
    PaymentsRedirectionResponse, PaymentsRequest, PaymentsResponse, PaymentsResponseForm,
    PaymentsRetrieveRequest, PaymentsSessionRequest, PaymentsSessionResponse, PaymentsStartRequest,
    PgRedirectResponse, PhoneDetails, RedirectionResponse, SessionToken, UrlDetails, VerifyRequest,
    VerifyResponse, WalletData,
};
use error_stack::{IntoReport, ResultExt};
use masking::PeekInterface;
use time::PrimitiveDateTime;

use crate::{
    core::errors,
    services::api,
    types::{
        self, api as api_types, storage,
        transformers::{Foreign, ForeignInto},
    },
};

pub(crate) trait PaymentsRequestExt {
    fn is_mandate(&self) -> Option<MandateTxnType>;
}

impl PaymentsRequestExt for PaymentsRequest {
    fn is_mandate(&self) -> Option<MandateTxnType> {
        match (&self.mandate_data, &self.mandate_id) {
            (None, None) => None,
            (_, Some(_)) => Some(MandateTxnType::RecurringMandateTxn),
            (Some(_), _) => Some(MandateTxnType::NewMandateTxn),
        }
    }
}

pub(crate) trait CustomerAcceptanceExt {
    fn get_ip_address(&self) -> Option<String>;
    fn get_user_agent(&self) -> Option<String>;
    fn get_accepted_at(&self) -> PrimitiveDateTime;
}

impl CustomerAcceptanceExt for CustomerAcceptance {
    fn get_ip_address(&self) -> Option<String> {
        self.online
            .as_ref()
            .map(|data| data.ip_address.peek().to_owned())
    }

    fn get_user_agent(&self) -> Option<String> {
        self.online.as_ref().map(|data| data.user_agent.clone())
    }

    fn get_accepted_at(&self) -> PrimitiveDateTime {
        self.accepted_at
            .unwrap_or_else(common_utils::date_time::now)
    }
}

impl super::Router for PaymentsRequest {}

// Core related api layer.
#[derive(Debug, Clone)]
pub struct Authorize;
#[derive(Debug, Clone)]
pub struct Capture;

#[derive(Debug, Clone)]
pub struct PSync;
#[derive(Debug, Clone)]
pub struct Void;

#[derive(Debug, Clone)]
pub struct Session;

#[derive(Debug, Clone)]
pub struct Verify;

pub(crate) trait PaymentIdTypeExt {
    fn get_payment_intent_id(&self) -> errors::CustomResult<String, errors::ValidationError>;
}

impl PaymentIdTypeExt for PaymentIdType {
    fn get_payment_intent_id(&self) -> errors::CustomResult<String, errors::ValidationError> {
        match self {
            Self::PaymentIntentId(id) => Ok(id.clone()),
            Self::ConnectorTransactionId(_) | Self::PaymentAttemptId(_) => {
                Err(errors::ValidationError::IncorrectValueProvided {
                    field_name: "payment_id",
                })
                .into_report()
                .attach_printable("Expected payment intent ID but got connector transaction ID")
            }
        }
    }
}

pub(crate) trait MandateValidationFieldsExt {
    fn is_mandate(&self) -> Option<MandateTxnType>;
}

impl MandateValidationFieldsExt for MandateValidationFields {
    fn is_mandate(&self) -> Option<MandateTxnType> {
        match (&self.mandate_data, &self.mandate_id) {
            (None, None) => None,
            (_, Some(_)) => Some(MandateTxnType::RecurringMandateTxn),
            (Some(_), _) => Some(MandateTxnType::NewMandateTxn),
        }
    }
}

impl From<Foreign<storage::PaymentIntent>> for Foreign<PaymentsResponse> {
    fn from(item: Foreign<storage::PaymentIntent>) -> Self {
        let item = item.0;
        PaymentsResponse {
            payment_id: Some(item.payment_id),
            merchant_id: Some(item.merchant_id),
            status: item.status.foreign_into(),
            amount: item.amount,
            amount_capturable: item.amount_captured,
            client_secret: item.client_secret.map(|s| s.into()),
            created: Some(item.created_at),
            currency: item.currency.map(|c| c.to_string()).unwrap_or_default(),
            description: item.description,
            metadata: item.metadata,
            customer_id: item.customer_id,
            ..Default::default()
        }
        .into()
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

pub trait PaymentCapture:
    api::ConnectorIntegration<Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
{
}

pub trait PaymentSession:
    api::ConnectorIntegration<Session, types::PaymentsSessionData, types::PaymentsResponseData>
{
}

pub trait PreVerify:
    api::ConnectorIntegration<Verify, types::VerifyRequestData, types::PaymentsResponseData>
{
}

pub trait Payment:
    api_types::ConnectorCommon
    + PaymentAuthorize
    + PaymentSync
    + PaymentCapture
    + PaymentVoid
    + PreVerify
    + PaymentSession
{
}

#[cfg(test)]
mod payments_test {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[allow(dead_code)]
    fn card() -> CCard {
        CCard {
            card_number: "1234432112344321".to_string().into(),
            card_exp_month: "12".to_string().into(),
            card_exp_year: "99".to_string().into(),
            card_holder_name: "JohnDoe".to_string().into(),
            card_cvc: "123".to_string().into(),
        }
    }

    #[allow(dead_code)]
    fn payments_request() -> PaymentsRequest {
        PaymentsRequest {
            amount: Some(Amount::from(200)),
            payment_method_data: Some(PaymentMethod::Card(card())),
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
