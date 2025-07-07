use hyperswitch_domain_models::router_response_types::RedirectForm;

use crate::utils::ForeignFrom;

impl ForeignFrom<RedirectForm> for diesel_models::payment_attempt::RedirectForm {
    fn foreign_from(redirect_form: RedirectForm) -> Self {
        match redirect_form {
            RedirectForm::Form {
                endpoint,
                method,
                form_fields,
            } => Self::Form {
                endpoint,
                method,
                form_fields,
            },
            RedirectForm::Html { html_data } => Self::Html { html_data },
            RedirectForm::BlueSnap {
                payment_fields_token,
            } => Self::BlueSnap {
                payment_fields_token,
            },
            RedirectForm::CybersourceAuthSetup {
                access_token,
                ddc_url,
                reference_id,
            } => Self::CybersourceAuthSetup {
                access_token,
                ddc_url,
                reference_id,
            },
            RedirectForm::CybersourceConsumerAuth {
                access_token,
                step_up_url,
            } => Self::CybersourceConsumerAuth {
                access_token,
                step_up_url,
            },
            RedirectForm::DeutschebankThreeDSChallengeFlow { acs_url, creq } => {
                Self::DeutschebankThreeDSChallengeFlow { acs_url, creq }
            }
            RedirectForm::Payme => Self::Payme,
            RedirectForm::Braintree {
                client_token,
                card_token,
                bin,
                acs_url,
            } => Self::Braintree {
                client_token,
                card_token,
                bin,
                acs_url,
            },
            RedirectForm::Nmi {
                amount,
                currency,
                public_key,
                customer_vault_id,
                order_id,
            } => Self::Nmi {
                amount,
                currency,
                public_key,
                customer_vault_id,
                order_id,
            },
            RedirectForm::Mifinity {
                initialization_token,
            } => Self::Mifinity {
                initialization_token,
            },
            RedirectForm::WorldpayDDCForm {
                endpoint,
                method,
                form_fields,
                collection_id,
            } => Self::WorldpayDDCForm {
                endpoint: common_utils::types::Url::wrap(endpoint),
                method,
                form_fields,
                collection_id,
            },
        }
    }
}

impl ForeignFrom<diesel_models::payment_attempt::RedirectForm> for RedirectForm {
    fn foreign_from(redirect_form: diesel_models::payment_attempt::RedirectForm) -> Self {
        match redirect_form {
            diesel_models::payment_attempt::RedirectForm::Form {
                endpoint,
                method,
                form_fields,
            } => Self::Form {
                endpoint,
                method,
                form_fields,
            },
            diesel_models::payment_attempt::RedirectForm::Html { html_data } => {
                Self::Html { html_data }
            }
            diesel_models::payment_attempt::RedirectForm::BlueSnap {
                payment_fields_token,
            } => Self::BlueSnap {
                payment_fields_token,
            },
            diesel_models::payment_attempt::RedirectForm::CybersourceAuthSetup {
                access_token,
                ddc_url,
                reference_id,
            } => Self::CybersourceAuthSetup {
                access_token,
                ddc_url,
                reference_id,
            },
            diesel_models::payment_attempt::RedirectForm::CybersourceConsumerAuth {
                access_token,
                step_up_url,
            } => Self::CybersourceConsumerAuth {
                access_token,
                step_up_url,
            },
            diesel_models::RedirectForm::DeutschebankThreeDSChallengeFlow { acs_url, creq } => {
                Self::DeutschebankThreeDSChallengeFlow { acs_url, creq }
            }
            diesel_models::payment_attempt::RedirectForm::Payme => Self::Payme,
            diesel_models::payment_attempt::RedirectForm::Braintree {
                client_token,
                card_token,
                bin,
                acs_url,
            } => Self::Braintree {
                client_token,
                card_token,
                bin,
                acs_url,
            },
            diesel_models::payment_attempt::RedirectForm::Nmi {
                amount,
                currency,
                public_key,
                customer_vault_id,
                order_id,
            } => Self::Nmi {
                amount,
                currency,
                public_key,
                customer_vault_id,
                order_id,
            },
            diesel_models::payment_attempt::RedirectForm::Mifinity {
                initialization_token,
            } => Self::Mifinity {
                initialization_token,
            },
            diesel_models::payment_attempt::RedirectForm::WorldpayDDCForm {
                endpoint,
                method,
                form_fields,
                collection_id,
            } => Self::WorldpayDDCForm {
                endpoint: endpoint.into_inner(),
                method,
                form_fields,
                collection_id,
            },
        }
    }
}
