use std::borrow::Cow;

use base64::Engine;
use common_utils::{
    ext_traits::{AsyncExt, ByteSliceExt, ValueExt},
    fp_utils, generate_id, pii,
};
use data_models::{
    mandates::MandateData,
    payments::{payment_attempt::PaymentAttempt, PaymentIntent},
};
use diesel_models::enums;
// TODO : Evaluate all the helper functions ()
use error_stack::{report, IntoReport, ResultExt};
#[cfg(feature = "kms")]
use external_services::kms;
use josekit::jwe;
use masking::{ExposeInterface, PeekInterface};
use openssl::{
    derive::Deriver,
    pkey::PKey,
    symm::{decrypt_aead, Cipher},
};
use router_env::{instrument, logger, tracing};
use time::Duration;
use uuid::Uuid;
use x509_parser::parse_x509_certificate;

use super::{
    operations::{BoxedOperation, Operation, PaymentResponse},
    CustomerDetails, PaymentData,
};
use crate::{
    configs::settings::{ConnectorRequestReferenceIdConfig, Server, TempLockerEnableConfig},
    connector,
    consts::{self, BASE64_ENGINE},
    core::{
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payment_methods::{cards, vault, PaymentMethodRetrieve},
        payments,
    },
    db::StorageInterface,
    routes::{metrics, payment_methods, AppState},
    services,
    types::{
        api::{self, admin, enums as api_enums, MandateValidationFieldsExt},
        domain::{
            self,
            types::{self, AsyncLift},
        },
        storage::{self, enums as storage_enums, ephemeral_key, CustomerUpdate::Update},
        transformers::{ForeignFrom, ForeignTryFrom},
        ErrorResponse, MandateReference, RouterData,
    },
    utils::{
        self,
        crypto::{self, SignMessage},
        OptionExt,
    },
};

pub fn create_identity_from_certificate_and_key(
    encoded_certificate: String,
    encoded_certificate_key: String,
) -> Result<reqwest::Identity, error_stack::Report<errors::ApiClientError>> {
    let decoded_certificate = BASE64_ENGINE
        .decode(encoded_certificate)
        .into_report()
        .change_context(errors::ApiClientError::CertificateDecodeFailed)?;

    let decoded_certificate_key = BASE64_ENGINE
        .decode(encoded_certificate_key)
        .into_report()
        .change_context(errors::ApiClientError::CertificateDecodeFailed)?;

    let certificate = String::from_utf8(decoded_certificate)
        .into_report()
        .change_context(errors::ApiClientError::CertificateDecodeFailed)?;

    let certificate_key = String::from_utf8(decoded_certificate_key)
        .into_report()
        .change_context(errors::ApiClientError::CertificateDecodeFailed)?;

    reqwest::Identity::from_pkcs8_pem(certificate.as_bytes(), certificate_key.as_bytes())
        .into_report()
        .change_context(errors::ApiClientError::CertificateDecodeFailed)
}

pub fn filter_mca_based_on_business_profile(
    merchant_connector_accounts: Vec<domain::MerchantConnectorAccount>,
    profile_id: Option<String>,
) -> Vec<domain::MerchantConnectorAccount> {
    if let Some(profile_id) = profile_id {
        merchant_connector_accounts
            .into_iter()
            .filter(|mca| mca.profile_id.as_ref() == Some(&profile_id))
            .collect::<Vec<_>>()
    } else {
        merchant_connector_accounts
    }
}

#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn create_or_update_address_for_payment_by_request(
    db: &dyn StorageInterface,
    req_address: Option<&api::Address>,
    address_id: Option<&str>,
    merchant_id: &str,
    customer_id: Option<&String>,
    merchant_key_store: &domain::MerchantKeyStore,
    payment_id: &str,
    storage_scheme: storage_enums::MerchantStorageScheme,
) -> CustomResult<Option<domain::Address>, errors::ApiErrorResponse> {
    let key = merchant_key_store.key.get_inner().peek();

    Ok(match address_id {
        Some(id) => match req_address {
            Some(address) => {
                let address_update = async {
                    Ok(storage::AddressUpdate::Update {
                        city: address
                            .address
                            .as_ref()
                            .and_then(|value| value.city.clone()),
                        country: address.address.as_ref().and_then(|value| value.country),
                        line1: address
                            .address
                            .as_ref()
                            .and_then(|value| value.line1.clone())
                            .async_lift(|inner| types::encrypt_optional(inner, key))
                            .await?,
                        line2: address
                            .address
                            .as_ref()
                            .and_then(|value| value.line2.clone())
                            .async_lift(|inner| types::encrypt_optional(inner, key))
                            .await?,
                        line3: address
                            .address
                            .as_ref()
                            .and_then(|value| value.line3.clone())
                            .async_lift(|inner| types::encrypt_optional(inner, key))
                            .await?,
                        state: address
                            .address
                            .as_ref()
                            .and_then(|value| value.state.clone())
                            .async_lift(|inner| types::encrypt_optional(inner, key))
                            .await?,
                        zip: address
                            .address
                            .as_ref()
                            .and_then(|value| value.zip.clone())
                            .async_lift(|inner| types::encrypt_optional(inner, key))
                            .await?,
                        first_name: address
                            .address
                            .as_ref()
                            .and_then(|value| value.first_name.clone())
                            .async_lift(|inner| types::encrypt_optional(inner, key))
                            .await?,
                        last_name: address
                            .address
                            .as_ref()
                            .and_then(|value| value.last_name.clone())
                            .async_lift(|inner| types::encrypt_optional(inner, key))
                            .await?,
                        phone_number: address
                            .phone
                            .as_ref()
                            .and_then(|value| value.number.clone())
                            .async_lift(|inner| types::encrypt_optional(inner, key))
                            .await?,
                        country_code: address
                            .phone
                            .as_ref()
                            .and_then(|value| value.country_code.clone()),
                        updated_by: storage_scheme.to_string(),
                    })
                }
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed while encrypting address")?;
                let address = db
                    .find_address_by_merchant_id_payment_id_address_id(
                        merchant_id,
                        payment_id,
                        id,
                        merchant_key_store,
                        storage_scheme,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Error while fetching address")?;
                Some(
                    db.update_address_for_payments(
                        address,
                        address_update,
                        payment_id.to_string(),
                        merchant_key_store,
                        storage_scheme,
                    )
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::AddressNotFound)?,
                )
            }
            None => Some(
                db.find_address_by_merchant_id_payment_id_address_id(
                    merchant_id,
                    payment_id,
                    id,
                    merchant_key_store,
                    storage_scheme,
                )
                .await,
            )
            .transpose()
            .to_not_found_response(errors::ApiErrorResponse::AddressNotFound)?,
        },
        None => match req_address {
            Some(address) => {
                // generate a new address here
                let address_details = address.address.clone().unwrap_or_default();
                Some(
                    db.insert_address_for_payments(
                        payment_id,
                        get_domain_address_for_payments(
                            address_details,
                            address,
                            merchant_id,
                            customer_id,
                            payment_id,
                            key,
                            storage_scheme,
                        )
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed while encrypting address while insert")?,
                        merchant_key_store,
                        storage_scheme,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed while inserting new address")?,
                )
            }
            None => None,
        },
    })
}

#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn create_or_find_address_for_payment_by_request(
    db: &dyn StorageInterface,
    req_address: Option<&api::Address>,
    address_id: Option<&str>,
    merchant_id: &str,
    customer_id: Option<&String>,
    merchant_key_store: &domain::MerchantKeyStore,
    payment_id: &str,
    storage_scheme: storage_enums::MerchantStorageScheme,
) -> CustomResult<Option<domain::Address>, errors::ApiErrorResponse> {
    let key = merchant_key_store.key.get_inner().peek();

    Ok(match address_id {
        Some(id) => Some(
            db.find_address_by_merchant_id_payment_id_address_id(
                merchant_id,
                payment_id,
                id,
                merchant_key_store,
                storage_scheme,
            )
            .await,
        )
        .transpose()
        .to_not_found_response(errors::ApiErrorResponse::AddressNotFound)?,
        None => match req_address {
            Some(address) => {
                // generate a new address here

                let address_details = address.address.clone().unwrap_or_default();
                Some(
                    db.insert_address_for_payments(
                        payment_id,
                        get_domain_address_for_payments(
                            address_details,
                            address,
                            merchant_id,
                            customer_id,
                            payment_id,
                            key,
                            storage_scheme,
                        )
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed while encrypting address while insert")?,
                        merchant_key_store,
                        storage_scheme,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed while inserting new address")?,
                )
            }
            None => None,
        },
    })
}

pub async fn get_domain_address_for_payments(
    address_details: api_models::payments::AddressDetails,
    address: &api_models::payments::Address,
    merchant_id: &str,
    customer_id: Option<&String>,
    payment_id: &str,
    key: &[u8],
    storage_scheme: enums::MerchantStorageScheme,
) -> CustomResult<domain::Address, common_utils::errors::CryptoError> {
    async {
        Ok(domain::Address {
            id: None,
            phone_number: address
                .phone
                .as_ref()
                .and_then(|a| a.number.clone())
                .async_lift(|inner| types::encrypt_optional(inner, key))
                .await?,
            country_code: address.phone.as_ref().and_then(|a| a.country_code.clone()),
            customer_id: customer_id.cloned(),
            merchant_id: merchant_id.to_string(),
            address_id: generate_id(consts::ID_LENGTH, "add"),
            city: address_details.city,
            country: address_details.country,
            line1: address_details
                .line1
                .async_lift(|inner| types::encrypt_optional(inner, key))
                .await?,
            line2: address_details
                .line2
                .async_lift(|inner| types::encrypt_optional(inner, key))
                .await?,
            line3: address_details
                .line3
                .async_lift(|inner| types::encrypt_optional(inner, key))
                .await?,
            state: address_details
                .state
                .async_lift(|inner| types::encrypt_optional(inner, key))
                .await?,
            created_at: common_utils::date_time::now(),
            first_name: address_details
                .first_name
                .async_lift(|inner| types::encrypt_optional(inner, key))
                .await?,
            last_name: address_details
                .last_name
                .async_lift(|inner| types::encrypt_optional(inner, key))
                .await?,
            modified_at: common_utils::date_time::now(),
            zip: address_details
                .zip
                .async_lift(|inner| types::encrypt_optional(inner, key))
                .await?,
            payment_id: Some(payment_id.to_owned()),
            updated_by: storage_scheme.to_string(),
        })
    }
    .await
}

pub async fn get_address_by_id(
    db: &dyn StorageInterface,
    address_id: Option<String>,
    merchant_key_store: &domain::MerchantKeyStore,
    payment_id: String,
    merchant_id: String,
    storage_scheme: storage_enums::MerchantStorageScheme,
) -> CustomResult<Option<domain::Address>, errors::ApiErrorResponse> {
    match address_id {
        None => Ok(None),
        Some(address_id) => Ok(db
            .find_address_by_merchant_id_payment_id_address_id(
                &merchant_id,
                &payment_id,
                &address_id,
                merchant_key_store,
                storage_scheme,
            )
            .await
            .ok()),
    }
}

pub async fn get_token_pm_type_mandate_details(
    state: &AppState,
    request: &api::PaymentsRequest,
    mandate_type: Option<api::MandateTransactionType>,
    merchant_account: &domain::MerchantAccount,
) -> RouterResult<(
    Option<String>,
    Option<storage_enums::PaymentMethod>,
    Option<storage_enums::PaymentMethodType>,
    Option<MandateData>,
    Option<payments::RecurringMandatePaymentData>,
    Option<payments::MandateConnectorDetails>,
)> {
    let mandate_data = request.mandate_data.clone().map(MandateData::foreign_from);
    match mandate_type {
        Some(api::MandateTransactionType::NewMandateTransaction) => {
            let setup_mandate = mandate_data.clone().get_required_value("mandate_data")?;
            Ok((
                request.payment_token.to_owned(),
                request.payment_method,
                request.payment_method_type,
                Some(setup_mandate),
                None,
                None,
            ))
        }
        Some(api::MandateTransactionType::RecurringMandateTransaction) => {
            let (
                token_,
                payment_method_,
                recurring_mandate_payment_data,
                payment_method_type_,
                mandate_connector,
            ) = get_token_for_recurring_mandate(state, request, merchant_account).await?;
            Ok((
                token_,
                payment_method_,
                payment_method_type_.or(request.payment_method_type),
                None,
                recurring_mandate_payment_data,
                mandate_connector,
            ))
        }
        None => Ok((
            request.payment_token.to_owned(),
            request.payment_method,
            request.payment_method_type,
            mandate_data,
            None,
            None,
        )),
    }
}

pub async fn get_token_for_recurring_mandate(
    state: &AppState,
    req: &api::PaymentsRequest,
    merchant_account: &domain::MerchantAccount,
) -> RouterResult<(
    Option<String>,
    Option<storage_enums::PaymentMethod>,
    Option<payments::RecurringMandatePaymentData>,
    Option<storage_enums::PaymentMethodType>,
    Option<payments::MandateConnectorDetails>,
)> {
    let db = &*state.store;
    let mandate_id = req.mandate_id.clone().get_required_value("mandate_id")?;

    let mandate = db
        .find_mandate_by_merchant_id_mandate_id(&merchant_account.merchant_id, mandate_id.as_str())
        .await
        .to_not_found_response(errors::ApiErrorResponse::MandateNotFound)?;

    let customer = req.customer_id.clone().get_required_value("customer_id")?;

    let payment_method_id = {
        if mandate.customer_id != customer {
            Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                message: "customer_id must match mandate customer_id".into()
            }))?
        }
        if mandate.mandate_status != storage_enums::MandateStatus::Active {
            Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                message: "mandate is not active".into()
            }))?
        };
        mandate.payment_method_id.clone()
    };
    verify_mandate_details(
        req.amount.get_required_value("amount")?.into(),
        req.currency.get_required_value("currency")?,
        mandate.clone(),
    )?;

    let payment_method = db
        .find_payment_method(payment_method_id.as_str())
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

    let token = Uuid::new_v4().to_string();
    let payment_method_type = payment_method.payment_method_type;
    let mandate_connector_details = payments::MandateConnectorDetails {
        connector: mandate.connector,
        merchant_connector_id: mandate.merchant_connector_id,
    };

    if let diesel_models::enums::PaymentMethod::Card = payment_method.payment_method {
        let _ = cards::get_lookup_key_from_locker(state, &token, &payment_method).await?;
        if let Some(payment_method_from_request) = req.payment_method {
            let pm: storage_enums::PaymentMethod = payment_method_from_request;
            if pm != payment_method.payment_method {
                Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                    message:
                        "payment method in request does not match previously provided payment \
                                  method information"
                            .into()
                }))?
            }
        };

        Ok((
            Some(token),
            Some(payment_method.payment_method),
            Some(payments::RecurringMandatePaymentData {
                payment_method_type,
            }),
            payment_method.payment_method_type,
            Some(mandate_connector_details),
        ))
    } else {
        Ok((
            None,
            Some(payment_method.payment_method),
            Some(payments::RecurringMandatePaymentData {
                payment_method_type,
            }),
            payment_method.payment_method_type,
            Some(mandate_connector_details),
        ))
    }
}

