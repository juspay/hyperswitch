use api_models::{enums as api_enums, payment_methods as api};
#[cfg(feature = "v1")]
use common_utils::{ext_traits::AsyncExt, id_type};
use error_stack::ResultExt;
pub use hyperswitch_domain_models::{
    errors::api_error_response,
    merchant_connector_account, merchant_key_store, payment_methods as domain,
    router_data::{ErrorResponse, RouterData},
};
#[cfg(feature = "v1")]
use router_env::logger;

use crate::{core::errors::CustomResult, state};
#[cfg(feature = "v1")]
pub async fn populate_bin_details_for_payment_method_create(
    card_details: api_models::payment_methods::CardDetail,
    db: &dyn state::PaymentMethodsStorageInterface,
) -> api_models::payment_methods::CardDetail {
    let card_isin: Option<_> = Some(card_details.card_number.get_card_isin());
    if card_details.card_issuer.is_some()
        && card_details.card_network.is_some()
        && card_details.card_type.is_some()
        && card_details.card_issuing_country.is_some()
    {
        api::CardDetail {
            card_issuer: card_details.card_issuer.to_owned(),
            card_network: card_details.card_network.clone(),
            card_type: card_details.card_type.to_owned(),
            card_issuing_country: card_details.card_issuing_country.to_owned(),
            card_exp_month: card_details.card_exp_month.clone(),
            card_exp_year: card_details.card_exp_year.clone(),
            card_cvc: card_details.card_cvc.clone(),
            card_holder_name: card_details.card_holder_name.clone(),
            card_number: card_details.card_number.clone(),
            nick_name: card_details.nick_name.clone(),
        }
    } else {
        let card_info = card_isin
            .clone()
            .async_and_then(|card_isin| async move {
                db.get_card_info(&card_isin)
                    .await
                    .map_err(|error| logger::error!(card_info_error=?error))
                    .ok()
            })
            .await
            .flatten()
            .map(|card_info| api::CardDetail {
                card_issuer: card_info.card_issuer,
                card_network: card_info.card_network.clone(),
                card_type: card_info.card_type,
                card_issuing_country: card_info.card_issuing_country,
                card_exp_month: card_details.card_exp_month.clone(),
                card_exp_year: card_details.card_exp_year.clone(),
                card_cvc: card_details.card_cvc.clone(),
                card_holder_name: card_details.card_holder_name.clone(),
                card_number: card_details.card_number.clone(),
                nick_name: card_details.nick_name.clone(),
            });
        card_info.unwrap_or_else(|| api::CardDetail {
            card_issuer: None,
            card_network: None,
            card_type: None,
            card_issuing_country: None,
            card_cvc: card_details.card_cvc.clone(),
            card_exp_month: card_details.card_exp_month.clone(),
            card_exp_year: card_details.card_exp_year.clone(),
            card_holder_name: card_details.card_holder_name.clone(),
            card_number: card_details.card_number.clone(),
            nick_name: card_details.nick_name.clone(),
        })
    }
}

#[cfg(feature = "v2")]
pub async fn populate_bin_details_for_payment_method_create(
    _card_details: api_models::payment_methods::CardDetail,
    _db: &dyn state::PaymentMethodsStorageInterface,
) -> api_models::payment_methods::CardDetail {
    todo!()
}

