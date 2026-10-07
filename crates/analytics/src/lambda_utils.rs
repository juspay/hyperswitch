use aws_config::{self, meta::region::RegionProviderChain, Region};
use aws_sdk_lambda::{types::InvocationType::Event, Client};
use aws_smithy_types::Blob;
use common_utils::errors::CustomResult;
use error_stack::{report, ResultExt};

use crate::errors::AnalyticsError;

async fn get_aws_client(region: String) -> Client {
    let region_provider = RegionProviderChain::first_try(Region::new(region));
    let sdk_config = aws_config::from_env().region(region_provider).load().await;
    Client::new(&sdk_config)
}

/// Identity of one invocation: the function, the region and the report request
/// the Lambda is handed.
///
/// This call returns `()`, so args are the only place anything about it is
/// recorded. The request goes on whole rather than as a digest, so a candidate
/// that builds a different report is a divergence someone can read rather than
/// an address that stopped matching.
#[cfg(feature = "deja")]
fn deja_args(function_name: &str, region: &str, json_bytes: &[u8]) -> serde_json::Value {
    // Every caller serialises a struct, so these bytes are a JSON document.
    let payload = serde_json::from_slice::<serde_json::Value>(json_bytes).unwrap_or_else(|_| {
        serde_json::Value::String(String::from_utf8_lossy(json_bytes).into_owned())
    });
    serde_json::json!({
        "function_name": function_name,
        "region": region,
        "payload": payload,
    })
}

/// Starts a report-generation Lambda, asynchronously, and returns once AWS has
/// accepted the event.
///
/// The invocation has an effect outside the process — a report is generated and
/// emailed — so a replay must never make it: the seam substitutes the recorded
/// outcome.
///
/// No `on_miss`. An `Ok(())` would tell the caller the report was queued when
/// nothing was, and the caller answers its own client with that.
#[cfg_attr(
    feature = "deja",
    deja::boundary(
        boundary = "lambda",
        component = "analytics::lambda_utils",
        operation = "invoke_lambda",
        op = ExternalCall,
        replay = Substitute,
        effect = Http,
        returns = Unit,
        codec = deja::codec::ResultCodec::<(), AnalyticsError>,
        args = deja_args(function_name, region, json_bytes),
    )
)]
pub async fn invoke_lambda(
    function_name: &str,
    region: &str,
    json_bytes: &[u8],
) -> CustomResult<(), AnalyticsError> {
    get_aws_client(region.to_string())
        .await
        .invoke()
        .function_name(function_name)
        .invocation_type(Event)
        .payload(Blob::new(json_bytes.to_owned()))
        .send()
        .await
        .map_err(|er| {
            let er_rep = format!("{er:?}");
            report!(er).attach_printable(er_rep)
        })
        .change_context(AnalyticsError::UnknownError)
        .attach_printable("Lambda invocation failed")?;
    Ok(())
}

#[cfg(all(test, feature = "deja"))]
mod tests {
    use crate::errors::AnalyticsError;

    /// The seam's own declaration, cut out of this file's source. The anchors are
    /// assembled at run time so this module's text does not contain them.
    #[allow(
        clippy::expect_used,
        clippy::indexing_slicing,
        reason = "test helper: a free fn, so allow-expect-in-tests does not cover it; the offsets come from `find` on the same string"
    )]
    fn invoke_lambda_declaration() -> &'static str {
        let source = include_str!("lambda_utils.rs");
        let start = source
            .find(&["deja::", "boundary("].concat())
            .expect("the seam attribute is present");
        let end = source[start..]
            .find(&["pub async fn ", "invoke_lambda("].concat())
            .expect("the seam is declared on invoke_lambda");
        &source[start..start + end]
    }

    #[test]
    fn the_lambda_seam_declares_what_it_invokes() {
        let declaration = invoke_lambda_declaration();
        for expected in [
            "boundary = \"lambda\"",
            "component = \"analytics::lambda_utils\"",
            "operation = \"invoke_lambda\"",
            "replay = Substitute",
            "effect = Http",
            "codec = deja::codec::ResultCodec::<(), AnalyticsError>",
            "args = deja_args(function_name, region, json_bytes)",
        ] {
            assert!(
                declaration.contains(expected),
                "declaration lost `{expected}`:\n{declaration}"
            );
        }
        assert!(
            !declaration.contains("on_miss"),
            "an invented Ok would report a report as queued; a miss must fail-stop"
        );
    }

    #[test]
    fn invocation_identity_separates_requests_and_records_the_request() {
        let payload = br#"{"payment_response_hash_key":"hunter2","email":"a@example.com"}"#;
        let first = super::deja_args("report_fn", "ap-south-1", payload);
        assert_eq!(first, super::deja_args("report_fn", "ap-south-1", payload));
        assert_ne!(first, super::deja_args("other_fn", "ap-south-1", payload));
        assert_ne!(first, super::deja_args("report_fn", "us-east-1", payload));
        assert_ne!(first, super::deja_args("report_fn", "ap-south-1", b"{}"));
        // What the Lambda was handed is on the tape, so a candidate that builds a
        // different report diverges on the request rather than on an address.
        assert_eq!(
            first.pointer("/payload/payment_response_hash_key"),
            Some(&serde_json::json!("hunter2"))
        );
    }

    /// Every variant but `NotImplemented` has to round-trip, and the match has
    /// no wildcard so a new variant stops this compiling until the hand-written
    /// reader in `errors.rs` is told about it.
    #[test]
    fn every_readable_analytics_error_survives_a_round_trip() {
        for error in [
            AnalyticsError::UnknownError,
            AnalyticsError::AccessForbiddenError,
            AnalyticsError::ForexFetchFailed,
            AnalyticsError::MissingEmail,
            AnalyticsError::InvalidReturnUrl("ftp://x".to_string()),
        ] {
            match &error {
                AnalyticsError::NotImplemented(_)
                | AnalyticsError::UnknownError
                | AnalyticsError::AccessForbiddenError
                | AnalyticsError::ForexFetchFailed
                | AnalyticsError::MissingEmail
                | AnalyticsError::InvalidReturnUrl(_) => {}
            }
            let wire = serde_json::to_string(&error).expect("serialises");
            let back: AnalyticsError = serde_json::from_str(&wire).expect("deserialises");
            assert_eq!(error.to_string(), back.to_string());
        }
    }

    #[test]
    fn a_recorded_not_implemented_is_refused_rather_than_invented() {
        let wire = serde_json::to_string(&AnalyticsError::NotImplemented("x")).expect("serialises");
        assert!(serde_json::from_str::<AnalyticsError>(&wire).is_err());
    }
}
