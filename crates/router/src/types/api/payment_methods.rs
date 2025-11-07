#[cfg(feature = "v2")]
pub use api_models::payment_methods::{
    CardDetail, CardDetailFromLocker, CardDetailsPaymentMethod, CardNetworkTokenizeRequest,
    CardNetworkTokenizeResponse, CardType, CustomerPaymentMethodResponseItem,
    DeleteTokenizeByTokenRequest, GetTokenizePayloadRequest, GetTokenizePayloadResponse,
    ListCountriesCurrenciesRequest, MigrateCardDetail, NetworkTokenDetailsPaymentMethod,
    NetworkTokenDetailsResponse, NetworkTokenResponse, PaymentMethodCollectLinkRenderRequest,
    PaymentMethodCollectLinkRequest, PaymentMethodCreate, PaymentMethodCreateData,
    PaymentMethodDeleteResponse, PaymentMethodId, PaymentMethodIntentConfirm,
    PaymentMethodIntentCreate, PaymentMethodListData, PaymentMethodListResponseForSession,
    PaymentMethodMigrate, PaymentMethodMigrateResponse, PaymentMethodResponse,
    PaymentMethodResponseData, PaymentMethodUpdate, PaymentMethodUpdateData, PaymentMethodsData,
    ProxyCardDetails, RequestPaymentMethodTypes, TokenDataResponse, TokenDetailsResponse,
    TokenizePayloadEncrypted, TokenizePayloadRequest, TokenizedCardValue1, TokenizedCardValue2,
    TokenizedWalletValue1, TokenizedWalletValue2, TotalPaymentMethodCountResponse,
};
#[cfg(feature = "v1")]
pub use api_models::payment_methods::{
    CardDetail, CardDetailFromLocker, CardDetailsPaymentMethod, CardNetworkTokenizeRequest,
    CardNetworkTokenizeResponse, CustomerPaymentMethod, CustomerPaymentMethodUpdateResponse,
    CustomerPaymentMethodsListResponse, DefaultPaymentMethod, DeleteTokenizeByTokenRequest,
    GetTokenizePayloadRequest, GetTokenizePayloadResponse, ListCountriesCurrenciesRequest,
    MigrateCardDetail, PaymentMethodCollectLinkRenderRequest, PaymentMethodCollectLinkRequest,
    PaymentMethodCreate, PaymentMethodCreateData, PaymentMethodDeleteResponse, PaymentMethodId,
    PaymentMethodListRequest, PaymentMethodListResponse, PaymentMethodMigrate,
    PaymentMethodMigrateResponse, PaymentMethodResponse, PaymentMethodUpdate, PaymentMethodsData,
    TokenizeCardRequest, TokenizeDataRequest, TokenizePayloadEncrypted, TokenizePayloadRequest,
    TokenizePaymentMethodRequest, TokenizedCardValue1, TokenizedCardValue2, TokenizedWalletValue1,
    TokenizedWalletValue2,
};
use error_stack::report;

use crate::core::{
    errors::{self, RouterResult},
    payments::helpers::validate_payment_method_type_against_payment_method,
};
#[cfg(feature = "v2")]
use crate::utils;

pub(crate) trait PaymentMethodCreateExt {
    fn validate(&self) -> RouterResult<()>;
}

// convert self.payment_method_type to payment_method and compare it against self.payment_method
#[cfg(feature = "v1")]
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

#[cfg(feature = "v2")]
impl PaymentMethodCreateExt for PaymentMethodIntentConfirm {
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
