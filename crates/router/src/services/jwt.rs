use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use hyperswitch_masking::PeekInterface;
use jsonwebtoken::{encode, EncodingKey, Header};

use crate::{configs::Settings, core::errors::UserErrors};

/// A JWT's absolute expiry, in a form a tape can carry. [`UserErrors`] cannot
/// be: no serde derives, and payload-bearing variants unrelated to this call.
/// Both failures here report as `InternalServerError`, all a caller can tell
/// apart, so one variant covers the boundary; [`generate_exp`] maps it back.
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

    /// The Ok-only codec wrote an `Err` as a `Debug` sentinel naming no variant,
    /// so it must still refuse; and the third case is what makes the first mean
    /// anything, since a `kind` that went unread would accept any string.
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

    /// The test above names the codec directly, so reverting the attribute to
    /// `ResultOkCodec` would leave it green. The selection is not observable at
    /// run time -- the macro expands it into the generated body -- so it is read
    /// out of the declaration, whose slice ends at the attribute's own `)]`.
    #[test]
    fn the_seam_selects_the_typed_result_codec() {
        let source = include_str!("jwt.rs");
        let (_, after_operation) = source
            .split_once("operation = \"generate_exp\",")
            .expect("the seam must declare its operation");
        let (declaration, _) = after_operation
            .split_once(")]")
            .expect("the seam's attribute must be closed");
        assert!(
            declaration.contains(
                "codec = deja::codec::ResultCodec::<std::time::Duration, JwtExpiryOutcome>"
            ),
            "the expiry seam must select the typed result codec, or a recorded \
             failure replays as an unreconstructable sentinel: {declaration}"
        );
    }
}
