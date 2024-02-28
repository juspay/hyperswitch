use common_utils::pii::Email;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    core::errors,
    services,
    types::{self, api, storage::enums},
};

pub struct StancerRouterData<T> {
    pub amount: i32,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for StancerRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (_currency_unit, _currency, amount, item): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: amount
                .try_into()
                .map_err(|_| errors::ConnectorError::ParsingFailed)?,
            router_data: item,
        })
    }
}

pub mod card {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct Card {
        #[serde(rename = "id")]
        pub id: String,
        #[serde(rename = "last4")]
        pub last4: String,
        #[serde(rename = "brand")]
        pub brand: String,
        #[serde(rename = "exp_month")]
        pub exp_month: i32,
        #[serde(rename = "exp_year")]
        pub exp_year: i32,
        #[serde(rename = "created")]
        pub created: i32,
        #[serde(rename = "name", skip_serializing_if = "Option::is_none")]
        pub name: Option<String>,
        #[serde(rename = "funding", skip_serializing_if = "Option::is_none")]
        pub funding: Option<Funding>,
        #[serde(rename = "nature", skip_serializing_if = "Option::is_none")]
        pub nature: Option<Nature>,
        #[serde(rename = "network", skip_serializing_if = "Option::is_none")]
        pub network: Option<Network>,
        #[serde(rename = "zip_code", skip_serializing_if = "Option::is_none")]
        pub zip_code: Option<String>,
        #[serde(rename = "country", skip_serializing_if = "Option::is_none")]
        pub country: Option<String>,
    }
    impl Card {
        pub fn new(
            id: String,
            last4: String,
            brand: String,
            exp_month: i32,
            exp_year: i32,
            created: i32,
        ) -> Self {
            Self {
                id,
                last4,
                brand,
                exp_month,
                exp_year,
                created,
                name: None,
                funding: None,
                nature: None,
                network: None,
                zip_code: None,
                country: None,
            }
        }
    }
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
    pub enum Funding {
        #[serde(rename = "credit")]
        Credit,
        #[serde(rename = "debit")]
        Debit,
        #[serde(rename = "prepaid")]
        Prepaid,
        #[serde(rename = "universal")]
        Universal,
        #[serde(rename = "charge")]
        Charge,
        #[serde(rename = "deferred")]
        Deferred,
    }
    impl Default for Funding {
        fn default() -> Self {
            Self::Credit
        }
    }
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
    pub enum Nature {
        #[serde(rename = "personal")]
        Personal,
        #[serde(rename = "personnal")]
        Personnal,
        #[serde(rename = "corporate")]
        Corporate,
    }
    impl Default for Nature {
        fn default() -> Self {
            Self::Personal
        }
    }
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
    pub enum Network {
        #[serde(rename = "national")]
        National,
        #[serde(rename = "mastercard")]
        Mastercard,
        #[serde(rename = "visa")]
        Visa,
    }
    impl Default for Network {
        fn default() -> Self {
            Self::National
        }
    }
}

pub use self::create_payment_request_auth::CreatePaymentRequestAuth;
pub mod create_payment_request_card {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct CreatePaymentRequestCard {
        #[serde(rename = "number")]
        pub number: cards::CardNumber,
        #[serde(rename = "cvc")]
        pub cvc: Secret<String>,
        #[serde(rename = "exp_year")]
        pub exp_year: Secret<String>,
        #[serde(rename = "exp_month")]
        pub exp_month: Secret<String>,
    }
    impl CreatePaymentRequestCard {
        pub fn new(
            number: cards::CardNumber,
            cvc: Secret<String>,
            exp_year: Secret<String>,
            exp_month: Secret<String>,
        ) -> Self {
            Self {
                number,
                cvc,
                exp_year,
                exp_month,
            }
        }
    }
}

pub use self::create_payment_request_card::CreatePaymentRequestCard;
pub mod create_payment_request_device {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct CreatePaymentRequestDevice {
        #[serde(rename = "ip")]
        pub ip: String,
        #[serde(rename = "port", skip_serializing_if = "Option::is_none")]
        pub port: Option<i32>,
        #[serde(rename = "user_agent", skip_serializing_if = "Option::is_none")]
        pub user_agent: Option<String>,
        #[serde(rename = "http_accept", skip_serializing_if = "Option::is_none")]
        pub http_accept: Option<String>,
        #[serde(rename = "languages", skip_serializing_if = "Option::is_none")]
        pub languages: Option<String>,
    }
    impl CreatePaymentRequestDevice {
        pub fn new(ip: String) -> Self {
            Self {
                ip,
                port: None,
                user_agent: None,
                http_accept: None,
                languages: None,
            }
        }
    }
}

