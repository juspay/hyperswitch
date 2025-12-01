use std::{borrow::Cow, collections::HashSet, net::IpAddr, ops::Deref, str::FromStr};

pub use ::payment_methods::helpers::{
    populate_bin_details_for_payment_method_create,
    validate_payment_method_type_against_payment_method,
};
#[cfg(feature = "v2")]
use api_models::ephemeral_key::ClientSecretResponse;
use api_models::{
    mandates::RecurringDetails,
    payments::{additional_info as payment_additional_types, RequestSurchargeDetails},
};
use base64::Engine;
#[cfg(feature = "v1")]
use common_enums::enums::{CallConnectorAction, ExecutionMode, ExecutionPath, GatewaySystem};
use common_enums::ConnectorType;
#[cfg(feature = "v2")]
use common_utils::id_type::GenerateId;
use common_utils::{
    crypto::Encryptable,
    ext_traits::{AsyncExt, ByteSliceExt, Encode, ValueExt},
    fp_utils, generate_id,
    id_type::{self},
    new_type::{MaskedIban, MaskedSortCode},
    pii, type_name,
    types::{
        keymanager::{Identifier, ToEncryptable},
        MinorUnit,
    },
};
use diesel_models::enums;
// TODO : Evaluate all the helper functions ()
use error_stack::{report, ResultExt};
#[cfg(feature = "v1")]
use external_services::grpc_client;
use futures::future::Either;
pub use hyperswitch_domain_models::customer;
#[cfg(feature = "v1")]
use hyperswitch_domain_models::payments::payment_intent::CustomerData;
use hyperswitch_domain_models::{
    mandates::MandateData,
    payment_method_data::{GetPaymentMethodType, PazeWalletData},
    payments::{
        self as domain_payments, payment_attempt::PaymentAttempt,
        payment_intent::PaymentIntentFetchConstraints, PaymentIntent,
    },
    router_data::{InteracCustomerInfo, KlarnaSdkResponse},
};
pub use hyperswitch_interfaces::{
    api::ConnectorSpecifications,
    configs::MerchantConnectorAccountType,
    integrity::{CheckIntegrity, FlowIntegrity, GetIntegrityObject},
};
use josekit::jwe;
use masking::{ExposeInterface, PeekInterface, SwitchStrategy};
use num_traits::{FromPrimitive, ToPrimitive};
use openssl::{
    derive::Deriver,
    pkey::PKey,
    symm::{decrypt_aead, Cipher},
};
use rand::Rng;
#[cfg(feature = "v2")]
use redis_interface::errors::RedisError;
use router_env::{instrument, logger, tracing};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use x509_parser::parse_x509_certificate;

use super::{
    operations::{BoxedOperation, Operation, PaymentResponse},
    CustomerDetails, PaymentData,
};
#[cfg(feature = "v1")]
use crate::core::{
    payments::{
        call_connector_service, customers,
        flows::{ConstructFlowSpecificData, Feature},
        gateway::context as gateway_context,
        operations::ValidateResult as OperationsValidateResult,
        should_add_task_to_process_tracker, OperationSessionGetters, OperationSessionSetters,
        TokenizationAction,
    },
    unified_connector_service::update_gateway_system_in_feature_metadata,
    utils as core_utils,
};
#[cfg(feature = "v1")]
use crate::routes;
use crate::{
    configs::settings::{ConnectorRequestReferenceIdConfig, TempLockerEnableConfig},
    connector,
    consts::{self, BASE64_ENGINE},
    core::{
        authentication,
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        mandate::helpers::MandateGenericData,
        payment_methods::{
            self,
            cards::{self},
            network_tokenization, vault,
        },
        payments,
        pm_auth::retrieve_payment_method_from_auth_service,
    },
    db::StorageInterface,
    routes::{metrics, payment_methods as payment_methods_handler, SessionState},
    services,
    types::{
        api::{self, admin, enums as api_enums, MandateValidationFieldsExt},
        domain::{self, types},
        storage::{self, enums as storage_enums, ephemeral_key, CardTokenData},
        transformers::{ForeignFrom, ForeignTryFrom},
        AdditionalMerchantData, AdditionalPaymentMethodConnectorResponse, ErrorResponse,
        MandateReference, MerchantAccountData, MerchantRecipientData, PaymentsResponseData,
        RecipientIdType, RecurringMandatePaymentData, RouterData,
    },
    utils::{
        self,
        crypto::{self, SignMessage},
        OptionExt, StringExt,
    },
};
#[cfg(feature = "v2")]
use crate::{core::admin as core_admin, headers, types::ConnectorAuthType};
#[cfg(feature = "v1")]
use crate::{
    core::{payment_methods::cards::create_encrypted_data, unified_connector_service},
    types::storage::CustomerUpdate::Update,
};

#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn create_or_update_address_for_payment_by_request(
    session_state: &SessionState,
    req_address: Option<&api::Address>,
    address_id: Option<&str>,
    merchant_id: &id_type::MerchantId,
    customer_id: Option<&id_type::CustomerId>,
    merchant_key_store: &domain::MerchantKeyStore,
    payment_id: &id_type::PaymentId,
    storage_scheme: storage_enums::MerchantStorageScheme,
) -> CustomResult<Option<domain::Address>, errors::ApiErrorResponse> {
    let key = merchant_key_store.key.get_inner().peek();
    let db = &session_state.store;
    Ok(match address_id {
        Some(id) => match req_address {
            Some(address) => {
                let encrypted_data = types::crypto_operation(
                    &session_state.into(),
                    type_name!(domain::Address),
                    types::CryptoOperation::BatchEncrypt(
                        domain::FromRequestEncryptableAddress::to_encryptable(
                            domain::FromRequestEncryptableAddress {
                                line1: address.address.as_ref().and_then(|a| a.line1.clone()),
                                line2: address.address.as_ref().and_then(|a| a.line2.clone()),
                                line3: address.address.as_ref().and_then(|a| a.line3.clone()),
                                state: address.address.as_ref().and_then(|a| a.state.clone()),
                                first_name: address
                                    .address
                                    .as_ref()
                                    .and_then(|a| a.first_name.clone()),
                                last_name: address
                                    .address
                                    .as_ref()
                                    .and_then(|a| a.last_name.clone()),
                                zip: address.address.as_ref().and_then(|a| a.zip.clone()),
                                phone_number: address
                                    .phone
                                    .as_ref()
                                    .and_then(|phone| phone.number.clone()),
                                email: address
                                    .email
                                    .as_ref()
                                    .map(|a| a.clone().expose().switch_strategy()),
                                origin_zip: address
                                    .address
                                    .as_ref()
                                    .and_then(|a| a.origin_zip.clone()),
                            },
                        ),
                    ),
                    Identifier::Merchant(merchant_key_store.merchant_id.clone()),
                    key,
                )
                .await
                .and_then(|val| val.try_into_batchoperation())
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed while encrypting address")?;
                let encryptable_address =
                    domain::FromRequestEncryptableAddress::from_encryptable(encrypted_data)
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed while encrypting address")?;
                let address_update = storage::AddressUpdate::Update {
                    city: address
                        .address
                        .as_ref()
                        .and_then(|value| value.city.clone()),
                    country: address.address.as_ref().and_then(|value| value.country),
                    line1: encryptable_address.line1,
                    line2: encryptable_address.line2,
                    line3: encryptable_address.line3,
                    state: encryptable_address.state,
                    zip: encryptable_address.zip,
                    first_name: encryptable_address.first_name,
                    last_name: encryptable_address.last_name,
                    phone_number: encryptable_address.phone_number,
                    country_code: address
                        .phone
                        .as_ref()
                        .and_then(|value| value.country_code.clone()),
                    updated_by: storage_scheme.to_string(),
                    email: encryptable_address.email.map(|email| {
                        let encryptable: Encryptable<masking::Secret<String, pii::EmailStrategy>> =
                            Encryptable::new(
                                email.clone().into_inner().switch_strategy(),
                                email.into_encrypted(),
                            );
                        encryptable
                    }),
                    origin_zip: encryptable_address.origin_zip,
                };
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
                        payment_id.to_owned(),
                        merchant_key_store,
                        storage_scheme,
                    )
                    .await
                    .map(|payment_address| payment_address.address)
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
                .await
                .map(|payment_address| payment_address.address),
            )
            .transpose()
            .to_not_found_response(errors::ApiErrorResponse::AddressNotFound)?,
        },
        None => match req_address {
            Some(address) => {
                let address =
                    get_domain_address(session_state, address, merchant_id, key, storage_scheme)
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed while encrypting address while insert")?;

                let payment_address = domain::PaymentAddress {
                    address,
                    payment_id: payment_id.clone(),
                    customer_id: customer_id.cloned(),
                };

                Some(
                    db.insert_address_for_payments(
                        payment_id,
                        payment_address,
                        merchant_key_store,
                        storage_scheme,
                    )
                    .await
                    .map(|payment_address| payment_address.address)
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
    state: &SessionState,
    req_address: Option<&api::Address>,
    address_id: Option<&str>,
    merchant_id: &id_type::MerchantId,
    customer_id: Option<&id_type::CustomerId>,
    merchant_key_store: &domain::MerchantKeyStore,
    payment_id: &id_type::PaymentId,
    storage_scheme: storage_enums::MerchantStorageScheme,
) -> CustomResult<Option<domain::Address>, errors::ApiErrorResponse> {
    let key = merchant_key_store.key.get_inner().peek();
    let db = &state.store;
    Ok(match address_id {
        Some(id) => Some(
            db.find_address_by_merchant_id_payment_id_address_id(
                merchant_id,
                payment_id,
                id,
                merchant_key_store,
                storage_scheme,
            )
            .await
            .map(|payment_address| payment_address.address),
        )
        .transpose()
        .to_not_found_response(errors::ApiErrorResponse::AddressNotFound)?,
        None => match req_address {
            Some(address) => {
                // generate a new address here
                let address = get_domain_address(state, address, merchant_id, key, storage_scheme)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed while encrypting address while insert")?;

                let payment_address = domain::PaymentAddress {
                    address,
                    payment_id: payment_id.clone(),
                    customer_id: customer_id.cloned(),
                };

                Some(
                    db.insert_address_for_payments(
                        payment_id,
                        payment_address,
                        merchant_key_store,
                        storage_scheme,
                    )
                    .await
                    .map(|payment_address| payment_address.address)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed while inserting new address")?,
                )
            }
            None => None,
        },
    })
}

pub async fn get_domain_address(
    session_state: &SessionState,
    address: &api_models::payments::Address,
    merchant_id: &id_type::MerchantId,
    key: &[u8],
    storage_scheme: enums::MerchantStorageScheme,
) -> CustomResult<domain::Address, common_utils::errors::CryptoError> {
    async {
        let address_details = &address.address.as_ref();
        let encrypted_data = types::crypto_operation(
            &session_state.into(),
            type_name!(domain::Address),
            types::CryptoOperation::BatchEncrypt(
                domain::FromRequestEncryptableAddress::to_encryptable(
                    domain::FromRequestEncryptableAddress {
                        line1: address.address.as_ref().and_then(|a| a.line1.clone()),
                        line2: address.address.as_ref().and_then(|a| a.line2.clone()),
                        line3: address.address.as_ref().and_then(|a| a.line3.clone()),
                        state: address.address.as_ref().and_then(|a| a.state.clone()),
                        first_name: address.address.as_ref().and_then(|a| a.first_name.clone()),
                        last_name: address.address.as_ref().and_then(|a| a.last_name.clone()),
                        zip: address.address.as_ref().and_then(|a| a.zip.clone()),
                        phone_number: address
                            .phone
                            .as_ref()
                            .and_then(|phone| phone.number.clone()),
                        email: address
                            .email
                            .as_ref()
                            .map(|a| a.clone().expose().switch_strategy()),
                        origin_zip: address.address.as_ref().and_then(|a| a.origin_zip.clone()),
                    },
                ),
            ),
            Identifier::Merchant(merchant_id.to_owned()),
            key,
        )
        .await
        .and_then(|val| val.try_into_batchoperation())?;
        let encryptable_address =
            domain::FromRequestEncryptableAddress::from_encryptable(encrypted_data)
                .change_context(common_utils::errors::CryptoError::EncodingFailed)?;
        Ok(domain::Address {
            phone_number: encryptable_address.phone_number,
            country_code: address.phone.as_ref().and_then(|a| a.country_code.clone()),
            merchant_id: merchant_id.to_owned(),
            address_id: generate_id(consts::ID_LENGTH, "add"),
            city: address_details.and_then(|address_details| address_details.city.clone()),
            country: address_details.and_then(|address_details| address_details.country),
            line1: encryptable_address.line1,
            line2: encryptable_address.line2,
            line3: encryptable_address.line3,
            state: encryptable_address.state,
            created_at: common_utils::date_time::now(),
            first_name: encryptable_address.first_name,
            last_name: encryptable_address.last_name,
            modified_at: common_utils::date_time::now(),
            zip: encryptable_address.zip,
            updated_by: storage_scheme.to_string(),
            email: encryptable_address.email.map(|email| {
                let encryptable: Encryptable<masking::Secret<String, pii::EmailStrategy>> =
                    Encryptable::new(
                        email.clone().into_inner().switch_strategy(),
                        email.into_encrypted(),
                    );
                encryptable
            }),
            origin_zip: encryptable_address.origin_zip,
        })
    }
    .await
}

pub async fn get_address_by_id(
    state: &SessionState,
    address_id: Option<String>,
    merchant_key_store: &domain::MerchantKeyStore,
    payment_id: &id_type::PaymentId,
    merchant_id: &id_type::MerchantId,
    storage_scheme: storage_enums::MerchantStorageScheme,
) -> CustomResult<Option<domain::Address>, errors::ApiErrorResponse> {
    match address_id {
        None => Ok(None),
        Some(address_id) => {
            let db = &*state.store;
            Ok(db
                .find_address_by_merchant_id_payment_id_address_id(
                    merchant_id,
                    payment_id,
                    &address_id,
                    merchant_key_store,
                    storage_scheme,
                )
                .await
                .map(|payment_address| payment_address.address)
                .ok())
        }
    }
}

#[cfg(feature = "v1")]
pub async fn get_token_pm_type_mandate_details(
    state: &SessionState,
    request: &api::PaymentsRequest,
    mandate_type: Option<api::MandateTransactionType>,
    platform: &domain::Platform,
    payment_method_id: Option<String>,
    payment_intent_customer_id: Option<&id_type::CustomerId>,
) -> RouterResult<MandateGenericData> {
    let mandate_data = request.mandate_data.clone().map(MandateData::foreign_from);
    let (
        payment_token,
        payment_method,
        payment_method_type,
        mandate_data,
        recurring_payment_data,
        mandate_connector_details,
        payment_method_info,
    ) = match mandate_type {
        Some(api::MandateTransactionType::NewMandateTransaction) => (
            request.payment_token.to_owned(),
            request.payment_method,
            request.payment_method_type,
            mandate_data.clone(),
            None,
            None,
            None,
        ),
        Some(api::MandateTransactionType::RecurringMandateTransaction) => {
            match &request.recurring_details {
                Some(recurring_details) => {
                    match recurring_details {
                        RecurringDetails::NetworkTransactionIdAndCardDetails(_) => {
                            (None, request.payment_method, None, None, None, None, None)
                        }
                        RecurringDetails::ProcessorPaymentToken(processor_payment_token) => {
                            if let Some(mca_id) = &processor_payment_token.merchant_connector_id {
                                let db = &*state.store;

                                #[cfg(feature = "v1")]
                            let connector_name = db
                                .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                                    platform.get_processor().get_account().get_id(),
                                    mca_id,
                                    platform.get_processor().get_key_store(),
                                )
                                .await
                                .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                                    id: mca_id.clone().get_string_repr().to_string(),
                                })?.connector_name;

                                #[cfg(feature = "v2")]
                            let connector_name = db
                                .find_merchant_connector_account_by_id( mca_id, merchant_key_store)
                                .await
                                .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                                    id: mca_id.clone().get_string_repr().to_string(),
                                })?.connector_name;
                                (
                                    None,
                                    request.payment_method,
                                    None,
                                    None,
                                    None,
                                    Some(payments::MandateConnectorDetails {
                                        connector: connector_name,
                                        merchant_connector_id: Some(mca_id.clone()),
                                    }),
                                    None,
                                )
                            } else {
                                (None, request.payment_method, None, None, None, None, None)
                            }
                        }
                        RecurringDetails::MandateId(mandate_id) => {
                            let mandate_generic_data = Box::pin(get_token_for_recurring_mandate(
                                state,
                                request,
                                platform,
                                mandate_id.to_owned(),
                            ))
                            .await?;

                            (
                                mandate_generic_data.token,
                                mandate_generic_data.payment_method,
                                mandate_generic_data
                                    .payment_method_type
                                    .or(request.payment_method_type),
                                None,
                                mandate_generic_data.recurring_mandate_payment_data,
                                mandate_generic_data.mandate_connector,
                                mandate_generic_data.payment_method_info,
                            )
                        }
                        RecurringDetails::PaymentMethodId(payment_method_id) => {
                            let payment_method_info = state
                                .store
                                .find_payment_method(
                                    platform.get_processor().get_key_store(),
                                    payment_method_id,
                                    platform.get_processor().get_account().storage_scheme,
                                )
                                .await
                                .to_not_found_response(
                                    errors::ApiErrorResponse::PaymentMethodNotFound,
                                )?;
                            let customer_id = request
                                .get_customer_id()
                                .get_required_value("customer_id")?;

                            verify_mandate_details_for_recurring_payments(
                                &payment_method_info.merchant_id,
                                platform.get_processor().get_account().get_id(),
                                &payment_method_info.customer_id,
                                customer_id,
                            )?;

                            (
                                None,
                                payment_method_info.get_payment_method_type(),
                                payment_method_info.get_payment_method_subtype(),
                                None,
                                None,
                                None,
                                Some(payment_method_info),
                            )
                        }
                    }
                }
                None => {
                    if let Some(mandate_id) = request.mandate_id.clone() {
                        let mandate_generic_data = Box::pin(get_token_for_recurring_mandate(
                            state, request, platform, mandate_id,
                        ))
                        .await?;
                        (
                            mandate_generic_data.token,
                            mandate_generic_data.payment_method,
                            mandate_generic_data
                                .payment_method_type
                                .or(request.payment_method_type),
                            None,
                            mandate_generic_data.recurring_mandate_payment_data,
                            mandate_generic_data.mandate_connector,
                            mandate_generic_data.payment_method_info,
                        )
                    } else if request
                        .payment_method_type
                        .map(|payment_method_type_value| {
                            payment_method_type_value
                                .should_check_for_customer_saved_payment_method_type()
                        })
                        .unwrap_or(false)
                    {
                        let payment_request_customer_id = request.get_customer_id();
                        if let Some(customer_id) =
                            payment_request_customer_id.or(payment_intent_customer_id)
                        {
                            let customer_saved_pm_option = match state
                                .store
                                .find_payment_method_by_customer_id_merchant_id_list(
                                    platform.get_processor().get_key_store(),
                                    customer_id,
                                    platform.get_processor().get_account().get_id(),
                                    None,
                                )
                                .await
                            {
                                Ok(customer_payment_methods) => Ok(customer_payment_methods
                                    .iter()
                                    .find(|payment_method| {
                                        payment_method.get_payment_method_subtype()
                                            == request.payment_method_type
                                    })
                                    .cloned()),
                                Err(error) => {
                                    if error.current_context().is_db_not_found() {
                                        Ok(None)
                                    } else {
                                        Err(error)
                                            .change_context(
                                                errors::ApiErrorResponse::InternalServerError,
                                            )
                                            .attach_printable(
                                                "failed to find payment methods for a customer",
                                            )
                                    }
                                }
                            }?;

                            (
                                None,
                                request.payment_method,
                                request.payment_method_type,
                                None,
                                None,
                                None,
                                customer_saved_pm_option,
                            )
                        } else {
                            (
                                None,
                                request.payment_method,
                                request.payment_method_type,
                                None,
                                None,
                                None,
                                None,
                            )
                        }
                    } else {
                        let payment_method_info = payment_method_id
                            .async_map(|payment_method_id| async move {
                                state
                                    .store
                                    .find_payment_method(
                                        platform.get_processor().get_key_store(),
                                        &payment_method_id,
                                        platform.get_processor().get_account().storage_scheme,
                                    )
                                    .await
                                    .to_not_found_response(
                                        errors::ApiErrorResponse::PaymentMethodNotFound,
                                    )
                            })
                            .await
                            .transpose()?;
                        (
                            request.payment_token.to_owned(),
                            request.payment_method,
                            request.payment_method_type,
                            None,
                            None,
                            None,
                            payment_method_info,
                        )
                    }
                }
            }
        }
        None => {
            let payment_method_info = payment_method_id
                .async_map(|payment_method_id| async move {
                    state
                        .store
                        .find_payment_method(
                            platform.get_processor().get_key_store(),
                            &payment_method_id,
                            platform.get_processor().get_account().storage_scheme,
                        )
                        .await
                        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)
                })
                .await
                .transpose()?;
            (
                request.payment_token.to_owned(),
                request.payment_method,
                request.payment_method_type,
                mandate_data,
                None,
                None,
                payment_method_info,
            )
        }
    };
    Ok(MandateGenericData {
        token: payment_token,
        payment_method,
        payment_method_type,
        mandate_data,
        recurring_mandate_payment_data: recurring_payment_data,
        mandate_connector: mandate_connector_details,
        payment_method_info,
    })
}

#[cfg(feature = "v1")]
pub async fn get_token_for_recurring_mandate(
    state: &SessionState,
    req: &api::PaymentsRequest,
    platform: &domain::Platform,
    mandate_id: String,
) -> RouterResult<MandateGenericData> {
    let db = &*state.store;

    let mandate = db
        .find_mandate_by_merchant_id_mandate_id(
            platform.get_processor().get_account().get_id(),
            mandate_id.as_str(),
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MandateNotFound)?;
    let original_payment_intent = mandate
        .original_payment_id
        .as_ref()
        .async_map(|payment_id| async {
            db.find_payment_intent_by_payment_id_merchant_id(
                payment_id,
                &mandate.merchant_id,
                platform.get_processor().get_key_store(),
                platform.get_processor().get_account().storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            .map_err(|err| logger::error!(mandate_original_payment_not_found=?err))
            .ok()
        })
        .await
        .flatten();

    let original_payment_attempt = original_payment_intent
        .as_ref()
        .async_map(|payment_intent| async {
            db.find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                &payment_intent.payment_id,
                &mandate.merchant_id,
                payment_intent.active_attempt.get_id().as_str(),
                platform.get_processor().get_account().storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            .map_err(|err| logger::error!(mandate_original_payment_attempt_not_found=?err))
            .ok()
        })
        .await
        .flatten();

    let original_payment_authorized_amount = original_payment_attempt
        .clone()
        .map(|pa| pa.net_amount.get_total_amount().get_amount_as_i64());
    let original_payment_authorized_currency =
        original_payment_intent.clone().and_then(|pi| pi.currency);
    let customer = req.get_customer_id().get_required_value("customer_id")?;

    let payment_method_id = {
        if &mandate.customer_id != customer {
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
        .find_payment_method(
            platform.get_processor().get_key_store(),
            payment_method_id.as_str(),
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

    let token = Uuid::new_v4().to_string();
    let payment_method_type = payment_method.get_payment_method_subtype();
    let mandate_connector_details = payments::MandateConnectorDetails {
        connector: mandate.connector,
        merchant_connector_id: mandate.merchant_connector_id,
    };

    if let Some(enums::PaymentMethod::Card) = payment_method.get_payment_method_type() {
        if state.conf.locker.locker_enabled {
            let _ = cards::get_lookup_key_from_locker(
                state,
                &token,
                &payment_method,
                platform.get_processor().get_key_store(),
            )
            .await?;
        }

        if let Some(payment_method_from_request) = req.payment_method {
            let pm: storage_enums::PaymentMethod = payment_method_from_request;
            if payment_method
                .get_payment_method_type()
                .is_some_and(|payment_method| payment_method != pm)
            {
                Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                    message:
                        "payment method in request does not match previously provided payment \
                            method information"
                            .into()
                }))?
            }
        };

        Ok(MandateGenericData {
            token: Some(token),
            payment_method: payment_method.get_payment_method_type(),
            recurring_mandate_payment_data: Some(RecurringMandatePaymentData {
                payment_method_type,
                original_payment_authorized_amount,
                original_payment_authorized_currency,
                mandate_metadata: None,
            }),
            payment_method_type: payment_method.get_payment_method_subtype(),
            mandate_connector: Some(mandate_connector_details),
            mandate_data: None,
            payment_method_info: Some(payment_method),
        })
    } else {
        Ok(MandateGenericData {
            token: None,
            payment_method: payment_method.get_payment_method_type(),
            recurring_mandate_payment_data: Some(RecurringMandatePaymentData {
                payment_method_type,
                original_payment_authorized_amount,
                original_payment_authorized_currency,
                mandate_metadata: None,
            }),
            payment_method_type: payment_method.get_payment_method_subtype(),
            mandate_connector: Some(mandate_connector_details),
            mandate_data: None,
            payment_method_info: Some(payment_method),
        })
    }
}

#[instrument(skip_all)]
/// Check weather the merchant id in the request
/// and merchant id in the merchant account are same.
pub fn validate_merchant_id(
    merchant_id: &id_type::MerchantId,
    request_merchant_id: Option<&id_type::MerchantId>,
) -> CustomResult<(), errors::ApiErrorResponse> {
    // Get Merchant Id from the merchant
    // or get from merchant account

    let request_merchant_id = request_merchant_id.unwrap_or(merchant_id);

    utils::when(merchant_id.ne(request_merchant_id), || {
        Err(report!(errors::ApiErrorResponse::PreconditionFailed {
            message: format!(
                "Invalid `merchant_id`: {} not found in merchant account",
                request_merchant_id.get_string_repr()
            )
        }))
    })
}

#[instrument(skip_all)]
pub fn validate_request_amount_and_amount_to_capture(
    op_amount: Option<api::Amount>,
    op_amount_to_capture: Option<MinorUnit>,
    surcharge_details: Option<RequestSurchargeDetails>,
) -> CustomResult<(), errors::ApiErrorResponse> {
    match (op_amount, op_amount_to_capture) {
        (None, _) => Ok(()),
        (Some(_amount), None) => Ok(()),
        (Some(amount), Some(amount_to_capture)) => {
            match amount {
                api::Amount::Value(amount_inner) => {
                    // If both amount and amount to capture is present
                    // then amount to be capture should be less than or equal to request amount
                    let total_capturable_amount = MinorUnit::new(amount_inner.get())
                        + surcharge_details
                            .map(|surcharge_details| surcharge_details.get_total_surcharge_amount())
                            .unwrap_or_default();
                    utils::when(!amount_to_capture.le(&total_capturable_amount), || {
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

#[cfg(feature = "v1")]
/// if capture method = automatic, amount_to_capture(if provided) must be equal to amount
#[instrument(skip_all)]
pub fn validate_amount_to_capture_and_capture_method(
    payment_attempt: Option<&PaymentAttempt>,
    request: &api_models::payments::PaymentsRequest,
) -> CustomResult<(), errors::ApiErrorResponse> {
    let option_net_amount = hyperswitch_domain_models::payments::payment_attempt::NetAmount::from_payments_request_and_payment_attempt(
        request,
        payment_attempt,
    );
    let capture_method = request
        .capture_method
        .or(payment_attempt
            .map(|payment_attempt| payment_attempt.capture_method.unwrap_or_default()))
        .unwrap_or_default();
    if matches!(
        capture_method,
        api_enums::CaptureMethod::Automatic | api_enums::CaptureMethod::SequentialAutomatic
    ) {
        let total_capturable_amount =
            option_net_amount.map(|net_amount| net_amount.get_total_amount());

        let amount_to_capture = request
            .amount_to_capture
            .or(payment_attempt.and_then(|pa| pa.amount_to_capture));

        if let Some((total_capturable_amount, amount_to_capture)) =
            total_capturable_amount.zip(amount_to_capture)
        {
            utils::when(amount_to_capture != total_capturable_amount, || {
                Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                    message: "amount_to_capture must be equal to total_capturable_amount when capture_method = automatic".into()
                }))
            })
        } else {
            Ok(())
        }
    } else {
        Ok(())
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
        let card_cvc =
            cvc.parse::<u16>()
                .change_context(errors::ApiErrorResponse::InvalidDataValue {
                    field_name: "card_cvc",
                })?;
        ::cards::CardSecurityCode::try_from(card_cvc).change_context(
            errors::ApiErrorResponse::PreconditionFailed {
                message: "Invalid Card CVC".to_string(),
            },
        )?;

        validate_card_expiry(&card.card_exp_month, &card.card_exp_year)?;
    }
    Ok(())
}

#[instrument(skip_all)]
pub fn validate_card_expiry(
    card_exp_month: &masking::Secret<String>,
    card_exp_year: &masking::Secret<String>,
) -> CustomResult<(), errors::ApiErrorResponse> {
    let exp_month = card_exp_month
        .peek()
        .to_string()
        .parse::<u8>()
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "card_exp_month",
        })?;
    let month = ::cards::CardExpirationMonth::try_from(exp_month).change_context(
        errors::ApiErrorResponse::PreconditionFailed {
            message: "Invalid Expiry Month".to_string(),
        },
    )?;

    let mut year_str = card_exp_year.peek().to_string();
    if year_str.len() == 2 {
        year_str = format!("20{year_str}");
    }
    let exp_year =
        year_str
            .parse::<u16>()
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "card_exp_year",
            })?;
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

    Ok(())
}

pub fn infer_payment_type(
    amount: api::Amount,
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
            reason: "Expected one out of recurring_details and mandate_data but got both".into(),
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

pub fn validate_recurring_details_and_token(
    recurring_details: &Option<RecurringDetails>,
    payment_token: &Option<String>,
    mandate_id: &Option<String>,
) -> CustomResult<(), errors::ApiErrorResponse> {
    utils::when(
        recurring_details.is_some() && payment_token.is_some(),
        || {
            Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                message: "Expected one out of recurring_details and payment_token but got both"
                    .into()
            }))
        },
    )?;

    utils::when(recurring_details.is_some() && mandate_id.is_some(), || {
        Err(report!(errors::ApiErrorResponse::PreconditionFailed {
            message: "Expected one out of recurring_details and mandate_id but got both".into()
        }))
    })?;

    Ok(())
}

pub fn validate_overcapture_request(
    enable_overcapture: &Option<common_types::primitive_wrappers::EnableOvercaptureBool>,
    capture_method: &Option<common_enums::CaptureMethod>,
) -> CustomResult<(), errors::ApiErrorResponse> {
    if let Some(overcapture) = enable_overcapture {
        utils::when(
            *overcapture.deref()
                && !matches!(*capture_method, Some(common_enums::CaptureMethod::Manual)),
            || {
                Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                    message: "Invalid overcapture request: supported only with manual capture"
                        .into()
                }))
            },
        )?;
    }

    Ok(())
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
        _ => None,
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
    customer_id: Option<&id_type::CustomerId>,
) -> RouterResult<()> {
    match (has_setup_future_usage, customer_id) {
        (true, None) => Err(errors::ApiErrorResponse::PreconditionFailed {
            message: "customer_id is mandatory when setup_future_usage is given".to_string(),
        }
        .into()),
        _ => Ok(()),
    }
}

#[cfg(feature = "v1")]
pub fn create_startpay_url(
    base_url: &str,
    payment_attempt: &PaymentAttempt,
    payment_intent: &PaymentIntent,
) -> String {
    format!(
        "{}/payments/redirect/{}/{}/{}",
        base_url,
        payment_intent.get_id().get_string_repr(),
        payment_intent.merchant_id.get_string_repr(),
        payment_attempt.attempt_id
    )
}

