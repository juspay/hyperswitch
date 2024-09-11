#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
pub use api_models::payment_methods::{
    CardDetail, CardDetailFromLocker, CardDetailsPaymentMethod, CardType, CustomerPaymentMethod,
    CustomerPaymentMethodsListResponse, DefaultPaymentMethod, DeleteTokenizeByTokenRequest,
    GetTokenizePayloadRequest, GetTokenizePayloadResponse, ListCountriesCurrenciesRequest,
    PaymentMethodCollectLinkRenderRequest, PaymentMethodCollectLinkRequest, PaymentMethodCreate,
    PaymentMethodCreateData, PaymentMethodDeleteResponse, PaymentMethodId,
    PaymentMethodIntentConfirm, PaymentMethodIntentCreate, PaymentMethodList,
    PaymentMethodListData, PaymentMethodListRequest, PaymentMethodListResponse,
    PaymentMethodMigrate, PaymentMethodResponse, PaymentMethodResponseData, PaymentMethodUpdate,
    PaymentMethodsData, TokenizePayloadEncrypted, TokenizePayloadRequest, TokenizedCardValue1,
    TokenizedCardValue2, TokenizedWalletValue1, TokenizedWalletValue2, PaymentMethodIntentConfirmInternal
};
#[cfg(all(
    any(feature = "v2", feature = "v1"),
    not(feature = "payment_methods_v2")
))]
pub use api_models::payment_methods::{
    CardDetail, CardDetailFromLocker, CardDetailsPaymentMethod, CustomerPaymentMethod,
    CustomerPaymentMethodsListResponse, DefaultPaymentMethod, DeleteTokenizeByTokenRequest,
    GetTokenizePayloadRequest, GetTokenizePayloadResponse, ListCountriesCurrenciesRequest,
    PaymentMethodCollectLinkRenderRequest, PaymentMethodCollectLinkRequest, PaymentMethodCreate,
    PaymentMethodCreateData, PaymentMethodDeleteResponse, PaymentMethodId, PaymentMethodList,
    PaymentMethodListRequest, PaymentMethodListResponse, PaymentMethodMigrate,
    PaymentMethodResponse, PaymentMethodUpdate, PaymentMethodsData, TokenizePayloadEncrypted,
    TokenizePayloadRequest, TokenizedCardValue1, TokenizedCardValue2, TokenizedWalletValue1,
    TokenizedWalletValue2,
};
use error_stack::report;

// #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
// use crate::core::payment_methods::helpers::validate_payment_method_data_against_payment_method;
use crate::core::{
    errors::{self, RouterResult},
    payments::helpers::validate_payment_method_type_against_payment_method,
};

pub(crate) trait PaymentMethodCreateExt {
    fn validate(&self) -> RouterResult<()>;
}

// convert self.payment_method_type to payment_method and compare it against self.payment_method
#[cfg(all(
    any(feature = "v2", feature = "v1"),
    not(feature = "payment_methods_v2")
))]
impl PaymentMethodCreateExt for PaymentMethodCreate {
    fn validate(&self) -> RouterResult<()> {
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

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
impl PaymentMethodCreateExt for PaymentMethodCreate {
    fn validate(&self) -> RouterResult<()> {
        if !validate_payment_method_type_against_payment_method(
            self.payment_method,
            self.payment_method_type,
        ) {
            return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "Invalid 'payment_method_type' provided".to_string()
            })
            .attach_printable("Invalid payment method type"));
        }

        if !PaymentMethodCreate::validate_payment_method_data_against_payment_method(
            self.payment_method,
            self.payment_method_data.clone(),
        ) {
            return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "Invalid 'payment_method_data' provided".to_string()
            })
            .attach_printable("Invalid payment method data"));
        }
        Ok(())
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
impl PaymentMethodCreateExt for PaymentMethodIntentConfirm {
    fn validate(&self) -> RouterResult<()> {
        if !validate_payment_method_type_against_payment_method(
            self.payment_method,
            self.payment_method_type,
        ) {
            return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "Invalid 'payment_method_type' provided".to_string()
            })
            .attach_printable("Invalid payment method type"));
        }

        if !PaymentMethodIntentConfirm::validate_payment_method_data_against_payment_method(
            self.payment_method,
            self.payment_method_data.clone(),
        ) {
            return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "Invalid 'payment_method_data' provided".to_string()
            })
            .attach_printable("Invalid payment method data"));
        }
        Ok(())
    }
}
