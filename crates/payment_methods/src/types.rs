use common_utils::id_type;
use error_stack::report;
use serde::{Deserialize, Serialize};
use crate::{core::errors,helpers::validate_payment_method_type_against_payment_method};
use api_models::payment_methods::{PaymentMethodCreate};

#[cfg(feature = "v1")]
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteCardToken {
    pub card_reference: String, //network token requestor ref id
    pub customer_id: id_type::CustomerId,
}

#[cfg(feature = "v2")]
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteCardToken {
    pub card_reference: String, //network token requestor ref id
    pub customer_id: id_type::GlobalCustomerId,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum DeleteNetworkTokenStatus {
    Success,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct NetworkTokenErrorInfo {
    pub code: String,
    pub developer_message: String,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct NetworkTokenErrorResponse {
    pub error_message: String,
    pub error_info: NetworkTokenErrorInfo,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct DeleteNetworkTokenResponse {
    pub status: DeleteNetworkTokenStatus,
}

pub(crate) trait PaymentMethodCreateExt {
    fn validate(&self) -> errors::PmResult<()>;
}

// convert self.payment_method_type to payment_method and compare it against self.payment_method
#[cfg(feature = "v1")]
impl PaymentMethodCreateExt for PaymentMethodCreate {
    fn validate(&self) -> errors::PmResult<()> {
        if let Some(pm) = self.payment_method {
            if let Some(payment_method_type) = self.payment_method_type {
                if !validate_payment_method_type_against_payment_method(pm, payment_method_type) {
                    return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                        message: "Invalid 'payment_method_type' provided".to_string()
                    })
                    .attach_printable("Invalid payment method type"));
                }
            }
        }
        Ok(())
    }
}

#[cfg(feature = "v2")]
impl PaymentMethodCreateExt for PaymentMethodCreate {
    fn validate(&self) -> RouterResult<()> {
        utils::when(
            !validate_payment_method_type_against_payment_method(
                self.payment_method_type,
                self.payment_method_subtype,
            ),
            || {
                Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                    message: "Invalid 'payment_method_type' provided".to_string()
                })
                .attach_printable("Invalid payment method type"))
            },
        )?;

        utils::when(
            !Self::validate_payment_method_data_against_payment_method(
                self.payment_method_type,
                self.payment_method_data.clone(),
            ),
            || {
                Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                    message: "Invalid 'payment_method_data' provided".to_string()
                })
                .attach_printable("Invalid payment method data"))
            },
        )?;
        Ok(())
    }
}
