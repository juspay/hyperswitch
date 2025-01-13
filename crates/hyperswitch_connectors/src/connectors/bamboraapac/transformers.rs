use common_enums::enums;
use common_utils::types::MinorUnit;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_request_types::{
        PaymentsAuthorizeData, PaymentsCaptureData, PaymentsSyncData, RefundsData, ResponseId,
        SetupMandateRequestData,
    },
    router_response_types::{MandateReference, PaymentsResponseData, RefundsResponseData},
    types,
};
use hyperswitch_interfaces::{
    consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE},
    errors,
};
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::ResponseRouterData,
    utils::{self, CardData as _, PaymentsAuthorizeRequestData, RouterData as _},
};

type Error = error_stack::Report<errors::ConnectorError>;

pub struct BamboraapacRouterData<T> {
    pub amount: MinorUnit,
    pub router_data: T,
}

impl<T> TryFrom<(MinorUnit, T)> for BamboraapacRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from((amount, item): (MinorUnit, T)) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BamboraapacMeta {
    pub authorize_id: String,
}

// request body in soap format
pub fn get_payment_body(
    req: &BamboraapacRouterData<&types::PaymentsAuthorizeRouterData>,
) -> Result<Vec<u8>, Error> {
    let transaction_data = get_transaction_body(req)?;
    let body = format!(
        r#"
            <soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/"
            xmlns:dts="http://www.ippayments.com.au/interface/api/dts">
                <soapenv:Body>
                    <dts:SubmitSinglePayment>
                        <dts:trnXML>
                            <![CDATA[
                                {}
                            ]]>
                        </dts:trnXML>
                    </dts:SubmitSinglePayment>
                </soapenv:Body>
            </soapenv:Envelope>
        "#,
        transaction_data
    );

    Ok(body.as_bytes().to_vec())
}

fn get_transaction_body(
    req: &BamboraapacRouterData<&types::PaymentsAuthorizeRouterData>,
) -> Result<String, Error> {
    let auth_details = BamboraapacAuthType::try_from(&req.router_data.connector_auth_type)?;
    let transaction_type = get_transaction_type(req.router_data.request.capture_method)?;
    let card_info = get_card_data(req.router_data)?;
    let transaction_data = format!(
        r#"
        <Transaction>
            <CustRef>{}</CustRef>
            <Amount>{}</Amount>
            <TrnType>{}</TrnType>
            <AccountNumber>{}</AccountNumber>
            {}
            <Security>
                    <UserName>{}</UserName>
                    <Password>{}</Password>
            </Security>
        </Transaction>
    "#,
        req.router_data.connector_request_reference_id.to_owned(),
        req.amount,
        transaction_type,
        auth_details.account_number.peek(),
        card_info,
        auth_details.username.peek(),
        auth_details.password.peek(),
    );

    Ok(transaction_data)
}

fn get_card_data(req: &types::PaymentsAuthorizeRouterData) -> Result<String, Error> {
    let card_data = match &req.request.payment_method_data {
        PaymentMethodData::Card(card) => {
            let card_holder_name = req.get_billing_full_name()?;

            if req.request.setup_future_usage == Some(enums::FutureUsage::OffSession) {
                format!(
                    r#"
                    <CreditCard Registered="False">
                        <TokeniseAlgorithmID>2</TokeniseAlgorithmID>
                        <CardNumber>{}</CardNumber>
                        <ExpM>{}</ExpM>
                        <ExpY>{}</ExpY>
                        <CVN>{}</CVN>
                        <CardHolderName>{}</CardHolderName>
                    </CreditCard>
                "#,
                    card.card_number.get_card_no(),
                    card.card_exp_month.peek(),
                    card.get_expiry_year_4_digit().peek(),
                    card.card_cvc.peek(),
                    card_holder_name.peek(),
                )
            } else {
                format!(
                    r#"
                    <CreditCard Registered="False">
                        <CardNumber>{}</CardNumber>
                        <ExpM>{}</ExpM>
                        <ExpY>{}</ExpY>
                        <CVN>{}</CVN>
                        <CardHolderName>{}</CardHolderName>
                    </CreditCard>
                "#,
                    card.card_number.get_card_no(),
                    card.card_exp_month.peek(),
                    card.get_expiry_year_4_digit().peek(),
                    card.card_cvc.peek(),
                    card_holder_name.peek(),
                )
            }
        }
        PaymentMethodData::MandatePayment => {
            format!(
                r#"
                <CreditCard>
                <TokeniseAlgorithmID>2</TokeniseAlgorithmID>
                <CardNumber>{}</CardNumber>
                </CreditCard>
            "#,
                req.request.get_connector_mandate_id()?
            )
        }
        _ => {
            return Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Bambora APAC"),
            ))?
        }
    };
    Ok(card_data)
}

