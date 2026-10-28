use base64::Engine;
use common_enums::{enums, AttemptStatus};
use common_utils::{
    consts::BASE64_ENGINE,
    crypto::{self, SignMessage},
    request::Method,
    types::StringMajorUnit,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{PaymentMethodData, WalletData},
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::payments::{Authorize, PSync},
    router_request_types::{PaymentsAuthorizeData, PaymentsSyncData, ResponseId},
    router_response_types::{PaymentsResponseData, RedirectForm},
    types::PaymentsAuthorizeRouterData,
};
use hyperswitch_interfaces::{
    consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE},
    errors,
};
use hyperswitch_masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::ResponseRouterData,
    utils::{self, PaymentsAuthorizeRequestData},
};

/// eSewa signs these request fields, in this order, joined as `key=value` pairs with commas.
const SIGNED_FIELD_NAMES: &str = "total_amount,transaction_uuid,product_code";

pub struct EsewaRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for EsewaRouterData<T> {
    fn from((amount, item): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

/// `api_key` holds the merchant secret key (HMAC key) and `key1` the product code
/// (merchant code, `EPAYTEST` in UAT).
pub struct EsewaAuthType {
    pub(super) secret_key: Secret<String>,
    pub(super) product_code: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for EsewaAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                secret_key: api_key.to_owned(),
                product_code: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

/// Form posted to `api/epay/main/v2/form` (ePay v2). eSewa answers with a 302 to its
/// hosted payment page for the booking it creates.
#[derive(Debug, Serialize)]
pub struct EsewaPaymentsRequest {
    amount: StringMajorUnit,
    /// Tax and charges are not itemised, but eSewa requires them; they are sent as zero.
    tax_amount: StringMajorUnit,
    total_amount: StringMajorUnit,
    transaction_uuid: String,
    product_code: Secret<String>,
    product_service_charge: StringMajorUnit,
    product_delivery_charge: StringMajorUnit,
    success_url: String,
    failure_url: String,
    signed_field_names: &'static str,
    signature: Secret<String>,
}

impl TryFrom<&EsewaRouterData<&PaymentsAuthorizeRouterData>> for EsewaPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &EsewaRouterData<&PaymentsAuthorizeRouterData>) -> Result<Self, Self::Error> {
        let router_data = item.router_data;
        match &router_data.request.payment_method_data {
            PaymentMethodData::Wallet(WalletData::Esewa(_)) => {}
            _ => {
                return Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("eSewa"),
                )
                .into())
            }
        }
        if router_data.request.currency != enums::Currency::NPR {
            return Err(errors::ConnectorError::NotSupported {
                message: format!("{} currency", router_data.request.currency),
                connector: "eSewa".into(),
            }
            .into());
        }

        let auth = EsewaAuthType::try_from(&router_data.connector_auth_type)?;
        let transaction_uuid = router_data.connector_request_reference_id.clone();
        let signature = sign(
            &auth.secret_key,
            &format!(
                "total_amount={},transaction_uuid={},product_code={}",
                item.amount.get_amount_as_string(),
                transaction_uuid,
                auth.product_code.peek()
            ),
        )?;
        // eSewa redirects the customer here on both outcomes (and for pending payments),
        // so both point at the redirect-response route, which always runs PSync.
        let return_url = router_data.request.get_router_return_url()?;

        Ok(Self {
            amount: item.amount.clone(),
            tax_amount: StringMajorUnit::zero(),
            total_amount: item.amount.clone(),
            transaction_uuid,
            product_code: auth.product_code,
            product_service_charge: StringMajorUnit::zero(),
            product_delivery_charge: StringMajorUnit::zero(),
            success_url: return_url.clone(),
            failure_url: return_url,
            signed_field_names: SIGNED_FIELD_NAMES,
            signature,
        })
    }
}

/// HMAC-SHA256 over `message` with the merchant secret key, base64 encoded.
fn sign(
    secret_key: &Secret<String>,
    message: &str,
) -> Result<Secret<String>, error_stack::Report<errors::ConnectorError>> {
    let signature = crypto::HmacSha256
        .sign_message(secret_key.clone().expose().as_bytes(), message.as_bytes())
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
    Ok(Secret::new(BASE64_ENGINE.encode(signature)))
}

/// What the form post returns: the `Location` of eSewa's hosted payment page.
#[derive(Debug, Serialize)]
pub struct EsewaRedirectResponse {
    pub payment_page_url: url::Url,
}