pub fn validate_payment_method_type_against_payment_method(
    payment_method: api_enums::PaymentMethod,
    payment_method_type: api_enums::PaymentMethodType,
) -> bool {
    match payment_method {
        #[cfg(feature = "v1")]
        api_enums::PaymentMethod::Card => matches!(
            payment_method_type,
            api_enums::PaymentMethodType::Credit | api_enums::PaymentMethodType::Debit
        ),
        #[cfg(feature = "v2")]
        api_enums::PaymentMethod::Card => matches!(
            payment_method_type,
            api_enums::PaymentMethodType::Credit
                | api_enums::PaymentMethodType::Debit
                | api_enums::PaymentMethodType::Card
        ),
        api_enums::PaymentMethod::PayLater => matches!(
            payment_method_type,
            api_enums::PaymentMethodType::Affirm
                | api_enums::PaymentMethodType::Alma
                | api_enums::PaymentMethodType::AfterpayClearpay
                | api_enums::PaymentMethodType::Klarna
                | api_enums::PaymentMethodType::PayBright
                | api_enums::PaymentMethodType::Atome
                | api_enums::PaymentMethodType::Walley
                | api_enums::PaymentMethodType::Breadpay
                | api_enums::PaymentMethodType::Flexiti
                | api_enums::PaymentMethodType::Payjustnow
        ),
        api_enums::PaymentMethod::Wallet => matches!(
            payment_method_type,
            api_enums::PaymentMethodType::AmazonPay
                | api_enums::PaymentMethodType::Bluecode
                | api_enums::PaymentMethodType::Paysera
                | api_enums::PaymentMethodType::Skrill
                | api_enums::PaymentMethodType::ApplePay
                | api_enums::PaymentMethodType::GooglePay
                | api_enums::PaymentMethodType::Paypal
                | api_enums::PaymentMethodType::AliPay
                | api_enums::PaymentMethodType::AliPayHk
                | api_enums::PaymentMethodType::Dana
                | api_enums::PaymentMethodType::MbWay
                | api_enums::PaymentMethodType::MobilePay
                | api_enums::PaymentMethodType::SamsungPay
                | api_enums::PaymentMethodType::Twint
                | api_enums::PaymentMethodType::Vipps
                | api_enums::PaymentMethodType::TouchNGo
                | api_enums::PaymentMethodType::Swish
                | api_enums::PaymentMethodType::WeChatPay
                | api_enums::PaymentMethodType::GoPay
                | api_enums::PaymentMethodType::Gcash
                | api_enums::PaymentMethodType::Momo
                | api_enums::PaymentMethodType::KakaoPay
                | api_enums::PaymentMethodType::Cashapp
                | api_enums::PaymentMethodType::Mifinity
                | api_enums::PaymentMethodType::Paze
                | api_enums::PaymentMethodType::RevolutPay
        ),
        api_enums::PaymentMethod::BankRedirect => matches!(
            payment_method_type,
            api_enums::PaymentMethodType::Giropay
                | api_enums::PaymentMethodType::Ideal
                | api_enums::PaymentMethodType::Sofort
                | api_enums::PaymentMethodType::Eft
                | api_enums::PaymentMethodType::Eps
                | api_enums::PaymentMethodType::BancontactCard
                | api_enums::PaymentMethodType::Blik
                | api_enums::PaymentMethodType::LocalBankRedirect
                | api_enums::PaymentMethodType::OnlineBankingThailand
                | api_enums::PaymentMethodType::OnlineBankingCzechRepublic
                | api_enums::PaymentMethodType::OnlineBankingFinland
                | api_enums::PaymentMethodType::OnlineBankingFpx
                | api_enums::PaymentMethodType::OnlineBankingPoland
                | api_enums::PaymentMethodType::OnlineBankingSlovakia
                | api_enums::PaymentMethodType::Przelewy24
                | api_enums::PaymentMethodType::Trustly
                | api_enums::PaymentMethodType::Bizum
                | api_enums::PaymentMethodType::Interac
                | api_enums::PaymentMethodType::OpenBankingUk
                | api_enums::PaymentMethodType::OpenBankingPIS
        ),
        api_enums::PaymentMethod::BankTransfer => matches!(
            payment_method_type,
            api_enums::PaymentMethodType::Ach
                | api_enums::PaymentMethodType::SepaBankTransfer
                | api_enums::PaymentMethodType::Bacs
                | api_enums::PaymentMethodType::Multibanco
                | api_enums::PaymentMethodType::Pix
                | api_enums::PaymentMethodType::Pse
                | api_enums::PaymentMethodType::PermataBankTransfer
                | api_enums::PaymentMethodType::BcaBankTransfer
                | api_enums::PaymentMethodType::BniVa
                | api_enums::PaymentMethodType::BriVa
                | api_enums::PaymentMethodType::CimbVa
                | api_enums::PaymentMethodType::DanamonVa
                | api_enums::PaymentMethodType::MandiriVa
                | api_enums::PaymentMethodType::LocalBankTransfer
                | api_enums::PaymentMethodType::InstantBankTransfer
                | api_enums::PaymentMethodType::InstantBankTransferFinland
                | api_enums::PaymentMethodType::InstantBankTransferPoland
                | api_enums::PaymentMethodType::IndonesianBankTransfer
        ),
        api_enums::PaymentMethod::BankDebit => matches!(
            payment_method_type,
            api_enums::PaymentMethodType::Ach
                | api_enums::PaymentMethodType::Sepa
                | api_enums::PaymentMethodType::SepaGuarenteedDebit
                | api_enums::PaymentMethodType::Bacs
                | api_enums::PaymentMethodType::Becs
        ),
        api_enums::PaymentMethod::Crypto => matches!(
            payment_method_type,
            api_enums::PaymentMethodType::CryptoCurrency
        ),
        api_enums::PaymentMethod::Reward => matches!(
            payment_method_type,
            api_enums::PaymentMethodType::Evoucher | api_enums::PaymentMethodType::ClassicReward
        ),
        api_enums::PaymentMethod::RealTimePayment => matches!(
            payment_method_type,
            api_enums::PaymentMethodType::Fps
                | api_enums::PaymentMethodType::DuitNow
                | api_enums::PaymentMethodType::PromptPay
                | api_enums::PaymentMethodType::VietQr
        ),
        api_enums::PaymentMethod::Upi => matches!(
            payment_method_type,
            api_enums::PaymentMethodType::UpiCollect
                | api_enums::PaymentMethodType::UpiIntent
                | api_enums::PaymentMethodType::UpiQr
        ),
        api_enums::PaymentMethod::Voucher => matches!(
            payment_method_type,
            api_enums::PaymentMethodType::Boleto
                | api_enums::PaymentMethodType::Efecty
                | api_enums::PaymentMethodType::PagoEfectivo
                | api_enums::PaymentMethodType::RedCompra
                | api_enums::PaymentMethodType::RedPagos
                | api_enums::PaymentMethodType::Indomaret
                | api_enums::PaymentMethodType::Alfamart
                | api_enums::PaymentMethodType::Oxxo
                | api_enums::PaymentMethodType::SevenEleven
                | api_enums::PaymentMethodType::Lawson
                | api_enums::PaymentMethodType::MiniStop
                | api_enums::PaymentMethodType::FamilyMart
                | api_enums::PaymentMethodType::Seicomart
                | api_enums::PaymentMethodType::PayEasy
        ),
        api_enums::PaymentMethod::GiftCard => {
            matches!(
                payment_method_type,
                api_enums::PaymentMethodType::Givex | api_enums::PaymentMethodType::PaySafeCard
            )
        }
        api_enums::PaymentMethod::CardRedirect => matches!(
            payment_method_type,
            api_enums::PaymentMethodType::Knet
                | api_enums::PaymentMethodType::Benefit
                | api_enums::PaymentMethodType::MomoAtm
                | api_enums::PaymentMethodType::CardRedirect
        ),
        api_enums::PaymentMethod::OpenBanking => matches!(
            payment_method_type,
            api_enums::PaymentMethodType::OpenBankingPIS
        ),
        api_enums::PaymentMethod::MobilePayment => matches!(
            payment_method_type,
            api_enums::PaymentMethodType::DirectCarrierBilling
        ),
    }
}