#[instrument(skip_all)]
/// Check weather the merchant id in the request
/// and merchant id in the merchant account are same.
pub fn validate_merchant_id(
    merchant_id: &str,
    request_merchant_id: Option<&str>,
) -> CustomResult<(), errors::ApiErrorResponse> {
    // Get Merchant Id from the merchant
    // or get from merchant account

    let request_merchant_id = request_merchant_id.unwrap_or(merchant_id);

    utils::when(merchant_id.ne(request_merchant_id), || {
        Err(report!(errors::ApiErrorResponse::PreconditionFailed {
            message: format!(
                "Invalid `merchant_id`: {request_merchant_id} not found in merchant account"
            )
        }))
    })
}

#[instrument(skip_all)]
pub fn validate_request_amount_and_amount_to_capture(
    op_amount: Option<api::Amount>,
    op_amount_to_capture: Option<i64>,
) -> CustomResult<(), errors::ApiErrorResponse> {
    match (op_amount, op_amount_to_capture) {
        (None, _) => Ok(()),
        (Some(_amount), None) => Ok(()),
        (Some(amount), Some(amount_to_capture)) => {
            match amount {
                api::Amount::Value(amount_inner) => {
                    // If both amount and amount to capture is present
                    // then amount to be capture should be less than or equal to request amount
                    utils::when(!amount_to_capture.le(&amount_inner.get()), || {
                        Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                            message: format!(
                            "amount_to_capture is greater than amount capture_amount: {amount_to_capture:?} request_amount: {amount:?}"
                        )
                        }))
                    })
                }
                api::Amount::Zero => {
                    // If the amount is Null but still amount_to_capture is passed this is invalid and
                    Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                        message: "amount_to_capture should not exist for when amount = 0"
                            .to_string()
                    }))
                }
            }
        }
    }
}

#[instrument(skip_all)]
pub fn validate_card_data(
    payment_method_data: Option<api::PaymentMethodData>,
) -> CustomResult<(), errors::ApiErrorResponse> {
    if let Some(api::PaymentMethodData::Card(card)) = payment_method_data {
        let cvc = card.card_cvc.peek().to_string();
        if cvc.len() < 3 || cvc.len() > 4 {
            Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                message: "Invalid card_cvc length".to_string()
            }))?
        }
        let card_cvc = cvc.parse::<u16>().into_report().change_context(
            errors::ApiErrorResponse::InvalidDataValue {
                field_name: "card_cvc",
            },
        )?;
        ::cards::CardSecurityCode::try_from(card_cvc).change_context(
            errors::ApiErrorResponse::PreconditionFailed {
                message: "Invalid Card CVC".to_string(),
            },
        )?;

        let exp_month = card
            .card_exp_month
            .peek()
            .to_string()
            .parse::<u8>()
            .into_report()
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "card_exp_month",
            })?;
        let month = ::cards::CardExpirationMonth::try_from(exp_month).change_context(
            errors::ApiErrorResponse::PreconditionFailed {
                message: "Invalid Expiry Month".to_string(),
            },
        )?;
        let mut year_str = card.card_exp_year.peek().to_string();
        if year_str.len() == 2 {
            year_str = format!("20{}", year_str);
        }
        let exp_year = year_str.parse::<u16>().into_report().change_context(
            errors::ApiErrorResponse::InvalidDataValue {
                field_name: "card_exp_year",
            },
        )?;
        let year = ::cards::CardExpirationYear::try_from(exp_year).change_context(
            errors::ApiErrorResponse::PreconditionFailed {
                message: "Invalid Expiry Year".to_string(),
            },
        )?;

        let card_expiration = ::cards::CardExpiration { month, year };
        let is_expired = card_expiration.is_expired().change_context(
            errors::ApiErrorResponse::PreconditionFailed {
                message: "Invalid card data".to_string(),
            },
        )?;
        if is_expired {
            Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                message: "Card Expired".to_string()
            }))?
        }
    }
    Ok(())
}

pub fn infer_payment_type(
    amount: &api::Amount,
    mandate_type: Option<&api::MandateTransactionType>,
) -> api_enums::PaymentType {
    match mandate_type {
        Some(api::MandateTransactionType::NewMandateTransaction) => {
            if let api::Amount::Value(_) = amount {
                api_enums::PaymentType::NewMandate
            } else {
                api_enums::PaymentType::SetupMandate
            }
        }

        Some(api::MandateTransactionType::RecurringMandateTransaction) => {
            api_enums::PaymentType::RecurringMandate
        }

        None => api_enums::PaymentType::Normal,
    }
}

pub fn validate_mandate(
    req: impl Into<api::MandateValidationFields>,
    is_confirm_operation: bool,
) -> CustomResult<Option<api::MandateTransactionType>, errors::ApiErrorResponse> {
    let req: api::MandateValidationFields = req.into();
    match req.validate_and_get_mandate_type().change_context(
        errors::ApiErrorResponse::MandateValidationFailed {
            reason: "Expected one out of mandate_id and mandate_data but got both".to_string(),
        },
    )? {
        Some(api::MandateTransactionType::NewMandateTransaction) => {
            validate_new_mandate_request(req, is_confirm_operation)?;
            Ok(Some(api::MandateTransactionType::NewMandateTransaction))
        }
        Some(api::MandateTransactionType::RecurringMandateTransaction) => {
            validate_recurring_mandate(req)?;
            Ok(Some(
                api::MandateTransactionType::RecurringMandateTransaction,
            ))
        }
        None => Ok(None),
    }
}

fn validate_new_mandate_request(
    req: api::MandateValidationFields,
    is_confirm_operation: bool,
) -> RouterResult<()> {
    // We need not check for customer_id in the confirm request if it is already passed
    // in create request

    fp_utils::when(!is_confirm_operation && req.customer_id.is_none(), || {
        Err(report!(errors::ApiErrorResponse::PreconditionFailed {
            message: "`customer_id` is mandatory for mandates".into()
        }))
    })?;

    let mandate_data = req
        .mandate_data
        .clone()
        .get_required_value("mandate_data")?;

    if api_enums::FutureUsage::OnSession
        == req
            .setup_future_usage
            .get_required_value("setup_future_usage")?
    {
        Err(report!(errors::ApiErrorResponse::PreconditionFailed {
            message: "`setup_future_usage` must be `off_session` for mandates".into()
        }))?
    };

    // Only use this validation if the customer_acceptance is present
    if mandate_data
        .customer_acceptance
        .map(|inner| inner.acceptance_type == api::AcceptanceType::Online && inner.online.is_none())
        .unwrap_or(false)
    {
        Err(report!(errors::ApiErrorResponse::PreconditionFailed {
            message: "`mandate_data.customer_acceptance.online` is required when \
                      `mandate_data.customer_acceptance.acceptance_type` is `online`"
                .into()
        }))?
    }

    let mandate_details = match mandate_data.mandate_type {
        Some(api_models::payments::MandateType::SingleUse(details)) => Some(details),
        Some(api_models::payments::MandateType::MultiUse(details)) => details,
        None => None,
    };
    mandate_details.and_then(|md| md.start_date.zip(md.end_date)).map(|(start_date, end_date)|
        utils::when (start_date >= end_date, || {
        Err(report!(errors::ApiErrorResponse::PreconditionFailed {
            message: "`mandate_data.mandate_type.{multi_use|single_use}.start_date` should be greater than  \
            `mandate_data.mandate_type.{multi_use|single_use}.end_date`"
                .into()
        }))
    })).transpose()?;

    Ok(())
}

pub fn validate_customer_id_mandatory_cases(
    has_setup_future_usage: bool,
    customer_id: &Option<String>,
) -> RouterResult<()> {
    match (has_setup_future_usage, customer_id) {
        (true, None) => Err(errors::ApiErrorResponse::PreconditionFailed {
            message: "customer_id is mandatory when setup_future_usage is given".to_string(),
        })
        .into_report(),
        _ => Ok(()),
    }
}

pub fn create_startpay_url(
    server: &Server,
    payment_attempt: &PaymentAttempt,
    payment_intent: &PaymentIntent,
) -> String {
    format!(
        "{}/payments/redirect/{}/{}/{}",
        server.base_url,
        payment_intent.payment_id,
        payment_intent.merchant_id,
        payment_attempt.attempt_id
    )
}

pub fn create_redirect_url(
    router_base_url: &String,
    payment_attempt: &PaymentAttempt,
    connector_name: &String,
    creds_identifier: Option<&str>,
) -> String {
    let creds_identifier_path = creds_identifier.map_or_else(String::new, |cd| format!("/{}", cd));
    format!(
        "{}/payments/{}/{}/redirect/response/{}",
        router_base_url, payment_attempt.payment_id, payment_attempt.merchant_id, connector_name,
    ) + creds_identifier_path.as_ref()
}

pub fn create_webhook_url(
    router_base_url: &String,
    merchant_id: &String,
    connector_name: &String,
) -> String {
    format!(
        "{}/webhooks/{}/{}",
        router_base_url, merchant_id, connector_name
    )
}
pub fn create_complete_authorize_url(
    router_base_url: &String,
    payment_attempt: &PaymentAttempt,
    connector_name: &String,
) -> String {
    format!(
        "{}/payments/{}/{}/redirect/complete/{}",
        router_base_url, payment_attempt.payment_id, payment_attempt.merchant_id, connector_name
    )
}

fn validate_recurring_mandate(req: api::MandateValidationFields) -> RouterResult<()> {
    req.mandate_id.check_value_present("mandate_id")?;

    req.customer_id.check_value_present("customer_id")?;

    let confirm = req.confirm.get_required_value("confirm")?;
    if !confirm {
        Err(report!(errors::ApiErrorResponse::PreconditionFailed {
            message: "`confirm` must be `true` for mandates".into()
        }))?
    }

    let off_session = req.off_session.get_required_value("off_session")?;
    if !off_session {
        Err(report!(errors::ApiErrorResponse::PreconditionFailed {
            message: "`off_session` should be `true` for mandates".into()
        }))?
    }

    Ok(())
}

pub fn verify_mandate_details(
    request_amount: i64,
    request_currency: api_enums::Currency,
    mandate: storage::Mandate,
) -> RouterResult<()> {
    match mandate.mandate_type {
        storage_enums::MandateType::SingleUse => utils::when(
            mandate
                .mandate_amount
                .map(|mandate_amount| request_amount > mandate_amount)
                .unwrap_or(true),
            || {
                Err(report!(errors::ApiErrorResponse::MandateValidationFailed {
                    reason: "request amount is greater than mandate amount".to_string()
                }))
            },
        ),
        storage::enums::MandateType::MultiUse => utils::when(
            mandate
                .mandate_amount
                .map(|mandate_amount| {
                    (mandate.amount_captured.unwrap_or(0) + request_amount) > mandate_amount
                })
                .unwrap_or(false),
            || {
                Err(report!(errors::ApiErrorResponse::MandateValidationFailed {
                    reason: "request amount is greater than mandate amount".to_string()
                }))
            },
        ),
    }?;
    utils::when(
        mandate
            .mandate_currency
            .map(|mandate_currency| mandate_currency != request_currency)
            .unwrap_or(false),
        || {
            Err(report!(errors::ApiErrorResponse::MandateValidationFailed {
                reason: "cross currency mandates not supported".to_string()
            }))
        },
    )
}

#[instrument(skip_all)]
pub fn payment_attempt_status_fsm(
    payment_method_data: &Option<api::PaymentMethodData>,
    confirm: Option<bool>,
) -> storage_enums::AttemptStatus {
    match payment_method_data {
        Some(_) => match confirm {
            Some(true) => storage_enums::AttemptStatus::Pending,
            _ => storage_enums::AttemptStatus::ConfirmationAwaited,
        },
        None => storage_enums::AttemptStatus::PaymentMethodAwaited,
    }
}

pub fn payment_intent_status_fsm(
    payment_method_data: &Option<api::PaymentMethodData>,
    confirm: Option<bool>,
) -> storage_enums::IntentStatus {
    match payment_method_data {
        Some(_) => match confirm {
            Some(true) => storage_enums::IntentStatus::RequiresCustomerAction,
            _ => storage_enums::IntentStatus::RequiresConfirmation,
        },
        None => storage_enums::IntentStatus::RequiresPaymentMethod,
    }
}

pub async fn add_domain_task_to_pt<Op>(
    operation: &Op,
    state: &AppState,
    payment_attempt: &PaymentAttempt,
    requeue: bool,
    schedule_time: Option<time::PrimitiveDateTime>,
) -> CustomResult<(), errors::ApiErrorResponse>
where
    Op: std::fmt::Debug,
{
    if check_if_operation_confirm(operation) {
        match schedule_time {
            Some(stime) => {
                if !requeue {
                    // scheduler_metrics::TASKS_ADDED_COUNT.add(&metrics::CONTEXT, 1, &[]); // Metrics
                    super::add_process_sync_task(&*state.store, payment_attempt, stime)
                        .await
                        .into_report()
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed while adding task to process tracker")
                } else {
                    // scheduler_metrics::TASKS_RESET_COUNT.add(&metrics::CONTEXT, 1, &[]); // Metrics
                    super::reset_process_sync_task(&*state.store, payment_attempt, stime)
                        .await
                        .into_report()
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed while updating task in process tracker")
                }
            }
            None => Ok(()),
        }
    } else {
        Ok(())
    }
}