impl
    TryFrom<
        ResponseRouterData<
            Authorize,
            EsewaRedirectResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    > for RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            Authorize,
            EsewaRedirectResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let transaction_uuid = item.data.connector_request_reference_id.clone();
        Ok(Self {
            status: AttemptStatus::AuthenticationPending,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(transaction_uuid.clone()),
                // The page URL carries `bookingId` as a query param; `From<(Url, Method)>`
                // moves it into form fields so the GET form submits it.
                redirection_data: Box::new(Some(RedirectForm::from((
                    item.response.payment_page_url,
                    Method::Get,
                )))),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                network_txn_link_id: None,
                connector_response_reference_id: Some(transaction_uuid),
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
                payment_account_reference: None,
            }),
            ..item.data
        })
    }
}

/// Status check: `GET api/epay/transaction/status/` with these query params. The response
/// is unsigned, so it is fetched server-to-server rather than read from the redirect.
#[derive(Debug, Serialize)]
pub struct EsewaSyncQuery {
    pub product_code: Secret<String>,
    pub total_amount: StringMajorUnit,
    pub transaction_uuid: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EsewaPaymentStatus {
    Complete,
    Pending,
    /// Payment state could not be determined ("halted") on eSewa's side.
    Ambiguous,
    /// No transaction for this uuid and amount, or the 15 minute session expired.
    NotFound,
    /// Reversed by eSewa.
    Canceled,
    FullRefund,
    PartialRefund,
}

impl From<EsewaPaymentStatus> for AttemptStatus {
    fn from(item: EsewaPaymentStatus) -> Self {
        match item {
            EsewaPaymentStatus::Complete => Self::Charged,
            EsewaPaymentStatus::Pending | EsewaPaymentStatus::Ambiguous => Self::Pending,
            EsewaPaymentStatus::NotFound | EsewaPaymentStatus::Canceled => Self::Failure,
            // Refunds happen outside Hyperswitch (merchant portal); the payment itself
            // was captured.
            EsewaPaymentStatus::FullRefund | EsewaPaymentStatus::PartialRefund => Self::Charged,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EsewaStatusResponse {
    pub transaction_uuid: String,
    pub status: EsewaPaymentStatus,
    /// eSewa's reference for a completed payment.
    pub ref_id: Option<String>,
}

/// The status API reports outages as HTTP 200 with an error body, so a sync response is
/// either shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EsewaSyncResponse {
    Status(EsewaStatusResponse),
    Error(EsewaErrorResponse),
}

impl TryFrom<ResponseRouterData<PSync, EsewaSyncResponse, PaymentsSyncData, PaymentsResponseData>>
    for RouterData<PSync, PaymentsSyncData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<PSync, EsewaSyncResponse, PaymentsSyncData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let response = match item.response {
            EsewaSyncResponse::Status(response) => response,
            // Keep the current status: the payment may still complete.
            EsewaSyncResponse::Error(error) => {
                let error = error.into_error_response(item.http_code, Some(item.data.status));
                return Ok(Self {
                    response: Err(error),
                    ..item.data
                });
            }
        };

        let status = AttemptStatus::from(response.status);
        let response_data = if status == AttemptStatus::Failure {
            let message = match response.status {
                EsewaPaymentStatus::NotFound => "Payment not found or eSewa session expired",
                _ => "Payment canceled by eSewa",
            };
            Err(ErrorResponse {
                status_code: item.http_code,
                code: serde_json::to_value(response.status)
                    .ok()
                    .and_then(|value| value.as_str().map(str::to_owned))
                    .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
                message: message.to_string(),
                reason: Some(message.to_string()),
                attempt_status: Some(status),
                connector_transaction_id: Some(response.transaction_uuid),
                connector_response_reference_id: None,
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(response.transaction_uuid),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                network_txn_link_id: None,
                connector_response_reference_id: response.ref_id,
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
                payment_account_reference: None,
            })
        };

        Ok(Self {
            status,
            response: response_data,
            ..item.data
        })
    }
}

/// eSewa has two error shapes: `{"code":"ES104","message":"..."}` from the form endpoint and
/// `{"code":0,"error_message":"..."}` (duplicate uuid, service unavailable).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EsewaErrorResponse {
    pub code: Option<serde_json::Value>,
    pub message: Option<String>,
    pub error_message: Option<String>,
}