pub fn create_redirect_url(
    router_base_url: &String,
    payment_attempt: &PaymentAttempt,
    connector_name: impl std::fmt::Display,
    creds_identifier: Option<&str>,
) -> String {
    let creds_identifier_path = creds_identifier.map_or_else(String::new, |cd| format!("/{cd}"));
    format!(
        "{}/payments/{}/{}/redirect/response/{}",
        router_base_url,
        payment_attempt.payment_id.get_string_repr(),
        payment_attempt.merchant_id.get_string_repr(),
        connector_name,
    ) + creds_identifier_path.as_ref()
}

pub fn create_authentication_url(
    router_base_url: &str,
    payment_attempt: &PaymentAttempt,
) -> String {
    format!(
        "{router_base_url}/payments/{}/3ds/authentication",
        payment_attempt.payment_id.get_string_repr()
    )
}

pub fn create_authorize_url(
    router_base_url: &str,
    payment_attempt: &PaymentAttempt,
    connector_name: impl std::fmt::Display,
) -> String {
    format!(
        "{}/payments/{}/{}/authorize/{}",
        router_base_url,
        payment_attempt.payment_id.get_string_repr(),
        payment_attempt.merchant_id.get_string_repr(),
        connector_name
    )
}

pub fn create_webhook_url(
    router_base_url: &str,
    merchant_id: &id_type::MerchantId,
    merchant_connector_id_or_connector_name: &str,
) -> String {
    format!(
        "{}/webhooks/{}/{}",
        router_base_url,
        merchant_id.get_string_repr(),
        merchant_connector_id_or_connector_name,
    )
}

pub fn create_complete_authorize_url(
    router_base_url: &String,
    payment_attempt: &PaymentAttempt,
    connector_name: impl std::fmt::Display,
    creds_identifier: Option<&str>,
) -> String {
    let creds_identifier = creds_identifier.map_or_else(String::new, |creds_identifier| {
        format!("/{creds_identifier}")
    });
    format!(
        "{}/payments/{}/{}/redirect/complete/{}{}",
        router_base_url,
        payment_attempt.payment_id.get_string_repr(),
        payment_attempt.merchant_id.get_string_repr(),
        connector_name,
        creds_identifier
    )
}

fn validate_recurring_mandate(req: api::MandateValidationFields) -> RouterResult<()> {
    let recurring_details = req
        .recurring_details
        .get_required_value("recurring_details")?;

    match recurring_details {
        RecurringDetails::ProcessorPaymentToken(_)
        | RecurringDetails::NetworkTransactionIdAndCardDetails(_) => Ok(()),
        _ => {
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
    }
}

pub fn verify_mandate_details(
    request_amount: MinorUnit,
    request_currency: api_enums::Currency,
    mandate: storage::Mandate,
) -> RouterResult<()> {
    match mandate.mandate_type {
        storage_enums::MandateType::SingleUse => utils::when(
            mandate
                .mandate_amount
                .map(|mandate_amount| request_amount.get_amount_as_i64() > mandate_amount)
                .unwrap_or(true),
            || {
                Err(report!(errors::ApiErrorResponse::MandateValidationFailed {
                    reason: "request amount is greater than mandate amount".into()
                }))
            },
        ),
        storage::enums::MandateType::MultiUse => utils::when(
            mandate
                .mandate_amount
                .map(|mandate_amount| {
                    (mandate.amount_captured.unwrap_or(0) + request_amount.get_amount_as_i64())
                        > mandate_amount
                })
                .unwrap_or(false),
            || {
                Err(report!(errors::ApiErrorResponse::MandateValidationFailed {
                    reason: "request amount is greater than mandate amount".into()
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
                reason: "cross currency mandates not supported".into()
            }))
        },
    )
}

pub fn verify_mandate_details_for_recurring_payments(
    mandate_merchant_id: &id_type::MerchantId,
    merchant_id: &id_type::MerchantId,
    mandate_customer_id: &id_type::CustomerId,
    customer_id: &id_type::CustomerId,
) -> RouterResult<()> {
    if mandate_merchant_id != merchant_id {
        Err(report!(errors::ApiErrorResponse::MandateNotFound))?
    }
    if mandate_customer_id != customer_id {
        Err(report!(errors::ApiErrorResponse::PreconditionFailed {
            message: "customer_id must match mandate customer_id".into()
        }))?
    }

    Ok(())
}

#[instrument(skip_all)]
pub fn payment_attempt_status_fsm(
    payment_method_data: Option<&api::payments::PaymentMethodData>,
    confirm: Option<bool>,
) -> storage_enums::AttemptStatus {
    match payment_method_data {
        Some(_) => match confirm {
            Some(true) => storage_enums::AttemptStatus::PaymentMethodAwaited,
            _ => storage_enums::AttemptStatus::ConfirmationAwaited,
        },
        None => storage_enums::AttemptStatus::PaymentMethodAwaited,
    }
}

pub fn payment_intent_status_fsm(
    payment_method_data: Option<&api::PaymentMethodData>,
    confirm: Option<bool>,
) -> storage_enums::IntentStatus {
    match payment_method_data {
        Some(_) => match confirm {
            Some(true) => storage_enums::IntentStatus::RequiresPaymentMethod,
            _ => storage_enums::IntentStatus::RequiresConfirmation,
        },
        None => storage_enums::IntentStatus::RequiresPaymentMethod,
    }
}

#[cfg(feature = "v1")]
pub async fn add_domain_task_to_pt<Op>(
    operation: &Op,
    state: &SessionState,
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
                    // Here, increment the count of added tasks every time a payment has been confirmed or PSync has been called
                    metrics::TASKS_ADDED_COUNT.add(
                        1,
                        router_env::metric_attributes!(("flow", format!("{:#?}", operation))),
                    );
                    super::add_process_sync_task(&*state.store, payment_attempt, stime)
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed while adding task to process tracker")
                } else {
                    // When the requeue is true, we reset the tasks count as we reset the task every time it is requeued
                    metrics::TASKS_RESET_COUNT.add(
                        1,
                        router_env::metric_attributes!(("flow", format!("{:#?}", operation))),
                    );
                    super::reset_process_sync_task(&*state.store, payment_attempt, stime)
                        .await
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

pub fn response_operation<'a, F, R, D>() -> BoxedOperation<'a, F, R, D>
where
    F: Send + Clone,
    PaymentResponse: Operation<F, R, Data = D>,
{
    Box::new(PaymentResponse)
}

pub fn validate_max_amount(
    amount: api_models::payments::Amount,
) -> CustomResult<(), errors::ApiErrorResponse> {
    match amount {
        api_models::payments::Amount::Value(value) => {
            utils::when(value.get() > consts::MAX_ALLOWED_AMOUNT, || {
                Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                    message: format!(
                        "amount should not be more than {}",
                        consts::MAX_ALLOWED_AMOUNT
                    )
                }))
            })
        }
        api_models::payments::Amount::Zero => Ok(()),
    }
}

#[cfg(feature = "v1")]
/// Check whether the customer information that is sent in the root of payments request
/// and in the customer object are same, if the values mismatch return an error
pub fn validate_customer_information(
    request: &api_models::payments::PaymentsRequest,
) -> RouterResult<()> {
    if let Some(mismatched_fields) = request.validate_customer_details_in_request() {
        let mismatched_fields = mismatched_fields.join(", ");
        Err(errors::ApiErrorResponse::PreconditionFailed {
            message: format!(
                "The field names `{mismatched_fields}` sent in both places is ambiguous"
            ),
        })?
    } else {
        Ok(())
    }
}

pub async fn validate_card_ip_blocking_for_business_profile(
    state: &SessionState,
    ip: IpAddr,
    fingerprnt: masking::Secret<String>,
    card_testing_guard_config: &diesel_models::business_profile::CardTestingGuardConfig,
) -> RouterResult<String> {
    let cache_key = format!(
        "{}_{}_{}",
        consts::CARD_IP_BLOCKING_CACHE_KEY_PREFIX,
        fingerprnt.peek(),
        ip
    );

    let unsuccessful_payment_threshold = card_testing_guard_config.card_ip_blocking_threshold;

    validate_blocking_threshold(state, unsuccessful_payment_threshold, cache_key).await
}

pub async fn validate_guest_user_card_blocking_for_business_profile(
    state: &SessionState,
    fingerprnt: masking::Secret<String>,
    customer_id: Option<id_type::CustomerId>,
    card_testing_guard_config: &diesel_models::business_profile::CardTestingGuardConfig,
) -> RouterResult<String> {
    let cache_key = format!(
        "{}_{}",
        consts::GUEST_USER_CARD_BLOCKING_CACHE_KEY_PREFIX,
        fingerprnt.peek()
    );

    let unsuccessful_payment_threshold =
        card_testing_guard_config.guest_user_card_blocking_threshold;

    if customer_id.is_none() {
        Ok(validate_blocking_threshold(state, unsuccessful_payment_threshold, cache_key).await?)
    } else {
        Ok(cache_key)
    }
}

pub async fn validate_customer_id_blocking_for_business_profile(
    state: &SessionState,
    customer_id: id_type::CustomerId,
    profile_id: &id_type::ProfileId,
    card_testing_guard_config: &diesel_models::business_profile::CardTestingGuardConfig,
) -> RouterResult<String> {
    let cache_key = format!(
        "{}_{}_{}",
        consts::CUSTOMER_ID_BLOCKING_PREFIX,
        profile_id.get_string_repr(),
        customer_id.get_string_repr(),
    );

    let unsuccessful_payment_threshold = card_testing_guard_config.customer_id_blocking_threshold;

    validate_blocking_threshold(state, unsuccessful_payment_threshold, cache_key).await
}

pub async fn validate_blocking_threshold(
    state: &SessionState,
    unsuccessful_payment_threshold: i32,
    cache_key: String,
) -> RouterResult<String> {
    match services::card_testing_guard::get_blocked_count_from_cache(state, &cache_key).await {
        Ok(Some(unsuccessful_payment_count)) => {
            if unsuccessful_payment_count >= unsuccessful_payment_threshold {
                Err(errors::ApiErrorResponse::PreconditionFailed {
                    message: "Blocked due to suspicious activity".to_string(),
                })?
            } else {
                Ok(cache_key)
            }
        }
        Ok(None) => Ok(cache_key),
        Err(error) => Err(errors::ApiErrorResponse::InternalServerError).attach_printable(error)?,
    }
}

#[cfg(feature = "v1")]
/// Get the customer details from customer field if present
/// or from the individual fields in `PaymentsRequest`
#[instrument(skip_all)]
pub fn get_customer_details_from_request(
    request: &api_models::payments::PaymentsRequest,
) -> CustomerDetails {
    let customer_id = request.get_customer_id().map(ToOwned::to_owned);

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

pub async fn get_connector_default(
    _state: &SessionState,
    request_connector: Option<serde_json::Value>,
) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
    Ok(request_connector.map_or(
        api::ConnectorChoice::Decide,
        api::ConnectorChoice::StraightThrough,
    ))
}

