use api_models::enums as api_enums;
pub use api_models::payment_methods::{
    CardDetail, CardDetailFromLocker, CardDetailsPaymentMethod, CustomerPaymentMethod,
    CustomerPaymentMethodsListResponse, DeleteTokenizeByDateRequest, DeleteTokenizeByTokenRequest,
    GetTokenizePayloadRequest, GetTokenizePayloadResponse, PaymentMethodCreate,
    PaymentMethodDeleteResponse, PaymentMethodId, PaymentMethodList, PaymentMethodListRequest,
    PaymentMethodListResponse, PaymentMethodResponse, PaymentMethodUpdate,
    TokenizePayloadEncrypted, TokenizePayloadRequest, TokenizedCardValue1, TokenizedCardValue2,
    TokenizedWalletValue1, TokenizedWalletValue2,
};
use error_stack::report;

use crate::{
    core::errors::{self, RouterResult},
    types::transformers::ForeignFrom,
};

pub(crate) trait PaymentMethodCreateExt {
    fn validate(&self) -> RouterResult<()>;
}

// convert self.payment_method_type to payment_method and compare it against self.payment_method
impl PaymentMethodCreateExt for PaymentMethodCreate {
    fn validate(&self) -> RouterResult<()> {
        let payment_method: Option<api_enums::PaymentMethod> =
            self.payment_method_type.map(ForeignFrom::foreign_from);
        if payment_method
            .map(|payment_method| payment_method != self.payment_method)
            .unwrap_or(false)
        {
            return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "Invalid 'payment_method_type' provided".to_string()
            })
            .attach_printable("Invalid payment method type"));
        }
        Ok(())
    }
}
