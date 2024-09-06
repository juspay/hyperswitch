use hyperswitch_domain_models::{router_data_v2::PaymentFlowData, router_data::PaymentMethodToken};
use common_enums::{enums, enums::FutureUsage};
use common_utils::{id_type, pii};
use crate::utils::missing_field_err;
use hyperswitch_domain_models::router_data::RecurringMandatePaymentData;
use hyperswitch_interfaces::{api, errors};
use masking::Secret;
use api_models::payments::PhoneDetails;
use crate::utils::PhoneDetailsData;
use api_models::payments::Address;
use common_utils::pii::Email;
use api_models::payments::AddressDetails;
use common_utils::ext_traits::ValueExt;
type Error = error_stack::Report<errors::ConnectorError>;

pub trait PaymentFlowDataTrait {
    fn get_billing(&self) -> Result<&Address, Error>;
    fn get_billing_country(&self) -> Result<api_models::enums::CountryAlpha2, Error>;
    fn get_billing_phone(&self) -> Result<&PhoneDetails, Error>;
    fn get_description(&self) -> Result<String, Error>;
    fn get_return_url(&self) -> Result<String, Error>;
    fn get_billing_address(&self) -> Result<&AddressDetails, Error>;
    fn get_shipping_address(&self) -> Result<&AddressDetails, Error>;
    fn get_shipping_address_with_phone_number(&self) -> Result<&Address, Error>;
    fn get_connector_meta(&self) -> Result<pii::SecretSerdeValue, Error>;
    fn get_session_token(&self) -> Result<String, Error>;
    fn get_billing_first_name(&self) -> Result<Secret<String>, Error>;
    fn get_billing_full_name(&self) -> Result<Secret<String>, Error>;
    fn get_billing_last_name(&self) -> Result<Secret<String>, Error>;
    fn get_billing_line1(&self) -> Result<Secret<String>, Error>;
    fn get_billing_city(&self) -> Result<String, Error>;
    fn get_billing_email(&self) -> Result<Email, Error>;
    fn get_billing_phone_number(&self) -> Result<Secret<String>, Error>;
    fn to_connector_meta<T>(&self) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned;
    fn is_three_ds(&self) -> bool;
    fn get_payment_method_token(&self) -> Result<PaymentMethodToken, Error>;
    fn get_customer_id(&self) -> Result<id_type::CustomerId, Error>;
    fn get_connector_customer_id(&self) -> Result<String, Error>;
    fn get_preprocessing_id(&self) -> Result<String, Error>;
    fn get_recurring_mandate_payment_data(&self) -> Result<RecurringMandatePaymentData, Error>;

    fn get_optional_billing(&self) -> Option<&Address>;
    fn get_optional_shipping(&self) -> Option<&Address>;
    fn get_optional_shipping_line1(&self) -> Option<Secret<String>>;
    fn get_optional_shipping_line2(&self) -> Option<Secret<String>>;
    fn get_optional_shipping_city(&self) -> Option<String>;
    fn get_optional_shipping_country(&self) -> Option<enums::CountryAlpha2>;
    fn get_optional_shipping_zip(&self) -> Option<Secret<String>>;
    fn get_optional_shipping_state(&self) -> Option<Secret<String>>;
    fn get_optional_shipping_first_name(&self) -> Option<Secret<String>>;
    fn get_optional_shipping_last_name(&self) -> Option<Secret<String>>;
    fn get_optional_shipping_phone_number(&self) -> Option<Secret<String>>;
    fn get_optional_shipping_email(&self) -> Option<Email>;

    fn get_optional_billing_full_name(&self) -> Option<Secret<String>>;
    fn get_optional_billing_line1(&self) -> Option<Secret<String>>;
    fn get_optional_billing_line2(&self) -> Option<Secret<String>>;
    fn get_optional_billing_city(&self) -> Option<String>;
    fn get_optional_billing_country(&self) -> Option<enums::CountryAlpha2>;
    fn get_optional_billing_zip(&self) -> Option<Secret<String>>;
    fn get_optional_billing_state(&self) -> Option<Secret<String>>;
    fn get_optional_billing_first_name(&self) -> Option<Secret<String>>;
    fn get_optional_billing_last_name(&self) -> Option<Secret<String>>;
    fn get_optional_billing_phone_number(&self) -> Option<Secret<String>>;
    fn get_optional_billing_email(&self) -> Option<Email>;
}

impl PaymentFlowDataTrait for PaymentFlowData {
    fn get_billing(&self) -> Result<&Address, Error> {
        self.address
            .get_payment_method_billing()
            .ok_or_else(missing_field_err("billing"))
    }

