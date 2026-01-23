use common_utils::{
    errors::CustomResult,
    ext_traits::{BytesExt, Encode},
    id_type, pii,
    request::RequestContent,
    types::MinorUnit,
};

use api_models::payment_methods as pm_api_models;
use api_models::payments;
use api_models::payments::PaymentMethodData;
use diesel_models::schema_v2::payment_attempt::connector_token_details;
use error_stack::{report, ResultExt};
use masking::{ExposeInterface, Maskable, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    core::{errors, payment_methods},
    headers, logger, services, settings,
    types::domain,
};

#[derive(Debug, Clone, Serialize)]
pub struct ModularPMUpdateRequest {
    pub payment_method_data: Option<PaymentMethodUpdateData>,
    pub connector_token_details: Option<ConnectorTokenDetails>,
}

#[derive(Debug, Clone, Serialize)]
pub enum PaymentMethodUpdateData {
    Card(CardDetailUpdate),
}

#[derive(Debug, Clone, Serialize)]
pub struct CardDetailUpdate {
    pub card_holder_name: Option<Secret<String>>,
    pub nick_name: Option<Secret<String>>,
    pub card_cvc: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorTokenDetails {
    pub connector_id: id_type::MerchantConnectorAccountId,
    pub token_type: common_enums::TokenizationType,
    pub status: common_enums::ConnectorTokenStatus, 
    pub connector_token_request_reference_id: Option<String>,
    pub original_payment_authorized_amount: Option<MinorUnit>,
    pub original_payment_authorized_currency: Option<common_enums::Currency>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub token: Secret<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct PaymentMethodResponse {
    //payment method id
    pub id: String,
    pub merchant_id: id_type::MerchantId,
    pub customer_id: Option<CustomerId>,
    pub payment_method_type: Option<common_enums::PaymentMethod>,
    pub payment_method_subtype: Option<common_enums::PaymentMethodType>,
    pub recurring_enabled: Option<bool>,
    pub created: Option<time::PrimitiveDateTime>,
    pub last_used_at: Option<time::PrimitiveDateTime>,
    pub payment_method_data: Option<PaymentMethodResponseData>,
    pub connector_tokens: Option<Vec<ConnectorTokenDetails>>,
    pub network_token: Option<pm_api_models::NetworkTokenResponse>,
    pub storage_type: Option<common_enums::StorageType>,
    pub card_cvc_token_storage: Option<CardCVCTokenStorageDetails>,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CardCVCTokenStorageDetails {
    pub is_stored: bool,
    pub expires_at: Option<time::PrimitiveDateTime>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub enum PaymentMethodResponseData {
    Card(pm_api_models::CardDetailFromLocker),
}

impl TryFrom<domain::PaymentMethod> for ModularPMUpdateRequest {
    type Error = errors::ApiErrorResponse;

    fn try_from(pm_domain_data: domain::PaymentMethod) -> Result<Self, Self::Error> {
        // Extract payment method data
        // let payment_method_data = pm_domain_data
        //     .payment_method_data
        //     .map(|data| {
        //         data.into_inner()
        //             .expose()
        //             .and_then(|v| {
        //                 serde_json::from_value::<pm_api_models::PaymentMethodsData>(v).ok()
        //             })
        //             .and_then(|pmd| match pmd {
        //                 pm_api_models::PaymentMethodsData::Card(card) => Some(card),
        //                 _ => None,
        //             })
        //     })
        //     .map(|card| {
        //         PaymentMethodUpdateData::Card(CardDetailUpdate {
        //             card_holder_name: card.card_holder_name.clone(),
        //             nick_name: card.nick_name.clone(),
        //             card_cvc: card.card_cvc.clone(),
        //         })
        //     });

        // Extract connector token details from connector_mandate_details
        let connector_token_detail =
            pm_domain_data
                .connector_mandate_details
                .and_then(|mandate_details| {
                    mandate_details.as_object().and_then(|obj| {
                        obj.iter().next().and_then(|(connector_id_str, value)| {
                            let connector_id =
                                id_type::MerchantConnectorAccountId::wrap(connector_id_str.clone())
                                    .ok()?;

                            let mandate_value: serde_json::Value =
                                serde_json::to_value(value).ok()?;
                            let mandate_obj = mandate_value.as_object()?;

                            Some(ConnectorTokenDetails {
                                connector_id,
                                token_type: common_enums::TokenizationType::MultiUse,
                                status: mandate_obj
                                    .get("connector_mandate_status")
                                    .and_then(|v| {
                                        serde_json::from_value::<
                                                common_enums::ConnectorTokenStatus,
                                            >(v.clone())
                                            .ok()
                                    })
                                    .unwrap_or(common_enums::ConnectorTokenStatus::Active),
                                connector_token_request_reference_id: mandate_obj
                                    .get("connector_mandate_request_reference_id")
                                    .and_then(|v| v.as_str().map(String::from)),
                                original_payment_authorized_amount: mandate_obj
                                    .get("original_payment_authorized_amount")
                                    .and_then(|v| serde_json::from_value(v.clone()).ok()),
                                original_payment_authorized_currency: mandate_obj
                                    .get("original_payment_authorized_currency")
                                    .and_then(|v| serde_json::from_value(v.clone()).ok()),
                                metadata: mandate_obj
                                    .get("mandate_metadata")
                                    .and_then(|v| serde_json::from_value(v.clone()).ok()),
                                token: Secret::new(
                                    mandate_obj
                                        .get("connector_mandate_id")
                                        .and_then(|v| v.as_str().map(String::from))
                                        .unwrap_or_default(),
                                ),
                            })
                        })
                    })
                });

        Ok(ModularPMUpdateRequest {
            payment_method_data: None,
            connector_token_details: connector_token_detail,
        })
    }
}

pub async fn pm_modular_update(
    state: &crate::routes::SessionState,
    pm_domain_data: domain::PaymentMethod,
) -> CustomResult<PaymentMethodResponse, errors::ApiErrorResponse> {
    let update_request = ModularPMUpdateRequest::try_from(pm_domain_data.clone())?;
    
    pm_update_modular_api_call(
        state,
        update_request,
    )
    .await
}


pub async fn pm_update_modular_api_call(
    state: &crate::routes::SessionState,
    update_request: ModularPMUpdateRequest,
) -> CustomResult<PaymentMethodResponse, errors::ApiErrorResponse> {

    let modular_service = state
        .conf
        .payment_method_modular_service
        .as_ref()
        .ok_or_else(|| {
            report!(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Payment method modular service not configured")
        })?;

    let payment_method_id = pm_domain_data.get_id().to_string();
    let update_url = format!(
        "{}/payment_methods/{}",
        modular_service.base_url.as_str().trim_end_matches('/'),
        payment_method_id
    );

    logger::debug!(
        payment_method_id = %payment_method_id,
        "Sending payment method update request to modular service: {}",
        update_url
    );

    let mut request = services::Request::new(services::Method::Post, &update_url);
    request.add_header(headers::CONTENT_TYPE, "application/json".into());
    request.add_default_headers();
    request.set_body(RequestContent::Json(Box::new(update_request)));

    let response = services::call_connector_api(state, request, "pm_update_modular")
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to call payment method modular service")?;

    let response_data = response.map_err(|err_res| {
        logger::error!(
            error = ?err_res,
            "Payment method modular service returned error"
        );
        report!(errors::ApiErrorResponse::InternalServerError).attach_printable(format!(
            "Payment method modular service error: {:?}",
            err_res.response
        ))
    })?;

    let payment_method_response: PaymentMethodResponse = response_data
        .response
        .parse_struct("PaymentMethodResponse")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to deserialize payment method response")?;

    logger::info!(
        payment_method_id = %payment_method_id,
        "Successfully updated payment method via modular service"
    );

    Ok(payment_method_response)
}

#[derive(Debug, Clone, Serialize)]
pub struct ModularPMCreateRequest {
    pub payment_method_type: common_enums::PaymentMethod,
    pub payment_method_subtype: common_enums::PaymentMethodType,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub customer_id: id_types::CustomerId, // Payment method data will be saved when customer acceptance is given, hence customer id will always be present
    pub payment_method_data: PaymentMethodCreateData,
    pub billing: Option<payments::Address>,
    pub psp_tokenization: Option<common_types::payment_methods::PspTokenization>,
    pub network_tokenization: Option<common_types::payment_methods::NetworkTokenization>,
    pub storage_type: Option<common_enums::StorageType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentMethodCreateData {
    Card(pm_api_models::CardDetail),
}

impl TryFrom<api_models::payments::PaymentMethodData> for PaymentMethodCreateData {
    type Error = errors::ApiErrorResponse;

    fn try_from(value: api_models::payments::PaymentMethodData) -> Result<Self, Self::Error> {
        match value {
            api_models::payments::PaymentMethodData::Card(card) => {
                let card_detail = pm_api_models::CardDetail {
                    card_number: card.card_number,
                    card_exp_month: card.card_exp_month,
                    card_exp_year: card.card_exp_year,
                    card_holder_name: card.card_holder_name,
                    nick_name: card.nick_name,
                    card_issuing_country: card.card_issuing_country,
                    card_network: None,
                    card_issuer: card.card_issuer,
                    card_type: None,
                    card_cvc: None,
                    card_issuing_country_code: card.card_issuing_country_code,
                };
                Ok(PaymentMethodCreateData::Card(card_detail))
            }
            _ => Err(errors::ApiErrorResponse::NotSupported {
                message: "Unsupported payment method type for modular PM creation".to_string(),
            }),
        }
    }
}

pub async fn pm_create_modular(
    state: &crate::routes::SessionState,
    payment_method: common_enums::PaymentMethod,
    payment_method_type: common_enums::PaymentMethodType,
    payment_method_data: api_models::payments::PaymentMethodData,
    customer_id: common_utils::id_type::CustomerId,
    billing_address: payments::Address,
) -> CustomResult<PaymentMethodResponse, errors::ApiErrorResponse> {

    let payment_method_data = PaymentMethodCreateData::try_from(payment_method_data)?;

    let pm_create_request = ModularPMCreateRequest {
        payment_method_type: payment_method,
        payment_method_subtype: payment_method_type,
        metadata: None,
        customer_id,
        payment_method_data,
        billing: Some(billing_address),
        psp_tokenization: None,
        network_tokenization: None,
        storage_type: Some(common_enums::StorageType::Persistent),
    };
    
    pm_create_modular_api_call(
        state,
        pm_create_request,
    )
    .await
}



pub async fn pm_create_modular_api_call(
    state: &crate::routes::SessionState,
    pm_create_request: ModularPMCreateRequest,
) -> CustomResult<PaymentMethodResponse, errors::ApiErrorResponse> {
    let modular_service = state
        .conf
        .payment_method_modular_service
        .as_ref()
        .ok_or_else(|| {
            report!(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Payment method modular service not configured")
        })?;

    let create_url = format!(
        "{}/payment_methods",
        modular_service.base_url.as_str().trim_end_matches('/')
    );

    logger::debug!(
        "Sending payment method create request to modular service: {}",
        create_url
    );

    let mut request = services::Request::new(services::Method::Post, &create_url);
    request.add_header(headers::CONTENT_TYPE, "application/json".into());
    request.add_default_headers();
    request.set_body(RequestContent::Json(Box::new(pm_create_request)));

    let response = services::call_connector_api(state, request, "pm_create_modular")
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to call payment method modular service")?;

    let response_data = response.map_err(|err_res| {
        logger::error!(
            error = ?err_res,
            "Payment method modular service returned error"
        );
        report!(errors::ApiErrorResponse::InternalServerError).attach_printable(format!(
            "Payment method modular service error: {:?}",
            err_res.response
        ))
    })?;

    let payment_method_response: PaymentMethodResponse = response_data
        .response
        .parse_struct("PaymentMethodResponse")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to deserialize payment method response")?;

    logger::info!("Successfully created payment method via modular service");

    Ok(payment_method_response)
}
