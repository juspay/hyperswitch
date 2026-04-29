//! Connector-specific configuration transformer for UCS
//!
//! This module provides functionality to transform connector authentication data
//! into connector-specific configuration structures expected by the Unified Connector Service (UCS).

use std::{collections::HashMap, str::FromStr};

use common_enums::{connector_enums::Connector, enums::Currency};
use error_stack::ResultExt;
use hyperswitch_domain_models::router_data::ConnectorAuthType;
use hyperswitch_masking::{PeekInterface, Secret};
use serde::Serialize;

use crate::{
    core::errors::{self, RouterResult},
    types::transformers::ForeignTryFrom,
};

/// Connector-specific configuration wrapper for UCS.
/// Serializes as: `{"config": {"ConnectorName": {...}}}`
#[derive(Debug, Serialize)]
pub struct UcsConnectorConfig {
    pub config: serde_json::Map<String, serde_json::Value>,
}

impl UcsConnectorConfig {
    /// Creates a new UCS connector config with the connector name as the key in PascalCase
    pub fn new<T: Serialize>(connector: Connector, inner: T) -> RouterResult<Self> {
        let connector_name = format!("{:?}", connector); // PascalCase: Braintree, Cybersource
        let inner_json = serde_json::to_value(&inner)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to serialize connector config inner value")?;
        let mut config = serde_json::Map::new();
        config.insert(connector_name, inner_json);
        Ok(Self { config })
    }
}

