use common_enums::{enums::PaymentMethod, AttemptStatus, AuthenticationType, PaymentMethodType};
use common_utils::request::Method;
use diesel_models::enums as storage_enums;
use error_stack::ResultExt;
use hyperswitch_connectors::utils::CardData;
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
    for payments_grpc::PaymentsAuthorizeRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        router_data: &RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let currency = payments_grpc::Currency::foreign_try_from(router_data.request.currency)?;

        let payment_method =
            payments_grpc::PaymentMethod::foreign_try_from(router_data.payment_method)?;

        let payment_method_type = router_data
            .request
            .payment_method_type
            .map(payments_grpc::PaymentMethodType::foreign_try_from)
            .transpose()?;

        let payment_method_data = payments_grpc::PaymentMethodData::foreign_try_from(
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
            payment_method: payment_method.into(),
            payment_method_type: payment_method_type
                .map(|payment_method_type| payment_method_type.into()),
            payment_method_data: Some(payment_method_data),
            connector_customer: router_data
                .request
                .customer_id
                .as_ref()
                .map(|id| id.get_string_repr().to_string()),
            return_url: router_data.request.router_return_url.clone(),
            address: Some(address),
            auth_type: auth_type.into(),
            connector_request_reference_id: router_data.connector_request_reference_id.clone(),
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
            connector_meta_data: router_data.connector_meta_data.as_ref().and_then(|secret| {
                let binding = secret.clone();
                let value = binding.peek(); // Expose the secret value
                serde_json::to_vec(&value)
                    .map_err(|err| {
                        // Handle or log error as needed
                        logger::error!(error=?err);
                        err
                    })
                    .ok()
            }),
            access_token: None,
            session_token: None,
            payment_method_token: None,
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

impl ForeignTryFrom<PaymentMethod> for payments_grpc::PaymentMethod {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(payment_method: PaymentMethod) -> Result<Self, Self::Error> {
        match payment_method {
            PaymentMethod::Card => Ok(Self::Card),
            _ => Err(UnifiedConnectorServiceError::NotImplemented(format!(
                "Unimplemented payment method: {:?}",
                payment_method
            ))
            .into()),
        }
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

impl ForeignTryFrom<hyperswitch_domain_models::payment_method_data::PaymentMethodData>
    for payments_grpc::PaymentMethodData
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        payment_method_data: hyperswitch_domain_models::payment_method_data::PaymentMethodData,
    ) -> Result<Self, Self::Error> {
        match payment_method_data {
            hyperswitch_domain_models::payment_method_data::PaymentMethodData::Card(card) => {
                Ok(Self {
                    data: Some(payments_grpc::payment_method_data::Data::Card(
                        payments_grpc::Card {
                            card_number: card.card_number.get_card_no(),
                            card_exp_month: card
                                .get_card_expiry_month_2_digit()
                                .attach_printable(
                                    "Failed to extract 2-digit expiry month from card",
                                )
                                .change_context(UnifiedConnectorServiceError::InvalidDataFormat {
                                    field_name: "card_exp_month",
                                })?
                                .peek()
                                .to_string(),
                            card_exp_year: card.get_expiry_year_4_digit().peek().to_string(),
                            card_cvc: card.card_cvc.peek().to_string(),
                            ..Default::default()
                        },
                    )),
                })
            }
            _ => Err(UnifiedConnectorServiceError::NotImplemented(format!(
                "Unimplemented payment method: {:?}",
                payment_method_data
            ))
            .into()),
        }
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

                Some(payments_grpc::Address {
                    address: address.address.as_ref().map(|details| {
                        payments_grpc::AddressDetails {
                            city: details.city.clone(),
                            country: Some(country),
                            line1: details.line1.as_ref().map(|l| l.peek().to_string()),
                            line2: details.line2.as_ref().map(|l| l.peek().to_string()),
                            line3: details.line3.as_ref().map(|l| l.peek().to_string()),
                            zip: details.zip.as_ref().map(|z| z.peek().to_string()),
                            state: details.state.as_ref().map(|s| s.peek().to_string()),
                            first_name: details.first_name.as_ref().map(|f| f.peek().to_string()),
                            last_name: details.last_name.as_ref().map(|l| l.peek().to_string()),
                        }
                    }),
                    phone: address
                        .phone
                        .as_ref()
                        .map(|phone| payments_grpc::PhoneDetails {
                            number: phone.number.as_ref().map(|n| n.peek().to_string()),
                            country_code: phone.country_code.clone(),
                        }),
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

                Some(payments_grpc::Address {
                    address: address.address.as_ref().map(|details| {
                        payments_grpc::AddressDetails {
                            city: details.city.clone(),
                            country: Some(country),
                            line1: details.line1.as_ref().map(|l| l.peek().to_string()),
                            line2: details.line2.as_ref().map(|l| l.peek().to_string()),
                            line3: details.line3.as_ref().map(|l| l.peek().to_string()),
                            zip: details.zip.as_ref().map(|z| z.peek().to_string()),
                            state: details.state.as_ref().map(|s| s.peek().to_string()),
                            first_name: details.first_name.as_ref().map(|f| f.peek().to_string()),
                            last_name: details.last_name.as_ref().map(|l| l.peek().to_string()),
                        }
                    }),
                    phone: address
                        .phone
                        .as_ref()
                        .map(|phone| payments_grpc::PhoneDetails {
                            number: phone.number.as_ref().map(|n| n.peek().to_string()),
                            country_code: phone.country_code.clone(),
                        }),
                    email: address.email.as_ref().map(|e| e.peek().to_string()),
                })
            }
            None => None,
        };

        let unified_payment_method_billing = match payment_address.get_payment_method_billing() {
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

                Some(payments_grpc::Address {
                    address: address.address.as_ref().map(|details| {
                        payments_grpc::AddressDetails {
                            city: details.city.clone(),
                            country: Some(country),
                            line1: details.line1.as_ref().map(|l| l.peek().to_string()),
                            line2: details.line2.as_ref().map(|l| l.peek().to_string()),
                            line3: details.line3.as_ref().map(|l| l.peek().to_string()),
                            zip: details.zip.as_ref().map(|z| z.peek().to_string()),
                            state: details.state.as_ref().map(|s| s.peek().to_string()),
                            first_name: details.first_name.as_ref().map(|f| f.peek().to_string()),
                            last_name: details.last_name.as_ref().map(|l| l.peek().to_string()),
                        }
                    }),
                    phone: address
                        .phone
                        .as_ref()
                        .map(|phone| payments_grpc::PhoneDetails {
                            number: phone.number.as_ref().map(|n| n.peek().to_string()),
                            country_code: phone.country_code.clone(),
                        }),
                    email: address.email.as_ref().map(|e| e.peek().to_string()),
                })
            }
            None => None,
        };

        Ok(Self {
            shipping: shipping.clone(),
            billing: billing.clone(),
            unified_payment_method_billing: unified_payment_method_billing.clone(),
            payment_method_billing: unified_payment_method_billing.clone(),
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
            time_zone: browser_info.time_zone,
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
            threeds_server_transaction_id: authentication_data.threeds_server_transaction_id,
            message_version: None,
            ds_trans_id: authentication_data.ds_trans_id,
        })
    }
}