pub use self::customer::Customer;
pub mod dispute {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct Dispute {
        #[serde(rename = "id")]
        pub id: String,
        #[serde(rename = "amount")]
        pub amount: i32,
        #[serde(rename = "created")]
        pub created: i32,
        #[serde(rename = "date_bank")]
        pub date_bank: i32,
        #[serde(rename = "order_id", skip_serializing_if = "Option::is_none")]
        pub order_id: Option<String>,
        #[serde(rename = "payment")]
        pub payment: String,
        #[serde(rename = "response", skip_serializing_if = "Option::is_none")]
        pub response: Option<String>,
        #[serde(rename = "type")]
        pub r#type: String,
    }
    impl Dispute {
        pub fn new(
            id: String,
            amount: i32,
            created: i32,
            date_bank: i32,
            payment: String,
            r#type: String,
        ) -> Self {
            Self {
                id,
                amount,
                created,
                date_bank,
                order_id: None,
                payment,
                response: None,
                r#type,
            }
        }
    }
}

pub use self::dispute::Dispute;
pub mod get_disputes_200_response {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct GetDisputes200Response {
        #[serde(rename = "disputes")]
        pub disputes: Vec<Dispute>,
        #[serde(rename = "range")]
        pub range: Box<GetDisputes200ResponseRange>,
    }
    impl GetDisputes200Response {
        pub fn new(disputes: Vec<Dispute>, range: GetDisputes200ResponseRange) -> Self {
            Self {
                disputes,
                range: Box::new(range),
            }
        }
    }
}

pub use self::get_disputes_200_response::GetDisputes200Response;
pub mod get_disputes_200_response_range {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct GetDisputes200ResponseRange {
        #[serde(rename = "created")]
        pub created: i32,
        #[serde(rename = "start")]
        pub start: i32,
        #[serde(rename = "end")]
        pub end: i32,
        #[serde(rename = "limit")]
        pub limit: i32,
        #[serde(rename = "has_more")]
        pub has_more: bool,
        #[serde(rename = "order_id", skip_serializing_if = "Option::is_none")]
        pub order_id: Option<String>,
        #[serde(rename = "unique_id", skip_serializing_if = "Option::is_none")]
        pub unique_id: Option<String>,
    }
    impl GetDisputes200ResponseRange {
        pub fn new(created: i32, start: i32, end: i32, limit: i32, has_more: bool) -> Self {
            Self {
                created,
                start,
                end,
                limit,
                has_more,
                order_id: None,
                unique_id: None,
            }
        }
    }
}

pub use self::get_disputes_200_response_range::GetDisputes200ResponseRange;
pub mod get_disputes_404_response {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct GetDisputes404Response {
        #[serde(rename = "error")]
        pub error: Box<GetDisputes404ResponseError>,
    }
    impl GetDisputes404Response {
        pub fn new(error: GetDisputes404ResponseError) -> Self {
            Self {
                error: Box::new(error),
            }
        }
    }
}

pub use self::get_disputes_404_response::GetDisputes404Response;
pub mod get_disputes_404_response_error {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct GetDisputes404ResponseError {
        #[serde(rename = "message")]
        pub message: Box<GetDisputes404ResponseErrorMessage>,
        #[serde(rename = "type")]
        pub r#type: String,
    }
    impl GetDisputes404ResponseError {
        pub fn new(message: GetDisputes404ResponseErrorMessage, r#type: String) -> Self {
            Self {
                message: Box::new(message),
                r#type,
            }
        }
    }
}
pub use self::get_disputes_404_response_error::GetDisputes404ResponseError;
pub mod get_disputes_404_response_error_message {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct GetDisputes404ResponseErrorMessage {
        #[serde(rename = "id")]
        pub id: String,
    }
    impl GetDisputes404ResponseErrorMessage {
        pub fn new(id: String) -> Self {
            Self { id }
        }
    }
}