/// Metadata structures for parsing connector metadata
#[derive(Debug, serde::Deserialize)]
pub struct CybersourceMetadata {
    disable_avs: Option<bool>,
    disable_cvn: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
pub struct BraintreeMetadata {
    merchant_account_id: Secret<String>,
    merchant_config_currency: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct AdyenMetadata {
    endpoint_prefix: Option<Secret<String>>,
}

#[derive(Debug, serde::Deserialize)]
pub struct TruelayerMetadata {
    merchant_account_id: Option<Secret<String>>,
    account_holder_name: Option<Secret<String>>,
    private_key: Option<Secret<String>>,
    kid: Option<Secret<String>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct PaysafeMetadata {
    pub account_id: PaysafePaymentMethodDetails,
}

/// Paysafe payment method details for account_id configuration.
/// Contains card and ACH account IDs grouped by currency.
/// This struct is compatible with the UCS Paysafe connector expectations.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaysafePaymentMethodDetails {
    // UCS proto map fields — must always be present in serialized JSON (not Option)
    /// Card account IDs by currency
    #[serde(default)]
    pub card: HashMap<Currency, PaysafeCardAccountId>,
    /// ACH account IDs by currency
    #[serde(default)]
    pub ach: HashMap<Currency, PaysafeAchAccountId>,
}

/// Paysafe card account ID configuration for a specific currency
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaysafeCardAccountId {
    /// Non-3DS account ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_three_ds: Option<Secret<String>>,
    /// 3DS account ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub three_ds: Option<Secret<String>>,
}

/// Paysafe ACH account ID configuration for a specific currency
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaysafeAchAccountId {
    /// ACH account ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<Secret<String>>,
}

#[derive(Debug, serde::Deserialize)]
pub struct PeachpaymentsMetadata {
    client_merchant_reference_id: Secret<String>,
    merchant_payment_method_route_id: Secret<String>,
}

/// Connector-specific configuration enum for all supported connectors
#[derive(Debug, Clone, serde::Serialize)]
pub enum ConnectorSpecificConfig {
    /// Adyen connector configuration
    Adyen {
        api_key: Secret<String>,
        merchant_account: Secret<String>,
        review_key: Option<Secret<String>>,
        endpoint_prefix: Option<Secret<String>>,
    },
    /// Braintree connector configuration
    Braintree {
        public_key: Secret<String>,
        private_key: Secret<String>,
        merchant_account_id: Secret<String>,
        merchant_config_currency: Option<String>,
    },
    /// Cybersource connector configuration
    Cybersource {
        api_key: Secret<String>,
        merchant_account: Secret<String>,
        api_secret: Secret<String>,
        disable_avs: Option<bool>,
        disable_cvn: Option<bool>,
    },
    /// Stripe connector configuration
    Stripe { api_key: Secret<String> },
    /// PayPal connector configuration
    Paypal {
        client_id: Secret<String>,
        client_secret: Secret<String>,
        payer_id: Option<Secret<String>>,
    },
    /// Truelayer connector configuration
    Truelayer {
        client_id: Secret<String>,
        client_secret: Secret<String>,
        merchant_account_id: Option<Secret<String>>,
        account_holder_name: Option<Secret<String>>,
        private_key: Option<Secret<String>>,
        kid: Option<Secret<String>>,
    },
    /// Revolv3 connector configuration
    Revolv3 { api_key: Secret<String> },
    /// Fiservcommercehub connector configuration
    Fiservcommercehub {
        api_key: Secret<String>,
        secret: Secret<String>,
        merchant_id: Secret<String>,
        terminal_id: Secret<String>,
    },
    /// Checkout connector configuration
    Checkout {
        api_key: Secret<String>,
        api_secret: Secret<String>,
        processing_channel_id: Secret<String>,
    },
    /// Authorize.net connector configuration
    Authorizedotnet {
        name: Secret<String>,
        transaction_key: Secret<String>,
    },
    /// Bank of America connector configuration
    BankOfAmerica {
        api_key: Secret<String>,
        merchant_account: Secret<String>,
        api_secret: Secret<String>,
    },
    /// Bluesnap connector configuration
    Bluesnap {
        username: Secret<String>,
        password: Secret<String>,
    },
    /// Worldpay connector configuration
    Worldpay {
        username: Secret<String>,
        password: Secret<String>,
        entity_id: Secret<String>,
        merchant_name: Option<Secret<String>>,
    },
    /// Paysafe connector configuration
    Paysafe {
        username: Secret<String>,
        password: Secret<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        account_id: Option<PaysafePaymentMethodDetails>,
    },
    /// Trustpay connector configuration
    Trustpay {
        api_key: Secret<String>,
        project_id: Secret<String>,
        secret_key: Secret<String>,
    },
    /// Nexinets connector configuration
    Nexinets {
        merchant_id: Secret<String>,
        api_key: Secret<String>,
    },
    /// Bambora connector configuration
    Bambora {
        merchant_id: Secret<String>,
        api_key: Secret<String>,
    },
    /// Cashfree connector configuration
    Cashfree {
        app_id: Secret<String>,
        secret_key: Secret<String>,
    },
    /// Razorpay connector configuration
    Razorpay {
        api_key: Secret<String>,
        api_secret: Option<Secret<String>>,
    },
    /// Shift4 connector configuration
    Shift4 { api_key: Secret<String> },
    /// Globalpay connector configuration
    Globalpay {
        app_id: Secret<String>,
        app_key: Secret<String>,
    },
    /// Fiserv connector configuration
    Fiserv {
        api_key: Secret<String>,
        merchant_account: Secret<String>,
        api_secret: Secret<String>,
        terminal_id: Option<Secret<String>>,
    },
    /// Fiservemea connector configuration
    Fiservemea {
        api_key: Secret<String>,
        api_secret: Secret<String>,
    },
    /// Noon connector configuration
    Noon {
        api_key: Secret<String>,
        business_identifier: Secret<String>,
        application_identifier: Secret<String>,
    },
    /// Dlocal connector configuration
    Dlocal {
        x_login: Secret<String>,
        x_trans_key: Secret<String>,
        secret: Secret<String>,
    },
    /// Nuvei connector configuration
    Nuvei {
        merchant_id: Secret<String>,
        merchant_site_id: Secret<String>,
        merchant_secret: Secret<String>,
    },
    /// Mollie connector configuration
    Mollie {
        api_key: Secret<String>,
        profile_token: Option<Secret<String>>,
    },
    /// Multisafepay connector configuration
    Multisafepay { api_key: Secret<String> },
    /// Rapyd connector configuration
    Rapyd {
        access_key: Secret<String>,
        secret_key: Secret<String>,
    },
    /// Payu connector configuration
    Payu {
        api_key: Secret<String>,
        api_secret: Secret<String>,
    },
    /// Cryptopay connector configuration
    Cryptopay {
        api_key: Secret<String>,
        api_secret: Secret<String>,
    },
    /// Iatapay connector configuration
    Iatapay {
        client_id: Secret<String>,
        merchant_id: Secret<String>,
        client_secret: Secret<String>,
    },
    /// Cashtocode connector configuration
    Cashtocode {
        auth_key_map: HashMap<Currency, common_utils::pii::SecretSerdeValue>,
    },
    /// Payload connector configuration
    Payload {
        auth_key_map: HashMap<Currency, common_utils::pii::SecretSerdeValue>,
    },
    /// Xendit connector configuration
    Xendit { api_key: Secret<String> },
    /// Helcim connector configuration
    Helcim { api_key: Secret<String> },
    /// Airwallex connector configuration
    Airwallex {
        api_key: Secret<String>,
        client_id: Secret<String>,
    },
    /// Forte connector configuration
    Forte {
        api_access_id: Secret<String>,
        organization_id: Secret<String>,
        location_id: Secret<String>,
        api_secret_key: Secret<String>,
    },
    /// Paytm connector configuration
    Paytm {
        merchant_id: Secret<String>,
        merchant_key: Secret<String>,
        website: Secret<String>,
        client_id: Option<Secret<String>>,
    },
    /// Payme connector configuration
    Payme {
        seller_payme_id: Secret<String>,
        payme_client_key: Option<Secret<String>>,
    },
    /// Nmi connector configuration
    Nmi {
        api_key: Secret<String>,
        public_key: Option<Secret<String>>,
    },
    /// Volt connector configuration
    Volt {
        username: Secret<String>,
        password: Secret<String>,
        client_id: Secret<String>,
        client_secret: Secret<String>,
    },
    /// Phonepe connector configuration
    Phonepe {
        merchant_id: Secret<String>,
        salt_key: Secret<String>,
        salt_index: Secret<String>,
    },
    /// Elavon connector configuration
    Elavon {
        ssl_merchant_id: Secret<String>,
        ssl_user_id: Secret<String>,
        ssl_pin: Secret<String>,
    },
    /// Redsys connector configuration
    Redsys {
        merchant_id: Secret<String>,
        terminal_id: Secret<String>,
        sha256_pwd: Secret<String>,
    },
    /// Trustpayments connector configuration
    Trustpayments {
        username: Secret<String>,
        password: Secret<String>,
        site_reference: Secret<String>,
    },
    /// Novalnet connector configuration
    Novalnet {
        product_activation_key: Secret<String>,
        payment_access_key: Secret<String>,
        tariff_id: Secret<String>,
    },
    /// Gigadat connector configuration
    Gigadat {
        security_token: Secret<String>,
        access_token: Secret<String>,
        campaign_id: Secret<String>,
    },
    /// Zift connector configuration
    Zift {
        user_name: Secret<String>,
        password: Secret<String>,
        account_id: Secret<String>,
    },
    /// Getnet connector configuration
    Getnet {
        api_key: Secret<String>,
        api_secret: Secret<String>,
        seller_id: Secret<String>,
    },
    /// Hyperpg connector configuration
    Hyperpg {
        username: Secret<String>,
        password: Secret<String>,
        merchant_id: Secret<String>,
    },
    /// Fiuu connector configuration
    Fiuu {
        merchant_id: Secret<String>,
        verify_key: Secret<String>,
        secret_key: Secret<String>,
    },
    /// Tsys connector configuration
    Tsys {
        device_id: Secret<String>,
        transaction_key: Secret<String>,
        developer_id: Secret<String>,
    },
    /// Bamboraapac connector configuration
    Bamboraapac {
        username: Secret<String>,
        password: Secret<String>,
        account_number: Secret<String>,
    },
    /// Worldpayxml connector configuration
    Worldpayxml {
        api_username: Secret<String>,
        api_password: Secret<String>,
        merchant_code: Secret<String>,
    },
    /// Datatrans connector configuration
    Datatrans {
        merchant_id: Secret<String>,
        password: Secret<String>,
    },
    /// Placetopay connector configuration
    Placetopay {
        login: Secret<String>,
        tran_key: Secret<String>,
    },
    /// Loonio connector configuration
    Loonio {
        merchant_id: Secret<String>,
        merchant_token: Secret<String>,
    },
    /// Powertranz connector configuration
    Powertranz {
        power_tranz_id: Secret<String>,
        power_tranz_password: Secret<String>,
    },
    /// Hipay connector configuration
    Hipay {
        api_key: Secret<String>,
        api_secret: Secret<String>,
    },
    /// Jpmorgan connector configuration
    Jpmorgan {
        client_id: Secret<String>,
        client_secret: Secret<String>,
    },
    /// Peachpayments connector configuration
    Peachpayments {
        api_key: Secret<String>,
        tenant_id: Secret<String>,
        client_merchant_reference_id: Option<Secret<String>>,
        merchant_payment_method_route_id: Option<Secret<String>>,
    },
    /// Billwerk connector configuration
    Billwerk {
        api_key: Secret<String>,
        public_api_key: Secret<String>,
    },
    /// Authipay connector configuration
    Authipay {
        api_key: Secret<String>,
        api_secret: Secret<String>,
    },
    /// Nexixpay connector configuration
    Nexixpay { api_key: Secret<String> },
    /// Calida connector configuration
    Calida { api_key: Secret<String> },
    /// Celero connector configuration
    Celero { api_key: Secret<String> },
    /// Stax connector configuration
    Stax { api_key: Secret<String> },
    /// Silverflow connector configuration
    Silverflow {
        api_key: Secret<String>,
        api_secret: Secret<String>,
        merchant_acceptor_key: Secret<String>,
    },
    /// Wellsfargo connector configuration
    Wellsfargo {
        api_key: Secret<String>,
        merchant_account: Secret<String>,
        api_secret: Secret<String>,
    },
    /// Paybox connector configuration
    Paybox {
        site: Secret<String>,
        rank: Secret<String>,
        key: Secret<String>,
        merchant_id: Secret<String>,
    },
    /// Barclaycard connector configuration
    Barclaycard {
        api_key: Secret<String>,
        merchant_account: Secret<String>,
        api_secret: Secret<String>,
    },
    /// Finix connector configuration
    Finix {
        finix_user_name: Secret<String>,
        finix_password: Secret<String>,
        merchant_identity_id: Secret<String>,
        merchant_id: Secret<String>,
    },
    /// Worldpayvantiv connector configuration
    Worldpayvantiv {
        user: Secret<String>,
        password: Secret<String>,
        merchant_id: Secret<String>,
    },
    /// Trustly connector configuration
    Trustly {
        username: Secret<String>,
        password: Secret<String>,
        private_key: Secret<String>,
    },
    /// Itaubank connector configuration
    Itaubank {
        client_id: Secret<String>,
        client_secret: Secret<String>,
    },
    /// Imerchantsolutions connector configuration
    Imerchantsolutions { api_key: Secret<String> },
    /// Sanlam connector configuration
    Sanlam {
        api_key: Secret<String>,
        merchant_id: String,
    },
}

impl ForeignTryFrom<(Connector, &ConnectorAuthType, Option<&serde_json::Value>)>
    for ConnectorSpecificConfig
{
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn foreign_try_from(
        (connector, auth, metadata): (Connector, &ConnectorAuthType, Option<&serde_json::Value>),
    ) -> Result<Self, Self::Error> {
        let err = |msg: &str| {
            error_stack::report!(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(msg.to_string())
        };

        match connector {
            Connector::Adyen => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => {
                    let adyen_meta = metadata
                        .map(|m| {
                            serde_json::from_value::<AdyenMetadata>(m.clone())
                                .map_err(|_| err("Invalid Adyen metadata format"))
                        })
                        .transpose()?;

                    Ok(Self::Adyen {
                        api_key: api_key.clone(),
                        merchant_account: key1.clone(),
                        review_key: Some(api_secret.clone()),
                        endpoint_prefix: adyen_meta
                            .as_ref()
                            .and_then(|m| m.endpoint_prefix.clone()),
                    })
                }
                ConnectorAuthType::BodyKey { api_key, key1 } => {
                    let adyen_meta = metadata
                        .map(|m| {
                            serde_json::from_value::<AdyenMetadata>(m.clone())
                                .map_err(|_| err("Invalid Adyen metadata format"))
                        })
                        .transpose()?;

                    Ok(Self::Adyen {
                        api_key: api_key.clone(),
                        merchant_account: key1.clone(),
                        review_key: None,
                        endpoint_prefix: adyen_meta
                            .as_ref()
                            .and_then(|m| m.endpoint_prefix.clone()),
                    })
                }
                _ => Err(err("Adyen requires SignatureKey or BodyKey auth type")),
            },
            Connector::Cybersource => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => {
                    let cybersource_meta = metadata
                        .map(|m| {
                            serde_json::from_value::<CybersourceMetadata>(m.clone())
                                .map_err(|_| err("Invalid Cybersource metadata format"))
                        })
                        .transpose()?;

                    Ok(Self::Cybersource {
                        api_key: api_key.clone(),
                        merchant_account: key1.clone(),
                        api_secret: api_secret.clone(),
                        disable_avs: cybersource_meta.as_ref().and_then(|m| m.disable_avs),
                        disable_cvn: cybersource_meta.as_ref().and_then(|m| m.disable_cvn),
                    })
                }
                _ => Err(err("Cybersource requires SignatureKey auth type")),
            },
            Connector::Braintree => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1: _,
                    api_secret,
                } => {
                    let braintree_meta = metadata
                        .map(|m| {
                            serde_json::from_value::<BraintreeMetadata>(m.clone())
                                .map_err(|_| err("Invalid Braintree metadata format"))
                        })
                        .transpose()?
                        .ok_or_else(|| err("Braintree requires metadata with merchant_account_id and merchant_config_currency"))?;

                    Ok(Self::Braintree {
                        public_key: api_key.clone(),
                        private_key: api_secret.clone(),
                        merchant_account_id: braintree_meta.merchant_account_id,
                        merchant_config_currency: Some(braintree_meta.merchant_config_currency),
                    })
                }
                _ => Err(err("Braintree requires SignatureKey auth type")),
            },
            Connector::Stripe => match auth {
                ConnectorAuthType::HeaderKey { api_key } => Ok(Self::Stripe {
                    api_key: api_key.clone(),
                }),
                _ => Err(err("Stripe requires HeaderKey auth type")),
            },
            Connector::Truelayer => match auth {
                ConnectorAuthType::BodyKey { api_key, key1 } => {
                    let truelayer_meta = metadata
                        .map(|m| {
                            serde_json::from_value::<TruelayerMetadata>(m.clone())
                                .map_err(|_| err("Invalid Truelayer metadata format"))
                        })
                        .transpose()?;

                    Ok(Self::Truelayer {
                        client_id: api_key.clone(),
                        client_secret: key1.clone(),
                        merchant_account_id: truelayer_meta
                            .as_ref()
                            .and_then(|m| m.merchant_account_id.clone()),
                        account_holder_name: truelayer_meta
                            .as_ref()
                            .and_then(|m| m.account_holder_name.clone()),
                        private_key: truelayer_meta.as_ref().and_then(|m| m.private_key.clone()),
                        kid: truelayer_meta.as_ref().and_then(|m| m.kid.clone()),
                    })
                }
                _ => Err(err("Truelayer requires BodyKey auth type")),
            },
            Connector::Paypal => match auth {
                ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self::Paypal {
                    client_id: key1.clone(),
                    client_secret: api_key.clone(),
                    payer_id: None,
                }),
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Paypal {
                    client_id: key1.clone(),
                    client_secret: api_key.clone(),
                    payer_id: Some(api_secret.clone()),
                }),
                _ => Err(err("Paypal requires BodyKey or SignatureKey auth type")),
            },
            Connector::Revolv3 => match auth {
                ConnectorAuthType::HeaderKey { api_key } => Ok(Self::Revolv3 {
                    api_key: api_key.clone(),
                }),
                _ => Err(err("Revolv3 requires HeaderKey auth type")),
            },
            Connector::Fiservcommercehub => match auth {
                ConnectorAuthType::MultiAuthKey {
                    api_key,
                    key1,
                    api_secret,
                    key2,
                } => Ok(Self::Fiservcommercehub {
                    api_key: api_key.clone(),
                    secret: api_secret.clone(),
                    merchant_id: key1.clone(),
                    terminal_id: key2.clone(),
                }),
                _ => Err(err("Fiservcommercehub requires MultiAuthKey auth type")),
            },
            Connector::Calida => match auth {
                ConnectorAuthType::HeaderKey { api_key } => Ok(Self::Calida {
                    api_key: api_key.clone(),
                }),
                _ => Err(err("Calida requires HeaderKey auth type")),
            },
            Connector::Celero => match auth {
                ConnectorAuthType::HeaderKey { api_key } => Ok(Self::Celero {
                    api_key: api_key.clone(),
                }),
                _ => Err(err("Celero requires HeaderKey auth type")),
            },
            Connector::Helcim => match auth {
                ConnectorAuthType::HeaderKey { api_key } => Ok(Self::Helcim {
                    api_key: api_key.clone(),
                }),
                _ => Err(err("Helcim requires HeaderKey auth type")),
            },
            Connector::Multisafepay => match auth {
                ConnectorAuthType::HeaderKey { api_key } => Ok(Self::Multisafepay {
                    api_key: api_key.clone(),
                }),
                _ => Err(err("Multisafepay requires HeaderKey auth type")),
            },
            Connector::Nexixpay => match auth {
                ConnectorAuthType::HeaderKey { api_key } => Ok(Self::Nexixpay {
                    api_key: api_key.clone(),
                }),
                _ => Err(err("Nexixpay requires HeaderKey auth type")),
            },
            Connector::Shift4 => match auth {
                ConnectorAuthType::HeaderKey { api_key } => Ok(Self::Shift4 {
                    api_key: api_key.clone(),
                }),
                _ => Err(err("Shift4 requires HeaderKey auth type")),
            },
            Connector::Stax => match auth {
                ConnectorAuthType::HeaderKey { api_key } => Ok(Self::Stax {
                    api_key: api_key.clone(),
                }),
                _ => Err(err("Stax requires HeaderKey auth type")),
            },
            Connector::Xendit => match auth {
                ConnectorAuthType::HeaderKey { api_key } => Ok(Self::Xendit {
                    api_key: api_key.clone(),
                }),
                _ => Err(err("Xendit requires HeaderKey auth type")),
            },
            Connector::Airwallex => match auth {
                ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self::Airwallex {
                    api_key: api_key.clone(),
                    client_id: key1.clone(),
                }),
                _ => Err(err("Airwallex requires BodyKey auth type")),
            },
            Connector::Authorizedotnet => match auth {
                ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self::Authorizedotnet {
                    name: api_key.clone(),
                    transaction_key: key1.clone(),
                }),
                _ => Err(err("Authorizedotnet requires BodyKey auth type")),
            },
            Connector::Bambora => match auth {
                ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self::Bambora {
                    merchant_id: key1.clone(),
                    api_key: api_key.clone(),
                }),
                _ => Err(err("Bambora requires BodyKey auth type")),
            },
            Connector::Billwerk => match auth {
                ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self::Billwerk {
                    api_key: api_key.clone(),
                    public_api_key: key1.clone(),
                }),
                _ => Err(err("Billwerk requires BodyKey auth type")),
            },
            Connector::Bluesnap => match auth {
                ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self::Bluesnap {
                    username: key1.clone(),
                    password: api_key.clone(),
                }),
                _ => Err(err("Bluesnap requires BodyKey auth type")),
            },
            Connector::Cryptopay => match auth {
                ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self::Cryptopay {
                    api_key: api_key.clone(),
                    api_secret: key1.clone(),
                }),
                _ => Err(err("Cryptopay requires BodyKey auth type")),
            },
            Connector::Datatrans => match auth {
                ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self::Datatrans {
                    merchant_id: key1.clone(),
                    password: api_key.clone(),
                }),
                _ => Err(err("Datatrans requires BodyKey auth type")),
            },
            Connector::Globalpay => match auth {
                ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self::Globalpay {
                    app_id: key1.clone(),
                    app_key: api_key.clone(),
                }),
                _ => Err(err("Globalpay requires BodyKey auth type")),
            },
            Connector::Hipay => match auth {
                ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self::Hipay {
                    api_key: api_key.clone(),
                    api_secret: key1.clone(),
                }),
                _ => Err(err("Hipay requires BodyKey auth type")),
            },
            Connector::Jpmorgan => match auth {
                ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self::Jpmorgan {
                    client_id: api_key.clone(),
                    client_secret: key1.clone(),
                }),
                _ => Err(err("Jpmorgan requires BodyKey auth type")),
            },
            Connector::Loonio => match auth {
                ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self::Loonio {
                    merchant_id: api_key.clone(),
                    merchant_token: key1.clone(),
                }),
                _ => Err(err("Loonio requires BodyKey auth type")),
            },
            Connector::Paysafe => match auth {
                ConnectorAuthType::BodyKey { api_key, key1 } => {
                    let paysafe_meta = metadata
                        .map(|m| {
                            serde_json::from_value::<PaysafeMetadata>(m.clone())
                                .map_err(|_| err("Invalid Paysafe metadata format"))
                        })
                        .transpose()?;
                    Ok(Self::Paysafe {
                        username: api_key.clone(),
                        password: key1.clone(),
                        account_id: paysafe_meta.map(|m| m.account_id),
                    })
                }
                _ => Err(err("Paysafe requires BodyKey auth type")),
            },
            Connector::Payu => match auth {
                ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self::Payu {
                    api_key: api_key.clone(),
                    api_secret: key1.clone(),
                }),
                _ => Err(err("Payu requires BodyKey auth type")),
            },
            Connector::Placetopay => match auth {
                ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self::Placetopay {
                    login: api_key.clone(),
                    tran_key: key1.clone(),
                }),
                _ => Err(err("Placetopay requires BodyKey auth type")),
            },
            Connector::Powertranz => match auth {
                ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self::Powertranz {
                    power_tranz_id: key1.clone(),
                    power_tranz_password: api_key.clone(),
                }),
                _ => Err(err("Powertranz requires BodyKey auth type")),
            },
            Connector::Rapyd => match auth {
                ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self::Rapyd {
                    access_key: api_key.clone(),
                    secret_key: key1.clone(),
                }),
                _ => Err(err("Rapyd requires BodyKey auth type")),
            },
            Connector::Peachpayments => match auth {
                ConnectorAuthType::BodyKey { api_key, key1 } => {
                    let peach_meta = metadata
                        .map(|meta| {
                            serde_json::from_value::<PeachpaymentsMetadata>(meta.clone())
                                .map_err(|_| err("Invalid peachpayments metadata format"))
                        })
                        .transpose()?;
                    Ok(Self::Peachpayments {
                        api_key: api_key.clone(),
                        tenant_id: key1.clone(),
                        client_merchant_reference_id: peach_meta
                            .as_ref()
                            .map(|metadata| metadata.client_merchant_reference_id.clone()),
                        merchant_payment_method_route_id: peach_meta
                            .as_ref()
                            .map(|metadata| metadata.merchant_payment_method_route_id.clone()),
                    })
                }
                _ => Err(err("Peachpayments requires BodyKey auth type")),
            },
            Connector::Nexinets => match auth {
                ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self::Nexinets {
                    merchant_id: key1.clone(),
                    api_key: api_key.clone(),
                }),
                _ => Err(err("Nexinets requires BodyKey auth type")),
            },
            Connector::Razorpay => match auth {
                ConnectorAuthType::HeaderKey { api_key } => Ok(Self::Razorpay {
                    api_key: api_key.clone(),
                    api_secret: None,
                }),
                ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self::Razorpay {
                    api_key: api_key.clone(),
                    api_secret: Some(key1.clone()),
                }),
                _ => Err(err("Razorpay requires HeaderKey or BodyKey auth type")),
            },
            Connector::Mollie => match auth {
                ConnectorAuthType::HeaderKey { api_key } => Ok(Self::Mollie {
                    api_key: api_key.clone(),
                    profile_token: None,
                }),
                ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self::Mollie {
                    api_key: api_key.clone(),
                    profile_token: Some(key1.clone()),
                }),
                _ => Err(err("Mollie requires HeaderKey or BodyKey auth type")),
            },
            Connector::Nmi => match auth {
                ConnectorAuthType::HeaderKey { api_key } => Ok(Self::Nmi {
                    api_key: api_key.clone(),
                    public_key: None,
                }),
                ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self::Nmi {
                    api_key: api_key.clone(),
                    public_key: Some(key1.clone()),
                }),
                _ => Err(err("Nmi requires HeaderKey or BodyKey auth type")),
            },
            Connector::Authipay => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1: _,
                    api_secret,
                } => Ok(Self::Authipay {
                    api_key: api_key.clone(),
                    api_secret: api_secret.clone(),
                }),
                ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self::Authipay {
                    api_key: api_key.clone(),
                    api_secret: key1.clone(),
                }),
                _ => Err(err("Authipay requires SignatureKey or BodyKey auth type")),
            },
            Connector::Bankofamerica => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::BankOfAmerica {
                    api_key: api_key.clone(),
                    merchant_account: key1.clone(),
                    api_secret: api_secret.clone(),
                }),
                _ => Err(err("Bankofamerica requires SignatureKey auth type")),
            },
            Connector::Bamboraapac => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Bamboraapac {
                    username: api_key.clone(),
                    password: api_secret.clone(),
                    account_number: key1.clone(),
                }),
                _ => Err(err("Bamboraapac requires SignatureKey auth type")),
            },
            Connector::Barclaycard => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Barclaycard {
                    api_key: api_key.clone(),
                    merchant_account: key1.clone(),
                    api_secret: api_secret.clone(),
                }),
                _ => Err(err("Barclaycard requires SignatureKey auth type")),
            },
            Connector::Checkout => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Checkout {
                    api_key: api_key.clone(),
                    api_secret: api_secret.clone(),
                    processing_channel_id: key1.clone(),
                }),
                _ => Err(err("Checkout requires SignatureKey auth type")),
            },
            Connector::Dlocal => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Dlocal {
                    x_login: api_key.clone(),
                    x_trans_key: key1.clone(),
                    secret: api_secret.clone(),
                }),
                _ => Err(err("Dlocal requires SignatureKey auth type")),
            },
            Connector::Elavon => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Elavon {
                    ssl_merchant_id: api_key.clone(),
                    ssl_user_id: key1.clone(),
                    ssl_pin: api_secret.clone(),
                }),
                _ => Err(err("Elavon requires SignatureKey auth type")),
            },
            Connector::Fiserv => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Fiserv {
                    api_key: api_key.clone(),
                    merchant_account: key1.clone(),
                    api_secret: api_secret.clone(),
                    terminal_id: None,
                }),
                _ => Err(err("Fiserv requires SignatureKey auth type")),
            },
            Connector::Fiservemea => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1: _,
                    api_secret,
                } => Ok(Self::Fiservemea {
                    api_key: api_key.clone(),
                    api_secret: api_secret.clone(),
                }),
                ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self::Fiservemea {
                    api_key: api_key.clone(),
                    api_secret: key1.clone(),
                }),
                _ => Err(err("Fiservemea requires SignatureKey or BodyKey auth type")),
            },
            Connector::Fiuu => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Fiuu {
                    merchant_id: key1.clone(),
                    verify_key: api_key.clone(),
                    secret_key: api_secret.clone(),
                }),
                _ => Err(err("Fiuu requires SignatureKey auth type")),
            },
            Connector::Getnet => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Getnet {
                    api_key: api_key.clone(),
                    api_secret: api_secret.clone(),
                    seller_id: key1.clone(),
                }),
                _ => Err(err("Getnet requires SignatureKey auth type")),
            },
            Connector::Gigadat => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Gigadat {
                    security_token: api_secret.clone(),
                    access_token: api_key.clone(),
                    campaign_id: key1.clone(),
                }),
                _ => Err(err("Gigadat requires SignatureKey auth type")),
            },
            Connector::Hyperpg => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Hyperpg {
                    username: api_key.clone(),
                    password: key1.clone(),
                    merchant_id: api_secret.clone(),
                }),
                _ => Err(err("Hyperpg requires SignatureKey auth type")),
            },
            Connector::Iatapay => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Iatapay {
                    client_id: api_key.clone(),
                    merchant_id: key1.clone(),
                    client_secret: api_secret.clone(),
                }),
                _ => Err(err("Iatapay requires SignatureKey auth type")),
            },
            Connector::Noon => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Noon {
                    api_key: api_key.clone(),
                    business_identifier: key1.clone(),
                    application_identifier: api_secret.clone(),
                }),
                _ => Err(err("Noon requires SignatureKey auth type")),
            },
            Connector::Novalnet => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Novalnet {
                    product_activation_key: api_key.clone(),
                    payment_access_key: key1.clone(),
                    tariff_id: api_secret.clone(),
                }),
                _ => Err(err("Novalnet requires SignatureKey auth type")),
            },
            Connector::Nuvei => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Nuvei {
                    merchant_id: api_key.clone(),
                    merchant_site_id: key1.clone(),
                    merchant_secret: api_secret.clone(),
                }),
                _ => Err(err("Nuvei requires SignatureKey auth type")),
            },
            Connector::Phonepe => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Phonepe {
                    merchant_id: api_key.clone(),
                    salt_key: key1.clone(),
                    salt_index: api_secret.clone(),
                }),
                _ => Err(err("Phonepe requires SignatureKey auth type")),
            },
            Connector::Redsys => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Redsys {
                    merchant_id: api_key.clone(),
                    terminal_id: key1.clone(),
                    sha256_pwd: api_secret.clone(),
                }),
                _ => Err(err("Redsys requires SignatureKey auth type")),
            },
            Connector::Silverflow => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Silverflow {
                    api_key: api_key.clone(),
                    api_secret: api_secret.clone(),
                    merchant_acceptor_key: key1.clone(),
                }),
                _ => Err(err("Silverflow requires SignatureKey auth type")),
            },
            Connector::Trustpay => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Trustpay {
                    api_key: api_key.clone(),
                    project_id: key1.clone(),
                    secret_key: api_secret.clone(),
                }),
                _ => Err(err("Trustpay requires SignatureKey auth type")),
            },
            Connector::Trustpayments => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Trustpayments {
                    username: api_key.clone(),
                    password: key1.clone(),
                    site_reference: api_secret.clone(),
                }),
                _ => Err(err("Trustpayments requires SignatureKey auth type")),
            },
            Connector::Tsys => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Tsys {
                    device_id: api_key.clone(),
                    transaction_key: key1.clone(),
                    developer_id: api_secret.clone(),
                }),
                _ => Err(err("Tsys requires SignatureKey auth type")),
            },
            Connector::Wellsfargo => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Wellsfargo {
                    api_key: api_key.clone(),
                    merchant_account: key1.clone(),
                    api_secret: api_secret.clone(),
                }),
                _ => Err(err("Wellsfargo requires SignatureKey auth type")),
            },
            Connector::Worldpay => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Worldpay {
                    username: key1.clone(),
                    password: api_key.clone(),
                    entity_id: api_secret.clone(),
                    merchant_name: None,
                }),
                _ => Err(err("Worldpay requires SignatureKey auth type")),
            },
            Connector::Worldpayxml => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Worldpayxml {
                    api_username: api_key.clone(),
                    api_password: key1.clone(),
                    merchant_code: api_secret.clone(),
                }),
                _ => Err(err("Worldpayxml requires SignatureKey auth type")),
            },
            Connector::Zift => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Zift {
                    user_name: api_key.clone(),
                    password: api_secret.clone(),
                    account_id: key1.clone(),
                }),
                _ => Err(err("Zift requires SignatureKey auth type")),
            },
            Connector::Forte => match auth {
                ConnectorAuthType::MultiAuthKey {
                    api_key,
                    key1,
                    api_secret,
                    key2,
                } => Ok(Self::Forte {
                    api_access_id: api_key.clone(),
                    organization_id: key1.clone(),
                    location_id: key2.clone(),
                    api_secret_key: api_secret.clone(),
                }),
                _ => Err(err("Forte requires MultiAuthKey auth type")),
            },
            Connector::Paybox => match auth {
                ConnectorAuthType::MultiAuthKey {
                    api_key,
                    key1,
                    api_secret,
                    key2,
                } => Ok(Self::Paybox {
                    site: api_key.clone(),
                    rank: key1.clone(),
                    key: api_secret.clone(),
                    merchant_id: key2.clone(),
                }),
                _ => Err(err("Paybox requires MultiAuthKey auth type")),
            },
            Connector::Paytm => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Paytm {
                    merchant_id: api_key.clone(),
                    merchant_key: key1.clone(),
                    website: api_secret.clone(),
                    client_id: None,
                }),
                ConnectorAuthType::MultiAuthKey {
                    api_key,
                    key1,
                    api_secret,
                    key2,
                } => Ok(Self::Paytm {
                    merchant_id: api_key.clone(),
                    merchant_key: key1.clone(),
                    website: api_secret.clone(),
                    client_id: Some(key2.clone()),
                }),
                _ => Err(err("Paytm requires SignatureKey or MultiAuthKey auth type")),
            },
            Connector::Volt => match auth {
                ConnectorAuthType::MultiAuthKey {
                    api_key,
                    key1,
                    api_secret,
                    key2,
                } => Ok(Self::Volt {
                    username: api_key.clone(),
                    password: api_secret.clone(),
                    client_id: key1.clone(),
                    client_secret: key2.clone(),
                }),
                _ => Err(err("Volt requires MultiAuthKey auth type")),
            },
            Connector::Finix => match auth {
                ConnectorAuthType::MultiAuthKey {
                    api_key,
                    key1,
                    api_secret,
                    key2,
                } => Ok(Self::Finix {
                    finix_user_name: api_key.clone(),
                    finix_password: api_secret.clone(),
                    merchant_identity_id: key1.clone(),
                    merchant_id: key2.clone(),
                }),
                _ => Err(err("Finix requires MultiAuthKey auth type")),
            },
            Connector::Cashtocode => match auth {
                ConnectorAuthType::CurrencyAuthKey { auth_key_map } => Ok(Self::Cashtocode {
                    auth_key_map: auth_key_map.clone(),
                }),
                _ => Err(err("Cashtocode requires CurrencyAuthKey auth type")),
            },
            Connector::Payload => match auth {
                ConnectorAuthType::CurrencyAuthKey { auth_key_map } => Ok(Self::Payload {
                    auth_key_map: auth_key_map.clone(),
                }),
                _ => Err(err("Payload requires CurrencyAuthKey auth type")),
            },
            Connector::Worldpayvantiv => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Worldpayvantiv {
                    user: api_key.clone(),
                    password: api_secret.clone(),
                    merchant_id: key1.clone(),
                }),
                ConnectorAuthType::MultiAuthKey {
                    api_key,
                    key1,
                    api_secret,
                    key2: _,
                } => Ok(Self::Worldpayvantiv {
                    user: api_key.clone(),
                    password: api_secret.clone(),
                    merchant_id: key1.clone(),
                }),
                _ => Err(err(
                    "Worldpayvantiv requires SignatureKey or MultiAuthKey auth type",
                )),
            },
            Connector::Payme => match auth {
                ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self::Payme {
                    seller_payme_id: api_key.clone(),
                    payme_client_key: Some(key1.clone()),
                }),
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret: _,
                } => Ok(Self::Payme {
                    seller_payme_id: api_key.clone(),
                    payme_client_key: Some(key1.clone()),
                }),
                _ => Err(err("Payme requires BodyKey or SignatureKey auth type")),
            },
            Connector::Trustly => match auth {
                ConnectorAuthType::SignatureKey {
                    api_key,
                    key1,
                    api_secret,
                } => Ok(Self::Trustly {
                    username: api_key.clone(),
                    password: key1.clone(),
                    private_key: api_secret.clone(),
                }),
                _ => Err(err("Trustly requires SignatureKey auth type")),
            },
            Connector::Itaubank => match auth {
                ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self::Itaubank {
                    client_secret: api_key.clone(),
                    client_id: key1.clone(),
                }),
                _ => Err(err("Itaubank requires BodyKey auth type")),
            },
            Connector::Imerchantsolutions => match auth {
                ConnectorAuthType::HeaderKey { api_key } => Ok(Self::Imerchantsolutions {
                    api_key: api_key.clone(),
                }),
                _ => Err(err("Imerchantsolutions requires HeaderKey auth type")),
            },
            Connector::Sanlam => match auth {
                ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self::Sanlam {
                    api_key: api_key.clone(),
                    merchant_id: key1.peek().clone(),
                }),
                _ => Err(err("Sanlam requires BodyKey auth type")),
            },
            // --- Unsupported connectors ---
            _ => Err(
                error_stack::report!(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(format!(
                        "Connector {} not yet supported for ConnectorSpecificConfig",
                        connector
                    )),
            ),
        }
    }
}

/// Build the X_CONNECTOR_CONFIG header value for any connector
pub fn build_connector_config_header(
    connector_name: &str,
    auth_type: &ConnectorAuthType,
    merchant_account_metadata: Option<&serde_json::Value>,
) -> RouterResult<Option<String>> {
    let connector = Connector::from_str(connector_name)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| format!("Invalid connector name: {}", connector_name))?;

    let config = ConnectorSpecificConfig::foreign_try_from((
        connector,
        auth_type,
        merchant_account_metadata,
    ))?;

    let config_json = serde_json::to_value(&config)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to serialize connector config to JSON value")?;

    let mut outer_map = serde_json::Map::new();
    outer_map.insert("config".to_string(), config_json);

    let config_string = serde_json::to_string(&outer_map)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to serialize ConnectorSpecificConfig")?;

    Ok(Some(config_string))
}
