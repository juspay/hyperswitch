//! What the merchant attaches to a payment: its transaction reference, the statement descriptor
//! and custom parameters.

use common_types::payments::BillingDescriptor;
use sha2::{Digest, Sha256};

use super::PaymentItem;
use crate::connectors::aci::transformers::request::AciCustomParameters;

/// `merchantTransactionId`, from the merchant's order reference when it is usable.
///
/// Prefer the merchant-supplied order reference, but only when it already clears the floor: it
/// is unvalidated and carries no uniqueness guarantee, so a 1-character value should not be
/// padded into existence. `connector_request_reference_id` is unique per attempt and is reused
/// verbatim on retries, making it the safer base.
pub(super) fn merchant_transaction_id(
    item: PaymentItem<'_>,
    trim_to_acquirer_limit: bool,
) -> String {
    let router_data = item.router_data;
    let reference = router_data
        .request
        .merchant_order_reference_id
        .as_deref()
        .filter(|reference| reference.chars().count() >= ACI_MERCHANT_TXN_ID_MIN_LEN)
        .unwrap_or(&router_data.connector_request_reference_id);
    fit_merchant_transaction_id(reference, trim_to_acquirer_limit)
}

/// OPPWA validates `merchantTransactionId` against `[\s\S]{8,255}` — those bounds are universal
/// platform rules and always enforced.
const ACI_MERCHANT_TXN_ID_MIN_LEN: usize = 8;

const ACI_MERCHANT_TXN_ID_MAX_LEN: usize = 255;

/// *Some* ACI acquirers additionally reject anything over 16 characters, but this is not
/// universal, so the trim is opt-in per merchant connector account via the connector-metadata
/// flag `trim_merchant_transaction_id: true` (same mechanism as `test_mode`).
const ACI_ACQUIRER_MERCHANT_TXN_ID_TRIM_LEN: usize = 16;

/// Fit a reference into OPPWA's `merchantTransactionId` window.
///
/// The universal `{8,255}` bounds always apply. The acquirer-specific 16-character trim applies
/// only when the merchant connector account opts in via `trim_merchant_transaction_id`; otherwise
/// the reference passes through unaltered (short of the universal bounds).
///
/// When truncating, keep the high-entropy tail: hyperswitch references share a common prefix, so
/// truncating the front risks collapsing distinct payments onto one id, which ACI rejects as a
/// duplicate. Below the floor, extend with leading hex of the reference's own SHA-256 — the
/// filler varies with the input, so `"a"` and `"a0"` cannot pad to the same value, and it is
/// deterministic, so a retried attempt pads identically.
fn fit_merchant_transaction_id(reference: &str, trim_to_acquirer_limit: bool) -> String {
    let max_len = if trim_to_acquirer_limit {
        ACI_ACQUIRER_MERCHANT_TXN_ID_TRIM_LEN
    } else {
        ACI_MERCHANT_TXN_ID_MAX_LEN
    };
    let char_count = reference.chars().count();
    if char_count >= ACI_MERCHANT_TXN_ID_MIN_LEN {
        let skip = char_count.saturating_sub(max_len);
        return reference.chars().skip(skip).collect();
    }
    // 64 lowercase hex chars, so there is always enough filler to reach the floor.
    let digest = hex::encode(Sha256::digest(reference.as_bytes()));
    let pad_len = ACI_MERCHANT_TXN_ID_MIN_LEN.saturating_sub(char_count);
    let filler: String = digest.chars().take(pad_len).collect();
    format!("{reference}{filler}")
}

/// The statement descriptor for this payment.
///
/// Reading `billing_descriptor` alone also covers a merchant still sending the deprecated flat
/// `statement_descriptor_name`/`statement_descriptor_suffix`, because
/// `PaymentIntent::get_billing_descriptor` folds those in before the connector ever sees them.
pub(super) fn descriptor(item: PaymentItem<'_>) -> Option<String> {
    item.router_data
        .request
        .billing_descriptor
        .as_ref()
        .and_then(join_descriptor)
}

/// OPPWA's `descriptor` is `AN127`.
const ACI_DESCRIPTOR_MAX_LEN: usize = 127;

