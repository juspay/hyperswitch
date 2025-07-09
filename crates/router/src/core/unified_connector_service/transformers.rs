use std::collections::HashMap;

use common_enums::{AttemptStatus, AuthenticationType};
use common_utils::request::Method;
use diesel_models::enums as storage_enums;
use error_stack::ResultExt;
use external_services::grpc_client::unified_connector_service::UnifiedConnectorServiceError;
use hyperswitch_domain_models::{
    router_data::RouterData,
    router_flow_types::payments::{Authorize, PSync},
    router_request_types::{AuthenticationData, PaymentsAuthorizeData, PaymentsSyncData},
    router_response_types::{PaymentsResponseData, RedirectForm},
};
use masking::{ExposeInterface, PeekInterface};
use unified_connector_service_client::payments::{self as payments_grpc, Identifier};

use crate::{
    core::unified_connector_service::build_unified_connector_service_payment_method,
    types::transformers::ForeignTryFrom,
};
impl ForeignTryFrom<&RouterData<PSync, PaymentsSyncData, PaymentsResponseData>>
    for payments_grpc::PaymentServiceGetRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        router_data: &RouterData<PSync, PaymentsSyncData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let connector_transaction_id = router_data
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .map(|id| Identifier {
                id_type: Some(payments_grpc::identifier::IdType::Id(id)),
            })
            .ok();

        let connector_ref_id = router_data
            .request
            .connector_reference_id
            .clone()
            .map(|id| Identifier {
                id_type: Some(payments_grpc::identifier::IdType::Id(id)),
            });

        Ok(Self {
            transaction_id: connector_transaction_id,
            request_ref_id: connector_ref_id,
        })
    }
}

impl ForeignTryFrom<&RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>>
    for payments_grpc::PaymentServiceAuthorizeRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        router_data: &RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let currency = payments_grpc::Currency::foreign_try_from(router_data.request.currency)?;

        let payment_method = router_data
            .request
            .payment_method_type
            .map(|payment_method_type| {
                build_unified_connector_service_payment_method(
                    router_data.request.payment_method_data.clone(),
                    payment_method_type,
                )
            })
            .transpose()?;

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
            payment_method,
            return_url: router_data.request.router_return_url.clone(),
            address: Some(address),
            auth_type: auth_type.into(),
            enrolled_for_3ds: router_data.request.enrolled_for_3ds,
            request_incremental_authorization: router_data
                .request
                .request_incremental_authorization,
            minor_amount: router_data.request.amount,
            email: router_data
                .request
                .email
                .clone()
                .map(|e| e.expose().expose()),
            browser_info,
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
            request_ref_id: Some(Identifier {
                id_type: Some(payments_grpc::identifier::IdType::Id(
                    router_data.connector_request_reference_id.clone(),
                )),
            }),
            connector_customer_id: router_data
                .request
                .customer_id
                .as_ref()
                .map(|id| id.get_string_repr().to_string()),
            metadata: router_data
                .request
                .metadata
                .as_ref()
                .and_then(|val| val.as_object())
                .map(|map| {
                    map.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect::<HashMap<String, String>>()
                })
                .unwrap_or_default(),
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

impl ForeignTryFrom<common_enums::CardNetwork> for payments_grpc::CardNetwork {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(card_network: common_enums::CardNetwork) -> Result<Self, Self::Error> {
        match card_network {
            common_enums::CardNetwork::Visa => Ok(Self::Visa),
            common_enums::CardNetwork::Mastercard => Ok(Self::Mastercard),
            common_enums::CardNetwork::JCB => Ok(Self::Jcb),
            common_enums::CardNetwork::DinersClub => Ok(Self::Diners),
            common_enums::CardNetwork::Discover => Ok(Self::Discover),
            common_enums::CardNetwork::CartesBancaires => Ok(Self::CartesBancaires),
            common_enums::CardNetwork::UnionPay => Ok(Self::Unionpay),
            common_enums::CardNetwork::RuPay => Ok(Self::Rupay),
            common_enums::CardNetwork::Maestro => Ok(Self::Maestro),
            _ => Err(
                UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                    "Card Network not supported".to_string(),
                )
                .into(),
            ),
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
        let shipping = payment_address.get_shipping().map(|address| {
            let details = address.address.as_ref();

            let get_str =
                |opt: &Option<masking::Secret<String>>| opt.as_ref().map(|s| s.peek().to_owned());

            let get_plain = |opt: &Option<String>| opt.clone();

            let country = details.and_then(|details| {
                details
                    .country
                    .as_ref()
                    .and_then(|c| payments_grpc::CountryAlpha2::from_str_name(&c.to_string()))
                    .map(|country| country.into())
            });

            payments_grpc::Address {
                first_name: get_str(&details.and_then(|d| d.first_name.clone())),
                last_name: get_str(&details.and_then(|d| d.last_name.clone())),
                line1: get_str(&details.and_then(|d| d.line1.clone())),
                line2: get_str(&details.and_then(|d| d.line2.clone())),
                line3: get_str(&details.and_then(|d| d.line3.clone())),
                city: get_plain(&details.and_then(|d| d.city.clone())),
                state: get_str(&details.and_then(|d| d.state.clone())),
                zip_code: get_str(&details.and_then(|d| d.zip.clone())),
                country_alpha2_code: country,
                email: address.email.as_ref().map(|e| e.peek().to_string()),
                phone_number: address
                    .phone
                    .as_ref()
                    .and_then(|phone| phone.number.as_ref().map(|n| n.peek().to_string())),
                phone_country_code: address.phone.as_ref().and_then(|p| p.country_code.clone()),
            }
        });