pub use self::get_disputes_404_response_error_message::GetDisputes404ResponseErrorMessage;
pub mod list_payments_200_response {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct ListPayments200Response {
        #[serde(rename = "live_mode")]
        pub live_mode: bool,
        #[serde(rename = "payments")]
        pub payments: Vec<Payment>,
        #[serde(rename = "range")]
        pub range: Box<ListPayments200ResponseRange>,
    }
    impl ListPayments200Response {
        pub fn new(
            live_mode: bool,
            payments: Vec<Payment>,
            range: ListPayments200ResponseRange,
        ) -> Self {
            Self {
                live_mode,
                payments,
                range: Box::new(range),
            }
        }
    }
}
pub use self::list_payments_200_response::ListPayments200Response;
pub mod list_payments_200_response_range {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct ListPayments200ResponseRange {
        #[serde(rename = "created")]
        pub created: i32,
        #[serde(rename = "start")]
        pub start: i32,
        #[serde(rename = "end")]
        pub end: i32,
        #[serde(rename = "has_more")]
        pub has_more: bool,
        #[serde(rename = "limit")]
        pub limit: i32,
        #[serde(rename = "order_id", skip_serializing_if = "Option::is_none")]
        pub order_id: Option<String>,
        #[serde(rename = "unique_id", skip_serializing_if = "Option::is_none")]
        pub unique_id: Option<String>,
    }
    impl ListPayments200ResponseRange {
        pub fn new(created: i32, start: i32, end: i32, has_more: bool, limit: i32) -> Self {
            Self {
                created,
                start,
                end,
                has_more,
                limit,
                order_id: None,
                unique_id: None,
            }
        }
    }
}

pub use self::list_payments_200_response_range::ListPayments200ResponseRange;
pub mod payment {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct Payment {
        #[serde(rename = "id")]
        pub id: String,
        #[serde(rename = "amount")]
        pub amount: i32,
        #[serde(rename = "currency")]
        pub currency: String,
        #[serde(rename = "created")]
        pub created: i32,
        #[serde(rename = "fee", skip_serializing_if = "Option::is_none")]
        pub fee: Option<i32>,
        #[serde(rename = "description", skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[serde(rename = "order_id", skip_serializing_if = "Option::is_none")]
        pub order_id: Option<String>,
        #[serde(rename = "unique_id", skip_serializing_if = "Option::is_none")]
        pub unique_id: Option<String>,
        #[serde(rename = "method", skip_serializing_if = "Option::is_none")]
        pub method: Option<Method>,
        #[serde(rename = "sepa", skip_serializing_if = "Option::is_none")]
        pub sepa: Option<Box<Sepa>>,
        #[serde(rename = "card", skip_serializing_if = "Option::is_none")]
        pub card: Option<Box<Card>>,
        #[serde(rename = "status", skip_serializing_if = "Option::is_none")]
        pub status: Option<Status>,
        #[serde(rename = "response", skip_serializing_if = "Option::is_none")]
        pub response: Option<String>,
        #[serde(rename = "methods_allowed", skip_serializing_if = "Option::is_none")]
        pub methods_allowed: Option<Vec<MethodsAllowed>>,
        #[serde(rename = "return_url", skip_serializing_if = "Option::is_none")]
        pub return_url: Option<String>,
        #[serde(rename = "auth", skip_serializing_if = "Option::is_none")]
        pub auth: Option<Box<PaymentAuth>>,
        #[serde(rename = "device", skip_serializing_if = "Option::is_none")]
        pub device: Option<Box<PaymentDevice>>,
        #[serde(rename = "customer", skip_serializing_if = "Option::is_none")]
        pub customer: Option<Box<Customer>>,
    }
    impl Payment {
        pub fn new(id: String, amount: i32, currency: String, created: i32) -> Self {
            Self {
                id,
                amount,
                currency,
                created,
                fee: None,
                description: None,
                order_id: None,
                unique_id: None,
                method: None,
                sepa: None,
                card: None,
                status: None,
                response: None,
                methods_allowed: None,
                return_url: None,
                auth: None,
                device: None,
                customer: None,
            }
        }
    }
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
    pub enum Method {
        #[serde(rename = "sepa")]
        Sepa,
        #[serde(rename = "card")]
        Card,
    }
    impl Default for Method {
        fn default() -> Self {
            Self::Sepa
        }
    }
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
    pub enum Status {
        #[serde(rename = "authorized")]
        Authorized,
        #[serde(rename = "canceled")]
        Canceled,
        #[serde(rename = "captured")]
        Captured,
        #[serde(rename = "capture_sent")]
        CaptureSent,
        #[serde(rename = "disputed")]
        Disputed,
        #[serde(rename = "expired")]
        Expired,
        #[serde(rename = "failed")]
        Failed,
        #[serde(rename = "refused")]
        Refused,
        #[serde(rename = "to_capture")]
        ToCapture,
    }
    impl Default for Status {
        fn default() -> Self {
            Self::Authorized
        }
    }
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
    pub enum MethodsAllowed {
        #[serde(rename = "card")]
        Card,
        #[serde(rename = "sepa")]
        Sepa,
    }
    impl Default for MethodsAllowed {
        fn default() -> Self {
            Self::Card
        }
    }
}