#[cfg(feature = "v2")]
pub async fn get_connector_data_from_request(
    state: &SessionState,
    req: Option<common_types::domain::MerchantConnectorAuthDetails>,
) -> CustomResult<api::ConnectorData, errors::ApiErrorResponse> {
    let connector = req
        .as_ref()
        .map(|connector_details| connector_details.connector_name.to_string())
        .ok_or(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "merchant_connector_details",
        })?;
    let connector_data: api::ConnectorData = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector,
        api::GetToken::Connector,
        None,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Invalid connector name received")?;
    Ok(connector_data)
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
#[allow(clippy::type_complexity)]
pub async fn create_customer_if_not_exist<'a, F: Clone, R, D>(
    _state: &SessionState,
    _operation: BoxedOperation<'a, F, R, D>,
    _payment_data: &mut PaymentData<F>,
    _req: Option<CustomerDetails>,
    _merchant_id: &id_type::MerchantId,
    _key_store: &domain::MerchantKeyStore,
    _storage_scheme: common_enums::enums::MerchantStorageScheme,
) -> CustomResult<(BoxedOperation<'a, F, R, D>, Option<domain::Customer>), errors::StorageError> {
    todo!()
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
#[allow(clippy::type_complexity)]
pub async fn create_customer_if_not_exist<'a, F: Clone, R, D>(
    state: &SessionState,
    operation: BoxedOperation<'a, F, R, D>,
    payment_data: &mut PaymentData<F>,
    req: Option<CustomerDetails>,
    merchant_id: &id_type::MerchantId,
    key_store: &domain::MerchantKeyStore,
    storage_scheme: common_enums::enums::MerchantStorageScheme,
) -> CustomResult<(BoxedOperation<'a, F, R, D>, Option<domain::Customer>), errors::StorageError> {
    let request_customer_details = req
        .get_required_value("customer")
        .change_context(errors::StorageError::ValueNotFound("customer".to_owned()))?;

    let temp_customer_data = if request_customer_details.name.is_some()
        || request_customer_details.email.is_some()
        || request_customer_details.phone.is_some()
        || request_customer_details.phone_country_code.is_some()
        || request_customer_details.tax_registration_id.is_some()
    {
        Some(CustomerData {
            name: request_customer_details.name.clone(),
            email: request_customer_details.email.clone(),
            phone: request_customer_details.phone.clone(),
            phone_country_code: request_customer_details.phone_country_code.clone(),
            tax_registration_id: request_customer_details.tax_registration_id.clone(),
        })
    } else {
        None
    };

    // Updation of Customer Details for the cases where both customer_id and specific customer
    // details are provided in Payment Update Request
    let raw_customer_details = payment_data
        .payment_intent
        .customer_details
        .clone()
        .map(|customer_details_encrypted| {
            customer_details_encrypted
                .into_inner()
                .expose()
                .parse_value::<CustomerData>("CustomerData")
        })
        .transpose()
        .change_context(errors::StorageError::DeserializationFailed)
        .attach_printable("Failed to parse customer data from payment intent")?
        .map(|parsed_customer_data| CustomerData {
            name: request_customer_details
                .name
                .clone()
                .or(parsed_customer_data.name.clone()),
            email: request_customer_details
                .email
                .clone()
                .or(parsed_customer_data.email.clone()),
            phone: request_customer_details
                .phone
                .clone()
                .or(parsed_customer_data.phone.clone()),
            phone_country_code: request_customer_details
                .phone_country_code
                .clone()
                .or(parsed_customer_data.phone_country_code.clone()),
            tax_registration_id: request_customer_details
                .tax_registration_id
                .clone()
                .or(parsed_customer_data.tax_registration_id.clone()),
        })
        .or(temp_customer_data);
    let key_manager_state = state.into();
    payment_data.payment_intent.customer_details = raw_customer_details
        .clone()
        .async_map(|customer_details| {
            create_encrypted_data(&key_manager_state, key_store, customer_details)
        })
        .await
        .transpose()
        .change_context(errors::StorageError::EncryptionError)
        .attach_printable("Unable to encrypt customer details")?;

    let customer_id = request_customer_details
        .customer_id
        .or(payment_data.payment_intent.customer_id.clone());
    let db = &*state.store;
    let key_manager_state = &state.into();
    let optional_customer = match customer_id {
        Some(customer_id) => {
            let customer_data = db
                .find_customer_optional_by_customer_id_merchant_id(
                    &customer_id,
                    merchant_id,
                    key_store,
                    storage_scheme,
                )
                .await?;
            let key = key_store.key.get_inner().peek();
            let encrypted_data = types::crypto_operation(
                key_manager_state,
                type_name!(domain::Customer),
                types::CryptoOperation::BatchEncrypt(
                    domain::FromRequestEncryptableCustomer::to_encryptable(
                        domain::FromRequestEncryptableCustomer {
                            name: request_customer_details.name.clone(),
                            email: request_customer_details
                                .email
                                .as_ref()
                                .map(|e| e.clone().expose().switch_strategy()),
                            phone: request_customer_details.phone.clone(),
                            tax_registration_id: None,
                        },
                    ),
                ),
                Identifier::Merchant(key_store.merchant_id.clone()),
                key,
            )
            .await
            .and_then(|val| val.try_into_batchoperation())
            .change_context(errors::StorageError::SerializationFailed)
            .attach_printable("Failed while encrypting Customer while Update")?;
            let encryptable_customer =
                domain::FromRequestEncryptableCustomer::from_encryptable(encrypted_data)
                    .change_context(errors::StorageError::SerializationFailed)
                    .attach_printable("Failed while encrypting Customer while Update")?;
            Some(match customer_data {
                Some(c) => {
                    // Update the customer data if new data is passed in the request
                    if request_customer_details.email.is_some()
                        | request_customer_details.name.is_some()
                        | request_customer_details.phone.is_some()
                        | request_customer_details.phone_country_code.is_some()
                        | request_customer_details.tax_registration_id.is_some()
                    {
                        let customer_update = Update {
                            name: encryptable_customer.name,
                            email: encryptable_customer.email.map(|email| {
                                let encryptable: Encryptable<
                                    masking::Secret<String, pii::EmailStrategy>,
                                > = Encryptable::new(
                                    email.clone().into_inner().switch_strategy(),
                                    email.into_encrypted(),
                                );
                                encryptable
                            }),
                            phone: Box::new(encryptable_customer.phone),
                            phone_country_code: request_customer_details.phone_country_code,
                            description: None,
                            connector_customer: Box::new(None),
                            metadata: Box::new(None),
                            address_id: None,
                            tax_registration_id: encryptable_customer.tax_registration_id,
                            last_modified_by: None,
                        };

                        db.update_customer_by_customer_id_merchant_id(
                            customer_id,
                            merchant_id.to_owned(),
                            c,
                            customer_update,
                            key_store,
                            storage_scheme,
                        )
                        .await
                    } else {
                        Ok(c)
                    }
                }
                None => {
                    let new_customer = domain::Customer {
                        customer_id,
                        merchant_id: merchant_id.to_owned(),
                        name: encryptable_customer.name,
                        email: encryptable_customer.email.map(|email| {
                            let encryptable: Encryptable<
                                masking::Secret<String, pii::EmailStrategy>,
                            > = Encryptable::new(
                                email.clone().into_inner().switch_strategy(),
                                email.into_encrypted(),
                            );
                            encryptable
                        }),
                        phone: encryptable_customer.phone,
                        phone_country_code: request_customer_details.phone_country_code.clone(),
                        description: None,
                        created_at: common_utils::date_time::now(),
                        metadata: None,
                        modified_at: common_utils::date_time::now(),
                        connector_customer: None,
                        address_id: None,
                        default_payment_method_id: None,
                        updated_by: None,
                        version: common_types::consts::API_VERSION,
                        tax_registration_id: encryptable_customer.tax_registration_id,
                        // TODO: Populate created_by from authentication context once it is integrated in auth data
                        created_by: None,
                        last_modified_by: None, // Same as created_by on creation
                    };
                    metrics::CUSTOMER_CREATED.add(1, &[]);
                    db.insert_customer(new_customer, key_store, storage_scheme)
                        .await
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
                    storage_scheme,
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

#[cfg(feature = "v1")]
pub async fn retrieve_payment_method_with_temporary_token(
    state: &SessionState,
    token: &str,
    payment_intent: &PaymentIntent,
    payment_attempt: &PaymentAttempt,
    merchant_key_store: &domain::MerchantKeyStore,
    card_token_data: Option<&domain::CardToken>,
) -> RouterResult<Option<(domain::PaymentMethodData, enums::PaymentMethod)>> {
    let (pm, supplementary_data) =
        vault::Vault::get_payment_method_data_from_locker(state, token, merchant_key_store)
            .await
            .attach_printable(
                "Payment method for given token not found or there was a problem fetching it",
            )?;

    utils::when(
        supplementary_data
            .customer_id
            .ne(&payment_intent.customer_id),
        || {
            Err(errors::ApiErrorResponse::PreconditionFailed { message: "customer associated with payment method and customer passed in payment are not same".into() })
        },
    )?;

    Ok::<_, error_stack::Report<errors::ApiErrorResponse>>(match pm {
        Some(domain::PaymentMethodData::Card(card)) => {
            let mut updated_card = card.clone();
            let mut is_card_updated = false;

            // The card_holder_name from locker retrieved card is considered if it is a non-empty string or else card_holder_name is picked
            // from payment_method_data.card_token object
            let name_on_card =
                card_token_data.and_then(|token_data| token_data.card_holder_name.clone());

            if let Some(name) = name_on_card.clone() {
                if !name.peek().is_empty() {
                    is_card_updated = true;
                    updated_card.nick_name = name_on_card;
                }
            }

            if let Some(token_data) = card_token_data {
                if let Some(cvc) = token_data.card_cvc.clone() {
                    is_card_updated = true;
                    updated_card.card_cvc = cvc;
                }
            }

            // populate additional card details from payment_attempt.payment_method_data (additional_payment_data) if not present in the locker
            if updated_card.card_issuer.is_none()
                || updated_card.card_network.is_none()
                || updated_card.card_type.is_none()
                || updated_card.card_issuing_country.is_none()
            {
                let additional_payment_method_data: Option<
                    api_models::payments::AdditionalPaymentData,
                > = payment_attempt
                    .payment_method_data
                    .clone()
                    .and_then(|data| match data {
                        serde_json::Value::Null => None, // This is to handle the case when the payment_method_data is null
                        _ => Some(data.parse_value("AdditionalPaymentData")),
                    })
                    .transpose()
                    .map_err(|err| logger::error!("Failed to parse AdditionalPaymentData {err:?}"))
                    .ok()
                    .flatten();
                if let Some(api_models::payments::AdditionalPaymentData::Card(card)) =
                    additional_payment_method_data
                {
                    is_card_updated = true;
                    updated_card.card_issuer = updated_card.card_issuer.or(card.card_issuer);
                    updated_card.card_network = updated_card.card_network.or(card.card_network);
                    updated_card.card_type = updated_card.card_type.or(card.card_type);
                    updated_card.card_issuing_country = updated_card
                        .card_issuing_country
                        .or(card.card_issuing_country);
                };
            };

            if is_card_updated {
                let updated_pm = domain::PaymentMethodData::Card(updated_card);
                vault::Vault::store_payment_method_data_in_locker(
                    state,
                    Some(token.to_owned()),
                    &updated_pm,
                    payment_intent.customer_id.to_owned(),
                    enums::PaymentMethod::Card,
                    merchant_key_store,
                )
                .await?;

                Some((updated_pm, enums::PaymentMethod::Card))
            } else {
                Some((
                    domain::PaymentMethodData::Card(card),
                    enums::PaymentMethod::Card,
                ))
            }
        }

        Some(the_pm @ domain::PaymentMethodData::Wallet(_)) => {
            Some((the_pm, enums::PaymentMethod::Wallet))
        }

        Some(the_pm @ domain::PaymentMethodData::BankTransfer(_)) => {
            Some((the_pm, enums::PaymentMethod::BankTransfer))
        }

        Some(the_pm @ domain::PaymentMethodData::BankRedirect(_)) => {
            Some((the_pm, enums::PaymentMethod::BankRedirect))
        }

        Some(the_pm @ domain::PaymentMethodData::BankDebit(_)) => {
            Some((the_pm, enums::PaymentMethod::BankDebit))
        }

        Some(_) => Err(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Payment method received from locker is unsupported by locker")?,

        None => None,
    })
}

#[cfg(feature = "v2")]
pub async fn retrieve_card_with_permanent_token(
    state: &SessionState,
    locker_id: &str,
    _payment_method_id: &id_type::GlobalPaymentMethodId,
    payment_intent: &PaymentIntent,
    card_token_data: Option<&domain::CardToken>,
    _merchant_key_store: &domain::MerchantKeyStore,
    _storage_scheme: enums::MerchantStorageScheme,
) -> RouterResult<domain::PaymentMethodData> {
    todo!()
}

pub enum VaultFetchAction {
    FetchCardDetailsFromLocker,
    FetchCardDetailsForNetworkTransactionIdFlowFromLocker,
    FetchNetworkTokenDataFromTokenizationService(String),
    FetchNetworkTokenDetailsFromLocker(api_models::payments::NetworkTokenWithNTIRef),
    NoFetchAction,
}

pub fn decide_payment_method_retrieval_action(
    is_network_tokenization_enabled: bool,
    mandate_id: Option<api_models::payments::MandateIds>,
    connector: Option<api_enums::Connector>,
    network_tokenization_supported_connectors: &HashSet<api_enums::Connector>,
    should_retry_with_pan: bool,
    network_token_requestor_ref_id: Option<String>,
) -> VaultFetchAction {
    let standard_flow = || {
        determine_standard_vault_action(
            is_network_tokenization_enabled,
            mandate_id,
            connector,
            network_tokenization_supported_connectors,
            network_token_requestor_ref_id,
        )
    };

    if should_retry_with_pan {
        VaultFetchAction::FetchCardDetailsFromLocker
    } else {
        standard_flow()
    }
}

pub async fn is_ucs_enabled(state: &SessionState, config_key: &str) -> bool {
    let db = state.store.as_ref();
    db.find_config_by_key_unwrap_or(config_key, Some("false".to_string()))
        .await
        .inspect_err(|error| {
            logger::error!(
                ?error,
                "Failed to fetch `{config_key}` UCS enabled config from DB"
            );
        })
        .ok()
        .and_then(|config| {
            config
                .config
                .parse::<bool>()
                .inspect_err(|error| {
                    logger::error!(?error, "Failed to parse `{config_key}` UCS enabled config");
                })
                .ok()
        })
        .unwrap_or(false)
}

#[derive(Debug, Clone, Deserialize)]
pub struct RolloutConfig {
    pub rollout_percent: f64,
    pub http_url: Option<String>,
    pub https_url: Option<String>,
}

impl Default for RolloutConfig {
    fn default() -> Self {
        Self {
            rollout_percent: 0.0,
            http_url: None,
            https_url: None,
        }
    }
}

// Re-export ProxyOverride from hyperswitch_interfaces
pub use hyperswitch_interfaces::types::ProxyOverride;

#[derive(Debug, Clone)]
pub struct RolloutExecutionResult {
    pub should_execute: bool,
    pub proxy_override: Option<ProxyOverride>,
}

/// Validates a proxy URL, filtering out invalid ones and logging warnings
fn validate_proxy_url(url: Option<String>, url_type: &str) -> Option<String> {
    url.and_then(|url_str| {
        if url_str.trim().is_empty() || url::Url::parse(&url_str).is_err() {
            logger::warn!(
                invalid_url = %url_str,
                url_type = url_type,
                "Invalid proxy URL in rollout config, ignoring"
            );
            None
        } else {
            Some(url_str)
        }
    })
}

/// Creates proxy override with validated URLs and logging
fn create_proxy_override(
    http_url: Option<String>,
    https_url: Option<String>,
) -> Option<ProxyOverride> {
    let validated_http = validate_proxy_url(http_url, "HTTP");
    let validated_https = validate_proxy_url(https_url, "HTTPS");

    if validated_http.is_some() || validated_https.is_some() {
        if let Some(ref http_url) = validated_http {
            logger::info!(http_url = %http_url, "Using validated HTTP proxy URL from rollout config");
        }
        if let Some(ref https_url) = validated_https {
            logger::info!(https_url = %https_url, "Using validated HTTPS proxy URL from rollout config");
        }
        Some(ProxyOverride {
            http_url: validated_http,
            https_url: validated_https,
        })
    } else {
        None
    }
}

pub async fn should_execute_based_on_rollout(
    state: &SessionState,
    config_key: &str,
) -> RouterResult<RolloutExecutionResult> {
    let db = state.store.as_ref();

    match db.find_config_by_key(config_key).await {
        Ok(rollout_config) => {
            // Try to parse as JSON first (new format), fallback to float (legacy format)
            let config_result = match serde_json::from_str::<RolloutConfig>(&rollout_config.config)
            {
                Ok(config) => Ok(config),
                Err(err) => {
                    logger::debug!(
                        error = ?err,
                        config = %rollout_config.config,
                        "Config not in JSON format, trying legacy float format"
                    );
                    // Fallback to legacy format (simple float)
                    rollout_config.config.parse::<f64>()
                        .map(|percent| RolloutConfig {
                            rollout_percent: percent,
                            http_url: None,
                            https_url: None,
                        })
                        .map_err(|err| {
                            logger::error!(error = ?err, "Failed to parse rollout config as either JSON or float");
                            err
                        })
                }
            };

            match config_result {
                Ok(config) => {
                    if !(0.0..=1.0).contains(&config.rollout_percent) {
                        logger::warn!(
                            rollout_percent = config.rollout_percent,
                            "Rollout percent out of bounds. Must be between 0.0 and 1.0"
                        );
                        let proxy_override =
                            create_proxy_override(config.http_url, config.https_url);

                        return Ok(RolloutExecutionResult {
                            should_execute: false,
                            proxy_override,
                        });
                    }

                    let sampled_value: f64 = rand::thread_rng().gen_range(0.0..1.0);
                    let should_execute = sampled_value < config.rollout_percent;

                    let proxy_override = create_proxy_override(config.http_url, config.https_url);

                    Ok(RolloutExecutionResult {
                        should_execute,
                        proxy_override,
                    })
                }
                Err(err) => {
                    logger::error!(error = ?err, "Failed to parse rollout config");
                    Ok(RolloutExecutionResult {
                        should_execute: false,
                        proxy_override: None,
                    })
                }
            }
        }
        Err(err) => {
            logger::error!(error = ?err, "Failed to fetch rollout config from DB");
            Ok(RolloutExecutionResult {
                should_execute: false,
                proxy_override: None,
            })
        }
    }
}

pub fn determine_standard_vault_action(
    is_network_tokenization_enabled: bool,
    mandate_id: Option<api_models::payments::MandateIds>,
    connector: Option<api_enums::Connector>,
    network_tokenization_supported_connectors: &HashSet<api_enums::Connector>,
    network_token_requestor_ref_id: Option<String>,
) -> VaultFetchAction {
    let is_network_transaction_id_flow = mandate_id
        .as_ref()
        .map(|mandate_ids| mandate_ids.is_network_transaction_id_flow())
        .unwrap_or(false);

    if !is_network_tokenization_enabled {
        if is_network_transaction_id_flow {
            VaultFetchAction::FetchCardDetailsForNetworkTransactionIdFlowFromLocker
        } else {
            VaultFetchAction::FetchCardDetailsFromLocker
        }
    } else {
        match mandate_id {
            Some(mandate_ids) => match mandate_ids.mandate_reference_id {
                Some(api_models::payments::MandateReferenceId::NetworkTokenWithNTI(nt_data)) => {
                    VaultFetchAction::FetchNetworkTokenDetailsFromLocker(nt_data)
                }
                Some(api_models::payments::MandateReferenceId::NetworkMandateId(_)) => {
                    VaultFetchAction::FetchCardDetailsForNetworkTransactionIdFlowFromLocker
                }
                Some(api_models::payments::MandateReferenceId::ConnectorMandateId(_)) | None => {
                    VaultFetchAction::NoFetchAction
                }
            },
            None => {
                //saved card flow
                let is_network_token_supported_connector = connector
                    .map(|conn| network_tokenization_supported_connectors.contains(&conn))
                    .unwrap_or(false);

                match (
                    is_network_token_supported_connector,
                    network_token_requestor_ref_id,
                ) {
                    (true, Some(ref_id)) => {
                        VaultFetchAction::FetchNetworkTokenDataFromTokenizationService(ref_id)
                    }
                    (false, Some(_)) | (true, None) | (false, None) => {
                        VaultFetchAction::FetchCardDetailsFromLocker
                    }
                }
            }
        }
    }
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
pub async fn retrieve_payment_method_data_with_permanent_token(
    state: &SessionState,
    locker_id: &str,
    _payment_method_id: &str,
    payment_intent: &PaymentIntent,
    card_token_data: Option<&domain::CardToken>,
    merchant_key_store: &domain::MerchantKeyStore,
    _storage_scheme: enums::MerchantStorageScheme,
    mandate_id: Option<api_models::payments::MandateIds>,
    payment_method_info: domain::PaymentMethod,
    business_profile: &domain::Profile,
    connector: Option<String>,
    should_retry_with_pan: bool,
    vault_data: Option<&domain_payments::VaultData>,
) -> RouterResult<domain::PaymentMethodData> {
    let customer_id = payment_intent
        .customer_id
        .as_ref()
        .get_required_value("customer_id")
        .change_context(errors::ApiErrorResponse::UnprocessableEntity {
            message: "no customer id provided for the payment".to_string(),
        })?;

    let network_tokenization_supported_connectors = &state
        .conf
        .network_tokenization_supported_connectors
        .connector_list;

    let connector_variant = connector
        .as_ref()
        .map(|conn| {
            api_enums::Connector::from_str(conn.as_str())
                .change_context(errors::ApiErrorResponse::InvalidDataValue {
                    field_name: "connector",
                })
                .attach_printable_lazy(|| format!("unable to parse connector name {connector:?}"))
        })
        .transpose()?;

    let vault_fetch_action = decide_payment_method_retrieval_action(
        business_profile.is_network_tokenization_enabled,
        mandate_id,
        connector_variant,
        network_tokenization_supported_connectors,
        should_retry_with_pan,
        payment_method_info
            .network_token_requestor_reference_id
            .clone(),
    );

    let co_badged_card_data = payment_method_info
        .get_payment_methods_data()
        .and_then(|payment_methods_data| payment_methods_data.get_co_badged_card_data());

    match vault_fetch_action {
        VaultFetchAction::FetchCardDetailsFromLocker => {
            let card = vault_data
                .and_then(|vault_data| vault_data.get_card_vault_data())
                .map(Ok)
                .async_unwrap_or_else(|| async {
                    Box::pin(fetch_card_details_from_locker(
                        state,
                        customer_id,
                        &payment_intent.merchant_id,
                        locker_id,
                        card_token_data,
                        co_badged_card_data,
                        payment_method_info,
                        merchant_key_store,
                    ))
                    .await
                })
                .await?;

            Ok(domain::PaymentMethodData::Card(card))
        }
        VaultFetchAction::FetchCardDetailsForNetworkTransactionIdFlowFromLocker => {
            fetch_card_details_for_network_transaction_flow_from_locker(
                state,
                customer_id,
                &payment_intent.merchant_id,
                locker_id,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("failed to fetch card information from the permanent locker")
        }
        VaultFetchAction::FetchNetworkTokenDataFromTokenizationService(
            network_token_requestor_ref_id,
        ) => {
            logger::info!("Fetching network token data from tokenization service");
            match network_tokenization::get_token_from_tokenization_service(
                state,
                network_token_requestor_ref_id,
                &payment_method_info,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("failed to fetch network token data from tokenization service")
            {
                Ok(network_token_data) => {
                    Ok(domain::PaymentMethodData::NetworkToken(network_token_data))
                }
                Err(err) => {
                    logger::info!(
                        "Failed to fetch network token data from tokenization service {err:?}"
                    );
                    logger::info!("Falling back to fetch card details from locker");
                    Ok(domain::PaymentMethodData::Card(
                        vault_data
                            .and_then(|vault_data| vault_data.get_card_vault_data())
                            .map(Ok)
                            .async_unwrap_or_else(|| async {
                                Box::pin(fetch_card_details_from_locker(
                                    state,
                                    customer_id,
                                    &payment_intent.merchant_id,
                                    locker_id,
                                    card_token_data,
                                    co_badged_card_data,
                                    payment_method_info,
                                    merchant_key_store,
                                ))
                                .await
                            })
                            .await?,
                    ))
                }
            }
        }
        VaultFetchAction::FetchNetworkTokenDetailsFromLocker(nt_data) => {
            if let Some(network_token_locker_id) =
                payment_method_info.network_token_locker_id.as_ref()
            {
                let network_token_data = vault_data
                    .and_then(|vault_data| vault_data.get_network_token_data())
                    .map(Ok)
                    .async_unwrap_or_else(|| async {
                        fetch_network_token_details_from_locker(
                            state,
                            customer_id,
                            &payment_intent.merchant_id,
                            network_token_locker_id,
                            nt_data,
                        )
                        .await
                    })
                    .await?;
                Ok(domain::PaymentMethodData::NetworkToken(network_token_data))
            } else {
                Err(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Network token locker id is not present")
            }
        }
        VaultFetchAction::NoFetchAction => Err(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Payment method data is not present"),
    }
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
pub async fn retrieve_card_with_permanent_token_for_external_authentication(
    state: &SessionState,
    locker_id: &str,
    payment_intent: &PaymentIntent,
    card_token_data: Option<&domain::CardToken>,
    _merchant_key_store: &domain::MerchantKeyStore,
    _storage_scheme: enums::MerchantStorageScheme,
) -> RouterResult<domain::PaymentMethodData> {
    let customer_id = payment_intent
        .customer_id
        .as_ref()
        .get_required_value("customer_id")
        .change_context(errors::ApiErrorResponse::UnprocessableEntity {
            message: "no customer id provided for the payment".to_string(),
        })?;
    Ok(domain::PaymentMethodData::Card(
        fetch_card_details_from_internal_locker(
            state,
            customer_id,
            &payment_intent.merchant_id,
            locker_id,
            card_token_data,
            None,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("failed to fetch card information from the permanent locker")?,
    ))
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
pub async fn fetch_card_details_from_locker(
    state: &SessionState,
    customer_id: &id_type::CustomerId,
    merchant_id: &id_type::MerchantId,
    locker_id: &str,
    card_token_data: Option<&domain::CardToken>,
    co_badged_card_data: Option<api_models::payment_methods::CoBadgedCardData>,
    payment_method_info: domain::PaymentMethod,
    merchant_key_store: &domain::MerchantKeyStore,
) -> RouterResult<domain::Card> {
    match &payment_method_info.vault_source_details.clone() {
        domain::PaymentMethodVaultSourceDetails::ExternalVault {
            ref external_vault_source,
        } => {
            fetch_card_details_from_external_vault(
                state,
                merchant_id,
                card_token_data,
                co_badged_card_data,
                payment_method_info,
                merchant_key_store,
                external_vault_source,
            )
            .await
        }
        domain::PaymentMethodVaultSourceDetails::InternalVault => {
            fetch_card_details_from_internal_locker(
                state,
                customer_id,
                merchant_id,
                locker_id,
                card_token_data,
                co_badged_card_data,
            )
            .await
        }
    }
}

#[cfg(feature = "v1")]
pub async fn fetch_card_details_from_internal_locker(
    state: &SessionState,
    customer_id: &id_type::CustomerId,
    merchant_id: &id_type::MerchantId,
    locker_id: &str,
    card_token_data: Option<&domain::CardToken>,
    co_badged_card_data: Option<api_models::payment_methods::CoBadgedCardData>,
) -> RouterResult<domain::Card> {
    logger::debug!("Fetching card details from locker");
    let card = cards::get_card_from_locker(state, customer_id, merchant_id, locker_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("failed to fetch card information from the permanent locker")?;

    // The card_holder_name from locker retrieved card is considered if it is a non-empty string or else card_holder_name is picked
    // from payment_method_data.card_token object
    let name_on_card = if let Some(name) = card.name_on_card.clone() {
        if name.clone().expose().is_empty() {
            card_token_data
                .and_then(|token_data| token_data.card_holder_name.clone())
                .or(Some(name))
        } else {
            card.name_on_card
        }
    } else {
        card_token_data.and_then(|token_data| token_data.card_holder_name.clone())
    };

    let api_card = api::Card {
        card_number: card.card_number,
        card_holder_name: name_on_card,
        card_exp_month: card.card_exp_month,
        card_exp_year: card.card_exp_year,
        card_cvc: card_token_data
            .cloned()
            .unwrap_or_default()
            .card_cvc
            .unwrap_or_default(),
        card_issuer: None,
        nick_name: card.nick_name.map(masking::Secret::new),
        card_network: card
            .card_brand
            .map(|card_brand| enums::CardNetwork::from_str(&card_brand))
            .transpose()
            .map_err(|e| {
                logger::error!("Failed to parse card network {e:?}");
            })
            .ok()
            .flatten(),
        card_type: None,
        card_issuing_country: None,
        bank_code: None,
    };
    Ok(domain::Card::from((api_card, co_badged_card_data)))
}

#[cfg(feature = "v1")]
pub async fn fetch_card_details_from_external_vault(
    state: &SessionState,
    merchant_id: &id_type::MerchantId,
    card_token_data: Option<&domain::CardToken>,
    co_badged_card_data: Option<api_models::payment_methods::CoBadgedCardData>,
    payment_method_info: domain::PaymentMethod,
    merchant_key_store: &domain::MerchantKeyStore,
    external_vault_mca_id: &id_type::MerchantConnectorAccountId,
) -> RouterResult<domain::Card> {
    let merchant_connector_account_details = state
        .store
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            merchant_id,
            external_vault_mca_id,
            merchant_key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: external_vault_mca_id.get_string_repr().to_string(),
        })?;

    let vault_resp = vault::retrieve_payment_method_from_vault_external_v1(
        state,
        merchant_id,
        &payment_method_info,
        merchant_connector_account_details,
    )
    .await?;

    let payment_methods_data = payment_method_info.get_payment_methods_data();

    match vault_resp {
        hyperswitch_domain_models::vault::PaymentMethodVaultingData::Card(card) => Ok(
            domain::Card::from((card, card_token_data, co_badged_card_data)),
        ),
        hyperswitch_domain_models::vault::PaymentMethodVaultingData::CardNumber(card_number) => {
            let payment_methods_data = payment_methods_data
                .get_required_value("PaymentMethodsData")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Payment methods data not present")?;

            let card = payment_methods_data
                .get_card_details()
                .get_required_value("CardDetails")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Card details not present")?;

            Ok(
                domain::Card::try_from((card_number, card_token_data, co_badged_card_data, card))
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to generate card data")?,
            )
        }
        hyperswitch_domain_models::vault::PaymentMethodVaultingData::NetworkToken(_) => {
            Err(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Network Token not supproted")
        }
    }
}
#[cfg(feature = "v1")]
pub async fn fetch_network_token_details_from_locker(
    state: &SessionState,
    customer_id: &id_type::CustomerId,
    merchant_id: &id_type::MerchantId,
    network_token_locker_id: &str,
    network_transaction_data: api_models::payments::NetworkTokenWithNTIRef,
) -> RouterResult<domain::NetworkTokenData> {
    let mut token_data =
        cards::get_card_from_locker(state, customer_id, merchant_id, network_token_locker_id)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "failed to fetch network token information from the permanent locker",
            )?;
    let expiry = network_transaction_data
        .token_exp_month
        .zip(network_transaction_data.token_exp_year);
    if let Some((exp_month, exp_year)) = expiry {
        token_data.card_exp_month = exp_month;
        token_data.card_exp_year = exp_year;
    }

    let card_network = token_data
        .card_brand
        .map(|card_brand| enums::CardNetwork::from_str(&card_brand))
        .transpose()
        .map_err(|e| {
            logger::error!("Failed to parse card network {e:?}");
        })
        .ok()
        .flatten();

    let network_token_data = domain::NetworkTokenData {
        token_number: token_data.card_number,
        token_cryptogram: None,
        token_exp_month: token_data.card_exp_month,
        token_exp_year: token_data.card_exp_year,
        nick_name: token_data.nick_name.map(masking::Secret::new),
        card_issuer: None,
        card_network,
        card_type: None,
        card_issuing_country: None,
        bank_code: None,
        eci: None,
    };
    Ok(network_token_data)
}

#[cfg(feature = "v1")]
pub async fn fetch_card_details_for_network_transaction_flow_from_locker(
    state: &SessionState,
    customer_id: &id_type::CustomerId,
    merchant_id: &id_type::MerchantId,
    locker_id: &str,
) -> RouterResult<domain::PaymentMethodData> {
    let card_details_from_locker =
        cards::get_card_from_locker(state, customer_id, merchant_id, locker_id)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("failed to fetch card details from locker")?;

    let card_network = card_details_from_locker
        .card_brand
        .map(|card_brand| enums::CardNetwork::from_str(&card_brand))
        .transpose()
        .map_err(|e| {
            logger::error!("Failed to parse card network {e:?}");
        })
        .ok()
        .flatten();

    let card_details_for_network_transaction_id =
        hyperswitch_domain_models::payment_method_data::CardDetailsForNetworkTransactionId {
            card_number: card_details_from_locker.card_number,
            card_exp_month: card_details_from_locker.card_exp_month,
            card_exp_year: card_details_from_locker.card_exp_year,
            card_issuer: None,
            card_network,
            card_type: None,
            card_issuing_country: None,
            bank_code: None,
            nick_name: card_details_from_locker.nick_name.map(masking::Secret::new),
            card_holder_name: card_details_from_locker.name_on_card.clone(),
        };

    Ok(
        domain::PaymentMethodData::CardDetailsForNetworkTransactionId(
            card_details_for_network_transaction_id,
        ),
    )
}

#[cfg(feature = "v2")]
pub async fn retrieve_payment_method_from_db_with_token_data(
    state: &SessionState,
    merchant_key_store: &domain::MerchantKeyStore,
    token_data: &storage::PaymentTokenData,
    storage_scheme: storage::enums::MerchantStorageScheme,
) -> RouterResult<Option<domain::PaymentMethod>> {
    todo!()
}

#[cfg(feature = "v1")]
pub async fn retrieve_payment_method_from_db_with_token_data(
    state: &SessionState,
    merchant_key_store: &domain::MerchantKeyStore,
    token_data: &storage::PaymentTokenData,
    storage_scheme: storage::enums::MerchantStorageScheme,
) -> RouterResult<Option<domain::PaymentMethod>> {
    match token_data {
        storage::PaymentTokenData::PermanentCard(data) => {
            if let Some(ref payment_method_id) = data.payment_method_id {
                state
                    .store
                    .find_payment_method(merchant_key_store, payment_method_id, storage_scheme)
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)
                    .attach_printable("error retrieving payment method from DB")
                    .map(Some)
            } else {
                Ok(None)
            }
        }

        storage::PaymentTokenData::WalletToken(data) => state
            .store
            .find_payment_method(merchant_key_store, &data.payment_method_id, storage_scheme)
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)
            .attach_printable("error retrieveing payment method from DB")
            .map(Some),

        storage::PaymentTokenData::Temporary(_)
        | storage::PaymentTokenData::TemporaryGeneric(_)
        | storage::PaymentTokenData::Permanent(_)
        | storage::PaymentTokenData::AuthBankDebit(_) => Ok(None),
    }
}

#[cfg(feature = "v1")]
pub async fn retrieve_payment_token_data(
    state: &SessionState,
    token: String,
    payment_method: Option<storage_enums::PaymentMethod>,
) -> RouterResult<storage::PaymentTokenData> {
    let redis_conn = state
        .store
        .get_redis_conn()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get redis connection")?;

    let key = format!(
        "pm_token_{}_{}_hyperswitch",
        token,
        payment_method.get_required_value("payment_method")?
    );

    let token_data_string = redis_conn
        .get_key::<Option<String>>(&key.into())
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch the token from redis")?
        .ok_or(error_stack::Report::new(
            errors::ApiErrorResponse::UnprocessableEntity {
                message: "Token is invalid or expired".to_owned(),
            },
        ))?;

    let token_data_result = token_data_string
        .clone()
        .parse_struct("PaymentTokenData")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("failed to deserialize hyperswitch token data");

    let token_data = match token_data_result {
        Ok(data) => data,
        Err(e) => {
            // The purpose of this logic is backwards compatibility to support tokens
            // in redis that might be following the old format.
            if token_data_string.starts_with('{') {
                return Err(e);
            } else {
                storage::PaymentTokenData::temporary_generic(token_data_string)
            }
        }
    };

    Ok(token_data)
}

#[cfg(feature = "v2")]
pub async fn make_pm_data<'a, F: Clone, R, D>(
    _operation: BoxedOperation<'a, F, R, D>,
    _state: &'a SessionState,
    _payment_data: &mut PaymentData<F>,
    _merchant_key_store: &domain::MerchantKeyStore,
    _customer: &Option<domain::Customer>,
    _storage_scheme: common_enums::enums::MerchantStorageScheme,
    _business_profile: Option<&domain::Profile>,
) -> RouterResult<(
    BoxedOperation<'a, F, R, D>,
    Option<domain::PaymentMethodData>,
    Option<String>,
)> {
    todo!()
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
pub async fn make_pm_data<'a, F: Clone, R, D>(
    operation: BoxedOperation<'a, F, R, D>,
    state: &'a SessionState,
    payment_data: &mut PaymentData<F>,
    merchant_key_store: &domain::MerchantKeyStore,
    customer: &Option<domain::Customer>,
    storage_scheme: common_enums::enums::MerchantStorageScheme,
    business_profile: &domain::Profile,
    should_retry_with_pan: bool,
) -> RouterResult<(
    BoxedOperation<'a, F, R, D>,
    Option<domain::PaymentMethodData>,
    Option<String>,
)> {
    use super::OperationSessionSetters;
    use crate::core::payments::OperationSessionGetters;

    let request = payment_data.payment_method_data.clone();

    let mut card_token_data = payment_data
        .payment_method_data
        .clone()
        .and_then(|pmd| match pmd {
            domain::PaymentMethodData::CardToken(token_data) => Some(token_data),
            _ => None,
        })
        .or(Some(domain::CardToken::default()));

    if let Some(cvc) = payment_data.card_cvc.clone() {
        if let Some(token_data) = card_token_data.as_mut() {
            token_data.card_cvc = Some(cvc);
        }
    }

    if payment_data.token_data.is_none() {
        if let Some(payment_method_info) = &payment_data.payment_method_info {
            if payment_method_info.get_payment_method_type()
                == Some(storage_enums::PaymentMethod::Card)
            {
                payment_data.token_data =
                    Some(storage::PaymentTokenData::PermanentCard(CardTokenData {
                        payment_method_id: Some(payment_method_info.get_id().clone()),
                        locker_id: payment_method_info
                            .locker_id
                            .clone()
                            .or(Some(payment_method_info.get_id().clone())),
                        token: payment_method_info
                            .locker_id
                            .clone()
                            .unwrap_or(payment_method_info.get_id().clone()),
                        network_token_locker_id: payment_method_info
                            .network_token_requestor_reference_id
                            .clone()
                            .or(Some(payment_method_info.get_id().clone())),
                    }));
            }
        }
    }

    let mandate_id = payment_data.mandate_id.clone();

    // TODO: Handle case where payment method and token both are present in request properly.
    let (payment_method, pm_id) = match (&request, payment_data.token_data.as_ref()) {
        (_, Some(hyperswitch_token)) => {
            let existing_vault_data = payment_data.get_vault_operation();

            let vault_data = existing_vault_data.and_then(|data| match data {
                domain_payments::VaultOperation::ExistingVaultData(vault_data) => Some(vault_data),
                domain_payments::VaultOperation::SaveCardData(_)
                | domain_payments::VaultOperation::SaveCardAndNetworkTokenData(_) => None,
            });

            let pm_data = Box::pin(payment_methods::retrieve_payment_method_with_token(
                state,
                merchant_key_store,
                hyperswitch_token,
                &payment_data.payment_intent,
                &payment_data.payment_attempt,
                card_token_data.as_ref(),
                customer,
                storage_scheme,
                mandate_id,
                payment_data.payment_method_info.clone(),
                business_profile,
                should_retry_with_pan,
                vault_data,
            ))
            .await;

            let payment_method_details = pm_data.attach_printable("in 'make_pm_data'")?;

            if let Some(ref payment_method_data) = payment_method_details.payment_method_data {
                let updated_vault_operation =
                    domain_payments::VaultOperation::get_updated_vault_data(
                        existing_vault_data,
                        payment_method_data,
                    );

                if let Some(vault_operation) = updated_vault_operation {
                    payment_data.set_vault_operation(vault_operation);
                }

                // Temporarily store payment method data along with the cvc in redis for saved card payments, if required by the connector based on its configs
                if payment_data.token.is_none() {
                    let (_, payment_token) = payment_methods::retrieve_payment_method_core(
                        &Some(payment_method_data.clone()),
                        state,
                        &payment_data.payment_intent,
                        &payment_data.payment_attempt,
                        merchant_key_store,
                        Some(business_profile),
                    )
                    .await?;

                    payment_data.token = payment_token;
                }
            };

            Ok::<_, error_stack::Report<errors::ApiErrorResponse>>(
                if let Some(payment_method_data) = payment_method_details.payment_method_data {
                    payment_data.payment_attempt.payment_method =
                        payment_method_details.payment_method;
                    (
                        Some(payment_method_data),
                        payment_method_details.payment_method_id,
                    )
                } else {
                    (None, payment_method_details.payment_method_id)
                },
            )
        }

        (Some(_), _) => {
            let (payment_method_data, payment_token) =
                payment_methods::retrieve_payment_method_core(
                    &request,
                    state,
                    &payment_data.payment_intent,
                    &payment_data.payment_attempt,
                    merchant_key_store,
                    Some(business_profile),
                )
                .await?;

            payment_data.token = payment_token;

            Ok((payment_method_data, None))
        }
        _ => Ok((None, None)),
    }?;

    Ok((operation, payment_method, pm_id))
}

#[cfg(feature = "v1")]
pub async fn store_in_vault_and_generate_ppmt(
    state: &SessionState,
    payment_method_data: &domain::PaymentMethodData,
    payment_intent: &PaymentIntent,
    payment_attempt: &PaymentAttempt,
    payment_method: enums::PaymentMethod,
    merchant_key_store: &domain::MerchantKeyStore,
    business_profile: Option<&domain::Profile>,
) -> RouterResult<String> {
    let router_token = vault::Vault::store_payment_method_data_in_locker(
        state,
        None,
        payment_method_data,
        payment_intent.customer_id.to_owned(),
        payment_method,
        merchant_key_store,
    )
    .await?;
    let parent_payment_method_token = generate_id(consts::ID_LENGTH, "token");
    let key_for_hyperswitch_token = payment_attempt.get_payment_method().map(|payment_method| {
        payment_methods_handler::ParentPaymentMethodToken::create_key_for_token((
            &parent_payment_method_token,
            payment_method,
        ))
    });

    let intent_fulfillment_time = business_profile
        .and_then(|b_profile| b_profile.get_order_fulfillment_time())
        .unwrap_or(consts::DEFAULT_FULFILLMENT_TIME);

    if let Some(key_for_hyperswitch_token) = key_for_hyperswitch_token {
        key_for_hyperswitch_token
            .insert(
                intent_fulfillment_time,
                storage::PaymentTokenData::temporary_generic(router_token),
                state,
            )
            .await?;
    };
    Ok(parent_payment_method_token)
}

#[cfg(feature = "v2")]
pub async fn store_payment_method_data_in_vault(
    state: &SessionState,
    payment_attempt: &PaymentAttempt,
    payment_intent: &PaymentIntent,
    payment_method: enums::PaymentMethod,
    payment_method_data: &domain::PaymentMethodData,
    merchant_key_store: &domain::MerchantKeyStore,
    business_profile: Option<&domain::Profile>,
) -> RouterResult<Option<String>> {
    todo!()
}

#[cfg(feature = "v1")]
pub async fn store_payment_method_data_in_vault(
    state: &SessionState,
    payment_attempt: &PaymentAttempt,
    payment_intent: &PaymentIntent,
    payment_method: enums::PaymentMethod,
    payment_method_data: &domain::PaymentMethodData,
    merchant_key_store: &domain::MerchantKeyStore,
    business_profile: Option<&domain::Profile>,
) -> RouterResult<Option<String>> {
    if should_store_payment_method_data_in_vault(
        &state.conf.temp_locker_enable_config,
        payment_attempt.connector.clone(),
        payment_method,
    ) || payment_intent.request_external_three_ds_authentication == Some(true)
    {
        let parent_payment_method_token = store_in_vault_and_generate_ppmt(
            state,
            payment_method_data,
            payment_intent,
            payment_attempt,
            payment_method,
            merchant_key_store,
            business_profile,
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
            && status != storage_enums::IntentStatus::PartiallyCapturedAndCapturable
            && status != storage_enums::IntentStatus::PartiallyAuthorizedAndRequiresCapture
            && status != storage_enums::IntentStatus::Processing,
        || {
            Err(report!(errors::ApiErrorResponse::PaymentUnexpectedState {
                field_name: "payment.status".to_string(),
                current_flow: "captured".to_string(),
                current_value: status.to_string(),
                states: "requires_capture, partially_captured_and_capturable, processing"
                    .to_string()
            }))
        },
    )
}

#[instrument(skip_all)]
pub(crate) fn validate_amount_to_capture(
    amount: i64,
    amount_to_capture: Option<i64>,
) -> RouterResult<()> {
    utils::when(amount_to_capture.is_some_and(|value| value <= 0), || {
        Err(report!(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "amount".to_string(),
            expected_format: "positive integer".to_string(),
        }))
    })?;

    utils::when(
        amount_to_capture.is_some() && (Some(amount) < amount_to_capture),
        || {
            Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "amount_to_capture is greater than amount".to_string()
            }))
        },
    )
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub(crate) fn validate_payment_method_fields_present(
    req: &api_models::payments::PaymentsRequest,
) -> RouterResult<()> {
    let payment_method_data =
        req.payment_method_data
            .as_ref()
            .and_then(|request_payment_method_data| {
                request_payment_method_data.payment_method_data.as_ref()
            });
    utils::when(
        req.payment_method.is_none() && payment_method_data.is_some(),
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
            && payment_method_data.is_none()
            && req.payment_token.is_none()
            && req.recurring_details.is_none()
            && req.ctp_service_details.is_none(),
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
        req.payment_method.is_some() && payment_method_data.is_some(),
        || {
            payment_method_data
                .cloned()
                .map_or(Ok(()), |payment_method_data| {
                    req.payment_method.map_or(Ok(()), |req_payment_method| {
                        validate_payment_method_and_payment_method_data(
                            payment_method_data,
                            req_payment_method,
                        )
                    })
                })
        },
    )?;

    Ok(())
}

pub fn check_force_psync_precondition(status: storage_enums::AttemptStatus) -> bool {
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

#[cfg(all(feature = "olap", feature = "v1"))]
pub(super) async fn filter_by_constraints(
    state: &SessionState,
    constraints: &PaymentIntentFetchConstraints,
    merchant_id: &id_type::MerchantId,
    key_store: &domain::MerchantKeyStore,
    storage_scheme: storage_enums::MerchantStorageScheme,
) -> CustomResult<Vec<PaymentIntent>, errors::StorageError> {
    let db = &*state.store;
    let result = db
        .filter_payment_intent_by_constraints(merchant_id, constraints, key_store, storage_scheme)
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
                message: format!("limit should be in between 1 and {PAYMENTS_LIST_MAX_LIMIT_V1}"),
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
            message: format!("limit should be in between 1 and {PAYMENTS_LIST_MAX_LIMIT_V2}"),
        })
    })?;
    Ok(())
}

#[cfg(feature = "v1")]
pub fn get_handle_response_url(
    payment_id: id_type::PaymentId,
    business_profile: &domain::Profile,
    response: &api::PaymentsResponse,
    connector: String,
) -> RouterResult<api::RedirectionResponse> {
    let payments_return_url = response.return_url.as_ref();

    let redirection_response = make_pg_redirect_response(payment_id, response, connector);

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

#[cfg(feature = "v1")]
pub fn get_handle_response_url_for_modular_authentication(
    authentication_id: id_type::AuthenticationId,
    business_profile: &domain::Profile,
    response: &api_models::authentication::AuthenticationAuthenticateResponse,
    connector: String,
    return_url: Option<String>,
    client_secret: Option<&masking::Secret<String>>,
    amount: Option<MinorUnit>,
) -> RouterResult<api::RedirectionResponse> {
    let authentication_return_url = return_url;
    let trans_status = response
        .transaction_status
        .clone()
        .get_required_value("transaction_status")?;

    let redirection_response = make_pg_redirect_response_for_authentication(
        authentication_id,
        connector,
        amount,
        trans_status,
    );

    let return_url = make_merchant_url_with_response_for_authentication(
        business_profile,
        redirection_response,
        authentication_return_url.as_ref(),
        client_secret,
        None,
    )
    .attach_printable("Failed to make merchant url with response")?;

    make_url_with_signature(&return_url, business_profile)
}

#[cfg(feature = "v1")]
pub fn make_merchant_url_with_response_for_authentication(
    business_profile: &domain::Profile,
    redirection_response: hyperswitch_domain_models::authentication::PgRedirectResponseForAuthentication,
    request_return_url: Option<&String>,
    client_secret: Option<&masking::Secret<String>>,
    manual_retry_allowed: Option<bool>,
) -> RouterResult<String> {
    // take return url if provided in the request else use merchant return url
    let url = request_return_url
        .or(business_profile.return_url.as_ref())
        .get_required_value("return_url")?;

    let status_check = redirection_response.status;

    let authentication_client_secret = client_secret
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Expected client secret to be `Some`")?;

    let authentication_id = redirection_response
        .authentication_id
        .get_string_repr()
        .to_owned();
    let merchant_url_with_response = if business_profile.redirect_to_merchant_with_http_post {
        url::Url::parse_with_params(
            url,
            &[
                ("status", status_check.to_string()),
                ("authentication_id", authentication_id),
                (
                    "authentication_client_secret",
                    authentication_client_secret.peek().to_string(),
                ),
                (
                    "manual_retry_allowed",
                    manual_retry_allowed.unwrap_or(false).to_string(),
                ),
            ],
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to parse the url with param")?
    } else {
        let amount = redirection_response.amount.get_required_value("amount")?;
        url::Url::parse_with_params(
            url,
            &[
                ("status", status_check.to_string()),
                ("authentication_id", authentication_id),
                (
                    "authentication_client_secret",
                    authentication_client_secret.peek().to_string(),
                ),
                ("amount", amount.to_string()),
                (
                    "manual_retry_allowed",
                    manual_retry_allowed.unwrap_or(false).to_string(),
                ),
            ],
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to parse the url with param")?
    };

    Ok(merchant_url_with_response.to_string())
}

#[cfg(feature = "v1")]
pub fn make_merchant_url_with_response(
    business_profile: &domain::Profile,
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
        .attach_printable("Expected client secret to be `Some`")?;

    let payment_id = redirection_response.payment_id.get_string_repr().to_owned();
    let merchant_url_with_response = if business_profile.redirect_to_merchant_with_http_post {
        url::Url::parse_with_params(
            url,
            &[
                ("status", status_check.to_string()),
                ("payment_id", payment_id),
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
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to parse the url with param")?
    } else {
        let amount = redirection_response.amount.get_required_value("amount")?;
        url::Url::parse_with_params(
            url,
            &[
                ("status", status_check.to_string()),
                ("payment_id", payment_id),
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
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to parse the url with param")?
    };

    Ok(merchant_url_with_response.to_string())
}

#[cfg(feature = "v1")]
pub async fn make_ephemeral_key(
    state: SessionState,
    customer_id: id_type::CustomerId,
    merchant_id: id_type::MerchantId,
) -> errors::RouterResponse<ephemeral_key::EphemeralKey> {
    let store = &state.store;
    let id = utils::generate_id(consts::ID_LENGTH, "eki");
    let secret = format!("epk_{}", &Uuid::new_v4().simple().to_string());
    let ek = ephemeral_key::EphemeralKeyNew {
        id,
        customer_id,
        merchant_id: merchant_id.to_owned(),
        secret,
    };
    let ek = store
        .create_ephemeral_key(ek, state.conf.eph_key.validity)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to create ephemeral key")?;
    Ok(services::ApplicationResponse::Json(ek))
}

#[cfg(feature = "v2")]
pub async fn make_client_secret(
    state: SessionState,
    resource_id: api_models::ephemeral_key::ResourceId,
    platform: domain::Platform,
    headers: &actix_web::http::header::HeaderMap,
) -> errors::RouterResponse<ClientSecretResponse> {
    let db = &state.store;

    match &resource_id {
        api_models::ephemeral_key::ResourceId::Customer(global_customer_id) => {
            db.find_customer_by_global_id(
                global_customer_id,
                platform.get_processor().get_key_store(),
                platform.get_processor().get_account().storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)?;
        }
    }

    let resource_id = match resource_id {
        api_models::ephemeral_key::ResourceId::Customer(global_customer_id) => {
            common_utils::types::authentication::ResourceId::Customer(global_customer_id)
        }
    };

    let client_secret = create_client_secret(
        &state,
        platform.get_processor().get_account().get_id(),
        resource_id,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Unable to create client secret")?;

    let response = ClientSecretResponse::foreign_try_from(client_secret)
        .attach_printable("Only customer is supported as resource_id in response")?;
    Ok(services::ApplicationResponse::Json(response))
}

#[cfg(feature = "v2")]
pub async fn create_client_secret(
    state: &SessionState,
    merchant_id: &id_type::MerchantId,
    resource_id: common_utils::types::authentication::ResourceId,
) -> RouterResult<ephemeral_key::ClientSecretType> {
    use common_utils::generate_time_ordered_id;

    let store = &state.store;
    let id = id_type::ClientSecretId::generate();
    let secret = masking::Secret::new(generate_time_ordered_id("cs"));

    let client_secret = ephemeral_key::ClientSecretTypeNew {
        id,
        merchant_id: merchant_id.to_owned(),
        secret,
        resource_id,
    };
    let client_secret = store
        .create_client_secret(client_secret, state.conf.eph_key.validity)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to create client secret")?;
    Ok(client_secret)
}

#[cfg(feature = "v1")]
pub async fn delete_ephemeral_key(
    state: SessionState,
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

#[cfg(feature = "v2")]
pub async fn delete_client_secret(
    state: SessionState,
    ephemeral_key_id: String,
) -> errors::RouterResponse<ClientSecretResponse> {
    let db = state.store.as_ref();
    let ephemeral_key = db
        .delete_client_secret(&ephemeral_key_id)
        .await
        .map_err(|err| match err.current_context() {
            errors::StorageError::ValueNotFound(_) => {
                err.change_context(errors::ApiErrorResponse::GenericNotFoundError {
                    message: "Ephemeral Key not found".to_string(),
                })
            }
            _ => err.change_context(errors::ApiErrorResponse::InternalServerError),
        })
        .attach_printable("Unable to delete ephemeral key")?;

    let response = ClientSecretResponse::foreign_try_from(ephemeral_key)
        .attach_printable("Only customer is supported as resource_id in response")?;
    Ok(services::ApplicationResponse::Json(response))
}

#[cfg(feature = "v1")]
pub fn make_pg_redirect_response(
    payment_id: id_type::PaymentId,
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

#[cfg(feature = "v1")]
pub fn make_pg_redirect_response_for_authentication(
    authentication_id: id_type::AuthenticationId,
    connector: String,
    amount: Option<MinorUnit>,
    trans_status: common_enums::TransactionStatus,
) -> hyperswitch_domain_models::authentication::PgRedirectResponseForAuthentication {
    hyperswitch_domain_models::authentication::PgRedirectResponseForAuthentication {
        authentication_id,
        status: trans_status,
        gateway_id: connector,
        customer_id: None,
        amount,
    }
}

#[cfg(feature = "v1")]
pub fn make_url_with_signature(
    redirect_url: &str,
    business_profile: &domain::Profile,
) -> RouterResult<api::RedirectionResponse> {
    let mut url = url::Url::parse(redirect_url)
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
    merchant_id: id_type::MerchantId,
    payment_id: id_type::PaymentId,
    connector: String,
    setup_mandate_details: Option<MandateData>,
    customer_id: &Option<id_type::CustomerId>,
    payment_method_id: String,
    connector_mandate_id: Option<pii::SecretSerdeValue>,
    network_txn_id: Option<String>,
    payment_method_data_option: Option<domain::payments::PaymentMethodData>,
    mandate_reference: Option<MandateReference>,
    merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,
) -> CustomResult<Option<storage::MandateNew>, errors::ApiErrorResponse> {
    match (setup_mandate_details, customer_id) {
        (Some(data), Some(cus_id)) => {
            let mandate_id = utils::generate_id(consts::ID_LENGTH, "man");

            // The construction of the mandate new must be visible
            let mut new_mandate = storage::MandateNew::default();

            let customer_acceptance = data
                .customer_acceptance
                .get_required_value("customer_acceptance")?;
            new_mandate
                .set_mandate_id(mandate_id)
                .set_customer_id(cus_id.clone())
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
                .set_customer_user_agent_extended(customer_acceptance.get_user_agent())
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
                    hyperswitch_domain_models::mandates::MandateDataType::SingleUse(data) => {
                        new_mandate
                            .set_mandate_amount(Some(data.amount.get_amount_as_i64()))
                            .set_mandate_currency(Some(data.currency))
                            .set_mandate_type(storage_enums::MandateType::SingleUse)
                            .to_owned()
                    }

                    hyperswitch_domain_models::mandates::MandateDataType::MultiUse(op_data) => {
                        match op_data {
                            Some(data) => new_mandate
                                .set_mandate_amount(Some(data.amount.get_amount_as_i64()))
                                .set_mandate_currency(Some(data.currency))
                                .set_start_date(data.start_date)
                                .set_end_date(data.end_date),
                            // .set_metadata(data.metadata),
                            // we are storing PaymentMethodData in metadata of mandate
                            None => &mut new_mandate,
                        }
                        .set_mandate_type(storage_enums::MandateType::MultiUse)
                        .to_owned()
                    }
                },
            ))
        }
        (_, _) => Ok(None),
    }
}

#[cfg(feature = "v1")]
// A function to manually authenticate the client secret with intent fulfillment time
pub fn authenticate_client_secret(
    request_client_secret: Option<&String>,
    payment_intent: &PaymentIntent,
) -> Result<(), errors::ApiErrorResponse> {
    match (request_client_secret, &payment_intent.client_secret) {
        (Some(req_cs), Some(pi_cs)) => {
            if req_cs != pi_cs {
                Err(errors::ApiErrorResponse::ClientSecretInvalid)
            } else {
                let current_timestamp = common_utils::date_time::now();

                let session_expiry = payment_intent.session_expiry.unwrap_or(
                    payment_intent
                        .created_at
                        .saturating_add(time::Duration::seconds(consts::DEFAULT_SESSION_EXPIRY)),
                );

                fp_utils::when(current_timestamp > session_expiry, || {
                    Err(errors::ApiErrorResponse::ClientSecretExpired)
                })
            }
        }
        // If there is no client in payment intent, then it has expired
        (Some(_), None) => Err(errors::ApiErrorResponse::ClientSecretExpired),
        _ => Ok(()),
    }
}

pub(crate) fn validate_payment_status_against_allowed_statuses(
    intent_status: storage_enums::IntentStatus,
    allowed_statuses: &[storage_enums::IntentStatus],
    action: &'static str,
) -> Result<(), errors::ApiErrorResponse> {
    fp_utils::when(!allowed_statuses.contains(&intent_status), || {
        Err(errors::ApiErrorResponse::PreconditionFailed {
            message: format!(
                "You cannot {action} this payment because it has status {intent_status}",
            ),
        })
    })
}

pub(crate) fn validate_payment_status_against_not_allowed_statuses(
    intent_status: storage_enums::IntentStatus,
    not_allowed_statuses: &[storage_enums::IntentStatus],
    action: &'static str,
) -> Result<(), errors::ApiErrorResponse> {
    fp_utils::when(not_allowed_statuses.contains(&intent_status), || {
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
    ctp_service_details: &Option<api_models::payments::CtpServiceDetails>,
) -> Result<(), errors::ApiErrorResponse> {
    utils::when(
        !matches!(
            payment_method_type,
            Some(api_enums::PaymentMethodType::Paypal)
        ) && !matches!(
            mandate_type,
            Some(api::MandateTransactionType::RecurringMandateTransaction)
        ) && token.is_none()
            && (payment_method_data.is_none() || payment_method.is_none())
            && ctp_service_details.is_none(),
        || {
            Err(errors::ApiErrorResponse::InvalidRequestData {
                message:
                    "A payment token or payment method data or ctp service details is required"
                        .to_string(),
            })
        },
    )
}

#[cfg(feature = "v2")]
// A function to perform database lookup and then verify the client secret
pub async fn verify_payment_intent_time_and_client_secret(
    state: &SessionState,
    platform: &domain::Platform,
    client_secret: Option<String>,
) -> error_stack::Result<Option<PaymentIntent>, errors::ApiErrorResponse> {
    todo!()
}

#[cfg(feature = "v1")]
// A function to perform database lookup and then verify the client secret
pub async fn verify_payment_intent_time_and_client_secret(
    state: &SessionState,
    platform: &domain::Platform,
    client_secret: Option<String>,
) -> error_stack::Result<Option<PaymentIntent>, errors::ApiErrorResponse> {
    let db = &*state.store;
    client_secret
        .async_map(|cs| async move {
            let payment_id = get_payment_id_from_client_secret(&cs)?;

            let payment_id = id_type::PaymentId::wrap(payment_id).change_context(
                errors::ApiErrorResponse::InvalidDataValue {
                    field_name: "payment_id",
                },
            )?;

            #[cfg(feature = "v1")]
            let payment_intent = db
                .find_payment_intent_by_payment_id_merchant_id(
                    &payment_id,
                    platform.get_processor().get_account().get_id(),
                    platform.get_processor().get_key_store(),
                    platform.get_processor().get_account().storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

            #[cfg(feature = "v2")]
            let payment_intent = db
                .find_payment_intent_by_id(
                    &state.into(),
                    &payment_id,
                    key_store,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

            authenticate_client_secret(Some(&cs), &payment_intent)?;
            Ok(payment_intent)
        })
        .await
        .transpose()
}

#[cfg(feature = "v1")]
/// Check whether the business details are configured in the merchant account
pub fn validate_business_details(
    business_country: Option<api_enums::CountryAlpha2>,
    business_label: Option<&String>,
    platform: &domain::Platform,
) -> RouterResult<()> {
    let primary_business_details = platform
        .get_processor()
        .get_account()
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

#[inline]
pub(crate) fn get_payment_id_from_client_secret(cs: &str) -> RouterResult<String> {
    let (payment_id, _) = cs
        .rsplit_once("_secret_")
        .ok_or(errors::ApiErrorResponse::ClientSecretInvalid)?;
    Ok(payment_id.to_string())
}

#[cfg(feature = "v1")]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authenticate_client_secret_session_not_expired() {
        let payment_intent = PaymentIntent {
            payment_id: id_type::PaymentId::try_from(Cow::Borrowed("23")).unwrap(),
            merchant_id: id_type::MerchantId::default(),
            status: storage_enums::IntentStatus::RequiresCapture,
            amount: MinorUnit::new(200),
            currency: None,
            amount_captured: None,
            customer_id: None,
            description: None,
            return_url: None,
            metadata: None,
            connector_id: None,
            shipping_address_id: None,
            billing_address_id: None,
            mit_category: None,
            tokenization: None,
            statement_descriptor_name: None,
            statement_descriptor_suffix: None,
            created_at: common_utils::date_time::now(),
            modified_at: common_utils::date_time::now(),
            last_synced: None,
            setup_future_usage: None,
            fingerprint_id: None,
            off_session: None,
            client_secret: Some("1".to_string()),
            active_attempt: hyperswitch_domain_models::RemoteStorageObject::ForeignID(
                "nopes".to_string(),
            ),
            business_country: None,
            business_label: None,
            order_details: None,
            allowed_payment_method_types: None,
            connector_metadata: None,
            feature_metadata: None,
            attempt_count: 1,
            payment_link_id: None,
            profile_id: Some(common_utils::generate_profile_id_of_default_length()),
            merchant_decision: None,
            payment_confirm_source: None,
            surcharge_applicable: None,
            updated_by: storage_enums::MerchantStorageScheme::PostgresOnly.to_string(),
            request_incremental_authorization: Some(
                common_enums::RequestIncrementalAuthorization::default(),
            ),
            incremental_authorization_allowed: None,
            authorization_count: None,
            session_expiry: Some(
                common_utils::date_time::now()
                    .saturating_add(time::Duration::seconds(consts::DEFAULT_SESSION_EXPIRY)),
            ),
            request_external_three_ds_authentication: None,
            split_payments: None,
            frm_metadata: None,
            customer_details: None,
            billing_details: None,
            merchant_order_reference_id: None,
            shipping_details: None,
            is_payment_processor_token_flow: None,
            organization_id: id_type::OrganizationId::default(),
            shipping_cost: None,
            tax_details: None,
            skip_external_tax_calculation: None,
            request_extended_authorization: None,
            psd2_sca_exemption_type: None,
            processor_merchant_id: id_type::MerchantId::default(),
            created_by: None,
            force_3ds_challenge: None,
            force_3ds_challenge_trigger: None,
            is_iframe_redirection_enabled: None,
            is_payment_id_from_merchant: None,
            payment_channel: None,
            tax_status: None,
            discount_amount: None,
            order_date: None,
            shipping_amount_tax: None,
            duty_amount: None,
            enable_partial_authorization: None,
            enable_overcapture: None,
            billing_descriptor: None,
            partner_merchant_identifier_details: None,
        };
        let req_cs = Some("1".to_string());
        assert!(authenticate_client_secret(req_cs.as_ref(), &payment_intent).is_ok());
        // Check if the result is an Ok variant
    }

    #[test]
    fn test_authenticate_client_secret_session_expired() {
        let created_at =
            common_utils::date_time::now().saturating_sub(time::Duration::seconds(20 * 60));
        let payment_intent = PaymentIntent {
            payment_id: id_type::PaymentId::try_from(Cow::Borrowed("23")).unwrap(),
            merchant_id: id_type::MerchantId::default(),
            status: storage_enums::IntentStatus::RequiresCapture,
            amount: MinorUnit::new(200),
            currency: None,
            amount_captured: None,
            customer_id: None,
            description: None,
            return_url: None,
            metadata: None,
            connector_id: None,
            shipping_address_id: None,
            mit_category: None,
            billing_address_id: None,
            statement_descriptor_name: None,
            statement_descriptor_suffix: None,
            created_at,
            modified_at: common_utils::date_time::now(),
            fingerprint_id: None,
            last_synced: None,
            setup_future_usage: None,
            off_session: None,
            client_secret: Some("1".to_string()),
            active_attempt: hyperswitch_domain_models::RemoteStorageObject::ForeignID(
                "nopes".to_string(),
            ),
            business_country: None,
            business_label: None,
            order_details: None,
            allowed_payment_method_types: None,
            connector_metadata: None,
            feature_metadata: None,
            attempt_count: 1,
            payment_link_id: None,
            profile_id: Some(common_utils::generate_profile_id_of_default_length()),
            merchant_decision: None,
            payment_confirm_source: None,
            surcharge_applicable: None,
            updated_by: storage_enums::MerchantStorageScheme::PostgresOnly.to_string(),
            request_incremental_authorization: Some(
                common_enums::RequestIncrementalAuthorization::default(),
            ),
            incremental_authorization_allowed: None,
            authorization_count: None,
            session_expiry: Some(
                created_at.saturating_add(time::Duration::seconds(consts::DEFAULT_SESSION_EXPIRY)),
            ),
            request_external_three_ds_authentication: None,
            split_payments: None,
            frm_metadata: None,
            customer_details: None,
            billing_details: None,
            merchant_order_reference_id: None,
            shipping_details: None,
            is_payment_processor_token_flow: None,
            organization_id: id_type::OrganizationId::default(),
            shipping_cost: None,
            tax_details: None,
            skip_external_tax_calculation: None,
            request_extended_authorization: None,
            psd2_sca_exemption_type: None,
            processor_merchant_id: id_type::MerchantId::default(),
            created_by: None,
            force_3ds_challenge: None,
            force_3ds_challenge_trigger: None,
            is_iframe_redirection_enabled: None,
            is_payment_id_from_merchant: None,
            payment_channel: None,
            tax_status: None,
            discount_amount: None,
            order_date: None,
            shipping_amount_tax: None,
            duty_amount: None,
            enable_partial_authorization: None,
            enable_overcapture: None,
            billing_descriptor: None,
            tokenization: None,
            partner_merchant_identifier_details: None,
        };
        let req_cs = Some("1".to_string());
        assert!(authenticate_client_secret(req_cs.as_ref(), &payment_intent,).is_err())
    }

    #[test]
    fn test_authenticate_client_secret_expired() {
        let payment_intent = PaymentIntent {
            payment_id: id_type::PaymentId::try_from(Cow::Borrowed("23")).unwrap(),
            merchant_id: id_type::MerchantId::default(),
            status: storage_enums::IntentStatus::RequiresCapture,
            amount: MinorUnit::new(200),
            currency: None,
            amount_captured: None,
            customer_id: None,
            description: None,
            return_url: None,
            metadata: None,
            connector_id: None,
            mit_category: None,
            tokenization: None,
            shipping_address_id: None,
            billing_address_id: None,
            statement_descriptor_name: None,
            statement_descriptor_suffix: None,
            created_at: common_utils::date_time::now().saturating_sub(time::Duration::seconds(20)),
            modified_at: common_utils::date_time::now(),
            last_synced: None,
            setup_future_usage: None,
            off_session: None,
            client_secret: None,
            fingerprint_id: None,
            active_attempt: hyperswitch_domain_models::RemoteStorageObject::ForeignID(
                "nopes".to_string(),
            ),
            business_country: None,
            business_label: None,
            order_details: None,
            allowed_payment_method_types: None,
            connector_metadata: None,
            feature_metadata: None,
            attempt_count: 1,
            payment_link_id: None,
            profile_id: Some(common_utils::generate_profile_id_of_default_length()),
            merchant_decision: None,
            payment_confirm_source: None,
            surcharge_applicable: None,
            updated_by: storage_enums::MerchantStorageScheme::PostgresOnly.to_string(),
            request_incremental_authorization: Some(
                common_enums::RequestIncrementalAuthorization::default(),
            ),
            incremental_authorization_allowed: None,
            authorization_count: None,
            session_expiry: Some(
                common_utils::date_time::now()
                    .saturating_add(time::Duration::seconds(consts::DEFAULT_SESSION_EXPIRY)),
            ),
            request_external_three_ds_authentication: None,
            split_payments: None,
            frm_metadata: None,
            customer_details: None,
            billing_details: None,
            merchant_order_reference_id: None,
            shipping_details: None,
            is_payment_processor_token_flow: None,
            organization_id: id_type::OrganizationId::default(),
            shipping_cost: None,
            tax_details: None,
            skip_external_tax_calculation: None,
            request_extended_authorization: None,
            psd2_sca_exemption_type: None,
            processor_merchant_id: id_type::MerchantId::default(),
            created_by: None,
            force_3ds_challenge: None,
            force_3ds_challenge_trigger: None,
            is_iframe_redirection_enabled: None,
            is_payment_id_from_merchant: None,
            payment_channel: None,
            tax_status: None,
            discount_amount: None,
            order_date: None,
            shipping_amount_tax: None,
            duty_amount: None,
            enable_partial_authorization: None,
            enable_overcapture: None,
            billing_descriptor: None,
            partner_merchant_identifier_details: None,
        };
        let req_cs = Some("1".to_string());
        assert!(authenticate_client_secret(req_cs.as_ref(), &payment_intent).is_err())
    }
}

// This function will be removed after moving this functionality to server_wrap and using cache instead of config
#[instrument(skip_all)]
pub async fn insert_merchant_connector_creds_to_config(
    db: &dyn StorageInterface,
    merchant_id: &id_type::MerchantId,
    merchant_connector_details: admin::MerchantConnectorDetailsWrap,
) -> RouterResult<()> {
    if let Some(encoded_data) = merchant_connector_details.encoded_data {
        let redis = &db
            .get_redis_conn()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to get redis connection")?;

        let key =
            merchant_id.get_creds_identifier_key(&merchant_connector_details.creds_identifier);

        redis
            .serialize_and_set_key_with_expiry(
                &key.as_str().into(),
                &encoded_data.peek(),
                consts::CONNECTOR_CREDS_TOKEN_TTL,
            )
            .await
            .map_or_else(
                |e| {
                    Err(e
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to insert connector_creds to config"))
                },
                |_| Ok(()),
            )
    } else {
        Ok(())
    }
}

/// Query for merchant connector account either by business label or profile id
/// If profile_id is passed use it, or use connector_label to query merchant connector account
#[instrument(skip_all)]
pub async fn get_merchant_connector_account(
    state: &SessionState,
    merchant_id: &id_type::MerchantId,
    creds_identifier: Option<&str>,
    key_store: &domain::MerchantKeyStore,
    profile_id: &id_type::ProfileId,
    connector_name: &str,
    merchant_connector_id: Option<&id_type::MerchantConnectorAccountId>,
) -> RouterResult<MerchantConnectorAccountType> {
    let db = &*state.store;
    match creds_identifier {
        Some(creds_identifier) => {
            let key = merchant_id.get_creds_identifier_key(creds_identifier);
            let cloned_key = key.clone();
            let redis_fetch = || async {
                db.get_redis_conn()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to get redis connection")
                    .async_and_then(|redis| async move {
                        redis
                            .get_and_deserialize_key(&key.as_str().into(), "String")
                            .await
                            .change_context(
                                errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                                    id: key.clone(),
                                },
                            )
                            .attach_printable(key.clone() + ": Not found in Redis")
                    })
                    .await
            };

            let db_fetch = || async {
                db.find_config_by_key(cloned_key.as_str())
                    .await
                    .to_not_found_response(
                        errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                            id: cloned_key.to_owned(),
                        },
                    )
            };

            let mca_config: String = redis_fetch()
                .await
                .map_or_else(
                    |_| {
                        Either::Left(async {
                            match db_fetch().await {
                                Ok(config_entry) => Ok(config_entry.config),
                                Err(e) => Err(e),
                            }
                        })
                    },
                    |result| Either::Right(async { Ok(result) }),
                )
                .await?;

            let private_key = state
                .conf
                .jwekey
                .get_inner()
                .tunnel_private_key
                .peek()
                .as_bytes();

            let decrypted_mca = services::decrypt_jwe(mca_config.as_str(), services::KeyIdCheck::SkipKeyIdCheck, private_key, jwe::RSA_OAEP_256)
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
            let mca: RouterResult<domain::MerchantConnectorAccount> =
                if let Some(merchant_connector_id) = merchant_connector_id {
                    #[cfg(feature = "v1")]
                    {
                        db.find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                            merchant_id,
                            merchant_connector_id,
                            key_store,
                        )
                        .await
                        .to_not_found_response(
                            errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                                id: merchant_connector_id.get_string_repr().to_string(),
                            },
                        )
                    }
                    #[cfg(feature = "v2")]
                    {
                        db.find_merchant_connector_account_by_id(merchant_connector_id, key_store)
                            .await
                            .to_not_found_response(
                                errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                                    id: merchant_connector_id.get_string_repr().to_string(),
                                },
                            )
                    }
                } else {
                    #[cfg(feature = "v1")]
                    {
                        db.find_merchant_connector_account_by_profile_id_connector_name(
                            profile_id,
                            connector_name,
                            key_store,
                        )
                        .await
                        .to_not_found_response(
                            errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                                id: format!(
                                    "profile id {} and connector name {connector_name}",
                                    profile_id.get_string_repr()
                                ),
                            },
                        )
                    }
                    #[cfg(feature = "v2")]
                    {
                        todo!()
                    }
                };
            mca.map(Box::new).map(MerchantConnectorAccountType::DbVal)
        }
    }
}

/// This function replaces the request and response type of routerdata with the
/// request and response type passed
/// # Arguments
///
/// * `router_data` - original router data
/// * `request` - new request core/helper
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
        tenant_id: router_data.tenant_id,
        address: router_data.address,
        amount_captured: router_data.amount_captured,
        minor_amount_captured: router_data.minor_amount_captured,
        auth_type: router_data.auth_type,
        connector: router_data.connector,
        connector_auth_type: router_data.connector_auth_type,
        connector_meta_data: router_data.connector_meta_data,
        description: router_data.description,
        payment_id: router_data.payment_id,
        payment_method: router_data.payment_method,
        payment_method_type: router_data.payment_method_type,
        status: router_data.status,
        attempt_id: router_data.attempt_id,
        access_token: router_data.access_token,
        session_token: router_data.session_token,
        payment_method_status: router_data.payment_method_status,
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
        frm_metadata: router_data.frm_metadata,
        refund_id: router_data.refund_id,
        dispute_id: router_data.dispute_id,
        connector_response: router_data.connector_response,
        integrity_check: Ok(()),
        connector_wallets_details: router_data.connector_wallets_details,
        additional_merchant_data: router_data.additional_merchant_data,
        header_payload: router_data.header_payload,
        connector_mandate_request_reference_id: router_data.connector_mandate_request_reference_id,
        authentication_id: router_data.authentication_id,
        psd2_sca_exemption_type: router_data.psd2_sca_exemption_type,
        raw_connector_response: router_data.raw_connector_response,
        is_payment_id_from_merchant: router_data.is_payment_id_from_merchant,
        l2_l3_data: router_data.l2_l3_data,
        minor_amount_capturable: router_data.minor_amount_capturable,
        authorized_amount: router_data.authorized_amount,
    }
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub fn get_attempt_type(
    payment_intent: &PaymentIntent,
    payment_attempt: &PaymentAttempt,
    is_manual_retry_enabled: Option<bool>,
    action: &str,
) -> RouterResult<AttemptType> {
    match payment_intent.status {
        enums::IntentStatus::Failed => {
            if matches!(is_manual_retry_enabled, Some(true)) {
                // if it is false, don't go ahead with manual retry
                fp_utils::when(
                    !validate_manual_retry_cutoff(
                        payment_intent.created_at,
                        payment_intent.session_expiry,
                    ),
                    || {
                        Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                            message:
                                format!("You cannot {action} this payment using `manual_retry` because the allowed duration has expired")
                            }
                        ))
                    },
                )?;

                metrics::MANUAL_RETRY_REQUEST_COUNT.add(
                    1,
                    router_env::metric_attributes!((
                        "merchant_id",
                        payment_attempt.merchant_id.clone(),
                    )),
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
                    | enums::AttemptStatus::PartialChargedAndChargeable
                    | enums::AttemptStatus::Voided
                    | enums::AttemptStatus::VoidedPostCharge
                    | enums::AttemptStatus::AutoRefunded
                    | enums::AttemptStatus::PaymentMethodAwaited
                    | enums::AttemptStatus::DeviceDataCollectionPending
                    | enums::AttemptStatus::IntegrityFailure
                    | enums::AttemptStatus::Expired
                    | enums::AttemptStatus::PartiallyAuthorized => {
                        metrics::MANUAL_RETRY_VALIDATION_FAILED.add(
                            1,
                            router_env::metric_attributes!((
                                "merchant_id",
                                payment_attempt.merchant_id.clone(),
                            )),
                        );
                        Err(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Payment Attempt unexpected state")
                    }

                    storage_enums::AttemptStatus::VoidFailed
                    | storage_enums::AttemptStatus::RouterDeclined
                    | storage_enums::AttemptStatus::CaptureFailed => {
                        metrics::MANUAL_RETRY_VALIDATION_FAILED.add(
                            1,
                            router_env::metric_attributes!((
                                "merchant_id",
                                payment_attempt.merchant_id.clone(),
                            )),
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
                            1,
                            router_env::metric_attributes!((
                                "merchant_id",
                                payment_attempt.merchant_id.clone(),
                            )),
                        );
                        Ok(AttemptType::New)
                    }
                }
            } else {
                Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                        message:
                            format!("You cannot {action} this payment because it has status {}, you can enable `manual_retry` in profile to try this payment again", payment_intent.status)
                        }
                    ))
            }
        }
        enums::IntentStatus::Cancelled
        | enums::IntentStatus::CancelledPostCapture
        | enums::IntentStatus::RequiresCapture
        | enums::IntentStatus::PartiallyCaptured
        | enums::IntentStatus::PartiallyCapturedAndCapturable
        | enums::IntentStatus::Processing
        | enums::IntentStatus::Succeeded
        | enums::IntentStatus::Conflicted
        | enums::IntentStatus::Expired
        | enums::IntentStatus::PartiallyAuthorizedAndRequiresCapture => {
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

fn validate_manual_retry_cutoff(
    created_at: time::PrimitiveDateTime,
    session_expiry: Option<time::PrimitiveDateTime>,
) -> bool {
    let utc_current_time = time::OffsetDateTime::now_utc();
    let primitive_utc_current_time =
        time::PrimitiveDateTime::new(utc_current_time.date(), utc_current_time.time());
    let time_difference_from_creation = primitive_utc_current_time - created_at;

    // cutoff time is 50% of session duration
    let cutoff_limit = match session_expiry {
        Some(session_expiry) => {
            let duration = session_expiry - created_at;
            duration.whole_seconds() / 2
        }
        None => consts::DEFAULT_SESSION_EXPIRY / 2,
    };

    time_difference_from_creation.whole_seconds() <= cutoff_limit
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum AttemptType {
    New,
    SameOld,
}

impl AttemptType {
    #[cfg(feature = "v1")]
    // The function creates a new payment_attempt from the previous payment attempt but doesn't populate fields like payment_method, error_code etc.
    // Logic to override the fields with data provided in the request should be done after this if required.
    // In case if fields are not overridden by the request then they contain the same data that was in the previous attempt provided it is populated in this function.
    #[inline(always)]
    fn make_new_payment_attempt(
        payment_method_data: Option<&api_models::payments::PaymentMethodData>,
        old_payment_attempt: PaymentAttempt,
        new_attempt_count: i16,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> storage::PaymentAttemptNew {
        let created_at @ modified_at @ last_synced = Some(common_utils::date_time::now());

        storage::PaymentAttemptNew {
            attempt_id: old_payment_attempt
                .payment_id
                .get_attempt_id(new_attempt_count),
            payment_id: old_payment_attempt.payment_id,
            merchant_id: old_payment_attempt.merchant_id,

            // A new payment attempt is getting created so, used the same function which is used to populate status in PaymentCreate Flow.
            status: payment_attempt_status_fsm(payment_method_data, Some(true)),

            currency: old_payment_attempt.currency,
            save_to_locker: old_payment_attempt.save_to_locker,

            connector: None,

            error_message: None,
            offer_amount: old_payment_attempt.offer_amount,
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
            amount_capturable: old_payment_attempt.net_amount.get_total_amount(),
            updated_by: storage_scheme.to_string(),
            authentication_data: None,
            encoded_data: None,
            merchant_connector_id: None,
            unified_code: None,
            unified_message: None,
            net_amount: old_payment_attempt.net_amount,
            external_three_ds_authentication_attempted: old_payment_attempt
                .external_three_ds_authentication_attempted,
            authentication_connector: None,
            authentication_id: None,
            mandate_data: old_payment_attempt.mandate_data,
            // New payment method billing address can be passed for a retry
            payment_method_billing_address_id: None,
            fingerprint_id: None,
            client_source: old_payment_attempt.client_source,
            client_version: old_payment_attempt.client_version,
            customer_acceptance: old_payment_attempt.customer_acceptance,
            organization_id: old_payment_attempt.organization_id,
            profile_id: old_payment_attempt.profile_id,
            connector_mandate_detail: None,
            request_extended_authorization: None,
            extended_authorization_applied: None,
            extended_authorization_last_applied_at: None,
            capture_before: None,
            card_discovery: None,
            processor_merchant_id: old_payment_attempt.processor_merchant_id,
            created_by: old_payment_attempt.created_by,
            setup_future_usage_applied: None,
            routing_approach: old_payment_attempt.routing_approach,
            connector_request_reference_id: None,
            network_transaction_id: None,
            network_details: None,
            is_stored_credential: old_payment_attempt.is_stored_credential,
            authorized_amount: old_payment_attempt.authorized_amount,
            tokenization: None,
        }
    }

    // #[cfg(feature = "v2")]
    // // The function creates a new payment_attempt from the previous payment attempt but doesn't populate fields like payment_method, error_code etc.
    // // Logic to override the fields with data provided in the request should be done after this if required.
    // // In case if fields are not overridden by the request then they contain the same data that was in the previous attempt provided it is populated in this function.
    // #[inline(always)]
    // fn make_new_payment_attempt(
    //     _payment_method_data: Option<&api_models::payments::PaymentMethodData>,
    //     _old_payment_attempt: PaymentAttempt,
    //     _new_attempt_count: i16,
    //     _storage_scheme: enums::MerchantStorageScheme,
    // ) -> PaymentAttempt {
    //     todo!()
    // }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    pub async fn modify_payment_intent_and_payment_attempt(
        &self,
        request: &api_models::payments::PaymentsRequest,
        fetched_payment_intent: PaymentIntent,
        fetched_payment_attempt: PaymentAttempt,
        state: &SessionState,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: storage::enums::MerchantStorageScheme,
    ) -> RouterResult<(PaymentIntent, PaymentAttempt)> {
        match self {
            Self::SameOld => Ok((fetched_payment_intent, fetched_payment_attempt)),
            Self::New => {
                let db = &*state.store;
                let new_attempt_count = fetched_payment_intent.attempt_count + 1;
                let new_payment_attempt_to_insert = Self::make_new_payment_attempt(
                    request
                        .payment_method_data
                        .as_ref()
                        .and_then(|request_payment_method_data| {
                            request_payment_method_data.payment_method_data.as_ref()
                        }),
                    fetched_payment_attempt,
                    new_attempt_count,
                    storage_scheme,
                );

                #[cfg(feature = "v1")]
                let new_payment_attempt = db
                    .insert_payment_attempt(new_payment_attempt_to_insert, storage_scheme)
                    .await
                    .to_duplicate_response(errors::ApiErrorResponse::DuplicatePayment {
                        payment_id: fetched_payment_intent.get_id().to_owned(),
                    })?;

                #[cfg(feature = "v2")]
                let new_payment_attempt = db
                    .insert_payment_attempt(
                        key_store,
                        new_payment_attempt_to_insert,
                        storage_scheme,
                    )
                    .await
                    .to_duplicate_response(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to insert payment attempt")?;

                let updated_payment_intent = db
                    .update_payment_intent(
                        fetched_payment_intent,
                        storage::PaymentIntentUpdate::StatusAndAttemptUpdate {
                            status: payment_intent_status_fsm(
                                request.payment_method_data.as_ref().and_then(
                                    |request_payment_method_data| {
                                        request_payment_method_data.payment_method_data.as_ref()
                                    },
                                ),
                                Some(true),
                            ),
                            active_attempt_id: new_payment_attempt.get_id().to_owned(),
                            attempt_count: new_attempt_count,
                            updated_by: storage_scheme.to_string(),
                        },
                        key_store,
                        storage_scheme,
                    )
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

                logger::info!(
                    "manual_retry payment for {:?} with attempt_id {:?}",
                    updated_payment_intent.get_id(),
                    new_payment_attempt.get_id()
                );

                Ok((updated_payment_intent, new_payment_attempt))
            }
        }
    }
}

#[inline(always)]
pub fn is_manual_retry_allowed(
    intent_status: &storage_enums::IntentStatus,
    attempt_status: &storage_enums::AttemptStatus,
    connector_request_reference_id_config: &ConnectorRequestReferenceIdConfig,
    merchant_id: &id_type::MerchantId,
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
            | enums::AttemptStatus::PartialChargedAndChargeable
            | enums::AttemptStatus::Voided
            | enums::AttemptStatus::VoidedPostCharge
            | enums::AttemptStatus::AutoRefunded
            | enums::AttemptStatus::PaymentMethodAwaited
            | enums::AttemptStatus::DeviceDataCollectionPending
            | enums::AttemptStatus::IntegrityFailure
            | enums::AttemptStatus::Expired
            | enums::AttemptStatus::PartiallyAuthorized => {
                logger::error!("Payment Attempt should not be in this state because Attempt to Intent status mapping doesn't allow it");
                None
            }

            enums::AttemptStatus::VoidFailed
            | enums::AttemptStatus::RouterDeclined
            | enums::AttemptStatus::CaptureFailed => Some(false),

            enums::AttemptStatus::AuthenticationFailed
            | enums::AttemptStatus::AuthorizationFailed
            | enums::AttemptStatus::Failure => Some(true),
        },
        enums::IntentStatus::Cancelled
        | enums::IntentStatus::CancelledPostCapture
        | enums::IntentStatus::RequiresCapture
        | enums::IntentStatus::PartiallyCaptured
        | enums::IntentStatus::PartiallyCapturedAndCapturable
        | enums::IntentStatus::Processing
        | enums::IntentStatus::Succeeded
        | enums::IntentStatus::Conflicted
        | enums::IntentStatus::Expired
        | enums::IntentStatus::PartiallyAuthorizedAndRequiresCapture => Some(false),

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
    pm_data: &domain::PaymentMethodData,
    db: &dyn StorageInterface,
    profile_id: &id_type::ProfileId,
) -> Result<
    Option<api_models::payments::AdditionalPaymentData>,
    error_stack::Report<errors::ApiErrorResponse>,
> {
    match pm_data {
        domain::PaymentMethodData::Card(card_data) => {
            //todo!
            let card_isin = Some(card_data.card_number.get_card_isin());
            let enable_extended_bin =db
            .find_config_by_key_unwrap_or(
                format!("{}_enable_extended_card_bin", profile_id.get_string_repr()).as_str(),
             Some("false".to_string()))
            .await.map_err(|err| services::logger::error!(message="Failed to fetch the config", extended_card_bin_error=?err)).ok();

            let card_extended_bin = match enable_extended_bin {
                Some(config) if config.config == "true" => {
                    Some(card_data.card_number.get_extended_card_bin())
                }
                _ => None,
            };

            // Added an additional check for card_data.co_badged_card_data.is_some()
            // because is_cobadged_card() only returns true if the card number matches a specific regex.
            // However, this regex does not cover all possible co-badged networks.
            // The co_badged_card_data field is populated based on a co-badged BIN lookup
            // and helps identify co-badged cards that may not match the regex alone.
            // Determine the card network based on cobadge detection and co-badged BIN data
            let is_cobadged_based_on_regex = card_data
                .card_number
                .is_cobadged_card()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "Card cobadge check failed due to an invalid card network regex",
                )?;

            let (card_network, signature_network, is_regulated) = card_data
                .co_badged_card_data
                .as_ref()
                .map(|co_badged_data| {
                    logger::debug!("Co-badged card data found");

                    (
                        card_data.card_network.clone(),
                        co_badged_data
                            .co_badged_card_networks_info
                            .get_signature_network(),
                        Some(co_badged_data.is_regulated),
                    )
                })
                .or_else(|| {
                    is_cobadged_based_on_regex.then(|| {
                        logger::debug!("Card network is cobadged (regex-based detection)");
                        (card_data.card_network.clone(), None, None)
                    })
                })
                .unwrap_or_else(|| {
                    logger::debug!("Card network is not cobadged");
                    (None, None, None)
                });

            let last4 = Some(card_data.card_number.get_last4());
            if card_data.card_issuer.is_some()
                && card_network.is_some()
                && card_data.card_type.is_some()
                && card_data.card_issuing_country.is_some()
                && card_data.bank_code.is_some()
            {
                Ok(Some(api_models::payments::AdditionalPaymentData::Card(
                    Box::new(api_models::payments::AdditionalCardInfo {
                        card_issuer: card_data.card_issuer.to_owned(),
                        card_network,
                        card_type: card_data.card_type.to_owned(),
                        card_issuing_country: card_data.card_issuing_country.to_owned(),
                        bank_code: card_data.bank_code.to_owned(),
                        card_exp_month: Some(card_data.card_exp_month.clone()),
                        card_exp_year: Some(card_data.card_exp_year.clone()),
                        card_holder_name: card_data.card_holder_name.clone(),
                        last4: last4.clone(),
                        card_isin: card_isin.clone(),
                        card_extended_bin: card_extended_bin.clone(),
                        // These are filled after calling the processor / connector
                        payment_checks: None,
                        authentication_data: None,
                        is_regulated,
                        signature_network: signature_network.clone(),
                    }),
                )))
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
                                card_network: card_network.clone().or(card_info.card_network),
                                bank_code: card_info.bank_code,
                                card_type: card_info.card_type,
                                card_issuing_country: card_info.card_issuing_country,
                                last4: last4.clone(),
                                card_isin: card_isin.clone(),
                                card_extended_bin: card_extended_bin.clone(),
                                card_exp_month: Some(card_data.card_exp_month.clone()),
                                card_exp_year: Some(card_data.card_exp_year.clone()),
                                card_holder_name: card_data.card_holder_name.clone(),
                                // These are filled after calling the processor / connector
                                payment_checks: None,
                                authentication_data: None,
                                is_regulated,
                                signature_network: signature_network.clone(),
                            },
                        ))
                    });
                Ok(Some(card_info.unwrap_or_else(|| {
                    api_models::payments::AdditionalPaymentData::Card(Box::new(
                        api_models::payments::AdditionalCardInfo {
                            card_issuer: None,
                            card_network,
                            bank_code: None,
                            card_type: None,
                            card_issuing_country: None,
                            last4,
                            card_isin,
                            card_extended_bin,
                            card_exp_month: Some(card_data.card_exp_month.clone()),
                            card_exp_year: Some(card_data.card_exp_year.clone()),
                            card_holder_name: card_data.card_holder_name.clone(),
                            // These are filled after calling the processor / connector
                            payment_checks: None,
                            authentication_data: None,
                            is_regulated,
                            signature_network: signature_network.clone(),
                        },
                    ))
                })))
            }
        }
        domain::PaymentMethodData::BankRedirect(bank_redirect_data) => match bank_redirect_data {
            domain::BankRedirectData::Eps { bank_name, .. } => Ok(Some(
                api_models::payments::AdditionalPaymentData::BankRedirect {
                    bank_name: bank_name.to_owned(),
                    details: None,
                    interac: None,
                },
            )),
            domain::BankRedirectData::Eft { .. } => Ok(Some(
                api_models::payments::AdditionalPaymentData::BankRedirect {
                    bank_name: None,
                    details: None,
                    interac: None,
                },
            )),
            domain::BankRedirectData::OnlineBankingFpx { issuer } => Ok(Some(
                api_models::payments::AdditionalPaymentData::BankRedirect {
                    bank_name: Some(issuer.to_owned()),
                    details: None,
                    interac: None,
                },
            )),
            domain::BankRedirectData::Ideal { bank_name, .. } => Ok(Some(
                api_models::payments::AdditionalPaymentData::BankRedirect {
                    bank_name: bank_name.to_owned(),
                    details: None,
                    interac: None,
                },
            )),
            domain::BankRedirectData::BancontactCard {
                card_number,
                card_exp_month,
                card_exp_year,
                card_holder_name,
            } => Ok(Some(
                api_models::payments::AdditionalPaymentData::BankRedirect {
                    bank_name: None,
                    details: Some(
                        payment_additional_types::BankRedirectDetails::BancontactCard(Box::new(
                            payment_additional_types::BancontactBankRedirectAdditionalData {
                                last4: card_number.as_ref().map(|c| c.get_last4()),
                                card_exp_month: card_exp_month.clone(),
                                card_exp_year: card_exp_year.clone(),
                                card_holder_name: card_holder_name.clone(),
                            },
                        )),
                    ),
                    interac: None,
                },
            )),
            domain::BankRedirectData::Blik { blik_code } => Ok(Some(
                api_models::payments::AdditionalPaymentData::BankRedirect {
                    bank_name: None,
                    details: blik_code.as_ref().map(|blik_code| {
                        payment_additional_types::BankRedirectDetails::Blik(Box::new(
                            payment_additional_types::BlikBankRedirectAdditionalData {
                                blik_code: Some(blik_code.to_owned()),
                            },
                        ))
                    }),
                    interac: None,
                },
            )),
            domain::BankRedirectData::Giropay {
                bank_account_bic,
                bank_account_iban,
                country,
            } => Ok(Some(
                api_models::payments::AdditionalPaymentData::BankRedirect {
                    bank_name: None,
                    details: Some(payment_additional_types::BankRedirectDetails::Giropay(
                        Box::new(
                            payment_additional_types::GiropayBankRedirectAdditionalData {
                                bic: bank_account_bic
                                    .as_ref()
                                    .map(|bic| MaskedSortCode::from(bic.to_owned())),
                                iban: bank_account_iban
                                    .as_ref()
                                    .map(|iban| MaskedIban::from(iban.to_owned())),
                                country: *country,
                            },
                        ),
                    )),
                    interac: None,
                },
            )),
            _ => Ok(Some(
                api_models::payments::AdditionalPaymentData::BankRedirect {
                    bank_name: None,
                    details: None,
                    interac: None,
                },
            )),
        },
        domain::PaymentMethodData::Wallet(wallet) => match wallet {
            domain::WalletData::ApplePay(apple_pay_wallet_data) => {
                let (card_exp_month, card_exp_year) = match apple_pay_wallet_data
                    .payment_data
                    .get_decrypted_apple_pay_payment_data_optional()
                {
                    Some(token) => (
                        Some(token.application_expiration_month.clone()),
                        Some(token.application_expiration_year.clone()),
                    ),
                    None => (None, None),
                };
                Ok(Some(api_models::payments::AdditionalPaymentData::Wallet {
                    apple_pay: Some(Box::new(api_models::payments::ApplepayPaymentMethod {
                        display_name: apple_pay_wallet_data.payment_method.display_name.clone(),
                        network: apple_pay_wallet_data.payment_method.network.clone(),
                        pm_type: apple_pay_wallet_data.payment_method.pm_type.clone(),
                        card_exp_month,
                        card_exp_year,
                    })),
                    google_pay: None,
                    samsung_pay: None,
                }))
            }
            domain::WalletData::GooglePay(google_pay_pm_data) => {
                let (card_exp_month, card_exp_year) = match google_pay_pm_data
                    .tokenization_data
                    .get_decrypted_google_pay_payment_data_optional()
                {
                    Some(token) => (
                        Some(token.card_exp_month.clone()),
                        Some(token.card_exp_year.clone()),
                    ),
                    None => (None, None),
                };
                Ok(Some(api_models::payments::AdditionalPaymentData::Wallet {
                    apple_pay: None,
                    google_pay: Some(Box::new(
                        payment_additional_types::WalletAdditionalDataForCard {
                            last4: google_pay_pm_data.info.card_details.clone(),
                            card_network: google_pay_pm_data.info.card_network.clone(),
                            card_type: Some(google_pay_pm_data.pm_type.clone()),
                            card_exp_month,
                            card_exp_year,
                        },
                    )),
                    samsung_pay: None,
                }))
            }
            domain::WalletData::SamsungPay(samsung_pay_pm_data) => {
                Ok(Some(api_models::payments::AdditionalPaymentData::Wallet {
                    apple_pay: None,
                    google_pay: None,
                    samsung_pay: Some(Box::new(
                        payment_additional_types::WalletAdditionalDataForCard {
                            last4: samsung_pay_pm_data
                                .payment_credential
                                .card_last_four_digits
                                .clone(),
                            card_network: samsung_pay_pm_data
                                .payment_credential
                                .card_brand
                                .to_string(),
                            card_type: None,
                            card_exp_month: None,
                            card_exp_year: None,
                        },
                    )),
                }))
            }
            _ => Ok(Some(api_models::payments::AdditionalPaymentData::Wallet {
                apple_pay: None,
                google_pay: None,
                samsung_pay: None,
            })),
        },
        domain::PaymentMethodData::PayLater(_) => Ok(Some(
            api_models::payments::AdditionalPaymentData::PayLater { klarna_sdk: None },
        )),
        domain::PaymentMethodData::BankTransfer(bank_transfer) => Ok(Some(
            api_models::payments::AdditionalPaymentData::BankTransfer {
                details: Some((*(bank_transfer.to_owned())).into()),
            },
        )),
        domain::PaymentMethodData::Crypto(crypto) => {
            Ok(Some(api_models::payments::AdditionalPaymentData::Crypto {
                details: Some(crypto.to_owned().into()),
            }))
        }
        domain::PaymentMethodData::BankDebit(bank_debit) => Ok(Some(
            api_models::payments::AdditionalPaymentData::BankDebit {
                details: Some(bank_debit.to_owned().into()),
            },
        )),
        domain::PaymentMethodData::MandatePayment => Ok(Some(
            api_models::payments::AdditionalPaymentData::MandatePayment {},
        )),
        domain::PaymentMethodData::Reward => {
            Ok(Some(api_models::payments::AdditionalPaymentData::Reward {}))
        }
        domain::PaymentMethodData::RealTimePayment(realtime_payment) => Ok(Some(
            api_models::payments::AdditionalPaymentData::RealTimePayment {
                details: Some((*(realtime_payment.to_owned())).into()),
            },
        )),
        domain::PaymentMethodData::Upi(upi) => {
            Ok(Some(api_models::payments::AdditionalPaymentData::Upi {
                details: Some(upi.to_owned().into()),
            }))
        }
        domain::PaymentMethodData::CardRedirect(card_redirect) => Ok(Some(
            api_models::payments::AdditionalPaymentData::CardRedirect {
                details: Some(card_redirect.to_owned().into()),
            },
        )),
        domain::PaymentMethodData::Voucher(voucher) => {
            Ok(Some(api_models::payments::AdditionalPaymentData::Voucher {
                details: Some(voucher.to_owned().into()),
            }))
        }
        domain::PaymentMethodData::GiftCard(gift_card) => Ok(Some(
            api_models::payments::AdditionalPaymentData::GiftCard {
                details: Some((*(gift_card.to_owned())).into()),
            },
        )),
        domain::PaymentMethodData::CardToken(card_token) => Ok(Some(
            api_models::payments::AdditionalPaymentData::CardToken {
                details: Some(card_token.to_owned().into()),
            },
        )),
        domain::PaymentMethodData::OpenBanking(open_banking) => Ok(Some(
            api_models::payments::AdditionalPaymentData::OpenBanking {
                details: Some(open_banking.to_owned().into()),
            },
        )),
        domain::PaymentMethodData::CardDetailsForNetworkTransactionId(card_data) => {
            let card_isin = Some(card_data.card_number.get_card_isin());
            let enable_extended_bin =db
            .find_config_by_key_unwrap_or(
                format!("{}_enable_extended_card_bin", profile_id.get_string_repr()).as_str(),
             Some("false".to_string()))
            .await.map_err(|err| services::logger::error!(message="Failed to fetch the config", extended_card_bin_error=?err)).ok();

            let card_extended_bin = match enable_extended_bin {
                Some(config) if config.config == "true" => {
                    Some(card_data.card_number.get_extended_card_bin())
                }
                _ => None,
            };

            let card_network = match card_data
                .card_number
                .is_cobadged_card()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "Card cobadge check failed due to an invalid card network regex",
                )? {
                true => card_data.card_network.clone(),
                false => None,
            };

            let last4 = Some(card_data.card_number.get_last4());
            if card_data.card_issuer.is_some()
                && card_network.is_some()
                && card_data.card_type.is_some()
                && card_data.card_issuing_country.is_some()
                && card_data.bank_code.is_some()
            {
                Ok(Some(api_models::payments::AdditionalPaymentData::Card(
                    Box::new(api_models::payments::AdditionalCardInfo {
                        card_issuer: card_data.card_issuer.to_owned(),
                        card_network,
                        card_type: card_data.card_type.to_owned(),
                        card_issuing_country: card_data.card_issuing_country.to_owned(),
                        bank_code: card_data.bank_code.to_owned(),
                        card_exp_month: Some(card_data.card_exp_month.clone()),
                        card_exp_year: Some(card_data.card_exp_year.clone()),
                        card_holder_name: card_data.card_holder_name.clone(),
                        last4: last4.clone(),
                        card_isin: card_isin.clone(),
                        card_extended_bin: card_extended_bin.clone(),
                        // These are filled after calling the processor / connector
                        payment_checks: None,
                        authentication_data: None,
                        is_regulated: None,
                        signature_network: None,
                    }),
                )))
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
                                card_network: card_network.clone().or(card_info.card_network),
                                bank_code: card_info.bank_code,
                                card_type: card_info.card_type,
                                card_issuing_country: card_info.card_issuing_country,
                                last4: last4.clone(),
                                card_isin: card_isin.clone(),
                                card_extended_bin: card_extended_bin.clone(),
                                card_exp_month: Some(card_data.card_exp_month.clone()),
                                card_exp_year: Some(card_data.card_exp_year.clone()),
                                card_holder_name: card_data.card_holder_name.clone(),
                                // These are filled after calling the processor / connector
                                payment_checks: None,
                                authentication_data: None,
                                is_regulated: None,
                                signature_network: None,
                            },
                        ))
                    });
                Ok(Some(card_info.unwrap_or_else(|| {
                    api_models::payments::AdditionalPaymentData::Card(Box::new(
                        api_models::payments::AdditionalCardInfo {
                            card_issuer: None,
                            card_network,
                            bank_code: None,
                            card_type: None,
                            card_issuing_country: None,
                            last4,
                            card_isin,
                            card_extended_bin,
                            card_exp_month: Some(card_data.card_exp_month.clone()),
                            card_exp_year: Some(card_data.card_exp_year.clone()),
                            card_holder_name: card_data.card_holder_name.clone(),
                            // These are filled after calling the processor / connector
                            payment_checks: None,
                            authentication_data: None,
                            is_regulated: None,
                            signature_network: None,
                        },
                    ))
                })))
            }
        }
        domain::PaymentMethodData::MobilePayment(mobile_payment) => Ok(Some(
            api_models::payments::AdditionalPaymentData::MobilePayment {
                details: Some(mobile_payment.to_owned().into()),
            },
        )),
        domain::PaymentMethodData::NetworkToken(_) => Ok(None),
    }
}