pub fn response_operation<'a, F, R, Ctx>() -> BoxedOperation<'a, F, R, Ctx>
where
    F: Send + Clone,
    Ctx: PaymentMethodRetrieve,
    PaymentResponse: Operation<F, R, Ctx>,
{
    Box::new(PaymentResponse)
}

#[instrument(skip_all)]
pub(crate) async fn get_payment_method_create_request(
    payment_method_data: Option<&api::PaymentMethodData>,
    payment_method: Option<storage_enums::PaymentMethod>,
    payment_method_type: Option<storage_enums::PaymentMethodType>,
    customer: &domain::Customer,
) -> RouterResult<api::PaymentMethodCreate> {
    match payment_method_data {
        Some(pm_data) => match payment_method {
            Some(payment_method) => match pm_data {
                api::PaymentMethodData::Card(card) => {
                    let card_detail = api::CardDetail {
                        card_number: card.card_number.clone(),
                        card_exp_month: card.card_exp_month.clone(),
                        card_exp_year: card.card_exp_year.clone(),
                        card_holder_name: Some(card.card_holder_name.clone()),
                        nick_name: card.nick_name.clone(),
                    };
                    let customer_id = customer.customer_id.clone();
                    let payment_method_request = api::PaymentMethodCreate {
                        payment_method,
                        payment_method_type,
                        payment_method_issuer: card.card_issuer.clone(),
                        payment_method_issuer_code: None,
                        card: Some(card_detail),
                        metadata: None,
                        customer_id: Some(customer_id),
                        card_network: card
                            .card_network
                            .as_ref()
                            .map(|card_network| card_network.to_string()),
                    };
                    Ok(payment_method_request)
                }
                _ => {
                    let payment_method_request = api::PaymentMethodCreate {
                        payment_method,
                        payment_method_type,
                        payment_method_issuer: None,
                        payment_method_issuer_code: None,
                        card: None,
                        metadata: None,
                        customer_id: Some(customer.customer_id.to_owned()),
                        card_network: None,
                    };
                    Ok(payment_method_request)
                }
            },
            None => Err(report!(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "payment_method_type"
            })
            .attach_printable("PaymentMethodType Required")),
        },
        None => Err(report!(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "payment_method_data"
        })
        .attach_printable("PaymentMethodData required Or Card is already saved")),
    }
}

pub async fn get_customer_from_details<F: Clone>(
    db: &dyn StorageInterface,
    customer_id: Option<String>,
    merchant_id: &str,
    payment_data: &mut PaymentData<F>,
    merchant_key_store: &domain::MerchantKeyStore,
) -> CustomResult<Option<domain::Customer>, errors::StorageError> {
    match customer_id {
        None => Ok(None),
        Some(c_id) => {
            let customer = db
                .find_customer_optional_by_customer_id_merchant_id(
                    &c_id,
                    merchant_id,
                    merchant_key_store,
                )
                .await?;
            payment_data.email = payment_data.email.clone().or_else(|| {
                customer.as_ref().and_then(|inner| {
                    inner
                        .email
                        .clone()
                        .map(|encrypted_value| encrypted_value.into())
                })
            });
            Ok(customer)
        }
    }
}

// Checks if the inner values of two options are not equal and throws appropriate error
fn validate_options_for_inequality<T: PartialEq>(
    first_option: Option<&T>,
    second_option: Option<&T>,
    field_name: &str,
) -> Result<(), errors::ApiErrorResponse> {
    fp_utils::when(
        first_option
            .zip(second_option)
            .map(|(value1, value2)| value1 != value2)
            .unwrap_or(false),
        || {
            Err(errors::ApiErrorResponse::PreconditionFailed {
                message: format!("The field name `{field_name}` sent in both places is ambiguous"),
            })
        },
    )
}

// Checks if the customer details are passed in both places
// If so, raise an error
pub fn validate_customer_details_in_request(
    request: &api_models::payments::PaymentsRequest,
) -> Result<(), errors::ApiErrorResponse> {
    if let Some(customer_details) = request.customer.as_ref() {
        validate_options_for_inequality(
            request.customer_id.as_ref(),
            Some(&customer_details.id),
            "customer_id",
        )?;

        validate_options_for_inequality(
            request.email.as_ref(),
            customer_details.email.as_ref(),
            "email",
        )?;

        validate_options_for_inequality(
            request.name.as_ref(),
            customer_details.name.as_ref(),
            "name",
        )?;

        validate_options_for_inequality(
            request.phone.as_ref(),
            customer_details.phone.as_ref(),
            "phone",
        )?;

        validate_options_for_inequality(
            request.phone_country_code.as_ref(),
            customer_details.phone_country_code.as_ref(),
            "phone_country_code",
        )?;
    }

    Ok(())
}

/// Get the customer details from customer field if present
/// or from the individual fields in `PaymentsRequest`
#[instrument(skip_all)]
pub fn get_customer_details_from_request(
    request: &api_models::payments::PaymentsRequest,
) -> CustomerDetails {
    let customer_id = request
        .customer
        .as_ref()
        .map(|customer_details| customer_details.id.clone())
        .or(request.customer_id.clone());

    let customer_name = request
        .customer
        .as_ref()
        .and_then(|customer_details| customer_details.name.clone())
        .or(request.name.clone());

    let customer_email = request
        .customer
        .as_ref()
        .and_then(|customer_details| customer_details.email.clone())
        .or(request.email.clone());

    let customer_phone = request
        .customer
        .as_ref()
        .and_then(|customer_details| customer_details.phone.clone())
        .or(request.phone.clone());

    let customer_phone_code = request
        .customer
        .as_ref()
        .and_then(|customer_details| customer_details.phone_country_code.clone())
        .or(request.phone_country_code.clone());

    CustomerDetails {
        customer_id,
        name: customer_name,
        email: customer_email,
        phone: customer_phone,
        phone_country_code: customer_phone_code,
    }
}

pub async fn get_connector_default(
    _state: &AppState,
    request_connector: Option<serde_json::Value>,
) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
    Ok(request_connector.map_or(
        api::ConnectorChoice::Decide,
        api::ConnectorChoice::StraightThrough,
    ))
}

#[instrument(skip_all)]
pub async fn create_customer_if_not_exist<'a, F: Clone, R, Ctx>(
    operation: BoxedOperation<'a, F, R, Ctx>,
    db: &dyn StorageInterface,
    payment_data: &mut PaymentData<F>,
    req: Option<CustomerDetails>,
    merchant_id: &str,
    key_store: &domain::MerchantKeyStore,
) -> CustomResult<(BoxedOperation<'a, F, R, Ctx>, Option<domain::Customer>), errors::StorageError> {
    let request_customer_details = req
        .get_required_value("customer")
        .change_context(errors::StorageError::ValueNotFound("customer".to_owned()))?;

    let customer_id = request_customer_details
        .customer_id
        .or(payment_data.payment_intent.customer_id.clone());

    let optional_customer = match customer_id {
        Some(customer_id) => {
            let customer_data = db
                .find_customer_optional_by_customer_id_merchant_id(
                    &customer_id,
                    merchant_id,
                    key_store,
                )
                .await?;

            Some(match customer_data {
                Some(c) => {
                    // Update the customer data if new data is passed in the request
                    if request_customer_details.email.is_some()
                        | request_customer_details.name.is_some()
                        | request_customer_details.phone.is_some()
                        | request_customer_details.phone_country_code.is_some()
                    {
                        let key = key_store.key.get_inner().peek();
                        let customer_update = async {
                            Ok(Update {
                                name: request_customer_details
                                    .name
                                    .async_lift(|inner| types::encrypt_optional(inner, key))
                                    .await?,
                                email: request_customer_details
                                    .email
                                    .clone()
                                    .async_lift(|inner| {
                                        types::encrypt_optional(
                                            inner.map(|inner| inner.expose()),
                                            key,
                                        )
                                    })
                                    .await?,
                                phone: Box::new(
                                    request_customer_details
                                        .phone
                                        .clone()
                                        .async_lift(|inner| types::encrypt_optional(inner, key))
                                        .await?,
                                ),
                                phone_country_code: request_customer_details.phone_country_code,
                                description: None,
                                connector_customer: None,
                                metadata: None,
                                address_id: None,
                            })
                        }
                        .await
                        .change_context(errors::StorageError::SerializationFailed)
                        .attach_printable("Failed while encrypting Customer while Update")?;

                        db.update_customer_by_customer_id_merchant_id(
                            customer_id,
                            merchant_id.to_string(),
                            customer_update,
                            key_store,
                        )
                        .await
                    } else {
                        Ok(c)
                    }
                }
                None => {
                    let new_customer = async {
                        let key = key_store.key.get_inner().peek();
                        Ok(domain::Customer {
                            customer_id: customer_id.to_string(),
                            merchant_id: merchant_id.to_string(),
                            name: request_customer_details
                                .name
                                .async_lift(|inner| types::encrypt_optional(inner, key))
                                .await?,
                            email: request_customer_details
                                .email
                                .clone()
                                .async_lift(|inner| {
                                    types::encrypt_optional(inner.map(|inner| inner.expose()), key)
                                })
                                .await?,
                            phone: request_customer_details
                                .phone
                                .clone()
                                .async_lift(|inner| types::encrypt_optional(inner, key))
                                .await?,
                            phone_country_code: request_customer_details.phone_country_code.clone(),
                            description: None,
                            created_at: common_utils::date_time::now(),
                            id: None,
                            metadata: None,
                            modified_at: common_utils::date_time::now(),
                            connector_customer: None,
                            address_id: None,
                        })
                    }
                    .await
                    .change_context(errors::StorageError::SerializationFailed)
                    .attach_printable("Failed while encrypting Customer while insert")?;
                    metrics::CUSTOMER_CREATED.add(&metrics::CONTEXT, 1, &[]);
                    db.insert_customer(new_customer, key_store).await
                }
            })
        }
        None => match &payment_data.payment_intent.customer_id {
            None => None,
            Some(customer_id) => db
                .find_customer_optional_by_customer_id_merchant_id(
                    customer_id,
                    merchant_id,
                    key_store,
                )
                .await?
                .map(Ok),
        },
    };
    Ok((
        operation,
        match optional_customer {
            Some(customer) => {
                let customer = customer?;

                payment_data.payment_intent.customer_id = Some(customer.customer_id.clone());
                payment_data.email = payment_data.email.clone().or_else(|| {
                    customer
                        .email
                        .clone()
                        .map(|encrypted_value| encrypted_value.into())
                });

                Some(customer)
            }
            None => None,
        },
    ))
}

pub async fn make_pm_data<'a, F: Clone, R, Ctx: PaymentMethodRetrieve>(
    operation: BoxedOperation<'a, F, R, Ctx>,
    state: &'a AppState,
    payment_data: &mut PaymentData<F>,
) -> RouterResult<(
    BoxedOperation<'a, F, R, Ctx>,
    Option<api::PaymentMethodData>,
)> {
    let request = &payment_data.payment_method_data.clone();
    let token = payment_data.token.clone();

    let hyperswitch_token = match payment_data.mandate_id {
        Some(_) => token,
        None => {
            if let Some(token) = token {
                let redis_conn = state
                    .store
                    .get_redis_conn()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to get redis connection")?;

                let key = format!(
                    "pm_token_{}_{}_hyperswitch",
                    token,
                    payment_data
                        .payment_attempt
                        .payment_method
                        .to_owned()
                        .get_required_value("payment_method")?,
                );

                let key = redis_conn
                    .get_key::<Option<String>>(&key)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to fetch the token from redis")?
                    .ok_or(error_stack::Report::new(
                        errors::ApiErrorResponse::UnprocessableEntity {
                            message: "Token is invalid or expired".to_owned(),
                        },
                    ))?;

                Some(key)
            } else {
                None
            }
        }
    };

    let card_cvc = payment_data.card_cvc.clone();

    // TODO: Handle case where payment method and token both are present in request properly.
    let payment_method = match (request, hyperswitch_token) {
        (_, Some(hyperswitch_token)) => {
            let (pm, supplementary_data) = vault::Vault::get_payment_method_data_from_locker(
                state,
                &hyperswitch_token,
            )
            .await
            .attach_printable(
                "Payment method for given token not found or there was a problem fetching it",
            )?;

            utils::when(
                supplementary_data
                    .customer_id
                    .ne(&payment_data.payment_intent.customer_id),
                || {
                    Err(errors::ApiErrorResponse::PreconditionFailed { message: "customer associated with payment method and customer passed in payment are not same".into() })
                },
            )?;

            Ok::<_, error_stack::Report<errors::ApiErrorResponse>>(match pm.clone() {
                Some(api::PaymentMethodData::Card(card)) => {
                    payment_data.payment_attempt.payment_method =
                        Some(storage_enums::PaymentMethod::Card);
                    if let Some(cvc) = card_cvc {
                        let mut updated_card = card;
                        updated_card.card_cvc = cvc;
                        let updated_pm = api::PaymentMethodData::Card(updated_card);
                        vault::Vault::store_payment_method_data_in_locker(
                            state,
                            Some(hyperswitch_token),
                            &updated_pm,
                            payment_data.payment_intent.customer_id.to_owned(),
                            enums::PaymentMethod::Card,
                        )
                        .await?;
                        Some(updated_pm)
                    } else {
                        pm
                    }
                }

                Some(api::PaymentMethodData::Wallet(_)) => {
                    payment_data.payment_attempt.payment_method =
                        Some(storage_enums::PaymentMethod::Wallet);
                    pm
                }

                Some(api::PaymentMethodData::BankTransfer(_)) => {
                    payment_data.payment_attempt.payment_method =
                        Some(storage_enums::PaymentMethod::BankTransfer);
                    pm
                }
                Some(api::PaymentMethodData::BankRedirect(_)) => {
                    payment_data.payment_attempt.payment_method =
                        Some(storage_enums::PaymentMethod::BankRedirect);
                    pm
                }
                Some(_) => Err(errors::ApiErrorResponse::InternalServerError)
                    .into_report()
                    .attach_printable(
                        "Payment method received from locker is unsupported by locker",
                    )?,

                None => None,
            })
        }

        (Some(_), _) => {
            let payment_method_data = Ctx::retrieve_payment_method(
                request,
                state,
                &payment_data.payment_intent,
                &payment_data.payment_attempt,
            )
            .await?;

            payment_data.token = payment_method_data.1;

            Ok(payment_method_data.0)
        }
        _ => Ok(None),
    }?;

    Ok((operation, payment_method))
}

