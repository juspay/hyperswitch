use common_enums::{AttemptStatus, AuthenticationType, PaymentMethodType};
use common_utils::request::Method;
use diesel_models::enums as storage_enums;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::RouterData,
    router_flow_types::payments::Authorize,
    router_request_types::{AuthenticationData, PaymentsAuthorizeData},
    router_response_types::{MandateReference, PaymentsResponseData, RedirectForm},
};
use masking::{ExposeInterface, PeekInterface};
use router_env::logger;
use rust_grpc_client::payments as payments_grpc;

use crate::{
    core::unified_connector_service::errors::UnifiedConnectorServiceError,
    types::transformers::ForeignTryFrom,
};

impl ForeignTryFrom<&RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>>
    for payments_grpc::PaymentServiceAuthorizeRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        router_data: &RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let currency = payments_grpc::Currency::foreign_try_from(router_data.request.currency)?;

        let payment_method_data = payments_grpc::PaymentMethod::foreign_try_from(
            router_data.request.payment_method_data.clone(),
        )?;

        let address = payments_grpc::PaymentAddress::foreign_try_from(router_data.address.clone())?;

        let auth_type = payments_grpc::AuthenticationType::foreign_try_from(router_data.auth_type)?;

        let browser_info = router_data
            .request
            .browser_info
            .clone()
            .map(payments_grpc::BrowserInformation::foreign_try_from)
            .transpose()?;

        let capture_method = router_data
            .request
            .capture_method
            .map(payments_grpc::CaptureMethod::foreign_try_from)
            .transpose()?;

        let authentication_data = router_data
            .request
            .authentication_data
            .clone()
            .map(payments_grpc::AuthenticationData::foreign_try_from)
            .transpose()?;

        Ok(Self {
            amount: router_data.request.amount,
            currency: currency.into(),
            payment_method: Some(payment_method_data),
            connector_customer_id: router_data
                .request
                .customer_id
                .as_ref()
                .map(|id| id.get_string_repr().to_string()),
            return_url: router_data.request.router_return_url.clone(),
            address: Some(address),
            auth_type: auth_type.into(),
            request_ref_id: Some(payments_grpc::Identifier {
                id_type: Some(payments_grpc::identifier::IdType::Id(
                    router_data.connector_request_reference_id.clone(),
                )),
            }),
            enrolled_for_3ds: router_data.request.enrolled_for_3ds,
            request_incremental_authorization: router_data
                .request
                .request_incremental_authorization,
            minor_amount: router_data.request.amount,
            email: router_data
                .request
                .email
                .clone()
                .map(|e| e.expose().peek().clone()),
            browser_info,
            metadata: router_data
                .connector_meta_data
                .as_ref()
                .and_then(|secret| {
                    let binding = secret.clone();
                    let value = binding.peek(); // Expose the secret value
                    serde_json::from_value::<std::collections::HashMap<String, String>>(
                        value.clone(),
                    )
                    .map_err(|err| {
                        logger::error!(error=?err);
                        err
                    })
                    .ok()
                })
                .unwrap_or_default(),
            access_token: None,
            session_token: None,
            order_tax_amount: router_data
                .request
                .order_tax_amount
                .map(|order_tax_amount| order_tax_amount.get_amount_as_i64()),
            customer_name: router_data
                .request
                .customer_name
                .clone()
                .map(|customer_name| customer_name.peek().to_owned()),
            capture_method: capture_method.map(|capture_method| capture_method.into()),
            webhook_url: router_data.request.webhook_url.clone(),
            complete_authorize_url: router_data.request.complete_authorize_url.clone(),
            setup_future_usage: None,
            off_session: None,
            customer_acceptance: None,
            order_category: router_data.request.order_category.clone(),
            payment_experience: None,
            authentication_data,
            request_extended_authorization: router_data
                .request
                .request_extended_authorization
                .map(|request_extended_authorization| request_extended_authorization.is_true()),
            merchant_order_reference_id: router_data.request.merchant_order_reference_id.clone(),
            shipping_cost: router_data
                .request
                .shipping_cost
                .map(|shipping_cost| shipping_cost.get_amount_as_i64()),
        })
    }
}