#[cfg(feature = "v1")]
pub fn validate_customer_access(
    payment_intent: &PaymentIntent,
    auth_flow: services::AuthFlow,
    request: &api::PaymentsRequest,
) -> Result<(), errors::ApiErrorResponse> {
    if auth_flow == services::AuthFlow::Client && request.get_customer_id().is_some() {
        let is_same_customer = request.get_customer_id() == payment_intent.customer_id.as_ref();
        if !is_same_customer {
            Err(errors::ApiErrorResponse::GenericUnauthorized {
                message: "Unauthorised access to update customer".to_string(),
            })?;
        }
    }
    Ok(())
}

pub fn is_apple_pay_simplified_flow(
    connector_metadata: Option<pii::SecretSerdeValue>,
    connector_name: Option<&String>,
) -> CustomResult<bool, errors::ApiErrorResponse> {
    let option_apple_pay_metadata = get_applepay_metadata(connector_metadata)
        .map_err(|error| {
            logger::info!(
                "Apple pay metadata parsing for {:?} in is_apple_pay_simplified_flow {:?}",
                connector_name,
                error
            )
        })
        .ok();

    // return true only if the apple flow type is simplified
    Ok(matches!(
        option_apple_pay_metadata,
        Some(
            api_models::payments::ApplepaySessionTokenMetadata::ApplePayCombined(
                api_models::payments::ApplePayCombinedMetadata::Simplified { .. }
            )
        )
    ))
}