pub async fn store_in_vault_and_generate_ppmt(
    state: &AppState,
    payment_method_data: &api_models::payments::PaymentMethodData,
    payment_intent: &PaymentIntent,
    payment_attempt: &PaymentAttempt,
    payment_method: enums::PaymentMethod,
) -> RouterResult<String> {
    let router_token = vault::Vault::store_payment_method_data_in_locker(
        state,
        None,
        payment_method_data,
        payment_intent.customer_id.to_owned(),
        payment_method,
    )
    .await?;
    let parent_payment_method_token = generate_id(consts::ID_LENGTH, "token");
    let key_for_hyperswitch_token = payment_attempt.payment_method.map(|payment_method| {
        payment_methods::ParentPaymentMethodToken::create_key_for_token((
            &parent_payment_method_token,
            payment_method,
        ))
    });
    if let Some(key_for_hyperswitch_token) = key_for_hyperswitch_token {
        key_for_hyperswitch_token
            .insert(Some(payment_intent.created_at), router_token, state)
            .await?;
    };
    Ok(parent_payment_method_token)
}

pub async fn store_payment_method_data_in_vault(
    state: &AppState,
    payment_attempt: &PaymentAttempt,
    payment_intent: &PaymentIntent,
    payment_method: enums::PaymentMethod,
    payment_method_data: &api::PaymentMethodData,
) -> RouterResult<Option<String>> {
    if should_store_payment_method_data_in_vault(
        &state.conf.temp_locker_enable_config,
        payment_attempt.connector.clone(),
        payment_method,
    ) {
        let parent_payment_method_token = store_in_vault_and_generate_ppmt(
            state,
            payment_method_data,
            payment_intent,
            payment_attempt,
            payment_method,
        )
        .await?;

        return Ok(Some(parent_payment_method_token));
    }

    Ok(None)
}
pub fn should_store_payment_method_data_in_vault(
    temp_locker_enable_config: &TempLockerEnableConfig,
    option_connector: Option<String>,
    payment_method: enums::PaymentMethod,
) -> bool {
    option_connector
        .map(|connector| {
            temp_locker_enable_config
                .0
                .get(&connector)
                .map(|config| config.payment_method.contains(&payment_method))
                .unwrap_or(false)
        })
        .unwrap_or(true)
}

#[instrument(skip_all)]
pub(crate) fn validate_capture_method(
    capture_method: storage_enums::CaptureMethod,
) -> RouterResult<()> {
    utils::when(
        capture_method == storage_enums::CaptureMethod::Automatic,
        || {
            Err(report!(errors::ApiErrorResponse::PaymentUnexpectedState {
                field_name: "capture_method".to_string(),
                current_flow: "captured".to_string(),
                current_value: capture_method.to_string(),
                states: "manual, manual_multiple, scheduled".to_string()
            }))
        },
    )
}

#[instrument(skip_all)]
pub(crate) fn validate_status_with_capture_method(
    status: storage_enums::IntentStatus,
    capture_method: storage_enums::CaptureMethod,
) -> RouterResult<()> {
    if status == storage_enums::IntentStatus::Processing
        && !(capture_method == storage_enums::CaptureMethod::ManualMultiple)
    {
        return Err(report!(errors::ApiErrorResponse::PaymentUnexpectedState {
            field_name: "capture_method".to_string(),
            current_flow: "captured".to_string(),
            current_value: capture_method.to_string(),
            states: "manual_multiple".to_string()
        }));
    }
    utils::when(
        status != storage_enums::IntentStatus::RequiresCapture
            && status != storage_enums::IntentStatus::PartiallyCaptured
            && status != storage_enums::IntentStatus::Processing,
        || {
            Err(report!(errors::ApiErrorResponse::PaymentUnexpectedState {
                field_name: "payment.status".to_string(),
                current_flow: "captured".to_string(),
                current_value: status.to_string(),
                states: "requires_capture, partially_captured, processing".to_string()
            }))
        },
    )
}

#[instrument(skip_all)]
pub(crate) fn validate_amount_to_capture(
    amount: i64,
    amount_to_capture: Option<i64>,
) -> RouterResult<()> {
    utils::when(
        amount_to_capture.is_some() && (Some(amount) < amount_to_capture),
        || {
            Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "amount_to_capture is greater than amount".to_string()
            }))
        },
    )
}

#[instrument(skip_all)]
pub(crate) fn validate_payment_method_fields_present(
    req: &api::PaymentsRequest,
) -> RouterResult<()> {
    utils::when(
        req.payment_method.is_none() && req.payment_method_data.is_some(),
        || {
            Err(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "payment_method",
            })
        },
    )?;

    utils::when(
        !matches!(
            req.payment_method,
            Some(api_enums::PaymentMethod::Card) | None
        ) && (req.payment_method_type.is_none()),
        || {
            Err(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "payment_method_type",
            })
        },
    )?;

    utils::when(
        req.payment_method.is_some()
            && req.payment_method_data.is_none()
            && req.payment_token.is_none(),
        || {
            Err(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "payment_method_data",
            })
        },
    )?;
    utils::when(
        req.payment_method.is_some() && req.payment_method_type.is_some(),
        || {
            req.payment_method
                .map_or(Ok(()), |req_payment_method| {
                    req.payment_method_type.map_or(Ok(()), |req_payment_method_type| {
                        if !validate_payment_method_type_against_payment_method(req_payment_method, req_payment_method_type) {
                            Err(errors::ApiErrorResponse::InvalidRequestData {
                                message: ("payment_method_type doesn't correspond to the specified payment_method"
                                    .to_string()),
                            })
                        } else {
                            Ok(())
                        }
                    })
                })
        },
    )?;

    let validate_payment_method_and_payment_method_data =
        |req_payment_method_data, req_payment_method: api_enums::PaymentMethod| {
            api_enums::PaymentMethod::foreign_try_from(req_payment_method_data).and_then(|payment_method|
                if req_payment_method != payment_method {
                    Err(errors::ApiErrorResponse::InvalidRequestData {
                        message: ("payment_method_data doesn't correspond to the specified payment_method"
                            .to_string()),
                    })
                } else {
                    Ok(())
                })
        };

    utils::when(
        req.payment_method.is_some() && req.payment_method_data.is_some(),
        || {
            req.payment_method_data
                .clone()
                .map_or(Ok(()), |req_payment_method_data| {
                    req.payment_method.map_or(Ok(()), |req_payment_method| {
                        validate_payment_method_and_payment_method_data(
                            req_payment_method_data,
                            req_payment_method,
                        )
                    })
                })
        },
    )?;

    Ok(())
}

pub fn validate_payment_method_type_against_payment_method(
    payment_method: api_enums::PaymentMethod,
    payment_method_type: api_enums::PaymentMethodType,
) -> bool {
    match payment_method {
        api_enums::PaymentMethod::Card => matches!(
            payment_method_type,
            api_enums::PaymentMethodType::Credit | api_enums::PaymentMethodType::Debit
        ),
        api_enums::PaymentMethod::PayLater => matches!(
            payment_method_type,
            api_enums::PaymentMethodType::Affirm
                | api_enums::PaymentMethodType::Alma
                | api_enums::PaymentMethodType::AfterpayClearpay
                | api_enums::PaymentMethodType::Klarna
                | api_enums::PaymentMethodType::PayBright
                | api_enums::PaymentMethodType::Atome
                | api_enums::PaymentMethodType::Walley
        ),
        api_enums::PaymentMethod::Wallet => matches!(
            payment_method_type,
            api_enums::PaymentMethodType::ApplePay
                | api_enums::PaymentMethodType::GooglePay
                | api_enums::PaymentMethodType::Paypal
                | api_enums::PaymentMethodType::AliPay
                | api_enums::PaymentMethodType::AliPayHk
                | api_enums::PaymentMethodType::Dana
                | api_enums::PaymentMethodType::MbWay
                | api_enums::PaymentMethodType::MobilePay
                | api_enums::PaymentMethodType::SamsungPay
                | api_enums::PaymentMethodType::Twint
                | api_enums::PaymentMethodType::Vipps
                | api_enums::PaymentMethodType::TouchNGo
                | api_enums::PaymentMethodType::Swish
                | api_enums::PaymentMethodType::WeChatPay
                | api_enums::PaymentMethodType::GoPay
                | api_enums::PaymentMethodType::Gcash
                | api_enums::PaymentMethodType::Momo
                | api_enums::PaymentMethodType::KakaoPay
                | api_enums::PaymentMethodType::Cashapp
        ),
        api_enums::PaymentMethod::BankRedirect => matches!(
            payment_method_type,
            api_enums::PaymentMethodType::Giropay
                | api_enums::PaymentMethodType::Ideal
                | api_enums::PaymentMethodType::Sofort
                | api_enums::PaymentMethodType::Eps
                | api_enums::PaymentMethodType::BancontactCard
                | api_enums::PaymentMethodType::Blik
                | api_enums::PaymentMethodType::OnlineBankingThailand
                | api_enums::PaymentMethodType::OnlineBankingCzechRepublic
                | api_enums::PaymentMethodType::OnlineBankingFinland
                | api_enums::PaymentMethodType::OnlineBankingFpx
                | api_enums::PaymentMethodType::OnlineBankingPoland
                | api_enums::PaymentMethodType::OnlineBankingSlovakia
                | api_enums::PaymentMethodType::Przelewy24
                | api_enums::PaymentMethodType::Trustly
                | api_enums::PaymentMethodType::Bizum
                | api_enums::PaymentMethodType::Interac
                | api_enums::PaymentMethodType::OpenBankingUk
        ),
        api_enums::PaymentMethod::BankTransfer => matches!(
            payment_method_type,
            api_enums::PaymentMethodType::Ach
                | api_enums::PaymentMethodType::Sepa
                | api_enums::PaymentMethodType::Bacs
                | api_enums::PaymentMethodType::Multibanco
                | api_enums::PaymentMethodType::Pix
                | api_enums::PaymentMethodType::Pse
                | api_enums::PaymentMethodType::PermataBankTransfer
                | api_enums::PaymentMethodType::BcaBankTransfer
                | api_enums::PaymentMethodType::BniVa
                | api_enums::PaymentMethodType::BriVa
                | api_enums::PaymentMethodType::CimbVa
                | api_enums::PaymentMethodType::DanamonVa
                | api_enums::PaymentMethodType::MandiriVa
        ),
        api_enums::PaymentMethod::BankDebit => matches!(
            payment_method_type,
            api_enums::PaymentMethodType::Ach
                | api_enums::PaymentMethodType::Sepa
                | api_enums::PaymentMethodType::Bacs
                | api_enums::PaymentMethodType::Becs
        ),
        api_enums::PaymentMethod::Crypto => matches!(
            payment_method_type,
            api_enums::PaymentMethodType::CryptoCurrency
        ),
        api_enums::PaymentMethod::Reward => matches!(
            payment_method_type,
            api_enums::PaymentMethodType::Evoucher | api_enums::PaymentMethodType::ClassicReward
        ),
        api_enums::PaymentMethod::Upi => matches!(
            payment_method_type,
            api_enums::PaymentMethodType::UpiCollect
        ),
        api_enums::PaymentMethod::Voucher => matches!(
            payment_method_type,
            api_enums::PaymentMethodType::Boleto
                | api_enums::PaymentMethodType::Efecty
                | api_enums::PaymentMethodType::PagoEfectivo
                | api_enums::PaymentMethodType::RedCompra
                | api_enums::PaymentMethodType::RedPagos
                | api_enums::PaymentMethodType::Indomaret
                | api_enums::PaymentMethodType::Alfamart
                | api_enums::PaymentMethodType::Oxxo
                | api_enums::PaymentMethodType::SevenEleven
                | api_enums::PaymentMethodType::Lawson
                | api_enums::PaymentMethodType::MiniStop
                | api_enums::PaymentMethodType::FamilyMart
                | api_enums::PaymentMethodType::Seicomart
                | api_enums::PaymentMethodType::PayEasy
        ),
        api_enums::PaymentMethod::GiftCard => {
            matches!(
                payment_method_type,
                api_enums::PaymentMethodType::Givex | api_enums::PaymentMethodType::PaySafeCard
            )
        }
        api_enums::PaymentMethod::CardRedirect => matches!(
            payment_method_type,
            api_enums::PaymentMethodType::Knet
                | api_enums::PaymentMethodType::Benefit
                | api_enums::PaymentMethodType::MomoAtm
        ),
    }
}

pub fn check_force_psync_precondition(status: &storage_enums::AttemptStatus) -> bool {
    !matches!(
        status,
        storage_enums::AttemptStatus::Charged
            | storage_enums::AttemptStatus::AutoRefunded
            | storage_enums::AttemptStatus::Voided
            | storage_enums::AttemptStatus::CodInitiated
            | storage_enums::AttemptStatus::Started
            | storage_enums::AttemptStatus::Failure
    )
}

pub fn append_option<T, U, F, V>(func: F, option1: Option<T>, option2: Option<U>) -> Option<V>
where
    F: FnOnce(T, U) -> V,
{
    Some(func(option1?, option2?))
}