impl ForeignTryFrom<common_enums::Currency> for payments_grpc::Currency {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(currency: common_enums::Currency) -> Result<Self, Self::Error> {
        Self::from_str_name(&currency.to_string()).ok_or_else(|| {
            UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                "Failed to parse currency".to_string(),
            )
            .into()
        })
    }
}

impl ForeignTryFrom<PaymentMethodType> for payments_grpc::PaymentMethodType {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(payment_method_type: PaymentMethodType) -> Result<Self, Self::Error> {
        match payment_method_type {
            PaymentMethodType::Ach => Ok(Self::Ach),
            PaymentMethodType::Affirm => Ok(Self::Affirm),
            PaymentMethodType::AfterpayClearpay => Ok(Self::AfterpayClearpay),
            PaymentMethodType::Alfamart => Ok(Self::Alfamart),
            PaymentMethodType::AliPay => Ok(Self::AliPay),
            PaymentMethodType::AliPayHk => Ok(Self::AliPayHk),
            PaymentMethodType::Alma => Ok(Self::Alma),
            PaymentMethodType::AmazonPay => Ok(Self::AmazonPay),
            PaymentMethodType::ApplePay => Ok(Self::ApplePay),
            PaymentMethodType::Atome => Ok(Self::Atome),
            PaymentMethodType::Bacs => Ok(Self::Bacs),
            PaymentMethodType::BancontactCard => Ok(Self::BancontactCard),
            PaymentMethodType::Becs => Ok(Self::Becs),
            PaymentMethodType::Benefit => Ok(Self::Benefit),
            PaymentMethodType::Bizum => Ok(Self::Bizum),
            PaymentMethodType::Blik => Ok(Self::Blik),
            PaymentMethodType::Boleto => Ok(Self::Boleto),
            PaymentMethodType::BcaBankTransfer => Ok(Self::BcaBankTransfer),
            PaymentMethodType::BniVa => Ok(Self::BniVa),
            PaymentMethodType::BriVa => Ok(Self::BriVa),
            PaymentMethodType::CardRedirect => Ok(Self::CardRedirect),
            PaymentMethodType::CimbVa => Ok(Self::CimbVa),
            PaymentMethodType::ClassicReward => Ok(Self::ClassicReward),
            PaymentMethodType::Credit => Ok(Self::Credit),
            PaymentMethodType::CryptoCurrency => Ok(Self::CryptoCurrency),
            PaymentMethodType::Cashapp => Ok(Self::Cashapp),
            PaymentMethodType::Dana => Ok(Self::Dana),
            PaymentMethodType::DanamonVa => Ok(Self::DanamonVa),
            PaymentMethodType::Debit => Ok(Self::Debit),
            PaymentMethodType::DuitNow => Ok(Self::DuitNow),
            PaymentMethodType::Efecty => Ok(Self::Efecty),
            PaymentMethodType::Eft => Ok(Self::Eft),
            PaymentMethodType::Eps => Ok(Self::Eps),
            PaymentMethodType::Fps => Ok(Self::Fps),
            PaymentMethodType::Evoucher => Ok(Self::Evoucher),
            PaymentMethodType::Giropay => Ok(Self::Giropay),
            PaymentMethodType::Givex => Ok(Self::Givex),
            PaymentMethodType::GooglePay => Ok(Self::GooglePay),
            PaymentMethodType::GoPay => Ok(Self::GoPay),
            PaymentMethodType::Gcash => Ok(Self::Gcash),
            PaymentMethodType::Ideal => Ok(Self::Ideal),
            PaymentMethodType::Interac => Ok(Self::Interac),
            PaymentMethodType::Indomaret => Ok(Self::Indomaret),
            PaymentMethodType::KakaoPay => Ok(Self::KakaoPay),
            PaymentMethodType::LocalBankRedirect => Ok(Self::LocalBankRedirect),
            PaymentMethodType::MandiriVa => Ok(Self::MandiriVa),
            PaymentMethodType::Knet => Ok(Self::Knet),
            PaymentMethodType::MbWay => Ok(Self::MbWay),
            PaymentMethodType::MobilePay => Ok(Self::MobilePay),
            PaymentMethodType::Momo => Ok(Self::Momo),
            PaymentMethodType::MomoAtm => Ok(Self::MomoAtm),
            PaymentMethodType::Multibanco => Ok(Self::Multibanco),
            PaymentMethodType::OnlineBankingThailand => Ok(Self::OnlineBankingThailand),
            PaymentMethodType::OnlineBankingCzechRepublic => Ok(Self::OnlineBankingCzechRepublic),
            PaymentMethodType::OnlineBankingFinland => Ok(Self::OnlineBankingFinland),
            PaymentMethodType::OnlineBankingFpx => Ok(Self::OnlineBankingFpx),
            PaymentMethodType::OnlineBankingPoland => Ok(Self::OnlineBankingPoland),
            PaymentMethodType::OnlineBankingSlovakia => Ok(Self::OnlineBankingSlovakia),
            PaymentMethodType::Oxxo => Ok(Self::Oxxo),
            PaymentMethodType::PagoEfectivo => Ok(Self::PagoEfectivo),
            PaymentMethodType::PermataBankTransfer => Ok(Self::PermataBankTransfer),
            PaymentMethodType::OpenBankingUk => Ok(Self::OpenBankingUk),
            PaymentMethodType::PayBright => Ok(Self::PayBright),
            PaymentMethodType::Paze => Ok(Self::Paze),
            PaymentMethodType::Pix => Ok(Self::Pix),
            PaymentMethodType::PaySafeCard => Ok(Self::PaySafeCard),
            PaymentMethodType::Przelewy24 => Ok(Self::Przelewy24),
            PaymentMethodType::PromptPay => Ok(Self::PromptPay),
            PaymentMethodType::Pse => Ok(Self::Pse),
            PaymentMethodType::RedCompra => Ok(Self::RedCompra),
            PaymentMethodType::RedPagos => Ok(Self::RedPagos),
            PaymentMethodType::SamsungPay => Ok(Self::SamsungPay),
            PaymentMethodType::Sepa => Ok(Self::Sepa),
            PaymentMethodType::SepaBankTransfer => Ok(Self::SepaBankTransfer),
            PaymentMethodType::Sofort => Ok(Self::Sofort),
            PaymentMethodType::Swish => Ok(Self::Swish),
            PaymentMethodType::TouchNGo => Ok(Self::TouchNGo),
            PaymentMethodType::Trustly => Ok(Self::Trustly),
            PaymentMethodType::Twint => Ok(Self::Twint),
            PaymentMethodType::UpiCollect => Ok(Self::UpiCollect),
            PaymentMethodType::UpiIntent => Ok(Self::UpiIntent),
            PaymentMethodType::Vipps => Ok(Self::Vipps),
            PaymentMethodType::VietQr => Ok(Self::VietQr),
            PaymentMethodType::Venmo => Ok(Self::Venmo),
            PaymentMethodType::Walley => Ok(Self::Walley),
            PaymentMethodType::WeChatPay => Ok(Self::WeChatPay),
            PaymentMethodType::SevenEleven => Ok(Self::SevenEleven),
            PaymentMethodType::Lawson => Ok(Self::Lawson),
            PaymentMethodType::MiniStop => Ok(Self::MiniStop),
            PaymentMethodType::FamilyMart => Ok(Self::FamilyMart),
            PaymentMethodType::Seicomart => Ok(Self::Seicomart),
            PaymentMethodType::PayEasy => Ok(Self::PayEasy),
            PaymentMethodType::LocalBankTransfer => Ok(Self::LocalBankTransfer),
            PaymentMethodType::OpenBankingPIS => Ok(Self::OpenBankingPis),
            PaymentMethodType::DirectCarrierBilling => Ok(Self::DirectCarrierBilling),
            PaymentMethodType::InstantBankTransfer => Ok(Self::InstantBankTransfer),
            _ => Err(UnifiedConnectorServiceError::NotImplemented(format!(
                "Unimplemented payment method type: {:?}",
                payment_method_type
            ))
            .into()),
        }
    }
}