pub use self::payment::Payment;
pub mod payment_auth {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct PaymentAuth {
        #[serde(rename = "redirect_url")]
        pub redirect_url: String,
        #[serde(rename = "return_url")]
        pub return_url: String,
        #[serde(rename = "status")]
        pub status: Status,
    }
    impl PaymentAuth {
        pub fn new(redirect_url: String, return_url: String, status: Status) -> Self {
            Self {
                redirect_url,
                return_url,
                status,
            }
        }
    }
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
    pub enum Status {
        #[serde(rename = "attempted")]
        Attempted,
        #[serde(rename = "available")]
        Available,
        #[serde(rename = "declined")]
        Declined,
        #[serde(rename = "expired")]
        Expired,
        #[serde(rename = "failed")]
        Failed,
        #[serde(rename = "requested")]
        Requested,
        #[serde(rename = "success")]
        Success,
        #[serde(rename = "unavailable")]
        Unavailable,
    }
    impl Default for Status {
        fn default() -> Self {
            Self::Attempted
        }
    }
}

pub use self::payment_auth::PaymentAuth;
pub mod payment_device {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct PaymentDevice {
        #[serde(rename = "ip")]
        pub ip: String,
        #[serde(rename = "port", skip_serializing_if = "Option::is_none")]
        pub port: Option<i32>,
        #[serde(rename = "user_agent", skip_serializing_if = "Option::is_none")]
        pub user_agent: Option<String>,
        #[serde(rename = "http_accept", skip_serializing_if = "Option::is_none")]
        pub http_accept: Option<String>,
        #[serde(rename = "languages", skip_serializing_if = "Option::is_none")]
        pub languages: Option<String>,
        #[serde(rename = "city", skip_serializing_if = "Option::is_none")]
        pub city: Option<String>,
        #[serde(rename = "country", skip_serializing_if = "Option::is_none")]
        pub country: Option<String>,
    }
    impl PaymentDevice {
        pub fn new(ip: String) -> Self {
            Self {
                ip,
                port: None,
                user_agent: None,
                http_accept: None,
                languages: None,
                city: None,
                country: None,
            }
        }
    }
}

pub use self::payment_device::PaymentDevice;
pub mod refund {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct Refund {
        #[serde(rename = "id")]
        pub id: String,
        #[serde(rename = "payment")]
        pub payment: String,
        #[serde(rename = "amount")]
        pub amount: i32,
        #[serde(rename = "currency")]
        pub currency: String,
        #[serde(rename = "status")]
        pub status: Status,
        #[serde(rename = "created")]
        pub created: i32,
        #[serde(rename = "date_refund", skip_serializing_if = "Option::is_none")]
        pub date_refund: Option<i32>,
        #[serde(rename = "date_bank", skip_serializing_if = "Option::is_none")]
        pub date_bank: Option<i32>,
        #[serde(rename = "live_mode", skip_serializing_if = "Option::is_none")]
        pub live_mode: Option<bool>,
    }
    impl Refund {
        pub fn new(
            id: String,
            payment: String,
            amount: i32,
            currency: String,
            status: Status,
            created: i32,
        ) -> Self {
            Self {
                id,
                payment,
                amount,
                currency,
                status,
                created,
                date_refund: None,
                date_bank: None,
                live_mode: None,
            }
        }
    }
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
    pub enum Status {
        #[serde(rename = "failed")]
        Failed,
        #[serde(rename = "not_honored")]
        NotHonored,
        #[serde(rename = "refund_sent")]
        RefundSent,
        #[serde(rename = "refunded")]
        Refunded,
        #[serde(rename = "to_refund")]
        ToRefund,
    }
    impl Default for Status {
        fn default() -> Self {
            Self::Failed
        }
    }
}