// This function will return the encrypted connector wallets details with Apple Pay certificates
// Currently apple pay certifiactes are stored in the metadata which is not encrypted.
// In future we want those certificates to be encrypted and stored in the connector_wallets_details.
// As part of migration fallback this function checks apple pay details are present in connector_wallets_details
// If yes, it will encrypt connector_wallets_details and store it in the database.
// If no, it will check if apple pay details are present in metadata and merge it with connector_wallets_details, encrypt and store it.
pub async fn get_connector_wallets_details_with_apple_pay_certificates(
    connector_metadata: &Option<masking::Secret<tera::Value>>,
    connector_wallets_details_optional: &Option<api_models::admin::ConnectorWalletDetails>,
) -> RouterResult<Option<masking::Secret<serde_json::Value>>> {
    let connector_wallet_details_with_apple_pay_metadata_optional =
        get_apple_pay_metadata_if_needed(connector_metadata, connector_wallets_details_optional)
            .await?;

    let connector_wallets_details = connector_wallet_details_with_apple_pay_metadata_optional
        .map(|details| {
            serde_json::to_value(details)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to serialize Apple Pay metadata as JSON")
        })
        .transpose()?
        .map(masking::Secret::new);

    Ok(connector_wallets_details)
}

async fn get_apple_pay_metadata_if_needed(
    connector_metadata: &Option<masking::Secret<tera::Value>>,
    connector_wallets_details_optional: &Option<api_models::admin::ConnectorWalletDetails>,
) -> RouterResult<Option<api_models::admin::ConnectorWalletDetails>> {
    if let Some(connector_wallets_details) = connector_wallets_details_optional {
        if connector_wallets_details.apple_pay_combined.is_some()
            || connector_wallets_details.apple_pay.is_some()
        {
            return Ok(Some(connector_wallets_details.clone()));
        }
        // Otherwise, merge Apple Pay metadata
        return get_and_merge_apple_pay_metadata(
            connector_metadata.clone(),
            Some(connector_wallets_details.clone()),
        )
        .await;
    }

    // If connector_wallets_details_optional is None, attempt to get Apple Pay metadata
    get_and_merge_apple_pay_metadata(connector_metadata.clone(), None).await
}

async fn get_and_merge_apple_pay_metadata(
    connector_metadata: Option<masking::Secret<tera::Value>>,
    connector_wallets_details_optional: Option<api_models::admin::ConnectorWalletDetails>,
) -> RouterResult<Option<api_models::admin::ConnectorWalletDetails>> {
    let apple_pay_metadata_optional = get_applepay_metadata(connector_metadata)
        .map_err(|error| {
            logger::error!(
                "Apple Pay metadata parsing failed in get_encrypted_connector_wallets_details_with_apple_pay_certificates {:?}",
                error
            );
        })
        .ok();

    if let Some(apple_pay_metadata) = apple_pay_metadata_optional {
        let updated_wallet_details = match apple_pay_metadata {
            api_models::payments::ApplepaySessionTokenMetadata::ApplePayCombined(
                apple_pay_combined_metadata,
            ) => {
                let combined_metadata_json = serde_json::to_value(apple_pay_combined_metadata)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to serialize Apple Pay combined metadata as JSON")?;

                api_models::admin::ConnectorWalletDetails {
                    apple_pay_combined: Some(masking::Secret::new(combined_metadata_json)),
                    apple_pay: connector_wallets_details_optional
                        .as_ref()
                        .and_then(|d| d.apple_pay.clone()),
                    amazon_pay: connector_wallets_details_optional
                        .as_ref()
                        .and_then(|d| d.amazon_pay.clone()),
                    samsung_pay: connector_wallets_details_optional
                        .as_ref()
                        .and_then(|d| d.samsung_pay.clone()),
                    paze: connector_wallets_details_optional
                        .as_ref()
                        .and_then(|d| d.paze.clone()),
                    google_pay: connector_wallets_details_optional
                        .as_ref()
                        .and_then(|d| d.google_pay.clone()),
                }
            }
            api_models::payments::ApplepaySessionTokenMetadata::ApplePay(apple_pay_metadata) => {
                let metadata_json = serde_json::to_value(apple_pay_metadata)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to serialize Apple Pay metadata as JSON")?;

                api_models::admin::ConnectorWalletDetails {
                    apple_pay: Some(masking::Secret::new(metadata_json)),
                    apple_pay_combined: connector_wallets_details_optional
                        .as_ref()
                        .and_then(|d| d.apple_pay_combined.clone()),
                    amazon_pay: connector_wallets_details_optional
                        .as_ref()
                        .and_then(|d| d.amazon_pay.clone()),
                    samsung_pay: connector_wallets_details_optional
                        .as_ref()
                        .and_then(|d| d.samsung_pay.clone()),
                    paze: connector_wallets_details_optional
                        .as_ref()
                        .and_then(|d| d.paze.clone()),
                    google_pay: connector_wallets_details_optional
                        .as_ref()
                        .and_then(|d| d.google_pay.clone()),
                }
            }
        };

        return Ok(Some(updated_wallet_details));
    }

    // Return connector_wallets_details if no Apple Pay metadata was found
    Ok(connector_wallets_details_optional)
}

pub fn get_applepay_metadata(
    connector_metadata: Option<pii::SecretSerdeValue>,
) -> RouterResult<api_models::payments::ApplepaySessionTokenMetadata> {
    connector_metadata
        .clone()
        .parse_value::<api_models::payments::ApplepayCombinedSessionTokenData>(
            "ApplepayCombinedSessionTokenData",
        )
        .map(|combined_metadata| {
            api_models::payments::ApplepaySessionTokenMetadata::ApplePayCombined(
                combined_metadata.apple_pay_combined,
            )
        })
        .or_else(|_| {
            connector_metadata
                .parse_value::<api_models::payments::ApplepaySessionTokenData>(
                    "ApplepaySessionTokenData",
                )
                .map(|old_metadata| {
                    api_models::payments::ApplepaySessionTokenMetadata::ApplePay(
                        old_metadata.apple_pay,
                    )
                })
        })
        .change_context(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "connector_metadata".to_string(),
            expected_format: "applepay_metadata_format".to_string(),
        })
}

pub fn calculate_debit_routing_savings(net_amount: i64, saving_percentage: f64) -> MinorUnit {
    logger::debug!(
        ?net_amount,
        ?saving_percentage,
        "Calculating debit routing saving amount"
    );

    let net_decimal = Decimal::from_i64(net_amount).unwrap_or_else(|| {
        logger::warn!(?net_amount, "Invalid net_amount, using 0");
        Decimal::ZERO
    });

    let percentage_decimal = Decimal::from_f64(saving_percentage).unwrap_or_else(|| {
        logger::warn!(?saving_percentage, "Invalid saving_percentage, using 0");
        Decimal::ZERO
    });

    let savings_decimal = net_decimal * percentage_decimal / Decimal::from(100);
    let rounded_savings = savings_decimal.round();

    let savings_int = rounded_savings.to_i64().unwrap_or_else(|| {
        logger::warn!(
            ?rounded_savings,
            "Debit routing savings calculation overflowed when converting to i64"
        );
        0
    });

    MinorUnit::new(savings_int)
}

pub fn get_debit_routing_savings_amount(
    payment_method_data: &domain::PaymentMethodData,
    payment_attempt: &PaymentAttempt,
) -> Option<MinorUnit> {
    let card_network = payment_attempt.extract_card_network()?;

    let saving_percentage =
        payment_method_data.extract_debit_routing_saving_percentage(&card_network)?;

    let net_amount = payment_attempt.get_total_amount().get_amount_as_i64();

    Some(calculate_debit_routing_savings(
        net_amount,
        saving_percentage,
    ))
}

