use api_models::analytics::{payments::PaymentReportColumn, GenerateReportRequest};
use common_utils::{
    crypto::{HmacSha512, SignMessage},
    date_time,
    ext_traits::ValueExt,
    id_type,
    request::{Method, RequestContent},
    types::{authentication::AuthInfo, MinorUnit},
};
use diesel_models::{process_tracker::business_status, PaymentAttempt, PaymentIntent};
use error_stack::ResultExt;
use hyperswitch_masking::Mask;
use router_env::logger;
use scheduler::{
    utils as scheduler_utils, workflows::ProcessTrackerWorkflow, SchedulerSessionState,
};
use strum::IntoEnumIterator;
use time::PrimitiveDateTime;

#[cfg(feature = "email")]
use crate::services::email::types::PaymentReportReady;
#[cfg(feature = "email")]
use crate::types::domain::UserEmail;
#[cfg(feature = "email")]
use crate::utils::user as user_utils;
use crate::{
    core::generate_report::PAYMENT_REPORT_TASK, errors, headers, routes::SessionState, services,
    types::storage,
};

// (retry frequency in seconds, count)
const REPORT_RETRY_FREQUENCIES: [(i32, i32); 1] = [(300, 3)];

const PAYMENT_REPORT_TYPE: &str = "payment_report";
const REPORT_GENERATION_COMPLETED_EVENT: &str = "report_generation.completed";
const REPORT_GENERATION_FAILED_EVENT: &str = "report_generation.failed";

const REPORT_FILE_PATH_PREFIX: &str = "reports";
// 7 days, the maximum validity supported by S3 presigned URLs
const REPORT_URL_EXPIRY_SECS: u32 = 604800;
const PAYMENT_REPORT_ROW_LIMIT: i64 = 50000;

const REPORT_TIMESTAMP_FORMAT: &[time::format_description::BorrowedFormatItem<'static>] =
    time::macros::format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

pub struct GenerateReportWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for GenerateReportWorkflow {
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        match process.name.as_deref() {
            Some(PAYMENT_REPORT_TASK) => {
                Box::pin(generate_and_deliver_payment_report(state, process)).await
            }
            _ => Err(errors::ProcessTrackerError::JobNotFound),
        }
    }

    async fn error_handler<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
        error: errors::ProcessTrackerError,
    ) -> errors::CustomResult<(), errors::ProcessTrackerError> {
        logger::error!(?error, process_id = %process.id, "Report generation workflow failed");

        let db = state.get_db();
        let next_schedule_time = scheduler_utils::get_time_from_delta(scheduler_utils::get_delay(
            process.retry_count + 1,
            &REPORT_RETRY_FREQUENCIES,
        ));

        match next_schedule_time {
            Some(schedule_time) => db
                .as_scheduler()
                .retry_process(process, schedule_time)
                .await
                .change_context(errors::ProcessTrackerError::ProcessUpdateFailed),
            None => {
                notify_payment_report_failure(state, &process).await;
                db.as_scheduler()
                    .finish_process_with_business_status(process, business_status::RETRIES_EXCEEDED)
                    .await
                    .change_context(errors::ProcessTrackerError::ProcessUpdateFailed)
            }
        }
    }
}

struct ReportScope {
    organization_id: id_type::OrganizationId,
    merchant_ids: Option<Vec<id_type::MerchantId>>,
    profile_ids: Option<Vec<id_type::ProfileId>>,
}

impl ReportScope {
    fn from_auth_info(auth: &AuthInfo) -> Self {
        match auth {
            AuthInfo::OrgLevel { org_id } => Self {
                organization_id: org_id.clone(),
                merchant_ids: None,
                profile_ids: None,
            },
            AuthInfo::MerchantLevel {
                org_id,
                merchant_ids,
                ..
            } => Self {
                organization_id: org_id.clone(),
                merchant_ids: Some(merchant_ids.clone()),
                profile_ids: None,
            },
            AuthInfo::ProfileLevel {
                org_id,
                merchant_id,
                profile_ids,
                ..
            } => Self {
                organization_id: org_id.clone(),
                merchant_ids: Some(vec![merchant_id.clone()]),
                profile_ids: Some(profile_ids.clone()),
            },
        }
    }
}