pub use self::refund::Refund;
pub mod sepa {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct Sepa {
        #[serde(rename = "id")]
        pub id: String,
        #[serde(rename = "name")]
        pub name: String,
        #[serde(rename = "created")]
        pub created: i32,
        #[serde(rename = "last4")]
        pub last4: String,
        #[serde(rename = "bic", skip_serializing_if = "Option::is_none")]
        pub bic: Option<String>,
        #[serde(rename = "live_mode", skip_serializing_if = "Option::is_none")]
        pub live_mode: Option<bool>,
        #[serde(rename = "mandate", skip_serializing_if = "Option::is_none")]
        pub mandate: Option<String>,
        #[serde(rename = "date_mandate", skip_serializing_if = "Option::is_none")]
        pub date_mandate: Option<i32>,
    }
    impl Sepa {
        pub fn new(id: String, name: String, created: i32, last4: String) -> Self {
            Self {
                id,
                name,
                created,
                last4,
                bic: None,
                live_mode: None,
                mandate: None,
                date_mandate: None,
            }
        }
    }
}

pub use self::sepa::Sepa;
pub mod update_payment_request {
    use super::*;
    #[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
    pub struct UpdatePaymentRequest {
        #[serde(rename = "amount", skip_serializing_if = "Option::is_none")]
        pub amount: Option<i32>,
        #[serde(rename = "currency", skip_serializing_if = "Option::is_none")]
        pub currency: Option<String>,
        #[serde(rename = "order_id", skip_serializing_if = "Option::is_none")]
        pub order_id: Option<String>,
        #[serde(rename = "description", skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[serde(rename = "sepa", skip_serializing_if = "Option::is_none")]
        pub sepa: Option<String>,
        #[serde(rename = "card", skip_serializing_if = "Option::is_none")]
        pub card: Option<String>,
        #[serde(rename = "status", skip_serializing_if = "Option::is_none")]
        pub status: Option<Status>,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
    pub enum Status {
        #[serde(rename = "authorize")]
        Authorize,
        #[serde(rename = "capture")]
        Capture,
    }
    impl Default for Status {
        fn default() -> Self {
            Self::Authorize
        }
    }
}

pub use self::update_payment_request::UpdatePaymentRequest;
pub mod update_sepa_request {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
    pub struct UpdateSepaRequest {
        #[serde(rename = "name", skip_serializing_if = "Option::is_none")]
        pub name: Option<String>,
        #[serde(rename = "mandate", skip_serializing_if = "Option::is_none")]
        pub mandate: Option<String>,
        #[serde(rename = "date_mandate", skip_serializing_if = "Option::is_none")]
        pub date_mandate: Option<i32>,
    }
}
pub use self::update_sepa_request::UpdateSepaRequest;

// CreatePaymentRequest
impl TryFrom<&StancerRouterData<&types::PaymentsAuthorizeRouterData>> for CreatePaymentRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &StancerRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let StancerRouterData {
            amount,
            router_data,
        } = item;
        let request = Self {
            description: router_data.description.to_owned(),
            order_id: Some(router_data.connector_request_reference_id.to_owned()),
            unique_id: Some(router_data.payment_id.to_owned()),
            capture: router_data.request.capture_method.map(
                |capture_method| match capture_method {
                    common_enums::CaptureMethod::Automatic => true,
                    common_enums::CaptureMethod::Manual
                    | common_enums::CaptureMethod::ManualMultiple
                    | common_enums::CaptureMethod::Scheduled => false,
                },
            ),
            customer: router_data.connector_customer.to_owned(),
            ..Self::new(
                *amount,
                router_data.request.currency.to_string().to_lowercase(),
            )
        };
        let use_3ds = matches!(
            router_data.auth_type,
            common_enums::AuthenticationType::ThreeDs
        );