    fn get_billing_country(&self) -> Result<api_models::enums::CountryAlpha2, Error> {
        self.address
            .get_payment_method_billing()
            .and_then(|a| a.address.as_ref())
            .and_then(|ad| ad.country)
            .ok_or_else(missing_field_err(
                "payment_method_data.billing.address.country",
            ))
    }

    fn get_billing_phone(&self) -> Result<&PhoneDetails, Error> {
        self.address
            .get_payment_method_billing()
            .and_then(|a| a.phone.as_ref())
            .ok_or_else(missing_field_err("billing.phone"))
    }

    fn get_optional_billing(&self) -> Option<&Address> {
        self.address.get_payment_method_billing()
    }

    fn get_optional_shipping(&self) -> Option<&Address> {
        self.address.get_shipping()
    }

    fn get_optional_shipping_first_name(&self) -> Option<Secret<String>> {
        self.address.get_shipping().and_then(|shipping_address| {
            shipping_address
                .clone()
                .address
                .and_then(|shipping_details| shipping_details.first_name)
        })
    }

    fn get_optional_shipping_last_name(&self) -> Option<Secret<String>> {
        self.address.get_shipping().and_then(|shipping_address| {
            shipping_address
                .clone()
                .address
                .and_then(|shipping_details| shipping_details.last_name)
        })
    }

    fn get_optional_shipping_line1(&self) -> Option<Secret<String>> {
        self.address.get_shipping().and_then(|shipping_address| {
            shipping_address
                .clone()
                .address
                .and_then(|shipping_details| shipping_details.line1)
        })
    }

    fn get_optional_shipping_line2(&self) -> Option<Secret<String>> {
        self.address.get_shipping().and_then(|shipping_address| {
            shipping_address
                .clone()
                .address
                .and_then(|shipping_details| shipping_details.line2)
        })
    }

    fn get_optional_shipping_city(&self) -> Option<String> {
        self.address.get_shipping().and_then(|shipping_address| {
            shipping_address
                .clone()
                .address
                .and_then(|shipping_details| shipping_details.city)
        })
    }

    fn get_optional_shipping_state(&self) -> Option<Secret<String>> {
        self.address.get_shipping().and_then(|shipping_address| {
            shipping_address
                .clone()
                .address
                .and_then(|shipping_details| shipping_details.state)
        })
    }

    fn get_optional_shipping_country(&self) -> Option<enums::CountryAlpha2> {
        self.address.get_shipping().and_then(|shipping_address| {
            shipping_address
                .clone()
                .address
                .and_then(|shipping_details| shipping_details.country)
        })
    }

    fn get_optional_shipping_zip(&self) -> Option<Secret<String>> {
        self.address.get_shipping().and_then(|shipping_address| {
            shipping_address
                .clone()
                .address
                .and_then(|shipping_details| shipping_details.zip)
        })
    }

    fn get_optional_shipping_email(&self) -> Option<Email> {
        self.address
            .get_shipping()
            .and_then(|shipping_address| shipping_address.clone().email)
    }

    fn get_optional_shipping_phone_number(&self) -> Option<Secret<String>> {
        self.address
            .get_shipping()
            .and_then(|shipping_address| shipping_address.clone().phone)
            .and_then(|phone_details| phone_details.get_number_with_country_code().ok())
    }

    fn get_description(&self) -> Result<String, Error> {
        self.description
            .clone()
            .ok_or_else(missing_field_err("description"))
    }
    fn get_return_url(&self) -> Result<String, Error> {
        self.return_url
            .clone()
            .ok_or_else(missing_field_err("return_url"))
    }
    fn get_billing_address(&self) -> Result<&AddressDetails, Error> {
        self.address
            .get_payment_method_billing()
            .as_ref()
            .and_then(|a| a.address.as_ref())
            .ok_or_else(missing_field_err("billing.address"))
    }

    fn get_connector_meta(&self) -> Result<pii::SecretSerdeValue, Error> {
        self.connector_meta_data
            .clone()
            .ok_or_else(missing_field_err("connector_meta_data"))
    }

    fn get_session_token(&self) -> Result<String, Error> {
        self.session_token
            .clone()
            .ok_or_else(missing_field_err("session_token"))
    }

