//! Migration-only conversion to the existing modular permanent-storage contract.

use std::str::FromStr;

use api_models::{payment_methods::PaymentMethodRecord, payments};
use common_utils::{
    id_type,
    request::{Method, RequestContent},
};
use hyperswitch_interfaces::micro_service::MicroserviceClientError;
use hyperswitch_masking::{PeekInterface, Secret};

use super::create::{
    CardDetail, CreatePaymentMethodResponse, ModularPMCreateRequest, ModularPaymentMethodResponse,
    PaymentMethodCreateData,
};

#[derive(Clone, Debug)]
pub struct MigrationCardInput {
    pub card: CardDetail,
    pub billing: Option<payments::Address>,
    pub subtype: Option<common_enums::PaymentMethodType>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MigrationCardInputError {
    pub code: &'static str,
    pub fields: Vec<&'static str>,
}

impl MigrationCardInputError {
    fn invalid(field: &'static str) -> Self {
        Self {
            code: "migration_card_input_invalid",
            fields: vec![field],
        }
    }
}

impl TryFrom<&PaymentMethodRecord> for MigrationCardInput {
    type Error = MigrationCardInputError;

    fn try_from(row: &PaymentMethodRecord) -> Result<Self, Self::Error> {
        let mut unsupported = Vec::new();
        macro_rules! reject {
            ($($field:ident),+ $(,)?) => { $(
                if row.$field.is_some() { unsupported.push(stringify!($field)); }
            )+ };
        }
        reject!(
            phone,
            phone_country_code,
            payment_instrument_id,
            connector_customer_id,
            connector_mandate_request_reference_id,
            card_number_masked,
            original_transaction_id,
            original_transaction_amount,
            original_transaction_currency,
            network_token_number,
            network_token_expiry_month,
            network_token_expiry_year,
            network_token_requestor_ref_id,
            account_number,
            routing_number,
            bank_account_holder_name,
            bank_type,
            bank_holder_type,
            bank_name
        );
        if row
            .payment_method
            .is_some_and(|method| method != common_enums::PaymentMethod::Card)
        {
            unsupported.push("payment_method");
        }
        if row.payment_method_type.is_some_and(|kind| {
            !matches!(
                kind,
                common_enums::PaymentMethodType::Credit | common_enums::PaymentMethodType::Debit
            )
        }) {
            unsupported.push("payment_method_type");
        }
        if !unsupported.is_empty() {
            return Err(MigrationCardInputError {
                code: "migration_card_input_unsupported",
                fields: unsupported,
            });
        }
        let raw = row
            .raw_card_number
            .as_ref()
            .ok_or_else(|| Self::Error::invalid("raw_card_number"))?;
        let card_number = cards::CardNumber::from_str(raw.peek())
            .map_err(|_| Self::Error::invalid("raw_card_number"))?;
        let month = row
            .card_expiry_month
            .as_ref()
            .ok_or_else(|| Self::Error::invalid("card_expiry_month"))?;
        let year = row
            .card_expiry_year
            .as_ref()
            .ok_or_else(|| Self::Error::invalid("card_expiry_year"))?;
        crate::core::migration::validate_card_expiry(month, year)
            .map_err(|_| Self::Error::invalid("card_expiry"))?;
        let month_number: u8 = month
            .peek()
            .parse()
            .map_err(|_| Self::Error::invalid("card_expiry_month"))?;
        let mut year_number: u16 = year
            .peek()
            .parse()
            .map_err(|_| Self::Error::invalid("card_expiry_year"))?;
        if year.peek().len() == 2 {
            year_number += 2000;
        }
        // Match the v2 validator, including expiry in the current month.
        let now = common_utils::date_time::now();
        if i32::from(year_number) < now.year()
            || (i32::from(year_number) == now.year() && month_number < u8::from(now.month()))
        {
            return Err(Self::Error::invalid("card_expiry"));
        }
        let card_network = row
            .card_scheme
            .as_ref()
            .map(|network| {
                common_enums::CardNetwork::from_str(network.trim())
                    .or_else(|_| {
                        common_enums::CardNetwork::from_str(&network.trim().to_uppercase())
                    })
                    .map_err(|_| Self::Error::invalid("card_scheme"))
            })
            .transpose()?;
        let address = payments::AddressDetails {
            city: row.billing_address_city.clone(),
            country: row.billing_address_country,
            line1: row.billing_address_line1.clone(),
            line2: row.billing_address_line2.clone(),
            line3: row.billing_address_line3.clone(),
            zip: row.billing_address_zip.clone(),
            state: row.billing_address_state.clone(),
            first_name: row.billing_address_first_name.clone(),
            last_name: row.billing_address_last_name.clone(),
            origin_zip: None,
        };
        let billing =
            (address != payments::AddressDetails::default()).then_some(payments::Address {
                address: Some(address),
                phone: None,
                email: None,
            });
        Ok(Self {
            card: CardDetail {
                card_number,
                card_exp_month: Secret::new(format!("{month_number:02}")),
                card_exp_year: Secret::new(format!("{year_number:04}")),
                card_holder_name: row.card_holder_name.clone(),
                nick_name: row.nick_name.clone(),
                card_network,
                card_issuing_country: None,
                card_issuer: None,
                card_type: None,
                card_subtype: None,
                card_segment_type: None,
                funding_source: None,
                card_cvc: None,
            },
            billing,
            subtype: row.payment_method_type,
        })
    }
}

impl MigrationCardInput {
    pub fn payload(&self, customer_id: id_type::GlobalCustomerId) -> ModularPMCreateRequest {
        ModularPMCreateRequest {
            payment_method_type: common_enums::PaymentMethod::Card,
            payment_method_subtype: self.subtype,
            metadata: None,
            customer_id,
            payment_method_data: PaymentMethodCreateData::Card(self.card.clone()),
            billing: self.billing.clone(),
            network_tokenization: None,
            storage_type: Some(common_enums::StorageType::Persistent),
        }
    }
}

#[derive(Debug)]
pub struct CreateMigrationPaymentMethod;

#[derive(Debug)]
pub struct CreateMigrationPaymentMethodRequest {
    pub card: MigrationCardInput,
    pub customer_id: id_type::GlobalCustomerId,
    pub modular_service_prefix: String,
}

impl TryFrom<&CreateMigrationPaymentMethodRequest> for ModularPMCreateRequest {
    type Error = MicroserviceClientError;
    fn try_from(request: &CreateMigrationPaymentMethodRequest) -> Result<Self, Self::Error> {
        Ok(request.card.payload(request.customer_id.clone()))
    }
}

impl CreateMigrationPaymentMethod {
    fn body(&self, payload: ModularPMCreateRequest) -> Option<RequestContent> {
        Some(RequestContent::Json(Box::new(payload)))
    }
    fn path(&self, request: &CreateMigrationPaymentMethodRequest) -> Vec<(&'static str, String)> {
        vec![("prefix", request.modular_service_prefix.clone())]
    }
}

hyperswitch_interfaces::impl_microservice_flow!(
    CreateMigrationPaymentMethod,
    method = Method::Post,
    path = "/{prefix}/payment-methods",
    v1_request = CreateMigrationPaymentMethodRequest,
    v2_request = ModularPMCreateRequest,
    v2_response = ModularPaymentMethodResponse,
    v1_response = CreatePaymentMethodResponse,
    client = crate::client::PaymentMethodClient<'_>,
    body = CreateMigrationPaymentMethod::body,
    path_params = CreateMigrationPaymentMethod::path,
    disable_connection_closed_retry = true
);

/// Reconciliation must include freshly vaulted `New` cards, which have not yet been used.
/// Ordinary saved-card listings keep their existing default behavior.
#[derive(Debug)]
pub struct ListMigrationCustomerPaymentMethods;

impl ListMigrationCustomerPaymentMethods {
    fn path(
        &self,
        request: &super::list::ListCustomerPaymentMethodsV1Request,
    ) -> Vec<(&'static str, String)> {
        vec![
            ("prefix", request.modular_service_prefix.clone()),
            (
                "customer_id",
                request.customer_id.get_string_repr().to_owned(),
            ),
        ]
    }

    fn query(
        &self,
        _request: &super::list::ListCustomerPaymentMethodsV1Request,
    ) -> Vec<(&'static str, String)> {
        vec![("include_new", "true".to_owned())]
    }
}

hyperswitch_interfaces::impl_microservice_flow!(
    ListMigrationCustomerPaymentMethods,
    method = Method::Get,
    path = "/{prefix}/customers/{customer_id}/saved-payment-methods",
    v1_request = super::list::ListCustomerPaymentMethodsV1Request,
    v2_request = crate::types::ModularListCustomerPaymentMethodsRequest,
    v2_response = crate::types::ModularListCustomerPaymentMethodsResponse,
    v1_response = super::list::ListCustomerPaymentMethodsRawResponse,
    client = crate::client::PaymentMethodClient<'_>,
    path_params = ListMigrationCustomerPaymentMethods::path,
    query_params = ListMigrationCustomerPaymentMethods::query
);