async fn generate_and_deliver_payment_report(
    state: &SessionState,
    process: storage::ProcessTracker,
) -> Result<(), errors::ProcessTrackerError> {
    let tracking_data: GenerateReportRequest = process
        .tracking_data
        .clone()
        .parse_value("GenerateReportRequest")?;

    let scope = ReportScope::from_auth_info(&tracking_data.auth);
    let start_time = tracking_data.request.time_range.start_time;
    let end_time = tracking_data
        .request
        .time_range
        .end_time
        .unwrap_or_else(date_time::now);

    let rows = state
        .store
        .find_payment_report_rows(
            &scope.organization_id,
            scope.merchant_ids.clone(),
            scope.profile_ids.clone(),
            start_time,
            end_time,
            PAYMENT_REPORT_ROW_LIMIT,
        )
        .await?;

    logger::info!(
        row_count = rows.len(),
        organization_id = %scope.organization_id.get_string_repr(),
        "Fetched rows for payment report"
    );

    let report_bytes = build_payment_report_csv(&rows)?;

    let file_key = build_report_file_key(&scope, start_time, end_time)?;
    state
        .file_storage_client
        .upload_file(&file_key, report_bytes)
        .await
        .map_err(|error| {
            logger::error!(?error, file_key, "Failed to upload payment report");
            errors::ProcessTrackerError::FlowExecutionError {
                flow: "PaymentReportUpload",
            }
        })?;

    let download_url = state
        .file_storage_client
        .get_signed_url(
            &file_key,
            std::time::Duration::from_secs(u64::from(REPORT_URL_EXPIRY_SECS)),
        )
        .await
        .map_err(|error| {
            logger::error!(?error, file_key, "Failed to generate payment report URL");
            errors::ProcessTrackerError::FlowExecutionError {
                flow: "PaymentReportSignedUrl",
            }
        })?;

    deliver_payment_report(
        state,
        &tracking_data,
        &scope,
        &download_url,
        start_time,
        end_time,
    )
    .await?;

    state
        .get_db()
        .as_scheduler()
        .finish_process_with_business_status(process, business_status::COMPLETED_BY_PT)
        .await?;

    Ok(())
}

fn build_report_file_key(
    scope: &ReportScope,
    start_time: PrimitiveDateTime,
    end_time: PrimitiveDateTime,
) -> Result<String, errors::ProcessTrackerError> {
    let format_for_key = |date: PrimitiveDateTime| {
        date_time::format_date(date, date_time::DateFormat::YYYYMMDDHHmmss)
            .map_err(|_| errors::ProcessTrackerError::TypeConversionError)
    };
    let start = format_for_key(start_time)?;
    let end = format_for_key(end_time)?;

    let mut segments = vec![
        REPORT_FILE_PATH_PREFIX.to_owned(),
        scope.organization_id.get_string_repr().to_owned(),
    ];
    if let Some(merchant_id) = scope
        .merchant_ids
        .as_ref()
        .and_then(|merchant_ids| merchant_ids.first())
    {
        segments.push(merchant_id.get_string_repr().to_owned());
    }
    if let Some(profile_id) = scope
        .profile_ids
        .as_ref()
        .and_then(|profile_ids| profile_ids.first())
    {
        segments.push(profile_id.get_string_repr().to_owned());
    }
    segments.push(String::from("payments"));
    segments.push(format!("payments_report_{start}_{end}.csv"));

    Ok(segments.join("/"))
}

fn build_payment_report_csv(
    rows: &[(PaymentAttempt, Option<PaymentIntent>)],
) -> Result<Vec<u8>, errors::ProcessTrackerError> {
    let columns: Vec<PaymentReportColumn> = PaymentReportColumn::iter().collect();
    let mut writer = csv::WriterBuilder::new()
        .quote_style(csv::QuoteStyle::Always)
        .from_writer(Vec::new());

    let csv_error = |error: csv::Error| {
        logger::error!(?error, "Failed to write payment report record");
        errors::ProcessTrackerError::FlowExecutionError {
            flow: "PaymentReportCsv",
        }
    };

    writer
        .write_record(columns.iter().map(AsRef::as_ref))
        .map_err(csv_error)?;

    for (attempt, intent) in rows {
        writer
            .write_record(
                columns
                    .iter()
                    .map(|column| payment_report_column_value(*column, attempt, intent.as_ref())),
            )
            .map_err(csv_error)?;
    }

    writer.into_inner().map_err(|error| {
        logger::error!(?error, "Failed to finalize payment report");
        errors::ProcessTrackerError::FlowExecutionError {
            flow: "PaymentReportCsv",
        }
    })
}

