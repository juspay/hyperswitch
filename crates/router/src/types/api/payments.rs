#[cfg(feature = "v1")]
pub use api_models::payments::{
    PaymentListFilterConstraints, PaymentListResponse, PaymentListResponseV2,
};
#[cfg(feature = "v2")]
pub use api_models::payments::{
    PaymentsConfirmIntentRequest, PaymentsCreateIntentRequest, PaymentsIntentResponse,
    PaymentsUpdateIntentRequest,
};
pub use api_models::{
    feature_matrix::{
        ConnectorFeatureMatrixResponse, FeatureMatrixListResponse, FeatureMatrixRequest,
    },
    payments::{
        AcceptanceType, Address, AddressDetails, Amount, AuthenticationForStartResponse, Card,
        CryptoData, CustomerAcceptance, CustomerDetailsResponse, MandateAmountData, MandateData,
        MandateTransactionType, MandateType, MandateValidationFields, NextActionType,
        OnlineMandate, OpenBankingSessionToken, PayLaterData, PaymentIdType,
        PaymentListConstraints, PaymentListFilters, PaymentListFiltersV2, PaymentMethodData,
        PaymentMethodDataRequest, PaymentMethodDataResponse, PaymentOp, PaymentRetrieveBody,
        PaymentRetrieveBodyWithCredentials, PaymentsAggregateResponse, PaymentsApproveRequest,
        PaymentsCancelRequest, PaymentsCaptureRequest, PaymentsCompleteAuthorizeRequest,
        PaymentsDynamicTaxCalculationRequest, PaymentsDynamicTaxCalculationResponse,
        PaymentsExternalAuthenticationRequest, PaymentsIncrementalAuthorizationRequest,
        PaymentsManualUpdateRequest, PaymentsPostSessionTokensRequest,
        PaymentsPostSessionTokensResponse, PaymentsRedirectRequest, PaymentsRedirectionResponse,
        PaymentsRejectRequest, PaymentsRequest, PaymentsResponse, PaymentsResponseForm,
        PaymentsRetrieveRequest, PaymentsSessionRequest, PaymentsSessionResponse,
        PaymentsStartRequest, PgRedirectResponse, PhoneDetails, RedirectionResponse, SessionToken,
        UrlDetails, VerifyRequest, VerifyResponse, WalletData,
    },
};
use error_stack::ResultExt;
pub use hyperswitch_domain_models::router_flow_types::payments::{
    Approve, Authorize, AuthorizeSessionToken, Balance, CalculateTax, Capture, CompleteAuthorize,
    CreateConnectorCustomer, IncrementalAuthorization, InitPayment, PSync, PaymentCreateIntent,
    PaymentGetIntent, PaymentMethodToken, PaymentUpdateIntent, PostProcessing, PostSessionTokens,
    PreProcessing, Reject, SdkSessionUpdate, Session, SetupMandate, Void,
};
pub use hyperswitch_interfaces::api::payments::{
    ConnectorCustomer, MandateSetup, Payment, PaymentApprove, PaymentAuthorize,
    PaymentAuthorizeSessionToken, PaymentCapture, PaymentIncrementalAuthorization,
    PaymentPostSessionTokens, PaymentReject, PaymentSession, PaymentSessionUpdate, PaymentSync,
    PaymentToken, PaymentVoid, PaymentsCompleteAuthorize, PaymentsPostProcessing,
    PaymentsPreProcessing, TaxCalculation,
};

pub use super::payments_v2::{
    ConnectorCustomerV2, MandateSetupV2, PaymentApproveV2, PaymentAuthorizeSessionTokenV2,
    PaymentAuthorizeV2, PaymentCaptureV2, PaymentIncrementalAuthorizationV2,
    PaymentPostSessionTokensV2, PaymentRejectV2, PaymentSessionUpdateV2, PaymentSessionV2,
    PaymentSyncV2, PaymentTokenV2, PaymentV2, PaymentVoidV2, PaymentsCompleteAuthorizeV2,
    PaymentsPostProcessingV2, PaymentsPreProcessingV2, TaxCalculationV2,
};
use crate::core::errors;

pub trait PaymentIdTypeExt {
    #[cfg(feature = "v1")]
    fn get_payment_intent_id(
        &self,
    ) -> errors::CustomResult<common_utils::id_type::PaymentId, errors::ValidationError>;

    #[cfg(feature = "v2")]
    fn get_payment_intent_id(
        &self,
    ) -> errors::CustomResult<common_utils::id_type::GlobalPaymentId, errors::ValidationError>;
}

impl PaymentIdTypeExt for PaymentIdType {
    #[cfg(feature = "v1")]
    fn get_payment_intent_id(
        &self,
    ) -> errors::CustomResult<common_utils::id_type::PaymentId, errors::ValidationError> {
        match self {
            Self::PaymentIntentId(id) => Ok(id.clone()),
            Self::ConnectorTransactionId(_)
            | Self::PaymentAttemptId(_)
            | Self::PreprocessingId(_) => Err(errors::ValidationError::IncorrectValueProvided {
                field_name: "payment_id",
            })
            .attach_printable("Expected payment intent ID but got connector transaction ID"),
        }
    }

    #[cfg(feature = "v2")]
    fn get_payment_intent_id(
        &self,
    ) -> errors::CustomResult<common_utils::id_type::GlobalPaymentId, errors::ValidationError> {
        match self {
            Self::PaymentIntentId(id) => Ok(id.clone()),
            Self::ConnectorTransactionId(_)
            | Self::PaymentAttemptId(_)
            | Self::PreprocessingId(_) => Err(errors::ValidationError::IncorrectValueProvided {
                field_name: "payment_id",
            })
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
        match (&self.mandate_data, &self.recurring_details) {
            (None, None) => Ok(None),
            (Some(_), Some(_)) => Err(errors::ValidationError::InvalidValue {
                message: "Expected one out of recurring_details and mandate_data but got both"
                    .to_string(),
            }
            .into()),
            (_, Some(_)) => Ok(Some(MandateTransactionType::RecurringMandateTransaction)),
            (Some(_), _) => Ok(Some(MandateTransactionType::NewMandateTransaction)),
        }
    }
}

#[cfg(feature = "v1")]
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
            card_holder_name: Some(masking::Secret::new("JohnDoe".to_string())),
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
            amount: Some(Amount::from(common_utils::types::MinorUnit::new(200))),
            payment_method_data: Some(PaymentMethodDataRequest {
                payment_method_data: Some(PaymentMethodData::Card(card())),
                billing: None,
            }),
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
        let sample_1 = PaymentIdType::PaymentIntentId(
            common_utils::id_type::PaymentId::try_from(std::borrow::Cow::Borrowed(
                "test_234565430uolsjdnf48i0",
            ))
            .unwrap(),
        );
        let s_sample_1 = serde_json::to_string(&sample_1).unwrap();
        let ds_sample_1 = serde_json::from_str::<PaymentIdType>(&s_sample_1).unwrap();
        assert_eq!(ds_sample_1, sample_1)
    }
}
