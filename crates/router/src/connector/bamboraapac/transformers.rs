use hyperswitch_interfaces::consts;
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, CardData, RouterData},
    core::{errors, mandate::MandateBehaviour},
    types::{self, domain, storage::enums},
};

type Error = error_stack::Report<errors::ConnectorError>;

// request body in soap format
pub fn get_payment_body(req: &types::PaymentsAuthorizeRouterData) -> Result<Vec<u8>, Error> {
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

fn get_transaction_body(req: &types::PaymentsAuthorizeRouterData) -> Result<String, Error> {
    let amount = req.request.get_amount();
    let auth_details = BamboraapacAuthType::try_from(&req.connector_auth_type)?;
    let card_info = get_card_data(req)?;
    let transaction_data = format!(
        r#"
        <Transaction>
            <CustRef>{}</CustRef>
            <Amount>{}</Amount>
            <TrnType>1</TrnType>
            <AccountNumber>{}</AccountNumber>
            {}
            <Security>
                    <UserName>{}</UserName>
                    <Password>{}</Password>
            </Security>
        </Transaction>
    "#,
        req.connector_request_reference_id.to_owned(),
        amount,
        auth_details.account_number.peek(),
        card_info,
        auth_details.username.peek(),
        auth_details.password.peek(),
    );

    Ok(transaction_data)
}

fn get_card_data(req: &types::PaymentsAuthorizeRouterData) -> Result<String, Error> {
    let card_holder_name = req.get_billing_full_name()?;
    let card_data = match &req.request.payment_method_data {
        domain::PaymentMethodData::Card(card) => {
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
        _ => {
            return Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Bambora APAC"),
            ))?
        }
    };
    Ok(card_data)
}

pub struct BamboraapacAuthType {
    username: Secret<String>,
    password: Secret<String>,
    account_number: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for BamboraapacAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::SignatureKey {
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
    declined_code: Option<String>,
    declined_message: Option<String>,
}

fn get_attempt_status(response_code: u8) -> enums::AttemptStatus {
    match response_code {
        0 => enums::AttemptStatus::Charged,
        _ => enums::AttemptStatus::Failure,
    }
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<F, BamboraapacPaymentsResponse, T, types::PaymentsResponseData>,
    > for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            BamboraapacPaymentsResponse,
            T,
            types::PaymentsResponseData,
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
        // transaction approved
        if response_code == 0 {
            Ok(Self {
                status: get_attempt_status(response_code),
                response: Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        connector_transaction_id.to_owned(),
                    ),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(connector_transaction_id),
                    incremental_authorization_allowed: None,
                    charge_id: None,
                }),
                ..item.data
            })
        }
        // transaction failed
        else {
            Ok(Self {
                status: get_attempt_status(response_code),
                response: Err(types::ErrorResponse {
                    status_code: item.http_code,
                    code: item
                        .response
                        .body
                        .submit_single_payment_response
                        .submit_single_payment_result
                        .response
                        .declined_code
                        .unwrap_or(consts::NO_ERROR_CODE.to_string()),
                    message: consts::NO_ERROR_MESSAGE.to_string(),
                    reason: item
                        .response
                        .body
                        .submit_single_payment_response
                        .submit_single_payment_result
                        .response
                        .declined_message,
                    attempt_status: None,
                    connector_transaction_id: None,
                }),
                ..item.data
            })
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct BamboraapacErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