impl ForeignTryFrom<common_enums::CardNetwork> for payments_grpc::CardNetwork {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(card_network: common_enums::CardNetwork) -> Result<Self, Self::Error> {
        Self::from_str_name(&card_network.to_string()).ok_or_else(|| {
            UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                "Failed to parse card network".to_string(),
            )
            .into()
        })
    }
}

impl ForeignTryFrom<hyperswitch_domain_models::payment_method_data::PaymentMethodData>
    for payments_grpc::PaymentMethod
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        value: hyperswitch_domain_models::payment_method_data::PaymentMethodData,
    ) -> Result<Self, Self::Error> {
        Ok(match value {
            hyperswitch_domain_models::payment_method_data::PaymentMethodData::Card(card) => {
                Self {
                    payment_method: Some(payments_grpc::payment_method::PaymentMethod::Card(
                        payments_grpc::CardPaymentMethodType {
                            card_type: card.card_type.as_ref().map(|card_type| {
                                match card_type.as_str() {
                                    "credit" => payments_grpc::card_payment_method_type::CardType::Credit(
                                        payments_grpc::CardDetails {
                                            card_number: card.card_number.peek().to_string(),
                                            card_exp_month: card.card_exp_month.peek().clone(),
                                            card_exp_year: card.card_exp_year.peek().clone(),
                                            card_cvc: card.card_cvc.peek().clone(),
                                            card_holder_name: card.card_holder_name.as_ref().map(|n| n.peek().to_string()),
                                            card_issuer: card.card_issuer.clone(),
                                            #[allow(clippy::as_conversions)]
                                            card_network: card.card_network.clone().and_then(|network|
                                                payments_grpc::CardNetwork::foreign_try_from(network).ok()
                                            ).map(|n| n as i32),
                                            card_type: card.card_type.clone(),
                                            bank_code: card.bank_code.clone(),
                                            nick_name: card.nick_name.as_ref().map(|n| n.peek().to_string()),
                                            card_issuing_country_alpha2: card.card_issuing_country.clone(),
                                        }
                                    ),
                                    "debit" => payments_grpc::card_payment_method_type::CardType::Debit(
                                        payments_grpc::CardDetails {
                                            card_number: card.card_number.peek().to_string(),
                                            card_exp_month: card.card_exp_month.peek().clone(),
                                            card_exp_year: card.card_exp_year.peek().clone(),
                                            card_cvc: card.card_cvc.peek().clone(),
                                            card_holder_name: card.card_holder_name.as_ref().map(|n| n.peek().to_string()),
                                            card_issuer: card.card_issuer.clone(),
                                            #[allow(clippy::as_conversions)]
                                            card_network: card.card_network.clone().and_then(|network|
                                                payments_grpc::CardNetwork::foreign_try_from(network).ok()
                                            ).map(|n| n as i32),
                                            card_type: card.card_type.clone(),
                                            bank_code: card.bank_code.clone(),
                                            nick_name: card.nick_name.as_ref().map(|n| n.peek().to_string()),
                                            card_issuing_country_alpha2: card.card_issuing_country.clone(),
                                        }
                                    ),
                                    _ => {
                                        // Default to credit if card_type is not recognized
                                        payments_grpc::card_payment_method_type::CardType::Credit(
                                            payments_grpc::CardDetails {
                                                card_number: card.card_number.peek().to_string(),
                                                card_exp_month: card.card_exp_month.peek().clone(),
                                                card_exp_year: card.card_exp_year.peek().clone(),
                                                card_cvc: card.card_cvc.peek().clone(),
                                                card_holder_name: card.card_holder_name.as_ref().map(|n| n.peek().to_string()),
                                                card_issuer: card.card_issuer.clone(),
                                                #[allow(clippy::as_conversions)]
                                                card_network: card.card_network.clone().and_then(|network|
                                                    payments_grpc::CardNetwork::foreign_try_from(network).ok()
                                                ).map(|n| n as i32),
                                                card_type: card.card_type.clone(),
                                                bank_code: card.bank_code.clone(),
                                                nick_name: card.nick_name.as_ref().map(|n| n.peek().to_string()),
                                                card_issuing_country_alpha2: card.card_issuing_country.clone(),
                                            }
                                        )
                                    }
                                }
                            }).or_else(|| {
                                // If card_type is None, default to credit
                                Some(payments_grpc::card_payment_method_type::CardType::Credit(
                                    payments_grpc::CardDetails {
                                        card_number: card.card_number.peek().to_string(),
                                        card_exp_month: card.card_exp_month.peek().clone(),
                                        card_exp_year: card.card_exp_year.peek().clone(),
                                        card_cvc: card.card_cvc.peek().clone(),
                                        card_holder_name: card.card_holder_name.as_ref().map(|n| n.peek().to_string()),
                                        card_issuer: card.card_issuer.clone(),
                                        #[allow(clippy::as_conversions)]
                                        card_network: card.card_network.clone().and_then(|network|
                                            payments_grpc::CardNetwork::foreign_try_from(network).ok()
                                        ).map(|n| n as i32),
                                        card_type: card.card_type.clone(),
                                        bank_code: card.bank_code.clone(),
                                        nick_name: card.nick_name.as_ref().map(|n| n.peek().to_string()),
                                        card_issuing_country_alpha2: card.card_issuing_country.clone(),
                                    }
                                ))
                            }),
                        }
                    )),
                }
            }
            _ => {
                // For unsupported payment methods, return an empty payment method
                // This could be changed to return an error if needed
                Self {
                    payment_method: None,
                }
            }
        })
    }
}