        match &router_data.request.payment_method_data {
            api::PaymentMethodData::Card(card) => Ok(Self {
                card: Some(
                    CreatePaymentRequestCard {
                        number: card.card_number.to_owned(),
                        cvc: card.card_cvc.to_owned(),
                        exp_year: card.card_exp_year.to_owned(),
                        exp_month: card.card_exp_month.to_owned(),
                    }
                    .into(),
                ),
                auth: use_3ds
                    .then(|| {
                        router_data
                            .return_url
                            .to_owned()
                            .map(|return_url| CreatePaymentRequestAuth { return_url }.into())
                    })
                    .flatten(),
                device: use_3ds
                    .then(|| {
                        router_data
                            .request
                            .browser_info
                            .as_ref()
                            .and_then(|browser_info| {
                                Some(
                                    CreatePaymentRequestDevice {
                                        ip: browser_info.ip_address.as_ref()?.to_string(),
                                        port: None,
                                        user_agent: browser_info.user_agent.to_owned(),
                                        http_accept: browser_info.accept_header.to_owned(),
                                        languages: browser_info.language.to_owned(),
                                    }
                                    .into(),
                                )
                            })
                    })
                    .flatten(),
                ..request
            }),
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

// UpdatePaymentRequest
impl TryFrom<&StancerRouterData<&types::PaymentsCaptureRouterData>> for UpdatePaymentRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &StancerRouterData<&types::PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let StancerRouterData { amount, .. } = item;

        Ok(Self {
            amount: Some(*amount),
            status: Some(update_payment_request::Status::Capture),
            ..Self::default()
        })
    }
}

// Auth Struct
pub struct StancerAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for StancerAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// Payment
impl From<payment::Status> for enums::AttemptStatus {
    fn from(value: payment::Status) -> Self {
        match value {
            payment::Status::Authorized => Self::Authorized,
            payment::Status::Canceled | payment::Status::Expired => Self::Voided,
            payment::Status::Captured => Self::Charged,
            payment::Status::ToCapture | payment::Status::CaptureSent => Self::CaptureInitiated,
            payment::Status::Refused | payment::Status::Failed => Self::Failure,
            payment::Status::Disputed => Self::AutoRefunded,
        }
    }
}

impl From<payment_auth::Status> for enums::AttemptStatus {
    fn from(value: payment_auth::Status) -> Self {
        match value {
            payment_auth::Status::Attempted
            | payment_auth::Status::Available
            | payment_auth::Status::Requested => Self::AuthenticationPending,
            payment_auth::Status::Declined
            | payment_auth::Status::Failed
            | payment_auth::Status::Unavailable => Self::AuthenticationFailed,
            payment_auth::Status::Expired => Self::Voided,
            payment_auth::Status::Success => Self::AuthenticationSuccessful,
        }
    }
}

impl<F, T> TryFrom<types::ResponseRouterData<F, Payment, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, Payment, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let types::ResponseRouterData::<_, _, _, _> { response, data, .. } = item;
        let Payment {
            status, auth, id, ..
        } = response;
        let three_ds_response = auth.as_ref().and_then(|auth| {
            matches!(auth.status, payment_auth::Status::Unavailable).then_some(
                types::PaymentsResponseData::ThreeDSEnrollmentResponse {
                    enrolled_v2: false,
                    related_transaction_id: Some(id.to_owned()),
                },
            )
        });

        Ok(Self {
            status: status
                .map(Into::into)
                .or(auth.as_ref().map(|auth| auth.status).map(Into::into))
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "status",
                })?,
            response: if let Some(three_ds_response) = three_ds_response {
                Ok(three_ds_response)
            } else {
                Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(id.to_owned()),
                    redirection_data: auth
                        .map(|auth| {
                            url::Url::parse(&auth.redirect_url)
                                .map_err(|_| errors::ConnectorError::ParsingFailed)
                        })
                        .transpose()?
                        .map(|redirect_url| {
                            services::RedirectForm::from((redirect_url, services::Method::Get))
                        }),
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(id),
                    incremental_authorization_allowed: None,
                })
            },
            ..data
        })
    }
}

pub use self::create_payment_request_device::CreatePaymentRequestDevice;
pub mod create_refund_request {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct CreateRefundRequest {
        #[serde(rename = "payment")]
        pub payment: String,
        #[serde(rename = "amount")]
        pub amount: i32,
    }
    impl CreateRefundRequest {
        pub fn new(payment: String, amount: i32) -> Self {
            Self { payment, amount }
        }
    }
}

