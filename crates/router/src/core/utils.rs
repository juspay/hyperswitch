use std::marker::PhantomData;

use error_stack::ResultExt;
use router_env::{instrument, tracing};

use super::payments::PaymentAddress;
use crate::{
    consts,
    core::errors::{self, RouterResult},
    routes::AppState,
    types::{
        self,
        storage::{self, enums},
    },
    utils::{generate_id, OptionExt, ValueExt},
};

#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn construct_refund_router_data<'a, F>(
    state: &'a AppState,
    connector_id: &str,
    merchant_account: &storage::MerchantAccount,
    money: (i64, enums::Currency),
    payment_intent: &'a storage::PaymentIntent,
    payment_attempt: &storage::PaymentAttempt,
    refund: &'a storage::Refund,
) -> RouterResult<types::RefundsRouterData<F>> {
    let db = &*state.store;
    let merchant_connector_account = db
        .find_merchant_connector_account_by_merchant_id_connector(
            &merchant_account.merchant_id,
            connector_id,
        )
        .await
        .change_context(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let auth_type: types::ConnectorAuthType = merchant_connector_account
        .connector_account_details
        .parse_value("ConnectorAuthType")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let status = payment_attempt.status;

    let (amount, currency) = money;

    let payment_method_type = payment_attempt
        .payment_method
        .get_required_value("payment_method_type")?;

    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: merchant_account.merchant_id.clone(),
        connector: merchant_connector_account.connector_name,
        payment_id: payment_attempt.payment_id.clone(),
        attempt_id: payment_attempt.attempt_id.clone(),
        status,
        payment_method: payment_method_type,
        connector_auth_type: auth_type,
        description: None,
        return_url: payment_intent.return_url.clone(),
        router_return_url: None,
        payment_method_id: payment_attempt.payment_method_id.clone(),
        // Does refund need shipping/billing address ?
        address: PaymentAddress::default(),
        auth_type: payment_attempt.authentication_type.unwrap_or_default(),
        connector_meta_data: None,
        amount_captured: payment_intent.amount_captured,
        request: types::RefundsData {
            refund_id: refund.refund_id.clone(),
            connector_transaction_id: refund.connector_transaction_id.clone(),
            refund_amount: refund.refund_amount,
            currency,
            amount,
            connector_metadata: payment_attempt.connector_metadata.clone(),
            reason: refund.refund_reason.clone(),
            connector_refund_id: refund.connector_refund_id.clone(),
        },

        response: Ok(types::RefundsResponseData {
            connector_refund_id: refund.connector_refund_id.clone().unwrap_or_default(),
            refund_status: refund.refund_status,
        }),
        access_token: None,
    };

    Ok(router_data)
}

pub fn get_or_generate_id(
    key: &str,
    provided_id: &Option<String>,
    prefix: &str,
) -> Result<String, errors::ApiErrorResponse> {
    let validate_id = |id| validate_id(id, key);
    provided_id
        .clone()
        .map_or(Ok(generate_id(consts::ID_LENGTH, prefix)), validate_id)
}

fn invalid_id_format_error(key: &str) -> errors::ApiErrorResponse {
    errors::ApiErrorResponse::InvalidDataFormat {
        field_name: key.to_string(),
        expected_format: format!(
            "length should be less than {} characters",
            consts::MAX_ID_LENGTH
        ),
    }
}

pub fn validate_id(id: String, key: &str) -> Result<String, errors::ApiErrorResponse> {
    if id.len() > consts::MAX_ID_LENGTH {
        Err(invalid_id_format_error(key))
    } else {
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_id_length_constraint() {
        let payment_id = "abcdefghijlkmnopqrstuvwzyzabcdefghijknlmnop".to_string(); //length = 43

        let result = validate_id(payment_id, "payment_id");
        assert!(result.is_err());
    }

    #[test]
    fn validate_id_proper_response() {
        let payment_id = "abcdefghijlkmnopqrst".to_string();

        let result = validate_id(payment_id.clone(), "payment_id");
        assert!(result.is_ok());
        let result = result.unwrap_or_default();
        assert_eq!(result, payment_id);
    }

    #[test]
    fn test_generate_id() {
        let generated_id = generate_id(consts::ID_LENGTH, "ref");
        assert_eq!(generated_id.len(), consts::ID_LENGTH + 4)
    }
}