fn get_transaction_type(capture_method: Option<enums::CaptureMethod>) -> Result<u8, Error> {
    match capture_method {
        Some(enums::CaptureMethod::Automatic) | None => Ok(1),
        Some(enums::CaptureMethod::Manual) => Ok(2),
        _ => Err(errors::ConnectorError::CaptureMethodNotSupported)?,
    }
}

pub struct BamboraapacAuthType {
    username: Secret<String>,
    password: Secret<String>,
    account_number: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for BamboraapacAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                username: api_key.to_owned(),
                password: api_secret.to_owned(),
                account_number: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "Envelope")]
#[serde(rename_all = "PascalCase")]
pub struct BamboraapacPaymentsResponse {
    body: BodyResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BodyResponse {
    submit_single_payment_response: SubmitSinglePaymentResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SubmitSinglePaymentResponse {
    submit_single_payment_result: SubmitSinglePaymentResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SubmitSinglePaymentResult {
    response: PaymentResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PaymentResponse {
    response_code: u8,
    receipt: String,
    credit_card_token: Option<String>,
    declined_code: Option<String>,
    declined_message: Option<String>,
}

fn get_attempt_status(
    response_code: u8,
    capture_method: Option<enums::CaptureMethod>,
) -> enums::AttemptStatus {
    match response_code {
        0 => match capture_method {
            Some(enums::CaptureMethod::Automatic) | None => enums::AttemptStatus::Charged,
            Some(enums::CaptureMethod::Manual) => enums::AttemptStatus::Authorized,
            _ => enums::AttemptStatus::Pending,
        },
        1 => enums::AttemptStatus::Failure,
        _ => enums::AttemptStatus::Pending,
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            BamboraapacPaymentsResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    > for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            BamboraapacPaymentsResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let response_code = item
            .response
            .body
            .submit_single_payment_response
            .submit_single_payment_result
            .response
            .response_code;
        let connector_transaction_id = item
            .response
            .body
            .submit_single_payment_response
            .submit_single_payment_result
            .response
            .receipt;

        let mandate_reference =
            if item.data.request.setup_future_usage == Some(enums::FutureUsage::OffSession) {
                let connector_mandate_id = item
                    .response
                    .body
                    .submit_single_payment_response
                    .submit_single_payment_result
                    .response
                    .credit_card_token;
                Some(MandateReference {
                    connector_mandate_id,
                    payment_method_id: None,
                    mandate_metadata: None,
                    connector_mandate_request_reference_id: None,
                })
            } else {
                None
            };
        // transaction approved
        if response_code == 0 {
            Ok(Self {
                status: get_attempt_status(response_code, item.data.request.capture_method),
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(
                        connector_transaction_id.to_owned(),
                    ),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(mandate_reference),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(connector_transaction_id),
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            })
        }
        // transaction failed
        else {
            let code = item
                .response
                .body
                .submit_single_payment_response
                .submit_single_payment_result
                .response
                .declined_code
                .unwrap_or(NO_ERROR_CODE.to_string());

            let declined_message = item
                .response
                .body
                .submit_single_payment_response
                .submit_single_payment_result
                .response
                .declined_message
                .unwrap_or(NO_ERROR_MESSAGE.to_string());
            Ok(Self {
                status: get_attempt_status(response_code, item.data.request.capture_method),
                response: Err(ErrorResponse {
                    status_code: item.http_code,
                    code,
                    message: declined_message.to_owned(),
                    reason: Some(declined_message),
                    attempt_status: None,
                    connector_transaction_id: None,
                }),
                ..item.data
            })
        }
    }
}

pub fn get_setup_mandate_body(req: &types::SetupMandateRouterData) -> Result<Vec<u8>, Error> {
    let card_holder_name = req.get_billing_full_name()?;
    let auth_details = BamboraapacAuthType::try_from(&req.connector_auth_type)?;
    let body = match &req.request.payment_method_data {
        PaymentMethodData::Card(card) => {
            format!(
                r#"
                <soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/"
                xmlns:sipp="http://www.ippayments.com.au/interface/api/sipp">
                <soapenv:Header/>
                <soapenv:Body>
                    <sipp:TokeniseCreditCard>
                        <sipp:tokeniseCreditCardXML>
                            <![CDATA[
                                <TokeniseCreditCard>
                                    <CardNumber>{}</CardNumber>
                                    <ExpM>{}</ExpM>
                                    <ExpY>{}</ExpY>
                                    <CardHolderName>{}</CardHolderName>
                                    <TokeniseAlgorithmID>2</TokeniseAlgorithmID>
                                    <UserName>{}</UserName>
                                    <Password>{}</Password>
                                </TokeniseCreditCard>
                            ]]>
                        </sipp:tokeniseCreditCardXML>
                    </sipp:TokeniseCreditCard>
                </soapenv:Body>
                </soapenv:Envelope>
                "#,
                card.card_number.get_card_no(),
                card.card_exp_month.peek(),
                card.get_expiry_year_4_digit().peek(),
                card_holder_name.peek(),
                auth_details.username.peek(),
                auth_details.password.peek(),
            )
        }
        _ => {
            return Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Bambora APAC"),
            ))?;
        }
    };

    Ok(body.as_bytes().to_vec())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "Envelope")]
#[serde(rename_all = "PascalCase")]
pub struct BamboraapacMandateResponse {
    body: MandateBodyResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MandateBodyResponse {
    tokenise_credit_card_response: TokeniseCreditCardResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TokeniseCreditCardResponse {
    tokenise_credit_card_result: TokeniseCreditCardResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TokeniseCreditCardResult {
    tokenise_credit_card_response: MandateResponseBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MandateResponseBody {
    return_value: u8,
    token: Option<String>,
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            BamboraapacMandateResponse,
            SetupMandateRequestData,
            PaymentsResponseData,
        >,
    > for RouterData<F, SetupMandateRequestData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            BamboraapacMandateResponse,
            SetupMandateRequestData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let response_code = item
            .response
            .body
            .tokenise_credit_card_response
            .tokenise_credit_card_result
            .tokenise_credit_card_response
            .return_value;

        let connector_mandate_id = item
            .response
            .body
            .tokenise_credit_card_response
            .tokenise_credit_card_result
            .tokenise_credit_card_response
            .token
            .ok_or(errors::ConnectorError::MissingConnectorMandateID)?;

        // transaction approved
        if response_code == 0 {
            Ok(Self {
                status: enums::AttemptStatus::Charged,
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::NoResponseId,
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(Some(MandateReference {
                        connector_mandate_id: Some(connector_mandate_id),
                        payment_method_id: None,
                        mandate_metadata: None,
                        connector_mandate_request_reference_id: None,
                    })),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            })
        }
        // transaction failed
        else {
            Ok(Self {
                status: enums::AttemptStatus::Failure,
                response: Err(ErrorResponse {
                    status_code: item.http_code,
                    code: NO_ERROR_CODE.to_string(),
                    message: NO_ERROR_MESSAGE.to_string(),
                    reason: None,
                    attempt_status: None,
                    connector_transaction_id: None,
                }),
                ..item.data
            })
        }
    }
}

// capture body in soap format
pub fn get_capture_body(
    req: &BamboraapacRouterData<&types::PaymentsCaptureRouterData>,
) -> Result<Vec<u8>, Error> {
    let receipt = req.router_data.request.connector_transaction_id.to_owned();
    let auth_details = BamboraapacAuthType::try_from(&req.router_data.connector_auth_type)?;
    let body = format!(
        r#"
            <soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/"
            xmlns:dts="http://www.ippayments.com.au/interface/api/dts">
                <soapenv:Body>
                    <dts:SubmitSingleCapture>
                        <dts:trnXML>
                            <![CDATA[
                                <Capture>
                                        <Receipt>{}</Receipt>
                                        <Amount>{}</Amount>
                                        <Security>
                                                <UserName>{}</UserName>
                                                <Password>{}</Password>
                                        </Security>
                                </Capture>
                            ]]>
                        </dts:trnXML>
                    </dts:SubmitSingleCapture>
                </soapenv:Body>
            </soapenv:Envelope>
        "#,
        receipt,
        req.amount,
        auth_details.username.peek(),
        auth_details.password.peek(),
    );

    Ok(body.as_bytes().to_vec())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "Envelope")]
#[serde(rename_all = "PascalCase")]
pub struct BamboraapacCaptureResponse {
    body: CaptureBodyResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CaptureBodyResponse {
    submit_single_capture_response: SubmitSingleCaptureResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SubmitSingleCaptureResponse {
    submit_single_capture_result: SubmitSingleCaptureResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SubmitSingleCaptureResult {
    response: CaptureResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CaptureResponse {
    response_code: u8,
    receipt: String,
    declined_code: Option<String>,
    declined_message: Option<String>,
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            BamboraapacCaptureResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    > for RouterData<F, PaymentsCaptureData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            BamboraapacCaptureResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let response_code = item
            .response
            .body
            .submit_single_capture_response
            .submit_single_capture_result
            .response
            .response_code;
        let connector_transaction_id = item
            .response
            .body
            .submit_single_capture_response
            .submit_single_capture_result
            .response
            .receipt;

        // storing receipt_id of authorize to metadata for future usage
        let connector_metadata = Some(serde_json::json!(BamboraapacMeta {
            authorize_id: item.data.request.connector_transaction_id.to_owned()
        }));
        // transaction approved
        if response_code == 0 {
            Ok(Self {
                status: enums::AttemptStatus::Charged,
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(
                        connector_transaction_id.to_owned(),
                    ),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata,
                    network_txn_id: None,
                    connector_response_reference_id: Some(connector_transaction_id),
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            })
        }
        // transaction failed
        else {
            let code = item
                .response
                .body
                .submit_single_capture_response
                .submit_single_capture_result
                .response
                .declined_code
                .unwrap_or(NO_ERROR_CODE.to_string());
            let declined_message = item
                .response
                .body
                .submit_single_capture_response
                .submit_single_capture_result
                .response
                .declined_message
                .unwrap_or(NO_ERROR_MESSAGE.to_string());
            Ok(Self {
                status: enums::AttemptStatus::Failure,
                response: Err(ErrorResponse {
                    status_code: item.http_code,
                    code,
                    message: declined_message.to_owned(),
                    reason: Some(declined_message),
                    attempt_status: None,
                    connector_transaction_id: None,
                }),
                ..item.data
            })
        }
    }
}

// refund body in soap format
pub fn get_refund_body(
    req: &BamboraapacRouterData<&types::RefundExecuteRouterData>,
) -> Result<Vec<u8>, Error> {
    let receipt = req.router_data.request.connector_transaction_id.to_owned();
    let auth_details = BamboraapacAuthType::try_from(&req.router_data.connector_auth_type)?;
    let body = format!(
        r#"
            <soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/"
            xmlns:dts="http://www.ippayments.com.au/interface/api/dts">
                <soapenv:Header/>
                <soapenv:Body>
                    <dts:SubmitSingleRefund>
                        <dts:trnXML>
                            <![CDATA[
                            <Refund>
                                <CustRef>{}</CustRef>
                                <Receipt>{}</Receipt>
                                <Amount>{}</Amount>
                                <Security>
                                    <UserName>{}</UserName>
                                    <Password>{}</Password>
                                </Security>
                            </Refund>
                            ]]>
                        </dts:trnXML>
                    </dts:SubmitSingleRefund>
                </soapenv:Body>
            </soapenv:Envelope>
        "#,
        req.router_data.request.refund_id.to_owned(),
        receipt,
        req.amount,
        auth_details.username.peek(),
        auth_details.password.peek(),
    );

    Ok(body.as_bytes().to_vec())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "Envelope")]
#[serde(rename_all = "PascalCase")]
pub struct BamboraapacRefundsResponse {
    body: RefundBodyResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RefundBodyResponse {
    submit_single_refund_response: SubmitSingleRefundResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SubmitSingleRefundResponse {
    submit_single_refund_result: SubmitSingleRefundResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SubmitSingleRefundResult {
    response: RefundResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RefundResponse {
    response_code: u8,
    receipt: String,
    declined_code: Option<String>,
    declined_message: Option<String>,
}

fn get_status(item: u8) -> enums::RefundStatus {
    match item {
        0 => enums::RefundStatus::Success,
        1 => enums::RefundStatus::Failure,
        _ => enums::RefundStatus::Pending,
    }
}

impl<F> TryFrom<ResponseRouterData<F, BamboraapacRefundsResponse, RefundsData, RefundsResponseData>>
    for RouterData<F, RefundsData, RefundsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, BamboraapacRefundsResponse, RefundsData, RefundsResponseData>,
    ) -> Result<Self, Self::Error> {
        let response_code = item
            .response
            .body
            .submit_single_refund_response
            .submit_single_refund_result
            .response
            .response_code;
        let connector_refund_id = item
            .response
            .body
            .submit_single_refund_response
            .submit_single_refund_result
            .response
            .receipt;

        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: connector_refund_id.to_owned(),
                refund_status: get_status(response_code),
            }),
            ..item.data
        })
    }
}

pub fn get_payment_sync_body(req: &types::PaymentsSyncRouterData) -> Result<Vec<u8>, Error> {
    let auth_details = BamboraapacAuthType::try_from(&req.connector_auth_type)?;
    let connector_transaction_id = req
        .request
        .connector_transaction_id
        .get_connector_transaction_id()
        .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
    let body = format!(
        r#"
            <soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/"
            xmlns:dts="http://www.ippayments.com.au/interface/api/dts">
                <soapenv:Header/>
                <soapenv:Body>
                    <dts:QueryTransaction>
                        <dts:queryXML>
                            <![CDATA[
                                <QueryTransaction>
                                    <Criteria>
                                        <AccountNumber>{}</AccountNumber>
                                        <TrnStartTimestamp>2024-06-23 00:00:00</TrnStartTimestamp>
                                        <TrnEndTimestamp>2099-12-31 23:59:59</TrnEndTimestamp>
                                        <Receipt>{}</Receipt>
                                    </Criteria>
                                    <Security>
                                        <UserName>{}</UserName>
                                        <Password>{}</Password>
                                    </Security>
                            </QueryTransaction>
                            ]]>
                        </dts:queryXML>
                    </dts:QueryTransaction>
                </soapenv:Body>
            </soapenv:Envelope>
        "#,
        auth_details.account_number.peek(),
        connector_transaction_id,
        auth_details.username.peek(),
        auth_details.password.peek(),
    );

    Ok(body.as_bytes().to_vec())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "Envelope")]
#[serde(rename_all = "PascalCase")]
pub struct BamboraapacSyncResponse {
    body: SyncBodyResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SyncBodyResponse {
    query_transaction_response: QueryTransactionResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct QueryTransactionResponse {
    query_transaction_result: QueryTransactionResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct QueryTransactionResult {
    query_response: QueryResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct QueryResponse {
    response: SyncResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SyncResponse {
    response_code: u8,
    receipt: String,
    declined_code: Option<String>,
    declined_message: Option<String>,
}

impl<F>
    TryFrom<ResponseRouterData<F, BamboraapacSyncResponse, PaymentsSyncData, PaymentsResponseData>>
    for RouterData<F, PaymentsSyncData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            BamboraapacSyncResponse,
            PaymentsSyncData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let response_code = item
            .response
            .body
            .query_transaction_response
            .query_transaction_result
            .query_response
            .response
            .response_code;
        let connector_transaction_id = item
            .response
            .body
            .query_transaction_response
            .query_transaction_result
            .query_response
            .response
            .receipt;
        // transaction approved
        if response_code == 0 {
            Ok(Self {
                status: get_attempt_status(response_code, item.data.request.capture_method),
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(
                        connector_transaction_id.to_owned(),
                    ),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(connector_transaction_id),
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            })
        }
        // transaction failed
        else {
            let code = item
                .response
                .body
                .query_transaction_response
                .query_transaction_result
                .query_response
                .response
                .declined_code
                .unwrap_or(NO_ERROR_CODE.to_string());
            let declined_message = item
                .response
                .body
                .query_transaction_response
                .query_transaction_result
                .query_response
                .response
                .declined_message
                .unwrap_or(NO_ERROR_MESSAGE.to_string());
            Ok(Self {
                status: get_attempt_status(response_code, item.data.request.capture_method),
                response: Err(ErrorResponse {
                    status_code: item.http_code,
                    code,
                    message: declined_message.to_owned(),
                    reason: Some(declined_message),
                    attempt_status: None,
                    connector_transaction_id: None,
                }),
                ..item.data
            })
        }
    }
}

pub fn get_refund_sync_body(req: &types::RefundSyncRouterData) -> Result<Vec<u8>, Error> {
    let auth_details = BamboraapacAuthType::try_from(&req.connector_auth_type)?;

    let body = format!(
        r#"
            <soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/"
            xmlns:dts="http://www.ippayments.com.au/interface/api/dts">
                <soapenv:Header/>
                <soapenv:Body>
                    <dts:QueryTransaction>
                        <dts:queryXML>
                            <![CDATA[
                                <QueryTransaction>
                                    <Criteria>
                                        <AccountNumber>{}</AccountNumber>
                                        <TrnStartTimestamp>2024-06-23 00:00:00</TrnStartTimestamp>
                                        <TrnEndTimestamp>2099-12-31 23:59:59</TrnEndTimestamp>
                                        <CustRef>{}</CustRef>
                                    </Criteria>
                                    <Security>
                                        <UserName>{}</UserName>
                                        <Password>{}</Password>
                                    </Security>
                            </QueryTransaction>
                            ]]>
                        </dts:queryXML>
                    </dts:QueryTransaction>
                </soapenv:Body>
            </soapenv:Envelope>
        "#,
        auth_details.account_number.peek(),
        req.request.refund_id,
        auth_details.username.peek(),
        auth_details.password.peek(),
    );

    Ok(body.as_bytes().to_vec())
}

impl<F> TryFrom<ResponseRouterData<F, BamboraapacSyncResponse, RefundsData, RefundsResponseData>>
    for RouterData<F, RefundsData, RefundsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, BamboraapacSyncResponse, RefundsData, RefundsResponseData>,
    ) -> Result<Self, Self::Error> {
        let response_code = item
            .response
            .body
            .query_transaction_response
            .query_transaction_result
            .query_response
            .response
            .response_code;
        let connector_refund_id = item
            .response
            .body
            .query_transaction_response
            .query_transaction_result
            .query_response
            .response
            .receipt;
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: connector_refund_id.to_owned(),
                refund_status: get_status(response_code),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct BamboraapacErrorResponse {
    pub declined_code: Option<String>,
    pub declined_message: Option<String>,
}