impl EsewaErrorResponse {
    pub fn into_error_response(
        self,
        status_code: u16,
        attempt_status: Option<AttemptStatus>,
    ) -> ErrorResponse {
        let code = match self.code {
            Some(serde_json::Value::String(code)) => code,
            Some(serde_json::Value::Number(code)) => code.to_string(),
            _ => NO_ERROR_CODE.to_string(),
        };
        let message = self
            .message
            .or(self.error_message)
            .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string());
        ErrorResponse {
            status_code,
            code,
            message: message.clone(),
            reason: Some(message),
            attempt_status,
            connector_transaction_id: None,
            connector_response_reference_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Example message from eSewa's ePay docs, signed with the public UAT secret key; eSewa
    // UAT accepts signatures computed this way and rejects others with ES104.
    #[test]
    fn signs_message_with_hmac_sha256_base64() {
        let signature = sign(
            &Secret::new("8gBm/:&EnhH.1/q".to_string()),
            "total_amount=100,transaction_uuid=11-201-13,product_code=EPAYTEST",
        )
        .expect("signature");
        assert_eq!(
            signature.expose(),
            "5DZywcrTKD0gia/rsSMcrRHmJl+4Tbol6S+lWgdJ94E="
        );
    }

    // Status API responses recorded from eSewa UAT.
    #[test]
    fn maps_status_responses() {
        let cases = [
            (
                r#"{"product_code":"EPAYTEST","transaction_uuid":"123","total_amount":100.0,"status":"COMPLETE","ref_id":"00066XV"}"#,
                AttemptStatus::Charged,
            ),
            (
                r#"{"product_code":"EPAYTEST","transaction_uuid":"pay_Xy12_1","total_amount":100.0,"status":"PENDING","ref_id":null}"#,
                AttemptStatus::Pending,
            ),
            (
                r#"{"product_code":"EPAYTEST","transaction_uuid":"pay_Xy12_1","total_amount":99.0,"status":"NOT_FOUND","ref_id":null}"#,
                AttemptStatus::Failure,
            ),
            (
                r#"{"product_code":"EPAYTEST","transaction_uuid":"a","total_amount":1.0,"status":"AMBIGUOUS","ref_id":null}"#,
                AttemptStatus::Pending,
            ),
            (
                r#"{"product_code":"EPAYTEST","transaction_uuid":"a","total_amount":1.0,"status":"CANCELED","ref_id":null}"#,
                AttemptStatus::Failure,
            ),
            (
                r#"{"product_code":"EPAYTEST","transaction_uuid":"a","total_amount":1.0,"status":"FULL_REFUND","ref_id":"X"}"#,
                AttemptStatus::Charged,
            ),
        ];
        for (body, expected) in cases {
            match serde_json::from_str::<EsewaSyncResponse>(body).expect("status response") {
                EsewaSyncResponse::Status(response) => {
                    assert_eq!(AttemptStatus::from(response.status), expected, "{body}")
                }
                EsewaSyncResponse::Error(_) => panic!("parsed as error: {body}"),
            }
        }
    }

    // The form endpoint's Location header; the booking id must survive as a GET form field.
    #[test]
    fn moves_booking_id_into_redirect_form_fields() {
        let url = url::Url::parse(
            "https://rc-epay.esewa.com.np/epay?bookingId=D8DTCY05T4xGe0kr9%2FSigQ%3D%3D",
        )
        .expect("url");
        match RedirectForm::from((url, Method::Get)) {
            RedirectForm::Form {
                endpoint,
                method,
                form_fields,
            } => {
                assert_eq!(endpoint, "https://rc-epay.esewa.com.np/epay");
                assert_eq!(method, Method::Get);
                assert_eq!(
                    form_fields.get("bookingId").map(String::as_str),
                    Some("D8DTCY05T4xGe0kr9/SigQ==")
                );
            }
            other => panic!("unexpected redirect form: {other:?}"),
        }
    }

    #[test]
    fn parses_both_error_shapes() {
        let form_error: EsewaErrorResponse =
            serde_json::from_str(r#"{"code":"ES104","message":"Invalid payload signature."}"#)
                .expect("form error");
        let error = form_error.into_error_response(400, None);
        assert_eq!(error.code, "ES104");
        assert_eq!(error.message, "Invalid payload signature.");

        let status_error: EsewaSyncResponse = serde_json::from_str(
            r#"{"code":0,"error_message":"Service is currently unavailable"}"#,
        )
        .expect("status error");
        assert!(matches!(status_error, EsewaSyncResponse::Error(_)));
    }

    // An outage during PSync must not fail the attempt: core treats a 2xx error without an
    // attempt status as a payment failure.
    #[test]
    fn outage_error_keeps_attempt_status() {
        let outage: EsewaErrorResponse = serde_json::from_str(
            r#"{"code":0,"error_message":"Service is currently unavailable"}"#,
        )
        .expect("status error");
        let error = outage.into_error_response(200, Some(AttemptStatus::AuthenticationPending));
        assert_eq!(error.code, "0");
        assert_eq!(error.reason.as_deref(), Some("Service is currently unavailable"));
        assert_eq!(error.attempt_status, Some(AttemptStatus::AuthenticationPending));
    }
}