#[cfg(feature = "olap")]
pub(super) async fn filter_by_constraints(
    db: &dyn StorageInterface,
    constraints: &api::PaymentListConstraints,
    merchant_id: &str,
    storage_scheme: storage_enums::MerchantStorageScheme,
) -> CustomResult<Vec<PaymentIntent>, errors::DataStorageError> {
    let result = db
        .filter_payment_intent_by_constraints(
            merchant_id,
            &constraints.clone().into(),
            storage_scheme,
        )
        .await?;
    Ok(result)
}

#[cfg(feature = "olap")]
pub(super) fn validate_payment_list_request(
    req: &api::PaymentListConstraints,
) -> CustomResult<(), errors::ApiErrorResponse> {
    use common_utils::consts::PAYMENTS_LIST_MAX_LIMIT_V1;

    utils::when(
        req.limit > PAYMENTS_LIST_MAX_LIMIT_V1 || req.limit < 1,
        || {
            Err(errors::ApiErrorResponse::InvalidRequestData {
                message: format!(
                    "limit should be in between 1 and {}",
                    PAYMENTS_LIST_MAX_LIMIT_V1
                ),
            })
        },
    )?;
    Ok(())
}
#[cfg(feature = "olap")]
pub(super) fn validate_payment_list_request_for_joins(
    limit: u32,
) -> CustomResult<(), errors::ApiErrorResponse> {
    use common_utils::consts::PAYMENTS_LIST_MAX_LIMIT_V2;

    utils::when(!(1..=PAYMENTS_LIST_MAX_LIMIT_V2).contains(&limit), || {
        Err(errors::ApiErrorResponse::InvalidRequestData {
            message: format!(
                "limit should be in between 1 and {}",
                PAYMENTS_LIST_MAX_LIMIT_V2
            ),
        })
    })?;
    Ok(())
}

pub fn get_handle_response_url(
    payment_id: String,
    business_profile: &diesel_models::business_profile::BusinessProfile,
    response: api::PaymentsResponse,
    connector: String,
) -> RouterResult<api::RedirectionResponse> {
    let payments_return_url = response.return_url.as_ref();

    let redirection_response = make_pg_redirect_response(payment_id, &response, connector);

    let return_url = make_merchant_url_with_response(
        business_profile,
        redirection_response,
        payments_return_url,
        response.client_secret.as_ref(),
        response.manual_retry_allowed,
    )
    .attach_printable("Failed to make merchant url with response")?;

    make_url_with_signature(&return_url, business_profile)
}

pub fn make_merchant_url_with_response(
    business_profile: &diesel_models::business_profile::BusinessProfile,
    redirection_response: api::PgRedirectResponse,
    request_return_url: Option<&String>,
    client_secret: Option<&masking::Secret<String>>,
    manual_retry_allowed: Option<bool>,
) -> RouterResult<String> {
    // take return url if provided in the request else use merchant return url
    let url = request_return_url
        .or(business_profile.return_url.as_ref())
        .get_required_value("return_url")?;

    let status_check = redirection_response.status;

    let payment_client_secret = client_secret
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .into_report()
        .attach_printable("Expected client secret to be `Some`")?;

    let merchant_url_with_response = if business_profile.redirect_to_merchant_with_http_post {
        url::Url::parse_with_params(
            url,
            &[
                ("status", status_check.to_string()),
                (
                    "payment_intent_client_secret",
                    payment_client_secret.peek().to_string(),
                ),
                (
                    "manual_retry_allowed",
                    manual_retry_allowed.unwrap_or(false).to_string(),
                ),
            ],
        )
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to parse the url with param")?
    } else {
        let amount = redirection_response.amount.get_required_value("amount")?;
        url::Url::parse_with_params(
            url,
            &[
                ("status", status_check.to_string()),
                (
                    "payment_intent_client_secret",
                    payment_client_secret.peek().to_string(),
                ),
                ("amount", amount.to_string()),
                (
                    "manual_retry_allowed",
                    manual_retry_allowed.unwrap_or(false).to_string(),
                ),
            ],
        )
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to parse the url with param")?
    };

    Ok(merchant_url_with_response.to_string())
}

pub async fn make_ephemeral_key(
    state: AppState,
    customer_id: String,
    merchant_id: String,
) -> errors::RouterResponse<ephemeral_key::EphemeralKey> {
    let store = &state.store;
    let id = utils::generate_id(consts::ID_LENGTH, "eki");
    let secret = format!("epk_{}", &Uuid::new_v4().simple().to_string());
    let ek = ephemeral_key::EphemeralKeyNew {
        id,
        customer_id,
        merchant_id,
        secret,
    };
    let ek = store
        .create_ephemeral_key(ek, state.conf.eph_key.validity)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to create ephemeral key")?;
    Ok(services::ApplicationResponse::Json(ek))
}