pub trait ForeignFrom<F> {
    fn foreign_from(from: F) -> Self;
}

/// Trait for converting from one foreign type to another
pub trait ForeignTryFrom<F>: Sized {
    /// Custom error for conversion failure
    type Error;
    /// Convert from a foreign type to the current type and return an error if the conversion fails
    fn foreign_try_from(from: F) -> Result<Self, Self::Error>;
}

#[cfg(feature = "v1")]
impl ForeignFrom<(Option<api::CardDetailFromLocker>, domain::PaymentMethod)>
    for api::PaymentMethodResponse
{
    fn foreign_from(
        (card_details, item): (Option<api::CardDetailFromLocker>, domain::PaymentMethod),
    ) -> Self {
        Self {
            merchant_id: item.merchant_id.to_owned(),
            customer_id: Some(item.customer_id.to_owned()),
            payment_method_id: item.get_id().clone(),
            payment_method: item.get_payment_method_type(),
            payment_method_type: item.get_payment_method_subtype(),
            card: card_details,
            recurring_enabled: Some(false),
            installment_payment_enabled: Some(false),
            payment_experience: None,
            metadata: item.metadata,
            created: Some(item.created_at),
            #[cfg(feature = "payouts")]
            bank_transfer: None,
            last_used_at: None,
            client_secret: item.client_secret,
        }
    }
}