    fn get_billing_first_name(&self) -> Result<Secret<String>, Error> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_details| billing_details.first_name.clone())
            })
            .ok_or_else(missing_field_err(
                "payment_method_data.billing.address.first_name",
            ))
    }

    fn get_billing_full_name(&self) -> Result<Secret<String>, Error> {
        self.get_optional_billing()
            .and_then(|billing_details| billing_details.address.as_ref())
            .and_then(|billing_address| billing_address.get_optional_full_name())
            .ok_or_else(missing_field_err(
                "payment_method_data.billing.address.first_name",
            ))
    }

    fn get_billing_last_name(&self) -> Result<Secret<String>, Error> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_details| billing_details.last_name.clone())
            })
            .ok_or_else(missing_field_err(
                "payment_method_data.billing.address.last_name",
            ))
    }

    fn get_billing_line1(&self) -> Result<Secret<String>, Error> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_details| billing_details.line1.clone())
            })
            .ok_or_else(missing_field_err(
                "payment_method_data.billing.address.line1",
            ))
    }
    fn get_billing_city(&self) -> Result<String, Error> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_details| billing_details.city)
            })
            .ok_or_else(missing_field_err(
                "payment_method_data.billing.address.city",
            ))
    }

    fn get_billing_email(&self) -> Result<Email, Error> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| billing_address.email.clone())
            .ok_or_else(missing_field_err("payment_method_data.billing.email"))
    }

    fn get_billing_phone_number(&self) -> Result<Secret<String>, Error> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| billing_address.clone().phone)
            .map(|phone_details| phone_details.get_number_with_country_code())
            .transpose()?
            .ok_or_else(missing_field_err("payment_method_data.billing.phone"))
    }

    fn get_optional_billing_line1(&self) -> Option<Secret<String>> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_details| billing_details.line1)
            })
    }

    fn get_optional_billing_line2(&self) -> Option<Secret<String>> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_details| billing_details.line2)
            })
    }

    fn get_optional_billing_city(&self) -> Option<String> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_details| billing_details.city)
            })
    }

    fn get_optional_billing_country(&self) -> Option<enums::CountryAlpha2> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_details| billing_details.country)
            })
    }

    fn get_optional_billing_zip(&self) -> Option<Secret<String>> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_details| billing_details.zip)
            })
    }

    fn get_optional_billing_state(&self) -> Option<Secret<String>> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_details| billing_details.state)
            })
    }

    fn get_optional_billing_first_name(&self) -> Option<Secret<String>> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_details| billing_details.first_name)
            })
    }

    fn get_optional_billing_last_name(&self) -> Option<Secret<String>> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_details| billing_details.last_name)
            })
    }

    fn get_optional_billing_phone_number(&self) -> Option<Secret<String>> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .phone
                    .and_then(|phone_data| phone_data.number)
            })
    }

    fn get_optional_billing_email(&self) -> Option<Email> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| billing_address.clone().email)
    }
    fn to_connector_meta<T>(&self) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        self.get_connector_meta()?
            .parse_value(std::any::type_name::<T>())
            .change_context(errors::ConnectorError::NoConnectorMetaData)
    }

    fn is_three_ds(&self) -> bool {
        matches!(self.auth_type, enums::AuthenticationType::ThreeDs)
    }

    fn get_shipping_address(&self) -> Result<&AddressDetails, Error> {
        self.address
            .get_shipping()
            .and_then(|a| a.address.as_ref())
            .ok_or_else(missing_field_err("shipping.address"))
    }

    fn get_shipping_address_with_phone_number(&self) -> Result<&Address, Error> {
        self.address
            .get_shipping()
            .ok_or_else(missing_field_err("shipping"))
    }

    fn get_payment_method_token(&self) -> Result<PaymentMethodToken, Error> {
        self.payment_method_token
            .clone()
            .ok_or_else(missing_field_err("payment_method_token"))
    }
    fn get_customer_id(&self) -> Result<id_type::CustomerId, Error> {
        self.customer_id
            .to_owned()
            .ok_or_else(missing_field_err("customer_id"))
    }
    fn get_connector_customer_id(&self) -> Result<String, Error> {
        self.connector_customer
            .to_owned()
            .ok_or_else(missing_field_err("connector_customer_id"))
    }
    fn get_preprocessing_id(&self) -> Result<String, Error> {
        self.preprocessing_id
            .to_owned()
            .ok_or_else(missing_field_err("preprocessing_id"))
    }
    fn get_recurring_mandate_payment_data(&self) -> Result<RecurringMandatePaymentData, Error> {
        self.recurring_mandate_payment_data
            .to_owned()
            .ok_or_else(missing_field_err("recurring_mandate_payment_data"))
    }

    fn get_optional_billing_full_name(&self) -> Option<Secret<String>> {
        self.get_optional_billing()
            .and_then(|billing_details| billing_details.address.as_ref())
            .and_then(|billing_address| billing_address.get_optional_full_name())
    }
}