impl ForeignTryFrom<hyperswitch_domain_models::payment_address::PaymentAddress>
    for payments_grpc::PaymentAddress
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        payment_address: hyperswitch_domain_models::payment_address::PaymentAddress,
    ) -> Result<Self, Self::Error> {
        let shipping = match payment_address.get_shipping() {
            Some(address) => {
                let country = address
                    .address
                    .as_ref()
                    .and_then(|details| {
                        details.country.as_ref().and_then(|c| {
                            payments_grpc::CountryAlpha2::from_str_name(&c.to_string())
                        })
                    })
                    .ok_or_else(|| {
                        UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                            "Invalid country code".to_string(),
                        )
                    })
                    .attach_printable("Invalid country code")?
                    .into();

                address
                    .address
                    .as_ref()
                    .map(|details| payments_grpc::Address {
                        city: details.city.clone(),
                        country_alpha2_code: Some(country),
                        line1: details.line1.as_ref().map(|l| l.peek().to_string()),
                        line2: details.line2.as_ref().map(|l| l.peek().to_string()),
                        line3: details.line3.as_ref().map(|l| l.peek().to_string()),
                        zip_code: details.zip.as_ref().map(|z| z.peek().to_string()),
                        state: details.state.as_ref().map(|s| s.peek().to_string()),
                        first_name: details.first_name.as_ref().map(|f| f.peek().to_string()),
                        last_name: details.last_name.as_ref().map(|l| l.peek().to_string()),
                        phone_number: address
                            .phone
                            .as_ref()
                            .and_then(|phone| phone.number.as_ref().map(|n| n.peek().to_string())),
                        phone_country_code: address
                            .phone
                            .as_ref()
                            .and_then(|phone| phone.country_code.clone()),
                        email: address.email.as_ref().map(|e| e.peek().to_string()),
                    })
            }
            None => None,
        };

        let billing = match payment_address.get_payment_billing() {
            Some(address) => {
                let country = address
                    .address
                    .as_ref()
                    .and_then(|details| {
                        details.country.as_ref().and_then(|c| {
                            payments_grpc::CountryAlpha2::from_str_name(&c.to_string())
                        })
                    })
                    .ok_or_else(|| {
                        UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                            "Invalid country code".to_string(),
                        )
                    })
                    .attach_printable("Invalid country code")?
                    .into();

                address
                    .address
                    .as_ref()
                    .map(|details| payments_grpc::Address {
                        city: details.city.clone(),
                        country_alpha2_code: Some(country),
                        line1: details.line1.as_ref().map(|l| l.peek().to_string()),
                        line2: details.line2.as_ref().map(|l| l.peek().to_string()),
                        line3: details.line3.as_ref().map(|l| l.peek().to_string()),
                        zip_code: details.zip.as_ref().map(|z| z.peek().to_string()),
                        state: details.state.as_ref().map(|s| s.peek().to_string()),
                        first_name: details.first_name.as_ref().map(|f| f.peek().to_string()),
                        last_name: details.last_name.as_ref().map(|l| l.peek().to_string()),
                        phone_number: address
                            .phone
                            .as_ref()
                            .and_then(|phone| phone.number.as_ref().map(|n| n.peek().to_string())),
                        phone_country_code: address
                            .phone
                            .as_ref()
                            .and_then(|phone| phone.country_code.clone()),
                        email: address.email.as_ref().map(|e| e.peek().to_string()),
                    })
            }
            None => None,
        };

        Ok(Self {
            shipping_address: shipping.clone(),
            billing_address: billing.clone(),
        })
    }
}

