use api_models::payment_methods::CardDetailFromLocker;
use api_models::payment_methods::CustomerPaymentMethodsListResponse as ListCustomerPaymentMethodsV1Response;
use cards::CardNumber;
use common_enums::{PaymentMethod, PaymentMethodType};
use common_utils::types::MinorUnit;
use common_utils::{id_type, request::Method};
use hyperswitch_domain_models::payment_method_data::NetworkTokenDetailsPaymentMethod;
use hyperswitch_interfaces::micro_service::{MicroserviceClientError, MicroserviceClientErrorKind};
use scheduler::consumer::diesel_models::schema::payment_intent::client_secret;
use serde::Deserialize;
use time;
const DUMMY_PM_ID: &str = "pm_dummy";

/// V1-facing retrieve flow type.
#[derive(Debug)]
pub struct ListCustomerPaymentMethods;

/// V1-facing retrieve request payload.
#[derive(Debug)]
pub struct ListCustomerPaymentMethodsV1Request {
    pub customer_id: id_type::CustomerId,
    pub query_params: api_models::payment_methods::PaymentMethodListRequest,
}

/// Dummy modular service request payload.
#[derive(Clone, Debug)]
// TODO: replace dummy request types with real v1/modular models.
pub struct ListCustomerPaymentMethodsV2Request;

/// Dummy modular service response payload.
#[derive(Debug, Deserialize)]
// TODO: replace dummy response types with real v1/modular models.
pub struct ListCustomerPaymentMethodsV2Response {
    pub customer_payment_methods: Vec<PaymentMethodResponseItem>,
}
pub struct PaymentMethodResponseItem {
    /// The unique identifier of the payment method.
    pub id: String,

    /// The unique identifier of the customer.
    pub customer_id: id_type::CustomerId,

    /// The type of payment method use for the payment.
    pub payment_method_type: PaymentMethod,

    /// This is a sub-category of payment method.
    pub payment_method_subtype: PaymentMethodType,

    /// Indicates whether the payment method supports recurring payments. Optional.
    pub recurring_enabled: Option<bool>,

    /// PaymentMethod Data from locker
    pub payment_method_data: Option<PaymentMethodListData>,

    /// Masked bank details from PM auth services
    pub bank: Option<api_models::payment_methods::MaskedBankDetails>,

    /// A timestamp (ISO 8601 code) that determines when the payment method was created
    pub created: time::PrimitiveDateTime,

    /// Whether this payment method requires CVV to be collected
    pub requires_cvv: bool,

    ///  A timestamp (ISO 8601 code) that determines when the payment method was last used
    pub last_used_at: time::PrimitiveDateTime,

    /// Indicates if the payment method has been set to default or not
    pub is_default: bool,

    /// The billing details of the payment method
    pub billing: Option<api_models::payments::Address>,

    ///The network token details for the payment method
    pub network_tokenization: Option<NetworkTokenResponse>,

    /// Whether psp_tokenization is enabled for the payment_method, this will be true when at least
    /// one multi-use token with status `Active` is available for the payment method
    pub psp_tokenization_enabled: bool,
}
/// V2 PaymentMethodListData enum
#[derive(Clone, Debug, Deserialize)]
pub enum PaymentMethodListData {
    Card(CardDetailFromLockerV2),
}
/// V2 CardDetailFromLocker for deserialization
#[derive(Clone, Debug, Deserialize)]
pub struct CardDetailFromLockerV2 {
    ///Country code of the card issuer
    pub issuer_country: Option<common_enums::CountryAlpha2>,

    ///Last 4 digits of the card number
    pub last4_digits: Option<String>,

    #[serde(skip)]
    /// Full card number (masked)
    pub card_number: Option<CardNumber>,

    /// Expiry month of the card
    pub expiry_month: Option<masking::Secret<String>>,

    /// Expiry year of the card
    pub expiry_year: Option<masking::Secret<String>>,

    /// Card holder name
    pub card_holder_name: Option<masking::Secret<String>>,

    /// Card fingerprint
    pub card_fingerprint: Option<masking::Secret<String>>,

    /// Nickname for the card
    pub nick_name: Option<masking::Secret<String>>,

    /// Card network
    pub card_network: Option<common_enums::CardNetwork>,

    /// Card ISIN
    pub card_isin: Option<String>,

    /// Card issuer
    pub card_issuer: Option<String>,

    /// Card type
    pub card_type: Option<String>,

    /// Indicates if the card is saved to locker
    pub saved_to_locker: bool,
}
/// V2 NetworkTokenResponse (for deserialization, ignored in transformation)
#[derive(Clone, Debug, Deserialize)]
pub struct NetworkTokenResponse {
    /// The payment method details related to the network token
    pub payment_method_data: NetworkTokenDetailsPaymentMethod,
}
impl TryFrom<&ListCustomerPaymentMethodsV1Request> for ListCustomerPaymentMethodsV2Request {
    type Error = MicroserviceClientError;

    fn try_from(value: &ListCustomerPaymentMethodsV1Request) -> Result<Self, Self::Error> {
        Ok(Self)
    }
}

impl TryFrom<ListCustomerPaymentMethodsV2Response> for ListCustomerPaymentMethodsV1Response {
    type Error = MicroserviceClientError;

