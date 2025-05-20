use api_models::{payment_methods::PaymentMethodId, proxy as proxy_api_models};
use common_utils::{ext_traits::OptionExt, id_type};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    errors::api_error_response::NotImplementedMessage, payment_methods,
};
use masking::Mask;
use x509_parser::nom::{
    bytes::complete::{tag, take_while1},
    character::complete::{char, multispace0},
    sequence::{delimited, preceded, terminated},
    IResult,
};

use crate::{
    core::errors::{self, RouterResult},
    routes::SessionState,
    types::domain,
};

pub struct ProxyRequestWrapper(pub proxy_api_models::ProxyRequest);

impl ProxyRequestWrapper {
    pub async fn get_vault_id(
        &self,
        state: &SessionState,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: common_enums::enums::MerchantStorageScheme,
    ) -> RouterResult<payment_methods::VaultId> {
        let token = &self.0.token;

        match self.0.token_type {
            proxy_api_models::TokenType::PaymentMethodId => {
                let pm_id = PaymentMethodId {
                    payment_method_id: token.clone(),
                };
                let pm_id =
                    id_type::GlobalPaymentMethodId::generate_from_string(pm_id.payment_method_id)
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Unable to generate GlobalPaymentMethodId")?;

                state
                    .store
                    .find_payment_method(&((state).into()), key_store, &pm_id, storage_scheme)
                    .await
                    .change_context(errors::ApiErrorResponse::PaymentMethodNotFound)?
                    .locker_id
                    .get_required_value("vault_id")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Locker id not present in Payment Method Entry")
            }
            proxy_api_models::TokenType::TokenizationId => {
                Err(report!(errors::ApiErrorResponse::NotImplemented {
                    message: NotImplementedMessage::Reason(
                        "Proxy flow using tokenization id".to_string(),
                    ),
                }))
            }
        }
    }

    pub fn get_headers(&self) -> Vec<(String, masking::Maskable<String>)> {
        self.0
            .headers
            .as_map()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone().into_masked()))
            .collect()
    }

    pub fn get_destination_url(&self) -> &str {
        self.0.destination_url.as_str()
    }

    pub fn get_method(&self) -> common_utils::request::Method {
        self.0.method
    }
}

#[derive(Debug)]
pub struct TokenReference {
    pub field: String,
}

pub fn parse_token(input: &str) -> IResult<&str, TokenReference> {
    let (input, field) = delimited(
        tag("{{"),
        preceded(
            multispace0,
            preceded(
                char('$'),
                terminated(
                    take_while1(|c: char| c.is_alphanumeric() || c == '_'),
                    multispace0,
                ),
            ),
        ),
        tag("}}"),
    )(input)?;

    Ok((
        input,
        TokenReference {
            field: field.to_string(),
        },
    ))
}

pub fn contains_token(s: &str) -> bool {
    s.contains("{{") && s.contains("$") && s.contains("}}")
}