/// Trim, then clamp to [`ACI_DESCRIPTOR_MAX_LEN`] *characters*, dropping a value that is empty
/// either way.
///
/// Unlike [`fit_merchant_transaction_id`], which keeps the high-entropy tail because a
/// truncated reference must stay unique, this is display text: the head is what the cardholder
/// needs to recognise, so truncation takes from the front. An over-long descriptor is shortened
/// rather than rejected — failing a payment over a cosmetic field would be the worse outcome of
/// the two. Counting characters rather than bytes keeps a multi-byte name whole.
fn clamp_descriptor(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.chars().take(ACI_DESCRIPTOR_MAX_LEN).collect())
}

/// Hyperswitch carries a statement descriptor and a suffix; OPPWA has the single `descriptor`.
/// Join whichever parts are present with one space, then fit the result to `AN127`.
fn join_descriptor(billing_descriptor: &BillingDescriptor) -> Option<String> {
    let joined = [
        billing_descriptor.statement_descriptor.as_deref(),
        billing_descriptor.statement_descriptor_suffix.as_deref(),
    ]
    .into_iter()
    .flatten()
    .map(str::trim)
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join(" ");

    clamp_descriptor(&joined)
}

/// `customParameters`: merchant metadata first, then the connector's own fixed fields.
///
/// Merchant entries are validated against OPPWA's name/value constraints and dropped if they
/// would be rejected or would collide with one of the fixed names — the body is a flat list of
/// pairs, so a colliding key is emitted twice rather than overridden.
pub(super) fn custom_parameters(item: PaymentItem<'_>) -> AciCustomParameters {
    let router_data = item.router_data;
    let mut custom_parameters = AciCustomParameters::default();
    let merchant_metadata = router_data
        .request
        .metadata
        .as_ref()
        .and_then(serde_json::Value::as_object);
    for (key, value) in merchant_metadata.into_iter().flatten() {
        let value = match value {
            serde_json::Value::String(text) => text.clone(),
            serde_json::Value::Null => continue,
            other => other.to_string(),
        };
        custom_parameters.insert_merchant_supplied(key, value);
    }
    custom_parameters.insert("orchestrator", "hyperswitch");
    custom_parameters.insert("paymentId", &router_data.payment_id);
    custom_parameters
}

#[cfg(test)]
mod tests {
    use common_utils::types::StringMajorUnit;
    use hyperswitch_masking::Secret;

    use super::*;
    use crate::connectors::aci::transformers::request::{
        AciBillingAddress, AciCustomerBrowserInfo, AciCustomerData, AciExternalThreeDsData,
        AciPaymentType, AciPaymentsRequest, PaymentDetails, TransactionDetails,
    };

    /// OPPWA's universal `merchantTransactionId` bounds are `[\s\S]{8,255}`. The 16-character
    /// trim is an acquirer-specific rule, opt-in per merchant connector account via
    /// `trim_merchant_transaction_id` — without the flag the reference must pass through
    /// unaltered (short of the universal bounds).
    #[test]
    fn merchant_transaction_id_is_clamped_to_the_documented_window() {
        let long = "pay_abcdefghijklmnopqrstuvwxyz0123456789";

        // Opted in: trim to the acquirer's 16-character limit, keeping the high-entropy tail.
        let trimmed = fit_merchant_transaction_id(long, true);
        assert_eq!(
            trimmed.chars().count(),
            ACI_ACQUIRER_MERCHANT_TXN_ID_TRIM_LEN
        );
        assert!(
            long.ends_with(&trimmed),
            "the high-entropy tail must be kept, not the shared prefix"
        );

        // Not opted in: pass through unaltered — the 16-char limit is not universal.
        assert_eq!(fit_merchant_transaction_id(long, false), long);

        // OPPWA's universal 255-char ceiling still applies without the flag.
        let oversized = "x".repeat(300);
        assert_eq!(
            fit_merchant_transaction_id(&oversized, false)
                .chars()
                .count(),
            ACI_MERCHANT_TXN_ID_MAX_LEN
        );

        // The universal 8-char floor applies in both modes.
        for trim in [true, false] {
            for short in ["a", "ord7", "1234567"] {
                let padded = fit_merchant_transaction_id(short, trim);
                assert_eq!(
                    padded.chars().count(),
                    ACI_MERCHANT_TXN_ID_MIN_LEN,
                    "`{short}` should pad up to the 8-character floor (trim={trim})"
                );
                assert!(
                    padded.starts_with(short),
                    "padding must preserve the original"
                );
            }
        }

        // Deterministic, so a retried attempt pads identically...
        assert_eq!(
            fit_merchant_transaction_id("ord7", false),
            fit_merchant_transaction_id("ord7", false)
        );
        // ...and input-dependent, so prefix-related references cannot collide.
        assert_ne!(
            fit_merchant_transaction_id("a", false),
            fit_merchant_transaction_id("a0", false)
        );
    }