pub async fn delete_ephemeral_key(
    state: AppState,
    ek_id: String,
) -> errors::RouterResponse<ephemeral_key::EphemeralKey> {
    let db = state.store.as_ref();
    let ek = db
        .delete_ephemeral_key(&ek_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to delete ephemeral key")?;
    Ok(services::ApplicationResponse::Json(ek))
}

pub fn make_pg_redirect_response(
    payment_id: String,
    response: &api::PaymentsResponse,
    connector: String,
) -> api::PgRedirectResponse {
    api::PgRedirectResponse {
        payment_id,
        status: response.status,
        gateway_id: connector,
        customer_id: response.customer_id.to_owned(),
        amount: Some(response.amount),
    }
}

pub fn make_url_with_signature(
    redirect_url: &str,
    business_profile: &diesel_models::business_profile::BusinessProfile,
) -> RouterResult<api::RedirectionResponse> {
    let mut url = url::Url::parse(redirect_url)
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to parse the url")?;

    let mut base_url = url.clone();
    base_url.query_pairs_mut().clear();

    let url = if business_profile.enable_payment_response_hash {
        let key = business_profile
            .payment_response_hash_key
            .as_ref()
            .get_required_value("payment_response_hash_key")?;
        let signature = hmac_sha512_sorted_query_params(
            &mut url.query_pairs().collect::<Vec<_>>(),
            key.as_str(),
        )?;

        url.query_pairs_mut()
            .append_pair("signature", &signature)
            .append_pair("signature_algorithm", "HMAC-SHA512");
        url.to_owned()
    } else {
        url.to_owned()
    };

    let parameters = url
        .query_pairs()
        .collect::<Vec<_>>()
        .iter()
        .map(|(key, value)| (key.clone().into_owned(), value.clone().into_owned()))
        .collect::<Vec<_>>();

    Ok(api::RedirectionResponse {
        return_url: base_url.to_string(),
        params: parameters,
        return_url_with_query_params: url.to_string(),
        http_method: if business_profile.redirect_to_merchant_with_http_post {
            services::Method::Post.to_string()
        } else {
            services::Method::Get.to_string()
        },
        headers: Vec::new(),
    })
}

pub fn hmac_sha512_sorted_query_params(
    params: &mut [(Cow<'_, str>, Cow<'_, str>)],
    key: &str,
) -> RouterResult<String> {
    params.sort();
    let final_string = params
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&");

    let signature = crypto::HmacSha512::sign_message(
        &crypto::HmacSha512,
        key.as_bytes(),
        final_string.as_bytes(),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to sign the message")?;

    Ok(hex::encode(signature))
}

pub fn check_if_operation_confirm<Op: std::fmt::Debug>(operations: Op) -> bool {
    format!("{operations:?}") == "PaymentConfirm"
}

#[allow(clippy::too_many_arguments)]
pub fn generate_mandate(
    merchant_id: String,
    payment_id: String,
    connector: String,
    setup_mandate_details: Option<MandateData>,
    customer: &Option<domain::Customer>,
    payment_method_id: String,
    connector_mandate_id: Option<pii::SecretSerdeValue>,
    network_txn_id: Option<String>,
    payment_method_data_option: Option<api_models::payments::PaymentMethodData>,
    mandate_reference: Option<MandateReference>,
    merchant_connector_id: Option<String>,
) -> CustomResult<Option<storage::MandateNew>, errors::ApiErrorResponse> {
    match (setup_mandate_details, customer) {
        (Some(data), Some(cus)) => {
            let mandate_id = utils::generate_id(consts::ID_LENGTH, "man");

            // The construction of the mandate new must be visible
            let mut new_mandate = storage::MandateNew::default();

            let customer_acceptance = data
                .customer_acceptance
                .get_required_value("customer_acceptance")?;
            new_mandate
                .set_mandate_id(mandate_id)
                .set_customer_id(cus.customer_id.clone())
                .set_merchant_id(merchant_id)
                .set_original_payment_id(Some(payment_id))
                .set_payment_method_id(payment_method_id)
                .set_connector(connector)
                .set_mandate_status(storage_enums::MandateStatus::Active)
                .set_connector_mandate_ids(connector_mandate_id)
                .set_network_transaction_id(network_txn_id)
                .set_customer_ip_address(
                    customer_acceptance
                        .get_ip_address()
                        .map(masking::Secret::new),
                )
                .set_customer_user_agent(customer_acceptance.get_user_agent())
                .set_customer_accepted_at(Some(customer_acceptance.get_accepted_at()))
                .set_metadata(payment_method_data_option.map(|payment_method_data| {
                    pii::SecretSerdeValue::new(
                        serde_json::to_value(payment_method_data).unwrap_or_default(),
                    )
                }))
                .set_connector_mandate_id(
                    mandate_reference.and_then(|reference| reference.connector_mandate_id),
                )
                .set_merchant_connector_id(merchant_connector_id);

            Ok(Some(
                match data.mandate_type.get_required_value("mandate_type")? {
                    data_models::mandates::MandateDataType::SingleUse(data) => new_mandate
                        .set_mandate_amount(Some(data.amount))
                        .set_mandate_currency(Some(data.currency))
                        .set_mandate_type(storage_enums::MandateType::SingleUse)
                        .to_owned(),

                    data_models::mandates::MandateDataType::MultiUse(op_data) => match op_data {
                        Some(data) => new_mandate
                            .set_mandate_amount(Some(data.amount))
                            .set_mandate_currency(Some(data.currency))
                            .set_start_date(data.start_date)
                            .set_end_date(data.end_date),
                        // .set_metadata(data.metadata),
                        // we are storing PaymentMethodData in metadata of mandate
                        None => &mut new_mandate,
                    }
                    .set_mandate_type(storage_enums::MandateType::MultiUse)
                    .to_owned(),
                },
            ))
        }
        (_, _) => Ok(None),
    }
}

// A function to manually authenticate the client secret with intent fulfillment time
pub fn authenticate_client_secret(
    request_client_secret: Option<&String>,
    payment_intent: &PaymentIntent,
    merchant_intent_fulfillment_time: Option<i64>,
) -> Result<(), errors::ApiErrorResponse> {
    match (request_client_secret, &payment_intent.client_secret) {
        (Some(req_cs), Some(pi_cs)) => {
            if req_cs != pi_cs {
                Err(errors::ApiErrorResponse::ClientSecretInvalid)
            } else {
                //This is done to check whether the merchant_account's intent fulfillment time has expired or not
                let payment_intent_fulfillment_deadline =
                    payment_intent.created_at.saturating_add(Duration::seconds(
                        merchant_intent_fulfillment_time
                            .unwrap_or(consts::DEFAULT_FULFILLMENT_TIME),
                    ));
                let current_timestamp = common_utils::date_time::now();
                fp_utils::when(
                    current_timestamp > payment_intent_fulfillment_deadline,
                    || Err(errors::ApiErrorResponse::ClientSecretExpired),
                )
            }
        }
        // If there is no client in payment intent, then it has expired
        (Some(_), None) => Err(errors::ApiErrorResponse::ClientSecretExpired),
        _ => Ok(()),
    }
}

pub async fn get_merchant_fullfillment_time(
    payment_link_id: Option<String>,
    intent_fulfillment_time: Option<i64>,
    db: &dyn StorageInterface,
) -> RouterResult<Option<i64>> {
    if let Some(payment_link_id) = payment_link_id {
        let payment_link_db = db
            .find_payment_link_by_payment_link_id(&payment_link_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentLinkNotFound)?;

        let curr_time = common_utils::date_time::now();
        Ok(payment_link_db
            .fulfilment_time
            .map(|merchant_expiry_time| (merchant_expiry_time - curr_time).whole_seconds()))
    } else {
        Ok(intent_fulfillment_time)
    }
}

pub(crate) fn validate_payment_status_against_not_allowed_statuses(
    intent_status: &storage_enums::IntentStatus,
    not_allowed_statuses: &[storage_enums::IntentStatus],
    action: &'static str,
) -> Result<(), errors::ApiErrorResponse> {
    fp_utils::when(not_allowed_statuses.contains(intent_status), || {
        Err(errors::ApiErrorResponse::PreconditionFailed {
            message: format!(
                "You cannot {action} this payment because it has status {intent_status}",
            ),
        })
    })
}

#[instrument(skip_all)]
pub(crate) fn validate_pm_or_token_given(
    payment_method: &Option<api_enums::PaymentMethod>,
    payment_method_data: &Option<api::PaymentMethodData>,
    payment_method_type: &Option<api_enums::PaymentMethodType>,
    mandate_type: &Option<api::MandateTransactionType>,
    token: &Option<String>,
) -> Result<(), errors::ApiErrorResponse> {
    utils::when(
        !matches!(
            payment_method_type,
            Some(api_enums::PaymentMethodType::Paypal)
        ) && !matches!(
            mandate_type,
            Some(api::MandateTransactionType::RecurringMandateTransaction)
        ) && token.is_none()
            && (payment_method_data.is_none() || payment_method.is_none()),
        || {
            Err(errors::ApiErrorResponse::InvalidRequestData {
                message: "A payment token or payment method data is required".to_string(),
            })
        },
    )
}

// A function to perform database lookup and then verify the client secret
pub async fn verify_payment_intent_time_and_client_secret(
    db: &dyn StorageInterface,
    merchant_account: &domain::MerchantAccount,
    client_secret: Option<String>,
) -> error_stack::Result<Option<PaymentIntent>, errors::ApiErrorResponse> {
    client_secret
        .async_map(|cs| async move {
            let payment_id = get_payment_id_from_client_secret(&cs)?;

            let payment_intent = db
                .find_payment_intent_by_payment_id_merchant_id(
                    &payment_id,
                    &merchant_account.merchant_id,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

            let intent_fulfillment_time = get_merchant_fullfillment_time(
                payment_intent.payment_link_id.clone(),
                merchant_account.intent_fulfillment_time,
                db,
            )
            .await?;

            authenticate_client_secret(Some(&cs), &payment_intent, intent_fulfillment_time)?;
            Ok(payment_intent)
        })
        .await
        .transpose()
}

/// Check whether the business details are configured in the merchant account
pub fn validate_business_details(
    business_country: Option<api_enums::CountryAlpha2>,
    business_label: Option<&String>,
    merchant_account: &domain::MerchantAccount,
) -> RouterResult<()> {
    let primary_business_details = merchant_account
        .primary_business_details
        .clone()
        .parse_value::<Vec<api_models::admin::PrimaryBusinessDetails>>("PrimaryBusinessDetails")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("failed to parse primary business details")?;

    business_country
        .zip(business_label)
        .map(|(business_country, business_label)| {
            primary_business_details
                .iter()
                .find(|business_details| {
                    &business_details.business == business_label
                        && business_details.country == business_country
                })
                .ok_or(errors::ApiErrorResponse::PreconditionFailed {
                    message: "business_details are not configured in the merchant account"
                        .to_string(),
                })
        })
        .transpose()?;

    Ok(())
}

/// Do lazy parsing of primary business details
/// If both country and label are passed, no need to parse business details from merchant_account
/// If any one is missing, get it from merchant_account
/// If there is more than one label or country configured in merchant account, then
/// passing business details for payment is mandatory to avoid ambiguity
pub fn get_business_details(
    business_country: Option<api_enums::CountryAlpha2>,
    business_label: Option<&String>,
    merchant_account: &domain::MerchantAccount,
) -> RouterResult<(api_enums::CountryAlpha2, String)> {
    let primary_business_details = merchant_account
        .primary_business_details
        .clone()
        .parse_value::<Vec<api_models::admin::PrimaryBusinessDetails>>("PrimaryBusinessDetails")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("failed to parse primary business details")?;

    match business_country.zip(business_label) {
        Some((business_country, business_label)) => {
            Ok((business_country.to_owned(), business_label.to_owned()))
        }
        _ => match primary_business_details.first() {
            Some(business_details) if primary_business_details.len() == 1 => Ok((
                business_country.unwrap_or_else(|| business_details.country.to_owned()),
                business_label
                    .map(ToString::to_string)
                    .unwrap_or_else(|| business_details.business.to_owned()),
            )),
            _ => Err(report!(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "business_country, business_label"
            })),
        },
    }
}

#[inline]
pub(crate) fn get_payment_id_from_client_secret(cs: &str) -> RouterResult<String> {
    let (payment_id, _) = cs
        .rsplit_once("_secret_")
        .ok_or(errors::ApiErrorResponse::ClientSecretInvalid)
        .into_report()?;
    Ok(payment_id.to_string())
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_authenticate_client_secret_fulfillment_time_not_expired() {
        let payment_intent = PaymentIntent {
            id: 21,
            payment_id: "23".to_string(),
            merchant_id: "22".to_string(),
            status: storage_enums::IntentStatus::RequiresCapture,
            amount: 200,
            currency: None,
            amount_captured: None,
            customer_id: None,
            description: None,
            return_url: None,
            metadata: None,
            connector_id: None,
            shipping_address_id: None,
            billing_address_id: None,
            statement_descriptor_name: None,
            statement_descriptor_suffix: None,
            created_at: common_utils::date_time::now(),
            modified_at: common_utils::date_time::now(),
            last_synced: None,
            setup_future_usage: None,
            off_session: None,
            client_secret: Some("1".to_string()),
            active_attempt: data_models::RemoteStorageObject::ForeignID("nopes".to_string()),
            business_country: None,
            business_label: None,
            order_details: None,
            allowed_payment_method_types: None,
            connector_metadata: None,
            feature_metadata: None,
            attempt_count: 1,
            payment_link_id: None,
            profile_id: None,
            merchant_decision: None,
            payment_confirm_source: None,
            surcharge_applicable: None,
            updated_by: storage_enums::MerchantStorageScheme::PostgresOnly.to_string(),
        };
        let req_cs = Some("1".to_string());
        let merchant_fulfillment_time = Some(900);
        assert!(authenticate_client_secret(
            req_cs.as_ref(),
            &payment_intent,
            merchant_fulfillment_time,
        )
        .is_ok()); // Check if the result is an Ok variant
    }

    #[test]
    fn test_authenticate_client_secret_fulfillment_time_expired() {
        let payment_intent = PaymentIntent {
            id: 21,
            payment_id: "23".to_string(),
            merchant_id: "22".to_string(),
            status: storage_enums::IntentStatus::RequiresCapture,
            amount: 200,
            currency: None,
            amount_captured: None,
            customer_id: None,
            description: None,
            return_url: None,
            metadata: None,
            connector_id: None,
            shipping_address_id: None,
            billing_address_id: None,
            statement_descriptor_name: None,
            statement_descriptor_suffix: None,
            created_at: common_utils::date_time::now().saturating_sub(Duration::seconds(20)),
            modified_at: common_utils::date_time::now(),
            last_synced: None,
            setup_future_usage: None,
            off_session: None,
            client_secret: Some("1".to_string()),
            active_attempt: data_models::RemoteStorageObject::ForeignID("nopes".to_string()),
            business_country: None,
            business_label: None,
            order_details: None,
            allowed_payment_method_types: None,
            connector_metadata: None,
            feature_metadata: None,
            attempt_count: 1,
            payment_link_id: None,
            profile_id: None,
            merchant_decision: None,
            payment_confirm_source: None,
            surcharge_applicable: None,
            updated_by: storage_enums::MerchantStorageScheme::PostgresOnly.to_string(),
        };
        let req_cs = Some("1".to_string());
        let merchant_fulfillment_time = Some(10);
        assert!(authenticate_client_secret(
            req_cs.as_ref(),
            &payment_intent,
            merchant_fulfillment_time,
        )
        .is_err())
    }

    #[test]
    fn test_authenticate_client_secret_expired() {
        let payment_intent = PaymentIntent {
            id: 21,
            payment_id: "23".to_string(),
            merchant_id: "22".to_string(),
            status: storage_enums::IntentStatus::RequiresCapture,
            amount: 200,
            currency: None,
            amount_captured: None,
            customer_id: None,
            description: None,
            return_url: None,
            metadata: None,
            connector_id: None,
            shipping_address_id: None,
            billing_address_id: None,
            statement_descriptor_name: None,
            statement_descriptor_suffix: None,
            created_at: common_utils::date_time::now().saturating_sub(Duration::seconds(20)),
            modified_at: common_utils::date_time::now(),
            last_synced: None,
            setup_future_usage: None,
            off_session: None,
            client_secret: None,
            active_attempt: data_models::RemoteStorageObject::ForeignID("nopes".to_string()),
            business_country: None,
            business_label: None,
            order_details: None,
            allowed_payment_method_types: None,
            connector_metadata: None,
            feature_metadata: None,
            attempt_count: 1,
            payment_link_id: None,
            profile_id: None,
            merchant_decision: None,
            payment_confirm_source: None,
            surcharge_applicable: None,
            updated_by: storage_enums::MerchantStorageScheme::PostgresOnly.to_string(),
        };
        let req_cs = Some("1".to_string());
        let merchant_fulfillment_time = Some(10);
        assert!(authenticate_client_secret(
            req_cs.as_ref(),
            &payment_intent,
            merchant_fulfillment_time,
        )
        .is_err())
    }
}

// This function will be removed after moving this functionality to server_wrap and using cache instead of config
#[instrument(skip_all)]
pub async fn insert_merchant_connector_creds_to_config(
    db: &dyn StorageInterface,
    merchant_id: &str,
    merchant_connector_details: admin::MerchantConnectorDetailsWrap,
) -> RouterResult<()> {
    if let Some(encoded_data) = merchant_connector_details.encoded_data {
        match db
            .insert_config(storage::ConfigNew {
                key: format!(
                    "mcd_{merchant_id}_{}",
                    merchant_connector_details.creds_identifier
                ),
                config: encoded_data.peek().to_owned(),
            })
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => {
                if err.current_context().is_db_unique_violation() {
                    Ok(())
                } else {
                    Err(err
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to insert connector_creds to config"))
                }
            }
        }
    } else {
        Ok(())
    }
}

#[derive(Clone)]
pub enum MerchantConnectorAccountType {
    DbVal(domain::MerchantConnectorAccount),
    CacheVal(api_models::admin::MerchantConnectorDetails),
}

impl MerchantConnectorAccountType {
    pub fn get_metadata(&self) -> Option<masking::Secret<serde_json::Value>> {
        match self {
            Self::DbVal(val) => val.metadata.to_owned(),
            Self::CacheVal(val) => val.metadata.to_owned(),
        }
    }

    pub fn get_connector_account_details(&self) -> serde_json::Value {
        match self {
            Self::DbVal(val) => val.connector_account_details.peek().to_owned(),
            Self::CacheVal(val) => val.connector_account_details.peek().to_owned(),
        }
    }

    pub fn is_disabled(&self) -> bool {
        match self {
            Self::DbVal(ref inner) => inner.disabled.unwrap_or(false),
            // Cached merchant connector account, only contains the account details,
            // the merchant connector account must only be cached if it's not disabled
            Self::CacheVal(_) => false,
        }
    }

    pub fn is_test_mode_on(&self) -> Option<bool> {
        match self {
            Self::DbVal(val) => val.test_mode,
            Self::CacheVal(_) => None,
        }
    }

    pub fn get_mca_id(&self) -> Option<String> {
        match self {
            Self::DbVal(db_val) => Some(db_val.merchant_connector_id.to_string()),
            Self::CacheVal(_) => None,
        }
    }
}

/// Query for merchant connector account either by business label or profile id
/// If profile_id is passed use it, or use connector_label to query merchant connector account
#[instrument(skip_all)]
pub async fn get_merchant_connector_account(
    state: &AppState,
    merchant_id: &str,
    creds_identifier: Option<String>,
    key_store: &domain::MerchantKeyStore,
    profile_id: &String,
    connector_name: &str,
    merchant_connector_id: Option<&String>,
) -> RouterResult<MerchantConnectorAccountType> {
    let db = &*state.store;
    match creds_identifier {
        Some(creds_identifier) => {
            let mca_config = db
                .find_config_by_key(format!("mcd_{merchant_id}_{creds_identifier}").as_str())
                .await
                .to_not_found_response(
                    errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                        id: format!("mcd_{merchant_id}_{creds_identifier}"),
                    },
                )?;

            #[cfg(feature = "kms")]
            let private_key = state
                .kms_secrets
                .jwekey
                .peek()
                .tunnel_private_key
                .as_bytes();

            #[cfg(not(feature = "kms"))]
            let private_key = state.conf.jwekey.tunnel_private_key.as_bytes();

            let decrypted_mca = services::decrypt_jwe(mca_config.config.as_str(), services::KeyIdCheck::SkipKeyIdCheck, private_key, jwe::RSA_OAEP_256)
                                     .await
                                     .change_context(errors::ApiErrorResponse::UnprocessableEntity{
                                        message: "decoding merchant_connector_details failed due to invalid data format!".into()})
                                     .attach_printable(
                                        "Failed to decrypt merchant_connector_details sent in request and then put in cache",
                                    )?;

            let res = String::into_bytes(decrypted_mca)
                        .parse_struct("MerchantConnectorDetails")
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable(
                            "Failed to parse merchant_connector_details sent in request and then put in cache",
                        )?;

            Ok(MerchantConnectorAccountType::CacheVal(res))
        }
        None => {
            if let Some(merchant_connector_id) = merchant_connector_id {
                db.find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                    merchant_id,
                    merchant_connector_id,
                    key_store,
                )
                .await
                .to_not_found_response(
                    errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                        id: merchant_connector_id.to_string(),
                    },
                )
            } else {
                db.find_merchant_connector_account_by_profile_id_connector_name(
                    profile_id,
                    connector_name,
                    key_store,
                )
                .await
                .to_not_found_response(
                    errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                        id: format!("profile id {profile_id} and connector name {connector_name}"),
                    },
                )
            }
        }
        .map(MerchantConnectorAccountType::DbVal),
    }
}

/// This function replaces the request and response type of routerdata with the
/// request and response type passed
/// # Arguments
///
/// * `router_data` - original router data
/// * `request` - new request
/// * `response` - new response
pub fn router_data_type_conversion<F1, F2, Req1, Req2, Res1, Res2>(
    router_data: RouterData<F1, Req1, Res1>,
    request: Req2,
    response: Result<Res2, ErrorResponse>,
) -> RouterData<F2, Req2, Res2> {
    RouterData {
        flow: std::marker::PhantomData,
        request,
        response,
        merchant_id: router_data.merchant_id,
        address: router_data.address,
        amount_captured: router_data.amount_captured,
        auth_type: router_data.auth_type,
        connector: router_data.connector,
        connector_auth_type: router_data.connector_auth_type,
        connector_meta_data: router_data.connector_meta_data,
        description: router_data.description,
        payment_id: router_data.payment_id,
        payment_method: router_data.payment_method,
        payment_method_id: router_data.payment_method_id,
        return_url: router_data.return_url,
        status: router_data.status,
        attempt_id: router_data.attempt_id,
        access_token: router_data.access_token,
        session_token: router_data.session_token,
        reference_id: router_data.reference_id,
        payment_method_token: router_data.payment_method_token,
        customer_id: router_data.customer_id,
        connector_customer: router_data.connector_customer,
        preprocessing_id: router_data.preprocessing_id,
        payment_method_balance: router_data.payment_method_balance,
        recurring_mandate_payment_data: router_data.recurring_mandate_payment_data,
        connector_request_reference_id: router_data.connector_request_reference_id,
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode: router_data.test_mode,
        connector_api_version: router_data.connector_api_version,
        connector_http_status_code: router_data.connector_http_status_code,
        external_latency: router_data.external_latency,
        apple_pay_flow: router_data.apple_pay_flow,
    }
}