    fn try_from(v2_response: ListCustomerPaymentMethodsV2Response) -> Result<Self, Self::Error> {
        let customer_payment_methods = v2_response
            .customer_payment_methods
            .into_iter()
            .map(|pm| api_models::payment_methods::CustomerPaymentMethod {
                payment_token: DUMMY_PM_ID.to_string(),
                payment_method_id: pm.id,
                customer_id: pm
                    .customer_id
                    .map(|id| id_type::CustomerId::try_from(std::borrow::Cow::from(id)))
                    .transpose()
                    .map_err(|e| MicroserviceClientError {
                        operation: "convert_global_customer_id".to_string(),
                        kind: MicroserviceClientErrorKind::Deserialize(format!(
                            "Failed to convert customer ID: {}",
                            e
                        )),
                    })?,
                payment_method: common_enums::PaymentMethod::from(pm.payment_method_type),
                payment_method_type: Some(common_enums::PaymentMethodType::from(
                    pm.payment_method_subtype,
                )),
                payment_method_issuer: None,
                payment_method_issuer_code: None,
                recurring_enabled: pm.recurring_enabled,
                installment_payment_enabled: None,
                payment_experience: None,
                card: pm.payment_method_data.map(|pmd| match pmd {
                    PaymentMethodListData::Card(v2_card) => CardDetailFromLocker {
                        scheme: None,
                        issuer_country: v2_card.issuer_country.map(|c| c.to_string()),
                        issuer_country_code: None,
                        last4_digits: v2_card.last4_digits,
                        card_number: v2_card.card_number,
                        expiry_month: v2_card.expiry_month,
                        expiry_year: v2_card.expiry_year,
                        card_token: None,
                        card_holder_name: v2_card.card_holder_name,
                        card_fingerprint: v2_card.card_fingerprint,
                        nick_name: v2_card.nick_name,
                        card_network: v2_card.card_network,
                        card_isin: v2_card.card_isin,
                        card_issuer: v2_card.card_issuer,
                        card_type: v2_card.card_type,
                        saved_to_locker: v2_card.saved_to_locker,
                    },
                }),
                metadata: None,
                created: Some(pm.created),
                bank_transfer: None,
                bank: pm.bank,
                surcharge_details: None,
                requires_cvv: pm.requires_cvv,
                last_used_at: Some(pm.last_used_at),
                default_payment_method_set: pm.is_default,
                billing: pm.billing,
            })
            .collect();

        Ok(Self {
            customer_payment_methods,
            is_guest_customer: None,
        })
    }
}

impl ListCustomerPaymentMethods {
    fn validate_request(
        &self,
        request: &ListCustomerPaymentMethodsV1Request,
    ) -> Result<(), MicroserviceClientError> {
        if request.customer_id.get_string_repr().trim().is_empty() {
            return Err(MicroserviceClientError {
                operation: std::any::type_name::<Self>().to_string(),
                kind: MicroserviceClientErrorKind::InvalidRequest(
                    "Customer ID cannot be empty".to_string(),
                ),
            });
        }
        Ok(())
    }

    fn build_path_params(
        &self,
        request: &ListCustomerPaymentMethodsV1Request,
    ) -> Vec<(&'static str, String)> {
        vec![(
            "customer_id",
            request.customer_id.get_string_repr().to_string(),
        )]
    }

    fn query_params(
        &self,
        request: &ListCustomerPaymentMethodsV1Request,
    ) -> Vec<(&'static str, String)> {
        let mut params = Vec::new();

        let qp = &request.query_params;

        if let Some(secret) = &qp.client_secret {
            params.push(("client_secret", secret.clone()));
        }

        if let Some(amount) = qp.amount {
            params.push(("amount", amount.to_string()));
        }

        if let Some(recurring) = qp.recurring_enabled {
            params.push(("recurring_enabled", recurring.to_string()));
        }

        if let Some(limit) = qp.limit {
            params.push(("limit", limit.to_string()));
        }

        if let Some(countries) = &qp.accepted_countries {
            params.push((
                "accepted_countries",
                serde_json::to_string(countries).unwrap(),
            ));
        }

        if let Some(currencies) = &qp.accepted_currencies {
            params.push((
                "accepted_currencies",
                serde_json::to_string(currencies).unwrap(),
            ));
        }

        if let Some(networks) = &qp.card_networks {
            params.push(("card_networks", serde_json::to_string(networks).unwrap()));
        }

        params
    }
}

hyperswitch_interfaces::impl_microservice_flow!(
    ListCustomerPaymentMethods,
    method = Method::Get,
    path = "/{customer_id}/payment_methods",
    v1_request = ListCustomerPaymentMethodsV1Request,
    v2_request = ListCustomerPaymentMethodsV2Request,
    v2_response = ListCustomerPaymentMethodsV2Response,
    v1_response = ListCustomerPaymentMethodsV1Response,
    client = crate::client::PaymentMethodClient<'_>,
    path_params = ListCustomerPaymentMethods::build_path_params,
    query_params = ListCustomerPaymentMethods::query_params,
    validate = ListCustomerPaymentMethods::validate_request
);