fn payment_report_column_value(
    column: PaymentReportColumn,
    attempt: &PaymentAttempt,
    intent: Option<&PaymentIntent>,
) -> String {
    match column {
        PaymentReportColumn::PaymentId => {
            display_optional(intent.map(|intent| intent.payment_id.get_string_repr().to_owned()))
        }
        PaymentReportColumn::AttemptId => attempt.attempt_id.clone(),
        PaymentReportColumn::Status => attempt.status.to_string(),
        PaymentReportColumn::Amount => format_amount_in_base_unit(
            intent.map(|intent| intent.amount),
            intent.and_then(|intent| intent.currency),
        ),
        PaymentReportColumn::Currency => {
            display_optional(intent.and_then(|intent| intent.currency))
        }
        PaymentReportColumn::Connector => attempt.connector.clone().unwrap_or_default(),
        PaymentReportColumn::ConnectorTransactionId => display_optional(
            attempt
                .connector_transaction_id
                .as_ref()
                .map(|transaction_id| transaction_id.get_id().clone()),
        ),
        PaymentReportColumn::AmountToCapture => format_amount_in_base_unit(
            attempt.amount_to_capture,
            intent.and_then(|intent| intent.currency),
        ),
        PaymentReportColumn::CustomerId => display_optional(
            intent
                .and_then(|intent| intent.customer_id.as_ref())
                .map(|customer_id| customer_id.get_string_repr().to_owned()),
        ),
        PaymentReportColumn::CreatedAt => format_timestamp(attempt.created_at),
        PaymentReportColumn::OrderDetails => intent
            .and_then(|intent| intent.order_details.as_ref())
            .and_then(|order_details| serde_json::to_string(order_details).ok())
            .unwrap_or_default(),
        PaymentReportColumn::ErrorMessage => attempt.error_message.clone().unwrap_or_default(),
        PaymentReportColumn::CaptureMethod => display_optional(attempt.capture_method),
        PaymentReportColumn::AuthenticationType => display_optional(attempt.authentication_type),
        PaymentReportColumn::MandateId => attempt.mandate_id.clone().unwrap_or_default(),
        PaymentReportColumn::PaymentMethod => display_optional(attempt.payment_method),
        PaymentReportColumn::PaymentMethodType => display_optional(attempt.payment_method_type),
        PaymentReportColumn::Metadata => {
            display_optional(intent.and_then(|intent| intent.metadata.as_ref()))
        }
        PaymentReportColumn::SetupFutureUsage => {
            display_optional(intent.and_then(|intent| intent.setup_future_usage))
        }
        PaymentReportColumn::StatementDescriptorName => intent
            .and_then(|intent| intent.statement_descriptor_name.clone())
            .unwrap_or_default(),
        PaymentReportColumn::Description => intent
            .and_then(|intent| intent.description.clone())
            .unwrap_or_default(),
        PaymentReportColumn::OffSession => {
            display_optional(intent.and_then(|intent| intent.off_session))
        }
        PaymentReportColumn::BusinessCountry => {
            display_optional(intent.and_then(|intent| intent.business_country))
        }
        PaymentReportColumn::BusinessLabel => intent
            .and_then(|intent| intent.business_label.clone())
            .unwrap_or_default(),
        PaymentReportColumn::BusinessSubLabel => {
            attempt.business_sub_label.clone().unwrap_or_default()
        }
        PaymentReportColumn::AllowedPaymentMethodTypes => {
            display_optional(intent.and_then(|intent| intent.allowed_payment_method_types.as_ref()))
        }
        PaymentReportColumn::PaymentMethodData => {
            display_optional(attempt.payment_method_data.as_ref())
        }
        PaymentReportColumn::CardNetwork => {
            get_card_field(attempt.payment_method_data.as_ref(), "card_network")
        }
        PaymentReportColumn::FingerprintId => intent
            .and_then(|intent| intent.fingerprint_id.clone())
            .unwrap_or_default(),
        PaymentReportColumn::ModifiedAt => format_timestamp(attempt.modified_at),
        PaymentReportColumn::ErrorCode => attempt.error_code.clone().unwrap_or_default(),
        PaymentReportColumn::PaymentMethodId => {
            attempt.payment_method_id.clone().unwrap_or_default()
        }
        PaymentReportColumn::CardHolderName => {
            get_card_field(attempt.payment_method_data.as_ref(), "card_holder_name")
        }
        PaymentReportColumn::MerchantOrderReferenceId => intent
            .and_then(|intent| intent.merchant_order_reference_id.clone())
            .unwrap_or_default(),
        PaymentReportColumn::ProfileId => display_optional(
            intent
                .and_then(|intent| intent.profile_id.as_ref())
                .map(|profile_id| profile_id.get_string_repr().to_owned()),
        ),
    }
}