        let billing = payment_address.get_payment_billing().map(|address| {
            let details = address.address.as_ref();

            let get_str =
                |opt: &Option<masking::Secret<String>>| opt.as_ref().map(|s| s.peek().to_owned());

            let get_plain = |opt: &Option<String>| opt.clone();

            let country = details.and_then(|details| {
                details
                    .country
                    .as_ref()
                    .and_then(|c| payments_grpc::CountryAlpha2::from_str_name(&c.to_string()))
                    .map(|country| country.into())
            });

            payments_grpc::Address {
                first_name: get_str(&details.and_then(|d| d.first_name.clone())),
                last_name: get_str(&details.and_then(|d| d.last_name.clone())),
                line1: get_str(&details.and_then(|d| d.line1.clone())),
                line2: get_str(&details.and_then(|d| d.line2.clone())),
                line3: get_str(&details.and_then(|d| d.line3.clone())),
                city: get_plain(&details.and_then(|d| d.city.clone())),
                state: get_str(&details.and_then(|d| d.state.clone())),
                zip_code: get_str(&details.and_then(|d| d.zip.clone())),
                country_alpha2_code: country,
                email: address.email.as_ref().map(|e| e.peek().to_string()),
                phone_number: address
                    .phone
                    .as_ref()
                    .and_then(|phone| phone.number.as_ref().map(|n| n.peek().to_string())),
                phone_country_code: address.phone.as_ref().and_then(|p| p.country_code.clone()),
            }
        });

        let unified_payment_method_billing =
            payment_address.get_payment_method_billing().map(|address| {
                let details = address.address.as_ref();

                let get_str = |opt: &Option<masking::Secret<String>>| {
                    opt.as_ref().map(|s| s.peek().to_owned())
                };

                let get_plain = |opt: &Option<String>| opt.clone();

                let country = details.and_then(|details| {
                    details
                        .country
                        .as_ref()
                        .and_then(|c| payments_grpc::CountryAlpha2::from_str_name(&c.to_string()))
                        .map(|country| country.into())
                });

                payments_grpc::Address {
                    first_name: get_str(&details.and_then(|d| d.first_name.clone())),
                    last_name: get_str(&details.and_then(|d| d.last_name.clone())),
                    line1: get_str(&details.and_then(|d| d.line1.clone())),
                    line2: get_str(&details.and_then(|d| d.line2.clone())),
                    line3: get_str(&details.and_then(|d| d.line3.clone())),
                    city: get_plain(&details.and_then(|d| d.city.clone())),
                    state: get_str(&details.and_then(|d| d.state.clone())),
                    zip_code: get_str(&details.and_then(|d| d.zip.clone())),
                    country_alpha2_code: country,
                    email: address.email.as_ref().map(|e| e.peek().to_string()),
                    phone_number: address
                        .phone
                        .as_ref()
                        .and_then(|phone| phone.number.as_ref().map(|n| n.peek().to_string())),
                    phone_country_code: address.phone.as_ref().and_then(|p| p.country_code.clone()),
                }
            });
        Ok(Self {
            shipping_address: shipping.or(unified_payment_method_billing.clone()),
            billing_address: billing.or(unified_payment_method_billing),
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
            ip_address: browser_info.ip_address.map(|ip| ip.to_string()),
            accept_header: browser_info.accept_header,
            user_agent: browser_info.user_agent,
            os_type: browser_info.os_type,
            os_version: browser_info.os_version,
            device_model: browser_info.device_model,
            accept_language: browser_info.accept_language,
            time_zone_offset_minutes: browser_info.time_zone,
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
                |id| Identifier {
                    id_type: Some(payments_grpc::identifier::IdType::Id(id)),
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
            payments_grpc::PaymentStatus::AttemptStatusUnspecified => Ok(Self::Unresolved),
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
            Some(payments_grpc::redirect_form::FormType::Uri(_)) => Err(
                UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                    "URI form type is not implemented".to_string(),
                )
                .into(),
            ),
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
            payments_grpc::HttpMethod::Unspecified => {
                Err(UnifiedConnectorServiceError::ResponseDeserializationFailed)
                    .attach_printable("Invalid Http Method")
            }
        }
    }
}