#[cfg(all(feature = "retry", feature = "v1"))]
pub async fn get_apple_pay_retryable_connectors<F, D>(
    state: &SessionState,
    platform: &domain::Platform,
    payment_data: &D,
    pre_routing_connector_data_list: &[api::ConnectorRoutingData],
    merchant_connector_id: Option<&id_type::MerchantConnectorAccountId>,
    business_profile: domain::Profile,
) -> CustomResult<Option<Vec<api::ConnectorRoutingData>>, errors::ApiErrorResponse>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + Send,
{
    let profile_id = business_profile.get_id();

    let pre_decided_connector_data_first = pre_routing_connector_data_list
        .first()
        .ok_or(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration)?;

    let merchant_connector_account_type = get_merchant_connector_account(
        state,
        platform.get_processor().get_account().get_id(),
        payment_data.get_creds_identifier(),
        platform.get_processor().get_key_store(),
        profile_id,
        &pre_decided_connector_data_first
            .connector_data
            .connector_name
            .to_string(),
        merchant_connector_id,
    )
    .await?;

    let connector_data_list = if is_apple_pay_simplified_flow(
        merchant_connector_account_type.get_metadata(),
        merchant_connector_account_type
            .get_connector_name()
            .as_ref(),
    )? {
        let merchant_connector_account_list = state
            .store
            .find_merchant_connector_account_by_merchant_id_and_disabled_list(
                platform.get_processor().get_account().get_id(),
                false,
                platform.get_processor().get_key_store(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InternalServerError)?;

        let profile_specific_merchant_connector_account_list = merchant_connector_account_list
            .filter_based_on_profile_and_connector_type(
                profile_id,
                ConnectorType::PaymentProcessor,
            );

        let mut connector_data_list = vec![pre_decided_connector_data_first.clone()];

        for merchant_connector_account in profile_specific_merchant_connector_account_list {
            if is_apple_pay_simplified_flow(
                merchant_connector_account.metadata.clone(),
                Some(&merchant_connector_account.connector_name),
            )? {
                let routing_data: api::ConnectorRoutingData =
                    api::ConnectorData::get_connector_by_name(
                        &state.conf.connectors,
                        &merchant_connector_account.connector_name.to_string(),
                        api::GetToken::Connector,
                        Some(merchant_connector_account.get_id()),
                    )
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Invalid connector name received")?
                    .into();

                if !connector_data_list.iter().any(|connector_details| {
                    connector_details.connector_data.merchant_connector_id
                        == routing_data.connector_data.merchant_connector_id
                }) {
                    connector_data_list.push(routing_data)
                }
            }
        }
        #[cfg(feature = "v1")]
        let fallback_connetors_list = crate::core::routing::helpers::get_merchant_default_config(
            &*state.clone().store,
            profile_id.get_string_repr(),
            &api_enums::TransactionType::Payment,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get merchant default fallback connectors config")?;

        let mut routing_connector_data_list = Vec::new();

        pre_routing_connector_data_list.iter().for_each(|pre_val| {
            routing_connector_data_list.push(pre_val.connector_data.merchant_connector_id.clone())
        });

        fallback_connetors_list.iter().for_each(|fallback_val| {
            routing_connector_data_list
                .iter()
                .all(|val| *val != fallback_val.merchant_connector_id)
                .then(|| {
                    routing_connector_data_list.push(fallback_val.merchant_connector_id.clone())
                });
        });

        // connector_data_list is the list of connectors for which Apple Pay simplified flow is configured.
        // This list is arranged in the same order as the merchant's connectors routingconfiguration.

        let mut ordered_connector_data_list = Vec::new();

        routing_connector_data_list
            .iter()
            .for_each(|merchant_connector_id| {
                let connector_data = connector_data_list.iter().find(|connector_data| {
                    *merchant_connector_id == connector_data.connector_data.merchant_connector_id
                });
                if let Some(connector_data_details) = connector_data {
                    ordered_connector_data_list.push(connector_data_details.clone());
                }
            });

        Some(ordered_connector_data_list)
    } else {
        None
    };
    Ok(connector_data_list)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApplePayData {
    version: masking::Secret<String>,
    data: masking::Secret<String>,
    signature: masking::Secret<String>,
    header: ApplePayHeader,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplePayHeader {
    ephemeral_public_key: masking::Secret<String>,
    public_key_hash: masking::Secret<String>,
    transaction_id: masking::Secret<String>,
}

impl ApplePayData {
    pub fn token_json(
        wallet_data: domain::WalletData,
    ) -> CustomResult<Self, errors::ConnectorError> {
        let json_wallet_data: Self = connector::utils::WalletData::get_wallet_token_as_json(
            &wallet_data,
            "Apple Pay".to_string(),
        )?;
        Ok(json_wallet_data)
    }

    pub async fn decrypt(
        &self,
        payment_processing_certificate: &masking::Secret<String>,
        payment_processing_certificate_key: &masking::Secret<String>,
    ) -> CustomResult<serde_json::Value, errors::ApplePayDecryptionError> {
        let merchant_id = self.merchant_id(payment_processing_certificate)?;
        let shared_secret = self.shared_secret(payment_processing_certificate_key)?;
        let symmetric_key = self.symmetric_key(&merchant_id, &shared_secret)?;
        let decrypted = self.decrypt_ciphertext(&symmetric_key)?;
        let parsed_decrypted: serde_json::Value = serde_json::from_str(&decrypted)
            .change_context(errors::ApplePayDecryptionError::DecryptionFailed)?;
        Ok(parsed_decrypted)
    }

    pub fn merchant_id(
        &self,
        payment_processing_certificate: &masking::Secret<String>,
    ) -> CustomResult<String, errors::ApplePayDecryptionError> {
        let cert_data = payment_processing_certificate.clone().expose();

        let base64_decode_cert_data = BASE64_ENGINE
            .decode(cert_data)
            .change_context(errors::ApplePayDecryptionError::Base64DecodingFailed)?;

        // Parsing the certificate using x509-parser
        let (_, certificate) = parse_x509_certificate(&base64_decode_cert_data)
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
            .attach_printable("Unable to find merchant ID extension in the certificate")?;

        Ok(apple_pay_m_id)
    }

    pub fn shared_secret(
        &self,
        payment_processing_certificate_key: &masking::Secret<String>,
    ) -> CustomResult<Vec<u8>, errors::ApplePayDecryptionError> {
        let public_ec_bytes = BASE64_ENGINE
            .decode(self.header.ephemeral_public_key.peek().as_bytes())
            .change_context(errors::ApplePayDecryptionError::Base64DecodingFailed)?;

        let public_key = PKey::public_key_from_der(&public_ec_bytes)
            .change_context(errors::ApplePayDecryptionError::KeyDeserializationFailed)
            .attach_printable("Failed to deserialize the public key")?;

        let decrypted_apple_pay_ppc_key = payment_processing_certificate_key.clone().expose();

        // Create PKey objects from EcKey
        let private_key = PKey::private_key_from_pem(decrypted_apple_pay_ppc_key.as_bytes())
            .change_context(errors::ApplePayDecryptionError::KeyDeserializationFailed)
            .attach_printable("Failed to deserialize the private key")?;

        // Create the Deriver object and set the peer public key
        let mut deriver = Deriver::new(&private_key)
            .change_context(errors::ApplePayDecryptionError::DerivingSharedSecretKeyFailed)
            .attach_printable("Failed to create a deriver for the private key")?;

        deriver
            .set_peer(&public_key)
            .change_context(errors::ApplePayDecryptionError::DerivingSharedSecretKeyFailed)
            .attach_printable("Failed to set the peer key for the secret derivation")?;

        // Compute the shared secret
        let shared_secret = deriver
            .derive_to_vec()
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
        logger::info!("Decrypt apple pay token");

        let data = BASE64_ENGINE
            .decode(self.data.peek().as_bytes())
            .change_context(errors::ApplePayDecryptionError::Base64DecodingFailed)?;
        let iv = [0u8; 16]; //Initialization vector IV is typically used in AES-GCM (Galois/Counter Mode) encryption for randomizing the encryption process.
        let ciphertext = data
            .get(..data.len() - 16)
            .ok_or(errors::ApplePayDecryptionError::DecryptionFailed)?;
        let tag = data
            .get(data.len() - 16..)
            .ok_or(errors::ApplePayDecryptionError::DecryptionFailed)?;
        let cipher = Cipher::aes_256_gcm();
        let decrypted_data = decrypt_aead(cipher, symmetric_key, Some(&iv), &[], ciphertext, tag)
            .change_context(errors::ApplePayDecryptionError::DecryptionFailed)?;
        let decrypted = String::from_utf8(decrypted_data)
            .change_context(errors::ApplePayDecryptionError::DecryptionFailed)?;

        Ok(decrypted)
    }
}

// Structs for keys and the main decryptor
pub struct GooglePayTokenDecryptor {
    root_signing_keys: Vec<GooglePayRootSigningKey>,
    recipient_id: masking::Secret<String>,
    private_key: PKey<openssl::pkey::Private>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncryptedData {
    signature: String,
    intermediate_signing_key: IntermediateSigningKey,
    protocol_version: GooglePayProtocolVersion,
    #[serde(with = "common_utils::custom_serde::json_string")]
    signed_message: GooglePaySignedMessage,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GooglePaySignedMessage {
    #[serde(with = "common_utils::Base64Serializer")]
    encrypted_message: masking::Secret<Vec<u8>>,
    #[serde(with = "common_utils::Base64Serializer")]
    ephemeral_public_key: masking::Secret<Vec<u8>>,
    #[serde(with = "common_utils::Base64Serializer")]
    tag: masking::Secret<Vec<u8>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntermediateSigningKey {
    signed_key: masking::Secret<String>,
    signatures: Vec<masking::Secret<String>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GooglePaySignedKey {
    key_value: masking::Secret<String>,
    key_expiration: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GooglePayRootSigningKey {
    key_value: masking::Secret<String>,
    key_expiration: String,
    protocol_version: GooglePayProtocolVersion,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub enum GooglePayProtocolVersion {
    #[serde(rename = "ECv2")]
    EcProtocolVersion2,
}

// Check expiration date validity
fn check_expiration_date_is_valid(
    expiration: &str,
) -> CustomResult<bool, errors::GooglePayDecryptionError> {
    let expiration_ms = expiration
        .parse::<i128>()
        .change_context(errors::GooglePayDecryptionError::InvalidExpirationTime)?;
    // convert milliseconds to nanoseconds (1 millisecond = 1_000_000 nanoseconds) to create OffsetDateTime
    let expiration_time =
        time::OffsetDateTime::from_unix_timestamp_nanos(expiration_ms * 1_000_000)
            .change_context(errors::GooglePayDecryptionError::InvalidExpirationTime)?;
    let now = time::OffsetDateTime::now_utc();

    Ok(expiration_time > now)
}

// Construct little endian format of u32
fn get_little_endian_format(number: u32) -> Vec<u8> {
    number.to_le_bytes().to_vec()
}

// Filter and parse the root signing keys based on protocol version and expiration time
fn filter_root_signing_keys(
    root_signing_keys: Vec<GooglePayRootSigningKey>,
) -> CustomResult<Vec<GooglePayRootSigningKey>, errors::GooglePayDecryptionError> {
    let filtered_root_signing_keys = root_signing_keys
        .iter()
        .filter(|key| {
            key.protocol_version == GooglePayProtocolVersion::EcProtocolVersion2
                && matches!(
                    check_expiration_date_is_valid(&key.key_expiration).inspect_err(
                        |err| logger::warn!(
                            "Failed to check expirattion due to invalid format: {:?}",
                            err
                        )
                    ),
                    Ok(true)
                )
        })
        .cloned()
        .collect::<Vec<GooglePayRootSigningKey>>();

    logger::info!(
        "Filtered {} out of {} root signing keys",
        filtered_root_signing_keys.len(),
        root_signing_keys.len()
    );

    Ok(filtered_root_signing_keys)
}

impl GooglePayTokenDecryptor {
    pub fn new(
        root_keys: masking::Secret<String>,
        recipient_id: masking::Secret<String>,
        private_key: masking::Secret<String>,
    ) -> CustomResult<Self, errors::GooglePayDecryptionError> {
        // base64 decode the private key
        let decoded_key = BASE64_ENGINE
            .decode(private_key.expose())
            .change_context(errors::GooglePayDecryptionError::Base64DecodingFailed)?;
        // base64 decode the root signing keys
        let decoded_root_signing_keys = BASE64_ENGINE
            .decode(root_keys.expose())
            .change_context(errors::GooglePayDecryptionError::Base64DecodingFailed)?;
        // create a private key from the decoded key
        let private_key = PKey::private_key_from_pkcs8(&decoded_key)
            .change_context(errors::GooglePayDecryptionError::KeyDeserializationFailed)
            .attach_printable("cannot convert private key from decode_key")?;

        // parse the root signing keys
        let root_keys_vector: Vec<GooglePayRootSigningKey> = decoded_root_signing_keys
            .parse_struct("GooglePayRootSigningKey")
            .change_context(errors::GooglePayDecryptionError::DeserializationFailed)?;

        // parse and filter the root signing keys by protocol version
        let filtered_root_signing_keys = filter_root_signing_keys(root_keys_vector)?;

        Ok(Self {
            root_signing_keys: filtered_root_signing_keys,
            recipient_id,
            private_key,
        })
    }

    // Decrypt the Google pay token
    pub fn decrypt_token(
        &self,
        data: String,
        should_verify_signature: bool,
    ) -> CustomResult<
        hyperswitch_domain_models::router_data::GooglePayPredecryptDataInternal,
        errors::GooglePayDecryptionError,
    > {
        // parse the encrypted data
        let encrypted_data: EncryptedData = data
            .parse_struct("EncryptedData")
            .change_context(errors::GooglePayDecryptionError::DeserializationFailed)?;

        // verify the signature if required
        if should_verify_signature {
            self.verify_signature(&encrypted_data)?;
        }

        let ephemeral_public_key = encrypted_data.signed_message.ephemeral_public_key.peek();
        let tag = encrypted_data.signed_message.tag.peek();
        let encrypted_message = encrypted_data.signed_message.encrypted_message.peek();

        // derive the shared key
        let shared_key = self.get_shared_key(ephemeral_public_key)?;

        // derive the symmetric encryption key and MAC key
        let derived_key = self.derive_key(ephemeral_public_key, &shared_key)?;
        // First 32 bytes for AES-256 and Remaining bytes for HMAC
        let (symmetric_encryption_key, mac_key) = derived_key
            .split_at_checked(32)
            .ok_or(errors::GooglePayDecryptionError::ParsingFailed)?;

        // verify the HMAC of the message
        self.verify_hmac(mac_key, tag, encrypted_message)?;

        // decrypt the message
        let decrypted = self.decrypt_message(symmetric_encryption_key, encrypted_message)?;

        // parse the decrypted data
        let decrypted_data: hyperswitch_domain_models::router_data::GooglePayPredecryptDataInternal =
            decrypted
                .parse_struct("GooglePayPredecryptDataInternal")
                .change_context(errors::GooglePayDecryptionError::DeserializationFailed)?;

        // check the expiration date of the decrypted data
        if matches!(
            check_expiration_date_is_valid(&decrypted_data.message_expiration),
            Ok(true)
        ) {
            Ok(decrypted_data)
        } else {
            Err(errors::GooglePayDecryptionError::DecryptedTokenExpired.into())
        }
    }

    // Verify the signature of the token
    fn verify_signature(
        &self,
        encrypted_data: &EncryptedData,
    ) -> CustomResult<(), errors::GooglePayDecryptionError> {
        // check the protocol version
        if encrypted_data.protocol_version != GooglePayProtocolVersion::EcProtocolVersion2 {
            return Err(errors::GooglePayDecryptionError::InvalidProtocolVersion.into());
        }

        // verify the intermediate signing key
        self.verify_intermediate_signing_key(encrypted_data)?;
        // validate and fetch the signed key
        let signed_key = self.validate_signed_key(&encrypted_data.intermediate_signing_key)?;
        // verify the signature of the token
        self.verify_message_signature(encrypted_data, &signed_key)
    }

    // Verify the intermediate signing key
    fn verify_intermediate_signing_key(
        &self,
        encrypted_data: &EncryptedData,
    ) -> CustomResult<(), errors::GooglePayDecryptionError> {
        let mut signatrues: Vec<openssl::ecdsa::EcdsaSig> = Vec::new();

        // decode and parse the signatures
        for signature in encrypted_data.intermediate_signing_key.signatures.iter() {
            let signature = BASE64_ENGINE
                .decode(signature.peek())
                .change_context(errors::GooglePayDecryptionError::Base64DecodingFailed)?;
            let ecdsa_signature = openssl::ecdsa::EcdsaSig::from_der(&signature)
                .change_context(errors::GooglePayDecryptionError::EcdsaSignatureParsingFailed)?;
            signatrues.push(ecdsa_signature);
        }

        // get the sender id i.e. Google
        let sender_id = String::from_utf8(consts::SENDER_ID.to_vec())
            .change_context(errors::GooglePayDecryptionError::DeserializationFailed)?;

        // construct the signed data
        let signed_data = self.construct_signed_data_for_intermediate_signing_key_verification(
            &sender_id,
            consts::PROTOCOL,
            encrypted_data.intermediate_signing_key.signed_key.peek(),
        )?;

        // check if any of the signatures are valid for any of the root signing keys
        for key in self.root_signing_keys.iter() {
            // decode and create public key
            let public_key = self
                .load_public_key(key.key_value.peek())
                .change_context(errors::GooglePayDecryptionError::DerivingPublicKeyFailed)?;
            // fetch the ec key from public key
            let ec_key = public_key
                .ec_key()
                .change_context(errors::GooglePayDecryptionError::DerivingEcKeyFailed)?;

            // hash the signed data
            let message_hash = openssl::sha::sha256(&signed_data);

            // verify if any of the signatures is valid against the given key
            for signature in signatrues.iter() {
                let result = signature.verify(&message_hash, &ec_key).change_context(
                    errors::GooglePayDecryptionError::SignatureVerificationFailed,
                )?;

                if result {
                    return Ok(());
                }
            }
        }

        Err(errors::GooglePayDecryptionError::InvalidIntermediateSignature.into())
    }

    // Construct signed data for intermediate signing key verification
    fn construct_signed_data_for_intermediate_signing_key_verification(
        &self,
        sender_id: &str,
        protocol_version: &str,
        signed_key: &str,
    ) -> CustomResult<Vec<u8>, errors::GooglePayDecryptionError> {
        let length_of_sender_id = u32::try_from(sender_id.len())
            .change_context(errors::GooglePayDecryptionError::ParsingFailed)?;
        let length_of_protocol_version = u32::try_from(protocol_version.len())
            .change_context(errors::GooglePayDecryptionError::ParsingFailed)?;
        let length_of_signed_key = u32::try_from(signed_key.len())
            .change_context(errors::GooglePayDecryptionError::ParsingFailed)?;

        let mut signed_data: Vec<u8> = Vec::new();
        signed_data.append(&mut get_little_endian_format(length_of_sender_id));
        signed_data.append(&mut sender_id.as_bytes().to_vec());
        signed_data.append(&mut get_little_endian_format(length_of_protocol_version));
        signed_data.append(&mut protocol_version.as_bytes().to_vec());
        signed_data.append(&mut get_little_endian_format(length_of_signed_key));
        signed_data.append(&mut signed_key.as_bytes().to_vec());

        Ok(signed_data)
    }

    // Validate and parse signed key
    fn validate_signed_key(
        &self,
        intermediate_signing_key: &IntermediateSigningKey,
    ) -> CustomResult<GooglePaySignedKey, errors::GooglePayDecryptionError> {
        let signed_key: GooglePaySignedKey = intermediate_signing_key
            .signed_key
            .clone()
            .expose()
            .parse_struct("GooglePaySignedKey")
            .change_context(errors::GooglePayDecryptionError::SignedKeyParsingFailure)?;
        if !matches!(
            check_expiration_date_is_valid(&signed_key.key_expiration),
            Ok(true)
        ) {
            return Err(errors::GooglePayDecryptionError::SignedKeyExpired)?;
        }
        Ok(signed_key)
    }

    // Verify the signed message
    fn verify_message_signature(
        &self,
        encrypted_data: &EncryptedData,
        signed_key: &GooglePaySignedKey,
    ) -> CustomResult<(), errors::GooglePayDecryptionError> {
        // create a public key from the intermediate signing key
        let public_key = self.load_public_key(signed_key.key_value.peek())?;
        // base64 decode the signature
        let signature = BASE64_ENGINE
            .decode(&encrypted_data.signature)
            .change_context(errors::GooglePayDecryptionError::Base64DecodingFailed)?;

        // parse the signature using ECDSA
        let ecdsa_signature = openssl::ecdsa::EcdsaSig::from_der(&signature)
            .change_context(errors::GooglePayDecryptionError::EcdsaSignatureFailed)?;

        // get the EC key from the public key
        let ec_key = public_key
            .ec_key()
            .change_context(errors::GooglePayDecryptionError::DerivingEcKeyFailed)?;

        // get the sender id i.e. Google
        let sender_id = String::from_utf8(consts::SENDER_ID.to_vec())
            .change_context(errors::GooglePayDecryptionError::DeserializationFailed)?;

        // serialize the signed message to string
        let signed_message = serde_json::to_string(&encrypted_data.signed_message)
            .change_context(errors::GooglePayDecryptionError::SignedKeyParsingFailure)?;

        // construct the signed data
        let signed_data = self.construct_signed_data_for_signature_verification(
            &sender_id,
            consts::PROTOCOL,
            &signed_message,
        )?;

        // hash the signed data
        let message_hash = openssl::sha::sha256(&signed_data);

        // verify the signature
        let result = ecdsa_signature
            .verify(&message_hash, &ec_key)
            .change_context(errors::GooglePayDecryptionError::SignatureVerificationFailed)?;

        if result {
            Ok(())
        } else {
            Err(errors::GooglePayDecryptionError::InvalidSignature)?
        }
    }

    // Fetch the public key
    fn load_public_key(
        &self,
        key: &str,
    ) -> CustomResult<PKey<openssl::pkey::Public>, errors::GooglePayDecryptionError> {
        // decode the base64 string
        let der_data = BASE64_ENGINE
            .decode(key)
            .change_context(errors::GooglePayDecryptionError::Base64DecodingFailed)?;

        // parse the DER-encoded data as an EC public key
        let ec_key = openssl::ec::EcKey::public_key_from_der(&der_data)
            .change_context(errors::GooglePayDecryptionError::DerivingEcKeyFailed)?;

        // wrap the EC key in a PKey (a more general-purpose public key type in OpenSSL)
        let public_key = PKey::from_ec_key(ec_key)
            .change_context(errors::GooglePayDecryptionError::DerivingPublicKeyFailed)?;

        Ok(public_key)
    }

    // Construct signed data for signature verification
    fn construct_signed_data_for_signature_verification(
        &self,
        sender_id: &str,
        protocol_version: &str,
        signed_key: &str,
    ) -> CustomResult<Vec<u8>, errors::GooglePayDecryptionError> {
        let recipient_id = self.recipient_id.clone().expose();
        let length_of_sender_id = u32::try_from(sender_id.len())
            .change_context(errors::GooglePayDecryptionError::ParsingFailed)?;
        let length_of_recipient_id = u32::try_from(recipient_id.len())
            .change_context(errors::GooglePayDecryptionError::ParsingFailed)?;
        let length_of_protocol_version = u32::try_from(protocol_version.len())
            .change_context(errors::GooglePayDecryptionError::ParsingFailed)?;
        let length_of_signed_key = u32::try_from(signed_key.len())
            .change_context(errors::GooglePayDecryptionError::ParsingFailed)?;

        let mut signed_data: Vec<u8> = Vec::new();
        signed_data.append(&mut get_little_endian_format(length_of_sender_id));
        signed_data.append(&mut sender_id.as_bytes().to_vec());
        signed_data.append(&mut get_little_endian_format(length_of_recipient_id));
        signed_data.append(&mut recipient_id.as_bytes().to_vec());
        signed_data.append(&mut get_little_endian_format(length_of_protocol_version));
        signed_data.append(&mut protocol_version.as_bytes().to_vec());
        signed_data.append(&mut get_little_endian_format(length_of_signed_key));
        signed_data.append(&mut signed_key.as_bytes().to_vec());

        Ok(signed_data)
    }

    // Derive a shared key using ECDH
    fn get_shared_key(
        &self,
        ephemeral_public_key_bytes: &[u8],
    ) -> CustomResult<Vec<u8>, errors::GooglePayDecryptionError> {
        let group = openssl::ec::EcGroup::from_curve_name(openssl::nid::Nid::X9_62_PRIME256V1)
            .change_context(errors::GooglePayDecryptionError::DerivingEcGroupFailed)?;

        let mut big_num_context = openssl::bn::BigNumContext::new()
            .change_context(errors::GooglePayDecryptionError::BigNumAllocationFailed)?;

        let ec_key = openssl::ec::EcPoint::from_bytes(
            &group,
            ephemeral_public_key_bytes,
            &mut big_num_context,
        )
        .change_context(errors::GooglePayDecryptionError::DerivingEcKeyFailed)?;

        // create an ephemeral public key from the given bytes
        let ephemeral_public_key = openssl::ec::EcKey::from_public_key(&group, &ec_key)
            .change_context(errors::GooglePayDecryptionError::DerivingPublicKeyFailed)?;

        // wrap the public key in a PKey
        let ephemeral_pkey = PKey::from_ec_key(ephemeral_public_key)
            .change_context(errors::GooglePayDecryptionError::DerivingPublicKeyFailed)?;

        // perform ECDH to derive the shared key
        let mut deriver = Deriver::new(&self.private_key)
            .change_context(errors::GooglePayDecryptionError::DerivingSharedSecretKeyFailed)?;

        deriver
            .set_peer(&ephemeral_pkey)
            .change_context(errors::GooglePayDecryptionError::DerivingSharedSecretKeyFailed)?;

        let shared_key = deriver
            .derive_to_vec()
            .change_context(errors::GooglePayDecryptionError::DerivingSharedSecretKeyFailed)?;

        Ok(shared_key)
    }

    // Derive symmetric key and MAC key using HKDF
    fn derive_key(
        &self,
        ephemeral_public_key_bytes: &[u8],
        shared_key: &[u8],
    ) -> CustomResult<Vec<u8>, errors::GooglePayDecryptionError> {
        // concatenate ephemeral public key and shared key
        let input_key_material = [ephemeral_public_key_bytes, shared_key].concat();

        // initialize HKDF with SHA-256 as the hash function
        // Salt is not provided as per the Google Pay documentation
        // https://developers.google.com/pay/api/android/guides/resources/payment-data-cryptography#encrypt-spec
        let hkdf: ::hkdf::Hkdf<sha2::Sha256> = ::hkdf::Hkdf::new(None, &input_key_material);

        // derive 64 bytes for the output key (symmetric encryption + MAC key)
        let mut output_key = vec![0u8; 64];
        hkdf.expand(consts::SENDER_ID, &mut output_key)
            .map_err(|err| {
                logger::error!(
                "Failed to derive the shared ephemeral key for Google Pay decryption flow: {:?}",
                err
            );
                report!(errors::GooglePayDecryptionError::DerivingSharedEphemeralKeyFailed)
            })?;

        Ok(output_key)
    }

    // Verify the Hmac key
    // https://developers.google.com/pay/api/android/guides/resources/payment-data-cryptography#encrypt-spec
    fn verify_hmac(
        &self,
        mac_key: &[u8],
        tag: &[u8],
        encrypted_message: &[u8],
    ) -> CustomResult<(), errors::GooglePayDecryptionError> {
        let hmac_key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, mac_key);
        ring::hmac::verify(&hmac_key, encrypted_message, tag)
            .change_context(errors::GooglePayDecryptionError::HmacVerificationFailed)
    }

    // Method to decrypt the AES-GCM encrypted message
    fn decrypt_message(
        &self,
        symmetric_key: &[u8],
        encrypted_message: &[u8],
    ) -> CustomResult<Vec<u8>, errors::GooglePayDecryptionError> {
        //initialization vector IV is typically used in AES-GCM (Galois/Counter Mode) encryption for randomizing the encryption process.
        // zero iv is being passed as specified in Google Pay documentation
        // https://developers.google.com/pay/api/android/guides/resources/payment-data-cryptography#decrypt-token
        let iv = [0u8; 16];

        // extract the tag from the end of the encrypted message
        let tag = encrypted_message
            .get(encrypted_message.len() - 16..)
            .ok_or(errors::GooglePayDecryptionError::ParsingTagError)?;

        // decrypt the message using AES-256-CTR
        let cipher = Cipher::aes_256_ctr();
        let decrypted_data = decrypt_aead(
            cipher,
            symmetric_key,
            Some(&iv),
            &[],
            encrypted_message,
            tag,
        )
        .change_context(errors::GooglePayDecryptionError::DecryptionFailed)?;

        Ok(decrypted_data)
    }
}

pub fn decrypt_paze_token(
    paze_wallet_data: PazeWalletData,
    paze_private_key: masking::Secret<String>,
    paze_private_key_passphrase: masking::Secret<String>,
) -> CustomResult<serde_json::Value, errors::PazeDecryptionError> {
    let decoded_paze_private_key = BASE64_ENGINE
        .decode(paze_private_key.expose().as_bytes())
        .change_context(errors::PazeDecryptionError::Base64DecodingFailed)?;
    let decrypted_private_key = openssl::rsa::Rsa::private_key_from_pem_passphrase(
        decoded_paze_private_key.as_slice(),
        paze_private_key_passphrase.expose().as_bytes(),
    )
    .change_context(errors::PazeDecryptionError::CertificateParsingFailed)?;
    let decrypted_private_key_pem = String::from_utf8(
        decrypted_private_key
            .private_key_to_pem()
            .change_context(errors::PazeDecryptionError::CertificateParsingFailed)?,
    )
    .change_context(errors::PazeDecryptionError::CertificateParsingFailed)?;
    let decrypter = jwe::RSA_OAEP_256
        .decrypter_from_pem(decrypted_private_key_pem)
        .change_context(errors::PazeDecryptionError::CertificateParsingFailed)?;

    let paze_complete_response: Vec<&str> = paze_wallet_data
        .complete_response
        .peek()
        .split('.')
        .collect();
    let encrypted_jwe_key = paze_complete_response
        .get(1)
        .ok_or(errors::PazeDecryptionError::DecryptionFailed)?
        .to_string();
    let decoded_jwe_key = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(encrypted_jwe_key)
        .change_context(errors::PazeDecryptionError::Base64DecodingFailed)?;
    let jws_body: JwsBody = serde_json::from_slice(&decoded_jwe_key)
        .change_context(errors::PazeDecryptionError::DecryptionFailed)?;

    let (deserialized_payload, _deserialized_header) =
        jwe::deserialize_compact(jws_body.secured_payload.peek(), &decrypter)
            .change_context(errors::PazeDecryptionError::DecryptionFailed)?;
    let encoded_secured_payload_element = String::from_utf8(deserialized_payload)
        .change_context(errors::PazeDecryptionError::DecryptionFailed)?
        .split('.')
        .collect::<Vec<&str>>()
        .get(1)
        .ok_or(errors::PazeDecryptionError::DecryptionFailed)?
        .to_string();
    let decoded_secured_payload_element = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(encoded_secured_payload_element)
        .change_context(errors::PazeDecryptionError::Base64DecodingFailed)?;
    let parsed_decrypted: serde_json::Value =
        serde_json::from_slice(&decoded_secured_payload_element)
            .change_context(errors::PazeDecryptionError::DecryptionFailed)?;
    Ok(parsed_decrypted)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JwsBody {
    pub payload_id: String,
    pub session_id: String,
    pub secured_payload: masking::Secret<String>,
}

pub fn get_key_params_for_surcharge_details(
    payment_method_data: &domain::PaymentMethodData,
) -> Option<(
    common_enums::PaymentMethod,
    common_enums::PaymentMethodType,
    Option<common_enums::CardNetwork>,
)> {
    match payment_method_data {
        domain::PaymentMethodData::Card(card) => {
            // surcharge generated will always be same for credit as well as debit
            // since surcharge conditions cannot be defined on card_type
            Some((
                common_enums::PaymentMethod::Card,
                common_enums::PaymentMethodType::Credit,
                card.card_network.clone(),
            ))
        }
        domain::PaymentMethodData::CardRedirect(card_redirect_data) => Some((
            common_enums::PaymentMethod::CardRedirect,
            card_redirect_data.get_payment_method_type(),
            None,
        )),
        domain::PaymentMethodData::Wallet(wallet) => Some((
            common_enums::PaymentMethod::Wallet,
            wallet.get_payment_method_type(),
            None,
        )),
        domain::PaymentMethodData::PayLater(pay_later) => Some((
            common_enums::PaymentMethod::PayLater,
            pay_later.get_payment_method_type(),
            None,
        )),
        domain::PaymentMethodData::BankRedirect(bank_redirect) => Some((
            common_enums::PaymentMethod::BankRedirect,
            bank_redirect.get_payment_method_type(),
            None,
        )),
        domain::PaymentMethodData::BankDebit(bank_debit) => Some((
            common_enums::PaymentMethod::BankDebit,
            bank_debit.get_payment_method_type(),
            None,
        )),
        domain::PaymentMethodData::BankTransfer(bank_transfer) => Some((
            common_enums::PaymentMethod::BankTransfer,
            bank_transfer.get_payment_method_type(),
            None,
        )),
        domain::PaymentMethodData::Crypto(crypto) => Some((
            common_enums::PaymentMethod::Crypto,
            crypto.get_payment_method_type(),
            None,
        )),
        domain::PaymentMethodData::MandatePayment => None,
        domain::PaymentMethodData::Reward => None,
        domain::PaymentMethodData::RealTimePayment(real_time_payment) => Some((
            common_enums::PaymentMethod::RealTimePayment,
            real_time_payment.get_payment_method_type(),
            None,
        )),
        domain::PaymentMethodData::Upi(upi_data) => Some((
            common_enums::PaymentMethod::Upi,
            upi_data.get_payment_method_type(),
            None,
        )),
        domain::PaymentMethodData::Voucher(voucher) => Some((
            common_enums::PaymentMethod::Voucher,
            voucher.get_payment_method_type(),
            None,
        )),
        domain::PaymentMethodData::GiftCard(gift_card) => Some((
            common_enums::PaymentMethod::GiftCard,
            gift_card.get_payment_method_type(),
            None,
        )),
        domain::PaymentMethodData::OpenBanking(ob_data) => Some((
            common_enums::PaymentMethod::OpenBanking,
            ob_data.get_payment_method_type(),
            None,
        )),
        domain::PaymentMethodData::MobilePayment(mobile_payment) => Some((
            common_enums::PaymentMethod::MobilePayment,
            mobile_payment.get_payment_method_type(),
            None,
        )),
        domain::PaymentMethodData::CardToken(_)
        | domain::PaymentMethodData::NetworkToken(_)
        | domain::PaymentMethodData::CardDetailsForNetworkTransactionId(_) => None,
    }
}

pub fn validate_payment_link_request(
    request: &api::PaymentsRequest,
) -> Result<(), errors::ApiErrorResponse> {
    #[cfg(feature = "v1")]
    if request.confirm == Some(true) {
        return Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "cannot confirm a payment while creating a payment link".to_string(),
        });
    }

    if request.return_url.is_none() {
        return Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "return_url must be sent while creating a payment link".to_string(),
        });
    }
    Ok(())
}

pub async fn get_gsm_record(
    state: &SessionState,
    error_code: Option<String>,
    error_message: Option<String>,
    connector_name: String,
    flow: String,
) -> Option<hyperswitch_domain_models::gsm::GatewayStatusMap> {
    let get_gsm = || async {
        state.store.find_gsm_rule(
                connector_name.clone(),
                flow.clone(),
                "sub_flow".to_string(),
                error_code.clone().unwrap_or_default(), // TODO: make changes in connector to get a mandatory code in case of success or error response
                error_message.clone().unwrap_or_default(),
            )
            .await
            .map_err(|err| {
                if err.current_context().is_db_not_found() {
                    logger::warn!(
                        "GSM miss for connector - {}, flow - {}, error_code - {:?}, error_message - {:?}",
                        connector_name,
                        flow,
                        error_code,
                        error_message
                    );
                    metrics::AUTO_RETRY_GSM_MISS_COUNT.add( 1, &[]);
                } else {
                    metrics::AUTO_RETRY_GSM_FETCH_FAILURE_COUNT.add( 1, &[]);
                };
                err.change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("failed to fetch decision from gsm")
            })
    };
    get_gsm()
        .await
        .inspect_err(|err| {
            // warn log should suffice here because we are not propagating this error
            logger::warn!(get_gsm_decision_fetch_error=?err, "error fetching gsm decision");
        })
        .ok()
}

pub async fn get_unified_translation(
    state: &SessionState,
    unified_code: String,
    unified_message: String,
    locale: String,
) -> Option<String> {
    let get_unified_translation = || async {
        state.store.find_translation(
                unified_code.clone(),
                unified_message.clone(),
                locale.clone(),
            )
            .await
            .map_err(|err| {
                if err.current_context().is_db_not_found() {
                    logger::warn!(
                        "Translation missing for unified_code - {:?}, unified_message - {:?}, locale - {:?}",
                        unified_code,
                        unified_message,
                        locale
                    );
                }
                err.change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("failed to fetch translation from unified_translations")
            })
    };
    get_unified_translation()
        .await
        .inspect_err(|err| {
            // warn log should suffice here because we are not propagating this error
            logger::warn!(get_translation_error=?err, "error fetching unified translations");
        })
        .ok()
}
pub fn validate_order_details_amount(
    order_details: Vec<api_models::payments::OrderDetailsWithAmount>,
    amount: MinorUnit,
    should_validate: bool,
) -> Result<(), errors::ApiErrorResponse> {
    if should_validate {
        let total_order_details_amount: MinorUnit = order_details
            .iter()
            .map(|order| order.amount * order.quantity)
            .sum();

        if total_order_details_amount != amount {
            Err(errors::ApiErrorResponse::InvalidRequestData {
                message: "Total sum of order details doesn't match amount in payment request"
                    .to_string(),
            })
        } else {
            Ok(())
        }
    } else {
        Ok(())
    }
}

// This function validates the client secret expiry set by the merchant in the request
pub fn validate_session_expiry(session_expiry: u32) -> Result<(), errors::ApiErrorResponse> {
    if !(consts::MIN_SESSION_EXPIRY..=consts::MAX_SESSION_EXPIRY).contains(&session_expiry) {
        Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "session_expiry should be between 60(1 min) to 7890000(3 months).".to_string(),
        })
    } else {
        Ok(())
    }
}

pub fn get_recipient_id_for_open_banking(
    merchant_data: &AdditionalMerchantData,
) -> Result<Option<String>, errors::ApiErrorResponse> {
    match merchant_data {
        AdditionalMerchantData::OpenBankingRecipientData(data) => match data {
            MerchantRecipientData::ConnectorRecipientId(id) => Ok(Some(id.peek().clone())),
            MerchantRecipientData::AccountData(acc_data) => {
                let connector_recipient_id = match acc_data {
                    MerchantAccountData::Bacs {
                        connector_recipient_id,
                        ..
                    }
                    | MerchantAccountData::Iban {
                        connector_recipient_id,
                        ..
                    }
                    | MerchantAccountData::FasterPayments {
                        connector_recipient_id,
                        ..
                    }
                    | MerchantAccountData::Sepa {
                        connector_recipient_id,
                        ..
                    }
                    | MerchantAccountData::SepaInstant {
                        connector_recipient_id,
                        ..
                    }
                    | MerchantAccountData::Elixir {
                        connector_recipient_id,
                        ..
                    }
                    | MerchantAccountData::Bankgiro {
                        connector_recipient_id,
                        ..
                    }
                    | MerchantAccountData::Plusgiro {
                        connector_recipient_id,
                        ..
                    } => connector_recipient_id,
                };

                match connector_recipient_id {
                    Some(RecipientIdType::ConnectorId(id)) => Ok(Some(id.peek().clone())),
                    Some(RecipientIdType::LockerId(id)) => Ok(Some(id.peek().clone())),
                    _ => Err(errors::ApiErrorResponse::InvalidConnectorConfiguration {
                        config: "recipient_id".to_string(),
                    }),
                }
            }
            _ => Err(errors::ApiErrorResponse::InvalidConnectorConfiguration {
                config: "recipient_id".to_string(),
            }),
        },
    }
}

pub fn get_connector_data_with_token(
    state: &SessionState,
    connector_name: String,
    merchant_connector_account_id: Option<id_type::MerchantConnectorAccountId>,
    payment_method_type: api_models::enums::PaymentMethodType,
) -> RouterResult<api::ConnectorData> {
    let connector_data_result = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector_name.to_string(),
        // Default value, will be replaced by the result of decide_session_token_flow
        api::GetToken::Connector,
        merchant_connector_account_id.clone(),
    );
    let connector_type = decide_session_token_flow(
        &connector_data_result?.connector,
        payment_method_type,
        connector_name.clone(),
    );

    logger::debug!(session_token_flow=?connector_type, "Session token flow decided for payment method type: {:?}", payment_method_type);

    api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector_name.to_string(),
        connector_type,
        merchant_connector_account_id,
    )
    .inspect_err(|err| {
        logger::error!(session_token_error=?err);
    })
}

/// Decides the session token flow based on payment method type
pub fn decide_session_token_flow(
    connector: &hyperswitch_interfaces::connector_integration_interface::ConnectorEnum,
    payment_method_type: api_models::enums::PaymentMethodType,
    connector_name: String,
) -> api::GetToken {
    if connector.validate_sdk_session_token_for_payment_method(&payment_method_type) {
        logger::debug!(
            "SDK session token validation succeeded for payment_method_type {:?} in connector {} , proceeding with Connector token flow",
            payment_method_type, connector_name
        );
        return api::GetToken::Connector;
    }

    match payment_method_type {
        api_models::enums::PaymentMethodType::ApplePay => api::GetToken::ApplePayMetadata,
        api_models::enums::PaymentMethodType::GooglePay => api::GetToken::GpayMetadata,
        api_models::enums::PaymentMethodType::Paypal => api::GetToken::PaypalSdkMetadata,
        api_models::enums::PaymentMethodType::SamsungPay => api::GetToken::SamsungPayMetadata,
        api_models::enums::PaymentMethodType::Paze => api::GetToken::PazeMetadata,
        api_models::enums::PaymentMethodType::AmazonPay => api::GetToken::AmazonPayMetadata,
        _ => api::GetToken::Connector,
    }
}
// This function validates the intent fulfillment time expiry set by the merchant in the request
pub fn validate_intent_fulfillment_expiry(
    intent_fulfillment_time: u32,
) -> Result<(), errors::ApiErrorResponse> {
    if !(consts::MIN_INTENT_FULFILLMENT_EXPIRY..=consts::MAX_INTENT_FULFILLMENT_EXPIRY)
        .contains(&intent_fulfillment_time)
    {
        Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "intent_fulfillment_time should be between 60(1 min) to 1800(30 mins)."
                .to_string(),
        })
    } else {
        Ok(())
    }
}