fn display_optional(value: Option<impl ToString>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

fn format_timestamp(date: PrimitiveDateTime) -> String {
    date.format(&REPORT_TIMESTAMP_FORMAT)
        .unwrap_or_else(|_| date.to_string())
}

fn format_amount_in_base_unit(
    amount: Option<MinorUnit>,
    currency: Option<common_enums::Currency>,
) -> String {
    let Some(amount) = amount else {
        return String::new();
    };
    let amount = amount.get_amount_as_i64();
    // Unknown currencies are treated as two decimal, in line with the lambda based flow
    let currency = currency.unwrap_or(common_enums::Currency::USD);

    let sign = if amount < 0 { "-" } else { "" };
    let amount_abs = amount.unsigned_abs();
    if currency.is_zero_decimal_currency() {
        format!("{sign}{amount_abs}")
    } else if currency.is_three_decimal_currency() {
        format!("{sign}{}.{:03}", amount_abs / 1000, amount_abs % 1000)
    } else {
        format!("{sign}{}.{:02}", amount_abs / 100, amount_abs % 100)
    }
}

fn get_card_field(payment_method_data: Option<&serde_json::Value>, field: &str) -> String {
    payment_method_data
        .and_then(|payment_method_data| payment_method_data.get("card"))
        .and_then(|card| card.get(field))
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
        .unwrap_or_default()
}

#[derive(serde::Serialize)]
struct ReportWebhookPayload<'a> {
    org_id: &'a id_type::OrganizationId,
    #[serde(skip_serializing_if = "Option::is_none")]
    merchant_id: Option<&'a id_type::MerchantId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    profile_id: Option<&'a id_type::ProfileId>,
    event: &'static str,
    data: ReportWebhookData<'a>,
}

#[derive(serde::Serialize)]
#[serde(untagged)]
enum ReportWebhookData<'a> {
    Completed {
        report_type: &'static str,
        start_date_utc: String,
        end_date_utc: String,
        download_url: &'a str,
        expires_in_hours: u32,
    },
    Failed {
        code: &'static str,
        message: &'static str,
    },
}

async fn deliver_payment_report(
    state: &SessionState,
    tracking_data: &GenerateReportRequest,
    scope: &ReportScope,
    download_url: &str,
    start_time: PrimitiveDateTime,
    end_time: PrimitiveDateTime,
) -> Result<(), errors::ProcessTrackerError> {
    match tracking_data.request.return_url.as_ref() {
        Some(return_url) => {
            let payload = ReportWebhookPayload {
                org_id: &scope.organization_id,
                merchant_id: scope
                    .merchant_ids
                    .as_ref()
                    .and_then(|merchant_ids| merchant_ids.first()),
                profile_id: scope
                    .profile_ids
                    .as_ref()
                    .and_then(|profile_ids| profile_ids.first()),
                event: REPORT_GENERATION_COMPLETED_EVENT,
                data: ReportWebhookData::Completed {
                    report_type: PAYMENT_REPORT_TYPE,
                    start_date_utc: format_timestamp(start_time),
                    end_date_utc: format_timestamp(end_time),
                    download_url,
                    expires_in_hours: REPORT_URL_EXPIRY_SECS / 3600,
                },
            };
            send_report_webhook(
                state,
                return_url.get_string_repr(),
                tracking_data.payment_response_hash_key.as_deref(),
                &payload,
            )
            .await
        }
        None => send_report_emails(state, tracking_data, download_url, start_time, end_time).await,
    }
}