impl ForeignTryFrom<AuthenticationType> for payments_grpc::AuthenticationType {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(auth_type: AuthenticationType) -> Result<Self, Self::Error> {
        match auth_type {
            AuthenticationType::ThreeDs => Ok(Self::ThreeDs),
            AuthenticationType::NoThreeDs => Ok(Self::NoThreeDs),
        }
    }
}

impl ForeignTryFrom<hyperswitch_domain_models::router_request_types::BrowserInformation>
    for payments_grpc::BrowserInformation
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        browser_info: hyperswitch_domain_models::router_request_types::BrowserInformation,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            color_depth: browser_info.color_depth.map(|v| v.into()),
            java_enabled: browser_info.java_enabled,
            java_script_enabled: browser_info.java_script_enabled,
            language: browser_info.language,
            screen_height: browser_info.screen_height,
            screen_width: browser_info.screen_width,
            time_zone_offset_minutes: browser_info.time_zone,
            ip_address: browser_info.ip_address.map(|ip| ip.to_string()),
            accept_header: browser_info.accept_header,
            user_agent: browser_info.user_agent,
            os_type: browser_info.os_type,
            os_version: browser_info.os_version,
            device_model: browser_info.device_model,
            accept_language: browser_info.accept_language,
        })
    }
}

impl ForeignTryFrom<storage_enums::CaptureMethod> for payments_grpc::CaptureMethod {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(capture_method: storage_enums::CaptureMethod) -> Result<Self, Self::Error> {
        match capture_method {
            common_enums::CaptureMethod::Automatic => Ok(Self::Automatic),
            common_enums::CaptureMethod::Manual => Ok(Self::Manual),
            common_enums::CaptureMethod::ManualMultiple => Ok(Self::ManualMultiple),
            common_enums::CaptureMethod::Scheduled => Ok(Self::Scheduled),
            common_enums::CaptureMethod::SequentialAutomatic => Ok(Self::SequentialAutomatic),
        }
    }
}