    // ---------------------------------------------------------------------
    // Statement descriptor
    //
    // OPPWA's Basic Payment set carries one descriptor field, `descriptor` (AN127). The
    // `merchant.*` group is a separate section and is deliberately not mapped here.
    // ---------------------------------------------------------------------

    fn billing_descriptor(
        statement_descriptor: Option<&str>,
        suffix: Option<&str>,
    ) -> BillingDescriptor {
        BillingDescriptor {
            name: None,
            city: None,
            phone: None,
            statement_descriptor: statement_descriptor.map(str::to_string),
            statement_descriptor_suffix: suffix.map(str::to_string),
            reference: None,
        }
    }

    fn descriptor_only_request(descriptor: Option<&str>) -> AciPaymentsRequest {
        AciPaymentsRequest {
            txn_details: TransactionDetails {
                entity_id: Secret::new("8ac7a4c9".to_string()),
                amount: StringMajorUnit::zero(),
                currency: "ZAR".to_string(),
                payment_type: AciPaymentType::Debit,
            },
            payment_method: PaymentDetails::Mandate,
            instruction: None,
            shopper_result_url: None,
            three_ds_two_enrolled: None,
            recurring_type: None,
            transaction_category: None,
            merchant_transaction_id: None,
            test_mode: None,
            customer_browser_info: AciCustomerBrowserInfo::default(),
            customer_data: AciCustomerData::default(),
            billing_address: AciBillingAddress::default(),
            external_three_ds: AciExternalThreeDsData::default(),
            custom_parameters: AciCustomParameters::default(),
            descriptor: descriptor.map(str::to_string),
        }
    }

    /// The field has to survive composition into the request, not just serialize on its own:
    /// `AciPaymentsRequest` flattens five sub-structs, and `serde_urlencoded` is flat-only.
    #[test]
    fn descriptor_reaches_the_wire_on_a_composed_request() {
        let body =
            serde_urlencoded::to_string(descriptor_only_request(Some("PEACH STORE ORDER99")))
                .expect("request serializes");
        assert!(body.contains("descriptor=PEACH+STORE+ORDER99"), "{body}");
    }

    /// Nothing configured must leave the request byte-identical to the pre-PT-4088 wire.
    #[test]
    fn absent_descriptor_contributes_no_field() {
        let body =
            serde_urlencoded::to_string(descriptor_only_request(None)).expect("request serializes");
        assert!(!body.contains("descriptor"), "{body}");
    }

    /// Hyperswitch has a descriptor and a suffix; OPPWA has one field, so the parts are joined.
    #[test]
    fn descriptor_joins_the_statement_descriptor_and_its_suffix() {
        let cases = [
            (
                Some("PEACH STORE"),
                Some("ORDER99"),
                Some("PEACH STORE ORDER99"),
            ),
            (Some("PEACH STORE"), None, Some("PEACH STORE")),
            (None, Some("ORDER99"), Some("ORDER99")),
            (
                Some("  PEACH STORE  "),
                Some(" ORDER99 "),
                Some("PEACH STORE ORDER99"),
            ),
            // Blank parts are not a descriptor: an empty `descriptor=` is worse than omitting it.
            (Some("   "), Some(""), None),
            (None, None, None),
        ];
        for (statement_descriptor, suffix, expected) in cases {
            let billing = billing_descriptor(statement_descriptor, suffix);
            assert_eq!(
                join_descriptor(&billing),
                expected.map(str::to_string),
                "{statement_descriptor:?} + {suffix:?}"
            );
        }
    }

    /// Over-long values are clipped to `AN127` rather than failing the payment, and the count is
    /// in characters — a byte-wise truncation would split a multi-byte character.
    #[test]
    fn descriptor_is_clamped_to_the_documented_length() {
        let billing = billing_descriptor(Some(&"ß".repeat(200)), None);
        let descriptor = join_descriptor(&billing).expect("a descriptor is produced");
        assert_eq!(descriptor.chars().count(), ACI_DESCRIPTOR_MAX_LEN);
        assert!(descriptor.chars().all(|character| character == 'ß'));
    }
}