async fn send_report_webhook(
    state: &SessionState,
    return_url: &str,
    payment_response_hash_key: Option<&str>,
    payload: &ReportWebhookPayload<'_>,
) -> Result<(), errors::ProcessTrackerError> {
    let body = serde_json::to_vec(payload)
        .map_err(|_| errors::ProcessTrackerError::SerializationFailed)?;

    let mut request_headers = vec![(
        headers::CONTENT_TYPE.to_string(),
        "application/json".to_string().into_masked(),
    )];
    if let Some(hash_key) = payment_response_hash_key {
        let signature = HmacSha512
            .sign_message(hash_key.as_bytes(), &body)
            .map(hex::encode)
            .map_err(|error| {
                logger::error!(?error, "Failed to sign report webhook payload");
                errors::ProcessTrackerError::FlowExecutionError {
                    flow: "PaymentReportWebhookSignature",
                }
            })?;
        request_headers.push((
            headers::X_WEBHOOK_SIGNATURE.to_string(),
            signature.into_masked(),
        ));
    }

    let request = services::RequestBuilder::new()
        .method(Method::Post)
        .url(return_url)
        .attach_default_headers()
        .headers(request_headers)
        .set_body(RequestContent::RawBytes(body))
        .build();

    let response = state
        .api_client
        .send_request(state, request, None, false)
        .await
        .map_err(|error| {
            logger::error!(?error, "Failed to deliver payment report webhook");
            errors::ProcessTrackerError::FlowExecutionError {
                flow: "PaymentReportWebhook",
            }
        })?;

    if !response.status().is_success() {
        logger::error!(
            status = %response.status(),
            "Payment report webhook was not accepted by the merchant endpoint"
        );
        return Err(errors::ProcessTrackerError::FlowExecutionError {
            flow: "PaymentReportWebhook",
        });
    }

    Ok(())
}

#[cfg(feature = "email")]
async fn send_report_emails(
    state: &SessionState,
    tracking_data: &GenerateReportRequest,
    download_url: &str,
    start_time: PrimitiveDateTime,
    end_time: PrimitiveDateTime,
) -> Result<(), errors::ProcessTrackerError> {
    let mut recipients = vec![tracking_data.email.clone()];
    if let Some(emails) = tracking_data.request.emails.as_ref() {
        recipients.extend(emails.iter().cloned());
    }

    let start_date = format_timestamp(start_time);
    let end_date = format_timestamp(end_time);
    let expires_in_days = REPORT_URL_EXPIRY_SECS / 86400;

    for recipient in recipients {
        let email_contents = PaymentReportReady {
            recipient_email: UserEmail::from_pii_email(recipient).map_err(|error| {
                logger::error!(?error, "Failed to parse report recipient email");
                errors::ProcessTrackerError::FlowExecutionError {
                    flow: "PaymentReportEmail",
                }
            })?,
            subject: format!("Your Payments Report: {start_date} - {end_date}"),
            report_link: download_url.to_owned(),
            start_date: start_date.clone(),
            end_date: end_date.clone(),
            expires_in_days,
        };

        state
            .email_client
            .clone()
            .compose_and_send_email(
                user_utils::get_base_url(state),
                Box::new(email_contents),
                state.conf.proxy.https_url.as_ref(),
            )
            .await
            .map_err(errors::ProcessTrackerError::EEmailError)?;
    }

    Ok(())
}

/// Without the email feature there is no way to deliver the report link, so it is only
/// logged. Production builds are expected to enable the email feature.
#[cfg(not(feature = "email"))]
async fn send_report_emails(
    _state: &SessionState,
    _tracking_data: &GenerateReportRequest,
    download_url: &str,
    _start_time: PrimitiveDateTime,
    _end_time: PrimitiveDateTime,
) -> Result<(), errors::ProcessTrackerError> {
    logger::warn!(
        download_url,
        "Email feature is disabled; payment report generated but not delivered over email"
    );
    Ok(())
}

