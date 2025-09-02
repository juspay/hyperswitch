use api_models::{customers::CustomerRequest, payments::CustomerDetailsResponse};
use common_utils::{events::ApiEventMetric, id_type::GenerateId, pii};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    api::ApplicationResponse, merchant_context::MerchantContext,
    router_request_types::CustomerDetails,
};

use crate::{
    core::customers::create_customer,
    db::{errors, StorageInterface},
    routes::SessionState,
    types::{api::CustomerResponse, transformers::ForeignInto},
};

pub async fn get_or_create_customer(
    state: SessionState,
    customer_request: Option<CustomerRequest>,
    merchant_context: MerchantContext,
) -> errors::CustomerResponse<CustomerResponse> {
    let db: &dyn StorageInterface = &*state.store;

    // Create customer_id if not passed in request
    let customer_id = customer_request
        .as_ref()
        .and_then(|c| c.customer_id.clone())
        .unwrap_or_else(common_utils::id_type::CustomerId::generate);

    let merchant_id = merchant_context.get_merchant_account().get_id();
    let key_manager_state = &(&state).into();

    match db
        .find_customer_optional_by_customer_id_merchant_id(
            key_manager_state,
            &customer_id,
            merchant_id,
            merchant_context.get_merchant_key_store(),
            merchant_context.get_merchant_account().storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("subscription: unable to perform db read query")
        .unwrap()
    {
        // Customer found
        Some(customer) => {
            let api_customer: CustomerResponse =
                (customer, None::<api_models::payments::AddressDetails>).foreign_into();
            Ok(ApplicationResponse::Json(api_customer))
        }

        // Customer not found
        None => Ok(create_customer(
            state,
            merchant_context,
            customer_request.ok_or(errors::CustomersErrorResponse::CustomerNotFound)?,
            None,
        )
        .await?),
    }
}

pub fn get_customer_details_from_request(request: CreateSubscriptionRequest) -> CustomerDetails {
    let customer_id = request.get_customer_id().map(ToOwned::to_owned);
    let customer = request.customer.as_ref();
    CustomerDetails {
        customer_id,
        name: customer.and_then(|cus| cus.name.clone()),
        email: customer.and_then(|cus| cus.email.clone()),
        phone: customer.and_then(|cus| cus.phone.clone()),
        phone_country_code: customer.and_then(|cus| cus.phone_country_code.clone()),
        tax_registration_id: customer.and_then(|cus| cus.tax_registration_id.clone()),
    }
}

pub const SUBSCRIPTION_ID_PREFIX: &str = "sub";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateSubscriptionRequest {
    pub plan_id: Option<String>,
    pub coupon_code: Option<String>,
    pub mca_id: Option<String>,
    pub confirm: bool,
    pub customer_id: Option<common_utils::id_type::CustomerId>,
    pub customer: Option<CustomerRequest>,
}

impl CreateSubscriptionRequest {
    pub fn get_customer_id(&self) -> Option<&common_utils::id_type::CustomerId> {
        self.customer_id
            .as_ref()
            .or_else(|| self.customer.as_ref()?.customer_id.as_ref())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CreateSubscriptionResponse {
    pub subscription: Subscription,
    pub client_secret: Option<String>,
    pub merchant_id: String,
    pub mca_id: Option<String>,
    pub coupon_code: Option<String>,
    pub customer: Option<CustomerDetailsResponse>,
    pub invoice: Option<Invoice>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Subscription {
    pub id: String,
    pub status: SubscriptionStatus,
    pub plan_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum SubscriptionStatus {
    Created,
    Active,
    InActive,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Invoice {
    pub id: String,
    pub total: u64,
}

impl Subscription {
    pub fn new(id: impl Into<String>, status: SubscriptionStatus, plan_id: Option<String>) -> Self {
        Self {
            id: id.into(),
            status,
            plan_id,
        }
    }
}

impl Invoice {
    pub fn new(id: impl Into<String>, total: u64) -> Self {
        Self {
            id: id.into(),
            total,
        }
    }
}
impl CreateSubscriptionResponse {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        subscription: Subscription,
        merchant_id: impl Into<String>,
        mca_id: Option<String>,
    ) -> Self {
        Self {
            subscription,
            client_secret: None,
            merchant_id: merchant_id.into(),
            mca_id,
            coupon_code: None,
            customer: None,
            invoice: None,
        }
    }
}

pub fn map_customer_resp_to_details(r: &CustomerResponse) -> CustomerDetailsResponse {
    CustomerDetailsResponse {
        id: Some(r.customer_id.clone()),
        name: r.name.as_ref().map(|n| n.clone().into_inner()),
        email: r.email.as_ref().map(|e| pii::Email::from(e.clone())),
        phone: r.phone.as_ref().map(|p| p.clone().into_inner()),
        phone_country_code: r.phone_country_code.clone(),
    }
}

impl ApiEventMetric for CreateSubscriptionResponse {}
impl ApiEventMetric for CreateSubscriptionRequest {}
