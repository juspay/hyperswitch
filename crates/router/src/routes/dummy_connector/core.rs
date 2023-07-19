use app::AppState;
use common_utils::{ext_traits::AsyncExt, generate_id_with_default_len};
use error_stack::{report, IntoReport, ResultExt};
use masking::PeekInterface;
use rand::Rng;
use tokio::time as tokio;

use super::{errors, types, utils};
use crate::{
    routes::{app, dummy_connector::consts},
    services::api,
    utils::OptionExt,
};

pub async fn tokio_mock_sleep(delay: u64, tolerance: u64) {
    let mut rng = rand::thread_rng();
    let effective_delay = rng.gen_range((delay - tolerance)..(delay + tolerance));
    tokio::sleep(tokio::Duration::from_millis(effective_delay)).await
}

pub async fn payment(
    state: &AppState,
    req: types::DummyConnectorPaymentRequest,
) -> types::DummyConnectorResponse<types::DummyConnectorPaymentResponse> {
    tokio_mock_sleep(
        state.conf.dummy_connector.payment_duration,
        state.conf.dummy_connector.payment_tolerance,
    )
    .await;

    let timestamp = common_utils::date_time::now();
    let payment_id = generate_id_with_default_len(consts::PAYMENT_ID_PREFIX);
    let attempt_id = generate_id_with_default_len(consts::ATTEMPT_ID_PREFIX);
    let redis_conn = state.store.get_redis_conn();
    match req.payment_method_data {
        types::DummyConnectorPaymentMethodData::Card(card) => {
            let card_number = card.number.peek();

            match utils::get_flow_from_card_number(card_number)? {
                types::DummyConnectorFlow::NoThreeDS(status, error) => {
                    if let Some(error) = error {
                        Err(error).into_report()?;
                    }
                    let payment_data = types::DummyConnectorPaymentData::new(
                        payment_id.clone(),
                        status,
                        req.amount,
                        req.amount,
                        req.currency,
                        timestamp.clone(),
                        types::PaymentMethodType::Card,
                        None,
                        None,
                    );
                    utils::store_data_in_redis(
                        redis_conn,
                        payment_id.clone(),
                        payment_data.clone(),
                        state.conf.dummy_connector.payment_ttl,
                    )
                    .await?;
                    Ok(api::ApplicationResponse::Json(payment_data.into()))
                }
                types::DummyConnectorFlow::ThreeDS(_, _) => {
                    let payment_data = types::DummyConnectorPaymentData::new(
                        payment_id.clone(),
                        types::DummyConnectorStatus::Processing,
                        req.amount,
                        req.amount,
                        req.currency,
                        timestamp,
                        types::PaymentMethodType::Card,
                        Some(types::DummyConnectorNextAction::RedirectToUrl(format!(
                            "{}/dummy-connector/authorize/{}",
                            state.conf.server.base_url, attempt_id
                        ))),
                        req.return_url,
                    );
                    utils::store_data_in_redis(
                        redis_conn.clone(),
                        payment_id.clone(),
                        payment_data.clone(),
                        state.conf.dummy_connector.payment_ttl,
                    )
                    .await?;
                    utils::store_data_in_redis(
                        redis_conn,
                        attempt_id.clone(),
                        payment_id.clone(),
                        state.conf.dummy_connector.authorize_ttl,
                    )
                    .await?;
                    Ok(api::ApplicationResponse::Json(payment_data.into()))
                }
            }
        }
    }
}

pub async fn payment_data(
    state: &AppState,
    req: types::DummyConnectorPaymentRetrieveRequest,
) -> types::DummyConnectorResponse<types::DummyConnectorPaymentResponse> {
    let payment_id = req.payment_id;
    tokio_mock_sleep(
        state.conf.dummy_connector.payment_retrieve_duration,
        state.conf.dummy_connector.payment_retrieve_tolerance,
    )
    .await;

    let redis_conn = state.store.get_redis_conn();
    let payment_data = redis_conn
        .get_and_deserialize_key::<types::DummyConnectorPaymentData>(
            payment_id.as_str(),
            "DummyConnectorPaymentData",
        )
        .await
        .change_context(errors::DummyConnectorErrors::PaymentNotFound)?;

    Ok(api::ApplicationResponse::Json(payment_data.into()))
}

pub async fn payment_authorize(
    state: &AppState,
    req: types::DummyConnectorPaymentConfirmRequest,
) -> types::DummyConnectorResponse<String> {
    let redis_conn = state.store.get_redis_conn();
    let payment_data = redis_conn
        .get_and_deserialize_key::<String>(req.attempt_id.as_str(), "String")
        .await
        .async_and_then(|payment_id| async move {
            redis_conn
                .get_and_deserialize_key::<types::DummyConnectorPaymentData>(
                    payment_id.as_str(),
                    "DummyConnectorPaymentData",
                )
                .await
        })
        .await;

    if let Ok(payment_data_inner) = payment_data {
        let return_url = format!(
            "{}/dummy-connector/complete/{}",
            state.conf.server.base_url, req.attempt_id
        );
        Ok(api::ApplicationResponse::FileData((
            utils::get_authorize_page((payment_data_inner.amount / 100) as f64, return_url)
                .as_bytes()
                .to_vec(),
            mime::TEXT_HTML,
        )))
    } else {
        Ok(api::ApplicationResponse::FileData((
            utils::get_expired_page().as_bytes().to_vec(),
            mime::TEXT_HTML,
        )))
    }
}

