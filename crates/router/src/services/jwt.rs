use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use hyperswitch_masking::PeekInterface;
use jsonwebtoken::{encode, EncodingKey, Header};

use crate::{configs::Settings, core::errors::UserErrors};

/// The decision computing a JWT's absolute expiry reaches, as something that
/// can be recorded.
///
/// [`UserErrors`] cannot be: it derives neither `Serialize` nor `Deserialize`,
/// and it carries payload-bearing variants that have nothing to do with this
/// call. Both ways this can fail -- an overflowing `checked_add` and a clock
/// reading before the epoch -- are reported as
/// `UserErrors::InternalServerError`, which is all a caller can tell apart, so
/// that is this one variant and nothing else crosses the boundary. The mapping
/// back to the public error type stays in [`generate_exp`].
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, thiserror::Error,
)]
enum JwtExpiryOutcome {
    /// The expiry instant is not representable from the current clock reading.
    #[error("JWT expiry is not representable")]
    Unrepresentable,
}

pub fn generate_exp(
    exp_duration: std::time::Duration,
) -> CustomResult<std::time::Duration, UserErrors> {
    generate_exp_absolute(exp_duration).change_context(UserErrors::InternalServerError)
}

// deja: JWT expiry uses a raw SystemTime::now() that bypasses the instrumented
// date_time::now boundary, so the `exp` claim (and thus the whole signed token)
// diverges on replay. Record/replay the absolute expiry to reproduce
// byte-identical JWTs. The error side is its own narrow outcome type so a
// recorded failure replays as that failure rather than as an unreconstructable
// sentinel.
#[cfg_attr(
    feature = "deja",
    deja::id(component = "router::jwt", operation = "generate_exp",
        codec = deja::codec::ResultCodec::<std::time::Duration, JwtExpiryOutcome>,
        on_miss = { use common_utils::synth_shape::Synthesize as _; Ok(std::time::Duration::from_nanos(
            u64::try_from(__deja_miss.epoch_nanos()).unwrap_or(0)
        ).saturating_add(exp_duration)) },)
)]
#[allow(
    clippy::disallowed_methods,
    reason = "this function IS the seam: the deja::id attribute above records and replays the absolute expiry"
)]
fn generate_exp_absolute(
    exp_duration: std::time::Duration,
) -> CustomResult<std::time::Duration, JwtExpiryOutcome> {
    std::time::SystemTime::now()
        .checked_add(exp_duration)
        .ok_or(JwtExpiryOutcome::Unrepresentable)?
        .duration_since(std::time::UNIX_EPOCH)
        .change_context(JwtExpiryOutcome::Unrepresentable)
}

pub async fn generate_jwt<T>(
    claims_data: &T,
    settings: &Settings,
) -> CustomResult<String, UserErrors>
where
    T: serde::ser::Serialize,
{
    let jwt_secret = &settings.secrets.get_inner().jwt_secret;
    encode(
        &Header::default(),
        claims_data,
        &EncodingKey::from_secret(jwt_secret.peek().as_bytes()),
    )
    .change_context(UserErrors::InternalServerError)
}

#[cfg(all(test, feature = "deja"))]
mod deja_tests {
    use super::JwtExpiryOutcome;

    fn reconstruct(
        recorded: serde_json::Value,
    ) -> Option<common_utils::errors::CustomResult<std::time::Duration, JwtExpiryOutcome>> {
        <deja::codec::ResultCodec<std::time::Duration, JwtExpiryOutcome> as deja::codec::ReplayCodec>::reconstruct(recorded)
    }

    /// A recorded expiry failure rebuilds as the same variant.
    ///
    /// The Ok-only codec this site used wrote an `Err` as
    /// `{"deja_err": "<Debug>"}`, which names no variant and so cannot rebuild
    /// one -- the second case is that document, and it must still refuse. The
    /// third case is what makes the first mean anything: a single-variant enum
    /// whose `kind` went unread would accept any string at all.
    #[test]
    fn a_recorded_error_rebuilds_its_variant() {
        let rebuilt = reconstruct(serde_json::json!({
            "version": 1,
            "result": "Err",
            "kind": "Unrepresentable",
            "message": "JWT expiry is not representable",
        }))
        .expect("a typed error must reconstruct");
        let Err(report) = &rebuilt else {
            panic!("a recorded error must rebuild as an error");
        };
        assert_eq!(report.current_context(), &JwtExpiryOutcome::Unrepresentable);

        assert!(
            reconstruct(serde_json::json!({"deja_err": "Unrepresentable"})).is_none(),
            "the Ok-only sentinel names no variant and must refuse"
        );
        assert!(
            reconstruct(serde_json::json!({
                "version": 1,
                "result": "Err",
                "kind": "NotAVariant",
                "message": "",
            }))
            .is_none(),
            "a kind naming no variant must refuse rather than fabricate one"
        );
    }
}