impl ForeignTryFrom<AuthenticationData> for payments_grpc::AuthenticationData {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(authentication_data: AuthenticationData) -> Result<Self, Self::Error> {
        Ok(Self {
            eci: authentication_data.eci,
            cavv: authentication_data.cavv.peek().to_string(),
            threeds_server_transaction_id: authentication_data.threeds_server_transaction_id.map(
                |s| payments_grpc::Identifier {
                    id_type: Some(payments_grpc::identifier::IdType::Id(s)),
                },
            ),
            message_version: None,
            ds_transaction_id: authentication_data.ds_trans_id,
        })
    }
}

impl ForeignTryFrom<payments_grpc::PaymentStatus> for AttemptStatus {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(grpc_status: payments_grpc::PaymentStatus) -> Result<Self, Self::Error> {
        match grpc_status {
            payments_grpc::PaymentStatus::Started => Ok(Self::Started),
            payments_grpc::PaymentStatus::AuthenticationFailed => Ok(Self::AuthenticationFailed),
            payments_grpc::PaymentStatus::RouterDeclined => Ok(Self::RouterDeclined),
            payments_grpc::PaymentStatus::AuthenticationPending => Ok(Self::AuthenticationPending),
            payments_grpc::PaymentStatus::AuthenticationSuccessful => {
                Ok(Self::AuthenticationSuccessful)
            }
            payments_grpc::PaymentStatus::Authorized => Ok(Self::Authorized),
            payments_grpc::PaymentStatus::AuthorizationFailed => Ok(Self::AuthorizationFailed),
            payments_grpc::PaymentStatus::Charged => Ok(Self::Charged),
            payments_grpc::PaymentStatus::Authorizing => Ok(Self::Authorizing),
            payments_grpc::PaymentStatus::CodInitiated => Ok(Self::CodInitiated),
            payments_grpc::PaymentStatus::Voided => Ok(Self::Voided),
            payments_grpc::PaymentStatus::VoidInitiated => Ok(Self::VoidInitiated),
            payments_grpc::PaymentStatus::CaptureInitiated => Ok(Self::CaptureInitiated),
            payments_grpc::PaymentStatus::CaptureFailed => Ok(Self::CaptureFailed),
            payments_grpc::PaymentStatus::VoidFailed => Ok(Self::VoidFailed),
            payments_grpc::PaymentStatus::AutoRefunded => Ok(Self::AutoRefunded),
            payments_grpc::PaymentStatus::PartialCharged => Ok(Self::PartialCharged),
            payments_grpc::PaymentStatus::PartialChargedAndChargeable => {
                Ok(Self::PartialChargedAndChargeable)
            }
            payments_grpc::PaymentStatus::Unresolved => Ok(Self::Unresolved),
            payments_grpc::PaymentStatus::Pending => Ok(Self::Pending),
            payments_grpc::PaymentStatus::Failure => Ok(Self::Failure),
            payments_grpc::PaymentStatus::PaymentMethodAwaited => Ok(Self::PaymentMethodAwaited),
            payments_grpc::PaymentStatus::ConfirmationAwaited => Ok(Self::ConfirmationAwaited),
            payments_grpc::PaymentStatus::DeviceDataCollectionPending => {
                Ok(Self::DeviceDataCollectionPending)
            }
            payments_grpc::PaymentStatus::AttemptStatusUnspecified => Ok(Self::Pending),
        }
    }
}