#[instrument(skip_all)]
pub fn get_attempt_type(
    payment_intent: &PaymentIntent,
    payment_attempt: &PaymentAttempt,
    request: &api::PaymentsRequest,
    action: &str,
) -> RouterResult<AttemptType> {
    match payment_intent.status {
        enums::IntentStatus::Failed => {
            if matches!(
                request.retry_action,
                Some(api_models::enums::RetryAction::ManualRetry)
            ) {
                metrics::MANUAL_RETRY_REQUEST_COUNT.add(
                    &metrics::CONTEXT,
                    1,
                    &[metrics::request::add_attributes(
                        "merchant_id",
                        payment_attempt.merchant_id.clone(),
                    )],
                );
                match payment_attempt.status {
                    enums::AttemptStatus::Started
                    | enums::AttemptStatus::AuthenticationPending
                    | enums::AttemptStatus::AuthenticationSuccessful
                    | enums::AttemptStatus::Authorized
                    | enums::AttemptStatus::Charged
                    | enums::AttemptStatus::Authorizing
                    | enums::AttemptStatus::CodInitiated
                    | enums::AttemptStatus::VoidInitiated
                    | enums::AttemptStatus::CaptureInitiated
                    | enums::AttemptStatus::Unresolved
                    | enums::AttemptStatus::Pending
                    | enums::AttemptStatus::ConfirmationAwaited
                    | enums::AttemptStatus::PartialCharged
                    | enums::AttemptStatus::Voided
                    | enums::AttemptStatus::AutoRefunded
                    | enums::AttemptStatus::PaymentMethodAwaited
                    | enums::AttemptStatus::DeviceDataCollectionPending => {
                        metrics::MANUAL_RETRY_VALIDATION_FAILED.add(
                            &metrics::CONTEXT,
                            1,
                            &[metrics::request::add_attributes(
                                "merchant_id",
                                payment_attempt.merchant_id.clone(),
                            )],
                        );
                        Err(errors::ApiErrorResponse::InternalServerError)
                            .into_report()
                            .attach_printable("Payment Attempt unexpected state")
                    }

                    storage_enums::AttemptStatus::VoidFailed
                    | storage_enums::AttemptStatus::RouterDeclined
                    | storage_enums::AttemptStatus::CaptureFailed => {
                        metrics::MANUAL_RETRY_VALIDATION_FAILED.add(
                            &metrics::CONTEXT,
                            1,
                            &[metrics::request::add_attributes(
                                "merchant_id",
                                payment_attempt.merchant_id.clone(),
                            )],
                        );
                        Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                            message:
                                format!("You cannot {action} this payment because it has status {}, and the previous attempt has the status {}", payment_intent.status, payment_attempt.status)
                            }
                        ))
                    }

                    storage_enums::AttemptStatus::AuthenticationFailed
                    | storage_enums::AttemptStatus::AuthorizationFailed
                    | storage_enums::AttemptStatus::Failure => {
                        metrics::MANUAL_RETRY_COUNT.add(
                            &metrics::CONTEXT,
                            1,
                            &[metrics::request::add_attributes(
                                "merchant_id",
                                payment_attempt.merchant_id.clone(),
                            )],
                        );
                        Ok(AttemptType::New)
                    }
                }
            } else {
                Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                        message:
                            format!("You cannot {action} this payment because it has status {}, you can pass `retry_action` as `manual_retry` in request to try this payment again", payment_intent.status)
                        }
                    ))
            }
        }
        enums::IntentStatus::Cancelled
        | enums::IntentStatus::RequiresCapture
        | enums::IntentStatus::PartiallyCaptured
        | enums::IntentStatus::Processing
        | enums::IntentStatus::Succeeded => {
            Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                message: format!(
                    "You cannot {action} this payment because it has status {}",
                    payment_intent.status,
                ),
            }))
        }

        enums::IntentStatus::RequiresCustomerAction
        | enums::IntentStatus::RequiresMerchantAction
        | enums::IntentStatus::RequiresPaymentMethod
        | enums::IntentStatus::RequiresConfirmation => Ok(AttemptType::SameOld),
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum AttemptType {
    New,
    SameOld,
}