impl ForeignTryFrom<payments_grpc::AttemptStatus> for AttemptStatus {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(grpc_status: payments_grpc::AttemptStatus) -> Result<Self, Self::Error> {
        match grpc_status {
            payments_grpc::AttemptStatus::Started => Ok(Self::Started),
            payments_grpc::AttemptStatus::AuthenticationFailed => Ok(Self::AuthenticationFailed),
            payments_grpc::AttemptStatus::RouterDeclined => Ok(Self::RouterDeclined),
            payments_grpc::AttemptStatus::AuthenticationPending => Ok(Self::AuthenticationPending),
            payments_grpc::AttemptStatus::AuthenticationSuccessful => {
                Ok(Self::AuthenticationSuccessful)
            }
            payments_grpc::AttemptStatus::Authorized => Ok(Self::Authorized),
            payments_grpc::AttemptStatus::AuthorizationFailed => Ok(Self::AuthorizationFailed),
            payments_grpc::AttemptStatus::Charged => Ok(Self::Charged),
            payments_grpc::AttemptStatus::Authorizing => Ok(Self::Authorizing),
            payments_grpc::AttemptStatus::CodInitiated => Ok(Self::CodInitiated),
            payments_grpc::AttemptStatus::Voided => Ok(Self::Voided),
            payments_grpc::AttemptStatus::VoidInitiated => Ok(Self::VoidInitiated),
            payments_grpc::AttemptStatus::CaptureInitiated => Ok(Self::CaptureInitiated),
            payments_grpc::AttemptStatus::CaptureFailed => Ok(Self::CaptureFailed),
            payments_grpc::AttemptStatus::VoidFailed => Ok(Self::VoidFailed),
            payments_grpc::AttemptStatus::AutoRefunded => Ok(Self::AutoRefunded),
            payments_grpc::AttemptStatus::PartialCharged => Ok(Self::PartialCharged),
            payments_grpc::AttemptStatus::PartialChargedAndChargeable => {
                Ok(Self::PartialChargedAndChargeable)
            }
            payments_grpc::AttemptStatus::Unresolved => Ok(Self::Unresolved),
            payments_grpc::AttemptStatus::Pending => Ok(Self::Pending),
            payments_grpc::AttemptStatus::Failure => Ok(Self::Failure),
            payments_grpc::AttemptStatus::PaymentMethodAwaited => Ok(Self::PaymentMethodAwaited),
            payments_grpc::AttemptStatus::ConfirmationAwaited => Ok(Self::ConfirmationAwaited),
            payments_grpc::AttemptStatus::DeviceDataCollectionPending => {
                Ok(Self::DeviceDataCollectionPending)
            }
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

impl ForeignTryFrom<payments_grpc::Method> for Method {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(value: payments_grpc::Method) -> Result<Self, Self::Error> {
        match value {
            payments_grpc::Method::Get => Ok(Self::Get),
            payments_grpc::Method::Post => Ok(Self::Post),
        }
    }
}

impl ForeignTryFrom<payments_grpc::MandateReference> for MandateReference {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        mandate_reference: payments_grpc::MandateReference,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            connector_mandate_id: mandate_reference.connector_mandate_id,
            payment_method_id: None,
            mandate_metadata: None,
            connector_mandate_request_reference_id: None,
        })
    }
}
