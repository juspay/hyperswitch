use common_utils::pii::Email;
use masking::Secret;
use serde::{Deserialize, Serialize};

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
        ) -> Card {
            Card {
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
        fn default() -> Funding {
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
        fn default() -> Nature {
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
        fn default() -> Network {
            Self::National
        }
    }
}
pub use self::card::Card;
pub mod create_customer_request {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct CreateCustomerRequest {
        #[serde(rename = "name", skip_serializing_if = "Option::is_none")]
        pub name: Option<String>,
        #[serde(rename = "email", skip_serializing_if = "Option::is_none")]
        pub email: Option<Email>,
        #[serde(rename = "mobile", skip_serializing_if = "Option::is_none")]
        pub mobile: Option<Secret<String>>,
    }
    impl CreateCustomerRequest {
        pub fn new() -> CreateCustomerRequest {
            CreateCustomerRequest {
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
        pub fn new(amount: i32, currency: String) -> CreatePaymentRequest {
            CreatePaymentRequest {
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
        fn default() -> Status {
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
        fn default() -> MethodsAllowed {
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
        pub fn new(return_url: String) -> CreatePaymentRequestAuth {
            CreatePaymentRequestAuth { return_url }
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
        ) -> CreatePaymentRequestCard {
            CreatePaymentRequestCard {
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
        pub fn new(ip: String) -> CreatePaymentRequestDevice {
            CreatePaymentRequestDevice {
                ip,
                port: None,
                user_agent: None,
                http_accept: None,
                languages: None,
            }
        }
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
        pub fn new(payment: String, amount: i32) -> CreateRefundRequest {
            CreateRefundRequest { payment, amount }
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
        pub fn new(error: CreateSepa409ResponseError) -> CreateSepa409Response {
            CreateSepa409Response {
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
        pub fn new(
            message: CreateSepa409ResponseErrorMessage,
            r#type: String,
        ) -> CreateSepa409ResponseError {
            CreateSepa409ResponseError {
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
        pub fn new(error: String, id: String) -> CreateSepa409ResponseErrorMessage {
            CreateSepa409ResponseErrorMessage { error, id }
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
        pub fn new(iban: String, name: String) -> CreateSepaIbanonlyRequest {
            CreateSepaIbanonlyRequest { iban, name }
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
        pub fn new(name: String, iban: String) -> CreateSepaRequest {
            CreateSepaRequest {
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
        pub fn new(id: String) -> Customer {
            Customer {
                id,
                name: None,
                email: None,
                mobile: None,
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
        ) -> Dispute {
            Dispute {
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
        pub fn new(
            disputes: Vec<Dispute>,
            range: GetDisputes200ResponseRange,
        ) -> GetDisputes200Response {
            GetDisputes200Response {
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
        pub fn new(
            created: i32,
            start: i32,
            end: i32,
            limit: i32,
            has_more: bool,
        ) -> GetDisputes200ResponseRange {
            GetDisputes200ResponseRange {
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
        pub fn new(error: GetDisputes404ResponseError) -> GetDisputes404Response {
            GetDisputes404Response {
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
        pub fn new(
            message: GetDisputes404ResponseErrorMessage,
            r#type: String,
        ) -> GetDisputes404ResponseError {
            GetDisputes404ResponseError {
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
        pub fn new(id: String) -> GetDisputes404ResponseErrorMessage {
            GetDisputes404ResponseErrorMessage { id }
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
        ) -> ListPayments200Response {
            ListPayments200Response {
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
        pub fn new(
            created: i32,
            start: i32,
            end: i32,
            has_more: bool,
            limit: i32,
        ) -> ListPayments200ResponseRange {
            ListPayments200ResponseRange {
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
        pub fn new(id: String, amount: i32, currency: String, created: i32) -> Payment {
            Payment {
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
        fn default() -> Method {
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
        fn default() -> Status {
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
        fn default() -> MethodsAllowed {
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
        pub fn new(redirect_url: String, return_url: String, status: Status) -> PaymentAuth {
            PaymentAuth {
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
        fn default() -> Status {
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
        pub fn new(ip: String) -> PaymentDevice {
            PaymentDevice {
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
        ) -> Refund {
            Refund {
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
        fn default() -> Status {
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
        pub fn new(id: String, name: String, created: i32, last4: String) -> Sepa {
            Sepa {
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
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    impl UpdatePaymentRequest {
        pub fn new() -> UpdatePaymentRequest {
            UpdatePaymentRequest {
                amount: None,
                currency: None,
                order_id: None,
                description: None,
                sepa: None,
                card: None,
                status: None,
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
        fn default() -> Status {
            Self::Authorize
        }
    }
}
pub use self::update_payment_request::UpdatePaymentRequest;
pub mod update_sepa_request {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct UpdateSepaRequest {
        #[serde(rename = "name", skip_serializing_if = "Option::is_none")]
        pub name: Option<String>,
        #[serde(rename = "mandate", skip_serializing_if = "Option::is_none")]
        pub mandate: Option<String>,
        #[serde(rename = "date_mandate", skip_serializing_if = "Option::is_none")]
        pub date_mandate: Option<i32>,
    }
    impl UpdateSepaRequest {
        pub fn new() -> UpdateSepaRequest {
            UpdateSepaRequest {
                name: None,
                mandate: None,
                date_mandate: None,
            }
        }
    }
}
pub use self::update_sepa_request::UpdateSepaRequest;