impl AttemptType {
    // The function creates a new payment_attempt from the previous payment attempt but doesn't populate fields like payment_method, error_code etc.
    // Logic to override the fields with data provided in the request should be done after this if required.
    // In case if fields are not overridden by the request then they contain the same data that was in the previous attempt provided it is populated in this function.
    #[inline(always)]
    fn make_new_payment_attempt(
        payment_method_data: &Option<api_models::payments::PaymentMethodData>,
        old_payment_attempt: PaymentAttempt,
        new_attempt_count: i16,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> storage::PaymentAttemptNew {
        let created_at @ modified_at @ last_synced = Some(common_utils::date_time::now());

        storage::PaymentAttemptNew {
            attempt_id: utils::get_payment_attempt_id(
                &old_payment_attempt.payment_id,
                new_attempt_count,
            ),
            payment_id: old_payment_attempt.payment_id,
            merchant_id: old_payment_attempt.merchant_id,

            // A new payment attempt is getting created so, used the same function which is used to populate status in PaymentCreate Flow.
            status: payment_attempt_status_fsm(payment_method_data, Some(true)),

            amount: old_payment_attempt.amount,
            currency: old_payment_attempt.currency,
            save_to_locker: old_payment_attempt.save_to_locker,

            connector: None,

            error_message: None,
            offer_amount: old_payment_attempt.offer_amount,
            surcharge_amount: old_payment_attempt.surcharge_amount,
            tax_amount: old_payment_attempt.tax_amount,
            payment_method_id: None,
            payment_method: None,
            capture_method: old_payment_attempt.capture_method,
            capture_on: old_payment_attempt.capture_on,
            confirm: old_payment_attempt.confirm,
            authentication_type: old_payment_attempt.authentication_type,
            created_at,
            modified_at,
            last_synced,
            cancellation_reason: None,
            amount_to_capture: old_payment_attempt.amount_to_capture,

            // Once the payment_attempt is authorised then mandate_id is created. If this payment attempt is authorised then mandate_id will be overridden.
            // Since mandate_id is a contract between merchant and customer to debit customers amount adding it to newly created attempt
            mandate_id: old_payment_attempt.mandate_id,

            // The payment could be done from a different browser or same browser, it would probably be overridden by request data.
            browser_info: None,

            error_code: None,
            payment_token: None,
            connector_metadata: None,
            payment_experience: None,
            payment_method_type: None,
            payment_method_data: None,

            // In case it is passed in create and not in confirm,
            business_sub_label: old_payment_attempt.business_sub_label,
            // If the algorithm is entered in Create call from server side, it needs to be populated here, however it could be overridden from the request.
            straight_through_algorithm: old_payment_attempt.straight_through_algorithm,
            mandate_details: old_payment_attempt.mandate_details,
            preprocessing_step_id: None,
            error_reason: None,
            multiple_capture_count: None,
            connector_response_reference_id: None,
            amount_capturable: old_payment_attempt.amount,
            updated_by: storage_scheme.to_string(),
            authentication_data: None,
            encoded_data: None,
            merchant_connector_id: None,
        }
    }

    #[instrument(skip_all)]
    pub async fn modify_payment_intent_and_payment_attempt(
        &self,
        request: &api::PaymentsRequest,
        fetched_payment_intent: PaymentIntent,
        fetched_payment_attempt: PaymentAttempt,
        db: &dyn StorageInterface,
        storage_scheme: storage::enums::MerchantStorageScheme,
    ) -> RouterResult<(PaymentIntent, PaymentAttempt)> {
        match self {
            Self::SameOld => Ok((fetched_payment_intent, fetched_payment_attempt)),
            Self::New => {
                let new_attempt_count = fetched_payment_intent.attempt_count + 1;
                let new_payment_attempt = db
                    .insert_payment_attempt(
                        Self::make_new_payment_attempt(
                            &request.payment_method_data,
                            fetched_payment_attempt,
                            new_attempt_count,
                            storage_scheme,
                        ),
                        storage_scheme,
                    )
                    .await
                    .to_duplicate_response(errors::ApiErrorResponse::DuplicatePayment {
                        payment_id: fetched_payment_intent.payment_id.to_owned(),
                    })?;

                let updated_payment_intent = db
                    .update_payment_intent(
                        fetched_payment_intent,
                        storage::PaymentIntentUpdate::StatusAndAttemptUpdate {
                            status: payment_intent_status_fsm(
                                &request.payment_method_data,
                                Some(true),
                            ),
                            active_attempt_id: new_payment_attempt.attempt_id.clone(),
                            attempt_count: new_attempt_count,
                            updated_by: storage_scheme.to_string(),
                        },
                        storage_scheme,
                    )
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

                logger::info!(
                    "manual_retry payment for {} with attempt_id {}",
                    updated_payment_intent.payment_id,
                    new_payment_attempt.attempt_id
                );

                Ok((updated_payment_intent, new_payment_attempt))
            }
        }
    }

    #[instrument(skip_all)]
    pub async fn get_or_insert_connector_response(
        &self,
        payment_attempt: &PaymentAttempt,
        db: &dyn StorageInterface,
        storage_scheme: storage::enums::MerchantStorageScheme,
    ) -> RouterResult<storage::ConnectorResponse> {
        match self {
            Self::New => db
                .insert_connector_response(
                    payments::PaymentCreate::make_connector_response(payment_attempt),
                    storage_scheme,
                )
                .await
                .to_duplicate_response(errors::ApiErrorResponse::DuplicatePayment {
                    payment_id: payment_attempt.payment_id.clone(),
                }),
            Self::SameOld => db
                .find_connector_response_by_payment_id_merchant_id_attempt_id(
                    &payment_attempt.payment_id,
                    &payment_attempt.merchant_id,
                    &payment_attempt.attempt_id,
                    storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound),
        }
    }

    #[instrument(skip_all)]
    pub async fn get_connector_response(
        &self,
        db: &dyn StorageInterface,
        payment_id: &str,
        merchant_id: &str,
        attempt_id: &str,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> RouterResult<storage::ConnectorResponse> {
        match self {
            Self::New => Err(errors::ApiErrorResponse::InternalServerError)
                .into_report()
                .attach_printable("Precondition failed, the attempt type should not be `New`"),
            Self::SameOld => db
                .find_connector_response_by_payment_id_merchant_id_attempt_id(
                    payment_id,
                    merchant_id,
                    attempt_id,
                    storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound),
        }
    }
}

#[inline(always)]
pub fn is_manual_retry_allowed(
    intent_status: &storage_enums::IntentStatus,
    attempt_status: &storage_enums::AttemptStatus,
    connector_request_reference_id_config: &ConnectorRequestReferenceIdConfig,
    merchant_id: &str,
) -> Option<bool> {
    let is_payment_status_eligible_for_retry = match intent_status {
        enums::IntentStatus::Failed => match attempt_status {
            enums::AttemptStatus::Started
            | enums::AttemptStatus::AuthenticationPending
            | enums::AttemptStatus::AuthenticationSuccessful
            | enums::AttemptStatus::Authorized
            | enums::AttemptStatus::Charged
            | enums::AttemptStatus::Authorizing
            | enums::AttemptStatus::CodInitiated
            | enums::AttemptStatus::VoidInitiated
            | enums::AttemptStatus::CaptureInitiated
            | enums::AttemptStatus::Unresolved
            | enums::AttemptStatus::Pending
            | enums::AttemptStatus::ConfirmationAwaited
            | enums::AttemptStatus::PartialCharged
            | enums::AttemptStatus::Voided
            | enums::AttemptStatus::AutoRefunded
            | enums::AttemptStatus::PaymentMethodAwaited
            | enums::AttemptStatus::DeviceDataCollectionPending => {
                logger::error!("Payment Attempt should not be in this state because Attempt to Intent status mapping doesn't allow it");
                None
            }

            storage_enums::AttemptStatus::VoidFailed
            | storage_enums::AttemptStatus::RouterDeclined
            | storage_enums::AttemptStatus::CaptureFailed => Some(false),

            storage_enums::AttemptStatus::AuthenticationFailed
            | storage_enums::AttemptStatus::AuthorizationFailed
            | storage_enums::AttemptStatus::Failure => Some(true),
        },
        enums::IntentStatus::Cancelled
        | enums::IntentStatus::RequiresCapture
        | enums::IntentStatus::PartiallyCaptured
        | enums::IntentStatus::Processing
        | enums::IntentStatus::Succeeded => Some(false),

        enums::IntentStatus::RequiresCustomerAction
        | enums::IntentStatus::RequiresMerchantAction
        | enums::IntentStatus::RequiresPaymentMethod
        | enums::IntentStatus::RequiresConfirmation => None,
    };
    let is_merchant_id_enabled_for_retries = !connector_request_reference_id_config
        .merchant_ids_send_payment_id_as_connector_request_id
        .contains(merchant_id);
    is_payment_status_eligible_for_retry
        .map(|payment_status_check| payment_status_check && is_merchant_id_enabled_for_retries)
}

#[cfg(test)]
mod test {
    #![allow(clippy::unwrap_used)]
    #[test]
    fn test_client_secret_parse() {
        let client_secret1 = "pay_3TgelAms4RQec8xSStjF_secret_fc34taHLw1ekPgNh92qr";
        let client_secret2 = "pay_3Tgel__Ams4RQ_secret_ec8xSStjF_secret_fc34taHLw1ekPgNh92qr";
        let client_secret3 =
            "pay_3Tgel__Ams4RQ_secret_ec8xSStjF_secret__secret_fc34taHLw1ekPgNh92qr";

        assert_eq!(
            "pay_3TgelAms4RQec8xSStjF",
            super::get_payment_id_from_client_secret(client_secret1).unwrap()
        );
        assert_eq!(
            "pay_3Tgel__Ams4RQ_secret_ec8xSStjF",
            super::get_payment_id_from_client_secret(client_secret2).unwrap()
        );
        assert_eq!(
            "pay_3Tgel__Ams4RQ_secret_ec8xSStjF_secret_",
            super::get_payment_id_from_client_secret(client_secret3).unwrap()
        );
    }
}

#[instrument(skip_all)]
pub async fn get_additional_payment_data(
    pm_data: &api_models::payments::PaymentMethodData,
    db: &dyn StorageInterface,
) -> api_models::payments::AdditionalPaymentData {
    match pm_data {
        api_models::payments::PaymentMethodData::Card(card_data) => {
            let card_isin = Some(card_data.card_number.clone().get_card_isin());
            let last4 = Some(card_data.card_number.clone().get_last4());
            if card_data.card_issuer.is_some()
                && card_data.card_network.is_some()
                && card_data.card_type.is_some()
                && card_data.card_issuing_country.is_some()
                && card_data.bank_code.is_some()
            {
                api_models::payments::AdditionalPaymentData::Card(Box::new(
                    api_models::payments::AdditionalCardInfo {
                        card_issuer: card_data.card_issuer.to_owned(),
                        card_network: card_data.card_network.clone(),
                        card_type: card_data.card_type.to_owned(),
                        card_issuing_country: card_data.card_issuing_country.to_owned(),
                        bank_code: card_data.bank_code.to_owned(),
                        card_exp_month: Some(card_data.card_exp_month.clone()),
                        card_exp_year: Some(card_data.card_exp_year.clone()),
                        card_holder_name: Some(card_data.card_holder_name.clone()),
                        last4: last4.clone(),
                        card_isin: card_isin.clone(),
                    },
                ))
            } else {
                let card_info = card_isin
                    .clone()
                    .async_and_then(|card_isin| async move {
                        db.get_card_info(&card_isin)
                            .await
                            .map_err(|error| services::logger::warn!(card_info_error=?error))
                            .ok()
                    })
                    .await
                    .flatten()
                    .map(|card_info| {
                        api_models::payments::AdditionalPaymentData::Card(Box::new(
                            api_models::payments::AdditionalCardInfo {
                                card_issuer: card_info.card_issuer,
                                card_network: card_info.card_network.clone(),
                                bank_code: card_info.bank_code,
                                card_type: card_info.card_type,
                                card_issuing_country: card_info.card_issuing_country,
                                last4: last4.clone(),
                                card_isin: card_isin.clone(),
                                card_exp_month: Some(card_data.card_exp_month.clone()),
                                card_exp_year: Some(card_data.card_exp_year.clone()),
                                card_holder_name: Some(card_data.card_holder_name.clone()),
                            },
                        ))
                    });
                card_info.unwrap_or(api_models::payments::AdditionalPaymentData::Card(Box::new(
                    api_models::payments::AdditionalCardInfo {
                        card_issuer: None,
                        card_network: None,
                        bank_code: None,
                        card_type: None,
                        card_issuing_country: None,
                        last4,
                        card_isin,
                        card_exp_month: Some(card_data.card_exp_month.clone()),
                        card_exp_year: Some(card_data.card_exp_year.clone()),
                        card_holder_name: Some(card_data.card_holder_name.clone()),
                    },
                )))
            }
        }
        api_models::payments::PaymentMethodData::BankRedirect(bank_redirect_data) => {
            match bank_redirect_data {
                api_models::payments::BankRedirectData::Eps { bank_name, .. } => {
                    api_models::payments::AdditionalPaymentData::BankRedirect {
                        bank_name: bank_name.to_owned(),
                    }
                }
                api_models::payments::BankRedirectData::Ideal { bank_name, .. } => {
                    api_models::payments::AdditionalPaymentData::BankRedirect {
                        bank_name: bank_name.to_owned(),
                    }
                }
                _ => api_models::payments::AdditionalPaymentData::BankRedirect { bank_name: None },
            }
        }
        api_models::payments::PaymentMethodData::Wallet(_) => {
            api_models::payments::AdditionalPaymentData::Wallet {}
        }
        api_models::payments::PaymentMethodData::PayLater(_) => {
            api_models::payments::AdditionalPaymentData::PayLater {}
        }
        api_models::payments::PaymentMethodData::BankTransfer(_) => {
            api_models::payments::AdditionalPaymentData::BankTransfer {}
        }
        api_models::payments::PaymentMethodData::Crypto(_) => {
            api_models::payments::AdditionalPaymentData::Crypto {}
        }
        api_models::payments::PaymentMethodData::BankDebit(_) => {
            api_models::payments::AdditionalPaymentData::BankDebit {}
        }
        api_models::payments::PaymentMethodData::MandatePayment => {
            api_models::payments::AdditionalPaymentData::MandatePayment {}
        }
        api_models::payments::PaymentMethodData::Reward => {
            api_models::payments::AdditionalPaymentData::Reward {}
        }
        api_models::payments::PaymentMethodData::Upi(_) => {
            api_models::payments::AdditionalPaymentData::Upi {}
        }
        api_models::payments::PaymentMethodData::CardRedirect(_) => {
            api_models::payments::AdditionalPaymentData::CardRedirect {}
        }
        api_models::payments::PaymentMethodData::Voucher(_) => {
            api_models::payments::AdditionalPaymentData::Voucher {}
        }
        api_models::payments::PaymentMethodData::GiftCard(_) => {
            api_models::payments::AdditionalPaymentData::GiftCard {}
        }
    }
}

pub fn validate_customer_access(
    payment_intent: &PaymentIntent,
    auth_flow: services::AuthFlow,
    request: &api::PaymentsRequest,
) -> Result<(), errors::ApiErrorResponse> {
    if auth_flow == services::AuthFlow::Client && request.customer_id.is_some() {
        let is_same_customer = request.customer_id == payment_intent.customer_id;
        if !is_same_customer {
            Err(errors::ApiErrorResponse::GenericUnauthorized {
                message: "Unauthorised access to update customer".to_string(),
            })?;
        }
    }
    Ok(())
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ApplePayData {
    version: masking::Secret<String>,
    data: masking::Secret<String>,
    signature: masking::Secret<String>,
    header: ApplePayHeader,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplePayHeader {
    ephemeral_public_key: masking::Secret<String>,
    public_key_hash: masking::Secret<String>,
    transaction_id: masking::Secret<String>,
}

impl ApplePayData {
    pub fn token_json(
        wallet_data: api_models::payments::WalletData,
    ) -> CustomResult<Self, errors::ConnectorError> {
        let json_wallet_data: Self =
            connector::utils::WalletData::get_wallet_token_as_json(&wallet_data)?;
        Ok(json_wallet_data)
    }

    pub async fn decrypt(
        &self,
        state: &AppState,
    ) -> CustomResult<serde_json::Value, errors::ApplePayDecryptionError> {
        let merchant_id = self.merchant_id(state).await?;
        let shared_secret = self.shared_secret(state).await?;
        let symmetric_key = self.symmetric_key(&merchant_id, &shared_secret)?;
        let decrypted = self.decrypt_ciphertext(&symmetric_key)?;
        let parsed_decrypted: serde_json::Value = serde_json::from_str(&decrypted)
            .into_report()
            .change_context(errors::ApplePayDecryptionError::DecryptionFailed)?;
        Ok(parsed_decrypted)
    }

    pub async fn merchant_id(
        &self,
        state: &AppState,
    ) -> CustomResult<String, errors::ApplePayDecryptionError> {
        #[cfg(feature = "kms")]
        let cert_data = kms::get_kms_client(&state.conf.kms)
            .await
            .decrypt(&state.conf.applepay_decrypt_keys.apple_pay_ppc)
            .await
            .change_context(errors::ApplePayDecryptionError::DecryptionFailed)?;

        #[cfg(not(feature = "kms"))]
        let cert_data = &state.conf.applepay_decrypt_keys.apple_pay_ppc;

        let base64_decode_cert_data = BASE64_ENGINE
            .decode(cert_data)
            .into_report()
            .change_context(errors::ApplePayDecryptionError::Base64DecodingFailed)?;

        // Parsing the certificate using x509-parser
        let (_, certificate) = parse_x509_certificate(&base64_decode_cert_data)
            .into_report()
            .change_context(errors::ApplePayDecryptionError::CertificateParsingFailed)
            .attach_printable("Error parsing apple pay PPC")?;

        // Finding the merchant ID extension
        let apple_pay_m_id = certificate
            .extensions()
            .iter()
            .find(|extension| {
                extension
                    .oid
                    .to_string()
                    .eq(consts::MERCHANT_ID_FIELD_EXTENSION_ID)
            })
            .map(|ext| {
                let merchant_id = String::from_utf8_lossy(ext.value)
                    .trim()
                    .trim_start_matches('@')
                    .to_string();

                merchant_id
            })
            .ok_or(errors::ApplePayDecryptionError::MissingMerchantId)
            .into_report()
            .attach_printable("Unable to find merchant ID extension in the certificate")?;

        Ok(apple_pay_m_id)
    }

    pub async fn shared_secret(
        &self,
        state: &AppState,
    ) -> CustomResult<Vec<u8>, errors::ApplePayDecryptionError> {
        let public_ec_bytes = BASE64_ENGINE
            .decode(self.header.ephemeral_public_key.peek().as_bytes())
            .into_report()
            .change_context(errors::ApplePayDecryptionError::Base64DecodingFailed)?;

        let public_key = PKey::public_key_from_der(&public_ec_bytes)
            .into_report()
            .change_context(errors::ApplePayDecryptionError::KeyDeserializationFailed)
            .attach_printable("Failed to deserialize the public key")?;

        #[cfg(feature = "kms")]
        let decrypted_apple_pay_ppc_key = kms::get_kms_client(&state.conf.kms)
            .await
            .decrypt(&state.conf.applepay_decrypt_keys.apple_pay_ppc_key)
            .await
            .change_context(errors::ApplePayDecryptionError::DecryptionFailed)?;

        #[cfg(not(feature = "kms"))]
        let decrypted_apple_pay_ppc_key = &state.conf.applepay_decrypt_keys.apple_pay_ppc_key;
        // Create PKey objects from EcKey
        let private_key = PKey::private_key_from_pem(decrypted_apple_pay_ppc_key.as_bytes())
            .into_report()
            .change_context(errors::ApplePayDecryptionError::KeyDeserializationFailed)
            .attach_printable("Failed to deserialize the private key")?;

        // Create the Deriver object and set the peer public key
        let mut deriver = Deriver::new(&private_key)
            .into_report()
            .change_context(errors::ApplePayDecryptionError::DerivingSharedSecretKeyFailed)
            .attach_printable("Failed to create a deriver for the private key")?;

        deriver
            .set_peer(&public_key)
            .into_report()
            .change_context(errors::ApplePayDecryptionError::DerivingSharedSecretKeyFailed)
            .attach_printable("Failed to set the peer key for the secret derivation")?;

        // Compute the shared secret
        let shared_secret = deriver
            .derive_to_vec()
            .into_report()
            .change_context(errors::ApplePayDecryptionError::DerivingSharedSecretKeyFailed)
            .attach_printable("Final key derivation failed")?;
        Ok(shared_secret)
    }

    pub fn symmetric_key(
        &self,
        merchant_id: &str,
        shared_secret: &[u8],
    ) -> CustomResult<Vec<u8>, errors::ApplePayDecryptionError> {
        let kdf_algorithm = b"\x0did-aes256-GCM";
        let kdf_party_v = hex::decode(merchant_id)
            .into_report()
            .change_context(errors::ApplePayDecryptionError::Base64DecodingFailed)?;
        let kdf_party_u = b"Apple";
        let kdf_info = [&kdf_algorithm[..], kdf_party_u, &kdf_party_v[..]].concat();

        let mut hash = openssl::sha::Sha256::new();
        hash.update(b"\x00\x00\x00");
        hash.update(b"\x01");
        hash.update(shared_secret);
        hash.update(&kdf_info[..]);
        let symmetric_key = hash.finish();
        Ok(symmetric_key.to_vec())
    }

    pub fn decrypt_ciphertext(
        &self,
        symmetric_key: &[u8],
    ) -> CustomResult<String, errors::ApplePayDecryptionError> {
        let data = BASE64_ENGINE
            .decode(self.data.peek().as_bytes())
            .into_report()
            .change_context(errors::ApplePayDecryptionError::Base64DecodingFailed)?;
        let iv = [0u8; 16]; //Initialization vector IV is typically used in AES-GCM (Galois/Counter Mode) encryption for randomizing the encryption process.
        let ciphertext = &data[..data.len() - 16];
        let tag = &data[data.len() - 16..];
        let cipher = Cipher::aes_256_gcm();
        let decrypted_data = decrypt_aead(cipher, symmetric_key, Some(&iv), &[], ciphertext, tag)
            .into_report()
            .change_context(errors::ApplePayDecryptionError::DecryptionFailed)?;
        let decrypted = String::from_utf8(decrypted_data)
            .into_report()
            .change_context(errors::ApplePayDecryptionError::DecryptionFailed)?;

        Ok(decrypted)
    }
}

pub fn validate_payment_link_request(
    payment_link_object: &api_models::payments::PaymentLinkObject,
    confirm: Option<bool>,
    order_details: Option<Vec<api_models::payments::OrderDetailsWithAmount>>,
) -> Result<(), errors::ApiErrorResponse> {
    if let Some(cnf) = confirm {
        if !cnf {
            let current_time = Some(common_utils::date_time::now());
            if current_time > payment_link_object.link_expiry {
                return Err(errors::ApiErrorResponse::InvalidRequestData {
                    message: "link_expiry time cannot be less than current time".to_string(),
                });
            } else if order_details.is_none() {
                return Err(errors::ApiErrorResponse::InvalidRequestData {
                    message: "cannot create payment link without order details".to_string(),
                });
            }
        } else {
            return Err(errors::ApiErrorResponse::InvalidRequestData {
                message: "cannot confirm a payment while creating a payment link".to_string(),
            });
        }
    }
    Ok(())
}
