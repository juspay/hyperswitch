//! Where each transaction goes, and the transactions that act on an existing payment.
//!
//! | `paymentType` | Flow          | Endpoint                                   |
//! |---------------|---------------|--------------------------------------------|
//! | `DB` / `PA`   | authorize     | `v1/payments`, `v1/registrations/{id}/payments` |
//! | `PA`          | mandate setup | `v1/payments`                              |
//! | `CP`          | capture       | `v1/payments/{id}` ([`capture`])           |
//! | `RV`          | void          | `v1/payments/{id}` ([`reversal`])          |
//! | `RF`          | refund        | `v1/payments/{id}` ([`refund`])            |
//! | `PA` (npa)    | 3DS pre-auth  | `v1/threeDSecure`                          |
//! | —             | sync          | `GET v1/payments/{id}`                     |
//! | —             | 3DS post-auth | `GET v1/threeDSecure/{id}`                 |

pub mod capture;
pub mod refund;
pub mod reversal;

use common_utils::request::Method;
use hyperswitch_domain_models::{
    router_data::ConnectorAuthType, router_request_types::PaymentsAuthorizeData,
};
use hyperswitch_masking::PeekInterface;

use super::{AciAuthType, Error};
use crate::utils::PaymentsAuthorizeRequestData;

/// An ACI API endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Endpoint {
    /// `POST v1/payments`: a new payment.
    Payments,
    /// `POST v1/registrations/{id}/payments`: a charge against a stored registration.
    RegistrationPayments(String),
    /// `POST v1/payments/{id}`: a capture, reversal or refund of payment `id`.
    Payment(String),
    /// `GET v1/payments/{id}`: the status of payment `id`.
    PaymentStatus(String),
    /// `POST v1/threeDSecure`: a standalone 3DS authentication.
    ThreeDSecure,
    /// `GET v1/threeDSecure/{id}`: the result of authentication `id`.
    ThreeDSecureStatus(String),
}

impl Endpoint {
    /// Where an authorize goes: the registration it charges, or a new payment.
    ///
    /// Keyed off a connector mandate id specifically, while the card-on-file leg counts any
    /// `mandate_id` as a repeat, so a network-mandate MIT goes to `v1/payments` with a REPEATED
    /// block and its raw instrument.
    pub fn authorize(request: &PaymentsAuthorizeData) -> Self {
        request
            .connector_mandate_id()
            .map_or(Self::Payments, Self::RegistrationPayments)
    }

    pub fn method(&self) -> Method {
        match self {
            Self::Payments
            | Self::RegistrationPayments(_)
            | Self::Payment(_)
            | Self::ThreeDSecure => Method::Post,
            Self::PaymentStatus(_) | Self::ThreeDSecureStatus(_) => Method::Get,
        }
    }

    /// The URL under `base_url`. Queries name the entity in the query string, so they need the
    /// account's credentials.
    pub fn url(&self, base_url: &str, auth: &ConnectorAuthType) -> Result<String, Error> {
        let entity_query = || -> Result<String, Error> {
            Ok(format!(
                "?entityId={}",
                AciAuthType::try_from(auth)?.entity_id.peek()
            ))
        };
        Ok(match self {
            Self::Payments => format!("{base_url}v1/payments"),
            Self::RegistrationPayments(id) => format!("{base_url}v1/registrations/{id}/payments"),
            Self::Payment(id) => format!("{base_url}v1/payments/{id}"),
            Self::PaymentStatus(id) => format!("{base_url}v1/payments/{id}{}", entity_query()?),
            Self::ThreeDSecure => format!("{base_url}v1/threeDSecure"),
            Self::ThreeDSecureStatus(id) => {
                format!("{base_url}v1/threeDSecure/{id}{}", entity_query()?)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use hyperswitch_masking::Secret;

    use super::*;
    use crate::connectors::aci::test_fixtures::*;

    const BASE: &str = "https://eu-test.oppwa.com/";

    fn auth() -> ConnectorAuthType {
        ConnectorAuthType::BodyKey {
            api_key: Secret::new("key".to_string()),
            key1: Secret::new("entity".to_string()),
        }
    }

    #[test]
    fn endpoints_render_their_urls_and_methods() {
        let cases = [
            (Endpoint::Payments, "v1/payments", Method::Post),
            (
                Endpoint::RegistrationPayments("8ac7".to_string()),
                "v1/registrations/8ac7/payments",
                Method::Post,
            ),
            (
                Endpoint::Payment("8ac7".to_string()),
                "v1/payments/8ac7",
                Method::Post,
            ),
            (
                Endpoint::PaymentStatus("8ac7".to_string()),
                "v1/payments/8ac7?entityId=entity",
                Method::Get,
            ),
            (Endpoint::ThreeDSecure, "v1/threeDSecure", Method::Post),
            (
                Endpoint::ThreeDSecureStatus("8ac7".to_string()),
                "v1/threeDSecure/8ac7?entityId=entity",
                Method::Get,
            ),
        ];
        for (endpoint, path, method) in cases {
            assert_eq!(
                endpoint.url(BASE, &auth()).expect("url renders"),
                format!("{BASE}{path}")
            );
            assert_eq!(endpoint.method(), method, "{endpoint:?}");
        }
    }

    #[test]
    fn a_query_needs_the_entity() {
        assert!(Endpoint::PaymentStatus("8ac7".to_string())
            .url(BASE, &ConnectorAuthType::NoKey)
            .is_err());
        assert!(Endpoint::Payments
            .url(BASE, &ConnectorAuthType::NoKey)
            .is_ok());
    }

    #[test]
    fn only_a_connector_mandate_charges_its_registration() {
        assert_eq!(
            Endpoint::authorize(&mit_authorize_data(COF_RG, None, None)),
            Endpoint::RegistrationPayments(COF_RG.to_string())
        );
        assert_eq!(
            Endpoint::authorize(&cit_authorize_data()),
            Endpoint::Payments
        );
    }
}