impl ForeignTryFrom<payments_grpc::RedirectForm> for RedirectForm {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(value: payments_grpc::RedirectForm) -> Result<Self, Self::Error> {
        match value.form_type {
            Some(payments_grpc::redirect_form::FormType::Form(form)) => Ok(Self::Form {
                endpoint: form.clone().endpoint,
                method: Method::foreign_try_from(form.clone().method())?,
                form_fields: form.clone().form_fields,
            }),
            Some(payments_grpc::redirect_form::FormType::Html(html)) => Ok(Self::Html {
                html_data: html.html_data,
            }),
            None => Err(
                UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                    "Missing form type".to_string(),
                )
                .into(),
            ),
        }
    }
}

impl ForeignTryFrom<payments_grpc::HttpMethod> for Method {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(value: payments_grpc::HttpMethod) -> Result<Self, Self::Error> {
        match value {
            payments_grpc::HttpMethod::Get => Ok(Self::Get),
            payments_grpc::HttpMethod::Post => Ok(Self::Post),
            payments_grpc::HttpMethod::Put => Ok(Self::Put),
            payments_grpc::HttpMethod::Delete => Ok(Self::Delete),
            payments_grpc::HttpMethod::Unspecified => Ok(Self::Get), // Default to GET
        }
    }
}

impl ForeignTryFrom<payments_grpc::MandateReference> for MandateReference {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        mandate_reference: payments_grpc::MandateReference,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            connector_mandate_id: mandate_reference.mandate_id,
            payment_method_id: None,
            mandate_metadata: None,
            connector_mandate_request_reference_id: None,
        })
    }
}