#[cfg(feature = "v2")]
impl ForeignFrom<(Option<api::CardDetailFromLocker>, domain::PaymentMethod)>
    for api::PaymentMethodResponse
{
    fn foreign_from(
        (_card_details, _item): (Option<api::CardDetailFromLocker>, domain::PaymentMethod),
    ) -> Self {
        todo!()
    }
}

pub trait StorageErrorExt<T, E> {
    #[track_caller]
    fn to_not_found_response(self, not_found_response: E) -> error_stack::Result<T, E>;

    #[track_caller]
    fn to_duplicate_response(self, duplicate_response: E) -> error_stack::Result<T, E>;
}

impl<T> StorageErrorExt<T, api_error_response::ApiErrorResponse>
    for error_stack::Result<T, storage_impl::StorageError>
{
    #[track_caller]
    fn to_not_found_response(
        self,
        not_found_response: api_error_response::ApiErrorResponse,
    ) -> error_stack::Result<T, api_error_response::ApiErrorResponse> {
        self.map_err(|err| {
            let new_err = match err.current_context() {
                storage_impl::StorageError::ValueNotFound(_) => not_found_response,
                storage_impl::StorageError::CustomerRedacted => {
                    api_error_response::ApiErrorResponse::CustomerRedacted
                }
                _ => api_error_response::ApiErrorResponse::InternalServerError,
            };
            err.change_context(new_err)
        })
    }

    #[track_caller]
    fn to_duplicate_response(
        self,
        duplicate_response: api_error_response::ApiErrorResponse,
    ) -> error_stack::Result<T, api_error_response::ApiErrorResponse> {
        self.map_err(|err| {
            let new_err = match err.current_context() {
                storage_impl::StorageError::DuplicateValue { .. } => duplicate_response,
                _ => api_error_response::ApiErrorResponse::InternalServerError,
            };
            err.change_context(new_err)
        })
    }
}

#[cfg(feature = "v1")]
pub async fn validate_merchant_connector_ids_in_connector_mandate_details(
    state: &state::PaymentMethodsState,
    key_store: &merchant_key_store::MerchantKeyStore,
    connector_mandate_details: &api_models::payment_methods::CommonMandateReference,
    merchant_id: &id_type::MerchantId,
    card_network: Option<api_enums::CardNetwork>,
) -> CustomResult<(), api_error_response::ApiErrorResponse> {
    let db = &*state.store;
    let merchant_connector_account_list = db
        .find_merchant_connector_account_by_merchant_id_and_disabled_list(
            merchant_id,
            true,
            key_store,
        )
        .await
        .to_not_found_response(api_error_response::ApiErrorResponse::InternalServerError)?;

    let merchant_connector_account_details_hash_map: std::collections::HashMap<
        id_type::MerchantConnectorAccountId,
        merchant_connector_account::MerchantConnectorAccount,
    > = merchant_connector_account_list
        .iter()
        .map(|merchant_connector_account| {
            (
                merchant_connector_account.get_id(),
                merchant_connector_account.clone(),
            )
        })
        .collect();

    if let Some(payment_mandate_reference) = &connector_mandate_details.payments {
        let payments_map = payment_mandate_reference.0.clone();
        for (migrating_merchant_connector_id, migrating_connector_mandate_details) in payments_map {
            match (
                card_network.clone(),
                merchant_connector_account_details_hash_map.get(&migrating_merchant_connector_id),
            ) {
                (Some(api_enums::CardNetwork::Discover), Some(merchant_connector_account_details)) => {
                    if let ("cybersource", None) = (
                        merchant_connector_account_details.connector_name.as_str(),
                        migrating_connector_mandate_details
                            .original_payment_authorized_amount
                            .zip(
                                migrating_connector_mandate_details
                                    .original_payment_authorized_currency,
                            ),
                    ) {
                        Err(api_error_response::ApiErrorResponse::MissingRequiredFields {
                            field_names: vec![
                                "original_payment_authorized_currency",
                                "original_payment_authorized_amount",
                            ],
                        })
                        .attach_printable(format!(
                            "Invalid connector_mandate_details provided for connector {migrating_merchant_connector_id:?}",

                        ))?
                    }
                }
                (_, Some(_)) => (),
                (_, None) => Err(api_error_response::ApiErrorResponse::InvalidDataValue {
                    field_name: "merchant_connector_id",
                })
                .attach_printable_lazy(|| {
                    format!(
                        "{migrating_merchant_connector_id:?} invalid merchant connector id in connector_mandate_details",

                    )
                })?,
            }
        }
    } else {
        logger::error!("payment mandate reference not found");
    }
    Ok(())
}