pub fn add_connector_response_to_additional_payment_data(
    additional_payment_data: api_models::payments::AdditionalPaymentData,
    connector_response_payment_method_data: AdditionalPaymentMethodConnectorResponse,
) -> api_models::payments::AdditionalPaymentData {
    match (
        &additional_payment_data,
        connector_response_payment_method_data,
    ) {
        (
            api_models::payments::AdditionalPaymentData::Card(additional_card_data),
            AdditionalPaymentMethodConnectorResponse::Card {
                authentication_data,
                payment_checks,
                ..
            },
        ) => api_models::payments::AdditionalPaymentData::Card(Box::new(
            api_models::payments::AdditionalCardInfo {
                payment_checks,
                authentication_data,
                ..*additional_card_data.clone()
            },
        )),
        (
            api_models::payments::AdditionalPaymentData::PayLater { .. },
            AdditionalPaymentMethodConnectorResponse::PayLater {
                klarna_sdk: Some(KlarnaSdkResponse { payment_type }),
            },
        ) => api_models::payments::AdditionalPaymentData::PayLater {
            klarna_sdk: Some(api_models::payments::KlarnaSdkPaymentMethod { payment_type }),
        },
        (
            api_models::payments::AdditionalPaymentData::BankRedirect { .. },
            AdditionalPaymentMethodConnectorResponse::BankRedirect {
                interac: Some(InteracCustomerInfo { customer_info }),
            },
        ) => api_models::payments::AdditionalPaymentData::BankRedirect {
            bank_name: None,
            details: None,
            interac: Some(api_models::payments::InteracPaymentMethod { customer_info }),
        },

        _ => additional_payment_data,
    }
}

pub fn update_additional_payment_data_with_connector_response_pm_data(
    additional_payment_data: Option<serde_json::Value>,
    connector_response_pm_data: Option<AdditionalPaymentMethodConnectorResponse>,
) -> RouterResult<Option<serde_json::Value>> {
    let parsed_additional_payment_method_data = additional_payment_data
        .as_ref()
        .map(|payment_method_data| {
            payment_method_data
                .clone()
                .parse_value::<api_models::payments::AdditionalPaymentData>(
                    "additional_payment_method_data",
                )
        })
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("unable to parse value into additional_payment_method_data")?;

    let additional_payment_method_data = parsed_additional_payment_method_data
        .zip(connector_response_pm_data)
        .map(|(additional_pm_data, connector_response_pm_data)| {
            add_connector_response_to_additional_payment_data(
                additional_pm_data,
                connector_response_pm_data,
            )
        });

    additional_payment_method_data
        .as_ref()
        .map(Encode::encode_to_value)
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to encode additional pm data")
}

#[cfg(feature = "v2")]
pub async fn get_payment_method_details_from_payment_token(
    state: &SessionState,
    payment_attempt: &PaymentAttempt,
    payment_intent: &PaymentIntent,
    key_store: &domain::MerchantKeyStore,
    storage_scheme: enums::MerchantStorageScheme,
) -> RouterResult<Option<(domain::PaymentMethodData, enums::PaymentMethod)>> {
    todo!()
}

#[cfg(feature = "v1")]
pub async fn get_payment_method_details_from_payment_token(
    state: &SessionState,
    payment_attempt: &PaymentAttempt,
    payment_intent: &PaymentIntent,
    key_store: &domain::MerchantKeyStore,
    storage_scheme: enums::MerchantStorageScheme,
) -> RouterResult<Option<(domain::PaymentMethodData, enums::PaymentMethod)>> {
    let hyperswitch_token = if let Some(token) = payment_attempt.payment_token.clone() {
        let redis_conn = state
            .store
            .get_redis_conn()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to get redis connection")?;
        let key = format!(
            "pm_token_{}_{}_hyperswitch",
            token,
            payment_attempt
                .payment_method
                .to_owned()
                .get_required_value("payment_method")?,
        );
        let token_data_string = redis_conn
            .get_key::<Option<String>>(&key.into())
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to fetch the token from redis")?
            .ok_or(error_stack::Report::new(
                errors::ApiErrorResponse::UnprocessableEntity {
                    message: "Token is invalid or expired".to_owned(),
                },
            ))?;
        let token_data_result = token_data_string
            .clone()
            .parse_struct("PaymentTokenData")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("failed to deserialize hyperswitch token data");
        let token_data = match token_data_result {
            Ok(data) => data,
            Err(e) => {
                // The purpose of this logic is backwards compatibility to support tokens
                // in redis that might be following the old format.
                if token_data_string.starts_with('{') {
                    return Err(e);
                } else {
                    storage::PaymentTokenData::temporary_generic(token_data_string)
                }
            }
        };
        Some(token_data)
    } else {
        None
    };
    let token = hyperswitch_token
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("missing hyperswitch_token")?;
    match token {
        storage::PaymentTokenData::TemporaryGeneric(generic_token) => {
            retrieve_payment_method_with_temporary_token(
                state,
                &generic_token.token,
                payment_intent,
                payment_attempt,
                key_store,
                None,
            )
            .await
        }

        storage::PaymentTokenData::Temporary(generic_token) => {
            retrieve_payment_method_with_temporary_token(
                state,
                &generic_token.token,
                payment_intent,
                payment_attempt,
                key_store,
                None,
            )
            .await
        }

        storage::PaymentTokenData::Permanent(card_token) => {
            retrieve_card_with_permanent_token_for_external_authentication(
                state,
                &card_token.token,
                payment_intent,
                None,
                key_store,
                storage_scheme,
            )
            .await
            .map(|card| Some((card, enums::PaymentMethod::Card)))
        }

        storage::PaymentTokenData::PermanentCard(card_token) => {
            retrieve_card_with_permanent_token_for_external_authentication(
                state,
                &card_token.token,
                payment_intent,
                None,
                key_store,
                storage_scheme,
            )
            .await
            .map(|card| Some((card, enums::PaymentMethod::Card)))
        }

        storage::PaymentTokenData::AuthBankDebit(auth_token) => {
            retrieve_payment_method_from_auth_service(
                state,
                key_store,
                &auth_token,
                payment_intent,
                &None,
            )
            .await
        }

        storage::PaymentTokenData::WalletToken(_) => Ok(None),
    }
}

