use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use hyperswitch_masking::PeekInterface;
use jsonwebtoken::{encode, EncodingKey, Header};

use crate::{configs::Settings, core::errors::UserErrors};

// deja: JWT expiry uses a raw SystemTime::now() that bypasses the instrumented
// date_time::now boundary, so the `exp` claim (and thus the whole signed token)
// diverges on replay. Record/replay the absolute expiry to reproduce
// byte-identical JWTs.
#[cfg_attr(
    feature = "deja",
    deja::id(
        component = "router::jwt",
        operation = "generate_exp",
        // The typed codec, so the seam keeps the uniform contract: a recording
        // that threw replays as the same typed throw. It captures the
        // `UserErrors` this function already returns, rather than an outcome
        // type invented to suit the tape.
        codec = deja::codec::ResultCodec::<std::time::Duration, UserErrors>,
        on_miss = {
            use common_utils::synth_shape::Synthesize as _;
            Ok(
                std::time::Duration::from_nanos(
                    u64::try_from(__deja_miss.epoch_nanos()).unwrap_or(0),
                )
                .saturating_add(exp_duration),
            )
        },
    )
)]
#[allow(
    clippy::disallowed_methods,
    reason = "this function IS the seam: the deja::id attribute above records and replays the absolute expiry"
)]
pub fn generate_exp(
    exp_duration: std::time::Duration,
) -> CustomResult<std::time::Duration, UserErrors> {
    std::time::SystemTime::now()
        .checked_add(exp_duration)
        .ok_or(UserErrors::InternalServerError)?
        .duration_since(std::time::UNIX_EPOCH)
        .change_context(UserErrors::InternalServerError)
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
    use super::UserErrors;

    type Seam = deja::codec::ResultCodec<std::time::Duration, UserErrors>;

    fn capture(
        value: &common_utils::errors::CustomResult<std::time::Duration, UserErrors>,
    ) -> (serde_json::Value, bool) {
        <Seam as deja::codec::ReplayCodec>::capture(value)
    }

    fn reconstruct(
        recorded: serde_json::Value,
    ) -> Option<common_utils::errors::CustomResult<std::time::Duration, UserErrors>> {
        <Seam as deja::codec::ReplayCodec>::reconstruct(recorded)
    }

    /// What the recorder writes for an `Err` is what the fixture below assumes:
    /// a `kind` naming the variant. Captured rather than typed out, so the
    /// fixture cannot describe a shape the codec never produces.
    #[test]
    fn a_captured_error_round_trips_as_its_variant() {
        let (recorded, is_error) = capture(&Err(UserErrors::InternalServerError.into()));
        assert!(is_error, "an Err must be captured as an error");
        assert_eq!(
            recorded.get("kind").and_then(serde_json::Value::as_str),
            Some("InternalServerError"),
            "the recorded kind must name the variant: {recorded}"
        );
        let Some(Err(report)) = reconstruct(recorded) else {
            panic!("a captured error must reconstruct as an error");
        };
        assert!(matches!(
            report.current_context(),
            UserErrors::InternalServerError
        ));
    }

    /// A recorded failure rebuilds as the variant it was recorded as. The two
    /// refusals are what make that mean anything: the Ok-only codec's sentinel
    /// names no variant, and a `kind` naming none must refuse rather than pick
    /// one.
    #[test]
    fn a_recorded_error_rebuilds_its_variant() {
        let rebuilt = reconstruct(serde_json::json!({
            "version": 1,
            "result": "Err",
            "kind": "InternalServerError",
            "message": "User InternalServerError",
        }))
        .expect("a typed error must reconstruct");
        let Err(report) = &rebuilt else {
            panic!("a recorded error must rebuild as an error");
        };
        assert!(
            matches!(report.current_context(), UserErrors::InternalServerError),
            "the rebuilt error must carry the recorded variant"
        );

        assert!(
            reconstruct(serde_json::json!({"deja_err": "InternalServerError"})).is_none(),
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

    /// The test above names the codec directly, so swapping the attribute back
    /// to `ResultOkCodec` would leave it green. The selection is not observable
    /// at run time -- the macro expands it into the generated body -- so it is
    /// read out of the declaration, whose slice ends at the attribute's own
    /// `)]`.
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
            declaration
                .contains("codec = deja::codec::ResultCodec::<std::time::Duration, UserErrors>"),
            "the expiry seam must select the typed result codec, or a recorded \
             failure replays as an unreconstructable sentinel: {declaration}"
        );
    }
}