pub use self::create_refund_request::CreateRefundRequest;
pub mod create_sepa_409_response {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct CreateSepa409Response {
        #[serde(rename = "error")]
        pub error: Box<CreateSepa409ResponseError>,
    }
    impl CreateSepa409Response {
        pub fn new(error: CreateSepa409ResponseError) -> Self {
            Self {
                error: Box::new(error),
            }
        }
    }
}

pub use self::create_sepa_409_response::CreateSepa409Response;
pub mod create_sepa_409_response_error {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct CreateSepa409ResponseError {
        #[serde(rename = "message")]
        pub message: Box<CreateSepa409ResponseErrorMessage>,
        #[serde(rename = "type")]
        pub r#type: String,
    }
    impl CreateSepa409ResponseError {
        pub fn new(message: CreateSepa409ResponseErrorMessage, r#type: String) -> Self {
            Self {
                message: Box::new(message),
                r#type,
            }
        }
    }
}

pub use self::create_sepa_409_response_error::CreateSepa409ResponseError;
pub mod create_sepa_409_response_error_message {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct CreateSepa409ResponseErrorMessage {
        #[serde(rename = "error")]
        pub error: String,
        #[serde(rename = "id")]
        pub id: String,
    }
    impl CreateSepa409ResponseErrorMessage {
        pub fn new(error: String, id: String) -> Self {
            Self { error, id }
        }
    }
}

pub use self::create_sepa_409_response_error_message::CreateSepa409ResponseErrorMessage;
pub mod create_sepa_ibanonly_request {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct CreateSepaIbanonlyRequest {
        #[serde(rename = "iban")]
        pub iban: String,
        #[serde(rename = "name")]
        pub name: String,
    }
    impl CreateSepaIbanonlyRequest {
        pub fn new(iban: String, name: String) -> Self {
            Self { iban, name }
        }
    }
}

pub use self::create_sepa_ibanonly_request::CreateSepaIbanonlyRequest;
pub mod create_sepa_request {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct CreateSepaRequest {
        #[serde(rename = "name")]
        pub name: String,
        #[serde(rename = "iban")]
        pub iban: String,
        #[serde(rename = "bic", skip_serializing_if = "Option::is_none")]
        pub bic: Option<String>,
        #[serde(rename = "mandate", skip_serializing_if = "Option::is_none")]
        pub mandate: Option<String>,
        #[serde(rename = "date_mandate", skip_serializing_if = "Option::is_none")]
        pub date_mandate: Option<i32>,
    }
    impl CreateSepaRequest {
        pub fn new(name: String, iban: String) -> Self {
            Self {
                name,
                iban,
                bic: None,
                mandate: None,
                date_mandate: None,
            }
        }
    }
}

pub use self::create_sepa_request::CreateSepaRequest;
pub mod customer {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct Customer {
        #[serde(rename = "id")]
        pub id: String,
        #[serde(rename = "name", skip_serializing_if = "Option::is_none")]
        pub name: Option<String>,
        #[serde(rename = "email", skip_serializing_if = "Option::is_none")]
        pub email: Option<String>,
        #[serde(rename = "mobile", skip_serializing_if = "Option::is_none")]
        pub mobile: Option<String>,
    }
    impl Customer {
        pub fn new(id: String) -> Self {
            Self {
                id,
                name: None,
                email: None,
                mobile: None,
            }
        }
    }
}

// CreateRefundRequest
impl<F> TryFrom<&StancerRouterData<&types::RefundsRouterData<F>>> for CreateRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &StancerRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        let StancerRouterData {
            amount,
            router_data,
        } = item;

        Ok(Self {
            amount: *amount,
            payment: router_data.request.connector_transaction_id.to_owned(),
        })
    }
}