/// This function replaces the request and response type of routerdata with the
/// request and response type passed
/// # Arguments
///
/// * `router_data` - original router data
/// * `request` - new request core/helper
/// * `response` - new response
pub fn router_data_type_conversion<F1, F2, Req1, Req2, Res1, Res2>(
    router_data: RouterData<F1, Req1, Res1>,
    request: Req2,
    response: Result<Res2, ErrorResponse>,
) -> RouterData<F2, Req2, Res2> {
    RouterData {
        flow: std::marker::PhantomData,
        request,
        response,
        merchant_id: router_data.merchant_id,
        tenant_id: router_data.tenant_id,
        address: router_data.address,
        amount_captured: router_data.amount_captured,
        minor_amount_captured: router_data.minor_amount_captured,
        auth_type: router_data.auth_type,
        connector: router_data.connector,
        connector_auth_type: router_data.connector_auth_type,
        connector_meta_data: router_data.connector_meta_data,
        description: router_data.description,
        payment_id: router_data.payment_id,
        payment_method: router_data.payment_method,
        payment_method_type: router_data.payment_method_type,
        status: router_data.status,
        attempt_id: router_data.attempt_id,
        access_token: router_data.access_token,
        session_token: router_data.session_token,
        payment_method_status: router_data.payment_method_status,
        reference_id: router_data.reference_id,
        payment_method_token: router_data.payment_method_token,
        customer_id: router_data.customer_id,
        connector_customer: router_data.connector_customer,
        preprocessing_id: router_data.preprocessing_id,
        payment_method_balance: router_data.payment_method_balance,
        recurring_mandate_payment_data: router_data.recurring_mandate_payment_data,
        connector_request_reference_id: router_data.connector_request_reference_id,
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode: router_data.test_mode,
        connector_api_version: router_data.connector_api_version,
        connector_http_status_code: router_data.connector_http_status_code,
        external_latency: router_data.external_latency,
        apple_pay_flow: router_data.apple_pay_flow,
        frm_metadata: router_data.frm_metadata,
        refund_id: router_data.refund_id,
        dispute_id: router_data.dispute_id,
        connector_response: router_data.connector_response,
        integrity_check: Ok(()),
        connector_wallets_details: router_data.connector_wallets_details,
        additional_merchant_data: router_data.additional_merchant_data,
        header_payload: router_data.header_payload,
        connector_mandate_request_reference_id: router_data.connector_mandate_request_reference_id,
        authentication_id: router_data.authentication_id,
        psd2_sca_exemption_type: router_data.psd2_sca_exemption_type,
        raw_connector_response: router_data.raw_connector_response,
        is_payment_id_from_merchant: router_data.is_payment_id_from_merchant,
        l2_l3_data: router_data.l2_l3_data,
        minor_amount_capturable: router_data.minor_amount_capturable,
        authorized_amount: router_data.authorized_amount,
    }
}