// This function validates the  mandate_data with its setup_future_usage
pub fn validate_mandate_data_and_future_usage(
    setup_future_usages: Option<api_enums::FutureUsage>,
    mandate_details_present: bool,
) -> Result<(), errors::ApiErrorResponse> {
    if mandate_details_present
        && (Some(api_enums::FutureUsage::OnSession) == setup_future_usages
            || setup_future_usages.is_none())
    {
        Err(errors::ApiErrorResponse::PreconditionFailed {
            message: "`setup_future_usage` must be `off_session` for mandates".into(),
        })
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum UnifiedAuthenticationServiceFlow {
    ClickToPayInitiate,
    ExternalAuthenticationInitiate {
        acquirer_details: Option<authentication::types::AcquirerDetails>,
        card: Box<hyperswitch_domain_models::payment_method_data::Card>,
        token: String,
    },
    ExternalAuthenticationPostAuthenticate {
        authentication_id: id_type::AuthenticationId,
    },
}

#[cfg(feature = "v1")]
pub async fn decide_action_for_unified_authentication_service<F: Clone>(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    business_profile: &domain::Profile,
    payment_data: &mut PaymentData<F>,
    connector_call_type: &api::ConnectorCallType,
    mandate_type: Option<api_models::payments::MandateTransactionType>,
) -> RouterResult<Option<UnifiedAuthenticationServiceFlow>> {
    let external_authentication_flow = get_payment_external_authentication_flow_during_confirm(
        state,
        key_store,
        business_profile,
        payment_data,
        connector_call_type,
        mandate_type,
    )
    .await?;
    Ok(match external_authentication_flow {
        Some(PaymentExternalAuthenticationFlow::PreAuthenticationFlow {
            acquirer_details,
            card,
            token,
        }) => Some(
            UnifiedAuthenticationServiceFlow::ExternalAuthenticationInitiate {
                acquirer_details,
                card,
                token,
            },
        ),
        Some(PaymentExternalAuthenticationFlow::PostAuthenticationFlow { authentication_id }) => {
            Some(
                UnifiedAuthenticationServiceFlow::ExternalAuthenticationPostAuthenticate {
                    authentication_id,
                },
            )
        }
        None => {
            if let Some(payment_method) = payment_data.payment_attempt.payment_method {
                if payment_method == storage_enums::PaymentMethod::Card
                    && business_profile.is_click_to_pay_enabled
                    && payment_data.service_details.is_some()
                {
                    Some(UnifiedAuthenticationServiceFlow::ClickToPayInitiate)
                } else {
                    None
                }
            } else {
                logger::info!(
                    payment_method=?payment_data.payment_attempt.payment_method,
                    click_to_pay_enabled=?business_profile.is_click_to_pay_enabled,
                    "skipping unified authentication service call since payment conditions are not satisfied"
                );
                None
            }
        }
    })
}

pub enum PaymentExternalAuthenticationFlow {
    PreAuthenticationFlow {
        acquirer_details: Option<authentication::types::AcquirerDetails>,
        card: Box<hyperswitch_domain_models::payment_method_data::Card>,
        token: String,
    },
    PostAuthenticationFlow {
        authentication_id: id_type::AuthenticationId,
    },
}

#[cfg(feature = "v1")]
pub async fn get_payment_external_authentication_flow_during_confirm<F: Clone>(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    business_profile: &domain::Profile,
    payment_data: &mut PaymentData<F>,
    connector_call_type: &api::ConnectorCallType,
    mandate_type: Option<api_models::payments::MandateTransactionType>,
) -> RouterResult<Option<PaymentExternalAuthenticationFlow>> {
    let authentication_id = payment_data.payment_attempt.authentication_id.clone();
    let is_authentication_type_3ds = payment_data.payment_attempt.authentication_type
        == Some(common_enums::AuthenticationType::ThreeDs);
    let separate_authentication_requested = payment_data
        .payment_intent
        .request_external_three_ds_authentication
        .unwrap_or(false);
    let separate_three_ds_authentication_attempted = payment_data
        .payment_attempt
        .external_three_ds_authentication_attempted
        .unwrap_or(false);
    let connector_supports_separate_authn =
        authentication::utils::get_connector_data_if_separate_authn_supported(connector_call_type);
    logger::info!("is_pre_authn_call {:?}", authentication_id.is_none());
    logger::info!(
        "separate_authentication_requested {:?}",
        separate_authentication_requested
    );
    logger::info!(
        "payment connector supports external authentication: {:?}",
        connector_supports_separate_authn.is_some()
    );
    let card = payment_data.payment_method_data.as_ref().and_then(|pmd| {
        if let domain::PaymentMethodData::Card(card) = pmd {
            Some(card.clone())
        } else {
            None
        }
    });
    Ok(if separate_three_ds_authentication_attempted {
        authentication_id.map(|authentication_id| {
            PaymentExternalAuthenticationFlow::PostAuthenticationFlow { authentication_id }
        })
    } else if separate_authentication_requested
        && is_authentication_type_3ds
        && mandate_type
            != Some(api_models::payments::MandateTransactionType::RecurringMandateTransaction)
    {
        if let Some((connector_data, card)) = connector_supports_separate_authn.zip(card) {
            let token = payment_data
                .token
                .clone()
                .get_required_value("token")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "payment_data.token should not be None while making pre authentication call",
                )?;
            let payment_connector_mca = get_merchant_connector_account(
                state,
                &business_profile.merchant_id,
                None,
                key_store,
                business_profile.get_id(),
                connector_data.connector_name.to_string().as_str(),
                connector_data.merchant_connector_id.as_ref(),
            )
            .await?;
            let acquirer_details = payment_connector_mca
                .get_metadata()
                .clone()
                .and_then(|metadata| {
                    metadata
                    .peek()
                    .clone()
                    .parse_value::<authentication::types::AcquirerDetails>("AcquirerDetails")
                    .change_context(errors::ApiErrorResponse::PreconditionFailed {
                        message:
                            "acquirer_bin and acquirer_merchant_id not found in Payment Connector's Metadata"
                                .to_string(),
                    })
                    .inspect_err(|err| {
                        logger::error!(
                            "Failed to parse acquirer details from Payment Connector's Metadata: {:?}",
                            err
                        );
                    })
                    .ok()
                });
            Some(PaymentExternalAuthenticationFlow::PreAuthenticationFlow {
                card: Box::new(card),
                token,
                acquirer_details,
            })
        } else {
            None
        }
    } else {
        None
    })
}

pub fn get_redis_key_for_extended_card_info(
    merchant_id: &id_type::MerchantId,
    payment_id: &id_type::PaymentId,
) -> String {
    format!(
        "{}_{}_extended_card_info",
        merchant_id.get_string_repr(),
        payment_id.get_string_repr()
    )
}

pub fn check_integrity_based_on_flow<T, Request>(
    request: &Request,
    payment_response_data: &Result<PaymentsResponseData, ErrorResponse>,
) -> Result<(), common_utils::errors::IntegrityCheckError>
where
    T: FlowIntegrity,
    Request: GetIntegrityObject<T> + CheckIntegrity<Request, T>,
{
    let connector_transaction_id = match payment_response_data {
        Ok(resp_data) => match resp_data {
            PaymentsResponseData::TransactionResponse {
                connector_response_reference_id,
                ..
            } => connector_response_reference_id,
            PaymentsResponseData::TransactionUnresolvedResponse {
                connector_response_reference_id,
                ..
            } => connector_response_reference_id,
            PaymentsResponseData::PreProcessingResponse {
                connector_response_reference_id,
                ..
            } => connector_response_reference_id,
            _ => &None,
        },
        Err(_) => &None,
    };
    request.check_integrity(request, connector_transaction_id.to_owned())
}

pub async fn config_skip_saving_wallet_at_connector(
    db: &dyn StorageInterface,
    merchant_id: &id_type::MerchantId,
) -> CustomResult<Option<Vec<storage_enums::PaymentMethodType>>, errors::ApiErrorResponse> {
    let config = db
        .find_config_by_key_unwrap_or(
            &merchant_id.get_skip_saving_wallet_at_connector_key(),
            Some("[]".to_string()),
        )
        .await;
    Ok(match config {
        Ok(conf) => Some(
            serde_json::from_str::<Vec<storage_enums::PaymentMethodType>>(&conf.config)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("skip_save_wallet_at_connector config parsing failed")?,
        ),
        Err(error) => {
            logger::error!(?error);
            None
        }
    })
}

#[cfg(feature = "v1")]
pub async fn override_setup_future_usage_to_on_session<F, D>(
    db: &dyn StorageInterface,
    payment_data: &mut D,
) -> CustomResult<(), errors::ApiErrorResponse>
where
    F: Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send,
{
    if payment_data.get_payment_intent().setup_future_usage == Some(enums::FutureUsage::OffSession)
    {
        let skip_saving_wallet_at_connector_optional = config_skip_saving_wallet_at_connector(
            db,
            &payment_data.get_payment_intent().merchant_id,
        )
        .await?;

        if let Some(skip_saving_wallet_at_connector) = skip_saving_wallet_at_connector_optional {
            if let Some(payment_method_type) =
                payment_data.get_payment_attempt().get_payment_method_type()
            {
                if skip_saving_wallet_at_connector.contains(&payment_method_type) {
                    logger::debug!("Override setup_future_usage from off_session to on_session based on the merchant's skip_saving_wallet_at_connector configuration to avoid creating a connector mandate.");
                    payment_data
                        .set_setup_future_usage_in_payment_intent(enums::FutureUsage::OnSession);
                }
            }
        };
    };
    Ok(())
}

pub async fn validate_routing_id_with_profile_id(
    db: &dyn StorageInterface,
    routing_id: &id_type::RoutingId,
    profile_id: &id_type::ProfileId,
) -> CustomResult<(), errors::ApiErrorResponse> {
    let _routing_id = db
        .find_routing_algorithm_metadata_by_algorithm_id_profile_id(routing_id, profile_id)
        .await
        .map_err(|err| {
            if err.current_context().is_db_not_found() {
                logger::warn!(
                    "Routing id not found for routing id - {:?} and profile id - {:?}",
                    routing_id,
                    profile_id
                );
                err.change_context(errors::ApiErrorResponse::InvalidDataFormat {
                    field_name: "routing_algorithm_id".to_string(),
                    expected_format: "A valid routing_id that belongs to the business_profile"
                        .to_string(),
                })
            } else {
                err.change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to validate routing id")
            }
        })?;
    Ok(())
}

#[cfg(feature = "v1")]
pub async fn validate_merchant_connector_ids_in_connector_mandate_details(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    connector_mandate_details: &api_models::payment_methods::CommonMandateReference,
    merchant_id: &id_type::MerchantId,
    card_network: Option<api_enums::CardNetwork>,
) -> CustomResult<(), errors::ApiErrorResponse> {
    let db = &*state.store;
    let merchant_connector_account_list = db
        .find_merchant_connector_account_by_merchant_id_and_disabled_list(
            merchant_id,
            true,
            key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::InternalServerError)?;

    let merchant_connector_account_details_hash_map: std::collections::HashMap<
        id_type::MerchantConnectorAccountId,
        domain::MerchantConnectorAccount,
    > = merchant_connector_account_list
        .iter()
        .map(|merchant_connector_account| {
            (
                merchant_connector_account.get_id(),
                merchant_connector_account.clone(),
            )
        })
        .collect();

    if let Some(payment_mandate_reference) = &connector_mandate_details.payments {
        let payments_map = payment_mandate_reference.0.clone();
        for (migrating_merchant_connector_id, migrating_connector_mandate_details) in payments_map {
            match (
                card_network.clone(),
                merchant_connector_account_details_hash_map.get(&migrating_merchant_connector_id),
            ) {
                (Some(enums::CardNetwork::Discover), Some(merchant_connector_account_details)) => {
                    if let ("cybersource", None) = (
                        merchant_connector_account_details.connector_name.as_str(),
                        migrating_connector_mandate_details
                            .original_payment_authorized_amount
                            .zip(
                                migrating_connector_mandate_details
                                    .original_payment_authorized_currency,
                            ),
                    ) {
                        Err(errors::ApiErrorResponse::MissingRequiredFields {
                            field_names: vec![
                                "original_payment_authorized_currency",
                                "original_payment_authorized_amount",
                            ],
                        })
                        .attach_printable(format!(
                            "Invalid connector_mandate_details provided for connector {migrating_merchant_connector_id:?}",

                        ))?
                    }
                }
                (_, Some(_)) => (),
                (_, None) => Err(errors::ApiErrorResponse::InvalidDataValue {
                    field_name: "merchant_connector_id",
                })
                .attach_printable_lazy(|| {
                    format!(
                        "{migrating_merchant_connector_id:?} invalid merchant connector id in connector_mandate_details",

                    )
                })?,
            }
        }
    } else {
        router_env::logger::error!("payment mandate reference not found");
    }
    Ok(())
}

pub fn validate_platform_request_for_marketplace(
    amount: api::Amount,
    split_payments: Option<common_types::payments::SplitPaymentsRequest>,
) -> Result<(), errors::ApiErrorResponse> {
    match split_payments {
        Some(common_types::payments::SplitPaymentsRequest::StripeSplitPayment(
            stripe_split_payment,
        )) => match amount {
            api::Amount::Zero => {
                if stripe_split_payment
                    .application_fees
                    .as_ref()
                    .map_or(MinorUnit::zero(), |amount| *amount)
                    != MinorUnit::zero()
                {
                    return Err(errors::ApiErrorResponse::InvalidDataValue {
                        field_name: "split_payments.stripe_split_payment.application_fees",
                    });
                }
            }
            api::Amount::Value(amount) => {
                if stripe_split_payment
                    .application_fees
                    .as_ref()
                    .map_or(MinorUnit::zero(), |amount| *amount)
                    > amount.into()
                {
                    return Err(errors::ApiErrorResponse::InvalidDataValue {
                        field_name: "split_payments.stripe_split_payment.application_fees",
                    });
                }
            }
        },
        Some(common_types::payments::SplitPaymentsRequest::AdyenSplitPayment(
            adyen_split_payment,
        )) => {
            let total_split_amount: i64 = adyen_split_payment
                .split_items
                .iter()
                .map(|split_item| {
                    split_item
                        .amount
                        .unwrap_or(MinorUnit::new(0))
                        .get_amount_as_i64()
                })
                .sum();

            match amount {
                api::Amount::Zero => {
                    if total_split_amount != 0 {
                        return Err(errors::ApiErrorResponse::InvalidDataValue {
                            field_name: "Sum of split amounts should be equal to the total amount",
                        });
                    }
                }
                api::Amount::Value(amount) => {
                    let i64_amount: i64 = amount.into();
                    if !adyen_split_payment.split_items.is_empty()
                        && i64_amount != total_split_amount
                    {
                        return Err(errors::ApiErrorResponse::PreconditionFailed {
                            message: "Sum of split amounts should be equal to the total amount"
                                .to_string(),
                        });
                    }
                }
            };
            adyen_split_payment
                .split_items
                .iter()
                .try_for_each(|split_item| {
                    match split_item.split_type {
                        common_enums::AdyenSplitType::BalanceAccount => {
                            if split_item.account.is_none() {
                                return Err(errors::ApiErrorResponse::MissingRequiredField {
                                    field_name:
                                        "split_payments.adyen_split_payment.split_items.account",
                                });
                            }
                        }
                        common_enums::AdyenSplitType::Commission
                        | enums::AdyenSplitType::Vat
                        | enums::AdyenSplitType::TopUp => {
                            if split_item.amount.is_none() {
                                return Err(errors::ApiErrorResponse::MissingRequiredField {
                                    field_name:
                                        "split_payments.adyen_split_payment.split_items.amount",
                                });
                            }
                            if let enums::AdyenSplitType::TopUp = split_item.split_type {
                                if split_item.account.is_none() {
                                    return Err(errors::ApiErrorResponse::MissingRequiredField {
                                        field_name:
                                            "split_payments.adyen_split_payment.split_items.account",
                                    });
                                }
                                if adyen_split_payment.store.is_some() {
                                    return Err(errors::ApiErrorResponse::PreconditionFailed {
                                        message: "Topup split payment is not available via Adyen Platform"
                                            .to_string(),
                                    });
                                }
                            }
                        }
                        enums::AdyenSplitType::AcquiringFees
                        | enums::AdyenSplitType::PaymentFee
                        | enums::AdyenSplitType::AdyenFees
                        | enums::AdyenSplitType::AdyenCommission
                        | enums::AdyenSplitType::AdyenMarkup
                        | enums::AdyenSplitType::Interchange
                        | enums::AdyenSplitType::SchemeFee => {}
                    };
                    Ok(())
                })?;
        }
        Some(common_types::payments::SplitPaymentsRequest::XenditSplitPayment(
            xendit_split_payment,
        )) => match xendit_split_payment {
            common_types::payments::XenditSplitRequest::MultipleSplits(
                xendit_multiple_split_payment,
            ) => {
                match amount {
                    api::Amount::Zero => {
                        let total_split_amount: i64 = xendit_multiple_split_payment
                            .routes
                            .iter()
                            .map(|route| {
                                route
                                    .flat_amount
                                    .unwrap_or(MinorUnit::new(0))
                                    .get_amount_as_i64()
                            })
                            .sum();

                        if total_split_amount != 0 {
                            return Err(errors::ApiErrorResponse::InvalidDataValue {
                                field_name:
                                    "Sum of split amounts should be equal to the total amount",
                            });
                        }
                    }
                    api::Amount::Value(amount) => {
                        let total_payment_amount: i64 = amount.into();
                        let total_split_amount: i64 = xendit_multiple_split_payment
                    .routes
                    .into_iter()
                    .map(|route| {
                        if route.flat_amount.is_none() && route.percent_amount.is_none() {
                            Err(errors::ApiErrorResponse::InvalidRequestData {
                                message: "Expected either split_payments.xendit_split_payment.routes.flat_amount or split_payments.xendit_split_payment.routes.percent_amount to be provided".to_string(),
                            })
                        } else if route.flat_amount.is_some() && route.percent_amount.is_some(){
                            Err(errors::ApiErrorResponse::InvalidRequestData {
                                message: "Expected either split_payments.xendit_split_payment.routes.flat_amount or split_payments.xendit_split_payment.routes.percent_amount, but not both".to_string(),
                            })
                        } else {
                            Ok(route
                                .flat_amount
                                .map(|amount| amount.get_amount_as_i64())
                                .or(route.percent_amount.map(|percentage| (percentage * total_payment_amount) / 100))
                                .unwrap_or(0))
                            }
                            })
                            .collect::<Result<Vec<i64>, _>>()?
                            .into_iter()
                            .sum();

                        if total_payment_amount < total_split_amount {
                            return Err(errors::ApiErrorResponse::PreconditionFailed {
                                message:
                                    "The sum of split amounts should not exceed the total amount"
                                        .to_string(),
                            });
                        }
                    }
                };
            }
            common_types::payments::XenditSplitRequest::SingleSplit(_) => (),
        },
        None => (),
    }
    Ok(())
}

pub async fn is_merchant_eligible_authentication_service(
    merchant_id: &id_type::MerchantId,
    state: &SessionState,
) -> RouterResult<bool> {
    let merchants_eligible_for_authentication_service = state
        .store
        .as_ref()
        .find_config_by_key_unwrap_or(
            consts::AUTHENTICATION_SERVICE_ELIGIBLE_CONFIG,
            Some("[]".to_string()),
        )
        .await;

    let auth_eligible_array: Vec<String> = match merchants_eligible_for_authentication_service {
        Ok(config) => serde_json::from_str(&config.config)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("unable to parse authentication service config")?,
        Err(err) => {
            logger::error!(
                "Error fetching authentication service enabled merchant config {:?}",
                err
            );
            Vec::new()
        }
    };

    Ok(auth_eligible_array.contains(&merchant_id.get_string_repr().to_owned()))
}

#[cfg(feature = "v1")]
pub async fn validate_allowed_payment_method_types_request(
    state: &SessionState,
    profile_id: &id_type::ProfileId,
    platform: &domain::Platform,
    allowed_payment_method_types: Option<Vec<common_enums::PaymentMethodType>>,
) -> CustomResult<(), errors::ApiErrorResponse> {
    if let Some(allowed_payment_method_types) = allowed_payment_method_types {
        let db = &*state.store;
        let all_connector_accounts = db
            .find_merchant_connector_account_by_merchant_id_and_disabled_list(
                platform.get_processor().get_account().get_id(),
                false,
                platform.get_processor().get_key_store(),
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to fetch merchant connector account for given merchant id")?;

        let filtered_connector_accounts = all_connector_accounts
            .filter_based_on_profile_and_connector_type(
                profile_id,
                ConnectorType::PaymentProcessor,
            );

        let supporting_payment_method_types: HashSet<_> = filtered_connector_accounts
            .iter()
            .flat_map(|connector_account| {
                connector_account
                    .payment_methods_enabled
                    .clone()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|payment_methods_enabled| {
                        payment_methods_enabled
                            .parse_value::<api_models::admin::PaymentMethodsEnabled>(
                                "payment_methods_enabled",
                            )
                    })
                    .filter_map(|parsed_payment_method_result| {
                        parsed_payment_method_result
                            .inspect_err(|err| {
                                logger::error!(
                                    "Unable to deserialize payment methods enabled: {:?}",
                                    err
                                );
                            })
                            .ok()
                    })
                    .flat_map(|parsed_payment_methods_enabled| {
                        parsed_payment_methods_enabled
                            .payment_method_types
                            .unwrap_or_default()
                            .into_iter()
                            .map(|payment_method_type| payment_method_type.payment_method_type)
                    })
            })
            .collect();

        let unsupported_payment_methods: Vec<_> = allowed_payment_method_types
            .iter()
            .filter(|allowed_pmt| !supporting_payment_method_types.contains(allowed_pmt))
            .collect();

        if !unsupported_payment_methods.is_empty() {
            metrics::PAYMENT_METHOD_TYPES_MISCONFIGURATION_METRIC.add(
                1,
                router_env::metric_attributes!((
                    "merchant_id",
                    platform.get_processor().get_account().get_id().clone()
                )),
            );
        }

        fp_utils::when(
            unsupported_payment_methods.len() == allowed_payment_method_types.len(),
            || {
                Err(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration)
                    .attach_printable(format!(
                        "None of the allowed payment method types {allowed_payment_method_types:?} are configured for this merchant connector account.",

                    ))
            },
        )?;
    }

    Ok(())
}

async fn get_payment_update_enabled_for_client_auth(
    merchant_id: &id_type::MerchantId,
    state: &SessionState,
) -> bool {
    let key = merchant_id.get_payment_update_enabled_for_client_auth_key();
    let db = &*state.store;
    let update_enabled = db.find_config_by_key(key.as_str()).await;

    match update_enabled {
        Ok(conf) => conf.config.to_lowercase() == "true",
        Err(error) => {
            logger::error!(?error);
            false
        }
    }
}

pub async fn allow_payment_update_enabled_for_client_auth(
    merchant_id: &id_type::MerchantId,
    state: &SessionState,
    auth_flow: services::AuthFlow,
) -> Result<(), error_stack::Report<errors::ApiErrorResponse>> {
    match auth_flow {
        services::AuthFlow::Client => {
            if get_payment_update_enabled_for_client_auth(merchant_id, state).await {
                Ok(())
            } else {
                Err(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Client auth for payment update is not enabled.")
            }
        }
        services::AuthFlow::Merchant => Ok(()),
    }
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn get_merchant_connector_account_v2(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    merchant_connector_id: Option<&id_type::MerchantConnectorAccountId>,
) -> RouterResult<domain::MerchantConnectorAccount> {
    let db = &*state.store;
    match merchant_connector_id {
        Some(merchant_connector_id) => db
            .find_merchant_connector_account_by_id(merchant_connector_id, key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                id: merchant_connector_id.get_string_repr().to_string(),
            }),
        None => Err(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "merchant_connector_id",
        })
        .attach_printable("merchant_connector_id is not provided"),
    }
}

pub fn is_stored_credential(
    recurring_details: &Option<RecurringDetails>,
    payment_token: &Option<String>,
    is_mandate: bool,
    is_stored_credential_prev: Option<bool>,
) -> Option<bool> {
    if is_stored_credential_prev == Some(true)
        || recurring_details.is_some()
        || payment_token.is_some()
        || is_mandate
    {
        Some(true)
    } else {
        is_stored_credential_prev
    }
}

#[cfg(feature = "v1")]
/// Helper function to get the connector label used for storing connector_customer_id
/// Returns None if the merchant_connector_account ID is not present
pub fn get_connector_label_for_customer<F, D>(
    merchant_connector_account: &MerchantConnectorAccountType,
    payment_data: &D,
) -> Option<String>
where
    D: OperationSessionGetters<F>,
{
    // Get connector name from payment attempt
    let connector_name = payment_data.get_payment_attempt().connector.as_ref()?;

    // Generate the connector_label using the same logic as when storing connector_customer_id
    let connector_label = core_utils::get_connector_label(
        payment_data.get_payment_intent().business_country,
        payment_data.get_payment_intent().business_label.as_ref(),
        payment_data
            .get_payment_attempt()
            .business_sub_label
            .as_ref(),
        connector_name,
    );

    // Use the same priority as in call_create_connector_customer_if_required
    // 1. Try merchant_connector_account ID first
    // 2. Try the dynamically generated connector_label
    // 3. Fallback to format!("{connector_name}_{profile_id}")
    if let Some(connector_label) = merchant_connector_account
        .get_mca_id()
        .map(|mca_id| mca_id.get_string_repr().to_string())
        .or(connector_label)
    {
        Some(connector_label)
    } else {
        // Fallback to connector_name_profileId
        payment_data
            .get_payment_intent()
            .profile_id
            .as_ref()
            .map(|profile_id| format!("{}_{}", connector_name, profile_id.get_string_repr()))
    }
}

#[cfg(feature = "v1")]
/// Helper function to populate connector_customer_id from database before calling UCS
/// This checks if a connector_customer_id already exists in the database and populates it into router_data
/// Returns true if connector_customer_id was found and populated from DB, false otherwise
async fn populate_connector_customer_from_db_before_ucs(
    state: &SessionState,
    connector_label: Option<&str>,
    payment_attempt: &PaymentAttempt,
    payment_id: &str,
    customer: &Option<domain::Customer>,
    merchant_connector_account: &MerchantConnectorAccountType,
) -> RouterResult<Option<String>> {
    match (connector_label, payment_attempt.connector.as_ref()) {
        (Some(label), Some(connector_name)) => {
            let connector = api::ConnectorData::get_connector_by_name(
                &state.conf.connectors,
                connector_name,
                api::GetToken::Connector,
                merchant_connector_account.get_mca_id(),
            )
            .inspect_err(|e| {
                router_env::logger::warn!(
                    payment_id = %payment_id,
                    connector_name = %connector_name,
                    error = ?e,
                    "Failed to get connector by name while populating connector_customer_id"
                )
            })?;

            let (should_call_connector, existing_connector_customer_id) =
                customers::should_call_connector_create_customer(
                    &connector,
                    customer,
                    payment_attempt,
                    label,
                );

            match existing_connector_customer_id {
                Some(connector_customer_id) => {
                    router_env::logger::info!(
                        connector_customer_id = %connector_customer_id,
                        payment_id = %payment_id,
                        connector_label = %label,
                        should_call_connector = %should_call_connector,
                        "Populating connector_customer from DB before UCS call"
                    );
                    Ok(Some(connector_customer_id.to_string()))
                }
                None => {
                    router_env::logger::info!(
                        payment_id = %payment_id,
                        connector_label = %label,
                        should_call_connector = %should_call_connector,
                        "No connector_customer_id found in DB for UCS call"
                    );
                    Ok(None)
                }
            }
        }
        (Some(label), None) => {
            router_env::logger::debug!(
                payment_id = %payment_id,
                connector_label = %label,
                "No connector name available, skipping connector_customer_id population"
            );
            Ok(None)
        }
        (None, Some(connector_name)) => {
            router_env::logger::debug!(
                payment_id = %payment_id,
                connector_name = %connector_name,
                "No connector label available, skipping connector_customer_id population"
            );
            Ok(None)
        }
        (None, None) => Ok(None),
    }
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
/// Helper function to save connector_customer_id after UCS call with proper error logging
/// Handles the logic of checking if ID was already in DB and logging appropriately
async fn save_connector_customer_id_after_ucs<F, Req>(
    state: &SessionState,
    router_data: &RouterData<F, Req, PaymentsResponseData>,
    connector_customer_id_was_populated_from_db: Option<String>,
    connector_label: &Option<String>,
    customer: &Option<domain::Customer>,
    platform: &domain::Platform,
    payment_id: String,
) -> RouterResult<()> {
    match connector_customer_id_was_populated_from_db {
        None => {
            match (
                router_data.connector_customer.as_ref(),
                connector_label.as_ref(),
            ) {
                (Some(connector_customer_id), Some(connector_label_str)) => {
                    save_new_connector_customer_id_from_ucs(
                        state,
                        connector_customer_id,
                        connector_label_str,
                        customer,
                        platform,
                        payment_id.clone(),
                    )
                    .await
                    .inspect_err(|e| {
                        router_env::logger::warn!(
                            payment_id = %payment_id,
                            error = ?e,
                            "Failed to save connector_customer_id from UCS"
                        )
                    })
                }
                (None, Some(label)) => {
                    router_env::logger::debug!(
                        payment_id = %payment_id,
                        connector_label = %label,
                        "Missing connector_customer_id, skipping save"
                    );
                    Ok(())
                }
                (Some(id), None) => {
                    router_env::logger::debug!(
                        payment_id = %payment_id,
                        connector_customer_id = %id,
                        "Missing connector_label, skipping save"
                    );
                    Ok(())
                }
                (None, None) => {
                    router_env::logger::debug!(
                        payment_id = %payment_id,
                        "Missing both connector_customer_id and connector_label, skipping save"
                    );
                    Ok(())
                }
            }
        }
        Some(existing_id) => {
            router_env::logger::debug!(
                payment_id = %payment_id,
                existing_connector_customer_id = %existing_id,
                "Connector customer ID already exists in DB, skipping save"
            );
            Ok(())
        }
    }
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
/// Helper function to save a new connector_customer_id from UCS to database
/// This only saves if the ID doesn't already exist in the database
async fn save_new_connector_customer_id_from_ucs(
    state: &SessionState,
    connector_customer_id: &str,
    connector_label: &str,
    customer: &Option<domain::Customer>,
    platform: &domain::Platform,
    payment_id: String,
) -> RouterResult<()> {
    // Process only if customer exists
    match customer.as_ref() {
        Some(customer_data) => {
            router_env::logger::info!(
            "Saving new connector_customer_id from UCS to DB for payment_id={}, connector_label={}",
            payment_id,
            connector_label
        );

            // Create and save customer update
            match customers::update_connector_customer_in_customers(
                connector_label,
                Some(customer_data),
                Some(connector_customer_id.to_string()),
            )
            .await
            {
                Some(update) => {
                    let db = &*state.store;
                    let _ = db
                        .update_customer_by_customer_id_merchant_id(
                            customer_data.customer_id.clone(),
                            customer_data.merchant_id.clone(),
                            customer_data.clone(),
                            update,
                            platform.get_processor().get_key_store(),
                            platform.get_processor().get_account().storage_scheme,
                        )
                        .await
                        .inspect_err(|e| {
                            router_env::logger::warn!(
                                "Failed to save connector_customer_id to DB: {:?}",
                                e
                            )
                        });
                    Ok(())
                }
                None => {
                    router_env::logger::error!(
                        payment_id = %payment_id,
                        connector_label = %connector_label,
                        "No update generated for saving connector_customer_id"
                    );
                    Ok(())
                }
            }
        }
        None => {
            router_env::logger::debug!(
            "No customer data available, skipping connector_customer_id save for payment_id={}, connector_label={}",
            payment_id,
            connector_label
        );
            Ok(())
        }
    }
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
// Helper function to process through UCS gateway
pub async fn process_through_ucs<'a, F, RouterDReq, ApiRequest, D>(
    state: &'a SessionState,
    req_state: routes::app::ReqState,
    platform: &'a domain::Platform,
    operation: &'a BoxedOperation<'a, F, ApiRequest, D>,
    payment_data: &'a mut D,
    customer: &Option<domain::Customer>,
    call_connector_action: CallConnectorAction,
    validate_result: &'a OperationsValidateResult,
    schedule_time: Option<time::PrimitiveDateTime>,
    header_payload: domain_payments::HeaderPayload,
    frm_suggestion: Option<enums::FrmSuggestion>,
    business_profile: &'a domain::Profile,
    merchant_connector_account: MerchantConnectorAccountType,
    connector_data: &api::ConnectorData,
    router_data: RouterData<F, RouterDReq, PaymentsResponseData>,
) -> RouterResult<(
    RouterData<F, RouterDReq, PaymentsResponseData>,
    MerchantConnectorAccountType,
)>
where
    F: Send + Clone + Sync + 'static,
    RouterDReq: Send + Sync + Clone + 'static + Serialize,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
    D: ConstructFlowSpecificData<F, RouterDReq, PaymentsResponseData>,
    RouterData<F, RouterDReq, PaymentsResponseData>:
        Feature<F, RouterDReq> + Send + Clone + Serialize,
    dyn api::Connector: services::api::ConnectorIntegration<F, RouterDReq, PaymentsResponseData>,
{
    router_env::logger::info!(
        "Processing payment through UCS gateway system - payment_id={}, attempt_id={}",
        payment_data
            .get_payment_intent()
            .payment_id
            .get_string_repr(),
        payment_data.get_payment_attempt().attempt_id
    );

    // Add task to process tracker if needed
    if should_add_task_to_process_tracker(payment_data) {
        operation
            .to_domain()?
            .add_task_to_process_tracker(
                state,
                payment_data.get_payment_attempt(),
                validate_result.requeue,
                schedule_time,
            )
            .await
            .map_err(|error| router_env::logger::error!(process_tracker_error=?error))
            .ok();
    }

    // Update feature metadata to track UCS usage for stickiness
    update_gateway_system_in_feature_metadata(
        payment_data,
        GatewaySystem::UnifiedConnectorService,
    )?;

    let lineage_ids = grpc_client::LineageIds::new(
        business_profile.merchant_id.clone(),
        business_profile.get_id().clone(),
    );
    // Extract merchant_order_reference_id from payment data for UCS audit trail
    let merchant_order_reference_id = payment_data
        .get_payment_intent()
        .merchant_order_reference_id
        .clone();
    let (mut router_data, should_continue) = router_data
        .call_preprocessing_through_unified_connector_service(
            state,
            &header_payload,
            &lineage_ids,
            merchant_connector_account.clone(),
            platform,
            connector_data,
            ExecutionMode::Primary, // UCS is called in primary mode
            merchant_order_reference_id.clone(),
        )
        .await?;

    // Update trackers
    (_, *payment_data) = operation
        .to_update_tracker()?
        .update_trackers(
            state,
            req_state,
            payment_data.clone(),
            customer.clone(),
            platform.get_processor().get_account().storage_scheme,
            None,
            platform.get_processor().get_key_store(),
            frm_suggestion,
            header_payload.clone(),
        )
        .await?;
    let creds_identifier = payment_data.get_creds_identifier().map(str::to_owned);
    // Calculate connector label once for reuse
    let connector_label =
        get_connector_label_for_customer(&merchant_connector_account, payment_data);

    // Populate connector_customer_id from database before calling UCS
    // Track whether ID was found in DB to avoid redundant save later
    let connector_customer_id_was_populated_from_db =
        populate_connector_customer_from_db_before_ucs(
            state,
            connector_label.as_deref(),
            payment_data.get_payment_attempt(),
            payment_data
                .get_payment_intent()
                .payment_id
                .get_string_repr(),
            customer,
            &merchant_connector_account,
        )
        .await?;

    connector_customer_id_was_populated_from_db
        .as_ref()
        .map(|id| {
            router_data.connector_customer = Some(id.clone());
        });

    // Based on the preprocessing response, decide whether to continue with UCS call
    if should_continue {
        router_data
            .call_unified_connector_service(
                state,
                &header_payload,
                lineage_ids,
                merchant_connector_account.clone(),
                platform,
                connector_data,
                ExecutionMode::Primary, // UCS is called in primary mode
                merchant_order_reference_id,
                call_connector_action,
                creds_identifier,
            )
            .await?;
    }

    // Save connector_customer_id from UCS to database if needed
    let payment_id = payment_data
        .get_payment_intent()
        .payment_id
        .get_string_repr()
        .to_string();
    let _ = save_connector_customer_id_after_ucs(
        state,
        &router_data,
        connector_customer_id_was_populated_from_db,
        &connector_label,
        customer,
        platform,
        payment_id.clone(),
    )
    .await
    .inspect_err(|e| {
        router_env::logger::warn!(
            "Failed to save connector_customer_id after UCS call - payment_id={}, error={:?}",
            payment_id,
            e
        )
    }); // Don't fail payment if save fails

    Ok((router_data, merchant_connector_account))
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
// Helper function to process through Direct gateway
pub async fn process_through_direct<'a, F, RouterDReq, ApiRequest, D>(
    state: &'a SessionState,
    req_state: routes::app::ReqState,
    platform: &'a domain::Platform,
    connector: api::ConnectorData,
    operation: &'a BoxedOperation<'a, F, ApiRequest, D>,
    payment_data: &'a mut D,
    customer: &Option<domain::Customer>,
    call_connector_action: CallConnectorAction,
    validate_result: &'a OperationsValidateResult,
    schedule_time: Option<time::PrimitiveDateTime>,
    header_payload: domain_payments::HeaderPayload,
    frm_suggestion: Option<enums::FrmSuggestion>,
    business_profile: &'a domain::Profile,
    is_retry_payment: bool,
    all_keys_required: Option<bool>,
    merchant_connector_account: MerchantConnectorAccountType,
    router_data: RouterData<F, RouterDReq, PaymentsResponseData>,
    tokenization_action: TokenizationAction,
) -> RouterResult<(
    RouterData<F, RouterDReq, PaymentsResponseData>,
    MerchantConnectorAccountType,
)>
where
    F: Send + Clone + Sync + 'static,
    RouterDReq: Send + Sync + Clone + 'static + Serialize,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
    D: ConstructFlowSpecificData<F, RouterDReq, PaymentsResponseData>,
    RouterData<F, RouterDReq, PaymentsResponseData>:
        Feature<F, RouterDReq> + Send + Clone + Serialize,
    dyn api::Connector: services::api::ConnectorIntegration<F, RouterDReq, PaymentsResponseData>,
{
    router_env::logger::info!(
        "Processing payment through Direct gateway system - payment_id={}, attempt_id={}",
        payment_data
            .get_payment_intent()
            .payment_id
            .get_string_repr(),
        payment_data.get_payment_attempt().attempt_id
    );

    // Update feature metadata to track Direct routing usage for stickiness
    update_gateway_system_in_feature_metadata(payment_data, GatewaySystem::Direct)?;
    let lineage_ids = grpc_client::LineageIds::new(
        business_profile.merchant_id.clone(),
        business_profile.get_id().clone(),
    );
    let gateway_context = gateway_context::RouterGatewayContext {
        creds_identifier: None,
        platform: platform.clone(),
        header_payload: header_payload.clone(),
        lineage_ids,
        merchant_connector_account: merchant_connector_account.clone(),
        execution_path: ExecutionPath::Direct,
        execution_mode: ExecutionMode::NotApplicable,
    };

    call_connector_service(
        state,
        req_state,
        platform,
        connector,
        operation,
        payment_data,
        customer,
        call_connector_action,
        validate_result,
        schedule_time,
        header_payload,
        frm_suggestion,
        business_profile,
        is_retry_payment,
        all_keys_required,
        merchant_connector_account,
        router_data,
        tokenization_action,
        gateway_context,
    )
    .await
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
// Helper function to process through Direct with Shadow UCS
pub async fn process_through_direct_with_shadow_unified_connector_service<
    'a,
    F,
    RouterDReq,
    ApiRequest,
    D,
>(
    state: &'a SessionState,
    req_state: routes::app::ReqState,
    platform: &'a domain::Platform,
    connector: api::ConnectorData,
    operation: &'a BoxedOperation<'a, F, ApiRequest, D>,
    payment_data: &'a mut D,
    customer: &Option<domain::Customer>,
    call_connector_action: CallConnectorAction,
    shadow_ucs_call_connector_action: Option<CallConnectorAction>,
    validate_result: &'a OperationsValidateResult,
    schedule_time: Option<time::PrimitiveDateTime>,
    header_payload: domain_payments::HeaderPayload,
    frm_suggestion: Option<enums::FrmSuggestion>,
    business_profile: &'a domain::Profile,
    is_retry_payment: bool,
    all_keys_required: Option<bool>,
    merchant_connector_account: MerchantConnectorAccountType,
    router_data: RouterData<F, RouterDReq, PaymentsResponseData>,
    tokenization_action: TokenizationAction,
) -> RouterResult<(
    RouterData<F, RouterDReq, PaymentsResponseData>,
    MerchantConnectorAccountType,
)>
where
    F: Send + Clone + Sync + 'static,
    RouterDReq: Send + Sync + Clone + 'static + Serialize,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
    D: ConstructFlowSpecificData<F, RouterDReq, PaymentsResponseData>,
    RouterData<F, RouterDReq, PaymentsResponseData>:
        Feature<F, RouterDReq> + Send + Clone + Serialize,
    dyn api::Connector: services::api::ConnectorIntegration<F, RouterDReq, PaymentsResponseData>,
{
    router_env::logger::info!(
        "Processing payment through Direct gateway system with UCS in shadow mode - payment_id={}, attempt_id={}",
        payment_data.get_payment_intent().payment_id.get_string_repr(),
        payment_data.get_payment_attempt().attempt_id
    );

    // Extract merchant_order_reference_id from payment data for UCS audit trail
    let merchant_order_reference_id = payment_data
        .get_payment_intent()
        .merchant_order_reference_id
        .clone();

    // Calculate connector_label before cloning data for shadow UCS
    let unified_connector_service_connector_label =
        get_connector_label_for_customer(&merchant_connector_account, payment_data);

    let creds_identifier = payment_data.get_creds_identifier().map(str::to_owned);

    // Clone data needed for shadow UCS call
    let unified_connector_service_router_data = router_data.clone();
    let unified_connector_service_merchant_connector_account = merchant_connector_account.clone();
    let unified_connector_service_platform = platform.clone();
    let unified_connector_service_header_payload = header_payload.clone();
    let unified_connector_service_state = state.clone();
    let unified_connector_service_merchant_order_reference_id = merchant_order_reference_id;
    let unified_connector_service_creds_identifier = creds_identifier.clone();
    let unified_connector_service_customer = customer.clone();
    let unified_connector_service_payment_attempt_data = payment_data.get_payment_attempt().clone();
    let unified_connector_service_connector_payment_id = payment_data
        .get_payment_intent()
        .payment_id
        .get_string_repr()
        .to_string();

    let lineage_ids = grpc_client::LineageIds::new(
        business_profile.merchant_id.clone(),
        business_profile.get_id().clone(),
    );

    // Update feature metadata to track Direct routing usage for stickiness
    update_gateway_system_in_feature_metadata(payment_data, GatewaySystem::Direct)?;
    let execution_mode = ExecutionMode::NotApplicable;

    let gateway_context = gateway_context::RouterGatewayContext {
        creds_identifier: payment_data.get_creds_identifier().map(|id| id.to_string()),
        platform: platform.clone(),
        header_payload: header_payload.clone(),
        lineage_ids: lineage_ids.clone(),
        merchant_connector_account: merchant_connector_account.clone(),
        execution_path: ExecutionPath::Direct,
        execution_mode,
    };

    // Call Direct connector service
    let result = call_connector_service(
        state,
        req_state,
        platform,
        connector.clone(),
        operation,
        payment_data,
        customer,
        call_connector_action.clone(),
        validate_result,
        schedule_time,
        header_payload,
        frm_suggestion,
        business_profile,
        is_retry_payment,
        all_keys_required,
        merchant_connector_account,
        router_data,
        tokenization_action,
        gateway_context,
    )
    .await?;

    // Spawn shadow UCS call in background
    let direct_router_data = result.0.clone();
    tokio::spawn(async move {
        execute_shadow_unified_connector_service_call(
            unified_connector_service_state,
            unified_connector_service_router_data,
            direct_router_data,
            unified_connector_service_header_payload,
            lineage_ids,
            unified_connector_service_merchant_connector_account,
            &connector,
            unified_connector_service_platform,
            unified_connector_service_merchant_order_reference_id,
            call_connector_action,
            shadow_ucs_call_connector_action,
            unified_connector_service_creds_identifier,
            unified_connector_service_customer,
            unified_connector_service_payment_attempt_data,
            unified_connector_service_connector_label,
            unified_connector_service_connector_payment_id,
        )
        .await
        .map_err(|e| {
            router_env::logger::debug!(
                "Shadow UCS call in Direct with shadow UCS processing failed: {:?}",
                e
            )
        })
        .ok()
    });

    Ok(result)
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
// Helper function to execute shadow UCS call
pub async fn execute_shadow_unified_connector_service_call<F, RouterDReq>(
    state: SessionState,
    mut unified_connector_service_router_data: RouterData<F, RouterDReq, PaymentsResponseData>,
    direct_router_data: RouterData<F, RouterDReq, PaymentsResponseData>,
    header_payload: domain_payments::HeaderPayload,
    lineage_ids: grpc_client::LineageIds,
    merchant_connector_account: MerchantConnectorAccountType,
    connector_data: &api::ConnectorData,
    platform: domain::Platform,
    merchant_order_reference_id: Option<String>,
    call_connector_action: CallConnectorAction,
    shadow_ucs_call_connector_action: Option<CallConnectorAction>,
    creds_identifier: Option<String>,
    customer: Option<domain::Customer>,
    payment_attempt_data: PaymentAttempt,
    unified_connector_service_connector_label: Option<String>,
    unified_connector_service_payment_id: String,
) -> RouterResult<()>
where
    F: Send + Clone + Sync + 'static,
    RouterDReq: Send + Sync + Clone + 'static + Serialize,
    RouterData<F, RouterDReq, PaymentsResponseData>:
        Feature<F, RouterDReq> + Send + Clone + Serialize,
    dyn api::Connector: services::api::ConnectorIntegration<F, RouterDReq, PaymentsResponseData>,
{
    let connector_customer_id_was_populated_from_db =
        populate_connector_customer_from_db_before_ucs(
            &state,
            unified_connector_service_connector_label.as_deref(),
            &payment_attempt_data,
            &unified_connector_service_payment_id,
            &customer,
            &merchant_connector_account,
        )
        .await?;

    connector_customer_id_was_populated_from_db.map(|id| {
        unified_connector_service_router_data.connector_customer = Some(id);
    });
    // Call UCS in shadow mode
    let _unified_connector_service_result = unified_connector_service_router_data
        .call_unified_connector_service(
            &state,
            &header_payload,
            lineage_ids,
            merchant_connector_account,
            &platform,
            connector_data,
            ExecutionMode::Shadow, // Shadow mode for UCS
            merchant_order_reference_id,
            shadow_ucs_call_connector_action.unwrap_or(call_connector_action),
            creds_identifier,
        )
        .await
        .map_err(|e| router_env::logger::debug!("Shadow UCS call failed: {:?}", e));

    // Compare results
    match unified_connector_service::serialize_router_data_and_send_to_comparison_service(
        &state,
        direct_router_data,
        unified_connector_service_router_data,
    )
    .await
    {
        Ok(_) => {
            router_env::logger::debug!("Shadow UCS comparison completed successfully");
            Ok(())
        }
        Err(e) => {
            router_env::logger::debug!("Shadow UCS comparison failed: {:?}", e);
            Ok(())
        }
    }
}
