use crate::{
    core::errors,
    pii::PeekInterface,
    types::{self, api},
};

pub fn missing_field_err(
    message: &str,
) -> Box<dyn Fn() -> error_stack::Report<errors::ConnectorError> + '_> {
    Box::new(|| {
        errors::ConnectorError::MissingRequiredField {
            field_name: message.to_string(),
        }
        .into()
    })
}

type Error = error_stack::Report<errors::ConnectorError>;
pub trait PaymentsRequestData {
    fn get_attempt_id(&self) -> Result<String, Error>;
    fn get_billing_country(&self) -> Result<String, Error>;
    fn get_card(&self) -> Result<api::CCard, Error>;
}

pub trait CardData {
    fn get_card_number(&self) -> String;
    fn get_card_expiry_month(&self) -> String;
    fn get_card_expiry_year(&self) -> String;
    fn get_card_expiry_year_2_digit(&self) -> String;
    fn get_card_cvc(&self) -> String;
}
impl CardData for api::CCard {
    fn get_card_number(&self) -> String {
        self.card_number.peek().clone()
    }
    fn get_card_expiry_month(&self) -> String {
        self.card_exp_month.peek().clone()
    }
    fn get_card_expiry_year(&self) -> String {
        self.card_exp_year.peek().clone()
    }
    fn get_card_expiry_year_2_digit(&self) -> String {
        let year = self.card_exp_year.peek().clone();
        year[year.len() - 2..].to_string()
    }
    fn get_card_cvc(&self) -> String {
        self.card_cvc.peek().clone()
    }
}
impl PaymentsRequestData for types::PaymentsAuthorizeRouterData {
    fn get_attempt_id(&self) -> Result<String, Error> {
        self.attempt_id
            .clone()
            .ok_or_else(missing_field_err("attempt_id"))
    }

    fn get_billing_country(&self) -> Result<String, Error> {
        self.address
            .billing
            .clone()
            .and_then(|a| a.address)
            .and_then(|ad| ad.country)
            .ok_or_else(missing_field_err("billing.country"))
    }

    fn get_card(&self) -> Result<api::CCard, Error> {
        match self.request.payment_method_data.clone() {
            api::PaymentMethod::Card(card) => Ok(card),
            _ => Err(missing_field_err("card")()),
        }
    }
}
