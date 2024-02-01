pub use api_models::payment_methods::{
    CardDetail, CardDetailFromLocker, CardDetailsPaymentMethod, CustomerPaymentMethod,
    CustomerPaymentMethodsListResponse, DeleteTokenizeByTokenRequest, GetTokenizePayloadRequest,
    GetTokenizePayloadResponse, PaymentMethodCreate, PaymentMethodDeleteResponse, PaymentMethodId,
    PaymentMethodList, PaymentMethodListRequest, PaymentMethodListResponse, PaymentMethodResponse,
    PaymentMethodUpdate, PaymentMethodsData, TokenizePayloadEncrypted, TokenizePayloadRequest,
    TokenizedCardValue1, TokenizedCardValue2, TokenizedWalletValue1, TokenizedWalletValue2,
};
use error_stack::report;

use crate::core::{
    errors::{self, RouterResult},
    payments::helpers::validate_payment_method_type_against_payment_method,
};

pub(crate) trait PaymentMethodCreateExt {
    fn validate(&self) -> RouterResult<()>;
}

// convert self.payment_method_type to payment_method and compare it against self.payment_method
impl PaymentMethodCreateExt for PaymentMethodCreate {
        /// Validates the payment method type against the provided payment method, and returns Ok(()) if the validation passes.
    fn validate(&self) -> RouterResult<()> {
        if let Some(payment_method_type) = self.payment_method_type {
            if !validate_payment_method_type_against_payment_method(
                self.payment_method,
                payment_method_type,
            ) {
                return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                    message: "Invalid 'payment_method_type' provided".to_string()
                })
                .attach_printable("Invalid payment method type"));
            }
        }
        Ok(())
    }
}
