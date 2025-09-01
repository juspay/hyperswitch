use common_utils::{crypto::Encryptable, events::ApiEventMetric, id_type::GenerateId, pii, type_name, types::keymanager::{Identifier, ToEncryptable}};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{customer::Customer, merchant_context::MerchantContext, router_request_types::CustomerDetails, type_encryption::{crypto_operation, CryptoOperation}};
use masking::{ExposeInterface, PeekInterface, Secret, SwitchStrategy};

use crate::{db::{errors::{self, RouterResult}, StorageInterface}, routes::SessionState, types::domain as domain};

pub async fn get_or_create_customer(
    state: &SessionState,
    customer_details: &CustomerDetails,
    merchant_context: &MerchantContext,
) -> RouterResult<Option<Customer>> {
    let db: &dyn StorageInterface = &*state.store;

    // Create customer_id if not passed in request
    let customer_id = customer_details
        .customer_id
        .clone()
        .unwrap_or_else(|| common_utils::id_type::CustomerId::generate());

    let merchant_id = merchant_context.get_merchant_account().get_id();
    let key = merchant_context
        .get_merchant_key_store()
        .key
        .get_inner()
        .peek();
    let key_manager_state = &state.into();

    match db
        .find_customer_optional_by_customer_id_merchant_id(
            key_manager_state,
            &customer_id,
            merchant_id,
            merchant_context.get_merchant_key_store(),
            merchant_context.get_merchant_account().storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?
    {
        // Customer found
        Some(customer) => Ok(Some(customer)),

        // Customer not found
        // create only if atleast one of the fields were provided for customer creation or else throw error
        None => {
            if customer_details.name.is_some()
                || customer_details.email.is_some()
                || customer_details.phone.is_some()
                || customer_details.phone_country_code.is_some()
            {
                let encrypted_data = crypto_operation(
                    &state.into(),
                    type_name!(Customer),
                    CryptoOperation::BatchEncrypt(
                        domain::FromRequestEncryptableCustomer::to_encryptable(
                            domain::FromRequestEncryptableCustomer {
                                name: customer_details.name.clone(),
                                email: customer_details
                                    .email
                                    .clone()
                                    .map(|a| a.expose().switch_strategy()),
                                phone: customer_details.phone.clone(),
                                tax_registration_id: customer_details.tax_registration_id.clone(),
                            },
                        ),
                    ),
                    Identifier::Merchant(
                        merchant_context
                            .get_merchant_key_store()
                            .merchant_id
                            .clone(),
                    ),
                    key,
                )
                .await
                .and_then(|val| val.try_into_batchoperation())
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to encrypt customer")?;
                let encryptable_customer =
                    domain::FromRequestEncryptableCustomer::from_encryptable(encrypted_data)
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to form EncryptableCustomer")?;

                let customer = Customer {
                    customer_id: customer_id.clone(),
                    merchant_id: merchant_id.to_owned().clone(),
                    name: encryptable_customer.name,
                    email: encryptable_customer.email.map(|email| {
                        let encryptable: Encryptable<Secret<String, pii::EmailStrategy>> =
                            Encryptable::new(
                                email.clone().into_inner().switch_strategy(),
                                email.into_encrypted(),
                            );
                        encryptable
                    }),
                    phone: encryptable_customer.phone,
                    description: None,
                    phone_country_code: customer_details.phone_country_code.to_owned(),
                    metadata: None,
                    connector_customer: None,
                    created_at: common_utils::date_time::now(),
                    modified_at: common_utils::date_time::now(),
                    address_id: None,
                    default_payment_method_id: None,
                    updated_by: None,
                    version: common_types::consts::API_VERSION,
                    tax_registration_id: encryptable_customer.tax_registration_id,
                };

                Ok(Some(
                    db.insert_customer(
                        customer,
                        key_manager_state,
                        merchant_context.get_merchant_key_store(),
                        merchant_context.get_merchant_account().storage_scheme,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable_lazy(|| {
                        format!(
                            "Failed to insert customer [id - {customer_id:?}] for merchant [id - {merchant_id:?}]",
                        )
                    })?,
                ))
            } else {
                Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                    message: format!("customer for id - {customer_id:?} not found"),
                }))
            }
        }
    }
}


pub fn get_customer_details_from_request(
    request: CreateSubscriptionRequest,
) -> CustomerDetails {
    let customer_id = request.get_customer_id().map(ToOwned::to_owned);

    let customer_name = request
        .customer
        .as_ref()
        .and_then(|customer_details| customer_details.name.clone());

    let customer_email = request
        .customer
        .as_ref()
        .and_then(|customer_details| customer_details.email.clone());

    let customer_phone = request
        .customer
        .as_ref()
        .and_then(|customer_details| customer_details.phone.clone());

    let customer_phone_code = request
        .customer
        .as_ref()
        .and_then(|customer_details| customer_details.phone_country_code.clone());

    let tax_registration_id = request
        .customer
        .as_ref()
        .and_then(|customer_details| customer_details.tax_registration_id.clone());

    CustomerDetails {
        customer_id,
        name: customer_name,
        email: customer_email,
        phone: customer_phone,
        phone_country_code: customer_phone_code,
        tax_registration_id,
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
    pub customer: Option<CustomerDetails>,
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
    pub plan_id: Option<String>,
    pub coupon_code: Option<String>,
    pub customer: Option<Customer>,
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
    InActive
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
            plan_id: None,
            coupon_code: None,
            customer: None,
            invoice: None,
        }
    }
}

impl ApiEventMetric for CreateSubscriptionResponse {}
impl ApiEventMetric for CreateSubscriptionRequest {}
