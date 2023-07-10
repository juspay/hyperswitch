use std::collections::HashMap;

pub use api_models::payment_methods::{
    CardDetail, CardDetailFromLocker, CustomerPaymentMethod, CustomerPaymentMethodsListResponse,
    DeleteTokenizeByDateRequest, DeleteTokenizeByTokenRequest, GetTokenizePayloadRequest,
    GetTokenizePayloadResponse, PaymentMethodCreate, PaymentMethodDeleteResponse, PaymentMethodId,
    PaymentMethodList, PaymentMethodListRequest, PaymentMethodListResponse, PaymentMethodResponse,
    PaymentMethodUpdate, TokenizePayloadEncrypted, TokenizePayloadRequest,
    TokenizedBankInsensitiveValues, TokenizedBankSensitiveValues, TokenizedCardValue1,
    TokenizedCardValue2, TokenizedWalletValue1, TokenizedWalletValue2,
};
use error_stack::report;
use literally::hmap;
use once_cell::sync::Lazy;

use crate::{
    core::errors::{self, RouterResult},
    types::api::enums as api_enums,
};

/// Static collection that contains valid Payment Method Type and Payment Method SubType
/// tuples. Used for validation.
static PAYMENT_METHOD_TYPE_SET: Lazy<
    HashMap<api_enums::PaymentMethod, Vec<api_enums::PaymentMethodType>>,
> = Lazy::new(|| {
    use api_enums::{PaymentMethod as T, PaymentMethodType as ST};

    hmap! {
        T::Card => vec![
            ST::Credit,
            ST::Debit
        ],
        T::Wallet => vec![]
    }
});

/// Static collection that contains valid Payment Method Issuer and Payment Method Issuer
/// Type tuples. Used for validation.
static PAYMENT_METHOD_ISSUER_SET: Lazy<
    HashMap<api_enums::PaymentMethod, Vec<api_enums::PaymentMethodIssuerCode>>,
> = Lazy::new(|| {
    use api_enums::{PaymentMethod as T, PaymentMethodIssuerCode as IC};

    hmap! {
        T::Card => vec![
            IC::JpHdfc,
            IC::JpIcici,
        ],
        T::Wallet => vec![
            IC::JpApplepay,
            IC::JpGooglepay,
            IC::JpWechat
        ],
    }
});

pub(crate) trait PaymentMethodCreateExt {
    fn validate(&self) -> RouterResult<()>;
    fn check_subtype_mapping<T, U>(
        dict: &HashMap<T, Vec<U>>,
        the_type: T,
        the_subtype: Option<U>,
    ) -> bool
    where
        T: Eq + std::hash::Hash,
        U: PartialEq;
}

impl PaymentMethodCreateExt for PaymentMethodCreate {
    fn validate(&self) -> RouterResult<()> {
        let pm_subtype_map = Lazy::get(&PAYMENT_METHOD_TYPE_SET)
            .unwrap_or_else(|| Lazy::force(&PAYMENT_METHOD_TYPE_SET));
        if !Self::check_subtype_mapping(
            pm_subtype_map,
            self.payment_method,
            self.payment_method_type,
        ) {
            return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "Invalid 'payment_method_type' provided.".to_string()
            })
            .attach_printable("Invalid payment method type"));
        }

        let issuer_map = Lazy::get(&PAYMENT_METHOD_ISSUER_SET)
            .unwrap_or_else(|| Lazy::force(&PAYMENT_METHOD_ISSUER_SET));
        if !Self::check_subtype_mapping(
            issuer_map,
            self.payment_method,
            self.payment_method_issuer_code,
        ) {
            return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "Invalid 'payment_method_issuer_code' provided.".to_string()
            })
            .attach_printable("Invalid payment method issuer code"));
        }

        Ok(())
    }

    fn check_subtype_mapping<T, U>(
        dict: &HashMap<T, Vec<U>>,
        the_type: T,
        the_subtype: Option<U>,
    ) -> bool
    where
        T: Eq + std::hash::Hash,
        U: PartialEq,
    {
        let the_subtype = match the_subtype {
            Some(st) => st,
            None => return true,
        };

        dict.get(&the_type)
            .map(|subtypes| subtypes.contains(&the_subtype))
            .unwrap_or(true)
    }
}