pub async fn payment_complete(
    state: &AppState,
    req: types::DummyConnectorPaymentCompleteRequest,
) -> types::DummyConnectorResponse<()> {
    let redis_conn = state.store.get_redis_conn();
    let payment_data = redis_conn
        .get_and_deserialize_key::<String>(req.attempt_id.as_str(), "String")
        .await
        .async_and_then(|payment_id| async move {
            let redis_conn = state.store.get_redis_conn();
            redis_conn
                .get_and_deserialize_key::<types::DummyConnectorPaymentData>(
                    payment_id.as_str(),
                    "DummyConnectorPaymentData",
                )
                .await
        })
        .await;

    let payment_status = if req.confirm {
        types::DummyConnectorStatus::Succeeded
    } else {
        types::DummyConnectorStatus::Failed
    };
    let _ = redis_conn.delete_key(req.attempt_id.as_str()).await;
    if let Ok(payment_data) = payment_data {
        let payment_data_new = types::DummyConnectorPaymentData::new(
            payment_data.payment_id,
            payment_status,
            payment_data.amount,
            payment_data.eligible_amount,
            payment_data.currency,
            payment_data.created,
            payment_data.payment_method_type,
            None,
            payment_data.return_url,
        );
        utils::store_data_in_redis(
            redis_conn,
            payment_data_new.payment_id.clone(),
            payment_data_new.clone(),
            state.conf.dummy_connector.payment_ttl,
        )
        .await?;
        return Ok(api::ApplicationResponse::JsonForRedirection(
            api_models::payments::RedirectionResponse {
                return_url: String::new(),
                params: vec![],
                return_url_with_query_params: payment_data_new
                    .return_url
                    .unwrap_or("https://google.com".to_string()),
                http_method: "GET".to_string(),
                headers: vec![],
            },
        ));
    }
    Ok(api::ApplicationResponse::JsonForRedirection(
        api_models::payments::RedirectionResponse {
            return_url: String::new(),
            params: vec![],
            return_url_with_query_params: "https://google.com".to_string(),
            http_method: "GET".to_string(),
            headers: vec![],
        },
    ))
}

pub async fn refund_payment(
    state: &AppState,
    req: types::DummyConnectorRefundRequest,
) -> types::DummyConnectorResponse<types::DummyConnectorRefundResponse> {
    tokio_mock_sleep(
        state.conf.dummy_connector.refund_duration,
        state.conf.dummy_connector.refund_tolerance,
    )
    .await;

    let payment_id = req
        .payment_id
        .get_required_value("payment_id")
        .change_context(errors::DummyConnectorErrors::MissingRequiredField {
            field_name: "payment_id",
        })?;

    let redis_conn = state.store.get_redis_conn();
    let mut payment_data = redis_conn
        .get_and_deserialize_key::<types::DummyConnectorPaymentData>(
            payment_id.as_str(),
            "DummyConnectorPaymentData",
        )
        .await
        .change_context(errors::DummyConnectorErrors::PaymentNotFound)?;

    if payment_data.eligible_amount < req.amount {
        return Err(
            report!(errors::DummyConnectorErrors::RefundAmountExceedsPaymentAmount)
                .attach_printable("Eligible amount is lesser than refund amount"),
        );
    }

    if payment_data.status != types::DummyConnectorStatus::Succeeded {
        return Err(report!(errors::DummyConnectorErrors::PaymentNotSuccessful)
            .attach_printable("Payment is not successful to process the refund"));
    }

    let refund_id = generate_id_with_default_len("dummy_ref");
    payment_data.eligible_amount -= req.amount;
    utils::store_data_in_redis(
        redis_conn.to_owned(),
        payment_id,
        payment_data.to_owned(),
        state.conf.dummy_connector.payment_ttl,
    )
    .await?;

    let refund_data = types::DummyConnectorRefundResponse::new(
        types::DummyConnectorStatus::Succeeded,
        refund_id.to_owned(),
        payment_data.currency,
        common_utils::date_time::now(),
        payment_data.amount,
        req.amount,
    );

    utils::store_data_in_redis(
        redis_conn,
        refund_id,
        refund_data.to_owned(),
        state.conf.dummy_connector.refund_ttl,
    )
    .await?;
    Ok(api::ApplicationResponse::Json(refund_data))
}

pub async fn refund_data(
    state: &AppState,
    req: types::DummyConnectorRefundRetrieveRequest,
) -> types::DummyConnectorResponse<types::DummyConnectorRefundResponse> {
    let refund_id = req.refund_id;
    tokio_mock_sleep(
        state.conf.dummy_connector.refund_retrieve_duration,
        state.conf.dummy_connector.refund_retrieve_tolerance,
    )
    .await;

    let redis_conn = state.store.get_redis_conn();
    let refund_data = redis_conn
        .get_and_deserialize_key::<types::DummyConnectorRefundResponse>(
            refund_id.as_str(),
            "DummyConnectorRefundResponse",
        )
        .await
        .change_context(errors::DummyConnectorErrors::RefundNotFound)?;
    Ok(api::ApplicationResponse::Json(refund_data))
}
