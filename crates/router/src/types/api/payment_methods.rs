use std::collections::HashMap;

pub use api_models::payment_methods::{
    CardDetail, CardDetailFromLocker, CreatePaymentMethod, CustomerPaymentMethod,
    DeletePaymentMethodResponse, DeleteTokenizeByDateRequest, DeleteTokenizeByTokenRequest,
    GetTokenizePayloadRequest, GetTokenizePayloadResponse, ListCustomerPaymentMethodsResponse,
    ListPaymentMethodRequest, ListPaymentMethodResponse, PaymentMethodId, PaymentMethodResponse,
    TokenizePayloadEncrypted, TokenizePayloadRequest, TokenizedCardValue1, TokenizedCardValue2,
    UpdatePaymentMethod,
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
    HashMap<api_enums::PaymentMethodType, Vec<api_enums::PaymentMethodSubType>>,
> = Lazy::new(|| {
    use api_enums::{PaymentMethodSubType as ST, PaymentMethodType as T};

    hmap! {
        T::Card => vec![
            ST::Credit,
            ST::Debit
        ],
        T::BankTransfer => vec![],
        T::Netbanking => vec![],
        T::Upi => vec![
            ST::UpiIntent,
            ST::UpiCollect
        ],
        T::OpenBanking => vec![],
        T::ConsumerFinance => vec![],
        T::Wallet => vec![]
    }
});

/// Static collection that contains valid Payment Method Issuer and Payment Method Issuer
/// Type tuples. Used for validation.
static PAYMENT_METHOD_ISSUER_SET: Lazy<
    HashMap<api_enums::PaymentMethodType, Vec<api_enums::PaymentMethodIssuerCode>>,
> = Lazy::new(|| {
    use api_enums::{PaymentMethodIssuerCode as IC, PaymentMethodType as T};

    hmap! {
        T::Card => vec![
            IC::JpHdfc,
            IC::JpIcici,
        ],
        T::Upi => vec![
            IC::JpGooglepay,
            IC::JpPhonepay
        ],
        T::Netbanking => vec![
            IC::JpSofort,
            IC::JpGiropay
        ],
        T::Wallet => vec![
            IC::JpApplepay,
            IC::JpGooglepay,
            IC::JpWechat
        ],
        T::BankTransfer => vec![
            IC::JpSepa,
            IC::JpBacs
        ]
    }
});

pub(crate) trait CreatePaymentMethodExt {
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

impl CreatePaymentMethodExt for CreatePaymentMethod {
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