// Refund
impl From<refund::Status> for enums::RefundStatus {
    fn from(item: refund::Status) -> Self {
        match item {
            refund::Status::Failed | refund::Status::NotHonored => Self::Failure,
            refund::Status::RefundSent | refund::Status::ToRefund => Self::Pending,
            refund::Status::Refunded => Self::Success,
        }
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, Refund>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, Refund>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, Refund>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, Refund>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

pub use self::card::Card;
pub mod create_customer_request {
    use super::*;
    #[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
    pub struct CreateCustomerRequest {
        #[serde(rename = "name", skip_serializing_if = "Option::is_none")]
        pub name: Option<String>,
        #[serde(rename = "email", skip_serializing_if = "Option::is_none")]
        pub email: Option<Email>,
        #[serde(rename = "mobile", skip_serializing_if = "Option::is_none")]
        pub mobile: Option<Secret<String>>,
    }
    impl CreateCustomerRequest {
        pub fn new() -> Self {
            Self {
                name: None,
                email: None,
                mobile: None,
            }
        }
    }
}

pub use self::create_customer_request::CreateCustomerRequest;
pub mod create_payment_request {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct CreatePaymentRequest {
        #[serde(rename = "amount")]
        pub amount: i32,
        #[serde(rename = "currency")]
        pub currency: String,
        #[serde(rename = "description", skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[serde(rename = "order_id", skip_serializing_if = "Option::is_none")]
        pub order_id: Option<String>,
        #[serde(rename = "unique_id", skip_serializing_if = "Option::is_none")]
        pub unique_id: Option<String>,
        #[serde(rename = "customer", skip_serializing_if = "Option::is_none")]
        pub customer: Option<String>,
        #[serde(rename = "sepa", skip_serializing_if = "Option::is_none")]
        pub sepa: Option<String>,
        #[serde(rename = "card", skip_serializing_if = "Option::is_none")]
        pub card: Option<Box<CreatePaymentRequestCard>>,
        #[serde(rename = "status", skip_serializing_if = "Option::is_none")]
        pub status: Option<Status>,
        #[serde(rename = "methods_allowed", skip_serializing_if = "Option::is_none")]
        pub methods_allowed: Option<Vec<MethodsAllowed>>,
        #[serde(rename = "return_url", skip_serializing_if = "Option::is_none")]
        pub return_url: Option<String>,
        #[serde(rename = "auth", skip_serializing_if = "Option::is_none")]
        pub auth: Option<Box<CreatePaymentRequestAuth>>,
        #[serde(rename = "device", skip_serializing_if = "Option::is_none")]
        pub device: Option<Box<CreatePaymentRequestDevice>>,
        #[serde(rename = "capture", skip_serializing_if = "Option::is_none")]
        pub capture: Option<bool>,
    }
    impl CreatePaymentRequest {
        pub fn new(amount: i32, currency: String) -> Self {
            Self {
                amount,
                currency,
                description: None,
                order_id: None,
                unique_id: None,
                customer: None,
                sepa: None,
                card: None,
                status: None,
                methods_allowed: None,
                return_url: None,
                auth: None,
                device: None,
                capture: None,
            }
        }
    }
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
    pub enum Status {
        #[serde(rename = "authorize")]
        Authorize,
        #[serde(rename = "capture")]
        Capture,
    }
    impl Default for Status {
        fn default() -> Self {
            Self::Authorize
        }
    }
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
    pub enum MethodsAllowed {
        #[serde(rename = "card")]
        Card,
        #[serde(rename = "sepa")]
        Sepa,
    }
    impl Default for MethodsAllowed {
        fn default() -> Self {
            Self::Card
        }
    }
}

pub use self::create_payment_request::CreatePaymentRequest;
pub mod create_payment_request_auth {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct CreatePaymentRequestAuth {
        #[serde(rename = "return_url")]
        pub return_url: String,
    }
    impl CreatePaymentRequestAuth {
        pub fn new(return_url: String) -> Self {
            Self { return_url }
        }
    }
}

// CreateCustomerRequest
impl TryFrom<&types::ConnectorCustomerRouterData> for CreateCustomerRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::ConnectorCustomerRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            name: item.request.name.to_owned(),
            email: item.request.email.to_owned(),
            mobile: item.request.phone.to_owned(),
        })
    }
}

// Customer
impl<F, T> TryFrom<types::ResponseRouterData<F, Customer, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, Customer, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::PaymentsResponseData::ConnectorCustomerResponse {
                connector_customer_id: item.response.id,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StancerErrorResponse {
    Error {
        message: serde_json::Value,
        #[serde(rename = "type")]
        error_type: String,
    },
}