/// Notifies the merchant's webhook endpoint that report generation has permanently
/// failed, mirroring the failure notification of the lambda based flow. Failures here are
/// logged and swallowed since the task is already being finished.
async fn notify_payment_report_failure(state: &SessionState, process: &storage::ProcessTracker) {
    let tracking_data: GenerateReportRequest = match process
        .tracking_data
        .clone()
        .parse_value("GenerateReportRequest")
    {
        Ok(tracking_data) => tracking_data,
        Err(error) => {
            logger::error!(?error, "Failed to parse tracking data for failure webhook");
            return;
        }
    };

    let Some(return_url) = tracking_data.request.return_url.as_ref() else {
        return;
    };

    let scope = ReportScope::from_auth_info(&tracking_data.auth);
    let payload = ReportWebhookPayload {
        org_id: &scope.organization_id,
        merchant_id: scope
            .merchant_ids
            .as_ref()
            .and_then(|merchant_ids| merchant_ids.first()),
        profile_id: scope
            .profile_ids
            .as_ref()
            .and_then(|profile_ids| profile_ids.first()),
        event: REPORT_GENERATION_FAILED_EVENT,
        data: ReportWebhookData::Failed {
            code: "internal_server_error",
            message: "We could not generate the report due to an internal server error. Please request the report again.",
        },
    };

    if let Err(error) = send_report_webhook(
        state,
        return_url.get_string_repr(),
        tracking_data.payment_response_hash_key.as_deref(),
        &payload,
    )
    .await
    {
        logger::error!(?error, "Failed to deliver payment report failure webhook");
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use time::macros::datetime;

    use super::*;

    fn test_scope(with_merchant: bool, with_profile: bool) -> ReportScope {
        ReportScope {
            organization_id: id_type::OrganizationId::try_from_string(String::from("org_test"))
                .unwrap(),
            merchant_ids: with_merchant.then(|| {
                vec![
                    id_type::MerchantId::try_from(std::borrow::Cow::from("merchant_test")).unwrap(),
                ]
            }),
            profile_ids: with_profile.then(|| {
                vec![id_type::ProfileId::try_from(std::borrow::Cow::from("pro_test")).unwrap()]
            }),
        }
    }

    #[test]
    fn report_file_key_includes_scope_segments() {
        let start_time = datetime!(2026-07-01 00:00:00);
        let end_time = datetime!(2026-08-01 00:00:00);

        assert_eq!(
            build_report_file_key(&test_scope(false, false), start_time, end_time).unwrap(),
            "reports/org_test/payments/payments_report_20260701000000_20260801000000.csv"
        );
        assert_eq!(
            build_report_file_key(&test_scope(true, false), start_time, end_time).unwrap(),
            "reports/org_test/merchant_test/payments/payments_report_20260701000000_20260801000000.csv"
        );
        assert_eq!(
            build_report_file_key(&test_scope(true, true), start_time, end_time).unwrap(),
            "reports/org_test/merchant_test/pro_test/payments/payments_report_20260701000000_20260801000000.csv"
        );
    }

    #[test]
    fn amounts_are_converted_to_currency_base_units() {
        let amount = Some(MinorUnit::new(12345));
        assert_eq!(
            format_amount_in_base_unit(amount, Some(common_enums::Currency::USD)),
            "123.45"
        );
        assert_eq!(
            format_amount_in_base_unit(amount, Some(common_enums::Currency::JPY)),
            "12345"
        );
        assert_eq!(
            format_amount_in_base_unit(amount, Some(common_enums::Currency::BHD)),
            "12.345"
        );
        assert_eq!(
            format_amount_in_base_unit(Some(MinorUnit::new(5)), Some(common_enums::Currency::USD)),
            "0.05"
        );
        // Amounts that overflow u32 minor units must not be dropped
        assert_eq!(
            format_amount_in_base_unit(
                Some(MinorUnit::new(10_000_000_000)),
                Some(common_enums::Currency::USD)
            ),
            "100000000.00"
        );
        assert_eq!(format_amount_in_base_unit(amount, None), "123.45");
        assert_eq!(
            format_amount_in_base_unit(None, Some(common_enums::Currency::USD)),
            ""
        );
    }

    #[test]
    fn card_fields_are_read_from_payment_method_data() {
        let payment_method_data = serde_json::json!({
            "card": {
                "card_network": "Visa",
                "card_exp_year": 2030
            }
        });
        assert_eq!(
            get_card_field(Some(&payment_method_data), "card_network"),
            "Visa"
        );
        // Non-string and missing values are rendered as empty
        assert_eq!(
            get_card_field(Some(&payment_method_data), "card_exp_year"),
            ""
        );
        assert_eq!(
            get_card_field(Some(&payment_method_data), "card_holder_name"),
            ""
        );
        assert_eq!(get_card_field(None, "card_network"), "");
    }

    #[test]
    fn tracking_data_parses_into_generate_report_request() {
        let tracking_data = serde_json::json!({
            "request": {
                "timeRange": {
                    "start_time": "2026-07-01T00:00:00.000Z",
                    "end_time": "2026-08-01T00:00:00.000Z"
                },
                "emails": null,
                "returnUrl": null,
                "columns": null
            },
            "merchantId": "merchant_test",
            "auth": {
                "MerchantLevel": {
                    "org_id": "org_test",
                    "merchant_ids": ["merchant_test"],
                    "processor_merchant_ids": null
                }
            },
            "email": "user@example.com",
            "paymentResponseHashKey": null
        });

        let request: GenerateReportRequest = serde_json::from_value(tracking_data).unwrap();
        let scope = ReportScope::from_auth_info(&request.auth);
        assert_eq!(scope.organization_id.get_string_repr(), "org_test");
        assert_eq!(
            scope.merchant_ids.unwrap()[0].get_string_repr(),
            "merchant_test"
        );
        assert!(scope.profile_ids.is_none());
    }
}
